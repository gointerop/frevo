//! Unit tests for the evaluator core, over an in-memory node and a stub model.
//!
//! These see what the integration tests under `tests/` cannot yet: those drive
//! every case through the static check pass first, which does not exist, so
//! they report the evaluator's behaviour only once it does. The cases here
//! are the navigation rules of task 31 (`path.navigation.*` in the spec
//! delta), stated over inputs small enough to read, plus the property that
//! every unimplemented feature is [`EvalError::Unsupported`] and never an
//! empty collection.

use std::sync::Arc;

use super::navigate::logical_id;
use super::{EvalError, Evaluator};
use crate::seam::model::Model;
use crate::seam::node::{Node, NodeError};
use crate::seam::value::{Primitive, Value};
use crate::syntax::ast::Expression;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TestNode {
    type_name: String,
    is_resource: bool,
    children: Vec<(String, Value)>,
    primitive: Option<Primitive>,
}

fn resource(type_name: &str, children: Vec<(&str, Value)>) -> Value {
    Value::Node(Arc::new(TestNode {
        type_name: type_name.to_string(),
        is_resource: true,
        children: children
            .into_iter()
            .map(|(n, v)| (n.to_string(), v))
            .collect(),
        primitive: None,
    }))
}

fn element(type_name: &str, children: Vec<(&str, Value)>) -> Value {
    Value::Node(Arc::new(TestNode {
        type_name: type_name.to_string(),
        is_resource: false,
        children: children
            .into_iter()
            .map(|(n, v)| (n.to_string(), v))
            .collect(),
        primitive: None,
    }))
}

fn prim(type_name: &str, lexical: &str) -> Value {
    prim_with(type_name, lexical, Vec::new())
}

fn prim_with(type_name: &str, lexical: &str, extensions: Vec<Value>) -> Value {
    Value::Node(Arc::new(TestNode {
        type_name: type_name.to_string(),
        is_resource: false,
        children: Vec::new(),
        primitive: Some(Primitive {
            type_name: type_name.to_string(),
            lexical: lexical.to_string(),
            id: None,
            extensions,
        }),
    }))
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

/// A node whose navigation fails, as a malformed document would.
#[derive(Debug)]
struct BrokenNode;

impl Node for BrokenNode {
    fn type_name(&self) -> &str {
        "Patient"
    }

    fn is_resource(&self) -> bool {
        true
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Err(NodeError::new(format!("Patient.{name}"), "malformed").at(7))
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Err(NodeError::new("Patient", "malformed").at(7))
    }

    fn primitive(&self) -> Option<Primitive> {
        None
    }
}

struct StubModel;

const HIERARCHY: &[(&str, Option<&str>)] = &[
    ("Resource", None),
    ("DomainResource", Some("Resource")),
    ("Patient", Some("DomainResource")),
    ("Observation", Some("DomainResource")),
    ("Organization", Some("DomainResource")),
    ("Element", None),
    ("HumanName", Some("Element")),
    ("Quantity", Some("Element")),
    ("Extension", Some("Element")),
    ("string", Some("Element")),
    ("id", Some("string")),
    ("uri", Some("Element")),
    ("date", Some("Element")),
];

impl Model for StubModel {
    fn is_type(&self, type_name: &str) -> bool {
        HIERARCHY.iter().any(|(t, _)| *t == type_name)
    }

    fn base_type(&self, type_name: &str) -> Option<&str> {
        HIERARCHY
            .iter()
            .find(|(t, _)| *t == type_name)
            .and_then(|(_, b)| *b)
    }

    fn element_types(&self, _type_name: &str, _element: &str) -> Option<Vec<String>> {
        None
    }

    fn is_primitive(&self, type_name: &str) -> bool {
        type_name.chars().next().is_some_and(char::is_lowercase)
    }

    fn is_resource_type(&self, type_name: &str) -> bool {
        matches!(type_name, "Patient" | "Observation" | "Organization")
    }
}

fn human_name(given: &[&str]) -> Value {
    element(
        "HumanName",
        given.iter().map(|g| ("given", prim("string", g))).collect(),
    )
}

/// A cut-down `patient-example.xml`: three names carrying the five given
/// names in the same order, a birth date with one extension, a contained
/// organisation, and no prefix anywhere.
fn patient() -> Value {
    let birth_time = element(
        "Extension",
        vec![
            (
                "url",
                prim(
                    "uri",
                    "http://hl7.org/fhir/StructureDefinition/patient-birthTime",
                ),
            ),
            ("value", prim("dateTime", "1974-12-25T14:35:45-05:00")),
        ],
    );
    resource(
        "Patient",
        vec![
            ("id", prim("string", "example")),
            (
                "contained",
                resource("Organization", vec![("id", prim("string", "1"))]),
            ),
            ("name", human_name(&["Peter", "James"])),
            ("name", human_name(&["Jim"])),
            ("name", human_name(&["Peter", "James"])),
            (
                "birthDate",
                prim_with("date", "1974-12-25", vec![birth_time]),
            ),
        ],
    )
}

fn observation() -> Value {
    resource(
        "Observation",
        vec![(
            "value",
            element(
                "Quantity",
                vec![
                    ("value", prim("decimal", "185")),
                    ("unit", prim("string", "lbs")),
                ],
            ),
        )],
    )
}

fn eval(text: &str, focus: Option<&Value>) -> Result<Vec<Value>, EvalError> {
    let expr = Expression::parse(text).unwrap_or_else(|e| panic!("{text:?} must parse: {e}"));
    Evaluator::new(&StubModel).evaluate(&expr, focus)
}

fn ok(text: &str, focus: Option<&Value>) -> Vec<String> {
    eval(text, focus)
        .unwrap_or_else(|e| panic!("{text:?} must evaluate: {e}"))
        .iter()
        .map(describe)
        .collect()
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

fn unsupported(text: &str, focus: Option<&Value>) -> String {
    match eval(text, focus) {
        Err(EvalError::Unsupported { feature, .. }) => feature,
        other => panic!("{text:?} must be Unsupported, not {other:?}"),
    }
}

fn execution_error(text: &str, focus: Option<&Value>) -> String {
    match eval(text, focus) {
        Err(EvalError::Execution { message, .. }) => message,
        other => panic!("{text:?} must be an execution error, not {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The leading type name
// ---------------------------------------------------------------------------

#[test]
fn a_leading_type_name_matches_the_focus_type_and_its_base_types() {
    let p = patient();
    assert_eq!(ok("Patient.name.given[0]", Some(&p)), ["string:Peter"]);
    assert_eq!(ok("DomainResource.id", Some(&p)), ["id:example"]);
    assert_eq!(ok("Resource.id", Some(&p)), ["id:example"]);
    assert_eq!(ok("`Patient`.name[1].given", Some(&p)), ["string:Jim"]);
}

#[test]
fn a_leading_type_name_that_is_not_the_focus_type_yields_empty() {
    let p = patient();
    assert!(ok("Observation.value", Some(&p)).is_empty());
    assert!(ok("Encounter.status", Some(&p)).is_empty());
}

#[test]
fn an_unknown_type_name_compares_with_the_type_name_alone() {
    // Upstream's logical-model branch: a type the model does not know matches
    // only by name.
    let logical = resource("Widget", vec![("part", prim("string", "a"))]);
    assert_eq!(ok("Widget.part", Some(&logical)), ["string:a"]);
    assert!(ok("Gadget.part", Some(&logical)).is_empty());
}

#[test]
fn the_type_name_may_be_omitted() {
    let p = patient();
    assert_eq!(
        ok("name.given", Some(&p)),
        ok("Patient.name.given", Some(&p))
    );
}

#[test]
fn an_upper_case_name_after_the_entry_is_navigated_not_type_tested() {
    let p = patient();
    assert!(ok("name.Patient", Some(&p)).is_empty());
}

// ---------------------------------------------------------------------------
// Flattening, absence, order, empty focus
// ---------------------------------------------------------------------------

#[test]
fn navigation_flattens_in_document_order() {
    let p = patient();
    assert_eq!(
        ok("Patient.name.given", Some(&p)),
        [
            "string:Peter",
            "string:James",
            "string:Jim",
            "string:Peter",
            "string:James"
        ]
    );
}

#[test]
fn an_absent_element_is_empty_not_an_error() {
    let p = patient();
    assert!(ok("Patient.name.prefix", Some(&p)).is_empty());
    assert!(ok("Patient.name.prefix.period.start", Some(&p)).is_empty());
}

#[test]
fn navigation_over_an_empty_focus_is_empty() {
    assert!(ok("{}.name.given", None).is_empty());
    assert!(ok("name", None).is_empty());
    assert!(ok("{}", None).is_empty());
}

#[test]
fn a_literal_has_no_children() {
    assert!(ok("'abc'.length", None).is_empty());
    assert!(ok("(1).extension", None).is_empty());
}

#[test]
fn a_navigation_failure_propagates_as_an_error() {
    let broken = Value::Node(Arc::new(BrokenNode));
    match eval("Patient.name", Some(&broken)) {
        Err(EvalError::Node(e)) => assert_eq!(e.offset, Some(7)),
        other => panic!("a model failure must propagate, not {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Polymorphic elements
// ---------------------------------------------------------------------------

#[test]
fn a_choice_element_is_reached_by_its_base_name_and_not_by_the_typed_name() {
    let o = observation();
    assert_eq!(ok("Observation.value.unit", Some(&o)), ["string:lbs"]);
    // Polymorphic typed names are off, as in the suite's harness: the typed
    // spelling navigates nothing at runtime, and the check pass rejects it.
    assert!(ok("Observation.valueQuantity", Some(&o)).is_empty());
}

// ---------------------------------------------------------------------------
// The indexer
// ---------------------------------------------------------------------------

#[test]
fn the_indexer_is_zero_based() {
    let p = patient();
    assert_eq!(ok("Patient.name[0].given[0]", Some(&p)), ["string:Peter"]);
    assert_eq!(ok("Patient.name[1].given[0]", Some(&p)), ["string:Jim"]);
    assert_eq!(
        ok("Patient.name[2].given", Some(&p)),
        ["string:Peter", "string:James"]
    );
}

#[test]
fn an_out_of_range_index_selects_nothing_and_a_negative_one_is_an_execution_error() {
    let p = patient();
    assert!(ok("Patient.name[3]", Some(&p)).is_empty());
    assert!(ok("Patient.name[99]", Some(&p)).is_empty());
    // DEV-0012: upstream raises IndexOutOfBoundsException for a negative index.
    execution_error("Patient.name[-1]", Some(&p));
    execution_error("Patient.name[-99]", Some(&p));
    assert!(ok("Patient.name[{}]", Some(&p)).is_empty());
}

#[test]
fn a_non_integer_index_is_an_execution_error() {
    // DEV-0013: upstream returns empty for both, and converts '1' to 1.
    let p = patient();
    let message = execution_error("Patient.name['x']", Some(&p));
    assert!(message.contains("integer"), "{message}");
    execution_error("Patient.name['1']", Some(&p));
    execution_error("Patient.name[1.0]", Some(&p));
    execution_error("Patient.name[true]", Some(&p));
}

#[test]
fn an_integer_typed_node_is_an_index() {
    // The index is evaluated against the indexer's own focus -- the `name`
    // collection -- so `n` is navigated from each name. Only the first carries
    // one, which makes the index the single `positiveInt` 1.
    let first = element(
        "HumanName",
        vec![
            ("n", prim("positiveInt", "1")),
            ("given", prim("string", "A")),
        ],
    );
    let focus = resource(
        "Patient",
        vec![("name", first), ("name", human_name(&["B"]))],
    );
    assert_eq!(ok("name[n].given", Some(&focus)), ["string:B"]);
}

#[test]
fn an_index_with_more_than_one_item_is_an_execution_error() {
    let p = patient();
    execution_error("Patient.name[given]", Some(&p));
}

// ---------------------------------------------------------------------------
// Contained resources and the logical id
// ---------------------------------------------------------------------------

#[test]
fn a_contained_resources_elements_are_navigable() {
    let p = patient();
    assert_eq!(ok("contained.id", Some(&p)), ["id:1"]);
}

#[test]
fn resource_id_yields_the_logical_id() {
    let p = resource(
        "Patient",
        vec![("id", prim("id", "http://base/Patient/123/_history/222"))],
    );
    assert_eq!(ok("Patient.id", Some(&p)), ["id:123"]);
    // Through children() as well: `filterIdType` sits in getChildrenByName.
    assert_eq!(ok("Patient.children()", Some(&p)), ["id:123"]);
}

#[test]
fn an_id_typed_element_is_filtered_and_loses_its_own_extensions() {
    let ext = element("Extension", vec![("url", prim("uri", "http://x"))]);
    let focus = element(
        "HumanName",
        vec![("key", prim_with("id", "a/b", vec![ext]))],
    );
    assert_eq!(ok("key", Some(&focus)), ["id:b"]);
    assert!(ok("key.extension", Some(&focus)).is_empty());
}

#[test]
fn a_string_element_named_id_below_a_resource_is_not_filtered() {
    // `Element.id` is a `StringType`, not an `IdType`.
    let focus = element("HumanName", vec![("id", prim("string", "a/b"))]);
    assert_eq!(ok("id", Some(&focus)), ["string:a/b"]);
}

#[test]
fn logical_id_follows_id_type_set_value() {
    let cases: &[(&str, Option<&str>)] = &[
        ("example", Some("example")),
        ("Patient/123", Some("123")),
        ("Patient/123/_history/2", Some("123")),
        ("http://base/Patient/123/_history/222", Some("123")),
        ("123/_history/2", Some("123")),
        ("#local", Some("#local")),
        ("urn:uuid:1234", Some("urn:uuid:1234")),
        ("http://example.org/foo", None),
        ("Patient/", None),
        ("", None),
        ("   ", None),
        ("#", Some("#")),
    ];
    for (input, expected) in cases {
        assert_eq!(
            logical_id(input).as_deref(),
            *expected,
            "logical id of {input:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Primitive extensions
// ---------------------------------------------------------------------------

#[test]
fn a_primitives_extension_is_navigable_from_its_sibling() {
    let p = patient();
    assert_eq!(
        ok("Patient.birthDate.extension.url", Some(&p)),
        ["uri:http://hl7.org/fhir/StructureDefinition/patient-birthTime"]
    );
    assert_eq!(
        ok("Patient.birthDate.extension.value", Some(&p)),
        ["dateTime:1974-12-25T14:35:45-05:00"]
    );
    assert_eq!(ok("Patient.birthDate", Some(&p)), ["date:1974-12-25"]);
}

// ---------------------------------------------------------------------------
// $this, $index, $total
// ---------------------------------------------------------------------------

#[test]
fn dollar_this_at_the_entry_is_the_focus() {
    let p = patient();
    assert_eq!(ok("$this.name[1].given", Some(&p)), ["string:Jim"]);
    assert!(ok("$this", None).is_empty());
}

#[test]
fn dollar_index_outside_select_is_zero() {
    // `ExecutionContext.index` starts at 0 and only `select` moves it.
    assert_eq!(ok("$index", None), ["integer:0"]);
}

#[test]
fn dollar_total_outside_aggregate_is_an_execution_error() {
    let message = execution_error("$total", None);
    assert!(message.contains("$total"), "{message}");
}

// ---------------------------------------------------------------------------
// children() and descendants()
// ---------------------------------------------------------------------------

#[test]
fn children_returns_every_immediate_child_in_order() {
    let p = patient();
    let kids = ok("Patient.children()", Some(&p));
    assert_eq!(kids.len(), 6);
    assert_eq!(kids[0], "id:example");
    assert_eq!(kids[5], "date:1974-12-25");
}

#[test]
fn children_of_a_primitive_are_its_sibling_extensions() {
    let p = patient();
    assert_eq!(
        ok("Patient.birthDate.children()", Some(&p)),
        ["Extension:<element>"]
    );
}

#[test]
fn descendants_is_the_transitive_walk_without_the_focus() {
    let p = patient();
    let all = ok("Patient.descendants()", Some(&p));
    // id, contained, contained.id, 3 names, 5 given, birthDate, the extension,
    // its url and its value.
    assert_eq!(all.len(), 6 + 1 + 5 + 1 + 2);
    assert!(!all.contains(&"Patient:<element>".to_string()));
    assert_eq!(
        ok("Patient.descendants().given[2]", Some(&p)),
        ["string:Jim"]
    );
}

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

#[test]
fn literals_evaluate_to_themselves() {
    assert_eq!(ok("1", None), ["integer:1"]);
    assert_eq!(ok("'a'", None), ["string:a"]);
    assert_eq!(ok("1.50", None), ["decimal:1.50"]);
    assert_eq!(ok("true", None), ["boolean:true"]);
    assert_eq!(ok("@2015-02-04", None), ["date:2015-02-04"]);
    assert_eq!(ok("-1", None), ["integer:-1"]);
    assert_eq!(ok("-1.50", None), ["decimal:-1.50"]);
    assert_eq!(ok("+2", None), ["integer:2"]);
    assert_eq!(ok("(1)", None), ["integer:1"]);
}

#[test]
fn a_time_literal_with_an_offset_is_an_execution_error() {
    execution_error("@T10:30:00Z", None);
}

// ---------------------------------------------------------------------------
// The did-not-run property
// ---------------------------------------------------------------------------

#[test]
fn every_unimplemented_function_is_unsupported_and_never_empty() {
    let p = patient();
    // `memberOf`, `conformsTo`, `resolve()` (task 49) and `comparable()` are
    // built; these three are the only functions of the R4 table the evaluator
    // does not dispatch (every `Function` variant checked against the
    // `eval/` dispatch).
    for text in [
        "Patient.hasTemplateIdOf('http://x')",
        "Patient.text.htmlChecks()",
        "Patient.text.htmlChecks2()",
    ] {
        let feature = unsupported(text, Some(&p));
        assert!(feature.contains("function"), "{text}: {feature}");
    }
    // `comparable()` is built in full now (tasks 43, 44): two different UCUM
    // codes answer through `isComparable` (path.quantity.014).
    assert_eq!(
        ok("1 'cm'.comparable(1 '[in_i]')", Some(&p)),
        ["boolean:true"]
    );
    assert_eq!(ok("1 'cm'.comparable(1 's')", Some(&p)), ["boolean:false"]);
}

#[test]
fn every_binary_operator_is_unsupported_and_never_empty() {
    // Every operator is built now (`memberOf` was the last, task 49), so none
    // can be Unsupported as an operator. The property that survives is the
    // same guard against a silent empty result: a built operator applied to
    // operands whose rule is not is still Unsupported, never empty, and the
    // message names the missing rule.
    let p = patient();
    let text = "Patient.gender memberOf 'http://x'";
    let feature = unsupported(text, Some(&p));
    assert!(feature.contains("value sets"), "{text}: {feature}");
    // Quantity `/`, `*`, `<` and `~` are built (tasks 43, 44), and decimal
    // arithmetic carries any number of significant digits (path.literal.017):
    // operands that once needed more than 38 digits answer exactly, as
    // BigDecimal does, never truncated or empty.
    assert_eq!(
        ok(
            "1.0000000000000000000001 * 1.0000000000000000000001",
            Some(&p)
        ),
        ["decimal:1.00000000000000000000020000000000000000000001"]
    );
    assert_eq!(
        ok("99999999999999999999999999999999999999 + 1.5", Some(&p)),
        ["decimal:100000000000000000000000000000000000000.5"]
    );
    // What is still Unsupported is a decimal whose scale would shift its
    // digits by more than `MAX_SHIFT` places (DEBT-0014): never an empty
    // answer, and the message names the limit.
    let text = "1.5.round(20000)";
    let feature = unsupported(text, Some(&p));
    assert!(
        feature.contains("more than 10000 places"),
        "{text}: {feature}"
    );
}

#[test]
fn a_unary_sign_over_an_unbuilt_operation_is_unsupported() {
    // A sign over an operation that is still Unsupported (a scale shift past
    // `MAX_SHIFT`, DEBT-0014) surfaces it rather than negating an empty
    // result.
    unsupported("-(1.5.round(20000))", None);
    // A sign over the formerly refused wide decimals negates the exact result.
    assert_eq!(
        ok(
            "-(1.0000000000000000000001 * 1.0000000000000000000001)",
            None
        ),
        ["decimal:-1.00000000000000000000020000000000000000000001"]
    );
    assert_eq!(
        ok("-(99999999999999999999999999999999999999 + 1.5)", None),
        ["decimal:-100000000000000000000000000000000000000.5"]
    );
}

#[test]
fn an_unknown_environment_variable_is_an_error_and_never_empty() {
    // Every environment variable is built now, so none can be Unsupported; the
    // property that survives is the same guard against a silent empty result:
    // a name the engine does not know is an execution error, not an empty
    // collection.
    let p = patient();
    for text in ["%foo", "%vs", "%ext"] {
        let message = execution_error(text, Some(&p));
        assert!(
            message.contains("Invalid FHIRPath Constant"),
            "{text}: {message}"
        );
    }
}

#[test]
fn an_unsupported_feature_deep_inside_navigation_still_surfaces() {
    // Navigation that reaches nothing must not hide the unsupported step after
    // it: the error is raised whether or not the focus is empty.
    unsupported("{}.name.hasTemplateIdOf('http://x')", None);
    unsupported(
        "Patient.name.prefix.hasTemplateIdOf('http://x')",
        Some(&patient()),
    );
}

// ---------------------------------------------------------------------------
// Reuse
// ---------------------------------------------------------------------------

#[test]
fn a_parsed_expression_is_shared_and_reused_without_reparsing() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Expression>();

    let expr = Expression::parse("Patient.name.given").expect("parses");
    let p = patient();
    let o = observation();
    let evaluator = Evaluator::new(&StubModel);
    let first: Vec<String> = evaluator
        .evaluate(&expr, Some(&p))
        .expect("evaluates")
        .iter()
        .map(describe)
        .collect();
    let other = evaluator.evaluate(&expr, Some(&o)).expect("evaluates");
    let again: Vec<String> = evaluator
        .evaluate(&expr, Some(&p))
        .expect("evaluates")
        .iter()
        .map(describe)
        .collect();
    assert_eq!(first, again);
    assert!(other.is_empty());

    let shared = Arc::new(expr);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || {
                let p = patient();
                Evaluator::new(&StubModel)
                    .evaluate(&shared, Some(&p))
                    .map(|v| v.len())
            })
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().expect("joins").expect("evaluates"), 5);
    }
}

// ---------------------------------------------------------------------------
// Collection predicates (task 32)
// ---------------------------------------------------------------------------

#[test]
fn not_of_an_empty_collection_is_empty_and_all_of_one_is_true() {
    assert!(ok("{}.not()", None).is_empty());
    assert_eq!(ok("true.not()", None), ["boolean:false"]);
    assert_eq!(ok("{}.all(false)", None), ["boolean:true"]);
    assert_eq!(ok("{}.count()", None), ["integer:0"]);
    assert_eq!(ok("{}.empty()", None), ["boolean:true"]);
    assert_eq!(ok("{}.exists()", None), ["boolean:false"]);
}

#[test]
fn a_single_non_boolean_criteria_result_is_true_and_several_are_an_error() {
    let p = patient();
    assert_eq!(ok("Patient.name.exists('x')", Some(&p)), ["boolean:true"]);
    execution_error("Patient.name.exists(given)", Some(&p));
}

#[test]
fn a_boolean_fold_over_a_non_boolean_names_the_item() {
    let message = execution_error("'foo'.allTrue()", None);
    assert!(message.contains("foo"), "{message}");
    assert_eq!(ok("true.allTrue()", None), ["boolean:true"]);
    assert_eq!(ok("false.anyFalse()", None), ["boolean:true"]);
}

// ---------------------------------------------------------------------------
// Filtering, projection and subsetting (task 33)
// ---------------------------------------------------------------------------

#[test]
fn single_fails_for_an_empty_focus_as_well_as_for_a_long_one() {
    let p = patient();
    assert!(execution_error("{}.single()", None).contains("single"));
    assert!(execution_error("Patient.name.given.single()", Some(&p)).contains("single"));
    assert_eq!(ok("1.single()", None), ["integer:1"]);
}

#[test]
fn skip_negative_or_empty_and_take_empty_are_execution_errors() {
    execution_error("1.skip(-1)", None);
    execution_error("1.skip({})", None);
    execution_error("1.take({})", None);
    assert!(ok("1.take(-1)", None).is_empty());
    assert_eq!(ok("1.skip(0)", None), ["integer:1"]);
    assert!(ok("1.skip(1)", None).is_empty());
    assert!(ok("{}.tail()", None).is_empty());
}

#[test]
fn of_type_rejects_an_unknown_namespace_and_a_base_type_for_a_primitive() {
    assert!(execution_error("1.ofType(System.Patient)", None).contains("not valid"));
    assert_eq!(ok("1.ofType(System.Integer)", None), ["integer:1"]);
    // The stub model knows no primitive, so a `FHIR.` primitive name is an
    // unknown type to it.
    assert!(execution_error("1.ofType(FHIR.integer)", None).contains("not valid"));
}

#[test]
fn intersect_deduplicates_and_exclude_does_not() {
    assert_eq!(ok("1.intersect(1)", None), ["integer:1"]);
    assert_eq!(ok("1.exclude(2)", None), ["integer:1"]);
    assert!(ok("1.exclude(1)", None).is_empty());
}

// ---------------------------------------------------------------------------
// Combining and membership (task 34)
// ---------------------------------------------------------------------------

#[test]
fn union_deduplicates_and_combine_does_not() {
    assert_eq!(ok("(1 | 1 | 2)", None), ["integer:1", "integer:2"]);
    assert_eq!(ok("1.union(1)", None), ["integer:1"]);
    assert_eq!(ok("1.combine(1)", None), ["integer:1", "integer:1"]);
    assert_eq!(ok("1.combine(1).union(2).count()", None), ["integer:2"]);
}

#[test]
fn in_and_contains_disagree_about_an_empty_right_operand() {
    assert_eq!(ok("1 in (1 | 2)", None), ["boolean:true"]);
    assert_eq!(ok("(1 | 2) contains 3", None), ["boolean:false"]);
    assert!(ok("{} in (1 | 2)", None).is_empty());
    assert_eq!(ok("1 in {}", None), ["boolean:false"]);
    assert!(ok("{} contains 1", None).is_empty());
    assert!(ok("(1 | 2) contains {}", None).is_empty());
}

#[test]
fn equality_reaches_the_decimal_rule_and_the_canonical_quantity_rule() {
    assert_eq!(ok("1 = 1.0", None), ["boolean:true"]);
    assert_eq!(ok("1 != 2", None), ["boolean:true"]);
    assert!(ok("{} = 1", None).is_empty());
    assert_eq!(ok("1 = 'a'", None), ["boolean:false"]);
    assert_eq!(ok("@2015 = @2015", None), ["boolean:true"]);
    // Quantities compare their canonical forms (path.quantity.022, .019):
    // different canonical units are `false`, not empty.
    assert_eq!(ok("1 'mg' = 1 'mg'", None), ["boolean:true"]);
    assert_eq!(ok("4 'g' = 4000 'mg'", None), ["boolean:true"]);
    assert_eq!(ok("4 'g' = 4040 'mg'", None), ["boolean:false"]);
    assert_eq!(ok("1 'cm' = 1 's'", None), ["boolean:false"]);
}
