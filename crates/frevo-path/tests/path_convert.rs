//! Type conversion: `path.convert.001` through `path.convert.014`.
//!
//! ```text
//! cargo test -p frevo-path --test path_convert
//! ```
//!
//! # Scenarios against the pinned source -- read before fixing a red test here
//!
//! Three of the fourteen scenarios describe behaviour the pinned R4 engine does
//! not have. All three are recorded inline at the test that asserts them, and
//! all three are about the `toX` half of the set rather than the `convertsToX`
//! half:
//!
//! | Scenario | Says | `FHIRPathEngine` at the pinned SHA |
//! |---|---|---|
//! | `path.convert.011` | `@2014.toDateTime()` is a year-precision dateTime | `funcToDateTime` raises FHIRPATH_NOT_IMPLEMENTED |
//! | `path.convert.013` | `'14:34:28'.toTime()` is a time, `'abc'.toTime()` is empty | `funcToTime` raises FHIRPATH_NOT_IMPLEMENTED |
//! | `path.convert.012` | `toDate()` does not exist | agrees -- this one is `DEV-0009` and is asserted as written |
//!
//! `funcToDateTime` and `funcToTime` are stubs whose whole body is
//! `throw makeException(expr, I18nConstants.FHIRPATH_NOT_IMPLEMENTED, ...)`,
//! with the intended implementation commented out above it. The pinned suite
//! contains no `.toTime()` or `.toDateTime()` case at all, which is consistent
//! with that: the asymmetry `path.convert.012` records for `toDate` is really
//! three functions wide, not one.
//!
//! Written to the scenarios as stated. Resolving it needs either `DEV-nnnn`
//! entries for the two stubs, a `docs/known-gaps.md` entry with the owner's
//! approval, or a correction to the spec delta.
//!
//! Provenance: `FHIRPathEngine.java` and `ExpressionNode.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
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
        "the pinned {group} group is {expected_cases} cases"
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

/// Every case in `group` whose expression mentions `call` must pass, and the
/// number of such cases must be what this test was written against.
///
/// Selecting by the call text rather than by case name is deliberate: the
/// `testTypes` group is 101 cases covering every conversion, and the scenarios
/// carve it up by which function they are about.
fn expect_cases_calling(group: &str, call: &str, expected: usize) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    let selected: Vec<_> = g
        .cases
        .iter()
        .filter(|c| c.expression.contains(call))
        .collect();
    assert_eq!(
        selected.len(),
        expected,
        "{group} must have {expected} case(s) calling {call}; a different count means the \
         suite was re-pinned without this test being updated"
    );

    let mut failures = Vec::new();
    for case in &selected {
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
        "{} of {} {call} cases in {group} did not pass; the first few:\n  {}",
        failures.len(),
        selected.len(),
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

/// `expr` must fail to **parse**, with a real diagnostic.
fn expect_parse_error(expr: &str) -> String {
    let mut engine = fresh_engine();
    match engine.parse(expr) {
        Ok(_) => panic!("{expr:?} must fail to parse, but it parsed"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail to parse with a diagnostic, not: {e}"),
    }
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

fn primitive(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

fn boolean(value: bool) -> Vec<ResultItem> {
    primitive("boolean", if value { "true" } else { "false" })
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// path.convert.001 -- `toInteger`
// ---------------------------------------------------------------------------

// covers: path.convert.001
#[test]
fn to_integer_converts_strings_booleans_and_integers() {
    expect_group("testToInteger", 5);

    // `funcToInteger` takes the focus's string value and accepts an integer
    // spelling or the two boolean words, mapping them to 1 and 0
    // (FHIRPathEngine.java, `funcToInteger`). Anything else yields nothing.
    assert_eq!(eval("'1'.toInteger()"), primitive("integer", "1"));
    assert_eq!(eval("1.toInteger()"), primitive("integer", "1"));
    assert_eq!(eval("true.toInteger()"), primitive("integer", "1"));
    assert_eq!(eval("false.toInteger()"), primitive("integer", "0"));
    assert_eq!(eval("'1.0'.toInteger()"), empty());
}

// ---------------------------------------------------------------------------
// path.convert.002 -- `toDecimal`
// ---------------------------------------------------------------------------

// covers: path.convert.002
#[test]
fn to_decimal_converts_strings_booleans_integers_and_decimals() {
    expect_group("testToDecimal", 5);

    // `funcToDecimal` accepts whatever `Utilities.isDecimal(s, true)` accepts,
    // which includes an exponent, plus the two boolean words.
    assert_eq!(eval("'1'.toDecimal()"), primitive("decimal", "1"));
    assert_eq!(eval("1.toDecimal()"), primitive("decimal", "1"));
    assert_eq!(eval("'abc'.toDecimal()"), empty());
}

// ---------------------------------------------------------------------------
// path.convert.003 -- `toString`
// ---------------------------------------------------------------------------

// covers: path.convert.003
#[test]
fn to_string_converts_every_convertible_type() {
    expect_group("testToString", 5);

    assert_eq!(eval("1.toString()"), primitive("string", "1"));
    assert_eq!(eval("'1'.toString()"), primitive("string", "1"));
    assert_eq!(eval("true.toString()"), primitive("string", "true"));
    assert_eq!(eval("1.0.toString()"), primitive("string", "1.0"));
    assert_eq!(eval("{}.toString()"), empty());
}

// ---------------------------------------------------------------------------
// path.convert.004 -- `convertsToDate`
// ---------------------------------------------------------------------------

// covers: path.convert.004
#[test]
fn converts_to_date_accepts_year_month_and_day_precision_strings() {
    expect_cases_calling("testTypes", "convertsToDate()", 3);

    for s in ["2015", "2015-02", "2015-02-04"] {
        assert_eq!(
            eval(&format!("'{s}'.convertsToDate()")),
            boolean(true),
            "'{s}' is a date string"
        );
    }

    // And a string that is not a date is false rather than empty: `funcIsDate`
    // matches against a regex and always returns a boolean.
    assert_eq!(eval("'abc'.convertsToDate()"), boolean(false));
    assert_eq!(eval("'2015-02-04T14'.convertsToDate()"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.convert.005 -- `convertsToDateTime`
// ---------------------------------------------------------------------------

// covers: path.convert.005
#[test]
fn converts_to_datetime_accepts_every_precision_and_offset_form() {
    expect_cases_calling("testTypes", "convertsToDateTime()", 9);

    // The nine forms the scenario names, which are the nine cases above.
    for s in [
        "2015",
        "2015-02",
        "2015-02-04",
        "2015-02-04T14",
        "2015-02-04T14:34",
        "2015-02-04T14:34:28",
        "2015-02-04T14:34:28.123",
        "2015-02-04T14:34:28Z",
        "2015-02-04T14:34:28+10:00",
    ] {
        assert_eq!(
            eval(&format!("'{s}'.convertsToDateTime()")),
            boolean(true),
            "'{s}' is a dateTime string"
        );
    }

    assert_eq!(eval("'abc'.convertsToDateTime()"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.convert.006 -- `convertsToTime`
// ---------------------------------------------------------------------------

// covers: path.convert.006
#[test]
fn converts_to_time_accepts_hour_minute_second_and_millisecond_precision() {
    expect_cases_calling("testTypes", "convertsToTime()", 4);

    for s in ["14", "14:34", "14:34:28", "14:34:28.123"] {
        assert_eq!(
            eval(&format!("'{s}'.convertsToTime()")),
            boolean(true),
            "'{s}' is a time string"
        );
    }

    assert_eq!(eval("'abc'.convertsToTime()"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.convert.007 -- `convertsToBoolean`
// ---------------------------------------------------------------------------

// covers: path.convert.007
#[test]
fn converts_to_boolean_accepts_the_engines_accepted_spellings() {
    expect_cases_calling("testTypes", "convertsToBoolean()", 10);

    // `funcIsBoolean`: a boolean is always convertible; an integer is when it
    // is 0 or 1; a decimal when it compares equal to 0 or 1; a string when it
    // is "true" or "false", case folded. Everything else, and any focus that is
    // not a singleton, is false.
    assert_eq!(eval("true.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("false.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("1.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("0.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("1.0.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("'true'.convertsToBoolean()"), boolean(true));
    assert_eq!(eval("'false'.convertsToBoolean()"), boolean(true));
    assert_eq!(
        eval("'False'.convertsToBoolean()"),
        boolean(true),
        "the string comparison is case insensitive"
    );

    // And what it refuses: an out-of-range integer, a negative one, and any
    // other word -- including the ones a lenient reader might accept.
    assert_eq!(eval("2.convertsToBoolean()"), boolean(false));
    assert_eq!(eval("(-1).convertsToBoolean()"), boolean(false));
    assert_eq!(
        eval("'yes'.convertsToBoolean()"),
        boolean(false),
        "`yes` is not an accepted spelling"
    );
    assert_eq!(
        eval("'0'.convertsToBoolean()"),
        boolean(false),
        "the string `0` is not, even though the integer 0 is"
    );
}

// ---------------------------------------------------------------------------
// path.convert.008 -- `convertsToQuantity` and `toQuantity`
// ---------------------------------------------------------------------------

// covers: path.convert.008
#[test]
fn converts_to_quantity_and_to_quantity_convert_strings_and_numbers() {
    expect_cases_calling("testTypes", "convertsToQuantity()", 9);
    expect_cases_calling("testTypes", ".toQuantity()", 11);

    // A bare number becomes a dimensionless quantity, which is the UCUM code
    // `1`; a quoted UCUM code is taken as written; a calendar word is accepted
    // without quotes.
    assert_eq!(eval("1.convertsToQuantity()"), boolean(true));
    assert_eq!(eval("1.0.convertsToQuantity()"), boolean(true));
    assert_eq!(eval("'1'.convertsToQuantity()"), boolean(true));
    assert_eq!(eval("'1 day'.convertsToQuantity()"), boolean(true));
    assert_eq!(eval("'1 \\'wk\\''.convertsToQuantity()"), boolean(true));
    assert_eq!(eval("true.convertsToQuantity()"), boolean(true));

    // And what it refuses: an unquoted non-calendar unit, and a malformed
    // number.
    assert_eq!(eval("'1 wk'.convertsToQuantity()"), boolean(false));
    assert_eq!(eval("'1.a'.convertsToQuantity()"), boolean(false));

    assert_eq!(eval("1.toQuantity() = 1 '1'"), boolean(true));
    assert_eq!(eval("'1 day'.toQuantity() = 1 'd'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.convert.009 -- a failed conversion
// ---------------------------------------------------------------------------

// covers: path.convert.009
#[test]
fn a_failed_conversion_yields_empty_while_its_predicate_yields_false() {
    // The asymmetry the whole requirement rests on: `toX` returns a collection
    // and signals failure by returning nothing, while `convertsToX` returns a
    // boolean and signals failure by returning false. They never both answer
    // the same way.
    assert_eq!(eval("'abc'.toInteger()"), empty());
    assert_eq!(eval("'abc'.convertsToInteger()"), boolean(false));

    // The same for every other pair the R4 engine has in both halves.
    for (to, converts_to) in [
        ("'abc'.toInteger()", "'abc'.convertsToInteger()"),
        ("'abc'.toDecimal()", "'abc'.convertsToDecimal()"),
        ("'abc'.toBoolean()", "'abc'.convertsToBoolean()"),
        ("'abc'.toQuantity()", "'abc'.convertsToQuantity()"),
    ] {
        assert_eq!(eval(to), empty(), "{to} must yield nothing");
        assert_eq!(
            eval(converts_to),
            boolean(false),
            "{converts_to} must be false"
        );
    }

    // A conversion that succeeds yields one item and `true`.
    assert_eq!(eval("'1'.toInteger()"), primitive("integer", "1"));
    assert_eq!(eval("'1'.convertsToInteger()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.convert.010 -- a focus of more than one
// ---------------------------------------------------------------------------

// covers: path.convert.010
#[test]
fn a_conversion_function_on_a_collection_of_more_than_one_is_an_execution_error() {
    // `funcToString` builds one string per item and then raises
    // FHIRPATH_NO_COLLECTION when it has more than one, whose text is
    // "The function {0} can only be used on a singleton value but found {1}".
    let message = expect_evaluation_error("(1 | 2).toString()");
    assert!(
        message.contains("toString"),
        "the diagnostic must name the function: {message:?}"
    );

    // A one-item collection is a singleton and does convert, so the rule is
    // about cardinality.
    assert_eq!(eval("(1 | 1).toString()"), primitive("string", "1"));

    // Whereas the `convertsToX` half answers `false` for a multi-item focus
    // rather than raising -- `funcIsBoolean` and its siblings open with
    // `if (focus.size() != 1) result.add(new BooleanType(false))`.
    assert_eq!(eval("(1 | 2).convertsToBoolean()"), boolean(false));
    assert_eq!(eval("(1 | 2).convertsToInteger()"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.convert.011 -- `toDateTime` on a date
// ---------------------------------------------------------------------------

// covers: path.convert.011
#[test]
fn to_datetime_on_a_date_widens_precision_without_inventing_components() {
    // NOTE FOR REVIEW -- scenario against pinned source.
    //
    // `funcToDateTime` in the pinned R4 engine is a stub: its entire body is
    // `throw makeException(expr, I18nConstants.FHIRPATH_NOT_IMPLEMENTED, "toDateTime")`,
    // with the intended implementation commented out immediately above. The
    // pinned suite has no `.toDateTime()` case at all, which is consistent with
    // that. So the behaviour this scenario describes does not exist upstream:
    // `@2014.toDateTime()` raises "The function toDateTime is not implemented".
    //
    // Written to the scenario as stated, because a test encoding the stub would
    // bury the decision rather than surface it. See the module note.
    assert_eq!(
        eval("@2014.toDateTime()"),
        primitive("dateTime", "2014"),
        "a year-precision date widens to a year-precision dateTime, with no \
         month, day or time invented"
    );
    assert_eq!(
        eval("@2014.toDateTime().precision()"),
        primitive("integer", "4")
    );

    // The same at the other precisions: no component is added that the date did
    // not state.
    assert_eq!(
        eval("@2014-01.toDateTime()"),
        primitive("dateTime", "2014-01")
    );
    assert_eq!(
        eval("@2014-01-05.toDateTime()"),
        primitive("dateTime", "2014-01-05")
    );

    // And the result really is a dateTime rather than the date it came from,
    // which is the conversion's whole effect.
    assert_eq!(eval("@2014.toDateTime() is System.DateTime"), boolean(true));
    assert_eq!(eval("@2014.convertsToDateTime()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.convert.012 -- `toDate` does not exist (DEV-0009)
// ---------------------------------------------------------------------------

// covers: path.convert.012
#[test]
fn to_date_does_not_exist_and_is_a_parse_failure() {
    // DEV-0009, resolved as HAPI's behaviour. The R4 `ExpressionNode.Function`
    // enum has `ConvertsToDate` but no `ToDate`
    // (ExpressionNode.java:51-53), and `Function.fromCode` has a
    // `convertsToDate` entry at :144 with no `toDate` counterpart beside the
    // `toInteger`/`toDecimal`/`toString`/`toQuantity`/`toBoolean`/`toDateTime`/
    // `toTime` entries at :131-137. An unresolvable function name is refused by
    // the parser (FHIRPathEngine.java:1158-1166), so this never reaches
    // evaluation.
    let message = expect_parse_error("'2015'.toDate()");
    assert!(
        message.contains("toDate"),
        "the diagnostic must name the function the table does not have: {message:?}"
    );

    // The asymmetry is observable from one expression pair: the predicate
    // parses and evaluates while the conversion does not parse.
    assert_eq!(eval("'2015'.convertsToDate()"), boolean(true));

    // And the functions that do exist are not refused, so the failure is about
    // `toDate` specifically and not about the `toX` family.
    let mut engine = fresh_engine();
    for expr in [
        "'2015'.convertsToDate()",
        "'1'.toInteger()",
        "'1'.toDecimal()",
        "'1'.toString()",
        "'1'.toQuantity()",
        "'true'.toBoolean()",
    ] {
        engine
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr:?} is in the R4 function table, so it parses: {e}"));
    }
}

// ---------------------------------------------------------------------------
// path.convert.013 -- `toTime`
// ---------------------------------------------------------------------------

// covers: path.convert.013
#[test]
fn to_time_converts_and_yields_empty_on_failure() {
    // NOTE FOR REVIEW -- scenario against pinned source.
    //
    // `funcToTime` is the second of the two stubs: its body is
    // `throw makeException(expr, I18nConstants.FHIRPATH_NOT_IMPLEMENTED, "toTime")`.
    // So in the pinned engine both halves of this scenario raise rather than
    // converting or returning empty, and the pinned suite has no `.toTime()`
    // case. See the module note.
    assert_eq!(
        eval("'14:34:28'.toTime()"),
        primitive("time", "14:34:28"),
        "a time string converts to a single time"
    );
    assert_eq!(
        eval("'abc'.toTime()"),
        empty(),
        "and a failure yields nothing rather than raising"
    );

    // The predicate half, which does exist, agrees about what is convertible.
    assert_eq!(eval("'abc'.convertsToTime()"), boolean(false));
    assert_eq!(eval("'14:34:28'.convertsToTime()"), boolean(true));

    // Every precision `convertsToTime` accepts converts, so the two halves
    // agree on the accepted set and not merely on one example.
    for s in ["14", "14:34", "14:34:28", "14:34:28.123"] {
        assert_eq!(
            eval(&format!("'{s}'.toTime().count()")),
            primitive("integer", "1"),
            "'{s}' converts to one time"
        );
    }
}

// ---------------------------------------------------------------------------
// path.convert.014 -- `toBoolean`
// ---------------------------------------------------------------------------

// covers: path.convert.014
#[test]
fn to_boolean_converts_the_accepted_spellings_and_yields_empty_otherwise() {
    expect_cases_calling("testTypes", ".toBoolean()", 5);

    // What the pinned engine accepts, recorded. `funcToBoolean`'s string arm
    // compares case-insensitively against "true" and "false" and adds nothing
    // for anything else; its integer arm accepts 0 and 1; its decimal arm
    // accepts values comparing equal to 0 or 1.
    assert_eq!(eval("'true'.toBoolean()"), boolean(true));
    assert_eq!(eval("'false'.toBoolean()"), boolean(false));
    assert_eq!(eval("'TRUE'.toBoolean()"), boolean(true));
    assert_eq!(eval("'False'.toBoolean()"), boolean(false));

    // `yes` is not an accepted spelling, so it yields nothing.
    assert_eq!(
        eval("'yes'.toBoolean()"),
        empty(),
        "`yes` is not one of the two words the engine accepts"
    );
    // Nor is the string `0`, even though the integer 0 is accepted.
    assert_eq!(
        eval("'0'.toBoolean()"),
        empty(),
        "the string `0` is not accepted, though the integer 0 is"
    );
    assert_eq!(eval("0.toBoolean()"), boolean(false));
    assert_eq!(eval("1.toBoolean()"), boolean(true));
    assert_eq!(eval("'abc'.toBoolean()"), empty());

    // The accepted set is exactly the one `convertsToBoolean` reports true for
    // (`path.convert.007`): for each of these, one answers a boolean and the
    // other `true`, or one answers nothing and the other `false`. Never a
    // mixture.
    for spelling in [
        "'true'", "'false'", "'TRUE'", "'False'", "'yes'", "'0'", "'abc'", "0", "1", "2", "(-1)",
        "1.0", "true", "false",
    ] {
        let converted = eval(&format!("{spelling}.toBoolean()"));
        let predicate = eval(&format!("{spelling}.convertsToBoolean()"));
        let accepted = !converted.is_empty();
        assert_eq!(
            predicate,
            boolean(accepted),
            "{spelling}: toBoolean() returned {converted:?}, so convertsToBoolean() \
             must report {accepted}"
        );
    }
}
