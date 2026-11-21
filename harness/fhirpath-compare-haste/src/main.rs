//! The vendored FHIRPath suite through `haste-fhirpath`, graded by the oracle
//! harness Frevo is graded by (`crates/frevo-path/tests/oracle/`, included by
//! path), written to `../fhirpath-compare/haste-results.json` for
//! `fhirpath-compare` to merge (task 2, `path.oracle.013`, ADR 0004).
//!
//! The binding, stage by stage:
//!
//! - parse: `haste-fhirpath` has no public parser, so the expression is
//!   evaluated once over no input and only `FHIRPathError::ParseError` counts
//!   as a parse failure;
//! - check: `haste-fhirpath` has no static check pass, so nothing fails here;
//! - evaluate: the input, as the FHIR JSON `common::input_json` produces, is
//!   deserialised into `haste_fhir_model::r4::generated::resources::Resource`
//!   and evaluated, through the case's `context` expression first when it has
//!   one, on a thread of its own with a 10 s limit (a panic or a timeout is a
//!   failure), as `haste-fhirpath`'s own `tests/hl7_suite.rs` runs the suite;
//! - results: a `System` or FHIR primitive becomes a primitive item, a
//!   `Quantity` the suite's quantity form, anything else a complex item.
//!
//! An input the engine's model cannot deserialise leaves the case
//! `did-not-run`, with the reason recorded.

#[allow(dead_code)]
#[path = "../../../crates/frevo-path/tests/oracle/mod.rs"]
mod oracle;

#[allow(dead_code)]
#[path = "../../fhirpath-compare/src/common.rs"]
mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use haste_fhir_model::r4::conversion::{
    BOOLEAN_TYPES, DATE_TIME_TYPES, NUMBER_TYPES, STRING_TYPES, downcast_bool, downcast_datetime,
    downcast_number, downcast_string,
};
use haste_fhir_model::r4::generated::resources::Resource;
use haste_fhirpath::{FHIRPathError, FPEngine};
use haste_reflect::MetaValue;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, ParsedHandle, StrictnessFlags, SuiteEngine,
};
use oracle::input::SuiteInput;
use oracle::run::CaseStatus;
use oracle::value::ResultItem;

const SYSTEM_PREFIX: &str = "http://hl7.org/fhirpath/System.";
const TIMEOUT: Duration = Duration::from_secs(10);

struct Haste {
    expressions: Vec<String>,
    inputs: HashMap<String, Result<Arc<Resource>, String>>,
    /// Inputs the model could not read, by file, for the report.
    unreadable: HashMap<String, String>,
    /// `convertToBoolean` is the harness's grading rule, the same for every
    /// engine; Frevo's binding implements it without an evaluator.
    grader: Box<dyn SuiteEngine>,
}

fn diagnostic(message: String) -> EngineError {
    EngineError::Diagnostic {
        message,
        offset: None,
    }
}

/// `System.String` is `string`, `System.Quantity` is `Quantity`; FHIR types
/// keep their names.
fn type_name(value: &dyn MetaValue) -> String {
    let fhir_type = value.fhir_type();
    match fhir_type.strip_prefix(SYSTEM_PREFIX) {
        Some(system) => common::suite_type_name("System", system),
        None => fhir_type.to_string(),
    }
}

fn item(value: &dyn MetaValue) -> ResultItem {
    let fhir_type = value.fhir_type();
    let name = type_name(value);
    let primitive = |v: String| ResultItem::Primitive {
        fhir_type: name.clone(),
        value: v,
    };
    if let Some(integer) = value.as_any().downcast_ref::<i64>() {
        return primitive(integer.to_string());
    }
    if BOOLEAN_TYPES.contains(fhir_type)
        && let Ok(b) = downcast_bool(value)
    {
        return primitive(b.to_string());
    }
    if NUMBER_TYPES.contains(fhir_type)
        && let Ok(n) = downcast_number(value)
    {
        return primitive(n.to_string());
    }
    if STRING_TYPES.contains(fhir_type)
        && let Ok(s) = downcast_string(value)
    {
        return primitive(s);
    }
    if DATE_TIME_TYPES.contains(fhir_type)
        && let Ok(t) = downcast_datetime(value)
    {
        return primitive(t);
    }
    if name == "Quantity" {
        let field = |n: &str| value.get_field(n).and_then(|v| v.flatten().first().copied());
        let number = field("value")
            .and_then(|v| downcast_number(v).ok())
            .map(|n| n.to_string())
            .unwrap_or_default();
        let unit = ["code", "unit"]
            .iter()
            .find_map(|n| field(n).and_then(|v| downcast_string(v).ok()))
            .unwrap_or_default();
        return common::quantity_item(&number, &unit);
    }
    ResultItem::Complex { fhir_type: name }
}

/// Run `expression` (after `context`) over `resource` on its own thread.
fn evaluate(
    resource: Option<Arc<Resource>>,
    context: Option<String>,
    expression: String,
) -> Result<Vec<ResultItem>, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let engine = FPEngine::new();
                let input: Vec<&dyn MetaValue> = match resource.as_deref() {
                    Some(r) => vec![r],
                    None => vec![],
                };
                let focus = match &context {
                    Some(c) => Some(
                        engine
                            .evaluate(c, input.clone())
                            .await
                            .map_err(|e| format!("context expression failed: {e}"))?,
                    ),
                    None => None,
                };
                let values: Vec<&dyn MetaValue> = match &focus {
                    Some(f) => f.iter().collect(),
                    None => input,
                };
                let result = engine
                    .evaluate(&expression, values)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(result.iter().map(item).collect::<Vec<_>>())
            })
        }));
        let _ = sender.send(outcome.map_err(|p| {
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "panic".to_string())
        }));
    });
    match receiver.recv_timeout(TIMEOUT) {
        Ok(Ok(result)) => result,
        Ok(Err(panic)) => Err(format!("panicked: {panic}")),
        Err(_) => Err(format!("did not finish within {} s", TIMEOUT.as_secs())),
    }
}

/// Whether `expression` parses, by the only public route: an evaluation over
/// no input, of which only a `ParseError` is a parse failure.
fn parses(expression: &str) -> Result<(), String> {
    let expression = expression.to_string();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let runtime = tokio::runtime::Builder::new_current_thread().build().ok()?;
            runtime.block_on(async {
                match FPEngine::new().evaluate(&expression, vec![]).await {
                    Err(e @ FHIRPathError::ParseError(_)) => Some(e.to_string()),
                    _ => None,
                }
            })
        }));
        let _ = sender.send(outcome.ok().flatten());
    });
    match receiver.recv_timeout(TIMEOUT) {
        Ok(Some(parse_error)) => Err(parse_error),
        _ => Ok(()),
    }
}

impl SuiteEngine for Haste {
    fn configure(&mut self, _config: &EngineConfig) -> Result<(), EngineError> {
        Ok(())
    }

    fn parse(&mut self, expression: &str) -> Result<ParsedHandle, EngineError> {
        parses(expression).map_err(diagnostic)?;
        self.expressions.push(expression.to_string());
        Ok(ParsedHandle(self.expressions.len() - 1))
    }

    fn check(
        &mut self,
        _parsed: ParsedHandle,
        _context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        Ok(())
    }

    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        let resource = match input {
            None => None,
            Some(input) => {
                let loaded = self
                    .inputs
                    .entry(input.file.clone())
                    .or_insert_with(|| {
                        let text = common::input_json(input)?;
                        serde_json::from_str::<Resource>(&text)
                            .map(Arc::new)
                            .map_err(|e| format!("haste-fhir-model cannot read {}: {e}", input.file))
                    })
                    .clone();
                match loaded {
                    Ok(r) => Some(r),
                    Err(e) => {
                        self.unreadable.insert(input.file.clone(), e);
                        return Err(EngineError::NotImplemented);
                    }
                }
            }
        };
        let expression = self.expressions[parsed.0].clone();
        evaluate(resource, context.context.map(str::to_string), expression).map_err(diagnostic)
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

fn main() {
    let suite = oracle::load_pinned_suite();
    let mut engine = Haste {
        expressions: Vec::new(),
        inputs: HashMap::new(),
        unreadable: HashMap::new(),
        grader: oracle::engine::engine_under_test(),
    };
    let started = Instant::now();
    let report = oracle::run::run(&suite, &mut engine, EngineConfig::default());
    let seconds = started.elapsed().as_secs_f64();

    let inputfile: HashMap<String, Option<String>> = suite
        .cases()
        .map(|c| (c.address.to_string(), c.inputfile.clone()))
        .collect();
    let mut out = String::new();
    out.push_str("{\"engine\":\"haste-fhirpath\",\"version\":\"0.129.2\",");
    out.push_str(&format!("\"wall_clock_s\":{seconds:.3},\"cases\":[\n"));
    for (i, case) in report.cases.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let mut reason = case.reason.clone();
        if case.status == CaseStatus::DidNotRun
            && let Some(Some(file)) = inputfile.get(&case.address)
            && let Some(why) = engine.unreadable.get(file)
        {
            reason = why.clone();
        }
        common::outcome_json(&case.address, case.status.as_str(), &reason, &mut out);
    }
    out.push_str("\n]}\n");
    let path = oracle::input::repo_path("harness/fhirpath-compare/haste-results.json");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    println!(
        "haste-fhirpath 0.129.2: {} passed, {} failed, {} did-not-run of {} in {seconds:.2} s",
        report.passed(),
        report.count(CaseStatus::Failed),
        report.count(CaseStatus::DidNotRun),
        report.cases.len()
    );
}
