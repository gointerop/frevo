//! Boundary functions: `path.boundary.001` through `path.boundary.019`.
//!
//! ```text
//! cargo test -p frevo-path --test path_boundary
//! ```
//!
//! The reference is what HAPI 8.12.0's pinned engine does, not the FHIRPath
//! specification's prose (`DEV-0025`). That engine builds a decimal boundary
//! by editing the number's text: rounding a `9` up gives `:`, a borrow keeps a
//! leading `0`, and only a lower-case `e` is read as an exponent, whose digits
//! are appended to the result. Every date or dateTime boundary is a
//! `dateTime`. HAPI fails 18 of the suite's own boundary cases because of it
//! (`oracles/fhirpath/hapi-8.12.0-results.json:954-962`, `:987-995`), and
//! Frevo is required to fail exactly those 18 with HAPI's values and errors
//! (parity under `path.oracle.012`) and pass the rest.
//!
//! Where the scenario names suite cases, each test drives them through the
//! committed harness and then states the key values at the engine seam as
//! well. The second half matters because the harness compares a quantity
//! result with `Quantity.equalsDeep`, which uses `BigDecimal.equals` and so is
//! **scale sensitive**: `1.5865` is not equal to `1.58650000`
//! (`tests/oracle/value.rs::decimal_equals`). Scenarios `.012` to `.019` are
//! differential-only -- no suite case grades them -- so they are asserted at
//! the seam alone.
//!
//! The cases in the two groups are partitioned by what they operate on rather
//! than listed by name, so a re-pin that adds a case to a class is visible as a
//! count mismatch instead of silently going ungraded.
//!
//! Provenance: `FHIRPathEngine.java` (R4) at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, `funcLowBoundary` (`:4290`),
//! `funcHighBoundary` (`:4334`) and `funcPrecision` (`:4378`), plus
//! `Utilities.lowBoundaryForDecimal` / `highBoundaryForDecimal`
//! (`Utilities.java:1631-1720`), `DateTimeUtil.lowBoundaryForDate` /
//! `highBoundaryForDate` / `applyDatePrecision` / `applyTimePrecision`
//! (`DateTimeUtil.java:65-154`) and `BaseDateTimeType` (`:330-342`,
//! `:484-492`, `:540-553`, `:796-798`).

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use frevo_path::CheckError;
use oracle::engine::{
    self as seam, CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine,
    engine_under_test,
};
use oracle::input::{self, Document, SuiteInput};
use oracle::json;
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

/// Assert the class still has the size this test was written against, that
/// every case named in `hapi_failed` is graded `failed` -- as HAPI 8.12.0's own
/// run grades it -- and that every other case in the class passed.
fn expect_subject(group: &str, subject: Subject, expected: usize, hapi_failed: &[&str]) {
    let (_, report) = pinned();
    let cases = cases_of(group, subject);
    assert_eq!(
        cases.len(),
        expected,
        "{group} must have {expected} {subject:?} case(s); a different count means the \
         suite was re-pinned without this test being updated"
    );
    for name in hapi_failed {
        assert!(
            cases.iter().any(|c| c.address.name == *name),
            "{group}/{name} must be a {subject:?} case of the pinned suite"
        );
    }

    let mut mismatches = Vec::new();
    let mut passed = 0;
    for case in &cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        let required = if hapi_failed.contains(&case.address.name.as_str()) {
            CaseStatus::Failed
        } else {
            CaseStatus::Passed
        };
        if r.status == CaseStatus::Passed {
            passed += 1;
        }
        if r.status != required {
            mismatches.push(format!(
                "{} ({:?}): {} where HAPI 8.12.0 is {} -- {}",
                case.address,
                case.expression,
                r.status.as_str(),
                required.as_str(),
                r.reason
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} {subject:?} cases in {group} are not graded as HAPI 8.12.0 grades them; the \
         first few:\n  {}",
        mismatches.len(),
        cases.len(),
        mismatches[..mismatches.len().min(5)].join("\n  ")
    );
    assert_eq!(
        passed,
        expected - hapi_failed.len(),
        "{group}: {} {subject:?} cases pass on HAPI 8.12.0",
        expected - hapi_failed.len()
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

/// A case HAPI 8.12.0 fails, which Frevo must fail too and for HAPI's reason:
/// the run's reason must contain `fragment` -- the error HAPI raised, or the
/// value HAPI produced -- so a failure for some other cause is not parity.
fn expect_hapi_failure(group: &str, name: &str, fragment: &str) {
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
    assert!(
        r.reason.contains(fragment),
        "{}: {:?} must fail as HAPI 8.12.0 fails it ({fragment:?}), but failed with: {}",
        r.address,
        r.expression,
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

/// Parse, optionally check, and evaluate. The check, when run, must pass: a
/// rejection there is not the evaluation outcome a test asks about.
/// `static_check: false` is how `FhirPathR4` evaluates (`path.check.013`).
fn run_expr(
    expr: &str,
    inp: Option<&SuiteInput>,
    static_check: bool,
) -> Result<Vec<ResultItem>, EngineError> {
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
    if static_check {
        engine
            .check(parsed, &context, StrictnessFlags::CHECK)
            .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    }
    engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
}

fn eval_over(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    run_expr(expr, inp, true).unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

fn eval(expr: &str) -> Vec<ResultItem> {
    eval_over(expr, None)
}

/// Evaluate with the static check skipped.
fn eval_unchecked_over(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    run_expr(expr, inp, false)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate with the check skipped: {e}"))
}

fn eval_unchecked(expr: &str) -> Vec<ResultItem> {
    eval_unchecked_over(expr, None)
}

/// The message of the execution error `expr` must raise. A result, or a
/// feature the engine does not implement, is a failure of the test.
fn execution_error_of(expr: &str, inp: Option<&SuiteInput>, static_check: bool) -> String {
    match run_expr(expr, inp, static_check) {
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(other) => panic!("{expr:?} must be an execution error, but did not run: {other}"),
        Ok(items) => panic!("{expr:?} must be an execution error, but yielded {items:?}"),
    }
}

/// Assert `expr` is an execution error whose message is exactly `message`
/// and whose offset is exactly `offset`: the zero-based byte offset of the
/// failing node's first token, present exactly where HAPI's message carries a
/// position suffix (path.errors.002).
fn expect_error_at(
    expr: &str,
    inp: Option<&SuiteInput>,
    static_check: bool,
    message: &str,
    offset: Option<usize>,
) {
    match run_expr(expr, inp, static_check) {
        Err(EngineError::Diagnostic {
            message: got,
            offset: got_offset,
        }) => {
            assert_eq!(got, message, "{expr} must raise exactly this error");
            assert_eq!(
                got_offset, offset,
                "{expr} must report the failing node's zero-based byte offset"
            );
        }
        Err(other) => panic!("{expr:?} must be an execution error, but did not run: {other}"),
        Ok(items) => panic!("{expr:?} must be an execution error, but yielded {items:?}"),
    }
}

/// Assert `expr` is an execution error whose message is exactly `message`.
fn expect_error(expr: &str, inp: Option<&SuiteInput>, static_check: bool, message: &str) {
    assert_eq!(
        execution_error_of(expr, inp, static_check),
        message,
        "{expr} must raise exactly this error"
    );
}

/// Assert `expr` is an execution error whose message contains `fragment`.
fn expect_error_naming(expr: &str, inp: Option<&SuiteInput>, fragment: &str) {
    let message = execution_error_of(expr, inp, true);
    assert!(
        message.contains(fragment),
        "{expr} must raise an error naming {fragment:?}, but raised {message:?}"
    );
}

/// The static check's verdict on a literal expression with no input: the
/// message id of the rejection, or `None` when it passes.
fn check_rejection(expr: &str) -> Option<&'static str> {
    let parsed = frevo_path::Expression::parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {}", e.message()));
    match seam::check_expression(&parsed, &CaseContext::default(), &EngineConfig::default()) {
        Err(e) => panic!("{expr:?} could not be checked: {e}"),
        Ok(Ok(_)) => None,
        Ok(Err(CheckError::Failed { id, .. })) => Some(id),
        Ok(Err(e @ CheckError::NotImplemented { .. })) => {
            panic!("{expr:?} could not be checked: {e}")
        }
    }
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

fn date_time(value: &str) -> Vec<ResultItem> {
    primitive("dateTime", value)
}

fn time(value: &str) -> Vec<ResultItem> {
    primitive("time", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

/// A UCUM quantity literal's result: no `unit`, the code, and the UCUM system.
/// A valueless quantity has the empty lexical value, as the harness renders a
/// Quantity element with no `value` (`tests/oracle/engine.rs::result_item`).
fn ucum_quantity(value: &str, code: &str) -> Vec<ResultItem> {
    vec![ResultItem::Quantity(QuantityValue {
        value: value.to_string(),
        comparator: None,
        unit: None,
        system: Some(UCUM_SYSTEM.to_string()),
        code: Some(code.to_string()),
        has_element_id: false,
        has_extensions: false,
    })]
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

/// `new BigDecimal(...)`'s message for a text holding `c`, which HAPI raises
/// when the edited boundary text is not a decimal.
fn not_a_decimal(c: char) -> String {
    format!(
        "Character {c} is neither a decimal digit number, decimal point, nor \"e\" notation \
         exponential mark."
    )
}

/// The `FHIRPathEngine` error prefix `makeException` puts on its messages.
const PREFIX: &str = "Error evaluating FHIRPath expression: ";

// ---------------------------------------------------------------------------
// path.boundary.001 -- `lowBoundary` on a decimal
// ---------------------------------------------------------------------------

/// The nine `LowBoundary` decimal cases HAPI 8.12.0 fails
/// (`hapi-8.12.0-results.json:954-962`).
const LOW_HAPI_FAILED: [&str; 9] = [
    "LowBoundaryDecimalCarry1",
    "LowBoundaryDecimalCarry2",
    "LowBoundaryDecimalCarry3",
    "LowBoundaryDecimalBorrow1",
    "LowBoundaryDecimalBorrow2",
    "LowBoundaryDecimalExponent1",
    "LowBoundaryDecimalExponent2",
    "LowBoundaryDecimalExponent3",
    "LowBoundaryDecimalExponent4",
];

/// The nine `HighBoundary` decimal cases HAPI 8.12.0 fails
/// (`hapi-8.12.0-results.json:987-995`).
const HIGH_HAPI_FAILED: [&str; 9] = [
    "HighBoundaryDecimalCarry1",
    "HighBoundaryDecimalCarry2",
    "HighBoundaryDecimalCarry3",
    "HighBoundaryDecimalCarry4",
    "HighBoundaryDecimalExponent1",
    "HighBoundaryDecimalExponent2",
    "HighBoundaryDecimalExponent3",
    "HighBoundaryDecimalExponent4",
    "HighBoundaryDecimalExponent5",
];

// covers: path.boundary.001
#[test]
fn low_boundary_on_a_decimal_widens_to_the_implied_precision() {
    // 31 decimal cases: 22 pass on HAPI and the nine of `.004`/`.005` fail.
    expect_subject("LowBoundary", Subject::Decimal, 31, &LOW_HAPI_FAILED);

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

    // The range is 0 to 16 inclusive (`precision >= 0 && precision < 17`), for
    // a decimal and for an integer, and for both functions.
    assert_eq!(eval("1.587.lowBoundary(16)"), decimal("1.5865000000000000"));
    assert_eq!(eval("1.587.lowBoundary(17)"), empty());
    assert_eq!(eval("1.587.highBoundary(17)"), empty());
    assert_eq!(eval("1.lowBoundary(17)"), empty());
    assert_eq!(eval("1.highBoundary(-1)"), empty());

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
// path.boundary.004 -- no carry, no borrow normalisation
// ---------------------------------------------------------------------------

// covers: path.boundary.004
#[test]
fn low_boundary_neither_carries_nor_normalises_a_borrow() {
    // The suite's expectations are the specification's (`DEV-0025`); HAPI
    // fails all five, and Frevo must fail them the same way.
    for (name, expected) in [
        ("LowBoundaryDecimalCarry1", "-10.0"),
        ("LowBoundaryDecimalCarry2", "-100.0"),
        ("LowBoundaryDecimalCarry3", "-10"),
        ("LowBoundaryDecimalBorrow1", "9"),
        ("LowBoundaryDecimalBorrow2", "99.50"),
    ] {
        assert_eq!(
            sole_output_body("LowBoundary", name),
            expected,
            "LowBoundary/{name}"
        );
    }
    let colon = not_a_decimal(':');
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalCarry1", &colon);
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalCarry2", &colon);
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalCarry3", &colon);
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalBorrow1", "\"09\"");
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalBorrow2", "\"099.50\"");

    // Rounding the magnitude's `9` up gives `:`, which is not a decimal: the
    // `BigDecimal` message, in full.
    expect_error("(-9.96).lowBoundary(1)", None, true, &colon);
    expect_error("(-99.96).lowBoundary(1)", None, true, &colon);
    expect_error("(-9).lowBoundary(0)", None, true, &colon);

    // A borrow turns trailing `0`s into `9`s and keeps the leading `0`, and
    // the decimal keeps that lexical form.
    expect_decimal("10.lowBoundary(0)", "09");
    expect_decimal("100.lowBoundary(2)", "099.50");

    // With no leading `0` left behind the result is exact.
    expect_decimal("1.587.lowBoundary(2)", "1.58");
    expect_decimal("1.lowBoundary(0)", "0");
}

// ---------------------------------------------------------------------------
// path.boundary.005 -- `lowBoundary` on an exponent-form decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.005
#[test]
fn low_boundary_on_an_exponent_form_decimal_appends_the_exponents_digits() {
    for (name, expected) in [
        ("LowBoundaryDecimalExponent1", "945"),
        ("LowBoundaryDecimalExponent2", "0.00145000"),
        ("LowBoundaryDecimalExponent3", "-955"),
        ("LowBoundaryDecimalExponent4", "145.0"),
    ] {
        assert_eq!(
            sole_output_body("LowBoundary", name),
            expected,
            "LowBoundary/{name}"
        );
    }
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalExponent1", "\"92\"");
    expect_hapi_failure(
        "LowBoundary",
        "LowBoundaryDecimalExponent2",
        &not_a_decimal('-'),
    );
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalExponent3", "Character :");
    expect_hapi_failure("LowBoundary", "LowBoundaryDecimalExponent4", "\"1.5\"");

    // The input's decimals are `9.5e2`, `1.5e-3`, `-9.5e2` and `1.5E2`. The
    // boundary is computed on the mantissa before the first lower-case `e`,
    // and the exponent's text is appended with no `e`.
    let decimals = load_input("parameters-example-decimals.xml");
    // `9.5` at scale 0 is `9`, then `2`.
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value.lowBoundary(0)",
            Some(&decimals)
        ),
        decimal("92")
    );
    // `1.45000000` then `-3` is not a decimal.
    expect_error(
        "Parameters.parameter[1].value.lowBoundary()",
        Some(&decimals),
        true,
        &not_a_decimal('-'),
    );
    // The magnitude `9.5` rounds its `9` up to `:`.
    expect_error_naming(
        "Parameters.parameter[2].value.lowBoundary(0)",
        Some(&decimals),
        "Character :",
    );
    // An upper-case `E` is an ordinary character: `1.5E2` at one fractional
    // digit is cut to `1.5`.
    assert_eq!(
        eval_over(
            "Parameters.parameter[3].value.lowBoundary(1)",
            Some(&decimals)
        ),
        decimal("1.5")
    );
}

// ---------------------------------------------------------------------------
// path.boundary.006 -- `highBoundary` on a decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.006
#[test]
fn high_boundary_on_a_decimal_widens_to_the_implied_precision() {
    // 27 decimal cases: 18 pass on HAPI and the nine of `.007`/`.011` fail.
    expect_subject("HighBoundary", Subject::Decimal, 27, &HIGH_HAPI_FAILED);

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
// path.boundary.007 -- `highBoundary` does not carry
// ---------------------------------------------------------------------------

// covers: path.boundary.007
#[test]
fn high_boundary_does_not_carry_past_a_nine() {
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
        expect_hapi_failure("HighBoundary", name, &not_a_decimal(':'));
    }

    // The digit rounded up is a `9`, which becomes `:`.
    let colon = not_a_decimal(':');
    expect_error("9.95.highBoundary(1)", None, true, &colon);
    expect_error("0.996.highBoundary(1)", None, true, &colon);
    expect_error("99.96.highBoundary(1)", None, true, &colon);
    expect_error("9.highBoundary(0)", None, true, &colon);

    // A round-up of a digit below `9` is exact.
    expect_decimal("1.587.highBoundary(2)", "1.59");
    expect_decimal("1.highBoundary(0)", "2");
}

// ---------------------------------------------------------------------------
// path.boundary.008 -- quantities
// ---------------------------------------------------------------------------

// covers: path.boundary.008
#[test]
fn the_boundary_functions_work_on_quantities() {
    expect_subject("LowBoundary", Subject::Quantity, 1, &[]);
    expect_subject("HighBoundary", Subject::Quantity, 1, &[]);

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
        ucum_quantity("1.58650000", "cm")
    );
    assert_eq!(
        eval("1.587 'm'.highBoundary(8)"),
        ucum_quantity("1.58750000", "m")
    );
}

// ---------------------------------------------------------------------------
// path.boundary.009 -- dates, dateTimes and times
// ---------------------------------------------------------------------------

// covers: path.boundary.009
#[test]
fn the_boundary_functions_on_a_date_or_datetime_yield_a_datetime() {
    expect_subject("LowBoundary", Subject::Temporal, 5, &[]);
    expect_subject("HighBoundary", Subject::Temporal, 5, &[]);

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

    // A year widened to month precision runs from January to December, and
    // the result is a `dateTime` even though the input is a `date`
    // (`new DateTimeType(...)`, `FHIRPathEngine.java:4318`, `:4362`).
    assert_eq!(eval("@2014.lowBoundary(6)"), date_time("2014-01"));
    assert_eq!(eval("@2014.highBoundary(6)"), date_time("2014-12"));
    // A date's default precision is 10, the day.
    assert_eq!(eval("@2014.lowBoundary()"), date_time("2014-01-01"));

    // A time widened to millisecond precision runs from .000 to 59.999 -- the
    // seconds as well as the milliseconds, because the input stated neither.
    assert_eq!(eval("@T10:30.lowBoundary(9)"), time("10:30:00.000"));
    assert_eq!(eval("@T10:30.highBoundary(9)"), time("10:30:59.999"));

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
        date_time("2014-01-01T08:00:00.000+14:00")
    );
    assert_eq!(
        eval("@2014-01-01T08.highBoundary(17)"),
        date_time("2014-01-01T08:00:59.999-12:00")
    );

    // A dateTime that does state an offset keeps it.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateTimeMillisecond2"),
        "@2014-01-01T08:05:00.000+08:00"
    );
    assert_eq!(
        eval("@2014-01-01T08:05+08:00.lowBoundary(17)"),
        date_time("2014-01-01T08:05:00.000+08:00")
    );

    // And narrowing rather than widening drops components, the offset with
    // them, and is still a `dateTime`.
    assert_eq!(
        sole_output_body("LowBoundary", "LowBoundaryDateTimeMillisecond3"),
        "@2014-01-01"
    );
    assert_eq!(
        eval("@2014-01-01T08.lowBoundary(8)"),
        date_time("2014-01-01")
    );

    // The high boundary of a month at day precision is the month's last day,
    // leap years included.
    assert_eq!(eval("@2014-02.highBoundary(8)"), date_time("2014-02-28"));
    assert_eq!(eval("@2016-02.highBoundary(8)"), date_time("2016-02-29"));
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

    // At scale 0 the half-unit step shows as a borrow and a round-up.
    expect_decimal("1.lowBoundary(0)", "0");
    expect_decimal("1.highBoundary(0)", "2");
    expect_case("LowBoundary", "LowBoundaryDecimal8");
    expect_case("HighBoundary", "HighBoundaryDecimal9");

    // `1.toDecimal()` takes the same path, so the rule is about the value's
    // stated precision and not about the Integer type.
    expect_decimal("1.toDecimal().lowBoundary()", "0.50000000");
    expect_case("LowBoundary", "LowBoundaryDecimal7");
}

// ---------------------------------------------------------------------------
// path.boundary.011 -- `highBoundary` on an exponent-form decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.011
#[test]
fn high_boundary_on_an_exponent_form_decimal_appends_the_exponents_digits() {
    for (name, expected) in [
        ("HighBoundaryDecimalExponent1", "955"),
        ("HighBoundaryDecimalExponent2", "955.00000000"),
        ("HighBoundaryDecimalExponent3", "0.00155000"),
        ("HighBoundaryDecimalExponent4", "-945"),
        ("HighBoundaryDecimalExponent5", "155.0"),
    ] {
        assert_eq!(
            sole_output_body("HighBoundary", name),
            expected,
            "HighBoundary/{name}"
        );
    }
    expect_hapi_failure(
        "HighBoundary",
        "HighBoundaryDecimalExponent1",
        "Character :",
    );
    expect_hapi_failure(
        "HighBoundary",
        "HighBoundaryDecimalExponent2",
        "\"9.550000002\"",
    );
    expect_hapi_failure(
        "HighBoundary",
        "HighBoundaryDecimalExponent3",
        &not_a_decimal('-'),
    );
    expect_hapi_failure("HighBoundary", "HighBoundaryDecimalExponent4", "\"-92\"");
    expect_hapi_failure("HighBoundary", "HighBoundaryDecimalExponent5", "\"1.6\"");

    let decimals = load_input("parameters-example-decimals.xml");
    // `9.5` at scale 0 rounds its `9` up to `:`.
    expect_error_naming(
        "Parameters.parameter[0].value.highBoundary(0)",
        Some(&decimals),
        "Character :",
    );
    // `9.55000000` then `2`.
    assert_eq!(
        eval_over(
            "Parameters.parameter[0].value.highBoundary()",
            Some(&decimals)
        ),
        decimal("9.550000002")
    );
    // `1.55000000` then `-3` is not a decimal.
    expect_error(
        "Parameters.parameter[1].value.highBoundary()",
        Some(&decimals),
        true,
        &not_a_decimal('-'),
    );
    // The high boundary of `-9.5` is toward zero: `-9`, then `2`.
    assert_eq!(
        eval_over(
            "Parameters.parameter[2].value.highBoundary(0)",
            Some(&decimals)
        ),
        decimal("-92")
    );
    // `1.5E2` is not split, so the `5` is rounded up to `1.6`.
    assert_eq!(
        eval_over(
            "Parameters.parameter[3].value.highBoundary(1)",
            Some(&decimals)
        ),
        decimal("1.6")
    );
}

// ---------------------------------------------------------------------------
// path.boundary.012 -- a quantity's precision is not range-checked
// ---------------------------------------------------------------------------

// covers: path.boundary.012
#[test]
fn a_quantitys_boundary_precision_is_not_range_checked() {
    // Precision -1 cuts the text to "", which `new DecimalType("")` reads as
    // a null value: a single Quantity with no value, not an empty collection
    // and not an error.
    for expr in ["1.587 'cm'.lowBoundary(-1)", "1.587 'cm'.highBoundary(-1)"] {
        assert_eq!(
            eval(expr),
            ucum_quantity("", "cm"),
            "{expr} must be a single valueless Quantity with the code and system copied"
        );
        assert_eq!(
            eval(&format!("{expr}.count()")),
            primitive("integer", "1"),
            "{expr} is one item, not an empty collection"
        );
    }

    // A negative precision truncates the text rather than rounding at the tens.
    assert_eq!(eval("12.5 'cm'.lowBoundary(-1)"), ucum_quantity("1", "cm"));
    assert_eq!(eval("12.5 'cm'.highBoundary(-1)"), ucum_quantity("1", "cm"));

    // Precision -2 makes the cut index -1, and `charAt(-1)` raises. The JDK's
    // message text is not compared; that it is an execution error is.
    for expr in ["1.587 'cm'.lowBoundary(-2)", "1.587 'cm'.highBoundary(-2)"] {
        let _ = execution_error_of(expr, None, true);
    }

    // Past 16, where a decimal is empty, a quantity is computed regardless.
    assert_eq!(
        eval("1.587 'cm'.lowBoundary(20)"),
        ucum_quantity("1.58650000000000000000", "cm")
    );
    assert_eq!(eval("1.587.lowBoundary(20)"), empty());
}

// ---------------------------------------------------------------------------
// path.boundary.013 -- `precision()` on an empty or many-item focus
// ---------------------------------------------------------------------------

// covers: path.boundary.013
#[test]
fn precision_on_an_empty_or_many_item_focus_raises_naming_high_boundary() {
    // `funcPrecision` names `highBoundary` in its singleton error. The holder
    // is the `precision` function node, whose start is set, so
    // `makeExceptionPlural` appends its column (path.errors.002's rule): `{}`
    // and `.` are a token each, so `precision` is at column 4 (byte offset 3);
    // in `(1.5 | 2.5).precision()` the two spaces count 2 each, so it is at
    // column 15 (byte offset 12).
    expect_error_at(
        "{}.precision()",
        None,
        true,
        &format!(
            "{PREFIX}focus for highBoundary can only have one value, but has 0 values (@char 4)"
        ),
        Some(3),
    );
    expect_error_at(
        "(1.5 | 2.5).precision()",
        None,
        true,
        &format!(
            "{PREFIX}focus for highBoundary can only have one value, but has 2 values (@char 15)"
        ),
        Some(12),
    );

    // The boundaries are empty on an empty focus, not errors.
    assert_eq!(eval("{}.lowBoundary()"), empty());
    assert_eq!(eval("{}.highBoundary()"), empty());

    // And raise on a many-item focus, each naming itself.
    expect_error_at(
        "(1.5 | 2.5).lowBoundary()",
        None,
        true,
        &format!(
            "{PREFIX}focus for lowBoundary can only have one value, but has 2 values (@char 15)"
        ),
        Some(12),
    );
    expect_error_at(
        "(1.5 | 2.5).highBoundary()",
        None,
        true,
        &format!(
            "{PREFIX}focus for highBoundary can only have one value, but has 2 values (@char 15)"
        ),
        Some(12),
    );
}

// ---------------------------------------------------------------------------
// path.boundary.014 -- a focus of a type the functions do not take
// ---------------------------------------------------------------------------

// covers: path.boundary.014
#[test]
fn a_focus_of_a_type_the_functions_do_not_take_is_empty_not_an_error() {
    // The last arm builds an exception and does not throw it.
    for expr in [
        "1.precision()",
        "'abc'.precision()",
        "'abc'.lowBoundary()",
        "true.highBoundary()",
    ] {
        assert_eq!(
            eval_unchecked(expr),
            empty(),
            "{expr}, with the static check skipped, must be empty"
        );
    }

    // A Quantity focus passes the check but `funcPrecision` does not take it.
    assert_eq!(check_rejection("1 'cm'.precision()"), None);
    assert_eq!(eval("1 'cm'.precision()"), empty());

    // The check is stricter: an integer is admitted for the boundaries but
    // not for `precision`, and a string for neither.
    assert_eq!(
        check_rejection("1.precision()"),
        Some("FHIRPATH_CONTINUOUS_ONLY")
    );
    assert_eq!(
        check_rejection("'abc'.lowBoundary()"),
        Some("FHIRPATH_CONTINUOUS_ONLY")
    );
    assert_eq!(check_rejection("1.lowBoundary()"), None);

    // Dispatch is by the exact `fhirType()`: a `positiveInt` is not an
    // integer, a `code` is not a string the functions take, and an `instant`
    // is not a dateTime.
    let patient = load_input("patient-example.xml");
    let glossy = load_input("patient-glossy-example.xml");
    for function in ["lowBoundary()", "highBoundary()", "precision()"] {
        for (path, inp) in [
            ("Patient.telecom.rank.first()", &patient),
            ("Patient.gender", &patient),
            ("Patient.meta.lastUpdated", &glossy),
        ] {
            let expr = format!("{path}.{function}");
            assert_eq!(
                eval_unchecked_over(&expr, Some(inp)),
                empty(),
                "{expr} over {}, with the static check skipped, must be empty",
                inp.file
            );
        }
    }
}

// ---------------------------------------------------------------------------
// path.boundary.015 -- a date or dateTime precision is one of seven values
// ---------------------------------------------------------------------------

// covers: path.boundary.015
#[test]
fn a_date_or_datetime_boundary_precision_is_one_of_seven_values() {
    assert_eq!(eval("@2014-01-15.lowBoundary(7)"), date_time("2014-01"));
    assert_eq!(eval("@2014.lowBoundary(10)"), date_time("2014-01-01"));

    // Anything else raises; there is no range check returning empty.
    for (expr, n) in [
        ("@2014.lowBoundary(5)", 5),
        ("@2014.highBoundary(0)", 0),
        ("@2014-01-01T08.lowBoundary(12)", 12),
        ("@2014.lowBoundary(20)", 20),
    ] {
        expect_error(
            expr,
            None,
            true,
            &format!("Unsupported Date precision for boundary operation: {n}"),
        );
    }

    // Precision 14 keeps 17 characters and appends the offset, and the two
    // characters after the minute's `:` are read as the seconds.
    assert_eq!(
        eval("@2014-01-01T08.lowBoundary(14)"),
        date_time("2014-01-01T08:00:+14:00"),
        "the seconds text `+1` parses as 1"
    );
    expect_error(
        "@2014-01-01T08.highBoundary(14)",
        None,
        true,
        "Invalid date/time format: \"2014-01-01T08:00:-12:00\"",
    );
    assert_eq!(
        eval("@2014-01-01T08:05+08:00.lowBoundary(14)"),
        date_time("2014-01-01T08:05:+08:00")
    );
    assert_eq!(
        eval("@2014-01-01T08:05-05:00.highBoundary(14)"),
        date_time("2014-01-01T08:05:-05:00"),
        "the seconds text `-0` parses as 0"
    );
    expect_error(
        "@2014-01-01T08:05-10:00.highBoundary(14)",
        None,
        true,
        "Invalid date/time format: \"2014-01-01T08:05:-10:00\"",
    );
    expect_error(
        "@2014-01-01T08:05Z.lowBoundary(14)",
        None,
        true,
        "Invalid date/time format: \"2014-01-01T08:05:Z\"",
    );
}

// ---------------------------------------------------------------------------
// path.boundary.016 -- a time precision below 9 keeps a trailing separator
// ---------------------------------------------------------------------------

// covers: path.boundary.016
#[test]
fn a_time_boundary_precision_below_nine_keeps_a_trailing_separator() {
    assert_eq!(eval("@T10:30.lowBoundary(2)"), time("10:"));
    assert_eq!(eval("@T10:30.lowBoundary(4)"), time("10:30:"));
    assert_eq!(eval("@T10:30.lowBoundary(6)"), time("10:30:00."));
    assert_eq!(eval("@T10:30.highBoundary(2)"), time("10:"));
    assert_eq!(eval("@T10:30.highBoundary(6)"), time("10:30:59."));

    for n in [3, 8, 17] {
        expect_error(
            &format!("@T10:30.lowBoundary({n})"),
            None,
            true,
            &format!("Unsupported Time precision for boundary operation: {n}"),
        );
    }
}

// ---------------------------------------------------------------------------
// path.boundary.017 -- the precision argument
// ---------------------------------------------------------------------------

// covers: path.boundary.017
#[test]
fn the_precision_argument_must_be_a_single_value_read_as_an_integer() {
    // Both functions name `lowBoundary` in this error. The holder is the
    // function node: `1.587` is one token, so the function name is at column 7
    // (byte offset 6), under path.errors.002's rule.
    let multiple = format!(
        "{PREFIX}The parameter type Multiple Values is not legal for lowBoundary parameter 0. \
         expecting integer (@char 7)"
    );
    for expr in [
        "1.587.lowBoundary(1 | 2)",
        "1.587.highBoundary(1 | 2)",
        "1.587.lowBoundary({})",
    ] {
        expect_error_at(expr, None, false, &multiple, Some(6));
    }

    // The item's primitive text is read with `Integer.parseInt`.
    assert_eq!(eval_unchecked("1.587.lowBoundary('2')"), decimal("1.58"));
    expect_error(
        "1.587.lowBoundary(1.5)",
        None,
        false,
        "For input string: \"1.5\"",
    );
}

// ---------------------------------------------------------------------------
// path.boundary.018 -- a zero decimal
// ---------------------------------------------------------------------------

// covers: path.boundary.018
#[test]
fn a_zero_decimal_takes_a_fixed_half_unit_text() {
    expect_decimal("0.lowBoundary()", "-0.50000000");
    expect_decimal("0.highBoundary()", "0.50000000");
    expect_decimal("0.lowBoundary(0)", "-0");
    // The `5` rounds the leading `0` up.
    expect_decimal("0.highBoundary(0)", "1");

    // `isZero` takes every spelling of zero alike.
    expect_decimal("0.0.lowBoundary(1)", "-0.5");
    expect_decimal("(-0.0).lowBoundary(1)", "-0.5");
    expect_decimal("0.000.highBoundary(1)", "0.5");
}

// ---------------------------------------------------------------------------
// path.boundary.019 -- a quantity with no value
// ---------------------------------------------------------------------------

/// An Observation whose `valueQuantity` has a unit, system and code but no
/// `value`. No suite fixture has one.
fn valueless_quantity_observation() -> SuiteInput {
    let body = r#"{"resourceType":"Observation","status":"final","code":{"text":"length"},"valueQuantity":{"unit":"cm","system":"http://unitsofmeasure.org","code":"cm"}}"#;
    let document = Document::Json(
        json::parse(body).unwrap_or_else(|e| panic!("the hand-built Observation is JSON: {e}")),
    );
    SuiteInput {
        file: "hand-built-observation-valueless-quantity.json".to_string(),
        bytes: body.as_bytes().to_vec(),
        document,
    }
}

// covers: path.boundary.019
#[test]
fn a_quantity_with_no_value_has_no_boundary() {
    let obs = valueless_quantity_observation();

    // The input is what it says: one Quantity, with no value.
    assert_eq!(
        eval_over("Observation.value.count()", Some(&obs)),
        primitive("integer", "1")
    );
    assert_eq!(
        eval_over("Observation.value.value.exists()", Some(&obs)),
        primitive("boolean", "false")
    );

    // The text is tested before the precision is used, so any argument raises.
    for arg in ["", "0", "8", "-1", "40"] {
        expect_error(
            &format!("Observation.value.lowBoundary({arg})"),
            Some(&obs),
            true,
            "Unable to calculate lowBoundary for a null decimal string",
        );
        expect_error(
            &format!("Observation.value.highBoundary({arg})"),
            Some(&obs),
            true,
            "Unable to calculate highBoundary for a null decimal string",
        );
    }
}
