//! Literals: `path.literal.001` through `path.literal.015`.
//!
//! ```text
//! cargo test -p frevo-path --test path_literals
//! ```
//!
//! Where the scenario names a case in the vendored suite the case is driven
//! through the committed harness, so the expression and the expectation come
//! from `oracles/fhirpath/tests-fhir-r4.xml` and not from this file. The
//! structural scenarios -- what a quantity literal is made of, what `{}` is,
//! what happens past the 32-bit boundary -- go at the engine seam, because the
//! suite only ever observes those through `convertsToX()` and so cannot say
//! what the literal's parts are.
//!
//! Provenance: `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, principally `processConstant`
//! (:1067-1083) and the quantity-literal arm of `parseExpression` (:1105-1132),
//! plus `org.hl7.fhir.utilities.Utilities.isInteger` and
//! `org.hl7.fhir.utilities.DateTimeUtil.getDatePrecision`.

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
use oracle::value::{QuantityValue, ResultItem, UCUM_SYSTEM};

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

fn case_report(group: &str, name: &str) -> &'static CaseReport {
    let (suite, report) = pinned();
    let case = suite.case(group, name).unwrap_or_else(|| {
        panic!("{group}/{name} must resolve to exactly one case in the pinned suite")
    });
    report
        .get(&case.address.to_string())
        .unwrap_or_else(|| panic!("{} must appear in the run report", case.address))
}

/// Assert a suite case passed, naming the status and the reason so a
/// `did-not-run` is never read as a wrong answer.
fn expect_case(group: &str, name: &str) -> &'static CaseReport {
    let r = case_report(group, name);
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

/// The single expected `<output>` the pinned file records for a case.
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

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before any literal is parsed: {e}"));
    engine
}

/// Parse, check and evaluate a literal-only expression. Every stage must
/// succeed; `NotImplemented` panics, so a missing engine never counts as an
/// answer.
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

/// The stage at which the engine reported a diagnostic for `expr`, with that
/// diagnostic. `NotImplemented` panics rather than being read as the failure
/// the scenario asked for.
fn expect_failure(expr: &str) -> (Stage, String, Option<usize>) {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = match engine.parse(expr) {
        Err(EngineError::Diagnostic { message, offset }) => {
            return (Stage::Parse, message, offset);
        }
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at parse: {e}"),
        Ok(p) => p,
    };
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Err(EngineError::Diagnostic { message, offset }) => {
            return (Stage::Check, message, offset);
        }
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at check: {e}"),
        Ok(()) => {}
    }
    match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
        Err(EngineError::Diagnostic { message, offset }) => (Stage::Evaluate, message, offset),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at evaluate: {e}"),
        Ok(items) => panic!("{expr:?} must fail, but it evaluated to {items:?}"),
    }
}

fn primitive(fhir_type: &str, value: &str) -> ResultItem {
    ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }
}

/// The sole item of a one-item result, or a failure naming what came back.
fn sole(expr: &str) -> ResultItem {
    let items = eval(expr);
    assert_eq!(
        items.len(),
        1,
        "{expr:?} must yield exactly one item, not {items:?}"
    );
    items.into_iter().next().expect("one item")
}

/// `expr` must yield one integer with this value.
fn expect_integer(expr: &str, value: &str) {
    assert_eq!(sole(expr), primitive("integer", value), "{expr:?}");
}

// ---------------------------------------------------------------------------
// path.literal.001 -- `true` and `false`
// ---------------------------------------------------------------------------

// covers: path.literal.001
#[test]
fn true_and_false_are_boolean_literals() {
    for name in ["testLiteralTrue", "testLiteralFalse"] {
        let (ty, body) = sole_output("testLiterals", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testLiterals", name);
    }

    // And as bare literals they are booleans, which is what makes them usable
    // as the operands above: `processConstant` builds a BooleanType for exactly
    // the two spellings `true` and `false` (FHIRPathEngine.java:1074-1075).
    assert_eq!(sole("true"), primitive("boolean", "true"));
    assert_eq!(sole("false"), primitive("boolean", "false"));
}

// ---------------------------------------------------------------------------
// path.literal.002 -- a single-quoted string
// ---------------------------------------------------------------------------

// covers: path.literal.002
#[test]
fn a_single_quoted_run_is_a_string_literal() {
    let (suite, _) = pinned();
    let case = suite
        .case("testLiterals", "testLiteralString1")
        .expect("testLiterals/testLiteralString1 is addressable");
    assert_eq!(case.expression, "Patient.name.given.first() = 'Peter'");
    let (ty, body) = sole_output("testLiterals", "testLiteralString1");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testLiterals", "testLiteralString1");

    // The literal's own type is `string` and its value has the quotes removed.
    assert_eq!(sole("'Peter'"), primitive("string", "Peter"));
}

// ---------------------------------------------------------------------------
// path.literal.003 -- an integer literal is an integer
// ---------------------------------------------------------------------------

// covers: path.literal.003
#[test]
fn an_integer_literal_is_an_integer() {
    for name in [
        "testLiteralInteger1",
        "testLiteralInteger0",
        "testLiteralIntegerMax",
    ] {
        let (ty, body) = sole_output("testLiterals", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testLiterals", name);
    }

    // Including at the upper end of the signed 32-bit range, which is where
    // `Utilities.isInteger`'s bounds check puts the boundary.
    expect_integer("1", "1");
    expect_integer("0", "0");
    expect_integer("2147483647", "2147483647");
}

// ---------------------------------------------------------------------------
// path.literal.004 -- a negative integer needs parentheses
// ---------------------------------------------------------------------------

// covers: path.literal.004
#[test]
fn a_parenthesised_negative_integer_converts_to_an_integer() {
    let (suite, _) = pinned();
    let case = suite
        .case("testLiterals", "testLiteralIntegerNegative1")
        .expect("the case is addressable");
    assert_eq!(case.expression, "(-1).convertsToInteger()");
    assert_eq!(
        case.expected_failure, None,
        "the parenthesised form is valid, unlike its unparenthesised sibling"
    );
    let (ty, body) = sole_output("testLiterals", "testLiteralIntegerNegative1");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testLiterals", "testLiteralIntegerNegative1");

    // The parenthesised group really is a negative integer, so the unary minus
    // binds to the literal and not to a later function call.
    assert_eq!(sole("(-1)"), primitive("integer", "-1"));
}

// ---------------------------------------------------------------------------
// path.literal.005 -- an unparenthesised negative integer with a call
// ---------------------------------------------------------------------------

// covers: path.literal.005
#[test]
fn an_unparenthesised_negative_integer_before_a_call_is_an_execution_error() {
    let (suite, _) = pinned();
    let case = suite
        .case("testLiterals", "testLiteralIntegerNegative1Invalid")
        .expect("the case is addressable");
    assert_eq!(case.expression, "-1.convertsToInteger()");
    assert_eq!(
        case.expected_failure,
        Some(Stage::Evaluate),
        "the pinned case must still be invalid=\"execution\""
    );
    let r = expect_case("testLiterals", "testLiteralIntegerNegative1Invalid");
    assert_eq!(
        r.failed_stage,
        Some(Stage::Evaluate),
        "it must fail when evaluated and not when parsed: the expression is \
         syntactically fine"
    );

    // Why it fails: a unary minus is parsed as `0 - <rest>`
    // (`execute`'s `case Unary` pushes an IntegerType(0),
    // FHIRPathEngine.java:1471-1473), so the subtraction's right operand is the
    // boolean `1.convertsToInteger()` returns, and `opMinus` has no arm for
    // integer minus boolean (FHIRPathEngine.java:2961-2977).
    let (stage, message, _) = expect_failure("-1.convertsToInteger()");
    assert_eq!(stage, Stage::Evaluate);
    assert!(
        message.contains("integer") && message.contains("boolean"),
        "FHIRPATH_OP_INCOMPATIBLE names both operand types: {message:?}"
    );

    // And the same expression with the minus applied to a number evaluates, so
    // the failure is about negating a boolean.
    expect_integer("-1", "-1");
}

// ---------------------------------------------------------------------------
// path.literal.006 -- a decimal literal keeps its precision
// ---------------------------------------------------------------------------

// covers: path.literal.006
#[test]
fn a_decimal_literal_keeps_the_precision_it_was_written_with() {
    let (ty, body) = sole_output("Precision", "PrecisionDecimal");
    assert_eq!(
        (ty.as_str(), body.as_str()),
        ("", "5"),
        "1.58700.precision() is 5, so the two trailing zeros are retained"
    );
    expect_case("Precision", "PrecisionDecimal");

    // The literal is a decimal whose lexical form still carries the trailing
    // zeros: `processConstant` builds `new DecimalType(lexer.take())` from the
    // source text (FHIRPathEngine.java:1072), and `Utilities.getDecimalPrecision`
    // counts the digits that text records.
    assert_eq!(sole("1.58700"), primitive("decimal", "1.58700"));
    expect_integer("1.58700.precision()", "5");

    // A shorter spelling of the same number reports a shorter precision, which
    // is what makes this a statement about the literal and not about its value.
    expect_integer("1.587.precision()", "3");
}

// ---------------------------------------------------------------------------
// path.literal.007 -- date literals at each precision
// ---------------------------------------------------------------------------

// covers: path.literal.007
#[test]
fn date_literals_are_accepted_at_year_month_and_day_precision() {
    // The suite grades only `is(Date)` for these three.
    for name in [
        "testLiteralDateYear",
        "testLiteralDateMonth",
        "testLiteralDateDay",
    ] {
        let (ty, body) = sole_output("testLiterals", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testLiterals", name);
    }

    // Each literal is a single `date`, and the retained precision is the digit
    // count `DateTimeUtil.getDatePrecision` reports -- 4, 6 and 8, not 1, 2 and
    // 3. `Precision/PrecisionYear` pins the first of those three numbers in the
    // suite itself.
    for (expr, value, digits) in [
        ("@2014", "2014", "4"),
        ("@2014-01", "2014-01", "6"),
        ("@2014-01-05", "2014-01-05", "8"),
    ] {
        assert_eq!(sole(expr), primitive("date", value), "{expr} is a date");
        expect_integer(&format!("{expr}.precision()"), digits);
    }
}

// ---------------------------------------------------------------------------
// path.literal.008 -- a year-precision date reports precision 4
// ---------------------------------------------------------------------------

// covers: path.literal.008
#[test]
fn a_year_precision_date_reports_precision_four() {
    let (ty, body) = sole_output("Precision", "PrecisionYear");
    assert_eq!((ty.as_str(), body.as_str()), ("", "4"));
    expect_case("Precision", "PrecisionYear");
    expect_integer("@2014.precision()", "4");
}

// ---------------------------------------------------------------------------
// path.literal.009 -- dateTime literals at each precision and offset
// ---------------------------------------------------------------------------

// covers: path.literal.009
#[test]
fn datetime_literals_are_accepted_at_every_precision_and_offset_form() {
    // The suite's `is(DateTime)` cases, which cover the year-, month- and
    // day-precision `@...T` forms as well as the ones the scenario names.
    for name in [
        "testLiteralDateTimeYear",
        "testLiteralDateTimeMonth",
        "testLiteralDateTimeDay",
        "testLiteralDateTimeHour",
        "testLiteralDateTimeMinute",
        "testLiteralDateTimeSecond",
        "testLiteralDateTimeMillisecond",
        "testLiteralDateTimeUTC",
        "testLiteralDateTimeTimezoneOffset",
    ] {
        let (ty, body) = sole_output("testLiterals", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testLiterals", name);
    }

    // The five forms the scenario names, each a single `dateTime` whose value
    // is the literal text with the leading `@` removed -- `processDateConstant`
    // reassembles date, time and offset rather than normalising the offset
    // away (FHIRPathEngine.java:1673-1760).
    for (expr, value) in [
        ("@2014-01-05T10:30", "2014-01-05T10:30"),
        ("@2014-01-05T10:30:00", "2014-01-05T10:30:00"),
        ("@2014-01-05T10:30:00.000", "2014-01-05T10:30:00.000"),
        ("@2014-01-05T10:30:00Z", "2014-01-05T10:30:00Z"),
        ("@2014-01-05T10:30:00+10:00", "2014-01-05T10:30:00+10:00"),
    ] {
        assert_eq!(sole(expr), primitive("dateTime", value), "{expr}");
    }
}

// ---------------------------------------------------------------------------
// path.literal.010 -- a millisecond dateTime reports precision 17
// ---------------------------------------------------------------------------

// covers: path.literal.010
#[test]
fn a_millisecond_datetime_reports_precision_seventeen() {
    let (ty, body) = sole_output("Precision", "PrecisionDateTimeMilliseconds");
    assert_eq!(
        (ty.as_str(), body.as_str()),
        ("", "17"),
        "17 is the digit count of 20140105103000000"
    );
    expect_case("Precision", "PrecisionDateTimeMilliseconds");
    expect_integer("@2014-01-05T10:30:00.000.precision()", "17");
}

// ---------------------------------------------------------------------------
// path.literal.011 -- time literals at each precision
// ---------------------------------------------------------------------------

// covers: path.literal.011
#[test]
fn time_literals_are_accepted_at_each_precision() {
    let (ty_min, body_min) = sole_output("Precision", "PrecisionTimeMinutes");
    assert_eq!((ty_min.as_str(), body_min.as_str()), ("", "4"));
    expect_case("Precision", "PrecisionTimeMinutes");

    let (ty_ms, body_ms) = sole_output("Precision", "PrecisionTimeMilliseconds");
    assert_eq!((ty_ms.as_str(), body_ms.as_str()), ("", "9"));
    expect_case("Precision", "PrecisionTimeMilliseconds");

    expect_integer("@T10:30.precision()", "4");
    expect_integer("@T10:30:00.000.precision()", "9");

    // A time literal is a `time`, and the suite's `is(Time)` cases cover the
    // hour, minute, second and millisecond forms. The two offset-carrying time
    // literals are `invalid="execution"` upstream, which is why they are not in
    // this list.
    for name in [
        "testLiteralTimeHour",
        "testLiteralTimeMinute",
        "testLiteralTimeSecond",
        "testLiteralTimeMillisecond",
    ] {
        expect_case("testLiterals", name);
    }
    assert_eq!(sole("@T10:30"), primitive("time", "10:30"));
}

// ---------------------------------------------------------------------------
// path.literal.012 -- a quantity literal with a UCUM unit
// ---------------------------------------------------------------------------

// covers: path.literal.012
#[test]
fn a_quantity_literal_with_a_ucum_unit_is_a_quantity() {
    // FHIRPathEngine.java:1132 builds the literal as
    // `new Quantity().setValue(..).setUnit(unit).setSystem(..).setCode(ucum)`,
    // and for a string-constant unit `unit` is left null while `ucum` is the
    // quoted code. So `1 'cm'` carries a code and a system and no unit text.
    assert_eq!(
        sole("1 'cm'"),
        ResultItem::Quantity(QuantityValue {
            value: "1".to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("cm".to_string()),
            has_element_id: false,
            has_extensions: false,
        }),
        "a UCUM-unit quantity literal has a code and a system but no unit text"
    );

    // A decimal value keeps its lexical form, as `path.literal.006` requires of
    // decimals generally.
    assert_eq!(
        sole("10.1 'mg'"),
        ResultItem::Quantity(QuantityValue {
            value: "10.1".to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("mg".to_string()),
            has_element_id: false,
            has_extensions: false,
        })
    );

    // The suite's own grading of the same two shapes.
    expect_case("testLiterals", "testLiteralQuantityInteger");
    expect_case("testLiterals", "testLiteralQuantityDecimal");
}

// ---------------------------------------------------------------------------
// path.literal.013 -- calendar-duration keywords
// ---------------------------------------------------------------------------

// covers: path.literal.013
#[test]
fn calendar_duration_keywords_are_quantity_literals() {
    // FHIRPathEngine.java:1105-1131 recognises exactly sixteen keywords, eight
    // singular and eight plural, and maps them to a UCUM code -- except `year`
    // and `month`, which upstream explicitly declines to map ("this is not the
    // UCUM year"). For those two `ucum` stays null, so the literal carries
    // neither a system nor a code and only the unit word identifies it.
    /// One expected calendar-duration literal.
    struct CalendarLiteral {
        expr: &'static str,
        value: &'static str,
        /// The literal word the quantity retains as its `unit`.
        unit: &'static str,
        /// The UCUM code, where upstream maps one.
        code: Option<&'static str>,
    }

    const CASES: &[CalendarLiteral] = &[
        CalendarLiteral {
            expr: "7 days",
            value: "7",
            unit: "days",
            code: Some("d"),
        },
        CalendarLiteral {
            expr: "1 week",
            value: "1",
            unit: "week",
            code: Some("wk"),
        },
        CalendarLiteral {
            expr: "1 weeks",
            value: "1",
            unit: "weeks",
            code: Some("wk"),
        },
        CalendarLiteral {
            expr: "1 day",
            value: "1",
            unit: "day",
            code: Some("d"),
        },
        CalendarLiteral {
            expr: "1 hour",
            value: "1",
            unit: "hour",
            code: Some("h"),
        },
        CalendarLiteral {
            expr: "1 minute",
            value: "1",
            unit: "minute",
            code: Some("min"),
        },
        CalendarLiteral {
            expr: "1 second",
            value: "1",
            unit: "second",
            code: Some("s"),
        },
        CalendarLiteral {
            expr: "1 millisecond",
            value: "1",
            unit: "millisecond",
            code: Some("ms"),
        },
        CalendarLiteral {
            expr: "1 year",
            value: "1",
            unit: "year",
            code: None,
        },
        CalendarLiteral {
            expr: "1 years",
            value: "1",
            unit: "years",
            code: None,
        },
        CalendarLiteral {
            expr: "1 month",
            value: "1",
            unit: "month",
            code: None,
        },
        CalendarLiteral {
            expr: "1 months",
            value: "1",
            unit: "months",
            code: None,
        },
    ];

    for case in CASES {
        // The system is present exactly when a code is, because upstream sets
        // `system` from `ucum == null ? null : "http://unitsofmeasure.org"`.
        let system = case.code.map(|_| UCUM_SYSTEM.to_string());
        assert_eq!(
            sole(case.expr),
            ResultItem::Quantity(QuantityValue {
                value: case.value.to_string(),
                comparator: None,
                unit: Some(case.unit.to_string()),
                system,
                code: case.code.map(str::to_string),
                has_element_id: false,
                has_extensions: false,
            }),
            "{} is a calendar-duration quantity literal",
            case.expr
        );
    }

    // Plural and singular are treated alike in the one respect that decides
    // arithmetic and comparison -- the UCUM code -- and differ only in the unit
    // text they retain, which is the literal word.
    for (singular, plural) in [
        ("1 day", "1 days"),
        ("1 week", "1 weeks"),
        ("1 hour", "1 hours"),
        ("1 minute", "1 minutes"),
        ("1 second", "1 seconds"),
        ("1 millisecond", "1 milliseconds"),
        ("1 year", "1 years"),
        ("1 month", "1 months"),
    ] {
        let (a, b) = (sole(singular), sole(plural));
        match (&a, &b) {
            (ResultItem::Quantity(qa), ResultItem::Quantity(qb)) => {
                assert_eq!(qa.code, qb.code, "{singular} and {plural} share a code");
                assert_eq!(
                    qa.system, qb.system,
                    "{singular} and {plural} share a system"
                );
                assert_eq!(qa.value, qb.value);
                assert_ne!(
                    qa.unit, qb.unit,
                    "the retained unit text is the literal word, so it differs"
                );
            }
            _ => panic!("{singular} and {plural} must both be quantities: {a:?} {b:?}"),
        }
    }

    // The suite grades one of these.
    expect_case("testLiterals", "testLiteralQuantityDay");
}

// ---------------------------------------------------------------------------
// path.literal.014 -- the empty collection literal
// ---------------------------------------------------------------------------

// covers: path.literal.014
#[test]
fn the_empty_collection_literal_is_empty() {
    // `processConstant` returns a null constant for `{}`
    // (FHIRPathEngine.java:1076-1078) and `resolveConstant` turns that into an
    // empty list (FHIRPathEngine.java:1654-1656) -- not into a one-item
    // collection holding a null, which is what would make `{}.count()` 1.
    assert_eq!(eval("{}"), Vec::<ResultItem>::new());
    expect_integer("{}.count()", "0");
    assert_eq!(sole("{}.empty()"), primitive("boolean", "true"));
}

// ---------------------------------------------------------------------------
// path.literal.015 -- past the 32-bit boundary
// ---------------------------------------------------------------------------

// covers: path.literal.015
#[test]
fn an_integer_literal_beyond_the_32_bit_range_is_lexed_as_a_decimal() {
    // What the pinned engine does, recorded: `processConstant` asks
    // `Utilities.isInteger` first, and that method bounds-checks --
    // "check bounds -2,147,483,648..2,147,483,647", returning false for any
    // value of more than ten digits or a ten-digit value above 2147483647. The
    // next arm, `Utilities.isDecimal`, accepts it. So the literal becomes a
    // DecimalType. It is neither an integer nor an error.
    assert_eq!(
        sole("2147483648 is System.Decimal"),
        primitive("boolean", "true"),
        "past the boundary the literal is a System.Decimal"
    );
    assert_eq!(
        sole("2147483648 is System.Integer"),
        primitive("boolean", "false"),
        "and it is not a System.Integer"
    );

    // One below the boundary is still an integer, so the boundary is where
    // `Utilities.isInteger` puts it and not somewhere else.
    assert_eq!(
        sole("2147483647 is System.Integer"),
        primitive("boolean", "true")
    );
    assert_eq!(
        sole("2147483647 is System.Decimal"),
        primitive("boolean", "false")
    );

    // Being a decimal, it takes the decimal arm of `opPlus`
    // (FHIRPathEngine.java:2702-2703) and so does not wrap the way integer
    // addition does in `path.math.011`.
    assert_eq!(sole("2147483648 + 1"), primitive("decimal", "2147483649"));

    // And the literal evaluates at all, which is the part a lexer that rejected
    // it outright would fail.
    assert_eq!(sole("2147483648"), primitive("decimal", "2147483648"));
}
