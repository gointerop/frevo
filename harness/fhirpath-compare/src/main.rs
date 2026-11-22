//! The FHIRPath differential reference (task 2 of `add-fhirpath-and-codegen-r4`,
//! `path.oracle.013`, ADR 0004).
//!
//! Every case of the vendored suite (`oracles/fhirpath/tests-fhir-r4.xml`, 1,191
//! cases) is run through Frevo and `helios-fhirpath` 0.2.7 by the oracle
//! harness Frevo is graded by (`crates/frevo-path/tests/oracle/`, included by
//! path), so the loader, the staging and the grading are the same for both. The
//! `haste-fhirpath` outcomes come from `../fhirpath-compare-haste`, run first,
//! which writes `haste-results.json` the same way. HAPI 8.12.0's outcomes are
//! its recorded run, `oracles/fhirpath/hapi-8.12.0-results.json`. All four go to
//! `results.json`, one entry per case, with each engine's totals and the
//! wall-clock of each full run.
//!
//! `helios-fhirpath` binding, stage by stage, as its own
//! `tests/common/runner.rs` runs the suite:
//!
//! - parse: `helios_fhirpath::parse_expression`;
//! - check: `semantic_check::check_semantics` against the case's `contextType`
//!   when it has one (the only static check the crate has), else nothing;
//! - evaluate: the input as FHIR JSON (`common::input_json`) deserialised into
//!   `helios_fhir::r4::Resource`, with the `%sct`, `%loinc`, `%ucum`,
//!   `%vs-administrative-gender` and `%ext-patient-birthTime` constants its
//!   runner sets, evaluated through the case's `context` first when it has one,
//!   on a thread of its own with a 10 s limit;
//! - results: typed primitives keep their type (`System.String` is `string`), a
//!   `Quantity` takes the suite's quantity form, an object is a complex item.
//!
//! Run, from the repository root:
//!
//! ```sh
//! cargo run --release --manifest-path harness/fhirpath-compare-haste/Cargo.toml
//! cargo run --release --manifest-path harness/fhirpath-compare/Cargo.toml
//! ```

#[allow(dead_code)]
#[path = "../../../crates/frevo-path/tests/oracle/mod.rs"]
mod oracle;

#[allow(dead_code)]
mod common;

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use helios_fhirpath::{EvaluationContext, EvaluationResult, evaluator, semantic_check};

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, ParsedHandle, StrictnessFlags, SuiteEngine,
};
use oracle::input::SuiteInput;
use oracle::run::{CaseStatus, RunReport};
use oracle::value::ResultItem;

const TIMEOUT: Duration = Duration::from_secs(10);
const HAPI_RESULTS: &str = "oracles/fhirpath/hapi-8.12.0-results.json";
const HASTE_RESULTS: &str = "harness/fhirpath-compare/haste-results.json";
const RESULTS: &str = "harness/fhirpath-compare/results.json";

struct Helios {
    expressions: Vec<String>,
    inputs: HashMap<String, Result<String, String>>,
    unreadable: HashMap<String, String>,
    grader: Box<dyn SuiteEngine>,
}

fn diagnostic(message: String) -> EngineError {
    EngineError::Diagnostic {
        message,
        offset: None,
    }
}

fn type_of(ti: &Option<helios_fhirpath_support::TypeInfoResult>, default: &str) -> String {
    match ti {
        Some(t) => common::suite_type_name(&t.namespace, &t.name),
        None => default.to_string(),
    }
}

fn items(result: EvaluationResult, out: &mut Vec<ResultItem>) {
    let primitive = |fhir_type: String, value: String| ResultItem::Primitive { fhir_type, value };
    let temporal = |s: String| s.strip_prefix('@').map(str::to_string).unwrap_or(s);
    match result {
        EvaluationResult::Empty => {}
        EvaluationResult::Collection { items: list, .. } => {
            for i in list {
                items(i, out);
            }
        }
        EvaluationResult::Boolean(b, ti, _) => out.push(primitive(type_of(&ti, "boolean"), b.to_string())),
        EvaluationResult::String(s, ti, _) => out.push(primitive(type_of(&ti, "string"), s)),
        EvaluationResult::Decimal(d, ti, _) => out.push(primitive(type_of(&ti, "decimal"), d.to_string())),
        EvaluationResult::Integer(i, ti, _) | EvaluationResult::Integer64(i, ti, _) => {
            out.push(primitive(type_of(&ti, "integer"), i.to_string()))
        }
        EvaluationResult::Date(s, ti, _) => out.push(primitive(type_of(&ti, "date"), temporal(s))),
        EvaluationResult::DateTime(s, ti, _) => {
            out.push(primitive(type_of(&ti, "dateTime"), temporal(s)))
        }
        EvaluationResult::Time(s, ti, _) => {
            let s = temporal(s);
            out.push(primitive(
                type_of(&ti, "time"),
                s.strip_prefix('T').map(str::to_string).unwrap_or(s),
            ))
        }
        EvaluationResult::Quantity(d, unit, _, _) => {
            out.push(common::quantity_item(&d.to_string(), &unit))
        }
        EvaluationResult::Object { type_info, .. } => out.push(ResultItem::Complex {
            fhir_type: type_of(&type_info, "Object"),
        }),
    }
}

fn helios_context(json: Option<&str>) -> Result<EvaluationContext, String> {
    let mut context = match json {
        None => EvaluationContext::new_empty_with_default_version(),
        Some(text) => {
            let resource: helios_fhir::r4::Resource =
                serde_json::from_str(text).map_err(|e| format!("helios-fhir cannot read: {e}"))?;
            EvaluationContext::new(vec![helios_fhir::FhirResource::R4(Box::new(resource))])
        }
    };
    context.set_variable("sct", "http://snomed.info/sct".to_string());
    context.set_variable("loinc", "http://loinc.org".to_string());
    context.set_variable("ucum", "http://unitsofmeasure.org".to_string());
    context.set_variable(
        "vs-administrative-gender",
        "http://hl7.org/fhir/ValueSet/administrative-gender".to_string(),
    );
    context.set_variable(
        "ext-patient-birthTime",
        "http://hl7.org/fhir/StructureDefinition/patient-birthTime".to_string(),
    );
    Ok(context)
}

fn helios_evaluate(
    json: Option<String>,
    context_expr: Option<String>,
    expression: String,
) -> Result<Vec<ResultItem>, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let context = helios_context(json.as_deref())?;
                let result = match &context_expr {
                    None => helios_fhirpath::evaluate_expression(&expression, &context)?,
                    Some(c) => {
                        let focus = helios_fhirpath::evaluate_expression(c, &context)
                            .map_err(|e| format!("context expression failed: {e}"))?;
                        let focus = match focus {
                            EvaluationResult::Empty => Vec::new(),
                            EvaluationResult::Collection { items, .. } => items,
                            single => vec![single],
                        };
                        let parsed = helios_fhirpath::parse_expression(&expression)?;
                        let mut all = Vec::new();
                        for item in &focus {
                            match evaluator::evaluate(&parsed, &context, Some(item))
                                .map_err(|e| format!("{e:?}"))?
                            {
                                EvaluationResult::Empty => {}
                                EvaluationResult::Collection { items, .. } => all.extend(items),
                                single => all.push(single),
                            }
                        }
                        EvaluationResult::collection(all)
                    }
                };
                let mut out = Vec::new();
                items(result, &mut out);
                Ok::<_, String>(out)
            }));
            let _ = sender.send(outcome.map_err(|p| {
                p.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                    .unwrap_or_else(|| "panic".to_string())
            }));
        });
    if let Err(e) = spawned {
        return Err(format!("cannot start the evaluation thread: {e}"));
    }
    match receiver.recv_timeout(TIMEOUT) {
        Ok(Ok(result)) => result,
        Ok(Err(panic)) => Err(format!("panicked: {panic}")),
        Err(_) => Err(format!("did not finish within {} s", TIMEOUT.as_secs())),
    }
}

impl SuiteEngine for Helios {
    fn configure(&mut self, _config: &EngineConfig) -> Result<(), EngineError> {
        Ok(())
    }

    fn parse(&mut self, expression: &str) -> Result<ParsedHandle, EngineError> {
        helios_fhirpath::parse_expression(expression).map_err(diagnostic)?;
        self.expressions.push(expression.to_string());
        Ok(ParsedHandle(self.expressions.len() - 1))
    }

    fn check(
        &mut self,
        parsed: ParsedHandle,
        context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        if context.context_types.is_empty() {
            return Ok(());
        }
        let expr = helios_fhirpath::parse_expression(&self.expressions[parsed.0]).map_err(diagnostic)?;
        let types: Vec<String> = context
            .context_types
            .iter()
            .flat_map(|t| semantic_check::parse_context_type(t))
            .collect();
        semantic_check::check_semantics(&expr, &types, helios_fhir::FhirVersion::R4)
            .map_err(|e| diagnostic(format!("Semantic error: {e}")))
    }

    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        let json = match input {
            None => None,
            Some(input) => {
                let loaded = self
                    .inputs
                    .entry(input.file.clone())
                    .or_insert_with(|| {
                        let text = common::input_json(input)?;
                        // Read once here, so an input the model cannot read is
                        // reported as such rather than as a case failure.
                        helios_context(Some(&text)).map(|_| text)
                    })
                    .clone();
                match loaded {
                    Ok(text) => Some(text),
                    Err(e) => {
                        self.unreadable.insert(input.file.clone(), e);
                        return Err(EngineError::NotImplemented);
                    }
                }
            }
        };
        let expression = self.expressions[parsed.0].clone();
        helios_evaluate(json, context.context.map(str::to_string), expression).map_err(diagnostic)
    }

    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError> {
        self.grader.convert_to_boolean(items)
    }

    fn has_log(&self) -> bool {
        false
    }

    fn take_log(&mut self) -> String {
        String::new()
    }
}

/// One engine's outcome for one case.
#[derive(Clone)]
struct Outcome {
    status: String,
    reason: String,
}

fn outcomes_of(report: &RunReport, unreadable: Option<(&HashMap<String, String>, &HashMap<String, Option<String>>)>) -> BTreeMap<String, Outcome> {
    report
        .cases
        .iter()
        .map(|c| {
            let mut reason = c.reason.clone();
            if c.status == CaseStatus::DidNotRun
                && let Some((why, files)) = unreadable
                && let Some(Some(file)) = files.get(&c.address)
                && let Some(w) = why.get(file)
            {
                reason = w.clone();
            }
            (
                c.address.clone(),
                Outcome {
                    status: c.status.as_str().to_string(),
                    reason,
                },
            )
        })
        .collect()
}

/// The `{"address", "status", ...}` entries of a recorded run.
fn recorded(path: &str) -> (BTreeMap<String, Outcome>, Option<f64>) {
    let full = oracle::input::repo_path(path);
    let text = std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("cannot read {path}: {e}; run the haste crate first"));
    let doc: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} is not JSON: {e}"));
    let mut out = BTreeMap::new();
    for case in doc["cases"].as_array().into_iter().flatten() {
        let get = |k: &str| case[k].as_str().unwrap_or_default().to_string();
        out.insert(
            get("address"),
            Outcome {
                status: get("status"),
                reason: get("reason"),
            },
        );
    }
    let wall = doc["wall_clock_s"].as_str().and_then(|s| s.parse().ok()).or_else(|| doc["wall_clock_s"].to_string().parse().ok());
    (out, wall)
}

fn tally(outcomes: &BTreeMap<String, Outcome>) -> BTreeMap<String, usize> {
    let mut t = BTreeMap::new();
    for o in outcomes.values() {
        *t.entry(o.status.clone()).or_insert(0) += 1;
    }
    t
}

fn main() {
    let suite = oracle::load_pinned_suite();
    let files: HashMap<String, Option<String>> = suite
        .cases()
        .map(|c| (c.address.to_string(), c.inputfile.clone()))
        .collect();

    // Frevo, through the binding every gate uses.
    let started = Instant::now();
    let mut frevo = oracle::engine::engine_under_test();
    let frevo_report = oracle::run::run(&suite, frevo.as_mut(), EngineConfig::default());
    let frevo_s = started.elapsed().as_secs_f64();
    let frevo_out = outcomes_of(&frevo_report, None);

    // helios-fhirpath 0.2.7.
    let mut helios = Helios {
        expressions: Vec::new(),
        inputs: HashMap::new(),
        unreadable: HashMap::new(),
        grader: oracle::engine::engine_under_test(),
    };
    let started = Instant::now();
    let helios_report = oracle::run::run(&suite, &mut helios, EngineConfig::default());
    let helios_s = started.elapsed().as_secs_f64();
    let helios_out = outcomes_of(&helios_report, Some((&helios.unreadable, &files)));

    let (haste_out, haste_s) = recorded(HASTE_RESULTS);
    let (hapi_out, _) = recorded(HAPI_RESULTS);

    let mut out = String::new();
    out.push_str("{\n\"suite\":\"oracles/fhirpath/tests-fhir-r4.xml\",\n");
    out.push_str(&format!("\"case_count\":{},\n", suite.case_count()));
    out.push_str("\"grading\":\"crates/frevo-path/tests/oracle (the harness Frevo's gate uses), for frevo, helios and haste; hapi is its recorded run\",\n");
    out.push_str("\"inputs\":\"JSON inputs as fetched; XML inputs converted to FHIR JSON by harness/fhirpath-compare/src/common.rs over the pinned R4 type package; both reference engines read the same text\",\n");
    out.push_str(&format!(
        "\"environment\":{{\"os\":\"{}\",\"arch\":\"{}\",\"profile\":\"{}\"}},\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        if cfg!(debug_assertions) { "debug" } else { "release" }
    ));
    out.push_str("\"engines\":{\n");
    let engines: [(&str, &str, &BTreeMap<String, Outcome>, Option<f64>); 4] = [
        ("hapi", "8.12.0 (org.hl7.fhir.r4 FHIRPathTests at 73c432b5)", &hapi_out, None),
        ("frevo", "frevo-path at this commit", &frevo_out, Some(frevo_s)),
        ("helios", "helios-fhirpath 0.2.7", &helios_out, Some(helios_s)),
        ("haste", "haste-fhirpath 0.129.2", &haste_out, haste_s),
    ];
    for (i, (key, version, outcomes, wall)) in engines.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!("  \"{key}\":{{\"version\":"));
        common::write_json_string(version, &mut out);
        for (status, n) in tally(outcomes) {
            out.push_str(&format!(",\"{status}\":{n}"));
        }
        if let Some(w) = wall {
            out.push_str(&format!(",\"wall_clock_s\":{w:.3}"));
        }
        out.push('}');
    }
    out.push_str("\n},\n\"cases\":[\n");
    for (i, case) in suite.cases().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let address = case.address.to_string();
        out.push_str("{\"address\":");
        common::write_json_string(&address, &mut out);
        for (key, _, outcomes, _) in &engines {
            let o = outcomes.get(&address).cloned().unwrap_or(Outcome {
                status: "missing".to_string(),
                reason: String::new(),
            });
            out.push_str(&format!(",\"{key}\":"));
            common::write_json_string(&o.status, &mut out);
            if *key != "hapi" && o.status != "passed" && !o.reason.is_empty() {
                out.push_str(&format!(",\"{key}_reason\":"));
                common::write_json_string(&o.reason, &mut out);
            }
        }
        out.push('}');
    }
    out.push_str("\n]\n}\n");
    let path = oracle::input::repo_path(RESULTS);
    std::fs::write(&path, &out).unwrap_or_else(|e| panic!("cannot write {RESULTS}: {e}"));

    for (key, version, outcomes, wall) in &engines {
        let t = tally(outcomes);
        println!(
            "{key:6} {version:52} passed {:4}  failed {:4}  did-not-run {:4}  other {:4}{}",
            t.get("passed").copied().unwrap_or(0),
            t.get("failed").copied().unwrap_or(0),
            t.get("did-not-run").copied().unwrap_or(0),
            t.iter()
                .filter(|(k, _)| !["passed", "failed", "did-not-run"].contains(&k.as_str()))
                .map(|(_, n)| n)
                .sum::<usize>(),
            wall.map(|w| format!("  {w:.2} s")).unwrap_or_default()
        );
    }
    println!("wrote {RESULTS}");
}
