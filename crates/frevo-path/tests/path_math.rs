//! Arithmetic and math functions: `path.math.001`-`014` and
//! `path.mathfn.001`-`011`.
//!
//! ```text
//! cargo test -p frevo-path --test path_math
//! ```
//!
//! Sixteen of the twenty-five scenarios name a suite group, so they are driven
//! through the committed harness. The nine that do not are the ones the suite
//! cannot reach: integer overflow, the shape of date arithmetic, and the
//! focus-cardinality rules of the math functions.
//!
//! Two notes for review are recorded inline, each with the upstream line that
//! establishes it:
//!
//! - `path.math.014` -- subtracting two dateTimes is an error in the pinned
//!   engine, not a quantity. The scenario defers to the engine, so the test
//!   asserts the error.
//! - `path.mathfn.010` -- the scenario says `{}.abs()`, `{}.sqrt()` and
//!   `{}.round(2)` each yield an empty collection. The pinned engine raises for
//!   all three; `exp` is the only one of the ten math functions with an
//!   empty-focus guard. The test is written to the scenario and the conflict is
//!   flagged, per the instruction not to implement something else silently.
//!
//! Provenance: `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::run::{self, CaseReport, CaseStatus, RunReport};
use oracle::suite::{Stage, Suite};
use oracle::value::ResultItem;

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

fn expect_group(group: &str, expected_cases: usize) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    assert_eq!(
        g.cases.len(),
        expected_cases,
        "the pinned {group} group is {expected_cases} cases; a different count means it \
         was re-pinned without this test being updated"
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

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is evaluated: {e}"));
    engine
}

fn eval(expr: &str) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    engine
        .evaluate(parsed, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

/// The stage at which the engine reported a diagnostic for `expr`, with its
/// message. `NotImplemented` panics, so a missing engine is never read as the
/// failure the scenario asked for.
fn expect_failure(expr: &str) -> (Stage, String) {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = match engine.parse(expr) {
        Err(EngineError::Diagnostic { message, .. }) => return (Stage::Parse, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at parse: {e}"),
        Ok(p) => p,
    };
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Err(EngineError::Diagnostic { message, .. }) => return (Stage::Check, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at check: {e}"),
        Ok(()) => {}
    }
    match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
        Err(EngineError::Diagnostic { message, .. }) => (Stage::Evaluate, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at evaluate: {e}"),
        Ok(items) => panic!("{expr:?} must fail, but it evaluated to {items:?}"),
    }
}

/// `expr` must fail at the evaluate stage specifically.
fn expect_evaluation_error(expr: &str) -> String {
    let (stage, message) = expect_failure(expr);
    assert_eq!(
        stage,
        Stage::Evaluate,
        "{expr:?} must fail when evaluated, not at the {} stage: {message}",
        stage.as_str()
    );
    message
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

fn date(value: &str) -> Vec<ResultItem> {
    primitive("date", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// path.math.001 -- addition
// ---------------------------------------------------------------------------

// covers: path.math.001
#[test]
fn addition_works_on_integers_decimals_strings_and_quantities() {
    expect_group("testPlus", 29);

    // The four arms of `opPlus` (FHIRPathEngine.java:2698-2709), each named by
    // the scenario, with the result type each produces.
    assert_eq!(eval("1 + 1"), integer("2"));
    assert_eq!(eval("1.5 + 1.5"), primitive("decimal", "3.0"));
    assert_eq!(
        eval("1 + 1.5"),
        primitive("decimal", "2.5"),
        "a mixed pair takes the decimal arm"
    );
    assert_eq!(eval("'a' + 'b'"), primitive("string", "ab"));
}

// ---------------------------------------------------------------------------
// path.math.002 -- concatenation
// ---------------------------------------------------------------------------

// covers: path.math.002
#[test]
fn the_concatenation_operator_treats_an_empty_operand_as_the_empty_string() {
    expect_group("testConcatenate", 4);

    // The difference from `+`: `&` substitutes the empty string for an empty
    // operand instead of propagating the empty collection.
    assert_eq!(eval("'a' & 'b'"), primitive("string", "ab"));
    assert_eq!(eval("'1' & {}"), primitive("string", "1"));
    assert_eq!(eval("{} & 'b'"), primitive("string", "b"));
    assert_eq!(eval("{} & {}"), primitive("string", ""));

    // Whereas `+` with an empty operand is empty (`path.math.009`), which is
    // what makes the pair worth stating.
    assert_eq!(eval("'1' + {}"), empty());
}

// ---------------------------------------------------------------------------
// path.math.003 -- subtraction
// ---------------------------------------------------------------------------

// covers: path.math.003
#[test]
fn subtraction_works_on_integers_decimals_and_quantities() {
    expect_group("testMinus", 7);

    assert_eq!(eval("1 - 1"), integer("0"));
    assert_eq!(eval("1.8 - 1.2"), primitive("decimal", "0.6"));

    // The group's `'a'-'b'` case is `invalid="execution"`: unlike `+`, `-` has
    // no string arm.
    let r = expect_case("testMinus", "testMinus4");
    assert_eq!(r.failed_stage, Some(Stage::Evaluate));
    expect_evaluation_error("'a' - 'b'");
}

// ---------------------------------------------------------------------------
// path.math.004 -- multiplication
// ---------------------------------------------------------------------------

// covers: path.math.004
#[test]
fn multiplication_works_on_integers_decimals_and_quantities() {
    expect_group("testMultiply", 3);

    assert_eq!(eval("1 * 1"), integer("1"));
    assert_eq!(
        eval("1.2 * 1.8"),
        primitive("decimal", "2.16"),
        "exact decimal multiplication, so the scale is the sum of the scales"
    );
}

// ---------------------------------------------------------------------------
// path.math.005 -- division always yields a decimal
// ---------------------------------------------------------------------------

// covers: path.math.005
#[test]
fn division_always_yields_a_decimal() {
    expect_group("testDivide", 7);

    // `opDivideBy`'s numeric arm produces a DecimalType whatever the operand
    // types (FHIRPathEngine.java:3012-3016), so dividing two integers that
    // divide exactly is still a decimal and not an integer.
    assert_eq!(eval("4 / 2 = 2"), boolean(true));
    assert_eq!(eval("1 / 2 = 0.5"), boolean(true));
    assert_eq!(
        eval("(4 / 2) is System.Decimal"),
        boolean(true),
        "two integers that divide exactly still give a decimal"
    );
    assert_eq!(eval("(4 / 2) is System.Integer"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.math.006 -- `div`
// ---------------------------------------------------------------------------

// covers: path.math.006
#[test]
fn div_truncates_toward_zero() {
    expect_group("testDiv", 6);

    // Integer `div` is Java's `/` on two ints (FHIRPathEngine.java:3047-3050),
    // which truncates toward zero rather than flooring, so a negative
    // numerator rounds up.
    assert_eq!(eval("5 div 2"), integer("2"));
    assert_eq!(eval("(-5) div 2"), integer("-2"));
    assert_eq!(eval("2.2 div 1.8 = 1"), boolean(true));
    assert_eq!(
        eval("(2.2 div 1.8) is System.Integer"),
        boolean(true),
        "the decimal arm produces an IntegerType too (FHIRPathEngine.java:3051-3058)"
    );
}

// ---------------------------------------------------------------------------
// path.math.007 -- `mod`
// ---------------------------------------------------------------------------

// covers: path.math.007
#[test]
fn mod_returns_the_remainder() {
    expect_group("testMod", 8);

    assert_eq!(eval("5 mod 2"), integer("1"));
    assert_eq!(eval("4 mod 2"), integer("0"));
    assert_eq!(eval("2.2 mod 1.8 = 0.4"), boolean(true));
    assert_eq!(
        eval("(2.2 mod 1.8) is System.Decimal"),
        boolean(true),
        "the decimal arm produces a DecimalType"
    );
    // Java's `%` keeps the sign of the dividend.
    assert_eq!(eval("(-5) mod 2"), integer("-1"));
}

// ---------------------------------------------------------------------------
// path.math.008 -- division by zero
// ---------------------------------------------------------------------------

// covers: path.math.008
#[test]
fn division_by_zero_yields_empty() {
    // Three different mechanisms reach the same answer: `opDivideBy` catches
    // the UcumException `Decimal.divide` raises and returns nothing
    // (FHIRPathEngine.java:3017-3019), while `opDiv` and `opMod` guard the
    // divisor and add nothing to the result (:3048, :3089).
    assert_eq!(eval("1 / 0"), empty());
    assert_eq!(eval("1 div 0"), empty());
    assert_eq!(eval("1 mod 0"), empty());

    // Empty and not an error, and empty for the decimal spellings too -- the
    // suite grades `0.0 / 0`, `0 div 0.0`, `5.5 mod 0`, `5 mod 0.0` and
    // `0.0 mod 0`.
    assert_eq!(eval("0.0 / 0"), empty());
    assert_eq!(eval("0 div 0.0"), empty());
    assert_eq!(eval("5.5 mod 0"), empty());
    assert_eq!(eval("(1 / 0).empty()"), boolean(true));

    for (group, name) in [
        ("testDivide", "testDivide6"),
        ("testDivide", "testDivide7"),
        ("testDiv", "testDiv5"),
        ("testDiv", "testDiv6"),
        ("testMod", "testMod5"),
        ("testMod", "testMod6"),
        ("testMod", "testMod7"),
        ("testMod", "testMod8"),
    ] {
        expect_case(group, name);
    }
}

// ---------------------------------------------------------------------------
// path.math.009 -- an empty operand
// ---------------------------------------------------------------------------

// covers: path.math.009
#[test]
fn arithmetic_with_an_empty_operand_yields_empty() {
    // Each arithmetic operator opens with
    // `if (left.size() == 0 || right.size() == 0) return new ArrayList<Base>()`
    // (FHIRPathEngine.java:2678-2680 and the four mirrors).
    for op in ["+", "-", "*", "/", "div", "mod"] {
        assert_eq!(eval(&format!("{{}} {op} 1")), empty(), "{{}} {op} 1");
        assert_eq!(eval(&format!("1 {op} {{}}")), empty(), "1 {op} {{}}");
    }

    // Empty and not zero, and not the other operand.
    assert_eq!(eval("({} + 1).empty()"), boolean(true));
    assert_eq!(eval("({} + 1) = 1"), empty());
}

// ---------------------------------------------------------------------------
// path.math.010 -- incompatible types
// ---------------------------------------------------------------------------

// covers: path.math.010
#[test]
fn arithmetic_on_incompatible_types_is_an_execution_error() {
    // `opPlus` falls through to
    // `throw makeException(expr, FHIRPATH_OP_INCOMPATIBLE, "+", left.fhirType(), right.fhirType())`
    // (FHIRPathEngine.java:2708-2710). The message names the operator and both
    // types.
    let message = expect_evaluation_error("1 + true");
    assert!(
        message.contains("integer") && message.contains("boolean"),
        "the diagnostic must name both operand types: {message:?}"
    );

    // The same for the other operators, so the rule is about the operand types
    // and not about `+`.
    for op in ["-", "*", "/", "div", "mod"] {
        let expr = format!("1 {op} true");
        let message = expect_evaluation_error(&expr);
        assert!(
            message.contains("integer") && message.contains("boolean"),
            "{expr}: {message:?}"
        );
    }

    // And a boolean on the left, which takes the left-operand branch.
    expect_evaluation_error("true + 1");
}

// ---------------------------------------------------------------------------
// path.math.011 -- integer overflow
// ---------------------------------------------------------------------------

// covers: path.math.011
#[test]
fn integer_arithmetic_that_overflows_wraps_the_way_the_pinned_engine_does() {
    // What the pinned engine does, recorded: `opPlus`'s integer arm is
    // `new IntegerType(Integer.parseInt(l) + Integer.parseInt(r))`
    // (FHIRPathEngine.java:2700-2701). Java's `int` addition wraps on overflow
    // and raises nothing, so the answer is the two's-complement result. It is
    // not an error, not empty, and not a widened decimal.
    assert_eq!(
        eval("2147483647 + 1"),
        integer("-2147483648"),
        "2147483647 + 1 wraps to the most negative int"
    );

    // `opTimes`'s integer arm wraps for the same reason
    // (FHIRPathEngine.java:2789-2790).
    assert_eq!(eval("2147483647 * 2"), integer("-2"));

    // The result is still an integer, so the wrap is not a silent widening to
    // decimal -- which is what `path.literal.015` shows happens to a literal
    // past the same boundary.
    assert_eq!(eval("(2147483647 + 1) is System.Integer"), boolean(true));

    // And the decimal arm, reached when either operand is a decimal, does not
    // wrap: `BigDecimal.add` is unbounded.
    assert_eq!(
        eval("2147483647.0 + 1"),
        primitive("decimal", "2147483648.0")
    );
}

// ---------------------------------------------------------------------------
// path.math.012 -- date arithmetic with calendar durations
// ---------------------------------------------------------------------------

// covers: path.math.012
#[test]
fn date_arithmetic_adds_and_subtracts_calendar_durations() {
    // `dateAdd` switches on `q.hasCode() ? q.getCode() : q.getUnit()`
    // (FHIRPathEngine.java:2715-2804) and calls `BaseDateTimeType.add`, which
    // routes to `DateUtils.addYears` / `addMonths` / `addDays`
    // (BaseDateTimeType.java:117-141).
    //
    // `1 year` and `1 month` carry no code, so the switch reads their unit
    // word; `1 day` carries the code `d`. All three therefore reach a calendar
    // arm rather than the `a` / `mo` arms, which raise FHIRPATH_ARITHMETIC_QTY.
    assert_eq!(eval("@2014-01-01 + 1 year"), date("2015-01-01"));
    assert_eq!(eval("@2014-01-01 - 1 day"), date("2013-12-31"));

    // Month-end behaviour: `DateUtils.addMonths` clamps to the last day of the
    // target month rather than overflowing into March.
    assert_eq!(
        eval("@2014-01-31 + 1 month"),
        date("2014-02-28"),
        "January 31 plus one month is February 28 in 2014, not March 3"
    );

    // The UCUM spellings of year and month are refused, which is the other half
    // of why the calendar keywords carry no code: `case "a"` and `case "mo"`
    // raise (FHIRPathEngine.java:2727-2728 and :2733-2734).
    let message = expect_evaluation_error("@2014-01-01 + 1 'a'");
    assert!(
        !message.is_empty(),
        "adding a definite-duration year must be refused with a message"
    );
    expect_evaluation_error("@2014-01-01 + 1 'mo'");

    // A unit that is neither is refused too, which the suite grades as
    // `testMinus/testMinus6` (`@1974-12-25 - 1 'cm'`).
    expect_case("testMinus", "testMinus6");

    // And the suite's own date-arithmetic cases.
    expect_case("testMinus", "testMinus5");
    expect_case("testMinus", "testMinus7");
}

// ---------------------------------------------------------------------------
// path.math.013 -- precision is preserved
// ---------------------------------------------------------------------------

// covers: path.math.013
#[test]
fn date_arithmetic_preserves_the_operands_precision() {
    // `BaseDateTimeType.add` passes `getPrecision()` straight back into
    // `setValue` (BaseDateTimeType.java:119-121), so the result is rendered at
    // the operand's precision and no month or day is invented.
    assert_eq!(eval("@2014 + 1 year"), date("2015"));
    assert_eq!(eval("@2014.precision()"), integer("4"));
    assert_eq!(eval("(@2014 + 1 year).precision()"), integer("4"));

    // The same at month precision.
    assert_eq!(eval("@2014-01 + 1 month"), date("2014-02"));
    assert_eq!(eval("(@2014-01 + 1 month).precision()"), integer("6"));

    // And at day precision, where the full date is retained.
    assert_eq!(eval("(@2014-01-01 + 1 year).precision()"), integer("8"));
}

// ---------------------------------------------------------------------------
// path.math.014 -- subtracting two dateTimes
// ---------------------------------------------------------------------------

// covers: path.math.014
#[test]
fn subtracting_two_dates_is_an_error_and_not_a_duration() {
    // What the pinned engine does, recorded: it is an error, not a quantity.
    //
    // `opMinus` has five arms (FHIRPathEngine.java:2961-2977) and every one of
    // them requires the right operand to be numeric or a Quantity. Two dates
    // match none, so the method falls through to
    // `throw makeException(expr, FHIRPATH_OP_INCOMPATIBLE, "-", "date", "date")`
    // at :2977. The check pass declines it too: `operateTypes`'s `case Minus`
    // raises FHIRPATH_ARITHMETIC_MINUS when the left operand is a temporal type
    // and the right is not a Quantity (FHIRPathEngine.java:2066-2080) -- which
    // of the two stages reports it depends on the static type the literal is
    // given, so the stage is not asserted here, only the failure.
    let (stage, message) = expect_failure("@2014-01-02 - @2014-01-01");
    assert!(
        matches!(stage, Stage::Check | Stage::Evaluate),
        "the failure must come from the check or the evaluate stage, not from the parser"
    );
    assert!(
        !message.is_empty(),
        "the diagnostic must say why the operands cannot be subtracted"
    );

    // The same for two dateTimes and two times, so the rule is about the
    // operand kinds and not about `date` specifically.
    expect_failure("@2014-01-02T10:00:00 - @2014-01-01T10:00:00");

    // And subtracting a duration from a date does work, which is what makes the
    // failure above a statement about the right operand
    // (`path.math.012` is the scenario for it).
    assert_eq!(eval("@2014-01-02 - 1 day"), date("2014-01-01"));
}

// ---------------------------------------------------------------------------
// path.mathfn.001 -- `abs`
// ---------------------------------------------------------------------------

// covers: path.mathfn.001
#[test]
fn abs_works_on_integers_decimals_and_quantities() {
    expect_group("testAbs", 3);

    // `funcAbs`'s numeric arm parses the focus as a double and returns
    // `new DecimalType(Math.abs(d))` (FHIRPathEngine.java:4121-4123), so even
    // an integer focus comes back as a decimal; the quantity arm uses
    // `BigDecimal.abs` and keeps the unit (:4126-4128).
    assert_eq!(eval("(-5).abs()"), primitive("decimal", "5.0"));
    assert_eq!(eval("(-5.5).abs()"), primitive("decimal", "5.5"));
    assert_eq!(eval("(-5.5 'mg').abs() = 5.5 'mg'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.002 -- `ceiling`
// ---------------------------------------------------------------------------

// covers: path.mathfn.002
#[test]
fn ceiling_rounds_up() {
    expect_group("testCeiling", 3);

    assert_eq!(eval("1.1.ceiling() = 2"), boolean(true));
    assert_eq!(
        eval("(-1.1).ceiling() = -1"),
        boolean(true),
        "toward positive infinity, so a negative value rounds toward zero"
    );
    assert_eq!(eval("1.ceiling() = 1"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.003 -- `floor`
// ---------------------------------------------------------------------------

// covers: path.mathfn.003
#[test]
fn floor_rounds_down() {
    expect_group("testFloor", 3);

    assert_eq!(eval("2.1.floor() = 2"), boolean(true));
    assert_eq!(
        eval("(-2.1).floor() = -3"),
        boolean(true),
        "toward negative infinity, so a negative value rounds away from zero"
    );
    assert_eq!(eval("1.floor() = 1"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.004 -- `round`
// ---------------------------------------------------------------------------

// covers: path.mathfn.004
#[test]
fn round_rounds_to_the_requested_number_of_digits() {
    expect_group("testRound", 2);

    // `funcRound` is exact: `new BigDecimal(base.primitiveValue()).setScale(i, HALF_UP)`
    // (FHIRPathEngine.java:4409-4410). It is one of the two math functions that
    // does not go through a double, so the scale it sets is retained exactly.
    assert_eq!(eval("3.14159.round(3)"), primitive("decimal", "3.142"));
    assert_eq!(
        eval("1.round()"),
        primitive("decimal", "1"),
        "with no argument the scale is zero"
    );
    // HALF_UP, not half-even: 2.5 rounds to 3 and not to 2.
    assert_eq!(eval("2.5.round(0)"), primitive("decimal", "3"));
    assert_eq!(eval("3.5.round(0)"), primitive("decimal", "4"));
}

// ---------------------------------------------------------------------------
// path.mathfn.005 -- `sqrt`
// ---------------------------------------------------------------------------

// covers: path.mathfn.005
#[test]
fn sqrt_of_a_negative_number_yields_empty() {
    expect_group("testSqrt", 2);

    // `Math.sqrt(-1)` is NaN, and `new DecimalType(Double)` on a NaN raises,
    // which the surrounding `catch (Exception e) { /* just return nothing */ }`
    // swallows (FHIRPathEngine.java:4103-4107). So a negative focus is empty
    // rather than an error.
    assert_eq!(eval("(-1).sqrt()"), empty());
    assert_eq!(eval("(-1).sqrt().empty()"), boolean(true));

    // A non-negative focus computes, through a double.
    assert_eq!(eval("81.sqrt() = 9.0"), boolean(true));
    assert_eq!(eval("0.sqrt() = 0.0"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.006 -- `exp` and `ln`
// ---------------------------------------------------------------------------

// covers: path.mathfn.006
#[test]
fn exp_and_ln_are_inverses_within_the_engines_precision() {
    expect_group("testExp", 3);
    expect_group("testLn", 2);

    assert_eq!(eval("0.exp() = 1"), boolean(true));
    assert_eq!(eval("1.ln() = 0.0"), boolean(true));

    // Inverse within the precision a double gives, which is what the scenario's
    // qualifier means: the round trip is asserted through `round` rather than
    // as an exact decimal equality.
    assert_eq!(eval("1.exp().ln().round(6) = 1.0"), boolean(true));

    // `exp` is the only one of the ten math functions with an empty-focus
    // guard (`if (focus.size() == 0) return new ArrayList<Base>()`,
    // FHIRPathEngine.java:4177-4179), which the suite grades as testExp3. See
    // `path.mathfn.010` for what that means for the other nine.
    assert_eq!(eval("{}.exp()"), empty());
}

// ---------------------------------------------------------------------------
// path.mathfn.007 -- `log`
// ---------------------------------------------------------------------------

// covers: path.mathfn.007
#[test]
fn log_takes_the_base_as_its_argument() {
    expect_group("testLog", 2);

    assert_eq!(eval("16.log(2) = 4.0"), boolean(true));
    assert_eq!(eval("100.0.log(10.0) = 2.0"), boolean(true));

    // The argument is the base and not the operand, so the two are not
    // interchangeable.
    assert_eq!(eval("2.log(16) = 4.0"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.mathfn.008 -- `power`
// ---------------------------------------------------------------------------

// covers: path.mathfn.008
#[test]
fn power_of_a_negative_base_with_a_fractional_exponent_yields_empty() {
    expect_group("testPower", 3);

    // `Math.pow(-1, 0.5)` is NaN and the DecimalType construction that follows
    // raises, which the surrounding catch swallows
    // (FHIRPathEngine.java:4249-4271).
    assert_eq!(eval("(-1).power(0.5)"), empty());
    assert_eq!(eval("(-1).power(0.5).empty()"), boolean(true));

    // An integral exponent on the same negative base computes.
    assert_eq!(eval("(-1).power(2) = 1"), boolean(true));
    assert_eq!(eval("2.power(3) = 8"), boolean(true));
    assert_eq!(eval("2.5.power(2) = 6.25"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.009 -- `truncate`
// ---------------------------------------------------------------------------

// covers: path.mathfn.009
#[test]
fn truncate_discards_the_fractional_part() {
    expect_group("testTruncate", 3);

    // The other exact function: it discards digits rather than rounding, and
    // toward zero rather than toward negative infinity -- which is what
    // separates it from `floor`.
    assert_eq!(eval("1.00000001.truncate() = 1"), boolean(true));
    assert_eq!(eval("(-1.56).truncate() = -1"), boolean(true));
    assert_eq!(eval("(-1.56).floor() = -2"), boolean(true));
    assert_eq!(eval("101.truncate() = 101"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.mathfn.010 -- an empty focus
// ---------------------------------------------------------------------------

// covers: path.mathfn.010
#[test]
fn a_math_function_on_an_empty_collection_yields_empty() {
    // NOTE FOR REVIEW -- scenario against pinned source.
    //
    // The scenario says `{}.abs()`, `{}.sqrt()` and `{}.round(2)` each yield an
    // empty collection. In the pinned engine all three raise: their guard is
    // `if (focus.size() != 1) throw makeExceptionPlural(...)`, which covers
    // zero as well as many --
    //   funcSqrt  FHIRPathEngine.java:4095-4097
    //   funcAbs   FHIRPathEngine.java:4115-4117
    //   funcRound FHIRPathEngine.java:4396-4398
    // and the same guard appears in funcCeiling (:4139), funcFloor (:4157),
    // funcLn (:4201), funcLog (:4221), funcPower (:4249), funcTruncate (:4273)
    // and funcPrecision (:4379). `funcExp` alone guards with
    // `if (focus.size() == 0) return new ArrayList<Base>()` (:4177-4179), and
    // it is the only one of the ten the suite grades on an empty focus
    // (`testExp/testExp3`, `{}.exp().empty() = true`).
    //
    // Written to the scenario as stated. If the scenario is wrong this test is
    // the thing that says so, which is better than a test that quietly encodes
    // the opposite.
    assert_eq!(eval("{}.abs()"), empty());
    assert_eq!(eval("{}.sqrt()"), empty());
    assert_eq!(eval("{}.round(2)"), empty());

    // `exp`, where the scenario and the pinned engine agree.
    assert_eq!(eval("{}.exp()"), empty());
    expect_case("testExp", "testExp3");
}

// ---------------------------------------------------------------------------
// path.mathfn.011 -- a focus of more than one
// ---------------------------------------------------------------------------

// covers: path.mathfn.011
#[test]
fn a_math_function_on_a_collection_of_more_than_one_is_an_execution_error() {
    // `funcAbs`'s guard raises FHIRPATH_FOCUS, whose text is
    // "focus for {1} can only have one value, but has {0} values", so the
    // message names the function and the count.
    let message = expect_evaluation_error("(1 | 2).abs()");
    assert!(
        message.contains("abs"),
        "the diagnostic must name the function: {message:?}"
    );
    assert!(
        message.contains('2'),
        "and the number of values it was given: {message:?}"
    );

    // Every math function behaves the same way on a multi-item focus,
    // including `exp`, whose empty-focus guard does not extend to this case
    // (FHIRPathEngine.java:4180-4182).
    for call in [
        "abs()",
        "sqrt()",
        "ceiling()",
        "floor()",
        "exp()",
        "ln()",
        "log(2)",
        "power(2)",
        "truncate()",
        "round(2)",
    ] {
        let expr = format!("(1 | 2).{call}");
        expect_evaluation_error(&expr);
    }

    // A one-item collection is a singleton and does compute, so the rule is
    // about the cardinality.
    assert_eq!(eval("(1 | 1).abs()"), primitive("decimal", "1.0"));
}
