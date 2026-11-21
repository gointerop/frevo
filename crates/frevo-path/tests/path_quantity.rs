//! Quantities and UCUM: `path.quantity.001` through `path.quantity.026`.
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
//!    `isPrimitive()` on it (:2413) is a `NullPointerException`.
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
//! assertions stay red until the unit algebra exists (task 43).
//!
//! # The unit algebra (`path.quantity.016`, `.020`-`.024`)
//!
//! Every UCUM value asserted here is one the spec states, derived from
//! `reference/Ucum-java` at `db9d22ade3f0f4a667b45817b85a9ac4534e4224`
//! (`org.fhir:ucum` 1.0.10; live HAPI 8.12.0 runs 1.0.9, identical in every
//! class cited) over the pinned `ucum-essence.xml` (`2.0.1`), and confirmed by
//! compiling that source; none is computed here. Canonical forms that are not
//! observable through an operator are graded in-process through the
//! [`UcumOracle::canonical_form`] seam, which answers `NotImplemented` until
//! task 43 gives the crate a canonical-form API to bind it to.
//!
//! Positions in a `*` error's ` (@char C)` suffix are HAPI's lexer column of
//! the `*` token (`getOpStart()`, FHIRPathEngine.java:2801; the column rule of
//! `path.errors.002`: a token adds its length, a space adds 2), and Frevo's
//! offset is that token's zero-based byte offset.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, Document, SuiteInput};
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

/// The JVM's message for `opLessOrEqual` evaluating `lunit.equals(runit)` with
/// a null `lunit` (FHIRPathEngine.java:2533-2538), as the in-process HAPI
/// differential records it (`path.compare.014#01`, `path.quantity.019#17`,
/// `#18`).
const NPE_LUNIT_EQUALS: &str = "Cannot invoke \"String.equals(Object)\" because \"lunit\" is null";

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

fn string(value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: "string".to_string(),
        value: value.to_string(),
    }]
}

/// What `pairToQty` builds (FHIRPathEngine.java:2314-2316): the value, the
/// UCUM system, the canonical code -- absent when the canonical code is empty
/// (`Quantity.setCode`, Quantity.java:620-629) -- and no `unit`.
fn ucum_result(value: &str, code: Option<&str>) -> Vec<ResultItem> {
    vec![ResultItem::Quantity(QuantityValue {
        value: value.to_string(),
        comparator: None,
        unit: None,
        system: Some(UCUM_SYSTEM.to_string()),
        code: code.map(str::to_string),
        has_element_id: false,
        has_extensions: false,
    })]
}

/// `expr` over `inp` must parse and check, then fail at evaluation with a
/// diagnostic; returns the message and the offset as reported.
fn expect_error_with_offset(expr: &str, inp: Option<&SuiteInput>) -> (String, Option<usize>) {
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
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, offset }) => (message, offset),
        Err(e) => panic!("{expr:?} must fail with an execution error, not: {e}"),
    }
}

/// A `*` whose UCUM service raised `UcumException`: `opTimes` rethrows
/// `new PathEngineException(e.getMessage(), null, expr.getOpStart(), ...)`
/// (FHIRPathEngine.java:2800-2801), so the message is the exception's text,
/// then ` (@char C)` naming the `*` token, and the offset is that token's.
fn expect_times_error(expr: &str, inp: Option<&SuiteInput>, ucum_message: &str, column: usize) {
    let at = expr
        .find(" * ")
        .map(|i| i + 1)
        .unwrap_or_else(|| panic!("{expr:?} must contain one `*` operator"));
    let (message, offset) = expect_error_with_offset(expr, inp);
    assert_eq!(
        message,
        format!("{ucum_message} (@char {column})"),
        "{expr:?}: the UcumException's message and the `*` token's column, exactly"
    );
    assert_eq!(
        offset,
        Some(at),
        "{expr:?}: the offset is the `*` token's zero-based byte offset"
    );
}

/// The JVM's message for `divideBy` reading the `null` code of a UCUM-system
/// quantity that has no `code` (UcumEssenceService.java:322-323,
/// `path.quantity.021`).
const NPE_PAIR_CODE_CONTAINS: &str = "Cannot invoke \"String.contains(java.lang.CharSequence)\" \
     because the return value of \"org.fhir.ucum.Pair.getCode()\" is null";

/// The JVM's message for `multiply` or `divideBy` dereferencing the `null`
/// pair `qtyToPair` returns for a quantity without the UCUM system, naming the
/// `UcumEssenceService` parameter (`path.quantity.018`).
fn npe_pair_get_value(parameter: &str) -> String {
    format!("Cannot invoke \"org.fhir.ucum.Pair.getValue()\" because \"{parameter}\" is null")
}

/// A hand-built `Parameters` whose `parameter[i]` is the i-th
/// `valueQuantity` JSON object, given as its members' text.
fn parameters_with_quantities(name: &str, quantities: &[&str]) -> SuiteInput {
    let params: Vec<String> = quantities
        .iter()
        .enumerate()
        .map(|(i, q)| format!(r#"{{"name":"p{i}","valueQuantity":{{{q}}}}}"#))
        .collect();
    let body = format!(
        r#"{{"resourceType":"Parameters","parameter":[{}]}}"#,
        params.join(",")
    );
    let document = Document::Json(
        oracle::json::parse(&body)
            .unwrap_or_else(|e| panic!("the hand-built Parameters is JSON: {e}")),
    );
    SuiteInput {
        file: format!("hand-built-parameters-{name}.json"),
        bytes: body.into_bytes(),
        document,
    }
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

/// The UCUM-dependent surface.
pub trait UcumOracle {
    /// The `version` attribute of the UCUM data in force, reported beside the
    /// suite result (`path.quantity.016`).
    fn ucum_version(&mut self) -> Result<String, EngineError>;

    /// Load UCUM data from a file. An absent or unparseable file is an error,
    /// never a silent fall back to treating units as incomparable.
    fn load_ucum(&mut self, path: &Path) -> Result<String, EngineError>;

    /// `UcumEssenceService.getCanonicalForm(new Pair(new Decimal(value), code))`
    /// over the pinned file (UcumEssenceService.java:266-285): the canonical
    /// value's `asDecimal()` and the canonical unit string
    /// (`ExpressionComposer.compose`), `""` for a dimensionless code. `value`
    /// is the plain string the engine passes (`toPlainString()`,
    /// FHIRPathEngine.java:2293), so a whole `value` takes the
    /// `checkForCouldBeWholeNumber` step (:279-282). A `UcumException`, or the
    /// `NumberFormatException`, `NullPointerException` or `Error` the library
    /// lets escape, is a [`EngineError::Diagnostic`] carrying its
    /// `getMessage()` and no offset.
    fn canonical_form(&mut self, value: &str, code: &str) -> Result<(String, String), EngineError>;
}

fn ucum_oracle_under_test() -> Box<dyn UcumOracle> {
    Box::new(BoundUcum)
}

/// Bound over `frevo_path::UcumEssence` for loading and reporting the data
/// and over its `canonical_form` for the unit algebra (task 43). There is no
/// way to evaluate without UCUM data:
/// Frevo has no such configuration (`path.quantity.015`, `.016`).
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

    /// Bound to `frevo_path::UcumEssence::canonical_form` over the pinned file
    /// (task 43's port of `UcumEssenceService.getCanonicalForm`). Every
    /// `UcumError` is a `Diagnostic` with its `getMessage()` and no offset: a
    /// `UcumException`'s text (`.023`, `.024`), the `NumberFormatException`'s
    /// (`.024`), and the `NullPointerException`'s or `Error`'s (`.021`).
    fn canonical_form(&mut self, value: &str, code: &str) -> Result<(String, String), EngineError> {
        pinned_essence()?
            .canonical_form(value, code)
            .map_err(|e| EngineError::Diagnostic {
                message: e.message().to_owned(),
                offset: None,
            })
    }
}

/// The pinned essence file, loaded once for every canonical-form call. A load
/// failure is a graded diagnostic naming the file, never a silent fallback.
fn pinned_essence() -> Result<&'static frevo_path::UcumEssence, EngineError> {
    static ONCE: OnceLock<Result<frevo_path::UcumEssence, String>> = OnceLock::new();
    ONCE.get_or_init(|| {
        frevo_path::UcumEssence::load(&ucum_essence_path()).map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(|message| EngineError::Diagnostic {
        message: message.clone(),
        offset: None,
    })
}

/// The canonical form of `value` `code`, which must canonicalise.
fn canonical(value: &str, code: &str) -> (String, String) {
    ucum_oracle_under_test()
        .canonical_form(value, code)
        .unwrap_or_else(|e| panic!("{value} '{code}' must canonicalise: {e}"))
}

/// The `UcumException` message canonicalising `code` raises.
fn canonical_error(code: &str) -> String {
    match ucum_oracle_under_test().canonical_form("1", code) {
        Err(EngineError::Diagnostic { message, offset }) => {
            assert_eq!(offset, None, "{code:?}: a UcumException carries no offset");
            message
        }
        Ok(c) => panic!("{code:?} must raise UcumException, but canonicalised to {c:?}"),
        Err(e) => panic!("{code:?}: the UCUM service gave no graded answer: {e}"),
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
    // 0.01 metres, `1 's'` stays 1 second, and `0.01 < 1` is true. The shared
    // engine is configured with the pinned data, which is the only UCUM state
    // Frevo has.
    assert_eq!(
        eval("1 'cm' < 1 's'"),
        boolean(true),
        "0.01 metres compared against 1 second is true, because only the          magnitudes are compared"
    );
    // A boolean, not an error and not empty.
    assert_eq!(eval("(1 'cm' < 1 's').empty()"), boolean(false));

    // The engine's other branch, for a worker whose `getUcumService()` is
    // null (`makeBoolean(false)` when the code or unit lists differ,
    // FHIRPathEngine.java:2443-2445, :2490-2492, :2540-2542, :2588-2590), is
    // reached by neither HAPI configuration and is not reachable in Frevo,
    // which has no configuration without UCUM data, so the spec does not
    // grade it. What is graded is that Frevo cannot silently fall into it:
    // UCUM data that cannot be loaded is a failure, never an engine that
    // treats units as incomparable (path.quantity.016).
    let mut oracle = ucum_oracle_under_test();
    let absent = input::repo_path("oracles/fhir-test-cases/fetched/ucum/does-not-exist.xml");
    expect_load_failure(&mut *oracle, "an absent UCUM file", &absent);
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

    // The root's other two attributes, exactly as the suite worker's file
    // (`org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml` at
    // `73c432b5`) carries them, so the fetched copy is pinned to that file and
    // not to Ucum-java's bundled `2.2` file.
    for attr in [
        "revision=\"$Revision: 439 $\"",
        "revision-date=\"$Date: 2015-11-13 15:13:19 -0500 (Fri, 13 Nov 2015) $\"",
    ] {
        assert!(
            text.contains(attr),
            "the pinned UCUM data must carry {attr}"
        );
    }

    // The suite worker's class-path resource is byte-identical to the fetched
    // copy. `reference/` is cloned on demand and never in CI, so this half is
    // graded only where the clone is present; the fetched copy, which CI
    // fetches, is graded unconditionally above and below.
    let class_path = input::repo_path(
        "reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml",
    );
    if class_path.is_file() {
        let pinned = std::fs::read(&class_path).expect("the class-path resource is readable");
        assert!(
            pinned == text.as_bytes(),
            "the fetched copy must be byte-identical to the suite worker's file"
        );
    }

    // The file as `XmlDefinitionsParser.parse` reads it (:64-106) yields 24
    // prefixes, the 7 base units in file order, and 303 defined units.
    let essence = frevo_path::UcumEssence::load(&path)
        .unwrap_or_else(|e| panic!("the pinned UCUM file must load: {e}"));
    assert_eq!(essence.version(), UCUM_ESSENCE_VERSION);
    assert_eq!(essence.revision(), Some("$Revision: 439 $"));
    assert_eq!(
        essence.revision_date(),
        Some("$Date: 2015-11-13 15:13:19 -0500 (Fri, 13 Nov 2015) $")
    );
    assert_eq!(essence.prefixes().len(), 24);
    assert_eq!(
        essence
            .base_units()
            .iter()
            .map(|b| b.code.as_str())
            .collect::<Vec<_>>(),
        ["m", "s", "g", "rad", "K", "C", "cd"]
    );
    assert_eq!(essence.units().len(), 303);

    // A unit is metric or special only when the attribute is exactly `yes`
    // (:109-121): any other text loads, as not metric. `mol` is metric in the
    // pinned file.
    assert!(
        essence.unit("mol").is_some_and(|u| u.is_metric),
        "mol is metric in the pinned file"
    );
    let not_exactly_yes = mutate(
        &text,
        "<unit Code=\"mol\" CODE=\"MOL\" isMetric=\"yes\"",
        "<unit Code=\"mol\" CODE=\"MOL\" isMetric=\"YES\"",
    );
    let relaxed = frevo_path::UcumEssence::parse(&not_exactly_yes)
        .unwrap_or_else(|e| panic!("isMetric=\"YES\" is not a load failure: {e}"));
    assert!(
        relaxed.unit("mol").is_some_and(|u| !u.is_metric),
        "isMetric=\"YES\" is not `yes`, so mol is read as not metric"
    );

    // A short `revision-date` (25 characters or fewer) is read as
    // `yyyy-MM-dd` (:79-88; the branch 1.0.10 has).
    let short_date = mutate(
        &text,
        "revision-date=\"$Date: 2015-11-13 15:13:19 -0500 (Fri, 13 Nov 2015) $\"",
        "revision-date=\"2015-11-13\"",
    );
    let short = write_fixture("short-revision-date", &short_date);
    assert_eq!(
        oracle
            .load_ucum(&short)
            .unwrap_or_else(|e| panic!("a yyyy-MM-dd revision-date must load: {e}")),
        UCUM_ESSENCE_VERSION
    );

    // Every load failure fails the run rather than silently treating units as
    // incomparable (UcumEssenceService.java:91-99, XmlDefinitionsParser.java
    // :64-70, :103-105). One file per condition; Frevo's message must name the
    // file, and its text is otherwise not graded (no HAPI surface shows it).
    let absent = input::repo_path("oracles/fhir-test-cases/fetched/ucum/does-not-exist.xml");
    expect_load_failure(&mut *oracle, "an absent file", &absent);
    // Unreadable: a directory where the file should be.
    let unreadable = fixture_dir().join("unreadable.xml");
    std::fs::create_dir_all(&unreadable).expect("the fixture directory can be created");
    expect_load_failure(&mut *oracle, "an unreadable file", &unreadable);

    let revision_date = "revision-date=\"$Date: 2015-11-13 15:13:19 -0500 (Fri, 13 Nov 2015) $\"";
    let malformed: [(&str, String); 8] = [
        ("not-well-formed", mutate(&text, "</root>", "</rot>")),
        (
            "doctype",
            mutate(
                &text,
                "<?xml version=\"1.0\" encoding=\"ascii\"?>",
                "<?xml version=\"1.0\" encoding=\"ascii\"?>\n<!DOCTYPE root>",
            ),
        ),
        (
            "root-not-root",
            mutate(&mutate(&text, "<root ", "<ucum "), "</root>", "</ucum>"),
        ),
        ("missing-revision-date", mutate(&text, revision_date, "")),
        (
            // Longer than 25 characters, so characters 7-31 are parsed as
            // `yyyy-MM-dd HH:mm:ss Z`.
            "unparseable-long-revision-date",
            mutate(
                &text,
                revision_date,
                "revision-date=\"$Date: xxxx-11-13 15:13:19 -0500 (Fri, 13 Nov 2015) $\"",
            ),
        ),
        (
            // 25 characters or fewer, so parsed as `yyyy-MM-dd`.
            "unparseable-short-revision-date",
            mutate(&text, revision_date, "revision-date=\"13 Nov 2015\""),
        ),
        (
            "unknown-child-element",
            mutate(&text, "</root>", "<foo/>\n</root>"),
        ),
        (
            "malformed-prefix-value",
            mutate(&text, "<value value=\"1e24\">", "<value value=\"abc\">"),
        ),
    ];
    for (name, body) in &malformed {
        let file = write_fixture(name, body);
        expect_load_failure(&mut *oracle, name, &file);
    }
    let bad_unit = mutate(
        &text,
        "<value Unit=\"[pi].rad/360\" UNIT=\"[PI].RAD/360\" value=\"2\">",
        "<value Unit=\"[pi].rad/360\" UNIT=\"[PI].RAD/360\" value=\"two\">",
    );
    let file = write_fixture("malformed-unit-value", &bad_unit);
    expect_load_failure(&mut *oracle, "malformed-unit-value", &file);

    // And a file that is XML but not UCUM data at all.
    let not_ucum = input::repo_path(oracle::suite::SUITE_PATH);
    expect_load_failure(&mut *oracle, "the FHIRPath suite file", &not_ucum);
}

/// `text` with the single occurrence of `from` replaced by `to`. Requiring
/// exactly one occurrence keeps every fixture a one-condition mutation of the
/// pinned file.
fn mutate(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "the pinned file must contain {from:?} exactly once"
    );
    text.replacen(from, to, 1)
}

fn fixture_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("path_quantity_016");
    std::fs::create_dir_all(&dir).expect("the fixture directory can be created");
    dir
}

fn write_fixture(name: &str, body: &str) -> PathBuf {
    let file = fixture_dir().join(format!("{name}.xml"));
    std::fs::write(&file, body).expect("the fixture can be written");
    file
}

/// Loading `path` must fail with a graded diagnostic that names the file.
fn expect_load_failure(oracle: &mut dyn UcumOracle, condition: &str, path: &Path) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("the fixture has a file name");
    match oracle.load_ucum(path) {
        Err(EngineError::Diagnostic { message, .. }) => assert!(
            message.contains(name),
            "{condition}: the failure must name the file {name:?}: {message:?}"
        ),
        Ok(v) => panic!(
            "{condition}: the UCUM file must fail to load, not report version {v}. Silently \
             treating units as incomparable would turn every quantity comparison into the \
             no-UCUM branch without saying so."
        ),
        Err(e) => panic!("{condition}: the engine gave no graded answer: {e}"),
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

    // `zzz` is not a unit of the pinned file, so parsing it raises
    // UcumException (ExpressionParser.java:130-134), which
    // `qtyToCanonicalPair` turns into null (FHIRPathEngine.java:2292-2298),
    // and `qtyEqual` falls back to the `code` rule (:2276-2285). Different
    // codes are null, which `opEquals` makes empty (:2114-2127), on either
    // side and under `!=` too.
    assert_eq!(eval("1 'zzz' != 1 'cm'"), empty());
    assert_eq!(eval("1 'cm' = 1 'zzz'"), empty());

    // Under `~` the same null is unboxed by `doEquivalent` (:2204-2206): the
    // failure path.quantity.009 states.
    assert_eq!(
        expect_execution_error_over("1 'zzz' ~ 1 'cm'", None),
        NPE_QTY_EQUIVALENT
    );

    // `<` compares the `code` lists (:2438-2441): equal lists compare the
    // values; different ones canonicalise, the unknown unit's canonical
    // decimal is null (:2305-2311), and the recursive `opLessThan` calls
    // `isPrimitive()` on it (:2413).
    assert_eq!(eval("1 'zzz' < 2 'zzz'"), boolean(true));
    for expr in ["1 'zzz' < 1 'cm'", "1 'cm' < 1 'zzz'"] {
        assert_eq!(
            expect_execution_error_over(expr, None),
            NPE_LIST_GET_IS_PRIMITIVE,
            "{expr}"
        );
    }

    // `>`, `>=` and `<=` compare the `unit` lists, which two quoted-code
    // literals do not have (`:1132` sets `unit` only for a calendar keyword),
    // so they compare the values with no UCUM step (:2485-2488, :2533-2539,
    // :2583-2586).
    assert_eq!(eval("2 'zzz' > 1 'cm'"), boolean(true));
    assert_eq!(eval("1 'zzz' >= 1 'cm'"), boolean(true));
    assert_eq!(eval("1 'zzz' <= 2 'cm'"), boolean(true));

    // `isComparable` raises for the unknown unit (UcumEssenceService.java
    // :232-240, :333-343) and `funcComparable` answers false (:4632-4636).
    assert_eq!(eval("1 'zzz'.comparable(1 'cm')"), boolean(false));

    // `*` shows the parse error of path.quantity.024, with the ` (@char C)`
    // suffix naming the `*` token; `/` swallows it and is empty (:3020-3022).
    expect_times_error(
        "1 'zzz' * 1 'm'",
        None,
        "Error processing unit 'zzz.m': The unit 'zzz' is unknown' at position 0",
        11,
    );
    expect_times_error(
        "1 'm' * 1 'zzz'",
        None,
        "Error processing unit 'm.zzz': The unit 'zzz' is unknown' at position 2",
        9,
    );
    assert_eq!(eval("1 'zzz' / 1 'm'"), empty());
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
         undecidable; this is not path.quantity.015's no-UCUM-data branch, \
         which Frevo cannot reach"
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

    // The ordered operators dereference the null canonical decimal with
    // path.quantity.013's message: `<` when the `code` lists differ, `>` when
    // the `unit` lists differ. With equal (empty) code lists, `<` compares the
    // values.
    for expr in ["1 year < 2 'a'", "1 year > 1 'a'"] {
        assert_eq!(
            expect_execution_error_over(expr, None),
            NPE_LIST_GET_IS_PRIMITIVE,
            "{expr}"
        );
    }
    assert_eq!(eval("1 year < 2 years"), boolean(true));

    // `~` with differing codes falls back like `=`, and `doEquivalent`
    // unboxes the null (:2349-2358, :2204-2206).
    assert_eq!(
        expect_execution_error_over("1 year ~ 1 'cm'", None),
        NPE_QTY_EQUIVALENT
    );

    // `*` and `/` pass the null pair `qtyToPair` returns (:2319-2328) to the
    // UCUM service, which dereferences it; the NullPointerException is not a
    // UcumException, so neither arm catches it (:2797-2802, :3017-3022) and
    // `/` fails too. The JVM's message names the `UcumEssenceService`
    // parameter (UcumEssenceService.java:314-325). No offset.
    for (expr, parameter) in [
        ("1 year * 1 'm'", "o1"),
        ("1 'm' * 1 year", "o2"),
        ("1 year / 1 'm'", "dividend"),
        ("1 'm' / 1 year", "divisor"),
    ] {
        assert_eq!(
            expect_execution_error_over(expr, None),
            npe_pair_get_value(parameter),
            "{expr}"
        );
    }

    // Two quantities of the same non-UCUM system: equal codes compare their
    // values, differing codes are empty under `=` and fail under `~`.
    // `comparable()` is false for a missing or different system, and for the
    // same non-UCUM system true only when the codes are equal
    // (`funcComparable`, :4609-4641).
    let other = parameters_with_quantities(
        "non-ucum-system",
        &[
            r#""value":1,"system":"http://example.org/units","code":"x""#,
            r#""value":1,"system":"http://example.org/units","code":"x""#,
            r#""value":2,"system":"http://example.org/units","code":"y""#,
            r#""value":1,"system":"http://example.org/other","code":"x""#,
        ],
    );
    let p = |i: usize| format!("Parameters.parameter[{i}].value");
    assert_eq!(
        eval_over(&format!("{} = {}", p(0), p(1)), Some(&other)),
        boolean(true)
    );
    assert_eq!(
        eval_over(&format!("{} ~ {}", p(0), p(1)), Some(&other)),
        boolean(true)
    );
    assert_eq!(
        eval_over(&format!("{} = {}", p(0), p(2)), Some(&other)),
        empty()
    );
    assert_eq!(
        expect_execution_error_over(&format!("{} ~ {}", p(0), p(2)), Some(&other)),
        NPE_QTY_EQUIVALENT
    );
    // `comparable()`'s argument is evaluated against the focus, not the
    // resource (FHIRPathEngine.java:4613), so the other quantity is reached
    // through `%resource`; a `Parameters.` path there would be a type test
    // against the focus item and yield nothing (`:4614-4616` then answers
    // `false` whatever the codes).
    for (i, j, want) in [(0, 1, true), (0, 2, false), (0, 3, false)] {
        assert_eq!(
            eval_over(
                &format!("{}.comparable(%resource.parameter[{j}].value)", p(i)),
                Some(&other)
            ),
            boolean(want),
            "parameter[{i}].comparable(parameter[{j}])"
        );
    }
    assert_eq!(eval("1 year.comparable(1 'a')"), boolean(false));
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
            // The argument is evaluated against the focus (:4613), so the
            // other quantity is reached through `%resource`.
            "Parameters.parameter[2].value.comparable(%resource.parameter[3].value)",
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
    // The JVM's helpful NullPointerException text naming the local `lunit`,
    // as HAPI 8.12.0 under JDK 25 records it under both workers
    // (`path.quantity.019#17`, `#18`; `path.compare.014`).
    assert_eq!(
        message, NPE_LUNIT_EQUALS,
        "the `<=` failure's text, exactly"
    );
    assert_eq!(
        expect_execution_error_over("1 'kg' <= 1 year", None),
        NPE_LUNIT_EQUALS,
        "`1 'kg' <= 1 year`, the same NullPointerException"
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

    // And every operation of path.quantity.017, .018 and .020 to .024 that
    // reaches one of those calls, including the fallbacks for an unknown,
    // special or non-UCUM unit, which HAPI's server worker takes only after
    // it has been asked (:2265, :2338, :2443, :2793, :3013). Frevo answers as
    // the suite's worker, and a failure is the suite worker's, never
    // `HAPI-0274: `.
    assert_eq!(eval("1 'zzz' = 1 'cm'"), empty());
    assert_eq!(eval("1 'Cel' = 1 'Cel'"), boolean(true));
    assert_eq!(eval("1 'mg/dL' = 10 'mg/L'"), boolean(false));
    assert_eq!(eval("2 'm' * 3 'm'"), ucum_result("6", Some("m2")));
    let message = expect_execution_error_over("1 year < 2 'a'", None);
    assert!(
        !message.contains("HAPI-0274"),
        "Frevo answers as the suite worker, not the server worker: {message:?}"
    );
    assert_eq!(message, NPE_LIST_GET_IS_PRIMITIVE);
}

// ---------------------------------------------------------------------------
// path.quantity.020 -- canonical form
// ---------------------------------------------------------------------------

// covers: path.quantity.020
#[test]
fn a_canonical_form_is_a_value_over_base_units_sorted_by_base_unit_code() {
    // `Converter.normalise` (Converter.java:57-127) then
    // `ExpressionComposer.compose` (ExpressionComposer.java:97-109): base
    // units merged, zero exponents removed, sorted by `String.compareTo`
    // (upper case first), joined by `.`, a dimensionless code giving `""`.
    for (code, want) in [
        ("g/m", "g.m-1"),
        ("kg.m/s2", "g.m.s-2"),
        ("mm[Hg]", "g.m-1.s-2"),
        ("mg/dL", "g.m-3"),
        ("L", "m3"),
        ("l", "m3"),
        ("/min", "s-1"),
        ("10*3/uL", "m-3"),
        ("mol/L", "m-3"),
        ("1", ""),
        ("%", ""),
        ("[ppth]", ""),
        ("10*-3", ""),
        ("{tablets}", ""),
        ("[iU]", ""),
        ("[pi]", ""),
        // Each operator applies to the one component after it, left to
        // right; a parenthesised term is normalised on its own (:65-97).
        ("m/s.s", "m"),
        ("m/(s.s)", "m.s-2"),
    ] {
        assert_eq!(
            canonical("1", code).1,
            want,
            "the canonical unit of {code:?}"
        );
    }

    // The canonical value is the quantity's value times the code's factor,
    // by `Decimal.multiply` (UcumEssenceService.java:268-285).
    for (value, code, want_value, want_code) in [
        ("4.0000", "g", "4.0000", "g"),
        ("4000.0", "mg", "4.0000", "g"),
        ("4", "g", "4", "g"),
        ("4000", "mg", "4", "g"),
        ("4040", "mg", "4.04", "g"),
        ("1", "kg", "1000", "g"),
        ("1", "[lb_av]", "453.59237", "g"),
        ("185", "[lb_av]", "83914.58845", "g"),
        ("1", "[in_i]", "0.0254", "m"),
        ("2.54", "cm", "0.0254", "m"),
        ("1", "[ft_i]", "0.3048", "m"),
        ("12", "[in_i]", "0.3048", "m"),
        ("1.5", "km", "1500", "m"),
        ("7", "d", "604800", "s"),
        ("1", "wk", "604800", "s"),
        ("1", "h", "3600", "s"),
        ("1", "a", "31557600", "s"),
        ("1", "mo", "2629800", "s"),
        ("1", "mo_s", "2551442.976", "s"),
        ("1", "L", "0.001", "m3"),
        ("1", "mL", "0.000001", "m3"),
        ("1", "mg/dL", "10", "g.m-3"),
        ("1", "mm[Hg]", "133322", "g.m-1.s-2"),
        ("1", "/min", "0.01666666666666666666666666666667", "s-1"),
        ("1", "%", "0.01", ""),
        (
            "1",
            "[pi]",
            "3.1415926535897932384626433832795028841971693993751058209749445923",
            "",
        ),
        ("1", "mol/L", "602213670000000000000000000", "m-3"),
        ("-5.5", "mg", "-0.0055", "g"),
        // The whole-number step (`checkForCouldBeWholeNumber`,
        // UcumEssenceService.java:279-282, Decimal.java:831-855) applies only
        // when the value is whole.
        ("60", "/min", "1", "s-1"),
        ("60.0", "/min", "1.0000000000000000000000000000002", "s-1"),
    ] {
        assert_eq!(
            canonical(value, code),
            (want_value.to_string(), want_code.to_string()),
            "the canonical form of {value} '{code}'"
        );
    }

    // A canonical value keeps every digit `Decimal.multiply` produces, and so
    // does the result of `*` built from it: no limit on significant digits
    // (DEBT-0011, which this bullet and path.literal.017 close). HAPI:
    // `path.quantity.020#27`, `#42`-`#47`.
    const PI: &str = "3.1415926535897932384626433832795028841971693993751058209749445923";
    const TWO_PI: &str = "6.2831853071795864769252867665590057683943387987502116419498891846";
    assert_eq!(
        eval("(1 '[pi]' * 1 '1').toString()"),
        string(&format!("{PI} 'null'"))
    );
    assert_eq!(
        eval_unchecked("(1 '[pi]' * 1 '1').value", None),
        one("decimal", PI)
    );
    assert_eq!(eval("1 '[pi]' * 1 'm'"), ucum_result(PI, Some("m")));
    assert_eq!(eval("2 '[pi]' * 1 '1'"), ucum_result(TWO_PI, None));
    // `/` divides to path.math.015's digits.
    assert_eq!(
        eval("1 '[pi]' / 1 '1'"),
        ucum_result("3.14159265358979323846264", None)
    );
    assert_eq!(eval("1 '[pi]' = 1 '1'"), boolean(false));
    assert_eq!(eval("1 '[pi]' < 4 '1'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.quantity.021 -- `*` and `/` answer in canonical units
// ---------------------------------------------------------------------------

// covers: path.quantity.021
#[test]
fn quantity_multiplication_and_division_answer_in_canonical_units() {
    // `multiply` joins the codes as `<left>.<right>`, `divideBy` as
    // `<left>/<right>` with a side holding `/` or `*` parenthesised
    // (UcumEssenceService.java:314-325); the result is canonicalised and
    // built by `pairToQty` with no `unit` (FHIRPathEngine.java:2314-2316).
    for (expr, value, code) in [
        ("2.0 'cm' * 2.0 'm'", "0.040", Some("m2")),
        ("2 'cm' * 2 'm'", "0.04", Some("m2")),
        ("2 'm' * 3 'm'", "6", Some("m2")),
        ("2 'm/s' * 3 's'", "6", Some("m")),
        ("2 'g' * 3 '1'", "6", Some("g")),
        ("2 'm' * 3 '{tablets}'", "6", Some("m")),
        ("2 'kg' * 3 '[lb_av]'", "2721554.22", Some("g2")),
        ("2 'mm[Hg]' * 3 'm2'", "799932", Some("g.m.s-2")),
        ("2 '%' * 3 '%'", "0.0006", None),
        ("4.0 'g' / 2.0 'm'", "2.0", Some("g.m-1")),
        ("1.0 'm' / 1.0 'm'", "1.0", None),
        ("2 'g' / 4 'g'", "0.50", None),
        ("1 'm' / 3 's'", "0.3", Some("m.s-1")),
        (
            "6 'km' / 2 'h'",
            "0.8333333333333333333333333333334",
            Some("m.s-1"),
        ),
        ("6 'kg.m' / 2 's'", "3000", Some("g.m.s-1")),
        ("6 'g/m' / 2 'm/s'", "3", Some("g.m-2.s")),
        ("6 'g/m' / 3 's'", "2.00", Some("g.m-1.s-1")),
        ("6 'm' / 3 's/m'", "2", Some("m2.s-1")),
        ("6 'm.s' / 3 'g'", "2.00", Some("g-1.m.s")),
        ("6 '1' / 3 'm'", "2.00", Some("m-1")),
        ("6 '10*3' / 3 '1'", "2000", None),
    ] {
        assert_eq!(eval(expr), ucum_result(value, code), "{expr}");
    }

    // `toString()` is the value's plain string, a space and the code in
    // quotes (`convertToString`, :887-906), so a dimensionless result prints
    // `'null'`.
    assert_eq!(eval("(2 'm' * 3 'm').toString()"), string("6 'm2'"));
    assert_eq!(eval("(1.0 'm' / 1.0 'm').toString()"), string("1.0 'null'"));

    // Why the suite's three cases are true (path.quantity.006 to .008).
    for name in ["testQuantity9", "testQuantity10", "testQuantity11"] {
        expect_case("testQuantity", name);
    }

    // A zero divisor value is empty under `/`; a zero dividend is `0`.
    assert_eq!(eval("1 'm' / 0 's'"), empty());
    assert_eq!(eval("0 'm' / 0 's'"), ucum_result("0", Some("m.s-1")));

    // A UcumException fails `*` with its message and the `*` suffix, and
    // makes `/` empty (:2800-2801, :3020-3022).
    expect_times_error(
        "1 'm' * 1 'zzz'",
        None,
        "Error processing unit 'm.zzz': The unit 'zzz' is unknown' at position 2",
        9,
    );
    assert_eq!(eval("1 'zzz' / 1 'm'"), empty());

    // A UCUM-system quantity with no `code` enters with the code null
    // (`qtyToPair`, :2319-2328).
    let no_code = parameters_with_quantities(
        "ucum-no-code",
        &[
            r#""value":1,"system":"http://unitsofmeasure.org""#,
            r#""value":1,"system":"http://unitsofmeasure.org","code":"m""#,
            r#""value":0,"system":"http://unitsofmeasure.org","code":"m""#,
            r#""value":0,"system":"http://unitsofmeasure.org""#,
        ],
    );
    let p = |i: usize| format!("Parameters.parameter[{i}].value");
    // Under `*` the joined string spells it `null`. `Parameters.parameter[0]
    // .value` is 29 characters, so the `*` is at column 32, byte 30.
    expect_times_error(
        &format!("{} * {}", p(0), p(1)),
        Some(&no_code),
        "Error processing unit 'null.m': The unit 'null' is unknown' at position 0",
        32,
    );
    // Under `/`, on either side, `divideBy` divides the values and then reads
    // `getCode().contains("/")` (UcumEssenceService.java:322-323): the JVM's
    // NullPointerException, uncaught, with no offset; a zero dividend over a
    // zero divisor still reaches the code ...
    for (l, r) in [(0, 1), (1, 0), (3, 2), (2, 3)] {
        assert_eq!(
            expect_execution_error_over(&format!("{} / {}", p(l), p(r)), Some(&no_code)),
            NPE_PAIR_CODE_CONTAINS,
            "parameter[{l}] / parameter[{r}]"
        );
    }
    // ... and only a zero divisor under a non-zero dividend stays empty,
    // because `Decimal.divide` raises before any code is read
    // (Decimal.java:634-635).
    for (l, r) in [(0, 2), (1, 3)] {
        assert_eq!(
            eval_over(&format!("{} / {}", p(l), p(r)), Some(&no_code)),
            empty(),
            "parameter[{l}] / parameter[{r}]"
        );
    }
    // Under `=` the same quantity is read as the code `1` (:2293).
    assert_eq!(
        eval_over(&format!("{} = 1 '1'", p(0)), Some(&no_code)),
        boolean(true)
    );

    // A quantity and a number are the incompatible-type error (:2803-2804,
    // :3023-3024).
    expect_evaluation_error("2 'm' * 3");
    expect_evaluation_error("2 'm' / 3");
}

// ---------------------------------------------------------------------------
// path.quantity.022 -- canonical comparison
// ---------------------------------------------------------------------------

// covers: path.quantity.022
#[test]
fn canonical_values_compare_as_strings_for_equality_and_as_doubles_for_ordering() {
    // Different canonical unit strings: `=` false, `!=` true, `~` false, not
    // empty (FHIRPathEngine.java:2268-2273, :2341-2346).
    assert_eq!(eval("1 'cm' = 1 's'"), boolean(false));
    assert_eq!(eval("1 'g' != 1 'm'"), boolean(true));
    assert_eq!(eval("1 'cm' ~ 1 's'"), boolean(false));

    // Equal canonical units: the canonical values' strings under
    // path.equality.001's trailing-zero rule (:2270, :2161-2184).
    for (expr, want) in [
        ("4.0000 'g' = 4000.0 'mg'", true),
        ("4 'g' = 4000 'mg'", true),
        ("1 '[in_i]' = 2.54 'cm'", true),
        ("1 'L' = 1000 'mL'", true),
        ("1 'h' = 60 'min'", true),
        ("1 'a' = 365.25 'd'", true),
        ("1 'kg' = 1000 'g'", true),
        ("1 'g/L' = 1000 'mg/L'", true),
        ("185 '[lb_av]' = 83914.58845 'g'", true),
        ("100 '%' = 1 '1'", true),
        ("1 '10*3' = 1000 '1'", true),
        ("60 '/min' = 1 '/s'", true),
        ("4 'g' = 4040 'mg'", false),
        ("1 'mo' = 30 'd'", false),
        // `10` against `10.00`, read as `1` against `10` (DEBT-0020).
        ("1 'mg/dL' = 10 'mg/L'", false),
        ("10 'mg/L' = 1 'mg/dL'", false),
        // The whole-number step of path.quantity.020.
        ("60.0 '/min' = 1 '/s'", false),
    ] {
        assert_eq!(eval(expr), boolean(want), "{expr}");
    }

    // Ordering converts each canonical value with `Double.valueOf`, with no
    // check that the canonical units agree (:2419, :2440-2450).
    for (expr, want) in [
        ("1 'kg' < 2000 'g'", true),
        ("4 'g' < 4040 'mg'", true),
        ("1 'mo' < 30 'd'", false),
        ("6 days < 1 week", true),
        ("1 'cm' < 1 's'", true),
    ] {
        assert_eq!(eval(expr), boolean(want), "{expr}");
    }

    // The suite's cases, all passed by HAPI.
    for n in 1..=8 {
        expect_case("testQuantity", &format!("testQuantity{n}"));
    }
    for (group, name) in [
        ("testLessThan", "testLessThan31"),
        ("testEquality", "testEquality28"),
        ("testNEquality", "testNEquality24"),
        ("testEquivalent", "testEquivalent22"),
        ("testNotEquivalent", "testNotEquivalent22"),
        ("testLessThan", "testLessThan22"),
        ("testLessOrEqual", "testLessOrEqual22"),
        ("testGreatorOrEqual", "testGreatorOrEqual22"),
        ("testGreaterThan", "testGreaterThan22"),
    ] {
        expect_case(group, name);
    }
}

// ---------------------------------------------------------------------------
// path.quantity.023 -- special units
// ---------------------------------------------------------------------------

// covers: path.quantity.023
#[test]
fn special_units_are_not_canonicalised_unless_a_handler_maps_them_to_a_ratio_unit() {
    // Handlers with an offset (Converter.java:168-171).
    for code in ["Cel", "[degF]"] {
        assert_eq!(
            canonical_error(code),
            "Not handled yet (special unit with offset from 0 at intersect)",
            "{code}"
        );
    }
    // Special units with no handler (:163-164); the registry's `[hp_X]` and
    // `[hp_C]` keys do not match the file's codes.
    for code in [
        "[degRe]", "[hp'_X]", "[hp'_C]", "[hp'_M]", "[hp'_Q]", "B[10.nV]",
    ] {
        assert_eq!(
            canonical_error(code),
            "Not handled yet (special unit)",
            "{code}"
        );
    }
    // The seventh never reaches that check: `/` ends a symbol even inside
    // brackets (Lexer.java:136-140).
    assert_eq!(
        canonical_error("[m/s2/Hz^(1/2)]"),
        "Error processing unit '[m/s2/Hz^(1/2)]': The unit '[m' is unknown' at position 0"
    );

    // The other special units canonicalise as the handler's unit: the same
    // canonical unit string as that unit.
    for (special, unit) in [
        ("[p'diop]", "deg"),
        ("%[slope]", "deg"),
        ("[pH]", "mol/l"),
        ("Np", "1"),
        ("B", "1"),
        ("bit_s", "1"),
        ("B[SPL]", "10*-5.Pa"),
        ("B[V]", "V"),
        ("B[mV]", "mV"),
        ("B[uV]", "uV"),
        ("B[W]", "W"),
        ("B[kW]", "kW"),
    ] {
        assert_eq!(
            canonical("1", special).1,
            canonical("1", unit).1,
            "{special} canonicalises in {unit}'s units"
        );
    }
    for expr in [
        "1 '[pH]' = 1 'mol/L'",
        "1 'B' = 1 '1'",
        "1 'Np' = 1 '1'",
        "1 'B[SPL]' = 2 '10*-5.Pa'",
        "1 '%[slope]' = 1 'deg'",
    ] {
        assert_eq!(eval(expr), boolean(true), "{expr}");
    }

    // An operand that raises falls back as path.quantity.017 states.
    assert_eq!(eval("1 'Cel' = 1 'Cel'"), boolean(true));
    assert_eq!(eval("1 'Cel' = 274.15 'K'"), empty());
    assert_eq!(
        expect_execution_error_over("1 'Cel' ~ 274.15 'K'", None),
        NPE_QTY_EQUIVALENT
    );
    assert_eq!(
        expect_execution_error_over("1 'Cel' < 2 'K'", None),
        NPE_LIST_GET_IS_PRIMITIVE
    );
    assert_eq!(eval("1 'Cel'.comparable(1 'K')"), boolean(false));
    expect_times_error(
        "1 'Cel' * 1 'm'",
        None,
        "Not handled yet (special unit with offset from 0 at intersect)",
        11,
    );
    assert_eq!(eval("1 'Cel' / 1 'm'"), empty());
}

// ---------------------------------------------------------------------------
// path.quantity.024 -- the UCUM grammar and its messages
// ---------------------------------------------------------------------------

/// The `NumberFormatException` message `Lexer.getTokenAsInt` raises for a
/// number outside the Java `int` range (`path.quantity.024`).
const NFE_10000000000: &str = "For input string: \"10000000000\"";

// covers: path.quantity.024
#[test]
fn ucum_codes_are_parsed_by_the_pinned_grammar_and_its_errors_are_the_messages_times_shows() {
    // The lexer and parser (Lexer.java:61-140, ExpressionParser.java:48-146):
    // a number after a symbol is its exponent, and a final stray `)` is
    // accepted because the lexer has consumed it (:203-205).
    assert_eq!(canonical("1", "m2").1, "m2");
    assert_eq!(canonical("1", "m)").1, "m");

    // The messages, `<code>` being the whole code and the position the
    // 0-based start of the token being read. The quote before ` at position`
    // is in the library's text.
    for (code, want) in [
        (
            "zzz",
            "Error processing unit 'zzz': The unit 'zzz' is unknown' at position 0",
        ),
        (
            "m.zzz",
            "Error processing unit 'm.zzz': The unit 'zzz' is unknown' at position 2",
        ),
        (
            "[m/s2/Hz^(1/2)]",
            "Error processing unit '[m/s2/Hz^(1/2)]': The unit '[m' is unknown' at position 0",
        ),
        (
            "m(s",
            "Error processing unit 'm(s': Expected '/' or '.'' at position 1",
        ),
        (
            ".m",
            "Error processing unit '.m': unexpected token looking for a symbol or a number' at position 0",
        ),
        (
            "m.)",
            "Error processing unit 'm.)': unexpected token looking for a symbol or a number' at position 2",
        ),
        (
            "{abc",
            "Error processing unit'{abc': Annotation contains non-ascii characters",
        ),
        (
            "m  ",
            "Error processing unit 'm  ': unexpected character ' ' at position 1",
        ),
        (
            "m]",
            "Error processing unit 'm]': ] without [' at position 0",
        ),
        ("m)m", "Expression was not parsed completely. Syntax Error?"),
    ] {
        assert_eq!(canonical_error(code), want, "{code:?}");
    }
    // Templates whose position the spec does not place: the prefix only.
    assert!(canonical_error("m.").starts_with(
        "Error processing unit 'm.': unexpected end of expression looking for a symbol or a \
         number' at position "
    ));
    assert!(canonical_error("(m").starts_with(
        "Error processing unit '(m': Unexpected Token Type 'NONE' looking for a close bracket' \
         at position "
    ));
    // A sign with no digit and a nested `[` raise; their text is not stated.
    for code in ["+", "m-", "[m["] {
        canonical_error(code);
    }

    // For a UcumException only `*` shows the message; `=` falls back to the
    // code rule, `comparable()` is false, `/` is empty, and an ordered
    // operator that canonicalises fails with path.quantity.013's message.
    expect_times_error(
        "1 'm' * 1 'zzz'",
        None,
        "Error processing unit 'm.zzz': The unit 'zzz' is unknown' at position 2",
        9,
    );
    assert_eq!(eval("1 'm' = 1 'zzz'"), empty());
    assert_eq!(eval("1 'm'.comparable(1 'zzz')"), boolean(false));
    assert_eq!(eval("1 'm' / 1 'zzz'"), empty());
    assert_eq!(
        expect_execution_error_over("1 'm' < 1 'zzz'", None),
        NPE_LIST_GET_IS_PRIMITIVE
    );

    // A number outside the Java `int` range throws NumberFormatException from
    // `Lexer.getTokenAsInt` (Lexer.java:199-201), which no arm catches: every
    // operator fails with its message and no offset, except `comparable()`,
    // whose `getCanonicalUnits` re-wraps it (UcumEssenceService.java:237-239).
    for expr in [
        "1 '10000000000' = 1 'm'",
        "1 '10000000000' = 1 '10000000000'",
        "1 '10000000000' ~ 1 'm'",
        "1 '10000000000' ~ 1 '10000000000'",
        "1 '10000000000' < 1 'm'",
        "1 'm10000000000' * 1 'm'",
        "1 'm10000000000' / 1 'm'",
    ] {
        assert_eq!(
            expect_execution_error_over(expr, None),
            NFE_10000000000,
            "{expr}"
        );
    }
    assert_eq!(eval("1 '10000000000'.comparable(1 'm')"), boolean(false));

    // `[s]` is not a unit of the pinned file; `[in_i]` canonicalises to `m`.
    assert_eq!(eval("1 'cm'.comparable(1 '[s]')"), boolean(false));
    assert_eq!(eval("1 'cm'.comparable(1 '[in_i]')"), boolean(true));
    for name in ["Comparable1", "Comparable2", "Comparable3"] {
        expect_case("Comparable", name);
    }
}

// ---------------------------------------------------------------------------
// path.quantity.025 -- a quantity the engine builds navigates as a `Quantity`
// ---------------------------------------------------------------------------

/// `expr` over `inp`, parsed and evaluated with the static check **skipped**,
/// as `FhirPathR4` evaluates and as the differential's probes run with no
/// `contextType` (`path.navigation.019`).
fn eval_unchecked(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
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
        .evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate with the check skipped: {e}"))
}

/// `expr` must fail the static check run with `inp`'s resource type as the
/// context type (HAPI's `check(null, "Resource", type, type, node)`, as the
/// differential's `check: run` probes call it). Returns the message.
fn expect_check_failure_over(expr: &str, inp: &SuiteInput) -> String {
    let mut engine = fresh_engine();
    let resource_type = inp.resource_type();
    assert!(
        resource_type.is_some(),
        "{}: the check needs a context type",
        inp.file
    );
    let context = CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    };
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before the check can reject it: {e}"));
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Ok(()) => panic!(
            "{expr:?} must fail the static check with a context type, as HAPI's \
             FHIRPATH_UNKNOWN_NAME rejects it, but the check passed"
        ),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
    }
}

/// `expr` must pass the static check run with `inp`'s resource type as the
/// context type.
fn expect_check_passes_over(expr: &str, inp: &SuiteInput) {
    let mut engine = fresh_engine();
    let resource_type = inp.resource_type();
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
}

/// A one-item result of a primitive of this FHIR type.
fn one(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

/// `FHIRPATH_UNKNOWN_NAME` (Messages.properties:369) as `executeType` raises
/// it in the check (FHIRPathEngine.java:1590-1593), with the
/// `Error evaluating FHIRPath expression: ` prefix and ` (@char C)` suffix the
/// check's other failures carry.
fn unknown_name(name: &str, types: &str, column: usize) -> String {
    format!(
        "Error evaluating FHIRPath expression: The name '{name}' is not valid for any of the \
         possible types: [{types}] (@char {column})"
    )
}

const SYSTEM_QUANTITY: &str = "http://hl7.org/fhirpath/System.Quantity";

// covers: path.quantity.025
#[test]
fn a_quantity_the_engine_builds_is_a_fhir_quantity_whose_elements_navigate() {
    // Graded in-process against the values the scenario states; HAPI's
    // observations are `path.quantity.021#41`-`#46`, `path.quantity.023#25`
    // and the `path.quantity.025` probes, all with no `contextType`, so with
    // the check skipped.

    // A literal with a quoted UCUM code (FHIRPathEngine.java:1130-1132):
    // value, code and system set; `unit` set only for a calendar keyword
    // (:1109-1110).
    assert_eq!(eval_unchecked("(1 'cm').value", None), one("decimal", "1"));
    assert_eq!(eval_unchecked("(1 'cm').code", None), one("code", "cm"));
    assert_eq!(
        eval_unchecked("(1 'cm').system", None),
        one("uri", UCUM_SYSTEM)
    );
    assert_eq!(eval_unchecked("(1 'cm').unit", None), empty());

    // A calendar-duration literal: `unit` is the keyword as written, `code`
    // and `system` only for a keyword with a UCUM code (:1109-1132).
    assert_eq!(eval_unchecked("(1 year).unit", None), one("string", "year"));
    assert_eq!(eval_unchecked("(1 year).code", None), empty());
    assert_eq!(eval_unchecked("(1 year).system", None), empty());
    assert_eq!(eval_unchecked("(1 week).unit", None), one("string", "week"));
    assert_eq!(eval_unchecked("(1 week).code", None), one("code", "wk"));
    assert_eq!(
        eval_unchecked("(1 week).system", None),
        one("uri", UCUM_SYSTEM)
    );

    // A `*` or `/` result (`pairToQty`, :2314-2316): value, system and the
    // canonical code of path.quantity.021, and never a `unit`; a
    // dimensionless result has no code.
    assert_eq!(
        eval_unchecked("(2 'cm' * 1 'm').code", None),
        one("code", "m2")
    );
    assert_eq!(eval_unchecked("(2 'cm' * 1 'm').unit", None), empty());
    assert_eq!(eval_unchecked("(1 '%' * 1 '1').code", None), empty());

    // The code of a literal is the unit text as path.literal.016 decodes it.
    assert_eq!(
        eval_unchecked(r"(1 '[hp\'_X]').code", None),
        one("code", "[hp''_X]")
    );
}

// covers: path.quantity.025
#[test]
fn the_check_rejects_member_navigation_on_a_quantity_the_engine_builds() {
    // With a context type the check runs and rejects the same navigation:
    // a literal is typed `System.Quantity` (`resolveConstantType`,
    // FHIRPathEngine.java:3118-3119), which has no child types (:6335-6336),
    // and a `*` or `/` result has no type at all (`operateTypes`,
    // :2028-2045). Both messages and positions are derived from the source;
    // the `path.quantity.025` check probes observe HAPI's.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        expect_check_failure_over("(1 'cm').code", &patient),
        unknown_name("code", SYSTEM_QUANTITY, 11)
    );
    assert_eq!(
        expect_check_failure_over("(2 'cm' * 1 'm').code", &patient),
        unknown_name("code", "", 22)
    );

    // A `Quantity` read from a resource is typed as the FHIR `Quantity` and
    // passes the check.
    let observation = load_input("observation-example.xml");
    expect_check_passes_over("(Observation.value as Quantity).code", &observation);
}

// ---------------------------------------------------------------------------
// path.quantity.026 -- an empty quoted unit makes a UCUM quantity with no code
// ---------------------------------------------------------------------------

// covers: path.quantity.026
#[test]
fn an_empty_quoted_unit_makes_a_ucum_quantity_with_no_code() {
    // The literal passes "" to `setCode` (FHIRPathEngine.java:1130-1132),
    // which leaves the code absent (`Quantity.setCode`, Quantity.java:620-629,
    // `Utilities.noString`); the system is set because the unit text is not
    // null.
    assert_eq!(eval("1 ''"), ucum_result("1", None));

    // So it is the UCUM quantity with no code of path.quantity.021, under the
    // suite's worker (HAPI: `path.quantity.021#37`-`#40`,
    // `path.quantity.024#33`, `#34`).
    assert_eq!(eval("1 ''.toString()"), string("1 'null'"));
    assert_eq!(eval("1 '' = 1 '1'"), boolean(true));
    assert_eq!(eval("1 ''.comparable(1 '1')"), boolean(false));
    expect_times_error(
        "1 '' * 1 'm'",
        None,
        "Error processing unit 'null.m': The unit 'null' is unknown' at position 0",
        8,
    );
    expect_times_error(
        "1 'm' * 1 ''",
        None,
        "Error processing unit 'm.null': The unit 'null' is unknown' at position 2",
        9,
    );
    assert_eq!(
        expect_execution_error_over("1 '' / 1 'm'", None),
        NPE_PAIR_CODE_CONTAINS
    );

    // Its elements, with the check skipped (path.quantity.025).
    assert_eq!(eval_unchecked("(1 '').code", None), empty());
    assert_eq!(
        eval_unchecked("(1 '').system", None),
        one("uri", UCUM_SYSTEM)
    );
    assert_eq!(eval_unchecked("(1 '').value", None), one("decimal", "1"));
}

// covers: path.quantity.026
#[test]
fn the_check_rejects_member_navigation_on_an_empty_unit_literal() {
    // The literal is typed `System.Quantity`, so both fail the check as
    // path.quantity.025 states, at the member name's lexer column 9
    // (whitespace counting 2, FHIRLexer.java:321-324). Derived, not observed;
    // the `path.quantity.026` check probes observe HAPI's.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        expect_check_failure_over("(1 '').code", &patient),
        unknown_name("code", SYSTEM_QUANTITY, 9)
    );
    assert_eq!(
        expect_check_failure_over("(1 '').system", &patient),
        unknown_name("system", SYSTEM_QUANTITY, 9)
    );
}
