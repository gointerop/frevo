//! The differential for the FHIRPath scenarios of `add-fhirpath-and-codegen-r4`
//! that the vendored suite cannot grade.
//!
//! Reads `harness/fhirpath-compare/differential/probes.json` (written by
//! `probes.py`) and `differential/hapi-observed.json` (written by
//! `harness/hapi-fhirpath-differential`, HAPI 8.12.0's R4 `FHIRPathEngine`
//! in-process under both of its workers), evaluates every probe on Frevo
//! through the oracle harness's own seam (`crates/frevo-path/tests/oracle/`,
//! included by path), and writes `harness/fhirpath-compare/differential-results.json`:
//! per scenario, each probe's Frevo outcome beside HAPI's under the worker the
//! scenario states, and a verdict.
//!
//! Frevo is configured as the probe says and otherwise as `FhirPathR4`
//! configures HAPI: both `as` rules not enforced, polymorphic names off, no
//! host service, no application context, no static check, UTC.
//!
//! ```sh
//! cargo run --release --manifest-path harness/fhirpath-compare/Cargo.toml --bin differential
//! ```

#[allow(dead_code)]
#[path = "../../../../crates/frevo-path/tests/oracle/mod.rs"]
mod oracle;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use frevo_path::{
    CheckError, CheckOptions, ConstantMode, FunctionDetails, HostError, HostServices, TypeSet,
    Value,
};
use oracle::engine::{
    EngineError, check_outcome_in_type, evaluate_items, host_base_evaluator, parse_with_host,
    r4_model, typed_input,
};
use oracle::input::{self, Document, SuiteInput};
use oracle::value::ResultItem;
use serde_json::{Map, Value as Json, json};

const PROBES: &str = "harness/fhirpath-compare/differential/probes.json";
const HAPI: &str = "harness/fhirpath-compare/differential/hapi-observed.json";
const LOCAL_INPUTS: &str = "harness/fhirpath-compare/differential/inputs";
const RESULTS: &str = "harness/fhirpath-compare/differential-results.json";
const TIMEOUT: Duration = Duration::from_secs(30);

/// Every UTF-16 unit outside printable ASCII as `<U+XXXX>`, as the Java probe
/// writes it, so both sides' text is compared unit for unit.
fn vis(s: &str) -> String {
    let mut out = String::new();
    for u in s.encode_utf16() {
        if (0x20..=0x7e).contains(&u) {
            out.push(u as u8 as char);
        } else {
            out.push_str(&format!("<U+{u:04X}>"));
        }
    }
    out
}

fn render_item(item: &ResultItem) -> String {
    match item {
        ResultItem::Primitive { fhir_type, value } => format!("{fhir_type}:{}", vis(value)),
        ResultItem::Quantity(q) => format!(
            "Quantity:{}|{}|{}|{}",
            vis(&q.value),
            vis(q.unit.as_deref().unwrap_or("")),
            vis(q.system.as_deref().unwrap_or("")),
            vis(q.code.as_deref().unwrap_or(""))
        ),
        ResultItem::Complex { fhir_type } => fhir_type.clone(),
    }
}

/// A host-service parameter or focus value, rendered as the Java probe renders
/// a `Base`: `type:value` for a primitive, the type name otherwise.
fn render_value(v: &Value) -> String {
    match v {
        Value::Boolean(b) => format!("boolean:{b}"),
        Value::Integer(i) => format!("integer:{i}"),
        Value::Decimal(d) => format!("decimal:{}", vis(d.lexical())),
        Value::String(s) => format!("string:{}", vis(s)),
        Value::Date(d) => format!("date:{}", vis(&d.lexical)),
        Value::DateTime(d) => format!("dateTime:{}", vis(&d.lexical)),
        Value::Time(t) => format!("time:{}", vis(&t.lexical)),
        Value::Quantity(_) => "Quantity".to_string(),
        Value::Node(n) => match n.primitive() {
            Some(p) => format!("{}:{}", n.type_name(), vis(&p.lexical)),
            None => n.type_name().to_string(),
        },
    }
}

fn mode_name(m: ConstantMode) -> &'static str {
    match m {
        ConstantMode::Explicit => "EXPLICIT",
        ConstantMode::NoValue => "NOVALUE",
        ConstantMode::ImplicitBefore => "IMPLICIT_BEFORE",
        ConstantMode::ImplicitAfter => "IMPLICIT_AFTER",
    }
}

fn item(spec: &str) -> Value {
    let (t, v) = spec.split_once(':').expect("probe item is type:value");
    match t {
        "string" => Value::String(v.to_string()),
        "integer" => Value::Integer(v.parse().expect("integer item")),
        "boolean" => Value::Boolean(v == "true"),
        _ => panic!("probe item type {t}"),
    }
}

/// The probe's host service, the same table the Java probe answers from, with
/// every call recorded.
struct Host {
    spec: Json,
    calls: Mutex<Vec<String>>,
}

impl Host {
    fn record(&self, s: String) {
        self.calls.lock().unwrap().push(s);
    }
    fn arr(&self, k: &str) -> Vec<Json> {
        self.spec
            .get(k)
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_default()
    }
    fn mode_matches(want: &str, m: ConstantMode) -> bool {
        want == "*" || want == mode_name(m)
    }
    fn function(&self, name: &str) -> Option<Json> {
        self.arr("functions")
            .into_iter()
            .find(|f| f["name"].as_str() == Some(name))
    }
}

impl HostServices for Host {
    fn resolve_constant(&self, name: &str, mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        self.record(format!(
            "resolveConstant({},{})",
            vis(name),
            mode_name(mode)
        ));
        let mut out = Vec::new();
        for c in self.arr("constants") {
            if c[0].as_str() == Some(name) && Self::mode_matches(c[1].as_str().unwrap(), mode) {
                for i in c[2].as_array().unwrap() {
                    out.push(item(i.as_str().unwrap()));
                }
            }
        }
        Ok(out)
    }

    fn resolve_constant_type(
        &self,
        name: &str,
        mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        self.record(format!(
            "resolveConstantType({},{})",
            vis(name),
            mode_name(mode)
        ));
        for c in self.arr("constantTypes") {
            if c[0].as_str() == Some(name) && Self::mode_matches(c[1].as_str().unwrap(), mode) {
                return Ok(Some(TypeSet::of(c[2].as_str().unwrap())));
            }
        }
        Ok(None)
    }

    fn resolve_reference(
        &self,
        url: &str,
        _identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        self.record(format!("resolveReference({})", vis(url)));
        Ok(None)
    }

    fn conforms_to_profile(&self, _item: &Value, url: &str) -> Result<bool, HostError> {
        self.record(format!("conformsToProfile({})", vis(url)));
        self.spec
            .get("conformsTo")
            .and_then(Json::as_bool)
            .ok_or_else(|| HostError::new("the probe's host has no conformsTo answer"))
    }

    fn resolve_value_set(&self, url: &str) -> Result<Option<Value>, HostError> {
        self.record(format!("resolveValueSet({})", vis(url)));
        if self.spec.get("valueSets").and_then(Json::as_str) == Some("worker") {
            return Ok(r4_model().and_then(|m| m.value_set(url)).flatten());
        }
        Ok(None)
    }

    fn resolve_function(&self, name: &str) -> Option<FunctionDetails> {
        self.record(format!("resolveFunction({})", vis(name)));
        self.function(name).map(|f| FunctionDetails {
            name: name.to_string(),
            min_params: f["min"].as_u64().unwrap() as usize,
            max_params: f["max"].as_u64().unwrap() as usize,
        })
    }

    fn check_function(
        &self,
        name: &str,
        _focus: &TypeSet,
        params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        self.record(format!("checkFunction({},{})", vis(name), params.len()));
        match self.function(name) {
            Some(f) => Ok(TypeSet::of(f["type"].as_str().unwrap())),
            None => Err(HostError::new(format!("unknown function {name}"))),
        }
    }

    fn execute_function(
        &self,
        name: &str,
        focus: &[Value],
        params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        let p: Vec<String> = params
            .iter()
            .map(|l| l.iter().map(render_value).collect::<Vec<_>>().join(","))
            .collect();
        self.record(format!(
            "executeFunction({},focus={},params=[{}])",
            vis(name),
            focus.len(),
            p.join(";")
        ));
        Ok(self
            .function(name)
            .map(|f| {
                f["result"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|i| item(i.as_str().unwrap()))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn param_is_type(&self, name: &str, index: usize) -> bool {
        self.record(format!("paramIsType({},{index})", vis(name)));
        self.function(name)
            .and_then(|f| f.get("typeParams").cloned())
            .and_then(|t| t.as_array().cloned())
            .is_some_and(|t| t.iter().any(|i| i.as_u64() == Some(index as u64)))
    }

    fn log(&self, name: &str, focus: &[Value]) -> bool {
        self.record(format!("log({},{})", vis(name), focus.len()));
        self.spec
            .get("log")
            .and_then(Json::as_bool)
            .unwrap_or(false)
    }
}

fn fail(stage: &str, message: &str) -> Json {
    json!({"stage": stage, "error": vis(message)})
}

fn engine_fail(stage: &str, e: EngineError) -> Json {
    match e {
        EngineError::Diagnostic { message, offset } => {
            let mut o = fail(stage, &message);
            if let Some(off) = offset {
                o["offset"] = json!(off);
            }
            o
        }
        other => json!({"stage": "not-implemented", "error": other.to_string()}),
    }
}

fn load_input(r: &str) -> Result<SuiteInput, String> {
    if let Some(f) = r.strip_prefix("suite:") {
        return input::load(f).map_err(|e| e.to_string());
    }
    let f = r.strip_prefix("local:").ok_or("input ref")?;
    let path = input::repo_path(LOCAL_INPUTS).join(f);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8(bytes.clone()).map_err(|e| e.to_string())?;
    let document = if f.ends_with(".json") {
        Document::Json(oracle::json::parse(&text).map_err(|e| e.to_string())?)
    } else {
        Document::Xml(oracle::xml::parse(&text).map_err(|e| e.to_string())?)
    };
    Ok(SuiteInput {
        file: f.to_string(),
        bytes,
        document,
    })
}

fn flag(o: &Json, k: &str) -> bool {
    o.get(k).and_then(Json::as_bool).unwrap_or(false)
}

/// One probe on Frevo.
fn run(probe: &Json) -> Json {
    let host = probe.get("host").map(|h| Host {
        spec: h.clone(),
        calls: Mutex::new(Vec::new()),
    });
    let mut out = stages(probe, host.as_ref());
    if let Some(h) = host {
        out["hostCalls"] = json!(h.calls.into_inner().unwrap());
    }
    out
}

fn stages(probe: &Json, host: Option<&Host>) -> Json {
    let host_dyn: Option<&dyn HostServices> = host.map(|h| h as &dyn HostServices);
    let settings = probe.get("settings").cloned().unwrap_or(json!({}));
    let check = probe["check"].as_str().unwrap_or("skip");

    let (root, resource_type) = match probe.get("input").and_then(Json::as_str) {
        None => (None, None),
        Some(r) => match load_input(r) {
            Err(e) => return fail("input", &e),
            Ok(i) => {
                let rt = i.resource_type().map(str::to_string);
                match typed_input(&i) {
                    Some(v) => (Some(v), rt),
                    None => return fail("input", "the harness could not build a tree"),
                }
            }
        },
    };

    let expr = match parse_with_host(probe["expr"].as_str().unwrap(), host_dyn) {
        Ok(e) => e,
        Err(e) => return engine_fail("parse", e),
    };

    if check != "skip" {
        let options = CheckOptions {
            allow_polymorphic_names: flag(&settings, "polymorphic"),
            emit_sql_on_fhir_warning: false,
        };
        match check_outcome_in_type(&expr, resource_type.as_deref(), host_dyn, options) {
            Err(e) => return engine_fail("check", e),
            Ok(Err(CheckError::Failed {
                message, offset, ..
            })) => {
                let mut o = fail("check", &message);
                if let Some(off) = offset {
                    o["offset"] = json!(off);
                }
                return o;
            }
            Ok(Err(CheckError::NotImplemented { feature, .. })) => {
                return json!({"stage": "not-implemented", "error": feature});
            }
            Ok(Ok(_)) => {}
        }
        if check == "only" {
            return json!({"stage": "checked"});
        }
    }

    let Some(base) = host_base_evaluator() else {
        return json!({"stage": "not-implemented", "error": "no R4 type package"});
    };
    let mut ev = base
        .clone()
        .with_as_rules(
            flag(&settings, "asSingleton"),
            flag(&settings, "asCaseSensitive"),
        )
        .with_polymorphic_names(flag(&settings, "polymorphic"))
        .with_application_context(flag(probe, "app"));
    if let Some(h) = host_dyn {
        ev = ev.with_host_services(h);
    }
    let repeat = probe.get("repeat").and_then(Json::as_u64).unwrap_or(1);
    let mut items = Vec::new();
    for i in 0..repeat {
        if i > 0 {
            items.push("--".to_string());
        }
        match evaluate_items(&ev, &expr, root.as_ref()) {
            Ok(r) => items.extend(r.iter().map(render_item)),
            Err(EngineError::NotImplemented) => {
                // The seam folds the crate's own answer into one message;
                // ask the evaluator again for what it said.
                let detail = match ev.evaluate(&expr, root.as_ref()) {
                    Err(e) => format!("{e:?}"),
                    Ok(_) => "the evaluator answered on the second call".to_string(),
                };
                return json!({"stage": "not-implemented", "error": vis(&detail)});
            }
            Err(e) => return engine_fail("eval", e),
        }
    }
    let has_log = ev.has_log();
    json!({"stage": "ok", "items": items, "hasLog": has_log, "log": vis(&ev.take_log())})
}

fn run_with_timeout(probe: &Json) -> Json {
    let p = probe.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || {
            let r = std::panic::catch_unwind(|| run(&p));
            let _ = tx.send(r);
        })
        .expect("spawn");
    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(v)) => v,
        Ok(Err(panic)) => {
            let msg = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            fail("panic", &msg)
        }
        Err(_) => {
            json!({"stage": "timeout", "error": format!("no answer within {} s", TIMEOUT.as_secs())})
        }
    }
}

/// `Error @1, 15: The name ...` without its `Error @<line>, <column>: `.
fn strip_lexer_prefix(s: &str) -> &str {
    let Some(rest) = s.strip_prefix("Error @") else {
        return s;
    };
    let Some((pos, msg)) = rest.split_once(": ") else {
        return s;
    };
    let is_pos = pos.split_once(", ").is_some_and(|(l, c)| {
        !l.is_empty()
            && !c.is_empty()
            && l.bytes().all(|b| b.is_ascii_digit())
            && c.bytes().all(|b| b.is_ascii_digit())
    });
    if is_pos { msg } else { s }
}

/// Whether the two outcomes agree, under the probe's grade.
fn agree(grade: &str, frevo: &Json, hapi: &Json) -> bool {
    if frevo["stage"] != hapi["stage"] {
        return false;
    }
    let ok = frevo["stage"] == "ok";
    let same_calls = frevo.get("hostCalls") == hapi.get("hostCalls");
    match grade {
        "error" => !ok && same_calls,
        "shape" => {
            let types = |o: &Json| -> Vec<String> {
                o["items"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|i| i.as_str().unwrap().split(':').next().unwrap().to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            types(frevo) == types(hapi) && same_calls
        }
        _ => {
            if !same_calls {
                return false;
            }
            if ok {
                frevo["items"] == hapi["items"]
                    && frevo["hasLog"] == hapi["hasLog"]
                    && frevo["log"] == hapi["log"]
            } else if frevo["stage"] == "parse" {
                // `ParseError::message` is upstream's wording without
                // `FHIRLexerException`'s `Error @<line>, <column>: ` prefix;
                // the position travels as a byte offset instead
                // (`crates/frevo-path/src/syntax/error.rs:37-39`,
                // `path.parse.007`). The wording is compared, the position
                // is not.
                frevo["error"].as_str() == hapi["error"].as_str().map(strip_lexer_prefix)
            } else {
                frevo.get("error") == hapi.get("error")
            }
        }
    }
}

fn main() {
    let read = |p: &str| -> Json {
        let path = input::repo_path(p);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e} (see harness/fhirpath-compare/README.md)",
                path.display()
            )
        });
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let probes = read(PROBES);
    let hapi_doc = read(HAPI);
    let hapi = &hapi_doc["results"];

    let mut scenarios: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for probe in probes.as_array().expect("probes.json is an array") {
        let id = probe["id"].as_str().unwrap();
        let scenario = probe["scenario"].as_str().unwrap().to_string();
        let worker = probe["worker"].as_str().unwrap_or("suite");
        let other = if worker == "suite" { "server" } else { "suite" };
        let grade = probe["grade"].as_str().unwrap_or("exact");
        let frevo = run_with_timeout(probe);
        let h = hapi
            .get(id)
            .unwrap_or_else(|| panic!("{id}: no HAPI observation; rerun the Java probe"));
        let hapi_graded = &h[worker];
        let hapi_other = &h[other];
        let verdict = if agree(grade, &frevo, hapi_graded) {
            "agree"
        } else if hapi_graded["stage"] == "input" {
            // HAPI's own parser refuses the input, so no HAPI evaluation over
            // it exists and the probe cannot be graded either way.
            "hapi-unreachable"
        } else if probe.get("documented").is_some() {
            "disagree-documented"
        } else {
            "disagree"
        };
        *totals.entry(verdict).or_default() += 1;

        let mut row = Map::new();
        row.insert("id".into(), json!(id));
        for k in [
            "expr",
            "input",
            "check",
            "settings",
            "host",
            "app",
            "repeat",
            "grade",
            "documented",
            "note",
        ] {
            if let Some(v) = probe.get(k) {
                row.insert(k.into(), v.clone());
            }
        }
        row.insert("worker".into(), json!(worker));
        row.insert("frevo".into(), frevo);
        row.insert("hapi".into(), hapi_graded.clone());
        if !agree("exact", hapi_graded, hapi_other) {
            row.insert(format!("hapi_{other}_worker"), hapi_other.clone());
        }
        row.insert("verdict".into(), json!(verdict));
        scenarios
            .entry(scenario)
            .or_default()
            .push(Json::Object(row));
    }

    let mut by_scenario = Map::new();
    let mut scenario_totals: BTreeMap<&str, usize> = BTreeMap::new();
    for (s, rows) in scenarios {
        let count = |v: &str| rows.iter().filter(|r| r["verdict"] == v).count();
        let (a, d, dd, u) = (
            count("agree"),
            count("disagree"),
            count("disagree-documented"),
            count("hapi-unreachable"),
        );
        let verdict = if d > 0 {
            "disagree"
        } else if dd > 0 {
            "disagree-documented"
        } else {
            "agree"
        };
        *scenario_totals.entry(verdict).or_default() += 1;
        by_scenario.insert(
            s,
            json!({"verdict": verdict, "agree": a, "disagree": d, "disagree_documented": dd,
                   "hapi_unreachable": u, "probes": rows}),
        );
    }

    let doc = json!({
        "about": "Differential of the FHIRPath scenarios the vendored suite cannot grade: each probe on Frevo and on HAPI 8.12.0's R4 FHIRPathEngine in-process, under the worker the scenario states. See harness/fhirpath-compare/README.md.",
        "hapi": {
            "engine": hapi_doc["engine"], "suite_worker": hapi_doc["suite_worker"],
            "server_worker": hapi_doc["server_worker"], "ucum": hapi_doc["ucum"], "jvm": hapi_doc["jvm"]
        },
        "frevo": "frevo-path at this commit, through crates/frevo-path/tests/oracle",
        "totals": {"probes": totals, "scenarios": scenario_totals},
        "scenarios": by_scenario,
    });
    let path = input::repo_path(RESULTS);
    let mut text = serde_json::to_string_pretty(&doc).unwrap();
    text.push('\n');
    std::fs::write(&path, text).unwrap();
    println!("probes {totals:?}; scenarios {scenario_totals:?}; wrote {RESULTS}");
}
