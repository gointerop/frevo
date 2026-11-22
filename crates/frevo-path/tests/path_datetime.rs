//! Date and time functions: `path.datetime.001`-`003` and `005`-`007`.
//!
//! ```text
//! cargo test -p frevo-path --test path_datetime
//! ```
//!
//! `path.datetime.004` -- the configured time zone -- is covered by the
//! committed oracle harness in `fhirpath_oracle.rs` and is deliberately not
//! duplicated here.
//!
//! Provenance: `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`: `funcToday` (:4002-4006),
//! `funcNow`, `funcPrecision` (:4378-4394) and
//! `BaseDateTimeType.equalsUsingFhirPathRules`, plus
//! `org.hl7.fhir.utilities.DateTimeUtil.getDatePrecision` /
//! `getTimePrecision`, whose definition is
//! "strip the separators and count the digits".

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;
use std::time::Duration;

use oracle::engine::{CaseContext, EngineConfig, StrictnessFlags, SuiteEngine, engine_under_test};
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
        "{} of {} cases in {group} did not pass:\n  {}",
        failures.len(),
        g.cases.len(),
        failures.join("\n  ")
    );
}

/// The expressions a group's cases carry, so a scenario can state what the
/// group actually checks rather than only that it passes.
fn group_expressions(group: &str) -> Vec<&'static str> {
    let (suite, _) = pinned();
    suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"))
        .cases
        .iter()
        .map(|c| c.expression.as_str())
        .collect()
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

/// The one item of a one-item result, or a failure naming what came back.
fn sole(expr: &str) -> ResultItem {
    let items = eval(expr);
    assert_eq!(
        items.len(),
        1,
        "{expr:?} must yield exactly one item, not {items:?}"
    );
    items.into_iter().next().expect("one item")
}

// ---------------------------------------------------------------------------
// path.datetime.001 -- `today()`
// ---------------------------------------------------------------------------

// covers: path.datetime.001
#[test]
fn today_returns_the_current_date() {
    expect_group("testToday", 2);

    // What the group checks, so the scenario is readable without the oracle
    // file: that `today()` is after a 1974 birth date, and that its string form
    // is ten characters.
    let exprs = group_expressions("testToday");
    assert!(
        exprs.iter().any(|e| e.contains("Patient.birthDate")),
        "the group must still compare today() against a resource date: {exprs:?}"
    );
    assert!(
        exprs
            .iter()
            .any(|e| e.contains("today().toString().length()")),
        "and still check the rendered length: {exprs:?}"
    );

    // `funcToday` builds `new DateType(new Date(), TemporalPrecisionEnum.DAY)`,
    // so the result is a `date` and not a `dateTime`, and its rendering is
    // exactly `YYYY-MM-DD`.
    let item = sole("today()");
    match &item {
        ResultItem::Primitive { fhir_type, value } => {
            assert_eq!(fhir_type, "date", "today() must be a date");
            assert_eq!(value.len(), 10, "today() renders as YYYY-MM-DD: {value:?}");
            let parts: Vec<&str> = value.split('-').collect();
            assert_eq!(parts.len(), 3, "{value:?}");
            assert_eq!(parts[0].len(), 4, "{value:?}");
            assert!(
                parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())),
                "every component must be digits: {value:?}"
            );
        }
        other => panic!("today() must be a primitive date, not {other:?}"),
    }

    assert_eq!(eval("today().toString().length() = 10"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.datetime.002 -- `now()`
// ---------------------------------------------------------------------------

// covers: path.datetime.002
#[test]
fn now_returns_the_current_datetime() {
    expect_group("testNow", 2);

    let exprs = group_expressions("testNow");
    assert!(
        exprs.iter().any(|e| e.contains("Patient.birthDate")),
        "the group must still compare now() against a resource date: {exprs:?}"
    );

    // `now()` is a `dateTime`, not a `date`, and it carries more than a date's
    // ten characters -- which is what the group's `length() > 10` checks.
    let item = sole("now()");
    match &item {
        ResultItem::Primitive { fhir_type, value } => {
            assert_eq!(fhir_type, "dateTime", "now() must be a dateTime");
            assert!(
                value.len() > 10,
                "now() must carry a time as well as a date: {value:?}"
            );
            assert!(
                value.contains('T'),
                "and the two are separated by a T: {value:?}"
            );
        }
        other => panic!("now() must be a primitive dateTime, not {other:?}"),
    }

    assert_eq!(eval("now().toString().length() > 10"), boolean(true));

    // And it is after a date in the past, which is the other half of the group.
    assert_eq!(eval("@1974-12-25 < now()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.datetime.003 -- `now()` is read afresh (DEV-0008)
// ---------------------------------------------------------------------------

// covers: path.datetime.003
#[test]
fn now_is_read_afresh_on_every_call_and_no_evaluation_instant_is_cached() {
    // DEV-0008, resolved as HAPI's behaviour. `funcNow` reads the clock on each
    // call and the engine holds no per-evaluation instant, so the FHIRPath
    // specification's requirement that `now()` be consistent within one
    // evaluation does not hold. No case in the pinned suite grades it --
    // `testNow` only checks an ordering and a string length.
    //
    // Within one expression the difference is below the resolution of a
    // wall-clock test, so what is asserted here is what can be: the comparison
    // answers a single boolean -- never empty, never an error -- whichever of
    // the two instants is the later.
    let items = eval("now() = now()");
    assert_eq!(
        items.len(),
        1,
        "`now() = now()` must answer one item, not {items:?}"
    );
    match &items[0] {
        ResultItem::Primitive { fhir_type, value } => {
            assert_eq!(fhir_type, "boolean");
            assert!(
                value == "true" || value == "false",
                "the answer is whatever comparing two separately read instants \
                 gives, but it is a boolean: {value:?}"
            );
        }
        other => panic!("`now() = now()` must answer a boolean, not {other:?}"),
    }

    // Across evaluations the absence of a cache is directly observable: one
    // parsed expression, evaluated twice with real time in between, gives two
    // different instants. An engine that cached an evaluation instant on the
    // engine rather than reading the clock would return the same value twice.
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = engine
        .parse("now()")
        .unwrap_or_else(|e| panic!("`now()` must parse: {e}"));
    let first = engine
        .evaluate(parsed, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("the first evaluation must succeed: {e}"));
    std::thread::sleep(Duration::from_millis(50));
    let second = engine
        .evaluate(parsed, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("the second evaluation must succeed: {e}"));
    assert_ne!(
        first, second,
        "`now()` must be read afresh: two evaluations 50 ms apart cannot return \
         the same millisecond-precision instant"
    );

    // `today()` is the contrast: it is also read afresh, but at day precision,
    // so two evaluations moments apart do agree. That makes the difference
    // above a statement about the resolution and not about the engine
    // re-reading the clock only sometimes.
    let today = engine
        .parse("today()")
        .unwrap_or_else(|e| panic!("`today()` must parse: {e}"));
    let d1 = engine
        .evaluate(today, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{e}"));
    let d2 = engine
        .evaluate(today, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// path.datetime.005 -- `precision()`
// ---------------------------------------------------------------------------

// covers: path.datetime.005
#[test]
fn precision_reports_the_component_count_of_a_date_datetime_or_time() {
    expect_group("Precision", 5);

    // `funcPrecision` sends a decimal to `Utilities.getDecimalPrecision`, a
    // date or dateTime to `DateTimeUtil.getDatePrecision` and a time to
    // `getTimePrecision`. The two temporal functions strip the separators and
    // count the digits, so the number is a digit count and not a count of
    // components: a year is 4, a month 6, a day 8, a millisecond dateTime 17.
    assert_eq!(eval("@2014.precision()"), integer("4"));
    assert_eq!(eval("@2014-01.precision()"), integer("6"));
    assert_eq!(eval("@2014-01-05.precision()"), integer("8"));
    assert_eq!(eval("@2014-01-05T10:30:00.000.precision()"), integer("17"));

    // A time drops the `T` as well as the separators, so a minute-precision
    // time is 4 and a millisecond one 9.
    assert_eq!(eval("@T10:30.precision()"), integer("4"));
    assert_eq!(eval("@T10:30:00.000.precision()"), integer("9"));

    // And a decimal's precision is its significant-digit count, which is the
    // third arm of the same function (`path.literal.006`).
    assert_eq!(eval("1.58700.precision()"), integer("5"));
}

// ---------------------------------------------------------------------------
// path.datetime.006 -- an offset against UTC
// ---------------------------------------------------------------------------

// covers: path.datetime.006
#[test]
fn a_datetime_with_an_offset_and_one_in_utc_compare_correctly() {
    // What the pinned engine computes: `true`.
    //
    // `datesEqual` calls `BaseDateTimeType.equalsUsingFhirPathRules`. Both
    // sides have a timezone, so it takes the else branch, and both are at
    // second precision -- past DAY -- so each is converted with
    // `setTimeZoneZulu(true)` before `compareTimes`
    // (BaseDateTimeType.java, `equalsUsingFhirPathRules`). 10:30+10:00 and
    // 00:30Z are the same instant, so the comparison is 0 and the answer is
    // true.
    assert_eq!(
        eval("@2014-01-05T10:30:00+10:00 = @2014-01-05T00:30:00Z"),
        boolean(true)
    );
    // Symmetric.
    assert_eq!(
        eval("@2014-01-05T00:30:00Z = @2014-01-05T10:30:00+10:00"),
        boolean(true)
    );

    // Two instants that are genuinely different are unequal, so the offset is
    // being applied and not ignored.
    assert_eq!(
        eval("@2014-01-05T10:30:00+10:00 = @2014-01-05T10:30:00Z"),
        boolean(false)
    );

    // And the ordering uses the same conversion: the offset form is earlier
    // than a UTC instant an hour later.
    assert_eq!(
        eval("@2014-01-05T10:30:00+10:00 < @2014-01-05T01:30:00Z"),
        boolean(true)
    );

    // Crossing a day boundary, where ignoring the offset would give the
    // opposite answer.
    assert_eq!(
        eval("@2014-01-05T01:00:00+10:00 = @2014-01-04T15:00:00Z"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.datetime.007 -- `today()` has day precision
// ---------------------------------------------------------------------------

// covers: path.datetime.007
#[test]
fn today_has_day_precision() {
    // `funcToday` passes `TemporalPrecisionEnum.DAY` explicitly, so the value
    // renders as eight digits once the hyphens are stripped. 8 and not 10: the
    // separators do not count, which is the same rule `path.datetime.005`
    // records for a literal.
    assert_eq!(eval("today().precision()"), integer("8"));

    // Which is the same precision a day-precision literal reports, so the
    // function builds the same kind of value a literal does.
    assert_eq!(eval("@2014-01-05.precision()"), integer("8"));

    // And it is not a dateTime precision: 8, not 17.
    assert_ne!(eval("today().precision()"), integer("17"));

    // The rendered length is 10, which is the suite's `testToday2`, and the two
    // numbers differ by the two hyphens.
    assert_eq!(eval("today().toString().length() = 10"), boolean(true));
}
