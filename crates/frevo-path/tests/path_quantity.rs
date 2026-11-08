//! Quantities and UCUM: `path.quantity.001` through `path.quantity.018`.
//!
//! ```text
//! cargo test -p frevo-path --test path_quantity
//! ```
//!
//! # Scenarios against the pinned source -- read before fixing a red test here
//!
//! Several of the suite cases these scenarios name cannot pass against the
//! pinned R4 engine. The vendored `fhir-test-cases` file is newer than the
//! engine (`oracles/PINS.md` against `reference/PINS.md`), and the quantity
//! comparison code in `FHIRPathEngine` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094` has two defects the newer cases
//! were written to catch:
//!
//! 1. **`qtyEqual` and `qtyEquivalent` guard their unit comparison with
//!    `else if (!left.hasUnit() || right.hasUnit())`** -- FHIRPathEngine.java
//!    :2282 and :2355. Read it with both sides unitless: `!false || false` is
//!    true, so it enters and the inner `!(left.hasUnit() && right.hasUnit())`
//!    is also true, and the method returns null. Two unitless quantities are
//!    therefore *undecidable* rather than equal. Read it with a unit on the
//!    left and none on the right: `!true || false` is false, so the guard is
//!    skipped entirely and the values compare -- a quantity with a unit equals
//!    one without. Both readings are the opposite of what the suite expects.
//!    The condition looks like a typo for `if (left.hasUnit() || right.hasUnit())`.
//!
//! 2. **The four ordered comparisons disagree about which child identifies the
//!    unit.** `opLessThan` compares the `code` children (:2438-2439),
//!    `opGreater` and `opGreaterOrEqual` compare the `unit` children (:2481-2482,
//!    :2573-2574), and `opLessOrEqual` compares the `unit` children as strings
//!    (:2514-2517). When the comparison says the units differ, each falls into
//!    `qtyToCanonicalDecimal`, which returns **null** for any quantity whose
//!    `system` is not exactly `http://unitsofmeasure.org` (:2301-2303). The
//!    null is then added to a list and dereferenced by the recursive call's
//!    `left.get(0).isPrimitive()` (:2413), which is a NullPointerException.
//!
//! The affected cases, each of which the scenarios here require to pass:
//!
//! | Case | Expects | Pinned R4 engine |
//! |---|---|---|
//! | `testQuantityUnitComparison/testQuantityNoUnitEqual` | `true` | empty, by defect 1 |
//! | `testQuantityUnitComparison/testQuantityNoUnitEquivalent` | `true` | empty, by defect 1 |
//! | `testQuantityUnitComparison/testQuantityUnitVsNoUnitEqual` | empty | `true`, by defect 1 |
//! | `testQuantityUnitComparison/testQuantityUnitVsNoUnitEquivalent` | `false` | `true`, by defect 1 |
//! | `testQuantityUnitComparison/testQuantityNoUnitVsUnitEquivalent` | `false` | raises, by defect 1 unboxing a null Boolean in `doEquivalent` (:2205-2207) |
//! | `testLessThan/testLessThan33` | `true` | raises, by defect 2 |
//!
//! The tests below are written to the scenarios as stated, because a test that
//! quietly encoded the defect would hide the decision rather than force it.
//! Whoever resolves this needs either a `DEV-nnnn` entry in
//! `docs/deviations.md` for each defect Frevo chooses to reproduce, or a
//! `docs/known-gaps.md` entry with the owner's approval, or a correction to the
//! spec delta. It is not a decision a test can make.

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
fn two_unitless_quantities_compare_equal_and_equivalent() {
    // See the module note, defect 1: the pinned engine answers empty for both
    // of these, because `qtyEqual`'s unit guard is entered for two unitless
    // quantities and returns null (FHIRPathEngine.java:2282-2286).
    for name in ["testQuantityNoUnitEqual", "testQuantityNoUnitEquivalent"] {
        let (ty, body) = sole_output("testQuantityUnitComparison", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testQuantityUnitComparison", name);
    }

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

    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value = Parameters.parameter[1].value",
            Some(&quantities)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value ~ Parameters.parameter[1].value",
            Some(&quantities)
        ),
        boolean(true)
    );
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
fn a_quantity_with_a_unit_compared_to_one_without_yields_empty_for_equality() {
    // See the module note, defect 1: the pinned engine answers `true` for the
    // first of these two, because with a unit on the left and none on the right
    // `!left.hasUnit() || right.hasUnit()` is false and the guard is skipped
    // altogether (FHIRPathEngine.java:2282).
    for name in [
        "testQuantityUnitVsNoUnitEqual",
        "testQuantityNoUnitVsUnitEqual",
    ] {
        expects_no_output("testQuantityUnitComparison", name);
        expect_case("testQuantityUnitComparison", name);
    }

    let quantities = load_input("parameters-example-quantities.xml");
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value = Parameters.parameter[0].value",
            Some(&quantities)
        ),
        empty(),
        "a `tablets` quantity against a unitless one is undecidable, not equal"
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value = Parameters.parameter[2].value",
            Some(&quantities)
        ),
        empty(),
        "and the comparison is symmetric"
    );
}

// ---------------------------------------------------------------------------
// path.quantity.012 -- a unit against no unit, under `~`
// ---------------------------------------------------------------------------

// covers: path.quantity.012
#[test]
fn a_quantity_with_a_unit_compared_to_one_without_yields_false_for_equivalence() {
    // See the module note, defect 1: the pinned engine answers `true` for the
    // first of these and raises on the second, where `doEquivalent` unboxes the
    // null `Boolean` that `qtyEquivalent` returned
    // (FHIRPathEngine.java:2205-2207).
    for name in [
        "testQuantityUnitVsNoUnitEquivalent",
        "testQuantityNoUnitVsUnitEquivalent",
    ] {
        let (ty, body) = sole_output("testQuantityUnitComparison", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "false"));
        expect_case("testQuantityUnitComparison", name);
    }

    let quantities = load_input("parameters-example-quantities.xml");
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value ~ Parameters.parameter[0].value",
            Some(&quantities)
        ),
        boolean(false),
        "equivalence answers a boolean where equality answers empty"
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value ~ Parameters.parameter[2].value",
            Some(&quantities)
        ),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.quantity.013 -- unit text without a code, through the dedicated input
// ---------------------------------------------------------------------------

// covers: path.quantity.013
#[test]
fn the_three_cases_using_the_unit_text_input_hold() {
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

    // See the module note, defect 2: the first of these three raises in the
    // pinned engine. `opLessThan` identifies the unit by the `code` child, and
    // parameter[0] has none, so the units are judged to differ and the
    // canonicalisation of parameter[0] returns null. The other two identify the
    // unit by the `unit` child, which the two operands share, so they never
    // reach the canonicalisation at all.
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
        expect_case(group, name);
    }

    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value < Parameters.parameter[1].value",
            Some(&unit_text)
        ),
        boolean(true)
    );
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
    Box::new(UnboundUcum)
}

struct UnboundUcum;

impl UcumOracle for UnboundUcum {
    fn ucum_version(&mut self) -> Result<String, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn load_ucum(&mut self, _path: &Path) -> Result<String, EngineError> {
        Err(EngineError::NotImplemented)
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
