//! Combining: `path.combine.001` through `path.combine.006`.
//!
//! ```text
//! cargo test -p frevo-path --test path_combine
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Four of the six scenarios name a
//! suite group and are driven through the committed harness; the other two
//! state an expression and its result.
//!
//! The duplicate handling is the substance of the requirement, and the two
//! functions differ on purpose:
//!
//! | Construct | Duplicates | Site |
//! |---|---|---|
//! | `a | b`, `union()` | removed | `funcUnion`, FHIRPathEngine.java:5036 |
//! | `combine()` | kept | `funcCombine`, :5051 |
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{CaseContext, EngineConfig, StrictnessFlags, SuiteEngine, engine_under_test};
use oracle::input::{self, SuiteInput};
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

fn eval_over(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let resource_type = inp.and_then(|i| i.resource_type());
    let context = CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    };
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

// ---------------------------------------------------------------------------
// path.combine.001 -- the union operator removes duplicates
// ---------------------------------------------------------------------------

// covers: path.combine.001
#[test]
fn the_union_operator_unions_and_removes_duplicates() {
    expect_group("testUnion", 12);

    // `funcUnion` (FHIRPathEngine.java:5036) adds an item only when it is not
    // already present, so the operator is set union and not concatenation.
    assert_eq!(eval_over("(1 | 2 | 3).count()", None), integer("3"));
    assert_eq!(eval_over("(1 | 2 | 2).count()", None), integer("2"));
    assert_eq!(eval_over("(1|1).count()", None), integer("1"));

    // Deduplication uses the same equality as `=`, so two dateTimes written
    // identically at minute precision are one item -- which is `testUnion12`.
    assert_eq!(
        eval_over("(@2015-02-04T10:00 | @2015-02-04T10:00).count()", None),
        integer("1")
    );

    // It is associative over the operator and the function spelling alike.
    assert_eq!(eval_over("1.union(2).union(3).count()", None), integer("3"));
    assert_eq!(eval_over("1.union(2.union(3)).count()", None), integer("3"));
}

// ---------------------------------------------------------------------------
// path.combine.002 -- `union()` behaves as the operator
// ---------------------------------------------------------------------------

// covers: path.combine.002
#[test]
fn union_the_function_behaves_as_the_union_operator() {
    assert_eq!(
        eval_over("(1 | 2).union(2 | 3).count()", None),
        integer("3")
    );

    // The function and the operator are the same code path, so the two
    // spellings must agree item for item and not only in count.
    assert_eq!(
        eval_over("(1 | 2).union(2 | 3)", None),
        eval_over("1 | 2 | 2 | 3", None)
    );
    assert_eq!(
        eval_over("(1 | 2).union(2 | 3) = (1 | 2 | 3)", None),
        boolean(true)
    );
    expect_case("testUnion", "testUnion4");
    expect_case("testUnion", "testUnion5");

    // And `union` after `combine` deduplicates what `combine` kept, which is
    // `testUnion8`.
    assert_eq!(
        eval_over("1.combine(1).union(2).count()", None),
        integer("2")
    );
}

// ---------------------------------------------------------------------------
// path.combine.003 -- `combine` concatenates and keeps duplicates
// ---------------------------------------------------------------------------

// covers: path.combine.003
#[test]
fn combine_concatenates_and_keeps_duplicates() {
    expect_group("testCombine()", 3);

    // `funcCombine` (FHIRPathEngine.java:5051) adds every item of both
    // collections unconditionally, so duplicates survive and the count is the
    // sum of the two sizes.
    assert_eq!(eval_over("1.combine(1).count()", None), integer("2"));
    assert_eq!(eval_over("(1 | 2).combine(2).count()", None), integer("3"));

    // The group's two `name` cases keep the five given names and append the two
    // family names, then drop `Jim` -- six items, with `Peter` and `James`
    // appearing twice each. Order is concatenation order.
    assert_eq!(
        eval_patient("name.given.combine(name.family).exclude('Jim')"),
        vec![
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Peter".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "James".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Peter".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "James".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Chalmers".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Windsor".into()
            },
        ]
    );

    // And the first case grades `combine` over a transitive walk, which is the
    // one case in the group with a different input file
    // (`codesystem-example.xml`).
    let (suite, _) = pinned();
    assert_eq!(
        suite
            .case("testCombine()", "testCombine1")
            .expect("the case resolves")
            .inputfile
            .as_deref(),
        Some("codesystem-example.xml")
    );
    expect_case("testCombine()", "testCombine1");
}

// ---------------------------------------------------------------------------
// path.combine.004 -- `in` tests membership
// ---------------------------------------------------------------------------

// covers: path.combine.004
#[test]
fn in_tests_membership() {
    expect_group("testIn", 4);

    // `opIn` (FHIRPathEngine.java:2630): every item of the left must be found
    // in the right, compared with `doEquals`.
    assert_eq!(eval_over("1 in (1 | 2 | 3)", None), boolean(true));
    assert_eq!(eval_over("1 in (2 | 3)", None), boolean(false));
    assert_eq!(eval_over("'a' in ('a' | 'c' | 'd')", None), boolean(true));
    assert_eq!(eval_over("'b' in ('a' | 'c' | 'd')", None), boolean(false));

    // The left operand may be a collection, and then every item must be found.
    assert_eq!(eval_over("(1 | 2) in (1 | 2 | 3)", None), boolean(true));
    assert_eq!(eval_over("(1 | 9) in (1 | 2 | 3)", None), boolean(false));
}

// ---------------------------------------------------------------------------
// path.combine.005 -- `contains` tests membership the other way round
// ---------------------------------------------------------------------------

// covers: path.combine.005
#[test]
fn contains_tests_membership_the_other_way_round() {
    expect_group("testContainsCollection", 4);

    // `opContains` (FHIRPathEngine.java:2655) is `in` with the operands
    // swapped, which the group grades with exactly the same four pairs as
    // `testIn`.
    assert_eq!(eval_over("(1 | 2 | 3) contains 1", None), boolean(true));
    assert_eq!(eval_over("(2 | 3) contains 1", None), boolean(false));
    assert_eq!(
        eval_over("('a' | 'c' | 'd') contains 'a'", None),
        boolean(true)
    );
    assert_eq!(
        eval_over("('a' | 'c' | 'd') contains 'b'", None),
        boolean(false)
    );

    // The swap really is a swap: the same pair answers the same both ways.
    assert_eq!(
        eval_over("((1 | 2 | 3) contains 1) = (1 in (1 | 2 | 3))", None),
        boolean(true)
    );

    // `contains` on a string focus is the *string* function, a different
    // operator entirely (`path.string.007`), so a one-character collection
    // test and a substring test must not share an implementation.
    assert_eq!(eval_over("'abc'.contains('b')", None), boolean(true));
    assert_eq!(eval_over("('abc') contains 'b'", None), boolean(false));
}

// ---------------------------------------------------------------------------
// path.combine.006 -- `in` with an empty left operand yields empty
// ---------------------------------------------------------------------------

// covers: path.combine.006
#[test]
fn in_with_an_empty_left_operand_yields_empty() {
    // `opIn` (FHIRPathEngine.java:2631) returns an empty list when either
    // operand is empty, so the answer is empty and specifically not `false`.
    assert_eq!(eval_over("{} in (1 | 2)", None), empty());
    assert_eq!(eval_patient("Patient.name.prefix in (1 | 2)"), empty());

    // Not false, and not true: three distinct answers, and only one is right.
    assert_ne!(eval_over("{} in (1 | 2)", None), boolean(false));
    assert_ne!(eval_over("{} in (1 | 2)", None), boolean(true));
    assert_eq!(eval_over("({} in (1 | 2)).empty()", None), boolean(true));

    // An empty right operand is empty too, so the rule is about either side.
    assert_eq!(eval_over("1 in {}", None), empty());
}
