//! Boundary functions: `path.boundary.001` through `path.boundary.010`.
//!
//! ```text
//! cargo test -p frevo-path --test path_boundary
//! ```
//!
//! Every scenario here names cases in the suite's `LowBoundary` and
//! `HighBoundary` groups, so each test drives those cases through the committed
//! harness and then states the key values at the engine seam as well. The
//! second half matters because the harness compares a quantity result with
//! `Quantity.equalsDeep`, which uses `BigDecimal.equals` and so is
//! **scale sensitive**: `1.5865` is not equal to `1.58650000`
//! (`tests/oracle/value.rs::decimal_equals`). An implementation that produced
//! the right number with the wrong scale would fail the suite case and the seam
//! assertion alike, which is the intent -- the retained scale is the whole
//! point of a boundary function.
//!
//! The 70 cases in the two groups are partitioned by what they operate on
//! rather than listed by name, so a re-pin that adds a case to a class is
//! visible as a count mismatch instead of silently going ungraded.
//!
//! Provenance: `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, `funcLowBoundary` and
//! `funcHighBoundary`, plus `Utilities.lowBoundaryForDecimal` /
//! `highBoundaryForDecimal` and `DateTimeUtil.lowBoundaryForDate` /
//! `highBoundaryForDate`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{CaseContext, EngineConfig, StrictnessFlags, SuiteEngine, engine_under_test};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::{Case, Suite};
use oracle::value::{QuantityValue, ResultItem, UCUM_SYSTEM};

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// What a boundary case operates on, decided by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subject {
    Decimal,
    Quantity,
    Temporal,
}

fn subject_of(name: &str) -> Subject {
    if name.contains("Quantity") {
        Subject::Quantity
    } else if name.contains("Date") || name.contains("Time") {
        Subject::Temporal
    } else {
        Subject::Decimal
    }
}

/// Every case in `group` whose subject is `subject`.
fn cases_of(group: &str, subject: Subject) -> Vec<&'static Case> {
    let (suite, _) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    g.cases
        .iter()
        .filter(|c| subject_of(&c.address.name) == subject)
        .collect()
}

/// Assert every case in `group` with that subject passed, and that the class
/// still has the size this test was written against.
fn expect_subject(group: &str, subject: Subject, expected: usize) {
    let (_, report) = pinned();
    let cases = cases_of(group, subject);
    assert_eq!(
        cases.len(),
        expected,
        "{group} must have {expected} {subject:?} case(s); a different count means the \
         suite was re-pinned without this test being updated"
    );

    let mut failures = Vec::new();
    for case in &cases {
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
        "{} of {} {subject:?} cases in {group} did not pass; the first few:\n  {}",
        failures.len(),
        cases.len(),
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

/// The single expected `<output>` body the pinned file records for a case. The
/// `@` prefix a temporal expectation carries is part of the expectation and is
/// never stripped here (`tests/oracle/value.rs::fhirpath_string`).
fn sole_output_body(group: &str, name: &str) -> String {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert_eq!(case.outputs.len(), 1, "{group}/{name} has one output");
    case.outputs[0].body.clone()
}

/// A case the suite expects to produce nothing.
fn expects_no_output(group: &str, name: &str) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert!(
        case.outputs.is_empty(),
        "{group}/{name} must expect nothing, but it expects {:?}",
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

fn primitive(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

fn decimal(value: &str) -> Vec<ResultItem> {
    primitive("decimal", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

/// Assert an expression yields exactly this decimal, by its lexical form.
///
/// The scale is asserted and not only the number: `BigDecimal.equals` is
/// scale sensitive, so a result of `1.5865` where `1.58650000` is required is a
/// failure and not a rounding difference.
fn expect_decimal(expr: &str, lexical: &str) {
    assert_eq!(
        eval(expr),
        decimal(lexical),
        "{expr} must yield the decimal {lexical} exactly, scale included"
    );
}

// ---------------------------------------------------------------------------
// path.boundary.001 -- `lowBoundary` on a decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.001
#[test]
fn low_boundary_on_a_decimal_widens_to_the_implied_precision() {
    expect_subject("LowBoundary", Subject::Decimal, 31);

    // The default precision is 8 fractional digits, so a three-digit decimal
    // widens: half a unit in the last stated place, rendered at the default
    // scale.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimalDefault"),
        "1.58650000"
    );
    expect_decimal("1.587.lowBoundary()", "1.58650000");

    // An explicit precision sets the scale, both wider and narrower than the
    // input's.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimal1"),
        "1.586500"
    );
    expect_decimal("1.587.lowBoundary(6)", "1.586500");
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimal2"),
        "1.58"
    );
    expect_decimal("1.587.lowBoundary(2)", "1.58");
    assert_eq!(sole_output_body("LowBoundary", "LowBoundaryDecimal4"), "1");
    expect_decimal("1.587.lowBoundary(0)", "1");

    // And the scale really is retained: the same number at a different scale is
    // a different answer, which is what makes `1.58650000` an assertion about
    // precision rather than about value.
    assert_ne!(
        eval("1.587.lowBoundary()"),
        decimal("1.5865"),
        "the retained scale is part of the result"
    );
}

// ---------------------------------------------------------------------------
// path.boundary.002 -- an out-of-range precision
// ---------------------------------------------------------------------------

// covers: path.boundary.002
#[test]
fn low_boundary_with_an_out_of_range_precision_yields_empty() {
    for name in [
        "LowBoundaryDecimal3",
        "LowBoundaryDecimal5",
        "LowBoundaryDecimal6",
    ] {
        expects_no_output("LowBoundary", name);
        expect_case("LowBoundary", name);
    }

    // Negative, and past the limit on the other side. Empty, not an error and
    // not a clamped result.
    assert_eq!(eval("1.587.lowBoundary(-1)"), empty());
    assert_eq!(eval("1.587.lowBoundary(32)"), empty());
    assert_eq!(eval("1.587.lowBoundary(39)"), empty());
    assert_eq!(
        eval("1.587.lowBoundary(-1).empty()"),
        primitive("boolean", "true")
    );

    // The negative-value mirrors of the same two bounds, which the suite also
    // grades.
    expect_case("LowBoundary", "LowBoundaryNegDecimal3");
    expect_case("LowBoundary", "LowBoundaryNegDecimal5");

    // Zero is in range, so the boundary of the range is where the suite puts it.
    expect_decimal("1.587.lowBoundary(0)", "1");
}

// ---------------------------------------------------------------------------
// path.boundary.003 -- a negative decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.003
#[test]
fn low_boundary_on_a_negative_decimal_moves_away_from_zero() {
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryNegDecimalDefault"),
        "-1.58750000"
    );
    expect_case("LowBoundary", "LowBoundaryNegDecimalDefault");
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryNegDecimal2"),
        "-1.59"
    );
    expect_case("LowBoundary", "LowBoundaryNegDecimal2");

    // The low boundary of a negative number is more negative, so it is the
    // *high* digits of the positive magnitude: -1.587 goes to -1.5875 and not
    // to -1.5865.
    expect_decimal("(-1.587).lowBoundary()", "-1.58750000");
    expect_decimal("(-1.587).lowBoundary(2)", "-1.59");
    expect_decimal("(-1.587).lowBoundary(6)", "-1.587500");
    expect_decimal("(-1.587).lowBoundary(0)", "-2");

    // The positive mirror, so the asymmetry is visible in one test.
    expect_decimal("1.587.lowBoundary()", "1.58650000");
    expect_decimal("1.587.highBoundary()", "1.58750000");
}

// ---------------------------------------------------------------------------
// path.boundary.004 -- carry and borrow
// ---------------------------------------------------------------------------

// covers: path.boundary.004
#[test]
fn low_boundary_carries_and_borrows_across_digit_positions() {
    for (name, expected) in [
        ("LowBoundaryDecimalCarry1", "-10.0"),
        ("LowBoundaryDecimalCarry3", "-10"),
        ("LowBoundaryDecimalBorrow1", "9"),
        ("LowBoundaryDecimalBorrow2", "99.50"),
    ] {
        assert_eq!(
            sole_output_body("LowBoundary", name),
            expected,
            "LowBoundary/{name}"
        );
        expect_case("LowBoundary", name);
    }

    // A carry past the units place, on a negative value.
    expect_decimal("(-9.96).lowBoundary(1)", "-10.0");
    expect_decimal("(-9).lowBoundary(0)", "-10");
    // And a borrow, where the integer part loses a unit.
    expect_decimal("10.lowBoundary(0)", "9");
    expect_decimal("100.lowBoundary(2)", "99.50");

    // The second negative carry case, which the suite also grades.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimalCarry2"),
        "-100.0"
    );
    expect_case("LowBoundary", "LowBoundaryDecimalCarry2");
    expect_decimal("(-99.96).lowBoundary(1)", "-100.0");
}

// ---------------------------------------------------------------------------
// path.boundary.005 -- precision from the stored lexical form
// ---------------------------------------------------------------------------

// covers: path.boundary.005
#[test]
fn low_boundary_reads_precision_from_a_decimals_stored_lexical_form() {
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimalExponent1"),
        "945"
    );
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimalExponent2"),
        "0.00145000"
    );
    for name in [
        "LowBoundaryDecimalExponent1",
        "LowBoundaryDecimalExponent2",
        "LowBoundaryDecimalExponent3",
        "LowBoundaryDecimalExponent4",
    ] {
        expect_case("LowBoundary", name);
    }

    // The input's decimals are written with exponents -- `9.5e2`, `1.5e-3`,
    // `-9.5e2`, `1.5E2` -- and the precision the boundary is computed from is
    // the mantissa's, not the expanded form's. `9.5e2` is 950 to two
    // significant digits, so its low boundary at scale 0 is 945 and not 949.
    let decimals = load_input("parameters-example-decimals.xml");
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value.lowBoundary(0)",
            Some(&decimals)
        ),
        decimal("945")
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[1].value.lowBoundary()",
            Some(&decimals)
        ),
        decimal("0.00145000"),
        "1.5e-3 has two significant digits, so the boundary is 0.00145"
    );

    // The negative-exponent and upper-case-exponent spellings are read the same
    // way, which is what makes this about the lexical form rather than about
    // one notation.
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value.lowBoundary(0)",
            Some(&decimals)
        ),
        decimal("-955")
    );
    assert_eq!(
        eval_over(
            "Parameters.parameter[3].value.lowBoundary(1)",
            Some(&decimals)
        ),
        decimal("145.0")
    );
}

// ---------------------------------------------------------------------------
// path.boundary.006 -- `highBoundary` on a decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.006
#[test]
fn high_boundary_on_a_decimal_widens_to_the_implied_precision() {
    expect_subject("HighBoundary", Subject::Decimal, 27);

    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryDecimalDefault"),
        "1.58750000"
    );
    expect_decimal("1.587.highBoundary()", "1.58750000");
    expect_decimal("1.587.highBoundary(2)", "1.59");
    expect_decimal("1.587.highBoundary(6)", "1.587500");
    expect_decimal("(-1.587).highBoundary()", "-1.58650000");
    expect_decimal("(-1.587).highBoundary(2)", "-1.58");

    // Out of range is empty here too.
    assert_eq!(eval("1.587.highBoundary(-1)"), empty());
    assert_eq!(eval("1.587.highBoundary(39)"), empty());

    // And the scale is retained, as it is for `lowBoundary`.
    assert_ne!(eval("1.587.highBoundary()"), decimal("1.5875"));
}

// ---------------------------------------------------------------------------
// path.boundary.007 -- `highBoundary` carries
// ---------------------------------------------------------------------------

// covers: path.boundary.007
#[test]
fn high_boundary_carries_across_digit_positions() {
    for (name, expected) in [
        ("HighBoundaryDecimalCarry1", "10.0"),
        ("HighBoundaryDecimalCarry2", "1.0"),
        ("HighBoundaryDecimalCarry3", "100.0"),
        ("HighBoundaryDecimalCarry4", "10"),
    ] {
        assert_eq!(
            sole_output_body("HighBoundary", name),
            expected,
            "HighBoundary/{name}"
        );
        expect_case("HighBoundary", name);
    }

    expect_decimal("9.95.highBoundary(1)", "10.0");
    expect_decimal("0.996.highBoundary(1)", "1.0");
    expect_decimal("99.96.highBoundary(1)", "100.0");
    expect_decimal("9.highBoundary(0)", "10");
}

// ---------------------------------------------------------------------------
// path.boundary.008 -- quantities
// ---------------------------------------------------------------------------

// covers: path.boundary.008
#[test]
fn the_boundary_functions_work_on_quantities() {
    expect_subject("LowBoundary", Subject::Quantity, 1);
    expect_subject("HighBoundary", Subject::Quantity, 1);

    // The expectations are bare `<output>` bodies, which the harness parses
    // with `parseQuantityString` and compares with `Quantity.equalsDeep`
    // (`tests/oracle/compare.rs`). That comparison is scale sensitive, so
    // `1.58650000 'cm'` asserts the retained precision and not merely the
    // magnitude.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryQuantity"),
        "1.58650000 'cm'"
    );
    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryQuantity"),
        "1.58750000 'm'"
    );
    expect_case("LowBoundary", "LowBoundaryQuantity");
    expect_case("HighBoundary", "HighBoundaryQuantity");

    // The unit survives the boundary computation, which only touches the value.
    assert_eq!(
        eval("1.587 'cm'.lowBoundary(8)"),
        vec![ResultItem::Quantity(QuantityValue {
            value: "1.58650000".to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("cm".to_string()),
            has_element_id: false,
            has_extensions: false,
        })]
    );
    assert_eq!(
        eval("1.587 'm'.highBoundary(8)"),
        vec![ResultItem::Quantity(QuantityValue {
            value: "1.58750000".to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("m".to_string()),
            has_element_id: false,
            has_extensions: false,
        })]
    );
}

// ---------------------------------------------------------------------------
// path.boundary.009 -- dates, dateTimes and times
// ---------------------------------------------------------------------------

// covers: path.boundary.009
#[test]
fn the_boundary_functions_work_on_dates_datetimes_and_times() {
    expect_subject("LowBoundary", Subject::Temporal, 5);
    expect_subject("HighBoundary", Subject::Temporal, 5);

    // The expectations carry the `@` and `@T` prefixes `fpValue()` puts on a
    // temporal result, which the harness compares against and never strips.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateMonth"),
        "@2014-01"
    );
    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryDateMonth"),
        "@2014-12"
    );
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryTimeMillisecond"),
        "@T10:30:00.000"
    );
    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryTimeMillisecond"),
        "@T10:30:59.999"
    );

    // A year widened to month precision runs from January to December.
    assert_eq!(eval("@2014.lowBoundary(6)"), primitive("date", "2014-01"));
    assert_eq!(eval("@2014.highBoundary(6)"), primitive("date", "2014-12"));

    // A time widened to millisecond precision runs from .000 to 59.999 -- the
    // seconds as well as the milliseconds, because the input stated neither.
    assert_eq!(
        eval("@T10:30.lowBoundary(9)"),
        primitive("time", "10:30:00.000")
    );
    assert_eq!(
        eval("@T10:30.highBoundary(9)"),
        primitive("time", "10:30:59.999")
    );

    // A dateTime with no offset gets the extreme offsets, because an unstated
    // offset could be any of them: +14:00 gives the earliest instant and -12:00
    // the latest.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateTimeMillisecond1"),
        "@2014-01-01T08:00:00.000+14:00"
    );
    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryDateTimeMillisecond1"),
        "@2014-01-01T08:00:59.999-12:00"
    );
    assert_eq!(
        eval("@2014-01-01T08.lowBoundary(17)"),
        primitive("dateTime", "2014-01-01T08:00:00.000+14:00")
    );
    assert_eq!(
        eval("@2014-01-01T08.highBoundary(17)"),
        primitive("dateTime", "2014-01-01T08:00:59.999-12:00")
    );

    // A dateTime that does state an offset keeps it.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateTimeMillisecond2"),
        "@2014-01-01T08:05:00.000+08:00"
    );
    assert_eq!(
        eval("@2014-01-01T08:05+08:00.lowBoundary(17)"),
        primitive("dateTime", "2014-01-01T08:05:00.000+08:00")
    );

    // And narrowing rather than widening drops components instead of inventing
    // them.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateTimeMillisecond3"),
        "@2014-01-01"
    );
    assert_eq!(
        eval("@2014-01-01T08.lowBoundary(8)"),
        primitive("date", "2014-01-01")
    );
}

// ---------------------------------------------------------------------------
// path.boundary.010 -- an integer's boundaries
// ---------------------------------------------------------------------------

// covers: path.boundary.010
#[test]
fn an_integers_boundaries_are_decimals_half_a_unit_away() {
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDecimal9"),
        "0.50000"
    );
    assert_eq!(
        sole_output_body("HighBoundary", "HighBoundaryDecimal8"),
        "1.50000000"
    );
    expect_case("LowBoundary", "LowBoundaryDecimal9");
    expect_case("HighBoundary", "HighBoundaryDecimal8");

    // An integer is treated as having no fractional digits, so its boundaries
    // are half a unit either side -- and they come back as decimals at the
    // requested scale, not as integers.
    expect_decimal("1.lowBoundary(5)", "0.50000");
    expect_decimal("1.highBoundary()", "1.50000000");
    assert_eq!(
        eval("1.lowBoundary(5) is System.Decimal"),
        primitive("boolean", "true")
    );
    assert_eq!(
        eval("1.lowBoundary(5) is System.Integer"),
        primitive("boolean", "false")
    );

    // At scale 0 the half-unit step shows as a borrow and a carry.
    expect_decimal("1.lowBoundary(0)", "0");
    expect_decimal("1.highBoundary(0)", "2");
    expect_case("LowBoundary", "LowBoundaryDecimal8");
    expect_case("HighBoundary", "HighBoundaryDecimal9");

    // `1.toDecimal()` takes the same path, so the rule is about the value's
    // stated precision and not about the Integer type.
    expect_decimal("1.toDecimal().lowBoundary()", "0.50000000");
    expect_case("LowBoundary", "LowBoundaryDecimal7");
}
