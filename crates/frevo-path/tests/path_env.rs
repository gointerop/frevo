//! Environment variables: `path.env.001` through `path.env.017`.
//!
//! ```text
//! cargo test -p frevo-path --test path_env
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. The internally-resolved constants
//! are graded from the suite (`testVariables`, `testExtension`,
//! `isSemanticChecks`); everything about the **host service** is driven through
//! the crate's published `HostServices` trait, which exists today, with the
//! engine still unbound.
//!
//! `resolveConstant(ExecutionContext, String, ExpressionNode, mode)` at
//! FHIRPathEngine.java:1754 is the whole of the internal table:
//!
//!   %sct :1755   %loinc :1757   %ucum :1759   %resource :1761
//!   %rootResource :1765   %context :1769   %us-zip :1771
//!   %`vs-` :1773   %`cs-` :1775   %`ext-` :1777
//!   no host service -> FHIRPATH_UNKNOWN_CONSTANT :1782
//!   host service    -> resolveConstant(..., mode) :1784
//!
//! The four `ConstantMode` sites, which `path.env.015` through `path.env.017`
//! grade, are `EXPLICIT` at :1497 and :1610, `NOVALUE` at :1586 (exactly one
//! site), `IMPLICIT_BEFORE` at :3186 and `IMPLICIT_AFTER` at :3214.
//! `DEV-0010` records the implicit modes' divergence from the FHIRPath grammar.
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::{Mutex, OnceLock};

use frevo_path::{ConstantMode, FunctionDetails, HostError, HostServices, TypeSet, Value};

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// A recording host service, over the crate's published trait
// ---------------------------------------------------------------------------

/// Every question the engine asked this host service, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ask {
    Constant(String, ConstantMode),
    ConstantType(String, ConstantMode),
}

/// A host service that answers from a table and records what it was asked.
///
/// Implementing the real `frevo_path::HostServices` is deliberate: the shape of
/// the trait is part of what these scenarios specify, so a seam invented here
/// would grade nothing.
#[derive(Debug, Default)]
struct RecordingHost {
    /// `name` (without the `%`) to the string values answered for it.
    constants: Vec<(String, Vec<String>)>,
    asked: Mutex<Vec<Ask>>,
}

impl RecordingHost {
    fn new(constants: &[(&str, &[&str])]) -> Self {
        RecordingHost {
            constants: constants
                .iter()
                .map(|(n, vs)| {
                    (
                        (*n).to_string(),
                        vs.iter().map(|v| (*v).to_string()).collect(),
                    )
                })
                .collect(),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// A host service that owns no name at all, which is what every host
    /// service must answer for a name it does not own (`path.env.016`).
    fn answering_nothing() -> Self {
        RecordingHost::new(&[])
    }

    fn asked(&self) -> Vec<Ask> {
        self.asked
            .lock()
            .expect("the recorder is not poisoned")
            .clone()
    }

    fn modes_for(&self, name: &str) -> Vec<ConstantMode> {
        self.asked()
            .into_iter()
            .filter_map(|a| match a {
                Ask::Constant(n, m) | Ask::ConstantType(n, m) if n == name => Some(m),
                _ => None,
            })
            .collect()
    }
}

impl HostServices for RecordingHost {
    fn resolve_constant(&self, name: &str, mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        self.asked
            .lock()
            .expect("the recorder is not poisoned")
            .push(Ask::Constant(name.to_string(), mode));
        Ok(self
            .constants
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, vs)| vs.iter().map(|v| Value::String(v.clone())).collect())
            .unwrap_or_default())
    }

    fn resolve_constant_type(
        &self,
        name: &str,
        mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        self.asked
            .lock()
            .expect("the recorder is not poisoned")
            .push(Ask::ConstantType(name.to_string(), mode));
        Ok(self
            .constants
            .iter()
            .find(|(n, _)| n == name)
            .map(|_| TypeSet::of("string")))
    }

    fn resolve_reference(
        &self,
        _url: &str,
        _identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        Ok(None)
    }

    fn conforms_to_profile(&self, _item: &Value, _url: &str) -> Result<bool, HostError> {
        Ok(false)
    }

    fn resolve_value_set(&self, _url: &str) -> Result<Option<Value>, HostError> {
        Ok(None)
    }

    fn resolve_function(&self, _name: &str) -> Option<FunctionDetails> {
        None
    }

    fn check_function(
        &self,
        name: &str,
        _focus: &TypeSet,
        _params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        Err(HostError::new(format!("no such function: {name}")))
    }

    fn execute_function(
        &self,
        name: &str,
        _focus: &[Value],
        _params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        Err(HostError::new(format!("no such function: {name}")))
    }

    fn param_is_type(&self, _name: &str, _index: usize) -> bool {
        false
    }

    fn log(&self, _name: &str, _focus: &[Value]) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// The host-aware engine seam
//
// The harness's `SuiteEngine` cannot express a caller-supplied host service, a
// null-or-not application context, or a starting context inside the input, and
// all three decide these scenarios. So they are driven through this seam, whose
// unbound binding answers `NotImplemented` -- never empty and never a
// diagnostic.
// ---------------------------------------------------------------------------

/// One evaluation, with everything the four `ConstantMode` sites depend on.
struct EvalRequest<'a> {
    expr: &'a str,
    input: Option<&'a SuiteInput>,
    /// A FHIRPath expression selecting the node evaluation starts from, as the
    /// suite's `context` attribute does. `None` starts at the resource root.
    context_path: Option<&'a str>,
    host: Option<&'a dyn HostServices>,
    /// Whether a non-null application context is passed. Both implicit modes
    /// are gated on this and **not** on the host service
    /// (FHIRPathEngine.java:3184, :3213).
    app_context: bool,
}

impl<'a> EvalRequest<'a> {
    fn new(expr: &'a str) -> Self {
        EvalRequest {
            expr,
            input: None,
            context_path: None,
            host: None,
            app_context: false,
        }
    }

    fn over(mut self, input: &'a SuiteInput) -> Self {
        self.input = Some(input);
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

    fn with_app_context(mut self) -> Self {
        self.app_context = true;
        self
    }
}

trait HostAwareEngine {
    /// Evaluate, returning the engine's own owned values.
    fn evaluate(&mut self, req: &EvalRequest<'_>) -> Result<Vec<Value>, EngineError>;

    /// Run the **check** pass, which is the only pass that reaches
    /// `resolve_constant_type` and so the only one that can produce
    /// `ConstantMode::NoValue` (FHIRPathEngine.java:1586).
    ///
    /// `context_type` is `None` for "no focus type at all", which is the state
    /// the 90 suite cases with no `inputfile` are checked in.
    fn check(
        &mut self,
        expr: &str,
        context_type: Option<&str>,
        host: Option<&dyn HostServices>,
    ) -> Result<TypeSet, EngineError>;
}

fn host_engine_under_test() -> Box<dyn HostAwareEngine> {
    Box::new(UnboundHostEngine)
}

struct UnboundHostEngine;

impl HostAwareEngine for UnboundHostEngine {
    fn evaluate(&mut self, _req: &EvalRequest<'_>) -> Result<Vec<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn check(
        &mut self,
        _expr: &str,
        _context_type: Option<&str>,
        _host: Option<&dyn HostServices>,
    ) -> Result<TypeSet, EngineError> {
        Err(EngineError::NotImplemented)
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

/// An engine configured as the suite configures it.
fn fresh_engine() -> Box<dyn SuiteEngine> {
    engine_with(EngineConfig::default())
}

/// An engine with **no host service installed**, which is a different state
/// from a host service that answers nothing (`path.utility.022`).
fn engine_without_host() -> Box<dyn SuiteEngine> {
    engine_with(EngineConfig {
        install_test_host_services: false,
        ..EngineConfig::default()
    })
}

fn engine_with(config: EngineConfig) -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&config)
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

/// `expr` must fail at **evaluation** with a real diagnostic, on an engine
/// built by `build`.
fn expect_evaluation_failure_on(
    build: fn() -> Box<dyn SuiteEngine>,
    expr: &str,
    inp: Option<&SuiteInput>,
) -> String {
    let mut engine = build();
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

fn string(value: &str) -> Vec<ResultItem> {
    primitive("string", value)
}

/// One owned value, rendered as `type:lexical`.
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

/// Evaluate through the host-aware seam, requiring success.
fn host_eval(req: &EvalRequest<'_>) -> Vec<Value> {
    let mut engine = host_engine_under_test();
    engine
        .evaluate(req)
        .unwrap_or_else(|e| panic!("{:?} must evaluate: {e}", req.expr))
}

/// Evaluate through the host-aware seam, requiring a real diagnostic.
fn host_eval_failure(req: &EvalRequest<'_>) -> String {
    let mut engine = host_engine_under_test();
    match engine.evaluate(req) {
        Ok(values) => panic!(
            "{:?} must fail, but it returned {:?}",
            req.expr,
            rendered(&values)
        ),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{:?} must fail with a diagnostic, not: {e}", req.expr),
    }
}

// ---------------------------------------------------------------------------
// path.env.001 -- %sct, %loinc and %ucum
// ---------------------------------------------------------------------------

// covers: path.env.001
#[test]
fn sct_loinc_and_ucum_are_the_canonical_system_urls() {
    expect_group("testVariables", 4);

    assert_eq!(
        eval_patient("%sct = 'http://snomed.info/sct'"),
        boolean(true)
    );
    assert_eq!(eval_patient("%loinc = 'http://loinc.org'"), boolean(true));
    assert_eq!(
        eval_patient("%ucum = 'http://unitsofmeasure.org'"),
        boolean(true)
    );

    // Each resolves to a `string`, not to a `uri` (`new StringType(...)` at
    // FHIRPathEngine.java:1756, :1758 and :1760), which is what lets the
    // comparison with a string literal be true.
    assert_eq!(eval_patient("%sct"), string("http://snomed.info/sct"));
    assert_eq!(eval_patient("%loinc"), string("http://loinc.org"));
    assert_eq!(eval_patient("%ucum"), string("http://unitsofmeasure.org"));

    // These three are resolved internally, so they are answered with no host
    // service installed at all.
    let mut engine = engine_without_host();
    let inp = load_input("patient-example.xml");
    let context = context_for(Some(&inp));
    let parsed = engine.parse("%sct").expect("%sct parses");
    let values = engine
        .evaluate(parsed, Some(&inp), &context, StrictnessFlags::EVALUATE)
        .expect("%sct needs no host service");
    assert_eq!(values, string("http://snomed.info/sct"));
}

// ---------------------------------------------------------------------------
// path.env.002 -- the value-set shorthand
// ---------------------------------------------------------------------------

// covers: path.env.002
#[test]
fn a_value_set_shorthand_expands_to_an_hl7_valueset_url() {
    // `s.startsWith("%`vs-")` at FHIRPathEngine.java:1773 prepends
    // `http://hl7.org/fhir/ValueSet/` to whatever is between the backticks,
    // after the `vs-` prefix.
    assert_eq!(
        eval_patient(
            "%`vs-administrative-gender` = 'http://hl7.org/fhir/ValueSet/administrative-gender'"
        ),
        boolean(true)
    );
    expect_case("testVariables", "testVariables4");

    assert_eq!(
        eval_patient("%`vs-administrative-gender`"),
        string("http://hl7.org/fhir/ValueSet/administrative-gender")
    );

    // The name inside the backticks is taken verbatim, so a different name
    // gives a different URL rather than a lookup failure.
    assert_eq!(
        eval_patient("%`vs-no-such-value-set`"),
        string("http://hl7.org/fhir/ValueSet/no-such-value-set")
    );
}

// ---------------------------------------------------------------------------
// path.env.003 -- the code-system shorthand
// ---------------------------------------------------------------------------

// covers: path.env.003
#[test]
fn a_code_system_shorthand_expands_to_an_hl7_base_url() {
    // FHIRPathEngine.java:1775: `http://hl7.org/fhir/` plus the name inside the
    // backticks -- with **no** `CodeSystem/` path segment, which is what
    // distinguishes it from the value-set and extension forms.
    assert_eq!(
        eval_patient("%`cs-administrative-gender`"),
        string("http://hl7.org/fhir/administrative-gender")
    );
    assert_eq!(
        eval_patient("%`cs-administrative-gender` = 'http://hl7.org/fhir/administrative-gender'"),
        boolean(true)
    );

    // Not the ValueSet URL, which is the mistake the three prefixes invite.
    assert_ne!(
        eval_patient("%`cs-administrative-gender`"),
        string("http://hl7.org/fhir/ValueSet/administrative-gender")
    );
}

// ---------------------------------------------------------------------------
// path.env.004 -- the extension shorthand
// ---------------------------------------------------------------------------

// covers: path.env.004
#[test]
fn an_extension_shorthand_expands_to_an_hl7_structure_definition_url() {
    // FHIRPathEngine.java:1777: `http://hl7.org/fhir/StructureDefinition/` plus
    // the name, with the prefix `ext-` being six characters rather than five.
    assert_eq!(
        eval_patient("%`ext-patient-birthTime`"),
        string("http://hl7.org/fhir/StructureDefinition/patient-birthTime")
    );

    // `testExtension/testExtension2` grades it by passing the expansion to
    // `extension(url)` and finding the real extension, which is a stronger
    // statement than the string comparison alone.
    expect_case("testExtension", "testExtension2");
    assert_eq!(
        eval_patient("Patient.birthDate.extension(%`ext-patient-birthTime`).exists()"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.env.005 -- %us-zip
// ---------------------------------------------------------------------------

// covers: path.env.005
#[test]
fn us_zip_is_the_engines_built_in_zip_pattern() {
    // FHIRPathEngine.java:1771, verbatim.
    assert_eq!(eval_patient("%us-zip"), string("[0-9]{5}(-[0-9]{4}){0,1}"));

    // It is a pattern meant for `matches`, so it is asserted in use as well as
    // by its text.
    assert_eq!(eval_patient("'95014'.matches(%us-zip)"), boolean(true));
    assert_eq!(eval_patient("'95014-1234'.matches(%us-zip)"), boolean(true));
    assert_eq!(eval_patient("'abcde'.matches(%us-zip)"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.env.006 -- %resource is the resource the focus is inside
// ---------------------------------------------------------------------------

// covers: path.env.006
#[test]
fn resource_is_the_resource_the_focus_is_inside() {
    // The grading selector is these six `isSemanticChecks` cases, which are the
    // only live cases that reference `%resource`. The `testDollarResource`
    // group the delta's earlier wording named is commented out of the pinned
    // suite (`tests-fhir-r4.xml:1576-1582`) and is not one of the 1,191.
    let six = [
        "isGuardAndOtherImplies",
        "isGuardOtherAndImplies",
        "isGuardOrOtherImpliesBad",
        "isGuardOrOtherImpliesNoParensBad",
        "isGuardOrInnerImplies",
        "isGuardOrInnerImpliesBad",
    ];
    let (suite, _) = pinned();
    for name in six {
        let case = suite
            .case("isSemanticChecks", name)
            .unwrap_or_else(|| panic!("isSemanticChecks/{name} must resolve to one case"));
        assert!(
            case.expression.contains("%resource.status"),
            "{name} must be one of the %resource cases: {:?}",
            case.expression
        );
        assert_eq!(case.context.as_deref(), Some("Observation.effective"));
        expect_case("isSemanticChecks", name);
    }

    // And the property those cases depend on, stated directly: with an
    // `Observation.effective` as the focus, `%resource` is the enclosing
    // `Observation` (FHIRPathEngine.java:1761).
    let observation = load_input("observation-example.xml");
    let values = host_eval(
        &EvalRequest::new("%resource.status")
            .over(&observation)
            .at("Observation.effective"),
    );
    assert_eq!(rendered(&values), ["code:final"]);
}

// ---------------------------------------------------------------------------
// path.env.007 -- %resource with no focus resource
// ---------------------------------------------------------------------------

// covers: path.env.007
#[test]
fn resource_with_no_focus_resource_is_an_execution_error() {
    // `if (context.focusResource == null) { throw makeException(expr,
    // FHIRPATH_CANNOT_USE, "%resource", "no focus resource"); }`
    // (FHIRPathEngine.java:1762-1763). The message names the constant, which is
    // what the scenario asks for.
    let message = expect_evaluation_failure_on(fresh_engine, "%resource", None);
    assert!(
        message.contains("%resource"),
        "the error must name %resource: {message}"
    );

    // Not empty: an absent focus resource is a failure and not a missing value.
    let message = expect_evaluation_failure_on(fresh_engine, "%resource.status", None);
    assert!(message.contains("%resource"), "{message}");

    // The same through the host-aware seam, with no input at all: the guard is
    // on the focus resource and not on the presence of a host service.
    let message = host_eval_failure(&EvalRequest::new("%resource"));
    assert!(message.contains("%resource"), "{message}");
    let host = RecordingHost::answering_nothing();
    let message = host_eval_failure(&EvalRequest::new("%resource").with_host(&host));
    assert!(message.contains("%resource"), "{message}");
}

// ---------------------------------------------------------------------------
// path.env.008 -- %rootResource is the outermost resource
// ---------------------------------------------------------------------------

// covers: path.env.008
#[test]
fn root_resource_is_the_outermost_resource() {
    // `patient-container-example.json` is a `Patient` containing an
    // `Organization`. With the contained resource as the focus, `%resource` is
    // the contained `Organization` and `%rootResource` the enclosing `Patient`
    // (FHIRPathEngine.java:1761 and :1765) -- so the two constants differ, and
    // that difference is the scenario.
    let container = load_input("patient-container-example.json");

    let root = host_eval(
        &EvalRequest::new("%rootResource.resourceType")
            .over(&container)
            .at("Patient.contained"),
    );
    assert_eq!(rendered(&root), ["string:Patient"]);

    let focus = host_eval(
        &EvalRequest::new("%resource.resourceType")
            .over(&container)
            .at("Patient.contained"),
    );
    assert_eq!(rendered(&focus), ["string:Organization"]);

    // Stated as an inequality too, so an implementation that aliased the two
    // constants would fail here even if both answers happened to be a resource.
    assert_ne!(rendered(&root), rendered(&focus));
}

// ---------------------------------------------------------------------------
// path.env.009 -- %rootResource with no root resource
// ---------------------------------------------------------------------------

// covers: path.env.009
#[test]
fn root_resource_with_no_root_resource_is_an_execution_error() {
    // FHIRPathEngine.java:1766-1767, the mirror of `%resource`'s guard, with
    // its own message naming `%rootResource`.
    let message = expect_evaluation_failure_on(fresh_engine, "%rootResource", None);
    assert!(
        message.contains("%rootResource"),
        "the error must name %rootResource: {message}"
    );
    expect_evaluation_failure_on(fresh_engine, "%rootResource.id", None);
}

// ---------------------------------------------------------------------------
// path.env.010 -- %context is the node evaluation started from
// ---------------------------------------------------------------------------

// covers: path.env.010
#[test]
fn context_is_the_node_evaluation_started_from() {
    // FHIRPathEngine.java:1769-1770 returns `context.context` with no guard at
    // all, so `%context` is whatever node evaluation was handed.
    let patient = load_input("patient-example.xml");

    let values = host_eval(
        &EvalRequest::new("%context")
            .over(&patient)
            .at("Patient.name"),
    );
    assert_eq!(
        rendered(&values),
        [
            "HumanName:<element>",
            "HumanName:<element>",
            "HumanName:<element>"
        ],
        "the starting context is the three HumanNames, not the Patient"
    );

    // Navigating from it reaches the name's own children, which is the property
    // that makes `%context` useful in an invariant.
    let families = host_eval(
        &EvalRequest::new("%context.family")
            .over(&patient)
            .at("Patient.name"),
    );
    assert_eq!(rendered(&families), ["string:Chalmers", "string:Windsor"]);

    // Started from the resource root instead, `%context` is the Patient.
    let root = host_eval(&EvalRequest::new("%context.resourceType").over(&patient));
    assert_eq!(rendered(&root), ["string:Patient"]);
}

// ---------------------------------------------------------------------------
// path.env.011 -- an unknown variable with no host service
// ---------------------------------------------------------------------------

// covers: path.env.011
#[test]
fn an_unknown_environment_variable_with_no_host_service_is_an_execution_error() {
    // `} else if (hostServices == null) { throw makeException(expr,
    // FHIRPATH_UNKNOWN_CONSTANT, s); }` (FHIRPathEngine.java:1781-1782). The
    // message carries `s`, which still has its leading `%`.
    let patient = load_input("patient-example.xml");
    let message =
        expect_evaluation_failure_on(engine_without_host, "%nosuchvariable", Some(&patient));
    assert!(
        message.contains("nosuchvariable"),
        "the error must name the constant: {message}"
    );

    // Not empty, which is the failure mode the scenario exists to forbid.
    expect_evaluation_failure_on(
        engine_without_host,
        "%nosuchvariable.exists()",
        Some(&patient),
    );

    // With a host service installed the same name is delegated instead of
    // failing, and a host service that owns nothing answers empty -- so the
    // error is about the *absence* of a host service and not about the name.
    let host = RecordingHost::answering_nothing();
    let values = host_eval(
        &EvalRequest::new("%nosuchvariable")
            .over(&patient)
            .with_host(&host),
    );
    assert!(
        values.is_empty(),
        "a host service that owns no name answers empty: {:?}",
        rendered(&values)
    );
    assert_eq!(
        host.modes_for("nosuchvariable"),
        [ConstantMode::Explicit],
        "the name the expression wrote is asked for with the explicit mode"
    );
}

// ---------------------------------------------------------------------------
// path.env.012 -- a host service may supply a collection
// ---------------------------------------------------------------------------

// covers: path.env.012
#[test]
fn a_host_service_may_supply_an_environment_variable_including_a_collection() {
    // Upstream: `org.hl7.fhir.r4.test.FHIRPathTests.resolveConstantReturnsList`
    // (FHIRPathTests.java:318) installs a host service answering two
    // `StringType`s for one `%name` and asserts both come back, in order. That
    // is why `resolve_constant` returns a `Vec` and not an `Option`
    // (`design.md` section 4.4).
    let host = RecordingHost::new(&[("dummyConstant", &["dummyConstant1", "dummyConstant2"])]);

    let values = host_eval(&EvalRequest::new("%dummyConstant").with_host(&host));
    assert_eq!(
        rendered(&values),
        ["string:dummyConstant1", "string:dummyConstant2"],
        "both values, in the order the host service returned them"
    );

    // The name reaches the host service without its `%`, which is what
    // `s.substring(1)` at FHIRPathEngine.java:1784 does.
    assert_eq!(host.asked().len(), 1);
    assert_eq!(
        host.asked()[0],
        Ask::Constant("dummyConstant".to_string(), ConstantMode::Explicit)
    );

    // A single-valued answer still works, so the collection case is not a
    // special path.
    let one = RecordingHost::new(&[("single", &["only"])]);
    assert_eq!(
        rendered(&host_eval(&EvalRequest::new("%single").with_host(&one))),
        ["string:only"]
    );

    // And the upstream test evaluates with a **null** input, so a host constant
    // needs no resource at all.
    let values = host_eval(&EvalRequest::new("%dummyConstant.count()").with_host(&host));
    assert_eq!(rendered(&values), ["integer:2"]);
}

// ---------------------------------------------------------------------------
// path.env.013 -- defineVariable
// ---------------------------------------------------------------------------

// covers: path.env.013
#[test]
fn define_variable_introduces_a_variable_for_the_rest_of_the_expression() {
    // `funcDefineVariable` (FHIRPathEngine.java:5635) stashes the value into
    // the execution context under the given name and returns the focus
    // unchanged, so the rest of the expression sees both.
    assert_eq!(
        eval_patient("Patient.name.first().defineVariable('n', family).select(%n)"),
        string("Chalmers")
    );

    // With one argument the variable takes the focus (:5644).
    assert_eq!(
        eval_patient("Patient.name.first().family.defineVariable('f').select(%f)"),
        string("Chalmers")
    );

    // The focus passes through untouched, which is what lets `defineVariable`
    // appear mid-path.
    assert_eq!(
        eval_patient("Patient.name.defineVariable('x', 'v').count()"),
        primitive("integer", "3")
    );
}

// ---------------------------------------------------------------------------
// path.env.014 -- a defined variable does not leak
// ---------------------------------------------------------------------------

// covers: path.env.014
#[test]
fn a_variable_defined_by_define_variable_does_not_leak() {
    let patient = load_input("patient-example.xml");

    // Defined inside one argument, referenced outside it: the variable is not
    // in scope, so resolution falls through to the unknown-constant path and
    // fails naming the constant (FHIRPathEngine.java:1782).
    let message = expect_evaluation_failure_on(
        engine_without_host,
        "Patient.name.where(defineVariable('v', family).exists()).select(%v)",
        Some(&patient),
    );
    assert!(
        message.contains("v"),
        "the error must name the constant that is out of scope: {message}"
    );

    // And a variable never defined at all fails the same way, so the scoping
    // rule is what the first assertion is about.
    expect_evaluation_failure_on(engine_without_host, "%v", Some(&patient));
}

// ---------------------------------------------------------------------------
// path.env.015 -- the four constant-evaluation modes
// ---------------------------------------------------------------------------

// covers: path.env.015
#[test]
fn the_host_service_is_told_which_constant_evaluation_mode_is_in_force() {
    let patient = load_input("patient-example.xml");

    // Exactly four modes, and they are the ones `FhirPathR4.setEvaluationContext`
    // maps one-for-one onto HAPI's own enum.
    let all = [
        ConstantMode::Explicit,
        ConstantMode::NoValue,
        ConstantMode::ImplicitBefore,
        ConstantMode::ImplicitAfter,
    ];
    assert_eq!(all.len(), 4);

    // 1. explicit -- the expression wrote `%name` and the engine does not know
    //    it internally. The evaluation pass's constant node is :1497.
    let host = RecordingHost::new(&[("owned", &["v"])]);
    host_eval(&EvalRequest::new("%owned").over(&patient).with_host(&host));
    assert_eq!(host.modes_for("owned"), [ConstantMode::Explicit]);

    // ...and the check pass's constant node is :1610, also explicit. It is
    // **not** no-value: that is the distinction the scenario insists on, and a
    // host service keying its type answers off the mode must see the split.
    let check_host = RecordingHost::new(&[("owned", &["v"])]);
    let mut engine = host_engine_under_test();
    engine
        .check("%owned", Some("Patient"), Some(&check_host))
        .expect("checking an explicit constant must succeed");
    assert_eq!(
        check_host.modes_for("owned"),
        [ConstantMode::Explicit],
        "an explicit %name in the check pass is explicit, not no-value"
    );

    // 2. no-value -- one site only, :1586: the check pass's **name** node, at
    //    entry, with no focus type at all. That is how the 90 suite cases with
    //    no inputfile are checked.
    let novalue_host = RecordingHost::new(&[("name", &["v"])]);
    let mut engine = host_engine_under_test();
    engine
        .check("name", None, Some(&novalue_host))
        .expect("checking with no context type must reach the host service");
    assert_eq!(
        novalue_host.modes_for("name"),
        [ConstantMode::NoValue],
        "no focus type at the entry of the check pass is the no-value mode"
    );

    // With no host service that same site fails rather than yielding empty:
    // `executeContextType` (:3225-3226) raises FHIRPATH_HO_HOST_SERVICES.
    let mut engine = host_engine_under_test();
    match engine.check("name", None, None) {
        Err(EngineError::Diagnostic { message, .. }) => assert!(
            !message.is_empty(),
            "the no-host-services error must carry a message"
        ),
        Err(e) => panic!("the no-host-services path must be a diagnostic, not: {e}"),
        Ok(t) => {
            panic!("checking `name` with no focus type and no host service must fail, got {t}")
        }
    }

    // 3 and 4 are `path.env.016` and `path.env.017`, which assert the sites and
    // the application-context gate. Here only their distinctness is stated: no
    // two modes may be the same value, or a host service could not tell the
    // sites apart.
    let mut seen = all.to_vec();
    seen.dedup();
    assert_eq!(seen.len(), 4, "the four modes must be four distinct values");
}

// ---------------------------------------------------------------------------
// path.env.016 -- a host constant can shadow a real property (DEV-0010)
// ---------------------------------------------------------------------------

// covers: path.env.016
#[test]
fn a_host_constant_can_shadow_a_real_property() {
    let patient = load_input("patient-example.xml");

    // FHIRPathEngine.java:3184-3190, at the entry of `execute`:
    //
    //     if (atEntry && context.appInfo != null && hostServices != null) {
    //       List<Base> temp = hostServices.resolveConstant(this, context.appInfo,
    //           exp.getName(), FHIRPathConstantEvaluationMode.IMPLICIT_BEFORE);
    //       if (!temp.isEmpty()) { result.addAll(temp); return result; }
    //     }
    //
    // So a non-empty implicit-before answer for the leading **name** is
    // returned and the element is never navigated. `DEV-0010` records it.
    let shadowing = RecordingHost::new(&[("name", &["shadowed"])]);
    let values = host_eval(
        &EvalRequest::new("name")
            .over(&patient)
            .with_host(&shadowing)
            .with_app_context(),
    );
    assert_eq!(
        rendered(&values),
        ["string:shadowed"],
        "the host service's answer replaces the element entirely"
    );
    assert!(
        shadowing
            .modes_for("name")
            .contains(&ConstantMode::ImplicitBefore),
        "the host service is asked before navigation: {:?}",
        shadowing.asked()
    );

    // The real element is three HumanNames, so the shadowing is observable and
    // not a coincidence.
    assert_eq!(
        rendered(&host_eval(&EvalRequest::new("name").over(&patient))).len(),
        3
    );

    // It is asked before the leading-type-name special case too, so even a
    // resource type name can be shadowed.
    let typed = RecordingHost::new(&[("Patient", &["shadowed-type"])]);
    assert_eq!(
        rendered(&host_eval(
            &EvalRequest::new("Patient.name")
                .over(&patient)
                .with_host(&typed)
                .with_app_context()
        )),
        ["string:shadowed-type"]
    );

    // An empty answer leaves navigation to proceed, which is why a host service
    // must answer empty for every name it does not own.
    let nothing = RecordingHost::answering_nothing();
    assert_eq!(
        rendered(&host_eval(
            &EvalRequest::new("name.given.first()")
                .over(&patient)
                .with_host(&nothing)
                .with_app_context()
        )),
        ["string:Peter"]
    );

    // Entry is per sub-expression and per focus item, not once per expression:
    // `where()`'s criteria has its own entry, and the per-item loop asks for
    // each item separately.
    let per_item = RecordingHost::answering_nothing();
    host_eval(
        &EvalRequest::new("name.where(given.exists())")
            .over(&patient)
            .with_host(&per_item)
            .with_app_context(),
    );
    let given_asks = per_item.modes_for("given");
    assert_eq!(
        given_asks.len(),
        3,
        "the criteria's leading name is asked once per focus item: {:?}",
        per_item.asked()
    );
    assert!(
        given_asks
            .iter()
            .all(|m| *m == ConstantMode::ImplicitBefore)
    );

    // No live case grades this, so it is differential-only: the suite installs
    // its host service but passes no application context, so the site never
    // fires there.
    let (suite, _) = pinned();
    assert_eq!(suite.case_count(), 1191);
}

// ---------------------------------------------------------------------------
// path.env.017 -- implicit-after is a fall-through, and both need a context
// ---------------------------------------------------------------------------

// covers: path.env.017
#[test]
fn implicit_after_is_a_fall_through_and_both_implicit_modes_need_an_app_context() {
    let patient = load_input("patient-example.xml");

    // FHIRPathEngine.java:3212-3215, after navigation:
    //
    //     if (atEntry && context.appInfo != null && hostServices != null
    //         && result.isEmpty()) {
    //       result.addAll(hostServices.resolveConstant(..., IMPLICIT_AFTER));
    //     }
    //
    // `nosuchelement` is not an element of `Patient`, so navigation yields
    // empty and only then is the host service asked.
    let host = RecordingHost::new(&[("nosuchelement", &["from-host"])]);
    let values = host_eval(
        &EvalRequest::new("nosuchelement")
            .over(&patient)
            .with_host(&host)
            .with_app_context(),
    );
    assert_eq!(rendered(&values), ["string:from-host"]);

    // Both modes fire at that name, and in this order: before, then after.
    assert_eq!(
        host.modes_for("nosuchelement"),
        [ConstantMode::ImplicitBefore, ConstantMode::ImplicitAfter],
        "navigation runs between the two calls: {:?}",
        host.asked()
    );

    // The gate is the application context, **not** the host service. With the
    // same host service and a null application context neither mode fires, the
    // host service is not consulted for the leading name, and the result is
    // empty.
    let ungated = RecordingHost::new(&[("nosuchelement", &["from-host"])]);
    let values = host_eval(
        &EvalRequest::new("nosuchelement")
            .over(&patient)
            .with_host(&ungated),
    );
    assert!(
        values.is_empty(),
        "with no application context the answer is empty: {:?}",
        rendered(&values)
    );
    assert!(
        ungated.modes_for("nosuchelement").is_empty(),
        "the host service must not be consulted for the leading name: {:?}",
        ungated.asked()
    );

    // ...while `%name` still reaches it, which is what makes the gate specific
    // to name resolution rather than to the host service as a whole.
    let explicit = RecordingHost::new(&[("owned", &["v"])]);
    assert_eq!(
        rendered(&host_eval(
            &EvalRequest::new("%owned")
                .over(&patient)
                .with_host(&explicit)
        )),
        ["string:v"]
    );
    assert_eq!(explicit.modes_for("owned"), [ConstantMode::Explicit]);

    // And implicit-after really is a fall-through on empty: for a name that
    // *does* navigate, it is never asked.
    let present = RecordingHost::new(&[("gender", &["from-host"])]);
    let values = host_eval(
        &EvalRequest::new("gender")
            .over(&patient)
            .with_host(&present)
            .with_app_context(),
    );
    assert_eq!(
        rendered(&values),
        ["string:from-host"],
        "implicit-before still shadows a present element (path.env.016)"
    );
    let empty_host = RecordingHost::answering_nothing();
    let values = host_eval(
        &EvalRequest::new("gender")
            .over(&patient)
            .with_host(&empty_host)
            .with_app_context(),
    );
    assert_eq!(rendered(&values), ["code:male"]);
    assert_eq!(
        empty_host.modes_for("gender"),
        [ConstantMode::ImplicitBefore],
        "navigation succeeded, so implicit-after is not asked: {:?}",
        empty_host.asked()
    );
}
