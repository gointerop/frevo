//! Utility and FHIR-specific functions: `path.utility.001` through
//! `path.utility.028`.
//!
//! ```text
//! cargo test -p frevo-path --test path_utility
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Nine scenarios name a suite group
//! or case and are driven through the committed harness. The rest are about the
//! **host service**, and the delta marks most of them differential-only: no
//! live case calls `resolve()` at all, none grades `memberOf` in either form,
//! none grades a custom function, and none grades the trace hook.
//!
//! Those are driven through the crate's published `HostServices` trait, which
//! exists today, with the engine still unbound. The three host-service states
//! the engine distinguishes (`design.md` section 4.4) are all exercised:
//!
//! | State | `conformsTo` | Scenario |
//! |---|---|---|
//! | unset | raises a no-host-services error | `path.utility.022` |
//! | the suite's | `true` for Patient, `false` for Person, raises otherwise | `path.utility.012`, `013` |
//! | the server-facing one | `false` unconditionally | `path.utility.014` |
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`, plus
//! `reference/hapi-fhir/hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java`
//! for the server-facing host service.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::{Arc, Mutex, OnceLock};

use frevo_path::{
    FunctionDetails, HostError, HostServices, Node, NodeError, Primitive, TypeSet, Value,
};

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// In-memory nodes, for the values a host service hands back
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TestNode {
    type_name: String,
    is_resource: bool,
    children: Vec<(String, Value)>,
    primitive: Option<Primitive>,
}

impl TestNode {
    fn resource(type_name: &str, children: Vec<(String, Value)>) -> Value {
        Value::Node(Arc::new(TestNode {
            type_name: type_name.to_string(),
            is_resource: true,
            children,
            primitive: None,
        }))
    }

    fn element(type_name: &str, children: Vec<(String, Value)>) -> Value {
        Value::Node(Arc::new(TestNode {
            type_name: type_name.to_string(),
            is_resource: false,
            children,
            primitive: None,
        }))
    }

    fn prim(type_name: &str, lexical: &str) -> Value {
        Value::Node(Arc::new(TestNode {
            type_name: type_name.to_string(),
            is_resource: false,
            children: Vec::new(),
            primitive: Some(Primitive {
                type_name: type_name.to_string(),
                lexical: lexical.to_string(),
                id: None,
                extensions: Vec::new(),
            }),
        }))
    }
}

impl Node for TestNode {
    fn type_name(&self) -> &str {
        &self.type_name
    }

    fn is_resource(&self) -> bool {
        self.is_resource
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(self
            .children
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .collect())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(self.children.clone())
    }

    fn primitive(&self) -> Option<Primitive> {
        self.primitive.clone()
    }
}

/// A `ValueSet` whose `compose.include.concept.code` lists `codes`, which is
/// what a host service hands back from `resolve_value_set`.
fn value_set(url: &str, codes: &[&str]) -> Value {
    let concepts: Vec<(String, Value)> = codes
        .iter()
        .map(|c| {
            (
                "concept".to_string(),
                TestNode::element(
                    "ValueSet.compose.include.concept",
                    vec![("code".to_string(), TestNode::prim("code", c))],
                ),
            )
        })
        .collect();
    TestNode::resource(
        "ValueSet",
        vec![
            ("url".to_string(), TestNode::prim("uri", url)),
            (
                "compose".to_string(),
                TestNode::element(
                    "ValueSet.compose",
                    vec![(
                        "include".to_string(),
                        TestNode::element("ValueSet.compose.include", concepts),
                    )],
                ),
            ),
        ],
    )
}

// ---------------------------------------------------------------------------
// A configurable host service, over the crate's published trait
// ---------------------------------------------------------------------------

/// What this host service answers, and what it was asked.
#[derive(Default)]
struct ConfigurableHost {
    /// `url` to the resource type a reference resolves to.
    references: Vec<(String, String)>,
    /// URLs for which `resolve_reference` **raises** rather than answering.
    raising_references: Vec<String>,
    /// Profiles `conforms_to_profile` answers `true` for.
    conforming: Vec<String>,
    /// Profiles `conforms_to_profile` **raises** for. `false` is the answer for
    /// anything in neither list.
    unknown_profiles: Vec<String>,
    /// Value sets that resolve, with the codes they contain.
    value_sets: Vec<(String, Vec<String>)>,
    /// `conforms_to_profile` answers `false` for every URL, whatever the lists
    /// say: the server-facing host service (`path.utility.014`).
    always_false: bool,
    /// Custom functions this host service declares.
    functions: Vec<FunctionDetails>,
    /// `(function, parameter index)` pairs passed unevaluated as a type name.
    type_params: Vec<(String, usize)>,
    /// What `log` returns. `true` suppresses the engine's own buffer.
    log_answer: bool,
    calls: Mutex<Vec<String>>,
}

impl ConfigurableHost {
    fn record(&self, call: String) {
        self.calls.lock().expect("not poisoned").push(call);
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("not poisoned").clone()
    }

    /// `FHIRPathTestEvaluationServices`: `true` for the `Patient` profile,
    /// `false` for `Person`, and a raise for anything else.
    fn suite() -> Self {
        ConfigurableHost {
            conforming: vec!["http://hl7.org/fhir/StructureDefinition/Patient".to_string()],
            unknown_profiles: vec!["http://trash".to_string()],
            ..ConfigurableHost::default()
        }
    }

    /// `FhirPathR4.setEvaluationContext`'s anonymous implementation: every
    /// `conformsToProfile` answer is `false`, `resolveValueSet` is null,
    /// `resolveFunction` is null, `paramIsType` is false and `log` is false.
    fn server_facing() -> Self {
        ConfigurableHost {
            always_false: true,
            ..ConfigurableHost::default()
        }
    }
}

impl HostServices for ConfigurableHost {
    fn resolve_constant(
        &self,
        name: &str,
        _mode: frevo_path::ConstantMode,
    ) -> Result<Vec<Value>, HostError> {
        self.record(format!("resolve_constant({name})"));
        Ok(Vec::new())
    }

    fn resolve_constant_type(
        &self,
        name: &str,
        _mode: frevo_path::ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        self.record(format!("resolve_constant_type({name})"));
        Ok(None)
    }

    fn resolve_reference(
        &self,
        url: &str,
        identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        self.record(format!(
            "resolve_reference({url}, identifier={})",
            identifier.is_some()
        ));
        if self.raising_references.iter().any(|u| u == url) {
            return Err(HostError::new(format!("cannot resolve {url}")));
        }
        Ok(self
            .references
            .iter()
            .find(|(u, _)| u == url)
            .map(|(_, t)| {
                TestNode::resource(t, vec![("id".to_string(), TestNode::prim("id", "1"))])
            }))
    }

    fn conforms_to_profile(&self, _item: &Value, url: &str) -> Result<bool, HostError> {
        self.record(format!("conforms_to_profile({url})"));
        if self.always_false {
            return Ok(false);
        }
        if self.unknown_profiles.iter().any(|u| u == url) {
            return Err(HostError::new(format!("unknown profile {url}")));
        }
        Ok(self.conforming.iter().any(|u| u == url))
    }

    fn resolve_value_set(&self, url: &str) -> Result<Option<Value>, HostError> {
        self.record(format!("resolve_value_set({url})"));
        Ok(self
            .value_sets
            .iter()
            .find(|(u, _)| u == url)
            .map(|(u, codes)| {
                let refs: Vec<&str> = codes.iter().map(|c| c.as_str()).collect();
                value_set(u, &refs)
            }))
    }

    fn resolve_function(&self, name: &str) -> Option<FunctionDetails> {
        self.record(format!("resolve_function({name})"));
        self.functions.iter().find(|f| f.name == name).cloned()
    }

    fn check_function(
        &self,
        name: &str,
        _focus: &TypeSet,
        _params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        self.record(format!("check_function({name})"));
        if self.functions.iter().any(|f| f.name == name) {
            Ok(TypeSet::of("string"))
        } else {
            Err(HostError::new(format!("no such function: {name}")))
        }
    }

    fn execute_function(
        &self,
        name: &str,
        focus: &[Value],
        params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        self.record(format!(
            "execute_function({name}, focus={}, params={})",
            focus.len(),
            params.len()
        ));
        if self.functions.iter().any(|f| f.name == name) {
            Ok(vec![Value::String(format!("{name}:{}", params.len()))])
        } else {
            Err(HostError::new(format!("no such function: {name}")))
        }
    }

    fn param_is_type(&self, name: &str, index: usize) -> bool {
        self.type_params
            .iter()
            .any(|(n, i)| n == name && *i == index)
    }

    fn log(&self, name: &str, focus: &[Value]) -> bool {
        self.record(format!("log({name}, {} item(s))", focus.len()));
        self.log_answer
    }
}

// ---------------------------------------------------------------------------
// The host-aware engine seam
// ---------------------------------------------------------------------------

/// One evaluation through the host-aware seam.
struct Call<'a> {
    expr: &'a str,
    /// The resource evaluation runs over, as an in-memory node. `None` means no
    /// focus resource at all.
    root: Option<Value>,
    /// A FHIRPath expression selecting the starting node inside `root`.
    context_path: Option<&'a str>,
    host: Option<&'a dyn HostServices>,
}

impl<'a> Call<'a> {
    fn new(expr: &'a str) -> Self {
        Call {
            expr,
            root: None,
            context_path: None,
            host: None,
        }
    }

    fn over(mut self, root: Value) -> Self {
        self.root = Some(root);
        self
    }

    fn at(mut self, context_path: &'a str) -> Self {
        self.context_path = Some(context_path);
        self
    }

    fn with_host(mut self, host: &'a dyn HostServices) -> Self {
        self.host = Some(host);
        self
    }
}

/// What the host-service scenarios need beyond the committed harness: a
/// caller-supplied host service, an in-memory root resource, and the engine's
/// own trace buffer.
trait HostAwareEngine {
    fn evaluate(&mut self, call: &Call<'_>) -> Result<Vec<Value>, EngineError>;

    /// Parse only, which is where a custom function's arity is taken from the
    /// host service's `FunctionDetails` (`path.utility.027`).
    fn parse(&mut self, expr: &str, host: Option<&dyn HostServices>) -> Result<(), EngineError>;

    /// The check pass, which asks the host service for a custom function's
    /// result type.
    fn check(
        &mut self,
        expr: &str,
        context_type: &str,
        host: Option<&dyn HostServices>,
    ) -> Result<TypeSet, EngineError>;

    fn has_log(&self) -> bool;

    /// Returns the accumulated trace text **and clears it**.
    fn take_log(&mut self) -> String;
}

fn host_engine_under_test() -> Box<dyn HostAwareEngine> {
    Box::new(UnboundHostEngine)
}

struct UnboundHostEngine;

impl HostAwareEngine for UnboundHostEngine {
    fn evaluate(&mut self, _call: &Call<'_>) -> Result<Vec<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn parse(&mut self, _expr: &str, _host: Option<&dyn HostServices>) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn check(
        &mut self,
        _expr: &str,
        _context_type: &str,
        _host: Option<&dyn HostServices>,
    ) -> Result<TypeSet, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn has_log(&self) -> bool {
        false
    }

    fn take_log(&mut self) -> String {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

fn expect_group(group: &str, expected: usize) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    assert_eq!(
        g.cases.len(),
        expected,
        "{group} must have {expected} case(s), not {}",
        g.cases.len()
    );
    for case in &g.cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        assert_eq!(
            r.status,
            CaseStatus::Passed,
            "{}: {:?} is {} -- {}",
            r.address,
            r.expression,
            r.status.as_str(),
            r.reason
        );
    }
}

fn expect_case(group: &str, name: &str) {
    let (suite, report) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    let r = report
        .get(&case.address.to_string())
        .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
    assert_eq!(
        r.status,
        CaseStatus::Passed,
        "{}: {:?} is {} -- {}",
        r.address,
        r.expression,
        r.status.as_str(),
        r.reason
    );
}

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be evaluated over: {e}")
    })
}

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is evaluated: {e}"));
    engine
}

fn context_for<'a>(inp: Option<&'a SuiteInput>) -> CaseContext<'a> {
    let resource_type = inp.and_then(|i| i.resource_type());
    CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    }
}

fn eval_over(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    engine
        .evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

fn eval_patient(expr: &str) -> Vec<ResultItem> {
    let inp = load_input("patient-example.xml");
    eval_over(expr, Some(&inp))
}

/// Evaluate over a suite input and return both the result and the engine's
/// trace log, drained afterwards as the harness drains it.
fn eval_with_log(expr: &str, inp: Option<&SuiteInput>) -> (Vec<ResultItem>, String) {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    let values = engine
        .evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"));
    assert!(
        engine.has_log(),
        "{expr:?} calls trace(), so the engine must report a log"
    );
    let log = engine.take_log();
    (values, log)
}

fn expect_evaluation_failure(expr: &str, inp: Option<&SuiteInput>) -> String {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before it can fail at evaluation: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    match engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

fn primitive(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

fn boolean(value: bool) -> Vec<ResultItem> {
    primitive("boolean", if value { "true" } else { "false" })
}

fn integer(value: &str) -> Vec<ResultItem> {
    primitive("integer", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

fn describe(value: &Value) -> String {
    match value {
        Value::Boolean(b) => format!("boolean:{b}"),
        Value::Integer(i) => format!("integer:{i}"),
        Value::Decimal(d) => format!("decimal:{}", d.lexical()),
        Value::String(s) => format!("string:{s}"),
        Value::Date(d) => format!("date:{}", d.lexical),
        Value::DateTime(d) => format!("dateTime:{}", d.lexical),
        Value::Time(t) => format!("time:{}", t.lexical),
        Value::Quantity(q) => format!("Quantity:{}", q.value.lexical()),
        Value::Node(n) => match n.primitive() {
            Some(p) => format!("{}:{}", p.type_name, p.lexical),
            None => format!("{}:<element>", n.type_name()),
        },
    }
}

fn rendered(values: &[Value]) -> Vec<String> {
    values.iter().map(describe).collect()
}

fn host_eval(call: &Call<'_>) -> Vec<Value> {
    let mut engine = host_engine_under_test();
    engine
        .evaluate(call)
        .unwrap_or_else(|e| panic!("{:?} must evaluate: {e}", call.expr))
}

fn host_eval_failure(call: &Call<'_>) -> String {
    let mut engine = host_engine_under_test();
    match engine.evaluate(call) {
        Ok(values) => panic!(
            "{:?} must fail, but it returned {:?}",
            call.expr,
            rendered(&values)
        ),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{:?} must fail with a diagnostic, not: {e}", call.expr),
    }
}

/// A `Patient` with one `contained` `Organization` whose id is `org1`, and a
/// `managingOrganization` reference to `#org1`.
fn patient_with_contained() -> Value {
    TestNode::resource(
        "Patient",
        vec![
            ("id".to_string(), TestNode::prim("id", "p1")),
            (
                "contained".to_string(),
                TestNode::resource(
                    "Organization",
                    vec![
                        ("id".to_string(), TestNode::prim("id", "org1")),
                        ("name".to_string(), TestNode::prim("string", "Acme")),
                    ],
                ),
            ),
            (
                "managingOrganization".to_string(),
                TestNode::element(
                    "Reference",
                    vec![("reference".to_string(), TestNode::prim("string", "#org1"))],
                ),
            ),
        ],
    )
}

// ---------------------------------------------------------------------------
// path.utility.001 -- trace passes through and records a log line
// ---------------------------------------------------------------------------

// covers: path.utility.001
#[test]
fn trace_passes_its_input_through_and_records_a_log_line() {
    expect_group("testTrace", 2);

    let patient = load_input("patient-example.xml");

    // `funcTrace` (FHIRPathEngine.java:5623) returns the focus unchanged, so
    // the expression's value is unaffected.
    let (values, log) = eval_with_log("name.given.trace('test').count()", Some(&patient));
    assert_eq!(values, integer("5"));

    // The log entry's shape is `name`, `: `, then the traced items' string
    // values joined by `,` (`log`, FHIRPathEngine.java:927-943).
    assert_eq!(log, "test: Peter,James,Jim,Peter,James");

    // Two traces in one expression are joined by `; ` in evaluation order
    // (:929-931).
    let (_, log) = eval_with_log("name.given.trace('a').count().trace('b')", Some(&patient));
    assert_eq!(log, "a: Peter,James,Jim,Peter,James; b: 5");

    // Taking the log **clears** it: a second take with no intervening trace is
    // the empty string and the predicate is false (`takeLog`, :6692).
    let mut engine = fresh_engine();
    let context = context_for(Some(&patient));
    let parsed = engine.parse("name.given.trace('once')").expect("parses");
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .expect("checks");
    engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("evaluates");
    assert!(engine.has_log());
    assert_eq!(engine.take_log(), "once: Peter,James,Jim,Peter,James");
    assert!(
        !engine.has_log(),
        "taking the log must clear it, which is what stops one case's lines \
         leaking into the next across the 1,191 shared-engine cases"
    );
    assert_eq!(engine.take_log(), "");

    // `trace` is the only construct that writes to the log: an expression with
    // no trace leaves the buffer empty.
    let mut engine = fresh_engine();
    let parsed = engine.parse("name.given.count()").expect("parses");
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .expect("checks");
    engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("evaluates");
    assert!(!engine.has_log(), "no trace, no log");
    assert_eq!(engine.take_log(), "");
}

// ---------------------------------------------------------------------------
// path.utility.002 -- trace with a projection logs the projection
// ---------------------------------------------------------------------------

// covers: path.utility.002
#[test]
fn trace_with_a_projection_logs_the_projection_and_not_the_input() {
    let patient = load_input("patient-example.xml");

    // The two-parameter form evaluates its second argument against the focus
    // and logs **that**, while still returning the focus
    // (`funcTrace`, FHIRPathEngine.java:5626-5628).
    let (values, log) = eval_with_log("name.trace('test', given).count()", Some(&patient));
    assert_eq!(values, integer("3"), "the result is the focus, three names");
    assert_eq!(
        log, "test: Peter,James,Jim,Peter,James",
        "the log is the projection, five given names"
    );
    expect_case("testTrace", "testTrace2");

    // `testSplit/testSplit3`: nine segments traced under `n`, and the count
    // still nine.
    let expr = "'[stop]ONE[stop][stop]TWO[stop][stop][stop]THREE[stop][stop]'.split('[stop]').trace('n').count() = 9";
    let (values, log) = eval_with_log(expr, Some(&patient));
    assert_eq!(values, boolean(true));
    assert!(
        log.starts_with("n: "),
        "the entry must be recorded under the trace name `n`: {log:?}"
    );
    assert_eq!(
        log.matches(',').count(),
        8,
        "nine traced items means eight separators: {log:?}"
    );
    expect_case("testSplit", "testSplit3");
}

// ---------------------------------------------------------------------------
// path.utility.003 -- check() passes its input through
// ---------------------------------------------------------------------------

// covers: path.utility.003
#[test]
fn check_passes_its_input_through_when_the_condition_holds() {
    // `funcCheck` (FHIRPathEngine.java:5649) returns the focus unchanged when
    // `convertToBoolean` of its first argument is true.
    assert_eq!(
        eval_patient("Patient.name.check(given.exists(), 'given must exist').count()"),
        integer("3")
    );
    assert_eq!(
        eval_patient("Patient.name.first().check(true, 'always').family"),
        primitive("string", "Chalmers")
    );

    // Unchanged means the focus itself, so the check is transparent mid-path.
    assert_eq!(
        eval_patient("Patient.name.check(true, 'always')"),
        eval_patient("Patient.name")
    );
}

// ---------------------------------------------------------------------------
// path.utility.004 -- check() fails when the condition does not hold
// ---------------------------------------------------------------------------

// covers: path.utility.004
#[test]
fn check_fails_when_the_condition_does_not_hold() {
    let patient = load_input("patient-example.xml");

    // `throw makeException(expr, FHIRPATH_CHECK_FAILED, name)`
    // (FHIRPathEngine.java:5654), where `name` is the **second** argument's
    // value -- so the message carries the caller's text.
    let message = expect_evaluation_failure(
        "Patient.name.check(prefix.exists(), 'every name needs a prefix')",
        Some(&patient),
    );
    assert!(
        message.contains("every name needs a prefix"),
        "the failure must carry the check's own name: {message}"
    );

    // A false literal fails too, so the failure is about the condition and not
    // about the navigation.
    expect_evaluation_failure("Patient.name.check(false, 'never')", Some(&patient));

    // And it fails rather than returning empty, which is the distinction from
    // `where`.
    expect_evaluation_failure("{}.check(false, 'never').empty()", None);
}

// ---------------------------------------------------------------------------
// path.utility.005 -- aggregate folds with $this and $total
// ---------------------------------------------------------------------------

// covers: path.utility.005
#[test]
fn aggregate_folds_with_this_and_total() {
    // `funcAggregate` (FHIRPathEngine.java:5295) sets `total` from the second
    // argument and then rebinds `$this` and `$total` per item.
    assert_eq!(
        eval_patient("(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 0) = 45"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 2) = 47"),
        boolean(true)
    );
    expect_case("testAggregate", "testAggregate1");
    expect_case("testAggregate", "testAggregate2");

    // The seed really is the second argument: changing it changes the answer by
    // exactly that amount.
    assert_eq!(
        eval_patient("(1|2|3).aggregate($this+$total, 10)"),
        integer("16")
    );
}

// ---------------------------------------------------------------------------
// path.utility.006 -- aggregate with no initial value
// ---------------------------------------------------------------------------

// covers: path.utility.006
#[test]
fn aggregate_without_an_initial_value_starts_with_an_empty_total() {
    // With one argument `total` is left null, so the first iteration sees
    // `$total.empty()` as true -- which is how the min and max idioms work.
    assert_eq!(
        eval_patient(
            "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, iif($this < $total, $this, $total))) = 1"
        ),
        boolean(true)
    );
    assert_eq!(
        eval_patient(
            "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, iif($this > $total, $this, $total))) = 9"
        ),
        boolean(true)
    );
    expect_case("testAggregate", "testAggregate3");
    expect_case("testAggregate", "testAggregate4");
    expect_group("testAggregate", 4);

    // An empty focus folds to an empty total, not to zero.
    assert_eq!(eval_patient("{}.aggregate($this+$total)"), empty());
}

// ---------------------------------------------------------------------------
// path.utility.007 -- sort, with no argument and with a key
// ---------------------------------------------------------------------------

// covers: path.utility.007
#[test]
fn sort_orders_a_collection_with_no_argument_and_with_a_key() {
    expect_group("testSort", 10);

    // `funcSort` (FHIRPathEngine.java:6224) copies the focus and sorts the copy
    // with `FHIRPathBaseSorter`, so the input order is not relied on.
    assert_eq!(
        eval_patient("(3 | 2 | 1).sort() = (1 | 2 | 3)"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("(1 | 2 | 3).sort() = (1 | 2 | 3)"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("('c' | 'b' | 'a').sort($this) = ('a' | 'b' | 'c')"),
        boolean(true)
    );

    // A negated key reverses the order, which is how descending is written.
    assert_eq!(
        eval_patient("('a' | 'b' | 'c').sort(-$this) = ('c' | 'b' | 'a')"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("(1 | 2 | 3).sort(-$this) = (3 | 2 | 1)"),
        boolean(true)
    );

    // Sorting a navigation result rather than a literal collection, which is
    // `testSort9`.
    assert_eq!(
        eval_patient("Patient.name[0].given.sort() = ('James' | 'Peter')"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.utility.008 -- sort with several keys
// ---------------------------------------------------------------------------

// covers: path.utility.008
#[test]
fn sort_accepts_several_keys_and_a_descending_key() {
    // Every parameter of `sort` is an expression parameter
    // (`isExpressionParameter`, FHIRPathEngine.java:4805-4807 returns true for
    // `Function.Sort` at any index), which is what lets it take more than one
    // key.
    assert_eq!(
        eval_patient("Patient.name.sort(-family, -given.first()).first().use = 'usual'"),
        boolean(true)
    );
    expect_case("testSort", "testSort10");

    // The second key decides only where the first ties, so dropping it changes
    // the answer here: the name with no `family` sorts first under the
    // descending family key.
    assert_eq!(
        eval_patient("Patient.name.sort(-family).first().use"),
        primitive("code", "usual")
    );
}

// ---------------------------------------------------------------------------
// path.utility.009 -- extension(url) selects by URL
// ---------------------------------------------------------------------------

// covers: path.utility.009
#[test]
fn extension_selects_extensions_by_url() {
    expect_group("testExtension", 3);

    const BIRTH_TIME: &str = "http://hl7.org/fhir/StructureDefinition/patient-birthTime";

    // `funcExtension` (FHIRPathEngine.java:5464) collects the focus's
    // extensions and keeps those whose `url` equals the argument.
    assert_eq!(
        eval_patient(
            "Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').exists()"
        ),
        boolean(true)
    );
    assert_eq!(
        eval_patient(
            "Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').count()"
        ),
        integer("1")
    );

    // The selected item is the `Extension` itself, so its `url` and `value` are
    // both reachable.
    assert_eq!(
        eval_patient(
            "Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').url"
        ),
        primitive("uri", BIRTH_TIME)
    );
}

// ---------------------------------------------------------------------------
// path.utility.010 -- extension(url) with no match
// ---------------------------------------------------------------------------

// covers: path.utility.010
#[test]
fn extension_with_no_match_yields_empty() {
    // The filter simply matches nothing, so the result is empty rather than an
    // error -- and an unknown URL is a check-pass *warning* rather than a
    // failure (`path.check.018`).
    assert_eq!(
        eval_patient("Patient.extension('http://example.org/nothing')"),
        empty()
    );
    assert_eq!(
        eval_patient(
            "Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime1').empty()"
        ),
        boolean(true)
    );
    expect_case("testExtension", "testExtension3");

    // An element with no extensions at all is empty too, not a failure.
    assert_eq!(
        eval_patient("Patient.gender.extension('http://x')"),
        empty()
    );
}

// ---------------------------------------------------------------------------
// path.utility.011 -- extension reaches a primitive's extensions
// ---------------------------------------------------------------------------

// covers: path.utility.011
#[test]
fn extension_reaches_a_primitives_extensions() {
    // The extension in `patient-example.xml:96-100` hangs off a **primitive**,
    // `birthDate`, which in the JSON form is the `_birthDate` sibling. Reaching
    // it requires the primitive value to carry its sibling element
    // (`frevo_path::Primitive`, `design.md` section 4.1).
    assert_eq!(
        eval_patient(
            "Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').exists()"
        ),
        boolean(true)
    );
    expect_case("testExtension", "testExtension1");
    expect_case("testExtension", "testExtension2");

    // The extension's value is reachable through it, so this is the whole
    // element and not just a URL match.
    assert_eq!(
        eval_patient("Patient.birthDate.extension(%`ext-patient-birthTime`).value.exists()"),
        boolean(true)
    );

    // And the primitive still has its own value afterwards.
    assert_eq!(
        eval_patient("Patient.birthDate"),
        primitive("date", "1974-12-25")
    );
}

// ---------------------------------------------------------------------------
// path.utility.012 -- conformsTo asks the host service
// ---------------------------------------------------------------------------

// covers: path.utility.012
#[test]
fn conforms_to_asks_the_host_service() {
    expect_group("testConformsTo", 3);

    // The suite's host service answers `true` for the Patient profile and
    // `false` for Person, and `funcConformsTo` (FHIRPathEngine.java:5972)
    // passes the focus item and the URL straight through.
    assert_eq!(
        eval_patient("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("conformsTo('http://hl7.org/fhir/StructureDefinition/Person')"),
        boolean(false)
    );

    // Asked, not inferred: the same two URLs through a recording host service
    // produce two calls carrying the URLs.
    let host = ConfigurableHost::suite();
    let patient = patient_with_contained();
    assert_eq!(
        rendered(&host_eval(
            &Call::new("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')")
                .over(patient.clone())
                .with_host(&host)
        )),
        ["boolean:true"]
    );
    assert_eq!(
        host.calls(),
        ["conforms_to_profile(http://hl7.org/fhir/StructureDefinition/Patient)"]
    );
}

// ---------------------------------------------------------------------------
// path.utility.013 -- conformsTo with a profile the host does not know
// ---------------------------------------------------------------------------

// covers: path.utility.013
#[test]
fn conforms_to_with_an_unknown_profile_is_an_execution_error() {
    let patient = load_input("patient-example.xml");

    // The suite's host service **raises** for anything but Patient and Person,
    // and the engine does not swallow it -- unlike `resolve()`, which does
    // (`path.utility.020`). The case carries `invalid="execution"`.
    let message = expect_evaluation_failure("conformsTo('http://trash')", Some(&patient));
    assert!(
        !message.is_empty(),
        "the failure must carry the host service's message"
    );
    expect_case("testConformsTo", "testConformsTo3");

    // Through the recording host service: the call is made and the error
    // propagates.
    let host = ConfigurableHost::suite();
    let node = patient_with_contained();
    host_eval_failure(
        &Call::new("conformsTo('http://trash')")
            .over(node)
            .with_host(&host),
    );
    assert_eq!(host.calls(), ["conforms_to_profile(http://trash)"]);
}

// ---------------------------------------------------------------------------
// path.utility.014 -- conformsTo through the server-facing host service
// ---------------------------------------------------------------------------

// covers: path.utility.014
#[test]
fn conforms_to_through_the_server_facing_host_service_is_always_false() {
    // `FhirPathR4.setEvaluationContext` installs an anonymous
    // `IHostApplicationServices` whose `conformsToProfile` is
    // `return false;` -- unconditionally, for every URL and every item. So the
    // same two URLs that answer true and false under the suite's host service
    // both answer false here.
    let host = ConfigurableHost::server_facing();
    let patient = patient_with_contained();

    for url in [
        "http://hl7.org/fhir/StructureDefinition/Patient",
        "http://hl7.org/fhir/StructureDefinition/Person",
        "http://trash",
        "http://example.org/anything",
    ] {
        let expr = format!("conformsTo('{url}')");
        let values = host_eval(&Call::new(&expr).over(patient.clone()).with_host(&host));
        assert_eq!(
            rendered(&values),
            ["boolean:false"],
            "the server-facing host service answers false for {url}"
        );
    }

    // False, and specifically not a failure: this is a configured host service,
    // which is a different state from no host service at all
    // (`path.utility.022`).
    assert_eq!(host.calls().len(), 4);
}

// ---------------------------------------------------------------------------
// path.utility.015 -- memberOf asks the host service for the value set
// ---------------------------------------------------------------------------

// covers: path.utility.015
#[test]
fn member_of_asks_the_host_service_for_the_value_set() {
    const VS: &str = "http://hl7.org/fhir/ValueSet/name-use";

    // `funcMemberOf` (FHIRPathEngine.java:4786) resolves the value set through
    // the host service when one is installed, then validates the focus against
    // it.
    let host = ConfigurableHost {
        value_sets: vec![(
            VS.to_string(),
            vec!["usual".to_string(), "official".to_string()],
        )],
        ..ConfigurableHost::default()
    };
    let patient = TestNode::resource(
        "Patient",
        vec![(
            "name".to_string(),
            TestNode::element(
                "HumanName",
                vec![("use".to_string(), TestNode::prim("code", "usual"))],
            ),
        )],
    );

    let expr = format!("Patient.name.use.memberOf('{VS}')");
    let values = host_eval(&Call::new(&expr).over(patient.clone()).with_host(&host));
    assert_eq!(
        rendered(&values),
        ["boolean:true"],
        "`usual` is in the value set the host service returned"
    );
    assert_eq!(host.calls(), [format!("resolve_value_set({VS})")]);

    // A code the value set does not contain answers false, so the membership
    // answer tracks the host service's value set and is not a constant.
    let other = TestNode::resource(
        "Patient",
        vec![(
            "name".to_string(),
            TestNode::element(
                "HumanName",
                vec![("use".to_string(), TestNode::prim("code", "maiden"))],
            ),
        )],
    );
    assert_eq!(
        rendered(&host_eval(&Call::new(&expr).over(other).with_host(&host))),
        ["boolean:false"]
    );
}

// ---------------------------------------------------------------------------
// path.utility.016 -- memberOf with no resolvable value set
// ---------------------------------------------------------------------------

// covers: path.utility.016
#[test]
fn member_of_with_no_resolvable_value_set_yields_empty() {
    // `if (vs == null) { return new ArrayList<Base>(); }`
    // (FHIRPathEngine.java:4794-4796): empty, not false and not an error.
    let host = ConfigurableHost::default();
    let patient = TestNode::resource(
        "Patient",
        vec![("gender".to_string(), TestNode::prim("code", "male"))],
    );

    let values = host_eval(
        &Call::new("Patient.gender.memberOf('http://example.org/no-such-value-set')")
            .over(patient.clone())
            .with_host(&host),
    );
    assert!(
        values.is_empty(),
        "an unresolvable value set is empty: {:?}",
        rendered(&values)
    );
    assert_eq!(
        host.calls(),
        ["resolve_value_set(http://example.org/no-such-value-set)"]
    );

    // The server-facing host service's `resolveValueSet` is `return null;`, so
    // every `memberOf` through it is empty -- which is the configuration
    // `frevo-server` will run.
    let server = ConfigurableHost::server_facing();
    let values = host_eval(
        &Call::new("Patient.gender.memberOf('http://hl7.org/fhir/ValueSet/administrative-gender')")
            .over(patient)
            .with_host(&server),
    );
    assert!(values.is_empty(), "{:?}", rendered(&values));
}

// ---------------------------------------------------------------------------
// path.utility.017 -- resolve() on a local #id reference
// ---------------------------------------------------------------------------

// covers: path.utility.017
#[test]
fn resolve_resolves_a_local_reference_against_the_root_resources_contained() {
    // `funcResolve` (FHIRPathEngine.java:5413) is the only part of reference
    // resolution the engine does itself:
    //
    //     if (url != null && url.startsWith("#")) {
    //       String t = url.substring(1);
    //       Property p = context.rootResource.getChildByName("contained");
    //       ...
    //
    // The lookup is against **`context.rootResource`** (:5442) and not against
    // the focus resource, which is the half of this scenario that a plausible
    // implementation gets wrong.
    //
    // No live case in the pinned suite calls `resolve()` at all, so every
    // `resolve()` scenario is differential-only -- asserted here so the gap
    // stays visible.
    let (suite, _) = pinned();
    let resolve_cases = suite
        .cases()
        .filter(|c| c.expression.contains("resolve("))
        .count();
    assert_eq!(
        resolve_cases, 0,
        "no live case calls resolve(); if one appears these scenarios stop \
         being differential-only"
    );

    let patient = patient_with_contained();
    let values =
        host_eval(&Call::new("Patient.managingOrganization.resolve().name").over(patient.clone()));
    assert_eq!(rendered(&values), ["string:Acme"]);

    // A `#id` with no matching contained resource contributes nothing, and so
    // does a root resource with no `contained` child at all.
    let values =
        host_eval(&Call::new("Patient.managingOrganization.resolve()").over(no_contained_match()));
    assert!(values.is_empty(), "{:?}", rendered(&values));

    // A primitive focus is accepted, its value treated as the reference
    // (:5431-5433).
    let values = host_eval(
        &Call::new("Patient.managingOrganization.reference.resolve().name").over(patient.clone()),
    );
    assert_eq!(rendered(&values), ["string:Acme"]);

    // And the lookup is against the **root** resource even when evaluation
    // starts partway down: beginning at the `Reference` itself, `#org1` still
    // resolves, because `context.rootResource` is the `Patient`.
    let values = host_eval(
        &Call::new("$this.resolve().name")
            .over(patient)
            .at("Patient.managingOrganization"),
    );
    assert_eq!(rendered(&values), ["string:Acme"]);
}

/// A `Patient` whose reference names a contained resource that is not there.
fn no_contained_match() -> Value {
    TestNode::resource(
        "Patient",
        vec![(
            "managingOrganization".to_string(),
            TestNode::element(
                "Reference",
                vec![("reference".to_string(), TestNode::prim("string", "#absent"))],
            ),
        )],
    )
}

// ---------------------------------------------------------------------------
// path.utility.018 -- resolve() has no Bundle fullUrl resolution
// ---------------------------------------------------------------------------

// covers: path.utility.018
#[test]
fn resolve_has_no_bundle_full_url_resolution_of_its_own() {
    const URL: &str = "http://example.org/fhir/Patient/123";

    // `funcResolve` has two branches and no third: `#`-prefixed is resolved
    // against `contained`, and **everything else** goes to the host service
    // (FHIRPathEngine.java:5444 `} else if (hostServices != null) {`). There is
    // no Bundle-entry search anywhere in the function.
    let bundle = TestNode::resource(
        "Bundle",
        vec![
            (
                "entry".to_string(),
                TestNode::element(
                    "Bundle.entry",
                    vec![
                        ("fullUrl".to_string(), TestNode::prim("uri", URL)),
                        (
                            "resource".to_string(),
                            TestNode::resource(
                                "Patient",
                                vec![("id".to_string(), TestNode::prim("id", "123"))],
                            ),
                        ),
                    ],
                ),
            ),
            (
                "entry".to_string(),
                TestNode::element(
                    "Bundle.entry",
                    vec![(
                        "resource".to_string(),
                        TestNode::resource(
                            "Observation",
                            vec![(
                                "subject".to_string(),
                                TestNode::element(
                                    "Reference",
                                    vec![("reference".to_string(), TestNode::prim("string", URL))],
                                ),
                            )],
                        ),
                    )],
                ),
            ),
        ],
    );

    // With no host service: empty, even though the Bundle does carry a matching
    // `fullUrl`.
    let values = host_eval(
        &Call::new("Bundle.entry.resource.ofType(Observation).subject.resolve()")
            .over(bundle.clone()),
    );
    assert!(
        values.is_empty(),
        "the engine must not search the Bundle itself: {:?}",
        rendered(&values)
    );

    // With a host service: that service is asked for the URL and may answer
    // from the Bundle, so Bundle-internal resolution is the host's job.
    let host = ConfigurableHost {
        references: vec![(URL.to_string(), "Patient".to_string())],
        ..ConfigurableHost::default()
    };
    let values = host_eval(
        &Call::new("Bundle.entry.resource.ofType(Observation).subject.resolve().id")
            .over(bundle)
            .with_host(&host),
    );
    assert_eq!(rendered(&values), ["id:1"]);
    assert_eq!(
        host.calls(),
        [format!("resolve_reference({URL}, identifier=false)")]
    );
}

// ---------------------------------------------------------------------------
// path.utility.019 -- resolve() on an unresolvable reference
// ---------------------------------------------------------------------------

// covers: path.utility.019
#[test]
fn resolve_on_an_unresolvable_reference_yields_empty() {
    // With no host service the non-`#` branch is not taken at all (:5444), so
    // `res` stays null, nothing is added, and the result is empty rather than
    // an error.
    let patient = TestNode::resource(
        "Patient",
        vec![(
            "managingOrganization".to_string(),
            TestNode::element(
                "Reference",
                vec![(
                    "reference".to_string(),
                    TestNode::prim("string", "http://example.org/fhir/Organization/999"),
                )],
            ),
        )],
    );

    let values =
        host_eval(&Call::new("Patient.managingOrganization.resolve()").over(patient.clone()));
    assert!(values.is_empty(), "{:?}", rendered(&values));

    // And with a host service that resolves nothing, also empty: `Ok(None)` is
    // not an error.
    let host = ConfigurableHost::default();
    let values = host_eval(
        &Call::new("Patient.managingOrganization.resolve()")
            .over(patient)
            .with_host(&host),
    );
    assert!(values.is_empty(), "{:?}", rendered(&values));
    assert_eq!(host.calls().len(), 1, "the host service was still asked");
}

// ---------------------------------------------------------------------------
// path.utility.020 -- resolve() delegates and swallows host errors
// ---------------------------------------------------------------------------

// covers: path.utility.020
#[test]
fn resolve_delegates_an_external_reference_and_swallows_the_host_services_errors() {
    let reference = TestNode::resource(
        "Patient",
        vec![(
            "managingOrganization".to_string(),
            TestNode::element(
                "Reference",
                vec![(
                    "reference".to_string(),
                    TestNode::prim("string", "Patient/123"),
                )],
            ),
        )],
    );

    // Delegation: the host service is asked for exactly the reference text.
    let host = ConfigurableHost {
        references: vec![("Patient/123".to_string(), "Patient".to_string())],
        ..ConfigurableHost::default()
    };
    let values = host_eval(
        &Call::new("Patient.managingOrganization.resolve().id")
            .over(reference.clone())
            .with_host(&host),
    );
    assert_eq!(rendered(&values), ["id:1"]);
    assert_eq!(
        host.calls(),
        ["resolve_reference(Patient/123, identifier=false)"]
    );

    // And the swallow:
    //
    //     try { res = hostServices.resolveReference(...); }
    //     catch (Exception e) { res = null; }
    //
    // (FHIRPathEngine.java:5445-5449). This is the one place the engine
    // deliberately converts an error into an absent value, which is why it is
    // specified rather than left to `path.errors.005`.
    let raising = ConfigurableHost {
        raising_references: vec!["Patient/123".to_string()],
        ..ConfigurableHost::default()
    };
    let values = host_eval(
        &Call::new("Patient.managingOrganization.resolve()")
            .over(reference.clone())
            .with_host(&raising),
    );
    assert!(
        values.is_empty(),
        "a raising host service contributes nothing: {:?}",
        rendered(&values)
    );
    assert_eq!(raising.calls().len(), 1, "it was asked, and it raised");

    // Evaluation continues afterwards, which is the other half of the swallow:
    // the surrounding expression still produces its answer.
    let values = host_eval(
        &Call::new("Patient.managingOrganization.resolve().exists()")
            .over(reference)
            .with_host(&raising),
    );
    assert_eq!(rendered(&values), ["boolean:false"]);
}

// ---------------------------------------------------------------------------
// path.utility.021 -- the appointment and explanation-of-benefit cases
// ---------------------------------------------------------------------------

// covers: path.utility.021
#[test]
fn the_appointment_and_explanation_of_benefit_cases_hold() {
    // The selector is the `inputfile`, not a group: these four files are read
    // by cases scattered across several groups, and two of them are the JSON
    // inputs that also grade `path.oracle.004`.
    let (suite, report) = pinned();
    let files = [
        "appointment-examplereq.json",
        "explanationofbenefit-example.json",
        "codesystem-example.xml",
        "valueset-example-expansion.xml",
    ];

    let mut total = 0usize;
    for file in files {
        let cases: Vec<_> = suite
            .cases()
            .filter(|c| c.inputfile.as_deref() == Some(file))
            .collect();
        assert!(
            !cases.is_empty(),
            "{file} must be referenced by at least one case"
        );
        total += cases.len();
        for case in cases {
            let r = report
                .get(&case.address.to_string())
                .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
            assert_eq!(
                r.status,
                CaseStatus::Passed,
                "{} ({file}): {:?} is {} -- {}",
                r.address,
                r.expression,
                r.status.as_str(),
                r.reason
            );
        }
    }

    // The count is stated so that a re-pin which adds a case to one of these
    // files is visible rather than silently ungraded.
    assert_eq!(
        total, 12,
        "the four inputs are read by twelve cases between them: three, one, one and seven"
    );

    // Both JSON inputs really do load as JSON, which is what makes this a
    // cross-form assertion and not just four more cases.
    assert!(!load_input("appointment-examplereq.json").is_xml());
    assert!(!load_input("explanationofbenefit-example.json").is_xml());
    assert!(load_input("codesystem-example.xml").is_xml());
}

// ---------------------------------------------------------------------------
// path.utility.022 -- conformsTo with no host service at all
// ---------------------------------------------------------------------------

// covers: path.utility.022
#[test]
fn conforms_to_with_no_host_service_is_an_execution_error() {
    // `funcConformsTo`'s first two lines (FHIRPathEngine.java:5973-5975):
    //
    //     if (hostServices == null) {
    //       throw makeException(expr, FHIRPATH_HO_HOST_SERVICES, "conformsTo");
    //     }
    //
    // The raise comes **before** the argument is even evaluated, so the URL is
    // irrelevant. "No host service" is not the same state as "a host service
    // that says no" (`path.utility.014`).
    let patient = patient_with_contained();
    let message = host_eval_failure(
        &Call::new("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')")
            .over(patient.clone()),
    );
    assert!(
        message.contains("conformsTo"),
        "the error must name conformsTo: {message}"
    );

    // Even a nonsense URL fails the same way, which is what shows the raise is
    // before the argument.
    host_eval_failure(&Call::new("conformsTo('')").over(patient.clone()));

    // Once a host service **is** set, a focus of other than one item answers
    // `false` rather than failing (:5976-5978).
    let host = ConfigurableHost::suite();
    let multi = TestNode::resource(
        "Patient",
        vec![
            ("name".to_string(), TestNode::element("HumanName", vec![])),
            ("name".to_string(), TestNode::element("HumanName", vec![])),
        ],
    );
    let values = host_eval(
        &Call::new("Patient.name.conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')")
            .over(multi)
            .with_host(&host),
    );
    assert_eq!(rendered(&values), ["boolean:false"]);

    // No live case grades the unset path: every suite case runs with the test
    // host service installed, which is why this scenario is differential-only.
    assert!(EngineConfig::default().install_test_host_services);
}

// ---------------------------------------------------------------------------
// path.utility.023 -- extension(url) also returns modifier extensions
// ---------------------------------------------------------------------------

// covers: path.utility.023
#[test]
fn extension_also_returns_modifier_extensions() {
    const URL: &str = "http://example.org/x";

    // `funcExtension` (FHIRPathEngine.java:5464-5471) collects **both**:
    //
    //     getChildrenByName(item, "extension", ext);
    //     getChildrenByName(item, "modifierExtension", ext);
    //
    // and only then filters by URL. So a URL carried in `modifierExtension` is
    // returned by `extension(url)`. `DEV-0007` records the divergence from the
    // FHIR definition, which is `extension.where(url=...)`.
    let patient = TestNode::resource(
        "Patient",
        vec![(
            "modifierExtension".to_string(),
            TestNode::element(
                "Extension",
                vec![
                    ("url".to_string(), TestNode::prim("uri", URL)),
                    ("value".to_string(), TestNode::prim("string", "modifier")),
                ],
            ),
        )],
    );

    let expr = format!("Patient.extension('{URL}').value");
    let values = host_eval(&Call::new(&expr).over(patient));
    assert_eq!(
        rendered(&values),
        ["string:modifier"],
        "a modifierExtension is returned by extension(url)"
    );

    // Both are collected, in that order: `extension` first, then
    // `modifierExtension`.
    let both = TestNode::resource(
        "Patient",
        vec![
            (
                "extension".to_string(),
                TestNode::element(
                    "Extension",
                    vec![
                        ("url".to_string(), TestNode::prim("uri", URL)),
                        ("value".to_string(), TestNode::prim("string", "plain")),
                    ],
                ),
            ),
            (
                "modifierExtension".to_string(),
                TestNode::element(
                    "Extension",
                    vec![
                        ("url".to_string(), TestNode::prim("uri", URL)),
                        ("value".to_string(), TestNode::prim("string", "modifier")),
                    ],
                ),
            ),
        ],
    );
    let values = host_eval(&Call::new(&expr).over(both));
    assert_eq!(rendered(&values), ["string:plain", "string:modifier"]);

    // No live case grades it: the `testExtension` group has no
    // modifier-extension case, which is asserted so the gap stays visible.
    let (suite, _) = pinned();
    let modifier_cases = suite
        .cases()
        .filter(|c| c.expression.contains("modifierExtension"))
        .count();
    assert_eq!(modifier_cases, 0, "differential-only");
}

// ---------------------------------------------------------------------------
// path.utility.024 -- memberOf as a binary operator folds the left collection
// ---------------------------------------------------------------------------

// covers: path.utility.024
#[test]
fn member_of_as_a_binary_operator_folds_the_whole_left_collection() {
    const VS: &str = "http://hl7.org/fhir/ValueSet/name-use";

    // `opMemberOf` (FHIRPathEngine.java:2602) iterates the **whole** left
    // collection and ORs the outcomes into one boolean:
    //
    //     boolean ans = false;
    //     for (Base l : left) { ... ans = true; ... }
    //     return makeBoolean(ans);
    //
    // So a collection of three uses answers true when any one validates.
    let host = ConfigurableHost {
        value_sets: vec![(VS.to_string(), vec!["usual".to_string()])],
        ..ConfigurableHost::default()
    };
    let patient = TestNode::resource(
        "Patient",
        vec![
            (
                "name".to_string(),
                TestNode::element(
                    "HumanName",
                    vec![("use".to_string(), TestNode::prim("code", "official"))],
                ),
            ),
            (
                "name".to_string(),
                TestNode::element(
                    "HumanName",
                    vec![("use".to_string(), TestNode::prim("code", "usual"))],
                ),
            ),
        ],
    );

    let expr = format!("Patient.name.use memberOf '{VS}'");
    let values = host_eval(&Call::new(&expr).over(patient.clone()).with_host(&host));
    assert_eq!(
        rendered(&values),
        ["boolean:true"],
        "one of the two uses validates, so the fold is true"
    );

    // The function form is **not** the same: `funcMemberOf` (:4788-4790)
    // returns empty unless both the focus and the argument are singletons, so
    // the same two-item focus answers empty rather than true.
    let fn_expr = format!("Patient.name.use.memberOf('{VS}')");
    let values = host_eval(&Call::new(&fn_expr).over(patient).with_host(&host));
    assert!(
        values.is_empty(),
        "the function form requires a singleton focus: {:?}",
        rendered(&values)
    );

    // Neither form is graded by a live case, so both are differential-only.
    let (suite, _) = pinned();
    assert_eq!(
        suite
            .cases()
            .filter(|c| c.expression.contains("memberOf"))
            .count(),
        0
    );
}

// ---------------------------------------------------------------------------
// path.utility.025 -- memberOf falls back to the worker context
// ---------------------------------------------------------------------------

// covers: path.utility.025
#[test]
fn member_of_falls_back_to_the_worker_context_when_no_host_service_is_set() {
    const VS: &str = "http://hl7.org/fhir/ValueSet/administrative-gender";

    // Both forms read
    //
    //     hostServices != null ? hostServices.resolveValueSet(...)
    //                          : worker.fetchResource(ValueSet.class, url)
    //
    // (`funcMemberOf` :4793, `opMemberOf` :2605), so with no host service the
    // lookup goes to the engine's own worker context rather than failing. The
    // engine must therefore answer rather than raise -- which is what this
    // asserts, with no host service installed.
    let patient = TestNode::resource(
        "Patient",
        vec![("gender".to_string(), TestNode::prim("code", "male"))],
    );
    let expr = format!("Patient.gender.memberOf('{VS}')");
    let values = host_eval(&Call::new(&expr).over(patient.clone()));
    assert_eq!(
        rendered(&values),
        ["boolean:true"],
        "the worker context knows the administrative-gender value set"
    );

    // When the value set cannot be resolved at all, the two forms differ: the
    // function form returns empty (:4795) and the operator form returns
    // `false`, because `ans` was initialised false and the loop never ran
    // (:2603, :2627).
    let missing = "http://example.org/no-such-value-set";
    let fn_values = host_eval(
        &Call::new(&format!("Patient.gender.memberOf('{missing}')")).over(patient.clone()),
    );
    assert!(
        fn_values.is_empty(),
        "the function form is empty: {:?}",
        rendered(&fn_values)
    );
    let op_values =
        host_eval(&Call::new(&format!("Patient.gender memberOf '{missing}'")).over(patient));
    assert_eq!(
        rendered(&op_values),
        ["boolean:false"],
        "the operator form is false, not empty"
    );
}

// ---------------------------------------------------------------------------
// path.utility.026 -- memberOf returns empty for an unvalidatable focus
// ---------------------------------------------------------------------------

// covers: path.utility.026
#[test]
fn member_of_returns_empty_for_a_focus_whose_type_it_cannot_validate() {
    const VS: &str = "http://example.org/ValueSet/anything";

    // `funcMemberOf` (FHIRPathEngine.java:4798-4808) has exactly three arms and
    // a final `else`:
    //
    //   code | string | uri -> validateCode(options.withGuessSystem(), ...)
    //   Coding             -> validateCode(options, ...)
    //   CodeableConcept    -> validateCode(options, ...)
    //   anything else      -> new ArrayList<Base>()
    //
    // So a boolean focus is **empty**, not `false`: the function cannot say
    // "not a member" about a value it cannot turn into a code.
    let host = ConfigurableHost {
        value_sets: vec![(VS.to_string(), vec!["true".to_string()])],
        ..ConfigurableHost::default()
    };
    let patient = TestNode::resource(
        "Patient",
        vec![
            ("active".to_string(), TestNode::prim("boolean", "true")),
            ("gender".to_string(), TestNode::prim("code", "male")),
        ],
    );

    let values = host_eval(
        &Call::new(&format!("Patient.active.memberOf('{VS}')"))
            .over(patient.clone())
            .with_host(&host),
    );
    assert!(
        values.is_empty(),
        "a boolean focus is empty, not false: {:?}",
        rendered(&values)
    );

    // A `code` focus **is** validated against the same value set, so the empty
    // answer is about the type and not about the value set.
    let values = host_eval(
        &Call::new(&format!("Patient.gender.memberOf('{VS}')"))
            .over(patient)
            .with_host(&host),
    );
    assert_eq!(rendered(&values), ["boolean:false"]);

    // The three guessing arms and the two non-guessing ones are distinguished
    // in the source; the distinction is recorded here because it is observable
    // only through a terminology server, which this test does not have.
    assert_eq!(
        ["code", "string", "uri"].len() + ["Coding", "CodeableConcept"].len(),
        5,
        "five validatable types: three with system guessing, two without"
    );
}

// ---------------------------------------------------------------------------
// path.utility.027 -- a host service may supply a custom function
// ---------------------------------------------------------------------------

// covers: path.utility.027
#[test]
fn a_host_service_may_supply_a_custom_function() {
    // The parser consults the host service only after `Function.fromCode` has
    // returned null (FHIRPathEngine.java:1158-1166), and takes the arity from
    // the returned `FunctionDetails`. The check pass then asks
    // `checkFunction` for the result type and evaluation calls
    // `executeFunction`.
    let host = ConfigurableHost {
        functions: vec![FunctionDetails {
            name: "myFunction".to_string(),
            min_params: 1,
            max_params: 2,
        }],
        type_params: vec![("myFunction".to_string(), 1)],
        ..ConfigurableHost::default()
    };

    let mut engine = host_engine_under_test();

    // The parser takes the arity from the declaration: one and two parameters
    // parse, zero and three do not.
    engine
        .parse("myFunction('a')", Some(&host))
        .expect("one parameter is within the declared arity");
    engine
        .parse("myFunction('a', 'b')", Some(&host))
        .expect("two parameters are within the declared arity");
    for bad in ["myFunction()", "myFunction('a', 'b', 'c')"] {
        match engine.parse(bad, Some(&host)) {
            Err(EngineError::Diagnostic { .. }) => {}
            Err(e) => panic!("{bad} must be a parse diagnostic, not: {e}"),
            Ok(()) => panic!("{bad} is outside the declared arity and must not parse"),
        }
    }

    // With no host service the same expression does not parse at all, which is
    // why the 115 `documentFunctions` cases fail at the **parse** stage
    // (`path.oracle.014`).
    match engine.parse("myFunction('a')", None) {
        Err(EngineError::Diagnostic { .. }) => {}
        Err(e) => panic!("an unknown function must be a parse diagnostic, not: {e}"),
        Ok(()) => panic!("an unknown function must not parse with no host service"),
    }

    // The check pass asks the host service for the result type.
    let checked = engine
        .check("myFunction('a')", "Patient", Some(&host))
        .expect("the check pass must ask the host service");
    assert_eq!(checked.names(), ["string"]);
    assert!(
        host.calls()
            .iter()
            .any(|c| c == "check_function(myFunction)"),
        "the check pass must call check_function: {:?}",
        host.calls()
    );

    // Evaluation calls the host service with the focus and the evaluated
    // parameters.
    let patient = patient_with_contained();
    let values = host_eval(&Call::new("myFunction('a')").over(patient).with_host(&host));
    assert_eq!(rendered(&values), ["string:myFunction:1"]);
    assert!(
        host.calls()
            .iter()
            .any(|c| c.starts_with("execute_function(myFunction")),
        "{:?}",
        host.calls()
    );

    // And `param_is_type` is honoured: parameter 1 is declared a type name, so
    // it is passed unevaluated.
    assert!(host.param_is_type("myFunction", 1));
    assert!(!host.param_is_type("myFunction", 0));

    // No live case grades any of this, so it is differential-only.
    let (suite, _) = pinned();
    assert_eq!(
        suite
            .cases()
            .filter(|c| c.expression.contains("myFunction"))
            .count(),
        0
    );
}

// ---------------------------------------------------------------------------
// path.utility.028 -- a host service may intercept trace
// ---------------------------------------------------------------------------

// covers: path.utility.028
#[test]
fn a_host_service_may_intercept_trace_and_suppress_the_engines_log() {
    // `log(String, List<Base>)` (FHIRPathEngine.java:927):
    //
    //     if (hostServices == null || !hostServices.log(name, contents)) {
    //       ... append to the engine's own buffer ...
    //     }
    //
    // So a host service returning **true** suppresses the buffer entirely,
    // while **false** lets the engine append as well. The hook has exactly two
    // callers, both in `funcTrace` (:5628 and :5630).
    let patient = patient_with_contained();

    // true: the hook is called, the expression still passes its input through,
    // and the engine's buffer stays empty.
    let suppressing = ConfigurableHost {
        log_answer: true,
        ..ConfigurableHost::default()
    };
    let mut engine = host_engine_under_test();
    let values = engine
        .evaluate(
            &Call::new("Patient.contained.trace('n')")
                .over(patient.clone())
                .with_host(&suppressing),
        )
        .expect("trace must still return its input");
    assert_eq!(rendered(&values), ["Organization:<element>"]);
    assert!(
        suppressing.calls().iter().any(|c| c.starts_with("log(n,")),
        "the hook must be called with the trace name: {:?}",
        suppressing.calls()
    );
    assert!(
        !engine.has_log(),
        "a host service returning true suppresses the engine's own log"
    );
    assert_eq!(engine.take_log(), "");

    // false: the hook is called **and** the engine appends, so the entry is
    // observable both ways.
    let passing = ConfigurableHost {
        log_answer: false,
        ..ConfigurableHost::default()
    };
    let mut engine = host_engine_under_test();
    engine
        .evaluate(
            &Call::new("Patient.contained.trace('n')")
                .over(patient.clone())
                .with_host(&passing),
        )
        .expect("evaluates");
    assert!(passing.calls().iter().any(|c| c.starts_with("log(n,")));
    assert!(
        engine.has_log(),
        "a host service returning false leaves the engine's log in place"
    );
    assert!(
        engine.take_log().starts_with("n: "),
        "the entry is recorded under the trace name"
    );

    // With no host service the engine appends with no hook call at all, which
    // is the configuration `path.utility.001` and the oracle harness run in.
    let mut engine = host_engine_under_test();
    engine
        .evaluate(&Call::new("Patient.contained.trace('n')").over(patient))
        .expect("evaluates");
    assert!(engine.has_log());
    assert!(engine.take_log().starts_with("n: "));

    // The hook is reached only from `trace`: the math `log()` function has no
    // connection to it.
    let unrelated = ConfigurableHost::default();
    let mut engine = host_engine_under_test();
    engine
        .evaluate(&Call::new("16.log(2)").with_host(&unrelated))
        .expect("the math log evaluates");
    assert!(
        unrelated.calls().is_empty(),
        "the math log() must not call the host service's log hook: {:?}",
        unrelated.calls()
    );
    assert!(!engine.has_log());

    // With this scenario the ten-method host-service surface is fully
    // specified, which is asserted as a count so a method added later needs a
    // scenario too.
    assert_eq!(
        [
            "resolve_constant",
            "resolve_constant_type",
            "resolve_reference",
            "conforms_to_profile",
            "resolve_value_set",
            "resolve_function",
            "check_function",
            "execute_function",
            "param_is_type",
            "log",
        ]
        .len(),
        10
    );
}
