//! Collection predicates: `path.collection.001` through `path.collection.015`.
//!
//! ```text
//! cargo test -p frevo-path --test path_collection
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Nine of the fifteen scenarios name
//! a suite group, so the test drives the real group through the committed
//! harness; the other six state an expression and its result, which is asserted
//! at the engine seam. Nothing here is hand-rolled where a case exists.
//!
//! The empty-collection identities are the point of the requirement, and each
//! one comes from a specific line of the pinned engine rather than from the
//! FHIRPath specification:
//!
//! | Function | Empty focus | Site |
//! |---|---|---|
//! | `empty()` | `true` | `funcEmpty`, FHIRPathEngine.java:6286 |
//! | `exists()` | `false` | `funcExists`, :5383 |
//! | `not()` | **empty** | `funcNot`, :6292 -- `asBool` answers `Null` and nothing is appended |
//! | `all(c)` | `true` | `funcAll`, :4696 -- the fold starts `true` and the loop does not run |
//! | `count()` | `0` | `funcCount`, :6095 |
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.
//!
//! Every test fails today because no engine is bound to the harness seam.
//! `path.collection.009` expects a *failure*, so it matches on
//! `EngineError::Diagnostic` and an engine that refuses to run cannot satisfy
//! it.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::{Stage, Suite};
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// Assert every case in `group` passed, and that the group still has the size
/// this test was written against.
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

/// The stage `expression/@invalid` says a case must fail at, if any.
fn expected_failure(group: &str, name: &str) -> Option<Stage> {
    let (suite, _) = pinned();
    suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"))
        .expected_failure
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

/// Parse, check and evaluate `expr` over `inp`; every stage must succeed.
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

/// `expr` must parse and check and then fail at **evaluation** with a real
/// diagnostic. `NotImplemented` panics, so the unbound engine cannot satisfy a
/// scenario that expects a failure.
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

// ---------------------------------------------------------------------------
// path.collection.001 -- `empty()` is true for an empty collection
// ---------------------------------------------------------------------------

// covers: path.collection.001
#[test]
fn empty_is_true_for_an_empty_collection() {
    assert_eq!(eval_patient("Patient.name.prefix.empty()"), boolean(true));

    // False for a non-empty one, so the answer tracks the collection and is
    // not a constant.
    assert_eq!(eval_patient("Patient.name.empty()"), boolean(false));

    // `funcEmpty` (FHIRPathEngine.java:6286) delegates to
    // `ElementUtil.isEmpty`, so a collection of one item that itself has no
    // value is still non-empty: emptiness is about the collection.
    assert_eq!(
        eval_patient("Patient.name.prefix.empty().not()"),
        boolean(false)
    );
    assert_eq!(eval_over("{}.empty()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.002 -- `exists()` is true for a non-empty collection
// ---------------------------------------------------------------------------

// covers: path.collection.002
#[test]
fn exists_is_true_for_a_non_empty_collection() {
    expect_group("testExists", 5);

    // `funcExists` (FHIRPathEngine.java:5383) is `empty()` inverted, which the
    // group grades both ways round and over a literal collection
    // (`(1 | 2).exists()`).
    assert_eq!(eval_patient("Patient.name.exists()"), boolean(true));
    assert_eq!(eval_patient("Patient.name.prefix.exists()"), boolean(false));
    assert_eq!(eval_over("(1 | 2).exists()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.003 -- `exists(criteria)` filters before testing
// ---------------------------------------------------------------------------

// covers: path.collection.003
#[test]
fn exists_with_a_criteria_filters_before_testing() {
    assert_eq!(
        eval_patient("Patient.name.exists(given = 'Jim')"),
        boolean(true)
    );

    // A criteria no item satisfies is false, so the criteria really is applied
    // rather than ignored -- which the group's `use = 'nickname'` case grades.
    assert_eq!(
        eval_patient("Patient.name.exists(given = 'nobody')"),
        boolean(false)
    );
    expect_case("testExists", "testExists2");
    expect_case("testExists", "testExists3");

    // `exists(c)` is `where(c).exists()`, so the two spellings must agree.
    assert_eq!(
        eval_patient(
            "Patient.name.exists(given = 'Jim') = Patient.name.where(given = 'Jim').exists()"
        ),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.collection.004 -- `not()` inverts a singleton boolean
// ---------------------------------------------------------------------------

// covers: path.collection.004
#[test]
fn not_inverts_a_singleton_boolean() {
    assert_eq!(eval_patient("Patient.name.exists().not()"), boolean(false));
    assert_eq!(
        eval_patient("Patient.name.prefix.exists().not()"),
        boolean(true)
    );

    // `funcNot` (FHIRPathEngine.java:6292) appends `v != Equality.True`, so a
    // false input becomes true and a true input false -- and double negation
    // is the identity on a singleton boolean.
    assert_eq!(eval_over("true.not()", None), boolean(false));
    assert_eq!(eval_over("false.not()", None), boolean(true));
    assert_eq!(eval_over("true.not().not()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.005 -- `not()` on an empty collection yields empty
// ---------------------------------------------------------------------------

// covers: path.collection.005
#[test]
fn not_on_an_empty_collection_yields_empty() {
    // `funcNot` (FHIRPathEngine.java:6292) appends **nothing** when `asBool`
    // answers `Equality.Null`, which is what an empty focus produces. So the
    // result is empty and not `true`: `not()` is not `empty()`.
    assert_eq!(eval_over("{}.not()", None), empty());
    assert_eq!(eval_patient("Patient.name.prefix.not()"), empty());

    // Which is observably different from `true`, and from `empty()`.
    assert_ne!(eval_over("{}.not()", None), boolean(true));
    assert_eq!(eval_over("{}.not().empty()", None), boolean(true));
    assert_eq!(eval_over("{}.empty()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.006 -- `all(criteria)` over every item
// ---------------------------------------------------------------------------

// covers: path.collection.006
#[test]
fn all_is_true_when_every_item_satisfies_the_criteria() {
    expect_group("testAll", 4);

    // The group pairs `all(c)` with `select(c).allTrue()`, so the two spellings
    // are graded against each other as well as against the answer.
    assert_eq!(
        eval_patient("Patient.name.all(given.exists())"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("Patient.name.all(period.exists())"),
        boolean(false)
    );
    assert_eq!(
        eval_patient("Patient.name.select(given.exists()).allTrue()"),
        boolean(true)
    );

    // One failing item is enough: `funcAll` (FHIRPathEngine.java:4696) breaks
    // out of the loop on the first non-true criteria.
    assert_eq!(
        eval_patient("Patient.name.all(given = 'Peter')"),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.collection.007 -- `all(criteria)` over an empty collection
// ---------------------------------------------------------------------------

// covers: path.collection.007
#[test]
fn all_is_true_for_an_empty_collection() {
    // `funcAll` (FHIRPathEngine.java:4698-4709) initialises `all = true` and
    // the per-item loop does not run, so an empty focus yields `true` whatever
    // the criteria says -- including a criteria that is false for every
    // possible item.
    assert_eq!(
        eval_patient("Patient.name.prefix.all($this = 'x')"),
        boolean(true)
    );
    assert_eq!(eval_over("{}.all(false)", None), boolean(true));
    assert_eq!(eval_over("{}.all($this = 'anything')", None), boolean(true));

    // `true`, and specifically not empty: vacuous truth is a value here.
    assert_ne!(eval_over("{}.all(false)", None), empty());
}

// ---------------------------------------------------------------------------
// path.collection.008 -- the boolean folds
// ---------------------------------------------------------------------------

// covers: path.collection.008
#[test]
fn the_boolean_folds_reduce_a_boolean_collection() {
    expect_group("testCollectionBoolean", 6);

    // The four folds, each over a collection where it answers differently, so
    // no two of them could be satisfied by one shared implementation:
    // `funcAllTrue` :5552, `funcAnyTrue` :5585, `funcAllFalse` :5484,
    // `funcAnyFalse` :5518.
    assert_eq!(eval_over("(true | true).allTrue()", None), boolean(true));
    assert_eq!(eval_over("(true | false).allTrue()", None), boolean(false));
    assert_eq!(eval_over("(true | false).anyTrue()", None), boolean(true));
    assert_eq!(eval_over("(false | false).anyTrue()", None), boolean(false));
    assert_eq!(eval_over("(false | false).allFalse()", None), boolean(true));
    assert_eq!(eval_over("(true | false).allFalse()", None), boolean(false));
    assert_eq!(eval_over("(true | false).anyFalse()", None), boolean(true));
    assert_eq!(eval_over("(true | true).anyFalse()", None), boolean(false));
}

// ---------------------------------------------------------------------------
// path.collection.009 -- a fold over a non-boolean item fails
// ---------------------------------------------------------------------------

// covers: path.collection.009
#[test]
fn a_boolean_fold_over_a_non_boolean_item_is_an_execution_error() {
    // `funcAllTrue`'s no-parameter arm (FHIRPathEngine.java:5570) tests
    // `canConvertToBoolean(item)` and raises `Unable to convert '...' to a
    // boolean` when it cannot. The suite case carries `invalid="execution"`,
    // so the failure must come from the evaluate stage and not from the check.
    assert_eq!(
        expected_failure("from-Zulip", "from-zulip-2"),
        Some(Stage::Evaluate),
        "the case is invalid=\"execution\", so the stage matters"
    );
    expect_case("from-Zulip", "from-zulip-2");

    let patient = load_input("patient-example.xml");
    let message = expect_evaluation_failure("(true | 'foo').allTrue()", Some(&patient));
    assert!(
        message.contains("foo") || message.to_lowercase().contains("boolean"),
        "the error must say what could not be converted: {message}"
    );

    // The sibling case in the same group is **not** a failure: a non-boolean on
    // the right of `and` is singleton-converted instead. Keeping both here is
    // what stops an implementation from failing the whole group.
    assert_eq!(expected_failure("from-Zulip", "from-zulip-1"), None);
    assert_eq!(eval_over("(true and 'foo').empty()", None), boolean(false));
}

// ---------------------------------------------------------------------------
// path.collection.010 -- `subsetOf` compares by equality
// ---------------------------------------------------------------------------

// covers: path.collection.010
#[test]
fn subset_of_compares_by_equality() {
    expect_group("testSubSetOf", 3);

    // `funcSubsetOf` (FHIRPathEngine.java:5360) answers true when every item of
    // the focus is `doEquals`-equal to some item of the argument, so the empty
    // focus is a subset of everything and a longer focus is not a subset of a
    // shorter one.
    assert_eq!(
        eval_patient("Patient.name.first().subsetOf($this.name)"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("Patient.name.subsetOf($this.name.first())"),
        boolean(false)
    );
    assert_eq!(eval_over("{}.subsetOf(1 | 2)", None), boolean(true));

    // One of the group's three cases runs over the JSON
    // `explanationofbenefit-example.json` input, so the comparison is graded
    // over both input forms (`path.oracle.004`, `path.utility.021`).
    let (suite, _) = pinned();
    let case = suite
        .case("testSubSetOf", "testSubSetOf3")
        .expect("the case resolves");
    assert_eq!(
        case.inputfile.as_deref(),
        Some("explanationofbenefit-example.json")
    );
}

// ---------------------------------------------------------------------------
// path.collection.011 -- `supersetOf` compares by equality
// ---------------------------------------------------------------------------

// covers: path.collection.011
#[test]
fn superset_of_compares_by_equality() {
    expect_group("testSuperSetOf", 2);

    // `funcSupersetOf` (FHIRPathEngine.java:5337) is the mirror of `subsetOf`,
    // and the group grades exactly that pairing: the same two collections, the
    // other way round.
    assert_eq!(
        eval_patient("Patient.name.supersetOf($this.name.first())"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("Patient.name.first().supersetOf($this.name)"),
        boolean(false)
    );
    assert_eq!(eval_over("(1 | 2).supersetOf({})", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.012 -- `isDistinct` detects duplicates
// ---------------------------------------------------------------------------

// covers: path.collection.012
#[test]
fn is_distinct_detects_duplicates() {
    // `patient-example.xml` repeats `Peter` and `James` across its three names,
    // so the five given names are not distinct.
    assert_eq!(
        eval_patient("Patient.name.given"),
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
                value: "Jim".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Peter".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "James".into()
            },
        ]
    );
    assert_eq!(
        eval_patient("Patient.name.given.isDistinct()"),
        boolean(false)
    );

    // `funcIsDistinct` (FHIRPathEngine.java:5313) short-circuits to true at
    // size <= 1, so a singleton and an empty collection are both distinct.
    assert_eq!(eval_over("(1 | 2 | 3).isDistinct()", None), boolean(true));
    assert_eq!(
        eval_patient("Patient.name.first().given.isDistinct()",),
        boolean(true)
    );
    assert_eq!(eval_over("{}.isDistinct()", None), boolean(true));
    expect_case("testDistinct", "testDistinct1");
}

// ---------------------------------------------------------------------------
// path.collection.013 -- `distinct` removes duplicates, keeps first order
// ---------------------------------------------------------------------------

// covers: path.collection.013
#[test]
fn distinct_removes_duplicates_and_keeps_first_occurrence_order() {
    expect_group("testDistinct", 6);

    // First-occurrence order, not sorted order: `Jim` stays third because that
    // is where it first appears, and the two later duplicates are dropped.
    assert_eq!(
        eval_patient("Patient.name.given.distinct()"),
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
                value: "Jim".into()
            },
        ]
    );
    assert_eq!(
        eval_patient("Patient.name.given.distinct().count()"),
        integer("3")
    );
    assert_eq!(
        eval_patient("Patient.name.given.distinct().isDistinct()"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.collection.014 -- `count` returns the number of items
// ---------------------------------------------------------------------------

// covers: path.collection.014
#[test]
fn count_returns_the_number_of_items() {
    expect_group("testCount", 4);

    // `funcCount` (FHIRPathEngine.java:6095) returns `focus.size()` as an
    // integer, so it counts the collection and not the characters or the
    // repetitions of some child.
    assert_eq!(eval_patient("Patient.name.count()"), integer("3"));
    assert_eq!(eval_patient("Patient.name.given.count()"), integer("5"));
    assert_eq!(eval_patient("Patient.name.first().count()"), integer("1"));

    // The result is an `integer`, which the group's `= 3` cases depend on.
    assert_eq!(eval_patient("Patient.name.count() = 3"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.collection.015 -- `count` of an empty collection is zero
// ---------------------------------------------------------------------------

// covers: path.collection.015
#[test]
fn count_of_an_empty_collection_is_zero() {
    // Zero, and specifically not empty: `funcCount` always appends exactly one
    // integer.
    assert_eq!(eval_patient("Patient.name.prefix.count()"), integer("0"));
    assert_eq!(eval_over("{}.count()", None), integer("0"));
    assert_ne!(eval_over("{}.count()", None), empty());
    assert_eq!(eval_over("{}.count().exists()", None), boolean(true));
}
