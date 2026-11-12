//! Navigation: `path.navigation.001` through `path.navigation.024`.
//!
//! ```text
//! cargo test -p frevo-path --test path_navigation
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Most of these scenarios name a
//! group or a case in the vendored suite, so the test drives the real case
//! through the committed harness (`tests/oracle/`) rather than hand-rolling an
//! input, and then states the expression's own result at the engine seam where
//! the scenario states one.
//!
//! Two scenarios cannot be driven from the suite at all, and both say so:
//!
//! - `path.navigation.013` is `FHIRPathTests.testEvaluate_Id`, a JUnit test and
//!   not a suite case, whose input is a `Patient` whose id was set from a full
//!   URL. It is driven over the crate's published `Node` seam instead.
//! - `path.navigation.018` has no case either: `$total` outside `aggregate()`
//!   is an evaluation failure.
//!
//! Provenance, unless a test says otherwise:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.
//!
//! Why every test here fails today: no engine is bound to the harness seam, so
//! `parse`, `check` and `evaluate` all answer `EngineError::NotImplemented`,
//! which the harness grades `did-not-run` and which every helper below turns
//! into an explicit panic. A scenario that expects a *failure* is asserted
//! against `EngineError::Diagnostic` specifically, so an engine that refuses to
//! run can never satisfy it.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::{Arc, OnceLock};

use frevo_path::{Node, NodeError, Primitive, Value};

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, Document, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// Assert that every case in `group` passed, and that the group still has the
/// size this test was written against: a different count means the suite was
/// re-pinned without this test being revisited.
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

    let mut failures = Vec::new();
    for case in &g.cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        if r.status != CaseStatus::Passed {
            failures.push(format!(
                "{} ({:?}): {} -- {}",
                case.address,
                case.expression,
                r.status.as_str(),
                r.reason
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases in {group} did not pass; the first few:\n  {}",
        failures.len(),
        g.cases.len(),
        failures[..failures.len().min(5)].join("\n  ")
    );
}

/// Assert one case, addressed by `group/name`, passed.
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

/// The `<output>` bodies the pinned file records for a case, in order. A
/// temporal expectation's `@` prefix is part of the expectation and is never
/// stripped (`tests/oracle/value.rs::fhirpath_string`).
fn output_bodies(group: &str, name: &str) -> Vec<String> {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    case.outputs.iter().map(|o| o.body.clone()).collect()
}

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is evaluated: {e}"));
    engine
}

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be evaluated over: {e}")
    })
}

fn context_for<'a>(inp: Option<&'a SuiteInput>) -> CaseContext<'a> {
    let resource_type = inp.and_then(|i| i.resource_type());
    CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    }
}

/// Parse, check and evaluate `expr` over `inp`. Every stage must succeed, so a
/// missing engine is a panic naming the stage and never an empty result.
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

/// Evaluate over `patient-example.xml`, the input most of these scenarios name.
fn eval_patient(expr: &str) -> Vec<ResultItem> {
    let inp = load_input("patient-example.xml");
    eval_over(expr, Some(&inp))
}

/// `expr` must fail the **static check** over `inp` with a real diagnostic.
/// `NotImplemented` panics instead, so an unbound engine never looks like the
/// semantic failure the scenario asked for.
fn expect_check_failure(expr: &str, inp: &SuiteInput, flags: StrictnessFlags) -> String {
    let mut engine = fresh_engine();
    let context = context_for(Some(inp));
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before the check can reject it: {e}"));
    match engine.check(parsed, &context, flags) {
        Ok(()) => panic!("{expr:?} must fail the static check, but the check passed"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
    }
}

/// `expr` must parse and check over `inp` and then fail at **evaluation** with
/// a real diagnostic.
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

fn primitive(fhir_type: &str, value: &str) -> ResultItem {
    ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }
}

fn strings(values: &[&str]) -> Vec<ResultItem> {
    values.iter().map(|v| primitive("string", v)).collect()
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// The in-memory seam `path.navigation.013` needs
//
// `FHIRPathTests.testEvaluate_Id` is a JUnit test, not a suite case: its input
// is a `Patient` whose id was set from `http://base/Patient/123/_history/222`,
// which no file in `oracles/fhir-test-cases/fetched/r4/` carries. So that one
// scenario is driven over the crate's published `Node` seam, which exists
// today, with the engine still an unbound trait object.
// ---------------------------------------------------------------------------

/// A node built in memory, as a model implementation would hand one out.
#[derive(Debug)]
struct TestNode {
    type_name: String,
    is_resource: bool,
    children: Vec<(String, Value)>,
    primitive: Option<Primitive>,
}

impl TestNode {
    /// A resource with the given children.
    fn resource(type_name: &str, children: Vec<(String, Value)>) -> Arc<dyn Node> {
        Arc::new(TestNode {
            type_name: type_name.to_string(),
            is_resource: true,
            children,
            primitive: None,
        })
    }

    /// A primitive element: its FHIR type and its lexical form.
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

/// The surface `path.navigation.013` needs: evaluation over a node the caller
/// built, rather than over a suite input file.
trait NodeEngine {
    fn evaluate_over_node(
        &mut self,
        expr: &str,
        root: Arc<dyn Node>,
    ) -> Result<Vec<Value>, EngineError>;
}

/// Phase D binds the real engine here. Until then every call is
/// `NotImplemented`, which is not a result and is not a failure.
fn node_engine_under_test() -> Box<dyn NodeEngine> {
    Box::new(UnboundNodeEngine)
}

struct UnboundNodeEngine;

impl NodeEngine for UnboundNodeEngine {
    fn evaluate_over_node(
        &mut self,
        _expr: &str,
        _root: Arc<dyn Node>,
    ) -> Result<Vec<Value>, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

/// One result value rendered as `type:lexical`, with no `@` prefix: the prefix
/// is a property of the suite comparison and not of a value returned to a
/// caller (`path.oracle.008`).
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

// ---------------------------------------------------------------------------
// path.navigation.001 -- a leading resource type name matches the focus
// ---------------------------------------------------------------------------

// covers: path.navigation.001
#[test]
fn a_leading_resource_type_name_matches_the_focus_resource() {
    // `execute`'s leading-name special case (FHIRPathEngine.java:3190): at
    // entry, a name starting with an upper-case letter is matched against the
    // focus item's type and its base types rather than navigated.
    assert_eq!(
        eval_patient("Patient.name.given.first()"),
        vec![primitive("string", "Peter")]
    );

    // The suite grades the same navigation with the type name written plainly,
    // in backticks, and as a unicode escape.
    expect_case("testBasics", "testSimpleWithContext");
    expect_case("testBasics", "testSimpleBackTick1");
    expect_case("testBasics", "testEscapedIdentifierUnicode");
    assert_eq!(
        output_bodies("testBasics", "testEscapedIdentifierUnicode"),
        ["Peter"]
    );

    // A base type of the focus matches too, which is what makes the leading
    // name a type test and not a string comparison: the special case walks
    // `sd.getBaseDefinition()` until it runs out of base types.
    assert_eq!(
        eval_patient("Resource.id"),
        vec![primitive("id", "example")],
        "`Resource` is a base type of `Patient`, so it matches the focus"
    );
}

// ---------------------------------------------------------------------------
// path.navigation.002 -- the resource type name may be omitted
// ---------------------------------------------------------------------------

// covers: path.navigation.002
#[test]
fn the_resource_type_name_may_be_omitted() {
    assert_eq!(
        eval_patient("name.given.first()"),
        vec![primitive("string", "Peter")]
    );

    // `testBasics/testSimple` is the same navigation without the type name,
    // and `testEscapedIdentifier` the same again with backticks.
    expect_case("testBasics", "testSimple");
    expect_case("testBasics", "testEscapedIdentifier");

    // Both spellings must agree, which is the property the scenario is about.
    assert_eq!(
        eval_patient("name.given"),
        eval_patient("Patient.name.given"),
        "omitting the resource type name must not change the result"
    );
}

// ---------------------------------------------------------------------------
// path.navigation.003 -- a leading type name that is not the focus type
// ---------------------------------------------------------------------------

// covers: path.navigation.003
#[test]
fn a_leading_type_name_that_is_not_the_focus_type_yields_empty() {
    // The leading-name special case adds the focus item only when some type in
    // its hierarchy matches, so a non-matching type name contributes nothing
    // and every step after it navigates nothing.
    assert_eq!(eval_patient("Observation.value"), empty());
    assert_eq!(eval_patient("Encounter.status"), empty());

    // Empty, not an error: `exists()` over it answers false rather than
    // failing.
    assert_eq!(
        eval_patient("Observation.value.exists()"),
        vec![primitive("boolean", "false")]
    );

    // The suite grades the *checked* form of the same mistake as a semantic
    // error, which is a different stage and a different answer.
    expect_case("testBasics", "testSimpleWithWrongContext");
}

// ---------------------------------------------------------------------------
// path.navigation.004 -- navigation flattens across a repeating element
// ---------------------------------------------------------------------------

// covers: path.navigation.004
#[test]
fn navigation_flattens_across_a_repeating_element() {
    // Three `name` repetitions carrying five `given` values between them
    // flatten into one collection in document order.
    assert_eq!(
        eval_patient("Patient.name.given"),
        strings(&["Peter", "James", "Jim", "Peter", "James"])
    );

    // `testJoin/testJoin` is the scenario's named case: it joins exactly those
    // five values in exactly that order, so the order is graded by the suite
    // and not only here.
    assert_eq!(
        output_bodies("testJoin", "testJoin"),
        ["Peter,James,Jim,Peter,James"]
    );
    expect_case("testJoin", "testJoin");
    expect_case("testBasics", "testSimple");
}

// ---------------------------------------------------------------------------
// path.navigation.005 -- an absent element yields empty, not an error
// ---------------------------------------------------------------------------

// covers: path.navigation.005
#[test]
fn an_absent_element_yields_empty_rather_than_an_error() {
    assert_eq!(eval_patient("Patient.name.prefix"), empty());
    assert_eq!(eval_patient("Patient.name.suffix"), empty());

    // `testJoin/testJoinEmpty` grades the same absence through `join`, and its
    // expectation is no output at all.
    assert!(output_bodies("testJoin", "testJoinEmpty").is_empty());
    expect_case("testJoin", "testJoinEmpty");
    expect_case("testBasics", "testSimpleNone");

    // Absent shows up as an empty collection and never as a failure.
    assert_eq!(
        eval_patient("Patient.name.prefix.empty()"),
        vec![primitive("boolean", "true")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.006 -- a polymorphic element by its base name
// ---------------------------------------------------------------------------

// covers: path.navigation.006
#[test]
fn a_polymorphic_element_is_navigated_by_its_base_name() {
    let observation = load_input("observation-example.xml");
    assert_eq!(
        eval_over("Observation.value.exists()", Some(&observation)),
        vec![primitive("boolean", "true")]
    );
    expect_case("polymorphics", "testPolymorphicsA");

    // The value really is reached, not merely reported to exist.
    assert_eq!(
        eval_over("Observation.value.unit", Some(&observation)),
        vec![primitive("string", "lbs")]
    );
    expect_case("testObservations", "testPolymorphismA");
}

// ---------------------------------------------------------------------------
// path.navigation.007 -- a typed polymorphic property name is semantic
// ---------------------------------------------------------------------------

// covers: path.navigation.007
#[test]
fn a_typed_polymorphic_property_name_is_a_semantic_error() {
    let observation = load_input("observation-example.xml");

    // The check pass rejects it: `Observation.valueQuantity` is not an element
    // of `Observation`, because the choice element is named `value[x]` and the
    // engine never constructs the typed spelling (`design.md` section 4.2).
    let message = expect_check_failure(
        "Observation.valueQuantity.exists()",
        &observation,
        StrictnessFlags::CHECK,
    );
    assert!(
        message.contains("valueQuantity"),
        "the semantic error must name the element it rejected: {message}"
    );

    // Graded by two cases in two groups, which is why both are named.
    expect_case("polymorphics", "testPolymorphicsB");
    expect_case("testObservations", "testPolymorphismB");

    // The base name checks cleanly over the same input, so the rejection is
    // about the typed spelling and not about the element.
    assert_eq!(
        eval_over("Observation.value.exists()", Some(&observation)),
        vec![primitive("boolean", "true")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.008 -- the indexer selects by zero-based position
// ---------------------------------------------------------------------------

// covers: path.navigation.008
#[test]
fn the_indexer_selects_by_zero_based_position() {
    assert_eq!(
        eval_patient("Patient.name[0].given[0]"),
        vec![primitive("string", "Peter")]
    );
    assert_eq!(
        eval_patient("Patient.name[1].given[0]"),
        vec![primitive("string", "Jim")]
    );

    // Zero-based: `[0]` is the first item and `[1]` the second, which the
    // group's own cases grade as well.
    expect_group("testIndexer", 3);
}

// ---------------------------------------------------------------------------
// path.navigation.009 -- an out-of-range index yields empty
// ---------------------------------------------------------------------------

// covers: path.navigation.009
#[test]
fn an_out_of_range_index_yields_empty() {
    // `funcItem` (FHIRPathEngine.java:6277) adds an item only when the parsed
    // index is less than the focus size, so an index past the end contributes
    // nothing and is not an error.
    assert_eq!(eval_patient("Patient.name[99]"), empty());
    assert_eq!(eval_patient("Patient.name[3]"), empty());

    // The top of the range is exclusive: there are three names, so `[2]` is
    // the last one and `[3]` is empty.
    assert_eq!(
        eval_patient("Patient.name[2].given"),
        strings(&["Peter", "James"])
    );
    assert_eq!(
        eval_patient("Patient.name[99].exists()"),
        vec![primitive("boolean", "false")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.010 -- a negative index yields empty
// ---------------------------------------------------------------------------

// covers: path.navigation.010
#[test]
fn a_negative_index_yields_empty() {
    // Written to the scenario, which states empty.
    //
    // Contradiction with the pinned engine, recorded rather than silently
    // accommodated: `funcItem` at FHIRPathEngine.java:6277-6283 guards only the
    // upper bound -- `Utilities.isInteger(s) && Integer.parseInt(s) <
    // focus.size()` -- and `Utilities.isInteger` accepts a leading `-`
    // (`Utilities.java::isInteger`). So `-1 < 3` holds, the pinned engine
    // reaches `focus.get(-1)` and raises `IndexOutOfBoundsException`. The
    // suite's own `testIndexer/testIndexer3` expects empty and carries no
    // `invalid` attribute, so HAPI fails that case; Frevo passing it is
    // `ahead-of-hapi` under `path.oracle.012`, not a parity break.
    assert_eq!(eval_patient("Patient.name[-1]"), empty());
    assert_eq!(eval_patient("Patient.name[-99]"), empty());

    // Empty rather than a failure is the half of the scenario the suite case
    // grades.
    assert!(output_bodies("testIndexer", "testIndexer3").is_empty());
    expect_case("testIndexer", "testIndexer3");
}

// ---------------------------------------------------------------------------
// path.navigation.011 -- a non-integer index is an execution error
// ---------------------------------------------------------------------------

// covers: path.navigation.011
#[test]
fn a_non_integer_index_is_an_execution_error() {
    // Written to the scenario, which states that evaluation fails.
    //
    // Contradiction with the pinned engine, recorded: `funcItem`
    // (FHIRPathEngine.java:6279) tests `Utilities.isInteger(s)` and, when the
    // index is not an integer, falls through to `return result` with nothing
    // added -- so the pinned engine answers **empty** for `Patient.name['x']`
    // rather than failing, and no suite case grades either answer. The test is
    // written to the scenario as approved; if the owner resolves the conflict
    // the other way it becomes a `DEV-nnnn` entry and this assertion inverts.
    let patient = load_input("patient-example.xml");
    let message = expect_evaluation_failure("Patient.name['x']", Some(&patient));
    assert!(
        !message.is_empty(),
        "an execution error must carry a message"
    );

    // A digit inside a string is the same story: the index is a string,
    // whatever it contains.
    expect_evaluation_failure("Patient.name['1']", Some(&patient));
}

// ---------------------------------------------------------------------------
// path.navigation.012 -- a contained resource's elements are navigable
// ---------------------------------------------------------------------------

// covers: path.navigation.012
#[test]
fn a_contained_resources_elements_are_navigable() {
    let container = load_input("patient-container-example.json");
    assert_eq!(container.resource_type(), Some("Patient"));
    assert!(
        !container.is_xml(),
        "this scenario's input is the JSON one, which is also what grades \
         path.oracle.004"
    );

    assert_eq!(
        eval_over("contained.id", Some(&container)),
        vec![primitive("id", "1")]
    );
    assert_eq!(output_bodies("miscEngineTests", "testContainedId"), ["1"]);
    expect_case("miscEngineTests", "testContainedId");

    // The contained resource is navigable beyond its id, so this is navigation
    // into it rather than a special case for `contained.id`.
    assert_eq!(
        eval_over("contained.ofType(Organization).id", Some(&container)),
        vec![primitive("id", "1")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.013 -- Resource.id yields the logical id
// ---------------------------------------------------------------------------

// covers: path.navigation.013
#[test]
fn resource_id_yields_the_logical_id_and_not_the_full_url() {
    // Upstream: `org.hl7.fhir.r4.test.FHIRPathTests.testEvaluate_Id`
    // (FHIRPathTests.java:349) sets the id from
    // `http://base/Patient/123/_history/222` and asserts one result of `123`.
    // `filterIdType` (FHIRPathEngine.java:280) is what does it:
    // `toUnqualifiedVersionless().withResourceType(null)`.
    //
    // No suite input carries such an id, so this is driven over the published
    // `Node` seam. `evaluate_over_node` is `NotImplemented` today.
    let patient = TestNode::resource(
        "Patient",
        vec![(
            "id".to_string(),
            TestNode::prim("id", "http://base/Patient/123/_history/222"),
        )],
    );

    let mut engine = node_engine_under_test();
    let results = engine
        .evaluate_over_node("Patient.id", patient.clone())
        .unwrap_or_else(|e| panic!("`Patient.id` must evaluate over an in-memory Patient: {e}"));
    let rendered: Vec<String> = results.iter().map(describe).collect();
    assert_eq!(
        rendered,
        ["id:123"],
        "the base URL, the resource type and the version must all be stripped"
    );

    // The untouched element really did carry the full URL, so the test is about
    // the engine and not about the fixture.
    let raw = patient
        .children("id")
        .expect("the fixture's id navigates")
        .first()
        .map(describe)
        .expect("the fixture has an id");
    assert_eq!(raw, "id:http://base/Patient/123/_history/222");
}

// ---------------------------------------------------------------------------
// path.navigation.014 -- navigation into a primitive's extension
// ---------------------------------------------------------------------------

// covers: path.navigation.014
#[test]
fn navigation_into_a_primitives_extension_works() {
    // `patient-example.xml:96-100` carries one extension on `birthDate`, which
    // is the `_birthDate` sibling in the JSON form. Reaching it requires the
    // primitive value to keep its sibling element, which is what
    // `frevo_path::Primitive` carries (`design.md` section 4.1).
    const BIRTH_TIME: &str = "http://hl7.org/fhir/StructureDefinition/patient-birthTime";

    assert_eq!(
        eval_patient("Patient.birthDate.extension.url"),
        vec![primitive("uri", BIRTH_TIME)]
    );

    // The extension's value is reachable too, so navigation did not stop at the
    // url.
    assert_eq!(
        eval_patient("Patient.birthDate.extension.value.exists()"),
        vec![primitive("boolean", "true")]
    );

    // The suite grades the same extension through `extension(url)`, which is
    // the other way in (`path.utility.011`).
    expect_case("testExtension", "testExtension1");

    // The primitive keeps its own value: reaching the extension must not
    // replace the date with its sibling.
    assert_eq!(
        eval_patient("Patient.birthDate"),
        vec![primitive("date", "1974-12-25")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.015 -- `$this` is the current item
// ---------------------------------------------------------------------------

// covers: path.navigation.015
#[test]
fn dollar_this_is_the_current_item() {
    assert_eq!(
        eval_patient("Patient.name.given.where(substring($this.length()-3) = 'ter')"),
        strings(&["Peter", "Peter"])
    );
    assert_eq!(
        output_bodies("testDollar", "testDollarThis2"),
        ["Peter", "Peter"]
    );
    expect_case("testDollar", "testDollarThis2");

    // `$this` is rebound per item -- `funcWhere` (FHIRPathEngine.java:6249)
    // calls `changeThis(context, item)` for each one -- so it is the item and
    // not the whole collection, which is why `$this.length()` is a singleton
    // there.
    assert_eq!(
        eval_patient("Patient.name.given.where($this = 'Jim')"),
        strings(&["Jim"])
    );
}

// ---------------------------------------------------------------------------
// path.navigation.016 -- `$this` with no match yields empty
// ---------------------------------------------------------------------------

// covers: path.navigation.016
#[test]
fn dollar_this_with_no_match_yields_empty() {
    assert_eq!(
        eval_patient("Patient.name.given.where(substring($this.length()-3) = 'out')"),
        empty()
    );
    assert!(output_bodies("testDollar", "testDollarThis1").is_empty());
    expect_case("testDollar", "testDollarThis1");

    // Empty, not an error, even though the criteria is false for every item.
    assert_eq!(
        eval_patient("Patient.name.given.where($this = 'nobody').empty()"),
        vec![primitive("boolean", "true")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.017 -- `$index` is the current item's position
// ---------------------------------------------------------------------------

// covers: path.navigation.017
#[test]
fn dollar_index_is_the_current_items_position() {
    assert_eq!(
        eval_patient("Patient.telecom.select(iif(value='(03) 3410 5613', $index, {} ))"),
        vec![primitive("integer", "2")]
    );
    assert_eq!(output_bodies("index-part", "testIndex"), ["2"]);
    expect_group("index-part", 1);

    // `$index` is zero-based and set by `funcSelect`
    // (FHIRPathEngine.java:6263) through `setIndex(i)`, so the first item's
    // index is 0 and there are as many indices as items.
    assert_eq!(
        eval_patient("Patient.telecom.select($index).first()"),
        vec![primitive("integer", "0")]
    );
    assert_eq!(
        eval_patient("Patient.telecom.select($index).count() = Patient.telecom.count()"),
        vec![primitive("boolean", "true")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.018 -- `$total` is only defined inside `aggregate`
// ---------------------------------------------------------------------------

// covers: path.navigation.018
#[test]
fn dollar_total_outside_aggregate_fails() {
    // `$total` reads `context.getTotal()` (FHIRPathEngine.java:1478), and
    // `ExecutionContext.total` (:960) is set only by `funcAggregate`. Outside
    // an `aggregate()` argument it is null, so the pinned engine fails there --
    // with a `NullPointerException` rather than a diagnostic, which Frevo may
    // not do: `path.errors.003` requires every failure to be a returned error.
    // So the assertion is a `Diagnostic`, which is both what the scenario says
    // and what the constitution requires.
    let message = expect_evaluation_failure("$total", None);
    assert!(
        message.to_lowercase().contains("total"),
        "the failure must name $total: {message}"
    );

    expect_evaluation_failure("(1 | 2 | 3).select($total)", None);

    // Inside `aggregate()` it is defined, so the failure is about the scope and
    // not about the name. The suite grades those four cases
    // (`path.utility.005`, `path.utility.006`).
    assert_eq!(
        eval_over("(1|2|3).aggregate($this+$total, 0)", None),
        vec![primitive("integer", "6")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.019 -- navigation over an empty focus yields empty
// ---------------------------------------------------------------------------

// covers: path.navigation.019
#[test]
fn navigation_over_an_empty_focus_yields_empty() {
    // `case Name`'s per-item loop (FHIRPathEngine.java:1482) runs zero times
    // over an empty focus, so every step after an empty one is empty.
    assert_eq!(eval_over("{}.name.given", None), empty());
    assert_eq!(eval_over("{}.name", None), empty());
    assert_eq!(
        eval_over("{}.name.given.empty()", None),
        vec![primitive("boolean", "true")]
    );

    // Including when the empty focus arises from a real navigation rather than
    // from the `{}` literal.
    assert_eq!(eval_patient("Patient.name.prefix.period.start"), empty());
}

// ---------------------------------------------------------------------------
// path.navigation.020 -- `children()` returns the immediate children
// ---------------------------------------------------------------------------

// covers: path.navigation.020
#[test]
fn children_returns_the_immediate_children() {
    // The expected count comes from the fixture rather than from a literal:
    // `funcChildren` (FHIRPathEngine.java:4830) collects every child value of
    // every focus item, which for an XML-backed resource is exactly the root
    // element's child elements.
    let patient = load_input("patient-example.xml");
    let expected = match &patient.document {
        Document::Xml(d) => d.root.child_elements().len(),
        Document::Json(_) => panic!("patient-example.xml must load as XML"),
    };
    assert!(
        expected > 10,
        "the fixture must have a non-trivial number of children, not {expected}"
    );

    assert_eq!(
        eval_patient("Patient.children().count()"),
        vec![primitive("integer", &expected.to_string())]
    );

    // Immediate, not transitive: `descendants()` sees strictly more.
    assert_eq!(
        eval_patient("Patient.children().count() < Patient.descendants().count()"),
        vec![primitive("boolean", "true")]
    );

    // And the suite's own `children()` case, over a different input.
    expect_case("testRepeat", "testRepeat4");
}

// ---------------------------------------------------------------------------
// path.navigation.021 -- `descendants()` returns the transitive children
// ---------------------------------------------------------------------------

// covers: path.navigation.021
#[test]
fn descendants_returns_the_transitive_children() {
    // The grading selector is these six cases. The `testDollarResource` group
    // the delta's earlier wording named is commented out of the pinned suite
    // (`tests-fhir-r4.xml:1576-1582`) and is not one of the 1,191, which is why
    // it cannot be the selector.
    for name in [
        "testDistinct2",
        "testDistinct3",
        "testDistinct5",
        "testDistinct6",
    ] {
        expect_case("testDistinct", name);
    }
    expect_case("testRepeat", "testRepeat3");
    expect_case("testCombine()", "testCombine1");

    // The two cases that pin a count, so the transitive walk is graded by
    // number and not only by distinctness.
    assert_eq!(output_bodies("testDistinct", "testDistinct5"), ["10"]);
    let questionnaire = load_input("questionnaire-example.xml");
    assert_eq!(
        eval_over(
            "Questionnaire.descendants().linkId.distinct().count()",
            Some(&questionnaire)
        ),
        vec![primitive("integer", "10")]
    );
    assert_eq!(
        eval_over(
            "Questionnaire.descendants().code.count() = 23",
            Some(&questionnaire)
        ),
        vec![primitive("boolean", "true")]
    );

    // `descendants()` is `repeat(children())` (FHIRPathEngine.java:4810), so it
    // excludes the focus itself and includes every nested item.
    assert_eq!(
        eval_over(
            "Questionnaire.descendants().count() > Questionnaire.children().count()",
            Some(&questionnaire)
        ),
        vec![primitive("boolean", "true")]
    );
}

// ---------------------------------------------------------------------------
// path.navigation.022 -- `children()` produces an unordered collection
// ---------------------------------------------------------------------------

// covers: path.navigation.022
#[test]
fn children_produces_an_unordered_collection() {
    let patient = load_input("patient-example.xml");

    // `checkOrdered` (FHIRPathEngine.java:3845) throws whenever the focus's
    // collection status is UNORDERED, and `skip`'s call site is :3420. The rule
    // is unconditional: it reads no engine setting at all.
    let message = expect_check_failure(
        "Patient.children().skip(1)",
        &patient,
        StrictnessFlags::CHECK,
    );
    assert!(
        message.contains("skip"),
        "the ordered-only error must name the function: {message}"
    );

    // Unconditional means both strictness configurations reject it -- the
    // harness's check configuration and its evaluation configuration alike
    // (`path.strictness.002`).
    expect_check_failure(
        "Patient.children().skip(1)",
        &patient,
        StrictnessFlags::EVALUATE,
    );

    // The case carries `mode="strict"` and `checkOrderedFunctions="true"`,
    // which no R4 harness reads; the rejection must not be conditioned on
    // either, so the attributes are asserted present and the case asserted to
    // pass anyway.
    let (suite, _) = pinned();
    let case = suite
        .case("testDollar", "testDollarOrderNotAllowed")
        .expect("the case resolves");
    assert_eq!(case.test_mode.as_deref(), Some("strict"));
    assert!(case.check_ordered_functions);
    expect_case("testDollar", "testDollarOrderNotAllowed");
}

// ---------------------------------------------------------------------------
// path.navigation.023 -- document order across repeating elements
// ---------------------------------------------------------------------------

// covers: path.navigation.023
#[test]
fn navigation_preserves_document_order_across_repeating_elements() {
    assert_eq!(
        eval_patient("Patient.name.skip(1).given"),
        strings(&["Jim", "Peter", "James"])
    );
    assert_eq!(
        output_bodies("testDollar", "testDollarOrderAllowed"),
        ["Jim", "Peter", "James"]
    );
    expect_case("testDollar", "testDollarOrderAllowed");

    // Order is a property of the result, not of the comparison: the reversed
    // sequence is a different answer.
    assert_ne!(
        eval_patient("Patient.name.skip(1).given"),
        strings(&["James", "Peter", "Jim"]),
        "document order is asserted, not set membership"
    );
}

// ---------------------------------------------------------------------------
// path.navigation.024 -- the miscellaneous accessor cases
// ---------------------------------------------------------------------------

// covers: path.navigation.024
#[test]
fn the_miscellaneous_accessor_cases_all_hold() {
    expect_group("testMiscellaneousAccessorTests", 3);
    expect_group("testBasics", 8);
    expect_group("testObservations", 10);

    // Two properties of the first group are worth stating, because its two
    // `predicate` cases share one expression: `birthDate` with
    // `predicate="false"` compares as a date, and with `predicate="true"` as
    // the boolean `true` (`path.oracle.006`).
    assert_eq!(
        output_bodies("testMiscellaneousAccessorTests", "testExtractBirthDate"),
        ["@1974-12-25"],
        "the `@` prefix is part of the expectation and is never stripped"
    );
    assert_eq!(
        output_bodies("testMiscellaneousAccessorTests", "testPatientHasBirthDate"),
        ["true"]
    );

    // The value Frevo returns to a caller carries no prefix.
    assert_eq!(
        eval_patient("birthDate"),
        vec![primitive("date", "1974-12-25")]
    );
}
