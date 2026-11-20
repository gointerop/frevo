//! Quantities and UCUM: `path.quantity.001` through `path.quantity.019`.
//!
//! ```text
//! cargo test -p frevo-path --test path_quantity
//! ```
//!
//! # Where HAPI fails the suite, these tests require HAPI's answer
//!
//! The vendored `fhir-test-cases` file is newer than the pinned R4 engine
//! (`oracles/PINS.md` against `reference/PINS.md`), and the quantity comparison
//! code in `FHIRPathEngine` at `73c432b5f43ddbce979978a760ec4aa81e929094` has
//! two defects the newer cases were written to catch. The approved spec
//! restates `path.quantity.009`, `.011`, `.012` and `.013` as HAPI behaves
//! (`tasks/blockers.md`, 2026-11-20, "four `path.quantity` scenarios still
//! state the suite's answer"): the spec-prose difference is `DEV-0017`, the
//! wrong answers are `DEBT-0019`.
//!
//! 1. **`qtyEqual` and `qtyEquivalent` guard their unit comparison with
//!    `else if (!left.hasUnit() || right.hasUnit())`** (FHIRPathEngine.java
//!    :2282, :2355). It returns `null` when the **left** has no unit or the
//!    units differ, and otherwise falls through to compare the values, so a
//!    left quantity with a unit and a right one without compares by value
//!    alone. `opEquals` turns the `null` into an empty result (:2114-2127);
//!    `doEquivalent` unboxes it into a `boolean` (:2204-2206), a
//!    `NullPointerException`.
//!
//! 2. **`opLessThan` identifies the unit by the `code` children** (:2438-2440)
//!    where `<=`, `>` and `>=` use `unit`. With the codes differing it compares
//!    canonical decimals, and `qtyToCanonicalDecimal` returns `null` for a
//!    quantity with no UCUM system (:2301-2303); the recursive call's
//!    `isPrimitive()` on it (:2414) is a `NullPointerException`.
//!
//! HAPI's own run (`oracles/fhirpath/hapi-8.12.0-results.json`) grades:
//!
//! | Case | Suite expects | HAPI 8.12.0 | Line |
//! |---|---|---|---:|
//! | `testQuantityUnitComparison/testQuantityNoUnitEqual` | `true` | failed: empty | 632 |
//! | `testQuantityUnitComparison/testQuantityNoUnitEquivalent` | `true` | failed: raises | 633 |
//! | `testQuantityUnitComparison/testQuantityUnitVsNoUnitEqual` | empty | failed: `true` | 636 |
//! | `testQuantityUnitComparison/testQuantityNoUnitVsUnitEqual` | empty | passed | 637 |
//! | `testQuantityUnitComparison/testQuantityUnitVsNoUnitEquivalent` | `false` | failed: `true` | 638 |
//! | `testQuantityUnitComparison/testQuantityNoUnitVsUnitEquivalent` | `false` | failed: raises | 639 |
//! | `testLessThan/testLessThan33` | `true` | failed: raises | 631 |
//! | `testLessOrEqual/testLessOrEqual30` | `true` | passed | 669 |
//! | `testGreaterThan/testGreaterThan30` | `true` | passed | 728 |
//!
//! A case HAPI fails must be graded `failed` here, with HAPI's value (parity
//! under `path.oracle.012`); a Frevo pass would be `ahead-of-hapi`, not a pass
//! of the scenario. Where HAPI raises a `NullPointerException`, Frevo returns an
//! execution error -- never a panic (`path.errors.003`) -- with no offset
//! (`path.errors.002`) and the JVM's message as HAPI's run records it, exactly.
//!
//! `path.quantity.019` is the Frevo side of `DEV-0029`: HAPI's server-facing
//! worker raises `HAPI-0274: ` wherever the engine consults UCUM, and Frevo
//! answers as the suite's worker does in every configuration. Its UCUM-dependent
//! assertions stay red until the unit algebra exists (task 43;
//! `tasks/blockers.md`, 2026-11-20, "the UCUM unit algebra cannot be ported").

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseReport, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::{QuantityValue, ResultItem, UCUM_SYSTEM};

/// Where the repository keeps the UCUM data, as `design.md` section 5 records
/// it. It is fetched and not vendored, so an absent file is a run failure with
/// a message naming the fetch command rather than a silent fallback.
const UCUM_ESSENCE_PATH: &str = "oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml";

/// The `version` attribute of the pinned file's root element.
const UCUM_ESSENCE_VERSION: &str = "2.0.1";

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

fn expect_case(group: &str, name: &str) -> &'static CaseReport {
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
    r
}

/// The single expected `<output>` the pinned file records for a case, as
/// `(type, body)`. An empty body means "no value expected", which for these
/// cases means an empty result.
fn sole_output(group: &str, name: &str) -> (String, String) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert_eq!(case.outputs.len(), 1, "{group}/{name} has one output");
    (
        case.outputs[0].type_name.clone(),
        case.outputs[0].body.clone(),
    )
}

/// A case with no `<output>` at all, which is how the suite spells "empty".
fn expects_no_output(group: &str, name: &str) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert!(
        case.outputs.is_empty(),
        "{group}/{name} must expect an empty result, but it expects {:?}",
        case.outputs
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

fn eval(expr: &str) -> Vec<ResultItem> {
    eval_over(expr, None)
}

/// `expr` must parse and check, then fail at evaluation with a diagnostic.
fn expect_evaluation_error(expr: &str) -> String {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before it can fail at evaluation: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

/// The JVM's message for `doEquivalent` unboxing the `null` `qtyEquivalent`
/// returned, as HAPI's run records it (`hapi-8.12.0-results.json:633`, `:639`).
const NPE_QTY_EQUIVALENT: &str = "Cannot invoke \"java.lang.Boolean.booleanValue()\" because \
     the return value of \"org.hl7.fhir.r4.fhirpath.FHIRPathEngine.qtyEquivalent(\
     org.hl7.fhir.r4.model.Quantity, org.hl7.fhir.r4.model.Quantity)\" is null";

/// The JVM's message for the recursive `opLessThan` calling `isPrimitive()` on
/// the `null` canonical decimal, as HAPI's run records it
/// (`hapi-8.12.0-results.json:631`).
const NPE_LIST_GET_IS_PRIMITIVE: &str = "Cannot invoke \
     \"org.hl7.fhir.r4.model.Base.isPrimitive()\" because the return value of \
     \"java.util.List.get(int)\" is null";

/// A case HAPI 8.12.0 fails, which Frevo must grade `failed` too.
fn expect_hapi_failed(group: &str, name: &str) -> &'static CaseReport {
    let (suite, report) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    let r = report
        .get(&case.address.to_string())
        .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
    assert_eq!(
        r.status,
        CaseStatus::Failed,
        "{}: {:?} must be graded failed, as HAPI 8.12.0 grades it, but is {} -- {}",
        r.address,
        r.expression,
        r.status.as_str(),
        r.reason
    );
    r
}

/// A case HAPI 8.12.0 fails by raising a `NullPointerException`: graded
/// `failed`, at the evaluate stage, and for HAPI's reason.
fn expect_hapi_raised(group: &str, name: &str, jvm_message: &str) {
    let r = expect_hapi_failed(group, name);
    assert!(
        r.reason.contains(jvm_message),
        "{}: {:?} must fail as HAPI 8.12.0 fails it, raising {jvm_message:?}; it failed with: {}",
        r.address,
        r.expression,
        r.reason
    );
}

/// `expr` over `inp` must parse and check, then fail at evaluation with an
/// execution error carrying no offset (`path.errors.002`, `path.errors.003`).
/// Returns the message.
fn expect_execution_error_over(expr: &str, inp: Option<&SuiteInput>) -> String {
    let mut engine = fresh_engine();
    let resource_type = inp.and_then(|i| i.resource_type());
    let context = CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    };
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before it can fail at evaluation: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    match engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!(
            "{expr:?} must fail at evaluation, as HAPI raises a NullPointerException, but it \
             returned {items:?}"
        ),
        Err(EngineError::Diagnostic { message, offset }) => {
            assert_eq!(
                offset, None,
                "{expr:?}: a NullPointerException carries no position, so the error has no \
                 offset (path.errors.002); message {message:?}"
            );
            message
        }
        Err(e) => panic!("{expr:?} must fail with an execution error, not: {e}"),
    }
}

fn boolean(value: bool) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: "boolean".to_string(),
        value: if value { "true" } else { "false" }.to_string(),
    }]
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// path.quantity.001 -- commensurable units compare equal after conversion
// ---------------------------------------------------------------------------

// covers: path.quantity.001
#[test]
fn quantities_in_commensurable_ucum_units_compare_equal_after_conversion() {
    let (ty, body) = sole_output("testQuantity", "testQuantity1");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testQuantity", "testQuantity1");

    // `qtyEqual` converts both operands with `qtyToCanonicalPair`, requires the
    // canonical codes to match, and only then compares the two decimals
    // (FHIRPathEngine.java:2265-2276). The conversion is what makes grams and
    // milligrams comparable at all.
    assert_eq!(eval("4.0000 'g' = 4000.0 'mg'"), boolean(true));
    assert_eq!(eval("1 'm' = 100 'cm'"), boolean(true));

    // And a magnitude that does not match is false rather than empty, so the
    // conversion is being used and not skipped.
    assert_eq!(eval("1 'm' = 101 'cm'"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.quantity.002 -- equivalence rounds to the lower precision
// ---------------------------------------------------------------------------

// covers: path.quantity.002
#[test]
fn equivalence_of_quantities_rounds_to_the_lower_precision() {
    for name in ["testQuantity2", "testQuantity4"] {
        let (ty, body) = sole_output("testQuantity", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testQuantity", name);
    }

    // `qtyEquivalent` ends in `doEquivalent`, whose numeric branch is
    // `Utilities.equivalentNumber` (FHIRPathEngine.java:2213-2215): it rounds
    // both sides to the shorter of the two precisions before comparing. So
    // 4 g is equivalent to 4040 mg -- 4.040 g rounded to one significant digit.
    assert_eq!(eval("4 'g' ~ 4000 'mg'"), boolean(true));
    assert_eq!(eval("4 'g' ~ 4040 'mg'"), boolean(true));

    // Far enough apart and it is still not equivalent, so the rounding is to
    // the stated precision and not unbounded.
    assert_eq!(eval("4 'g' ~ 5000 'mg'"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.quantity.003 -- equality does not round
// ---------------------------------------------------------------------------

// covers: path.quantity.003
#[test]
fn equality_of_quantities_does_not_round() {
    let (ty, body) = sole_output("testQuantity", "testQuantity3");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testQuantity", "testQuantity3");

    // The same pair that `~` calls equivalent, `=` calls unequal: `doEquals`
    // sends the two canonical decimals to `decEqual`, which only strips
    // trailing zeros (FHIRPathEngine.java:2180-2184).
    assert_eq!(eval("4 'g' != 4040 'mg'"), boolean(true));
    assert_eq!(eval("4 'g' = 4040 'mg'"), boolean(false));
    assert_eq!(
        eval("4 'g' ~ 4040 'mg'"),
        boolean(true),
        "the contrast with equivalence is the point of the pair"
    );
}

// ---------------------------------------------------------------------------
// path.quantity.004 -- calendar durations against UCUM units
// ---------------------------------------------------------------------------

// covers: path.quantity.004
#[test]
fn calendar_durations_and_ucum_units_are_commensurable_where_the_engine_says_so() {
    for name in ["testQuantity5", "testQuantity6"] {
        let (ty, body) = sole_output("testQuantity", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testQuantity", name);
    }

    // `7 days` and `1 week` are both given UCUM codes by the literal parser --
    // `d` and `wk` (FHIRPathEngine.java:1117-1120) -- so both canonicalise and
    // compare. Which calendar words get a code is not uniform:
    // `path.literal.013` records that `year` and `month` get none.
    assert_eq!(eval("7 days = 1 week"), boolean(true));
    assert_eq!(eval("7 days = 1 'wk'"), boolean(true));
    assert_eq!(eval("24 hours = 1 day"), boolean(true));
    assert_eq!(eval("1000 milliseconds = 1 second"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.005 -- commensurable units compare by magnitude
// ---------------------------------------------------------------------------

// covers: path.quantity.005
#[test]
fn quantities_in_commensurable_units_compare_by_magnitude() {
    for name in ["testQuantity7", "testQuantity8"] {
        let (ty, body) = sole_output("testQuantity", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testQuantity", name);
    }

    assert_eq!(eval("6 days < 1 week"), boolean(true));
    assert_eq!(eval("8 days > 1 week"), boolean(true));

    // Both directions, so the ordering is not accidental.
    assert_eq!(eval("1 week < 6 days"), boolean(false));
    assert_eq!(eval("1 week > 8 days"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.quantity.006 -- multiplication multiplies the units
// ---------------------------------------------------------------------------

// covers: path.quantity.006
#[test]
fn multiplying_quantities_multiplies_their_units() {
    let (ty, body) = sole_output("testQuantity", "testQuantity9");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testQuantity", "testQuantity9");

    // `opTimes`'s quantity arm delegates to the UCUM service's `multiply`
    // (FHIRPathEngine.java:2792-2801), so the result's unit is the product of
    // the operands' units and not either one of them.
    assert_eq!(eval("2.0 'cm' * 2.0 'm' = 0.040 'm2'"), boolean(true));
    assert_eq!(
        eval("(2.0 'cm' * 2.0 'm') is System.Quantity"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.quantity.007 -- division divides the units
// ---------------------------------------------------------------------------

// covers: path.quantity.007
#[test]
fn dividing_quantities_divides_their_units() {
    let (ty, body) = sole_output("testQuantity", "testQuantity10");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testQuantity", "testQuantity10");

    // `opDivideBy`'s quantity arm delegates to the UCUM service's `divideBy`
    // (FHIRPathEngine.java:3013-3021).
    assert_eq!(eval("4.0 'g' / 2.0 'm' = 2 'g/m'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.008 -- a unit divided by itself
// ---------------------------------------------------------------------------

// covers: path.quantity.008
#[test]
fn dividing_a_unit_by_itself_yields_the_dimensionless_unit() {
    let (ty, body) = sole_output("testQuantity", "testQuantity11");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testQuantity", "testQuantity11");

    // The dimensionless unit is the UCUM code `1`, which is also what
    // `parseQuantityString` gives a bare decimal, so `1 '1'` and `1.toQuantity()`
    // name the same thing.
    assert_eq!(eval("1.0 'm' / 1.0 'm' = 1 '1'"), boolean(true));
    assert_eq!(eval("1.toQuantity() = 1 '1'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.009 -- two unitless quantities
// ---------------------------------------------------------------------------

// covers: path.quantity.009
#[test]
fn two_unitless_quantities_give_empty_equality_and_failing_equivalence() {
    // HAPI fails both suite cases (hapi-8.12.0-results.json:632, :633): with
    // no unit on the left, the unit guard returns null (module note, defect 1).
    for name in ["testQuantityNoUnitEqual", "testQuantityNoUnitEquivalent"] {
        let (ty, body) = sole_output("testQuantityUnitComparison", name);
        assert_eq!(
            (ty.as_str(), body.as_str()),
            ("boolean", "true"),
            "the suite expects true for {name}"
        );
    }
    let equal = expect_hapi_failed("testQuantityUnitComparison", "testQuantityNoUnitEqual");
    assert!(
        !equal.reason.is_empty(),
        "a failed case carries its reason: {equal:?}"
    );
    expect_hapi_raised(
        "testQuantityUnitComparison",
        "testQuantityNoUnitEquivalent",
        NPE_QTY_EQUIVALENT,
    );

    // The input really does carry two unitless quantities, so the scenario is
    // about the absence of a unit and not about the values.
    let quantities = load_input("parameters-example-quantities.xml");
    for index in [0usize, 1] {
        assert_eq!(
            eval_over(
                &format!("Parameters.parameter[{index}].value"),
                Some(&quantities)
            ),
            vec![ResultItem::Quantity(QuantityValue {
                value: "1".to_string(),
                comparator: None,
                unit: None,
                system: None,
                code: None,
                has_element_id: false,
                has_extensions: false,
            })],
            "parameter[{index}] must be a unitless Quantity of 1"
        );
    }

    // HAPI's values: equality is empty (`opEquals` turns the null into an
    // empty result), equivalence raises.
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value = Parameters.parameter[1].value",
            Some(&quantities)
        ),
        empty(),
        "HAPI finds no item (hapi-8.12.0-results.json:632)"
    );
    let message = expect_execution_error_over(
        "Parameters.parameter[0].value ~ Parameters.parameter[1].value",
        Some(&quantities),
    );
    assert_eq!(message, NPE_QTY_EQUIVALENT, "HAPI's JVM message, exactly");
}

// ---------------------------------------------------------------------------
// path.quantity.010 -- same unit text, no code
// ---------------------------------------------------------------------------

// covers: path.quantity.010
#[test]
fn two_quantities_with_the_same_unit_text_and_no_code_compare_equal() {
    for name in [
        "testQuantityUnitTextEqual",
        "testQuantityUnitTextEquivalent",
    ] {
        let (ty, body) = sole_output("testQuantityUnitComparison", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testQuantityUnitComparison", name);
    }

    // Neither operand canonicalises, because neither has a UCUM system, so the
    // comparison falls through to the unit-text branch and the two `tablets`
    // quantities compare on their values (FHIRPathEngine.java:2282-2286).
    let quantities = load_input("parameters-example-quantities.xml");
    assert_eq!(
        eval_over("Parameters.parameter[2].value.unit", Some(&quantities)),
        vec![ResultItem::Primitive {
            fhir_type: "string".to_string(),
            value: "tablets".to_string()
        }],
        "the unit is carried as text with no code, which is what the scenario turns on"
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value = Parameters.parameter[3].value",
            Some(&quantities)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value ~ Parameters.parameter[3].value",
            Some(&quantities)
        ),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.quantity.011 -- a unit against no unit, under `=`
// ---------------------------------------------------------------------------

// covers: path.quantity.011
#[test]
fn equality_of_a_quantity_with_a_unit_and_one_without_depends_on_which_side_has_it() {
    // With the unit on the left the guard is skipped and `1 = 1` compares, so
    // HAPI fails testQuantityUnitVsNoUnitEqual with one item
    // (hapi-8.12.0-results.json:636); with it on the right the guard returns
    // null and the result is empty, which the suite expects, so HAPI passes
    // testQuantityNoUnitVsUnitEqual (:637).
    for name in [
        "testQuantityUnitVsNoUnitEqual",
        "testQuantityNoUnitVsUnitEqual",
    ] {
        expects_no_output("testQuantityUnitComparison", name);
    }
    expect_hapi_failed(
        "testQuantityUnitComparison",
        "testQuantityUnitVsNoUnitEqual",
    );
    expect_case(
        "testQuantityUnitComparison",
        "testQuantityNoUnitVsUnitEqual",
    );

    let quantities = load_input("parameters-example-quantities.xml");
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value = Parameters.parameter[0].value",
            Some(&quantities)
        ),
        boolean(true),
        "a `tablets` quantity on the left against a unitless one compares by value \
         (HAPI's one item, hapi-8.12.0-results.json:636)"
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value = Parameters.parameter[2].value",
            Some(&quantities)
        ),
        empty(),
        "and the reverse is empty: the comparison is not symmetric"
    );
}

// ---------------------------------------------------------------------------
// path.quantity.012 -- a unit against no unit, under `~`
// ---------------------------------------------------------------------------

// covers: path.quantity.012
#[test]
fn equivalence_of_a_quantity_with_a_unit_and_one_without_is_true_one_way_and_fails_the_other() {
    // The same guard as path.quantity.011, with `doEquivalent` unboxing the
    // null: HAPI answers `true` for the first case
    // (hapi-8.12.0-results.json:638) and raises on the second (:639). Both
    // are failed, since the suite expects `false` for both.
    for name in [
        "testQuantityUnitVsNoUnitEquivalent",
        "testQuantityNoUnitVsUnitEquivalent",
    ] {
        let (ty, body) = sole_output("testQuantityUnitComparison", name);
        assert_eq!(
            (ty.as_str(), body.as_str()),
            ("boolean", "false"),
            "the suite expects false for {name}"
        );
    }
    expect_hapi_failed(
        "testQuantityUnitComparison",
        "testQuantityUnitVsNoUnitEquivalent",
    );
    expect_hapi_raised(
        "testQuantityUnitComparison",
        "testQuantityNoUnitVsUnitEquivalent",
        NPE_QTY_EQUIVALENT,
    );

    let quantities = load_input("parameters-example-quantities.xml");
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value ~ Parameters.parameter[0].value",
            Some(&quantities)
        ),
        boolean(true),
        "HAPI answers true (hapi-8.12.0-results.json:638)"
    );
    let message = expect_execution_error_over(
        "Parameters.parameter[0].value ~ Parameters.parameter[2].value",
        Some(&quantities),
    );
    assert_eq!(message, NPE_QTY_EQUIVALENT, "HAPI's JVM message, exactly");
}

// ---------------------------------------------------------------------------
// path.quantity.013 -- unit text without a code, through the dedicated input
// ---------------------------------------------------------------------------

// covers: path.quantity.013
#[test]
fn unit_text_without_a_code_passes_two_of_the_three_cases_and_raises_on_less_than() {
    // The input is a deliberate pair: parameter[0] has the unit `mg/dL` as text
    // only, parameter[1] has the same text plus the UCUM system and code. The
    // three cases that use it therefore compare a quantity that canonicalises
    // against one that does not.
    let unit_text = load_input("parameters-quantity-unit-text.xml");
    assert_eq!(
        eval_over("Parameters.parameter[0].value", Some(&unit_text)),
        vec![ResultItem::Quantity(QuantityValue {
            value: "1".to_string(),
            comparator: None,
            unit: Some("mg/dL".to_string()),
            system: None,
            code: None,
            has_element_id: false,
            has_extensions: false,
        })]
    );
    assert_eq!(
        eval_over("Parameters.parameter[1].value", Some(&unit_text)),
        vec![ResultItem::Quantity(QuantityValue {
            value: "2".to_string(),
            comparator: None,
            unit: Some("mg/dL".to_string()),
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("mg/dL".to_string()),
            has_element_id: false,
            has_extensions: false,
        })]
    );

    // All three cases expect `true`. `<=` and `>` compare the `unit` children,
    // which are equal, and then the values, so HAPI passes them
    // (hapi-8.12.0-results.json:669, :728). `<` compares the `code` children,
    // which differ, and canonicalising parameter[0] yields null (module note,
    // defect 2), so HAPI raises (:631).
    for (group, name) in [
        ("testLessThan", "testLessThan33"),
        ("testLessOrEqual", "testLessOrEqual30"),
        ("testGreaterThan", "testGreaterThan30"),
    ] {
        let (ty, body) = sole_output(group, name);
        assert_eq!(
            (ty.as_str(), body.as_str()),
            ("boolean", "true"),
            "{group}/{name}"
        );
    }
    expect_case("testLessOrEqual", "testLessOrEqual30");
    expect_case("testGreaterThan", "testGreaterThan30");
    expect_hapi_raised("testLessThan", "testLessThan33", NPE_LIST_GET_IS_PRIMITIVE);

    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value <= Parameters.parameter[1].value",
            Some(&unit_text)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[1].value > Parameters.parameter[0].value",
            Some(&unit_text)
        ),
        boolean(true)
    );
    let message = expect_execution_error_over(
        "Parameters.parameter[0].value < Parameters.parameter[1].value",
        Some(&unit_text),
    );
    assert_eq!(
        message, NPE_LIST_GET_IS_PRIMITIVE,
        "HAPI's JVM message, exactly"
    );
}

// ---------------------------------------------------------------------------
// path.quantity.014 -- `comparable()`
// ---------------------------------------------------------------------------

// covers: path.quantity.014
#[test]
fn comparable_reports_whether_two_quantities_are_commensurable() {
    for (name, expected) in [
        ("Comparable1", "true"),
        ("Comparable2", "false"),
        ("Comparable3", "false"),
    ] {
        let (_, body) = sole_output("Comparable", name);
        assert_eq!(body.as_str(), expected, "Comparable/{name}");
        expect_case("Comparable", name);
    }

    // `comparable()` answers about dimensions, not magnitudes: centimetres and
    // inches are commensurable, centimetres and seconds are not -- and the
    // answer is `false` rather than empty or an error, which is what makes it
    // usable as a guard before a comparison.
    assert_eq!(eval("1 'cm'.comparable(1 '[in_i]')"), boolean(true));
    assert_eq!(eval("1 'cm'.comparable(1 '[s]')"), boolean(false));
    assert_eq!(eval("1 'cm'.comparable(1 's')"), boolean(false));
    assert_eq!(eval("1 'cm'.comparable(1 'm')"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.015 -- incommensurable units (DEV-0005)
// ---------------------------------------------------------------------------

/// Which UCUM data the engine has.
///
/// The shared harness's `EngineConfig` does not carry this, because the suite
/// is always run with the pinned file. Upstream models the absence as
/// `worker.getUcumService() == null`, and every quantity comparison takes a
/// different branch for it (FHIRPathEngine.java:2443-2444, :2486-2487,
/// :2518-2519, :2578-2579), so "no UCUM" is observable behaviour and not a
/// degradation. `path.quantity.015` grades both states, so it needs a seam that
/// can express both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UcumData {
    /// The pinned `ucum-essence.xml` is loaded. What the server runs with.
    Pinned,
    /// No UCUM service at all.
    Absent,
}

/// The UCUM-dependent surface.
pub trait UcumOracle {
    /// The `version` attribute of the UCUM data in force, reported beside the
    /// suite result (`path.quantity.016`).
    fn ucum_version(&mut self) -> Result<String, EngineError>;

    /// Load UCUM data from a file. An absent or unparseable file is an error,
    /// never a silent fall back to [`UcumData::Absent`].
    fn load_ucum(&mut self, path: &Path) -> Result<String, EngineError>;

    fn evaluate_with_ucum(
        &mut self,
        expression: &str,
        ucum: UcumData,
    ) -> Result<Vec<ResultItem>, EngineError>;
}

fn ucum_oracle_under_test() -> Box<dyn UcumOracle> {
    Box::new(BoundUcum)
}

/// Bound over `frevo_path::UcumEssence` for loading and reporting the data
/// (task 43's loader). Evaluation with a chosen UCUM state stays
/// `NotImplemented` until the unit algebra (task 43) and quantity semantics
/// (task 44) exist: the crate has no surface that evaluates with or without a
/// UCUM service (`tasks/blockers.md`, 2026-11-20, "task 43: the UCUM unit
/// algebra cannot be ported").
struct BoundUcum;

/// A load failure is a graded diagnostic carrying the crate's own text, which
/// names the file.
fn ucum_load(path: &Path) -> Result<String, EngineError> {
    frevo_path::UcumEssence::load(path)
        .map(|e| e.version().to_owned())
        .map_err(|e| EngineError::Diagnostic {
            message: e.to_string(),
            offset: None,
        })
}

impl UcumOracle for BoundUcum {
    fn ucum_version(&mut self) -> Result<String, EngineError> {
        // The data in force is the pinned file, which is what the server and
        // the suite run with.
        ucum_load(&ucum_essence_path())
    }

    fn load_ucum(&mut self, path: &Path) -> Result<String, EngineError> {
        ucum_load(path)
    }

    fn evaluate_with_ucum(
        &mut self,
        _expression: &str,
        _ucum: UcumData,
    ) -> Result<Vec<ResultItem>, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

fn ucum_essence_path() -> PathBuf {
    input::repo_path(UCUM_ESSENCE_PATH)
}

// covers: path.quantity.015
#[test]
fn comparing_incommensurable_units_returns_a_boolean_and_not_an_error() {
    // DEV-0005, resolved as HAPI's behaviour. `qtyToCanonicalDecimal` returns
    // the canonical *value* and throws the canonical *unit* away
    // (FHIRPathEngine.java:2301-2313), and the comparison that follows is
    // between two bare decimals with no dimension check. So `1 'cm'` becomes
    // 0.01 metres, `1 's'` stays 1 second, and `0.01 < 1` is true.
    let mut oracle = ucum_oracle_under_test();

    let with_data = oracle
        .evaluate_with_ucum("1 'cm' < 1 's'", UcumData::Pinned)
        .unwrap_or_else(|e| panic!("with UCUM data loaded the comparison must answer: {e}"));
    assert_eq!(
        with_data,
        boolean(true),
        "0.01 metres compared against 1 second is true, because only the \
         magnitudes are compared"
    );

    // The same at the shared seam, which is configured with the pinned data.
    assert_eq!(eval("1 'cm' < 1 's'"), boolean(true));

    // With no UCUM service the comparison never reaches the canonicalisation:
    // each operator returns `makeBoolean(false)` outright when the unit
    // children differ (FHIRPathEngine.java:2443-2444). That is a different
    // answer, not the same answer by another route, which is why both states
    // are graded.
    let without_data = oracle
        .evaluate_with_ucum("1 'cm' < 1 's'", UcumData::Absent)
        .unwrap_or_else(|e| panic!("with no UCUM data the comparison must still answer: {e}"));
    assert_eq!(
        without_data,
        boolean(false),
        "with no UCUM service two quantities whose unit codes differ compare false"
    );

    // And where the unit codes agree the two states agree, because the
    // canonicalisation is not reached in either.
    for ucum in [UcumData::Pinned, UcumData::Absent] {
        let same_unit = oracle
            .evaluate_with_ucum("1 'cm' < 2 'cm'", ucum)
            .unwrap_or_else(|e| panic!("{ucum:?}: {e}"));
        assert_eq!(same_unit, boolean(true), "{ucum:?}");
    }
}

// ---------------------------------------------------------------------------
// path.quantity.016 -- the UCUM data is the pinned file
// ---------------------------------------------------------------------------

// covers: path.quantity.016
#[test]
fn ucum_data_comes_from_the_pinned_essence_file_and_its_absence_fails_the_run() {
    // The file the repository pins, which `design.md` section 5 records as
    // fetched rather than vendored.
    let path = ucum_essence_path();
    assert!(
        path.is_file(),
        "{UCUM_ESSENCE_PATH} must be present. It is fetched, not vendored: run \
         `bash oracles/fetch.sh`. Treating it as absent would silently change \
         every quantity comparison, which is what this scenario exists to stop."
    );
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{UCUM_ESSENCE_PATH} must be readable: {e}"));
    assert!(
        text.contains(&format!("version=\"{UCUM_ESSENCE_VERSION}\"")),
        "the pinned UCUM data must still be version {UCUM_ESSENCE_VERSION}; a \
         different version means it was re-fetched without this test being updated"
    );

    let mut oracle = ucum_oracle_under_test();

    // The version is reported, so a run can state which UCUM data produced its
    // numbers.
    let version = oracle
        .ucum_version()
        .unwrap_or_else(|e| panic!("the engine must report the UCUM version it loaded: {e}"));
    assert_eq!(version, UCUM_ESSENCE_VERSION);

    // Loading the pinned file succeeds and reports the same version.
    let loaded = oracle
        .load_ucum(&path)
        .unwrap_or_else(|e| panic!("the pinned UCUM file must load: {e}"));
    assert_eq!(loaded, UCUM_ESSENCE_VERSION);

    // An absent file fails rather than degrading to "units are incomparable".
    let missing = input::repo_path("oracles/fhir-test-cases/fetched/ucum/does-not-exist.xml");
    match oracle.load_ucum(&missing) {
        Err(EngineError::Diagnostic { message, .. }) => {
            assert!(
                !message.is_empty(),
                "the failure must name what could not be loaded"
            );
        }
        Ok(v) => panic!(
            "an absent UCUM file must fail the run, not report a version ({v}). \
             Silently treating units as incomparable would turn every quantity \
             comparison into the no-UCUM branch without saying so."
        ),
        Err(e) => panic!("the engine gave no graded answer: {e}"),
    }

    // So does an unparseable one. The suite file itself is valid XML but is not
    // UCUM data, which separates "cannot read" from "is not UCUM".
    let not_ucum = input::repo_path(oracle::suite::SUITE_PATH);
    match oracle.load_ucum(&not_ucum) {
        Err(EngineError::Diagnostic { .. }) => {}
        Ok(v) => panic!("a file that is not UCUM data must not load as version {v}"),
        Err(e) => panic!("the engine gave no graded answer: {e}"),
    }
}

// ---------------------------------------------------------------------------
// path.quantity.017 -- an unknown unit code
// ---------------------------------------------------------------------------

// covers: path.quantity.017
#[test]
fn a_quantity_with_an_unknown_unit_code_does_not_crash() {
    // What the pinned engine does, recorded.
    //
    // `qtyToCanonicalPair` catches the UcumException that `getCanonicalForm`
    // raises for an unrecognised code and returns null
    // (FHIRPathEngine.java:2288-2299). `qtyEqual` then falls past the
    // canonicalisation to its code comparison: two identical unknown codes
    // compare on their values, and two differing codes are undecidable.
    assert_eq!(
        eval("1 'zzz' = 1 'zzz'"),
        boolean(true),
        "identical unknown codes compare on the values alone"
    );
    assert_eq!(
        eval("1 'zzz' = 2 'zzz'"),
        boolean(false),
        "and differing values are unequal rather than undecidable"
    );
    assert_eq!(
        eval("1 'zzz' = 1 'cm'"),
        empty(),
        "an unknown code against a known one is undecidable, because only one \
         side canonicalises"
    );

    // Not an error in any of the three, which is the "does not crash" the
    // scenario asks for.
    assert_eq!(eval("(1 'zzz' = 1 'cm').empty()"), boolean(true));
    assert_eq!(eval("1 'zzz' ~ 1 'zzz'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.018 -- a non-UCUM system is not canonicalised
// ---------------------------------------------------------------------------

// covers: path.quantity.018
#[test]
fn a_quantity_whose_system_is_not_ucum_is_not_canonicalised() {
    // `1 year` is the literal-only way to get a quantity with no system at all:
    // the calendar-literal arm sets `system` from
    // `ucum == null ? null : "http://unitsofmeasure.org"` and leaves `ucum`
    // null for `year` and `month` (FHIRPathEngine.java:1109-1132, and
    // `path.literal.013`). Both `qtyToCanonicalPair` and
    // `qtyToCanonicalDecimal` begin with
    // `if (!"http://unitsofmeasure.org".equals(q.getSystem())) return null`
    // (:2289-2291, :2301-2303), so neither produces anything for it.
    assert_eq!(
        eval("1 year = 1 'cm'"),
        empty(),
        "with one side uncanonicalisable and the codes differing, equality is \
         undecidable -- which is a different answer from the no-UCUM-data case \
         in path.quantity.015, where the comparison answers false"
    );

    // The ordered comparison is where the missing canonical value is not
    // handled: `opLessThan` adds the null `qtyToCanonicalDecimal` returned to a
    // one-item list and the recursive call dereferences it at
    // `left.get(0).isPrimitive()` (FHIRPathEngine.java:2447-2451 into :2413),
    // which upstream is a NullPointerException. `path.errors.003` forbids
    // Frevo reproducing that as a panic, so the required behaviour is a
    // returned diagnostic -- never a boolean, and never empty.
    let message = expect_evaluation_error("1 year < 1 'cm'");
    assert!(
        !message.is_empty(),
        "the diagnostic must say that the operand has no canonical form"
    );

    // The same shape with both sides uncanonicalisable: `1 year` against
    // `1 month` have no code either, so the unit comparison decides and the
    // canonicalisation is never reached.
    assert_eq!(eval("1 year = 1 year"), boolean(true));

    // And a quantity that does carry the UCUM system canonicalises, which is
    // what makes the above about the system and not about calendar words.
    assert_eq!(eval("1 'wk' = 7 'd'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.019 -- the server-facing worker raises; Frevo answers (DEV-0029)
// ---------------------------------------------------------------------------

// covers: path.quantity.019
#[test]
fn frevo_answers_as_the_suite_worker_where_hapis_server_worker_raises_hapi_0274() {
    // HAPI's server-facing engine (`FhirPathR4`, `SearchParamExtractorR4`) runs
    // on `HapiWorkerContext`, whose `getUcumService()` throws
    // `UnsupportedOperationException("HAPI-0274: ")`
    // (HapiWorkerContext.java:372-374), so every quantity operation that
    // consults UCUM raises there. That half is differential-only, against a
    // live HAPI 8.12.0. This test is Frevo's half: in every configuration it
    // answers as the suite's worker does (`DEV-0029`, pending the owner), and
    // the operations that never consult UCUM answer the same under both
    // workers.
    let quantities = load_input("parameters-example-quantities.xml");
    let unit_text = load_input("parameters-quantity-unit-text.xml");

    // --- never consults the worker: the same answer under both workers ----

    // `comparable()` with equal codes is true without asking UCUM
    // (`funcComparable`, FHIRPathEngine.java:4628-4630).
    assert_eq!(eval("1 'cm'.comparable(2 'cm')"), boolean(true));
    // A quantity with no system, or no code, is not comparable (:4622-4627).
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value.comparable(Parameters.parameter[3].value)",
            Some(&quantities)
        ),
        boolean(false),
        "two `tablets` quantities with no system are not comparable"
    );
    assert_eq!(
        eval("1 year.comparable(1 'cm')"),
        boolean(false),
        "`1 year` has no system and no code"
    );

    // Ordered comparisons whose `unit` lists are equal answer by value
    // (path.quantity.013).
    expect_case("testLessOrEqual", "testLessOrEqual30");
    expect_case("testGreaterThan", "testGreaterThan30");
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value <= Parameters.parameter[1].value",
            Some(&unit_text)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[1].value > Parameters.parameter[0].value",
            Some(&unit_text)
        ),
        boolean(true)
    );

    // `<=` with no `unit` on the left and one on the right fails *before* the
    // worker is consulted, under both workers: `lunit.equals(runit)` on a null
    // `lunit` (FHIRPathEngine.java:2533-2538) is a NullPointerException, not
    // `HAPI-0274: `. Frevo returns an execution error with no offset.
    let message = expect_execution_error_over(
        "Parameters.parameter[0].value <= Parameters.parameter[2].value",
        Some(&quantities),
    );
    assert!(
        !message.contains("HAPI-0274"),
        "the `<=` failure is the NullPointerException, not the worker's: {message:?}"
    );
    // --- consults UCUM: HAPI's server worker raises `HAPI-0274: `, Frevo ----
    // --- answers as the suite's worker. These need the unit algebra (task 43).

    // `qtyEqual` / `qtyEquivalent` for two valued quantities (:2265, :2338).
    assert_eq!(eval("1 'mg' = 1 'mg'"), boolean(true));
    assert_eq!(eval("4.0000 'g' = 4000.0 'mg'"), boolean(true));
    assert_eq!(eval("4 'g' ~ 4040 'mg'"), boolean(true));
    // Ordered comparisons whose code or unit lists differ (:2443, :2490).
    assert_eq!(eval("6 days < 1 week"), boolean(true));
    assert_eq!(eval("8 days > 1 week"), boolean(true));
    assert_eq!(eval("1 'cm' < 1 's'"), boolean(true));
    // `*` and `/` of two quantities (:2793, :3013).
    assert_eq!(eval("2.0 'cm' * 2.0 'm' = 0.040 'm2'"), boolean(true));
    assert_eq!(eval("4.0 'g' / 2.0 'm' = 2 'g/m'"), boolean(true));
    // `comparable()` with two UCUM codes that differ (:4631).
    assert_eq!(eval("1 'cm'.comparable(1 '[in_i]')"), boolean(true));
    assert_eq!(eval("1 'cm'.comparable(1 's')"), boolean(false));
}
