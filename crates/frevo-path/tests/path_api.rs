//! The public evaluation surface: `path.api.001` through `path.api.008`.
//!
//! ```text
//! cargo test -p frevo-path --test path_api
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. The surface is shaped after
//! `ca.uhn.fhir.fhirpath.IFhirPath`, so the scenarios are graded against that
//! interface's five operations and the behaviour `FhirPathR4` gives them:
//!
//! | `IFhirPath` | Scenario |
//! |---|---|
//! | `evaluate(IBase, String, Class<T>)` | `path.api.001` |
//! | `parse(String)` + `evaluate(IBase, IParsedExpression, Class<T>)` | `path.api.002` |
//! | `evaluateFirst(...)` | `path.api.003`, `path.api.004` |
//! | the return-type check | `path.api.005` |
//! | `setEvaluationContext(...)` | `path.api.007` |
//!
//! Upstream JUnit provenance for the messages and the empty-result behaviour is
//! `reference/hapi-fhir/hapi-fhir-validation/src/test/java/ca/uhn/fhir/fhirpath/FhirPathTest.java`
//! (`testEvaluateNormal`, `testEvaluateUnknownPath`, `testEvaluateInvalidPath`,
//! `testEvaluateWrongType`) and
//! `reference/hapi-fhir/hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java`
//! for the three `Msg.code` values: 2409 for a parse failure, 255 for an
//! evaluation failure and 256 for an unexpected result type.
//!
//! `path.api.008` is a structural invariant that already holds -- the crate's
//! manifest names no dependency at all -- so that one test passes today. It is
//! kept because the invariant is what stops a later task from reaching for
//! `frevo-model` (ADR 0004 decision 5).

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::{Arc, Mutex};

use frevo_path::{
    ConstantMode, FunctionDetails, HostError, HostServices, Node, NodeError, Primitive, TypeSet,
    Value,
};

use oracle::engine::EngineError;
use oracle::input;

// ---------------------------------------------------------------------------
// In-memory nodes
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

/// `FhirPathTest.createPatientResourceWithTwoNames`, which is the fixture four
/// of the upstream JUnit tests use: two `HumanName`s, each with a family and
/// two given names.
fn patient_with_two_names() -> Value {
    let name = |f: &str, g1: &str, g2: &str| {
        TestNode::element(
            "HumanName",
            vec![
                ("family".to_string(), TestNode::prim("string", f)),
                ("given".to_string(), TestNode::prim("string", g1)),
                ("given".to_string(), TestNode::prim("string", g2)),
            ],
        )
    };
    TestNode::resource(
        "Patient",
        vec![
            ("name".to_string(), name("N1F1", "N1G1", "N1G2")),
            ("name".to_string(), name("N2F1", "N2G1", "N2G2")),
        ],
    )
}

// ---------------------------------------------------------------------------
// The public surface, as a seam
// ---------------------------------------------------------------------------

/// An opaque handle to a parsed expression -- `IFhirPath.IParsedExpression`,
/// which the interface's own documentation calls "a black box".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Parsed(usize);

/// The surface `path.api.*` specifies. Every method is synchronous: it returns
/// its answer and not a future, which is `path.api.006`.
trait FhirPath {
    fn parse(&mut self, expr: &str) -> Result<Parsed, EngineError>;

    /// Evaluate from the expression text, requiring every result to be of
    /// `want` -- the `Class<T>` parameter of the Java interface.
    fn evaluate(
        &mut self,
        input: &Value,
        expr: &str,
        want: &str,
    ) -> Result<Vec<Value>, EngineError>;

    /// Evaluate a previously parsed expression.
    fn evaluate_parsed(
        &mut self,
        input: &Value,
        parsed: Parsed,
        want: &str,
    ) -> Result<Vec<Value>, EngineError>;

    /// `evaluateFirst`: at most one value.
    fn evaluate_first(
        &mut self,
        input: &Value,
        expr: &str,
        want: &str,
    ) -> Result<Option<Value>, EngineError>;

    /// `setEvaluationContext`, which is how a host service is installed and
    /// replaced.
    fn set_host_services(&mut self, host: Arc<dyn HostServices>);

    /// How many times this instance has parsed anything. Not part of the Java
    /// interface; it is what makes `path.api.002`'s "parsed once" assertable.
    fn parse_count(&self) -> usize;
}

/// Phase D binds the real surface here.
fn fhir_path_under_test() -> Box<dyn FhirPath> {
    Box::new(UnboundFhirPath)
}

struct UnboundFhirPath;

impl FhirPath for UnboundFhirPath {
    fn parse(&mut self, _expr: &str) -> Result<Parsed, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate(
        &mut self,
        _input: &Value,
        _expr: &str,
        _want: &str,
    ) -> Result<Vec<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate_parsed(
        &mut self,
        _input: &Value,
        _parsed: Parsed,
        _want: &str,
    ) -> Result<Vec<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate_first(
        &mut self,
        _input: &Value,
        _expr: &str,
        _want: &str,
    ) -> Result<Option<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn set_host_services(&mut self, _host: Arc<dyn HostServices>) {}

    fn parse_count(&self) -> usize {
        0
    }
}

/// The type name that accepts anything, standing for `IBase.class` in the four
/// upstream tests that pass it.
const ANY: &str = "Base";

/// A host service that answers one constant and records nothing else, which is
/// all `path.api.007` needs.
#[derive(Debug, Default)]
struct OneConstantHost {
    asked: Mutex<Vec<String>>,
}

impl OneConstantHost {
    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl HostServices for OneConstantHost {
    fn resolve_constant(&self, name: &str, _mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(name.to_string());
        if name == "owned" {
            Ok(vec![Value::String("answered".to_string())])
        } else {
            Ok(Vec::new())
        }
    }

    fn resolve_constant_type(
        &self,
        _name: &str,
        _mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        Ok(None)
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

// ---------------------------------------------------------------------------
// path.api.001 -- evaluate from the expression text
// ---------------------------------------------------------------------------

// covers: path.api.001
#[test]
fn an_expression_can_be_evaluated_from_its_text() {
    // Upstream: `FhirPathTest.testEvaluateNormal` evaluates `Patient.name` over
    // a Patient with two names and asserts two results.
    let patient = patient_with_two_names();
    let mut fp = fhir_path_under_test();

    let values = fp
        .evaluate(&patient, "Patient.name", ANY)
        .expect("evaluating from the expression text must return the result collection");
    assert_eq!(values.len(), 2, "{:?}", rendered(&values));
    assert_eq!(
        rendered(&values),
        ["HumanName:<element>", "HumanName:<element>"]
    );

    // A primitive result comes back as the values themselves.
    let values = fp
        .evaluate(&patient, "Patient.name.family", ANY)
        .expect("evaluates");
    assert_eq!(rendered(&values), ["string:N1F1", "string:N2F1"]);

    // A malformed expression is a diagnostic, not a panic: upstream this is
    // `testEvaluateInvalidPath`, whose assertion is that the message contains
    // "termination at unexpected token" and which `FhirPathR4` wraps as
    // `Msg.code(2409)`.
    match fp.evaluate(&patient, "Patient....nameFOO", ANY) {
        Err(EngineError::Diagnostic { message, .. }) => {
            assert!(!message.is_empty(), "a parse failure must carry a message")
        }
        Err(e) => panic!("a malformed expression must be a diagnostic, not: {e}"),
        Ok(values) => panic!(
            "`Patient....nameFOO` must not evaluate: {:?}",
            rendered(&values)
        ),
    }
}

// ---------------------------------------------------------------------------
// path.api.002 -- a parsed expression can be evaluated repeatedly
// ---------------------------------------------------------------------------

// covers: path.api.002
#[test]
fn a_parsed_expression_can_be_evaluated_repeatedly() {
    // `IFhirPath.parse` exists so that "an identical expression ... evaluated
    // many times against different targets" is not re-parsed, which is also the
    // premise of `path.perf.002`.
    let mut fp = fhir_path_under_test();
    let parsed = fp
        .parse("Patient.name.family")
        .expect("the expression must parse once");
    assert_eq!(fp.parse_count(), 1);

    // 1,000 nodes, each with its own distinguishable value, so every evaluation
    // is checked to return **its own** result and not a cached one.
    let nodes: Vec<Value> = (0..1000)
        .map(|i| {
            TestNode::resource(
                "Patient",
                vec![(
                    "name".to_string(),
                    TestNode::element(
                        "HumanName",
                        vec![(
                            "family".to_string(),
                            TestNode::prim("string", &format!("F{i}")),
                        )],
                    ),
                )],
            )
        })
        .collect();

    for (i, node) in nodes.iter().enumerate() {
        let values = fp
            .evaluate_parsed(node, parsed, ANY)
            .unwrap_or_else(|e| panic!("evaluation {i} must succeed: {e}"));
        assert_eq!(rendered(&values), [format!("string:F{i}")]);
    }

    // And the expression was parsed exactly once across all 1,000 evaluations.
    assert_eq!(
        fp.parse_count(),
        1,
        "evaluating a parsed expression must not re-parse it"
    );
}

// ---------------------------------------------------------------------------
// path.api.003 -- a first-match operation returns at most one value
// ---------------------------------------------------------------------------

// covers: path.api.003
#[test]
fn a_first_match_operation_returns_at_most_one_value() {
    // `FhirPathR4.evaluateFirst` is `evaluate(...).stream().findFirst()`, so it
    // is the first in order and not an arbitrary one.
    let patient = patient_with_two_names();
    let mut fp = fhir_path_under_test();

    let value = fp
        .evaluate_first(&patient, "Patient.name.family", ANY)
        .expect("evaluating must succeed")
        .expect("there are two families, so there is a first");
    assert_eq!(
        describe(&value),
        "string:N1F1",
        "the first in document order"
    );

    // The full evaluation returns both, so "at most one" is a property of the
    // operation and not of the expression.
    let all = fp
        .evaluate(&patient, "Patient.name.family", ANY)
        .expect("evaluates");
    assert_eq!(rendered(&all), ["string:N1F1", "string:N2F1"]);

    // First in order, over an expression whose order is reversed by `sort`, so
    // the two cannot coincide by accident.
    let value = fp
        .evaluate_first(&patient, "Patient.name.family.sort(-$this)", ANY)
        .expect("evaluates")
        .expect("there is a first");
    assert_eq!(describe(&value), "string:N2F1");
}

// ---------------------------------------------------------------------------
// path.api.004 -- a first-match operation on no match
// ---------------------------------------------------------------------------

// covers: path.api.004
#[test]
fn a_first_match_operation_on_no_match_returns_nothing() {
    // Upstream: `FhirPathTest.testEvaluateUnknownPath` asserts that
    // `Patient.nameFOO` evaluates to an **empty list** rather than failing, so
    // `evaluateFirst` over it is `Optional.empty()`.
    let patient = patient_with_two_names();
    let mut fp = fhir_path_under_test();

    let value = fp
        .evaluate_first(&patient, "Patient.nameFOO", ANY)
        .expect("an unknown element must not fail the evaluation");
    assert!(value.is_none(), "no match means no value: {value:?}");

    // Distinguishable from a value that is itself empty: an empty **string** is
    // a value, and the two must not be confused.
    let with_empty_string = TestNode::resource(
        "Patient",
        vec![(
            "name".to_string(),
            TestNode::element(
                "HumanName",
                vec![("family".to_string(), TestNode::prim("string", ""))],
            ),
        )],
    );
    let value = fp
        .evaluate_first(&with_empty_string, "Patient.name.family", ANY)
        .expect("evaluates");
    assert_eq!(
        value.as_ref().map(describe),
        Some("string:".to_string()),
        "an empty string is a present value"
    );
    assert!(value.is_some());

    // And the full evaluation of the no-match case is an empty collection, not
    // a one-item collection holding nothing.
    let values = fp
        .evaluate(&patient, "Patient.nameFOO", ANY)
        .expect("evaluates");
    assert!(values.is_empty(), "{:?}", rendered(&values));
}

// ---------------------------------------------------------------------------
// path.api.005 -- an unexpected result type is reported
// ---------------------------------------------------------------------------

// covers: path.api.005
#[test]
fn a_result_of_an_unexpected_type_is_reported_and_not_silently_dropped() {
    // Upstream: `FhirPathTest.testEvaluateWrongType` asks for `Patient.name` as
    // a `string` and asserts the message **ends with**
    //
    //     FhirPath expression returned unexpected type HumanName - Expected <class>
    //
    // which `FhirPathR4` raises as `Msg.code(256)` after the evaluation, per
    // result item (`FhirPathR4.java`, the `for (IBase next : result)` loop).
    let patient = patient_with_two_names();
    let mut fp = fhir_path_under_test();

    match fp.evaluate(&patient, "Patient.name", "string") {
        Err(EngineError::Diagnostic { message, .. }) => {
            assert!(
                message.contains("HumanName"),
                "the error must name the type that was returned: {message}"
            );
            assert!(
                message.contains("string"),
                "and the type that was expected: {message}"
            );
        }
        Err(e) => panic!("a type mismatch must be a diagnostic, not: {e}"),
        Ok(values) => panic!(
            "asking for a string and getting a HumanName must be reported, not dropped: {:?}",
            rendered(&values)
        ),
    }

    // Dropped is the failure mode being forbidden, so the empty answer is
    // explicitly rejected: an implementation that filtered mismatched items
    // would return an empty collection here and pass a weaker test.
    //
    // Asking for the right type succeeds, so the check is about the mismatch.
    let values = fp
        .evaluate(&patient, "Patient.name.family", "string")
        .expect("a string result asked for as a string must succeed");
    assert_eq!(rendered(&values), ["string:N1F1", "string:N2F1"]);

    // And an empty result satisfies any requested type, because there is no
    // item to mismatch -- which is `testEvaluateUnknownPath` asking for
    // `HumanName` and getting an empty list.
    let values = fp
        .evaluate(&patient, "Patient.nameFOO", "HumanName")
        .expect("an empty result cannot mismatch");
    assert!(values.is_empty());
}

// ---------------------------------------------------------------------------
// path.api.006 -- evaluation is synchronous
// ---------------------------------------------------------------------------

// covers: path.api.006
#[test]
fn evaluation_is_synchronous() {
    // The requirement exists so the engine can be called from inside a write
    // transaction, which rules out needing an async runtime to drive it.
    //
    // Half of this is a type-level property and is enforced by this file
    // compiling at all: every `FhirPath` method returns its answer directly, so
    // there is no future to await and no executor to install.
    let patient = patient_with_two_names();
    let mut fp = fhir_path_under_test();
    let values = fp
        .evaluate(&patient, "Patient.name.family", ANY)
        .expect("evaluation must complete without an asynchronous runtime");
    assert_eq!(rendered(&values), ["string:N1F1", "string:N2F1"]);

    // The other half is that no async runtime is linked in at all, which is
    // checked against the manifest: `frevo-path` must declare no dependency on
    // `tokio`, `async-std`, `smol` or `futures`.
    let manifest = std::fs::read_to_string(input::repo_path("crates/frevo-path/Cargo.toml"))
        .expect("the crate manifest is readable");
    for runtime in ["tokio", "async-std", "smol", "futures"] {
        assert!(
            !manifest.contains(runtime),
            "frevo-path must not depend on {runtime}: evaluation is synchronous"
        );
    }

    // And nothing in the crate's own sources is an `async fn`, which is what
    // would force a caller to find a runtime.
    for file in source_files() {
        let text = std::fs::read_to_string(&file).expect("a source file is readable");
        assert!(
            !text.contains("async fn"),
            "{} declares an async fn; the evaluation surface is synchronous",
            file.display()
        );
    }
}

/// Every `.rs` file under `crates/frevo-path/src`, recursively.
fn source_files() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![input::repo_path("crates/frevo-path/src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the source directory is readable") {
            let path = entry.expect("a directory entry is readable").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    assert!(!out.is_empty(), "the crate has sources");
    out
}

// ---------------------------------------------------------------------------
// path.api.007 -- a host service is optional and replaceable
// ---------------------------------------------------------------------------

// covers: path.api.007
#[test]
fn a_host_service_is_optional_and_replaceable() {
    let patient = patient_with_two_names();

    // With no host service, an expression that needs none evaluates.
    let mut bare = fhir_path_under_test();
    let values = bare
        .evaluate(&patient, "Patient.name.family", ANY)
        .expect("an expression needing no host service must evaluate without one");
    assert_eq!(rendered(&values), ["string:N1F1", "string:N2F1"]);

    // ...and one that needs a host service fails rather than yielding empty
    // (`path.env.011`).
    match bare.evaluate(&patient, "%owned", ANY) {
        Err(EngineError::Diagnostic { message, .. }) => assert!(
            message.contains("owned"),
            "the error must name the constant: {message}"
        ),
        Err(e) => panic!("an unknown constant must be a diagnostic, not: {e}"),
        Ok(values) => panic!(
            "%owned must not resolve with no host service: {:?}",
            rendered(&values)
        ),
    }

    // With one installed, the same expression resolves and the host service is
    // consulted.
    let host = Arc::new(OneConstantHost::default());
    let mut configured = fhir_path_under_test();
    configured.set_host_services(host.clone());
    let values = configured
        .evaluate(&patient, "%owned", ANY)
        .expect("with a host service the constant resolves");
    assert_eq!(rendered(&values), ["string:answered"]);
    assert_eq!(host.asked(), ["owned"]);

    // Expressions needing no host service still work on the configured
    // instance, so installing one takes nothing away.
    let values = configured
        .evaluate(&patient, "Patient.name.family", ANY)
        .expect("evaluates");
    assert_eq!(rendered(&values), ["string:N1F1", "string:N2F1"]);

    // Replaceable: a second host service supersedes the first, and only the
    // second is asked afterwards.
    let second = Arc::new(OneConstantHost::default());
    configured.set_host_services(second.clone());
    configured
        .evaluate(&patient, "%owned", ANY)
        .expect("evaluates");
    assert_eq!(second.asked(), ["owned"]);
    assert_eq!(
        host.asked(),
        ["owned"],
        "the replaced host service must not be consulted again"
    );
}

// ---------------------------------------------------------------------------
// path.api.008 -- frevo-path does not depend on frevo-model
// ---------------------------------------------------------------------------

// covers: path.api.008
#[test]
fn frevo_path_does_not_depend_on_frevo_model() {
    // ADR 0004 decision 5: the dependency edge runs the other way, so
    // `frevo-model` implements `Node` and `Model` over its span-based codec and
    // this crate never names it.
    //
    // This invariant already holds, so this test passes today. It is kept
    // because it is what stops a later task from reaching for the model: the
    // engine must stay testable with a stub `Node`, which is the whole reason
    // the other tests in this file can exist at all.
    let manifest = std::fs::read_to_string(input::repo_path("crates/frevo-path/Cargo.toml"))
        .expect("the crate manifest is readable");
    assert!(
        !manifest.contains("frevo-model"),
        "frevo-path's manifest must not name frevo-model:\n{manifest}"
    );
    assert!(
        !manifest.contains("frevo_model"),
        "nor under its crate-name spelling"
    );

    // Nor may any source file *refer* to it. The check is on code references
    // and not on the spelling: `src/lib.rs` and `src/seam/mod.rs` both discuss
    // `frevo-model` in prose, explaining why the edge runs the other way, and a
    // test that forbade the word in a comment would be testing the wrong thing.
    for file in source_files() {
        let text = std::fs::read_to_string(&file).expect("a source file is readable");
        for reference in [
            "use frevo_model",
            "extern crate frevo_model",
            "frevo_model::",
        ] {
            assert!(
                !text.contains(reference),
                "{} refers to frevo_model ({reference})",
                file.display()
            );
        }
    }

    // And the edge really does run the other way, which is what makes the
    // prohibition meaningful rather than vacuous.
    let model_manifest = std::fs::read_to_string(input::repo_path("crates/frevo-model/Cargo.toml"))
        .expect("frevo-model's manifest is readable");
    assert!(
        model_manifest.contains("frevo-path"),
        "frevo-model depends on frevo-path for the trait definitions:\n{model_manifest}"
    );
}
