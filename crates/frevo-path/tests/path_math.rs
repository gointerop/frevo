//! Arithmetic and math functions: `path.math.001`-`016` and
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
//! `73c432b5f43ddbce979978a760ec4aa81e929094`. The digits of `/`, and of `div`
//! and `mod` with a decimal operand (`path.math.005`-`008`, `.015`, `.016`),
//! are `org.fhir.ucum.Decimal`'s: `reference/Ucum-java` at
//! `db9d22ade3f0f4a667b45817b85a9ac4534e4224` (`org.fhir:ucum` 1.0.10; live
//! HAPI 8.12.0 runs 1.0.9, whose `Decimal` is bytecode-identical). Every value
//! asserted for them is one the spec states, each confirmed by compiling that
//! source; none is computed here.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{Document, SuiteInput};
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

/// Assert the group's cases are graded as HAPI's own recorded run grades them
/// (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case except
/// the named ones, which must be `failed` (parity under `path.oracle.012`). A
/// `did-not-run` case satisfies neither, so an unbuilt feature never passes
/// vacuously, and a Frevo pass on a named case is `ahead-of-hapi`, not accepted.
fn expect_group_except(group: &str, expected_cases: usize, failing: &[&str]) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    assert_eq!(
        g.cases.len(),
        expected_cases,
        "the pinned {group} group is {expected_cases} cases; a different count means it          was re-pinned without this test being updated"
    );
    for name in failing {
        assert!(
            g.cases.iter().any(|c| c.address.name == *name),
            "{group}/{name} must be a case of the group"
        );
    }

    let mut problems = Vec::new();
    for case in &g.cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        let want = if failing.iter().any(|n| case.address.name == *n) {
            CaseStatus::Failed
        } else {
            CaseStatus::Passed
        };
        if r.status != want {
            problems.push(format!(
                "{} ({:?}): {} (wanted {}) -- {}",
                case.address,
                case.expression,
                r.status.as_str(),
                want.as_str(),
                r.reason
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {} cases in {group} are not graded as HAPI grades them; the first few:
  {}",
        problems.len(),
        g.cases.len(),
        problems[..problems.len().min(5)].join(
            "
  "
        )
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

/// `expr` must parse and check, then fail at evaluation with a diagnostic;
/// returns the message and the offset. `NotImplemented` panics, so an unbuilt
/// arm is never read as the error the scenario asks for.
fn expect_evaluation_error_at(expr: &str) -> (String, Option<usize>) {
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
        Err(EngineError::Diagnostic { message, offset }) => (message, offset),
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

/// `expr` evaluated over `inp`, after the static check.
fn eval_over(expr: &str, inp: &SuiteInput) -> Vec<ResultItem> {
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
    engine
        .evaluate(parsed, Some(inp), &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

/// A hand-built `Parameters` in XML whose `parameter[i]` carries
/// `valueDecimal` with the i-th text exactly as given. XML keeps a decimal's
/// lexical form, which is what an exponent-form operand needs
/// (`path.math.015`).
fn parameters_with_decimals(name: &str, values: &[&str]) -> SuiteInput {
    let mut body = String::from("<Parameters xmlns=\"http://hl7.org/fhir\">");
    for (i, v) in values.iter().enumerate() {
        body.push_str(&format!(
            "<parameter><name value=\"p{i}\"/><valueDecimal value=\"{v}\"/></parameter>"
        ));
    }
    body.push_str("</Parameters>");
    let document = Document::Xml(
        oracle::xml::parse(&body)
            .unwrap_or_else(|e| panic!("the hand-built Parameters is XML: {e}")),
    );
    SuiteInput {
        file: format!("hand-built-parameters-{name}.xml"),
        bytes: body.into_bytes(),
        document,
    }
}

fn decimal(value: &str) -> Vec<ResultItem> {
    primitive("decimal", value)
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
    expect_group_except("testPlus", 29, &["testPlusDate23", "testPlusDate24"]);

    // The two failed cases carry an offset (`Z`), so `dateAdd`'s
    // `DateTimeType.copy()` (FHIRPathEngine.java:2716) raises `Invalid
    // date/time string (datatype DateTimeType does not support MINUTE
    // precision)` (BaseDateTimeType.java:103-106) and not `Invalid date/time
    // format`. Evaluation fails.
    for expr in [
        "@2015-02-04T10:00Z + 1 hour = @2015-02-04T11:00Z",
        "@2015-02-04T10Z + 1 hour = @2015-02-04T11Z",
    ] {
        let message = expect_evaluation_error(expr);
        assert!(
            message.contains("does not support"),
            "{expr}: offset wording expected, got {message:?}"
        );
        assert!(
            !message.contains("Invalid date/time format"),
            "{expr}: not the no-offset wording, got {message:?}"
        );
    }

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
    expect_group_except("testMinus", 7, &["testMinus7"]);

    assert_eq!(eval("1 - 1"), integer("0"));
    assert_eq!(eval("1.8 - 1.2"), primitive("decimal", "0.6"));

    // Quirks of the pinned engine (`FHIRPathEngine.java:2953`, `:2965-2970`),
    // recorded as DEV-0017 and resolved as HAPI's behaviour: when the left is
    // the number `0` the result is the right quantity made absolute, not
    // negated.
    assert_eq!(eval("(0 - 5 'mg') = 5 'mg'"), boolean(true));
    assert_eq!(eval("(0 - (-5 'mg')) = 5 'mg'"), boolean(true));
    // When the left is a Quantity the branch adds nothing: `q - q` is empty,
    // not a difference.
    assert_eq!(eval("5 'mg' - 3 'mg'"), empty());
    // A Quantity on the right is accepted only when the left is a date,
    // dateTime, instant, Quantity, or a number whose string value is exactly
    // `0`; `5 - 3 'mg'` fails with a wrong-type error.
    let (_, message) = expect_failure("5 - 3 'mg'");
    assert!(
        !message.is_empty(),
        "the wrong-type error must carry a message"
    );
    // `0.0 - 5 'mg'` fails the guard at :2953 (its string value is not `0`)
    // and raises the same right-value wrong-type error (:2954).
    let (_, decimal_message) = expect_failure("0.0 - 5 'mg'");
    assert!(
        !decimal_message.is_empty(),
        "the wrong-type error must carry a message"
    );

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
    expect_group_except("testDivide", 7, &["testDivide7"]);

    // `opDivideBy`'s numeric arm accepts `integer`, `decimal`, `unsignedInt`
    // and `positiveInt` (FHIRPathEngine.java:3004) and always builds
    // `new DecimalType(Decimal.divide(...).asDecimal())` (:3004-3012), so
    // dividing two integers is a decimal, never an integer, and goes through
    // `Decimal.divide` like every other pair (`path.math.015` owns the digits).
    assert_eq!(eval("4 / 2 = 2"), boolean(true));
    assert_eq!(eval("1 / 2 = 0.5"), boolean(true));
    assert_eq!(eval("1 / 2"), decimal("0.5"), "1 / 2 is the decimal 0.5");
    assert_eq!(eval("4 / 2"), decimal("2"));
    assert_eq!(
        eval("(4 / 2) is System.Decimal"),
        boolean(true),
        "two integers that divide exactly still give a decimal"
    );
    assert_eq!(eval("(4 / 2) is System.Integer"), boolean(false));

    // testDivide7 (`0.0 / 0`), failed by HAPI: `Decimal.divide` returns
    // `Decimal.zero()` for a zero dividend before it looks at the divisor
    // (Decimal.java:631-632, the divisor check :634-635). `Decimal.zero()` is
    // `new Decimal("0")` (:236-242), whose `asDecimal()` is `0` (:293-308),
    // and the engine wraps it as `new DecimalType("0")` (:3009): one
    // `decimal` whose string value is exactly `0`.
    assert_eq!(eval("0.0 / 0"), decimal("0"));
    let r = expect_case_status("testDivide", "testDivide7", CaseStatus::Failed);
    assert_eq!(r.expression, "0.0 / 0");
}

/// The case's report, required to carry `want` (HAPI's recorded status).
fn expect_case_status(group: &str, name: &str, want: CaseStatus) -> &'static CaseReport {
    let (suite, report) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    let r = report
        .get(&case.address.to_string())
        .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
    assert_eq!(
        r.status,
        want,
        "{}: {:?} is {} (wanted {}) -- {}",
        r.address,
        r.expression,
        r.status.as_str(),
        want.as_str(),
        r.reason
    );
    r
}

// ---------------------------------------------------------------------------
// path.math.006 -- `div`
// ---------------------------------------------------------------------------

// covers: path.math.006
#[test]
fn div_truncates_toward_zero() {
    expect_group_except("testDiv", 6, &["testDiv6"]);

    // Integer `div` is Java's `/` on two ints (FHIRPathEngine.java:3047-3050),
    // which truncates toward zero rather than flooring, so a negative
    // numerator rounds up.
    assert_eq!(eval("5 div 2"), integer("2"));
    assert_eq!(eval("(-5) div 2"), integer("-2"));
    assert_eq!(eval("2.2 div 1.8 = 1"), boolean(true));
    assert_eq!(
        eval("(2.2 div 1.8) is System.Integer"),
        boolean(true),
        "the decimal arm produces an IntegerType too (FHIRPathEngine.java:3055-3060)"
    );

    // testDiv6 (`0 div 0.0`), failed by HAPI: the zero-dividend rule of
    // path.math.005. `Decimal.divInt` truncates the quotient `Decimal.zero()`
    // (Decimal.java:785-790, `trunc` :338-351) to `0`, and the engine wraps
    // that string as `new IntegerType("0")` (FHIRPathEngine.java:3060): one
    // `integer` `0`.
    assert_eq!(eval("0 div 0.0"), integer("0"));
    expect_case_status("testDiv", "testDiv6", CaseStatus::Failed);
}

// ---------------------------------------------------------------------------
// path.math.007 -- `mod`
// ---------------------------------------------------------------------------

// covers: path.math.007
#[test]
fn mod_returns_the_remainder() {
    expect_group_except("testMod", 8, &["testMod6", "testMod7", "testMod8"]);

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

    // testMod6 and testMod7, failed by HAPI: `Decimal.modulo` calls `divInt`,
    // which calls `divide` (Decimal.java:785-798), which throws
    // `"Attempt to divide "+toString()+" by zero"` for a non-zero dividend and
    // a zero divisor (:634-635; `toString()` is `asDecimal()`, :221-224).
    // `opMod` rethrows it as `new PathEngineException(e)`
    // (FHIRPathEngine.java:3098-3105), whose message is the cause's
    // `toString()` (PathEngineException.java:95-97) with no location suffix
    // and so no offset (path.errors.002). The texts are exactly HAPI's run's
    // (hapi-8.12.0-results.json:857-858).
    for (expr, want) in [
        (
            "5.5 mod 0",
            "org.fhir.ucum.UcumException: Attempt to divide 5.5 by zero",
        ),
        (
            "5 mod 0.0",
            "org.fhir.ucum.UcumException: Attempt to divide 5 by zero",
        ),
    ] {
        let (message, offset) = expect_evaluation_error_at(expr);
        assert_eq!(message, want, "{expr}: HAPI's message, exactly");
        assert_eq!(
            offset, None,
            "{expr}: `new PathEngineException(e)` has no location"
        );
    }
    for name in ["testMod6", "testMod7"] {
        let r = expect_case_status("testMod", name, CaseStatus::Failed);
        assert!(
            r.reason
                .contains("org.fhir.ucum.UcumException: Attempt to divide"),
            "testMod/{name} must fail with HAPI's message: {}",
            r.reason
        );
    }

    // testMod8 (`0.0 mod 0`), failed by HAPI: one `decimal` whose string value
    // is exactly `-0.0`, not `0`. `modulo` is `this - trunc(this / other) *
    // other` (Decimal.java:792-798): the quotient is `Decimal.zero()`
    // (:631-632), the product is `Decimal.zero()` (:539-540), and `subtract`
    // of two non-negative operands computes `other.doSubtract(this)` and flips
    // its sign (:381-384); `asDecimal` writes the `-` because its all-zeros
    // test counts the `.` (:157-164, :305-306); `new DecimalType("-0.0")`
    // keeps that text (DecimalType.java:153-161, :199-202).
    assert_eq!(eval("0.0 mod 0"), decimal("-0.0"));
    expect_case_status("testMod", "testMod8", CaseStatus::Failed);
    // Under path.equality.001's rule `-0` is not `0` after
    // `removeTrailingZeros`, so the negative zero is observable.
    assert_eq!(eval("(0.0 mod 0) = 0"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.math.008 -- division by zero
// ---------------------------------------------------------------------------

// covers: path.math.008
#[test]
fn division_by_zero_yields_empty_only_where_the_pinned_engine_says_so() {
    // Three different mechanisms reach the same answer: `opDivideBy` catches
    // the UcumException `Decimal.divide` raises and returns nothing
    // (FHIRPathEngine.java:3009-3011), while `opDiv` and `opMod` guard an
    // integer divisor and add nothing to the result (:3051-3054, :3091-3094).
    assert_eq!(eval("1 / 0"), empty());
    assert_eq!(eval("1 div 0"), empty());
    assert_eq!(eval("1 mod 0"), empty());

    // Integer divisors: also `5 div 0` and `5 mod 0` (:3051-3054, :3091-3094).
    assert_eq!(eval("5 div 0"), empty());
    assert_eq!(eval("5 mod 0"), empty());
    assert_eq!(eval("(1 / 0).empty()"), boolean(true));

    // `0.0 / 0`, `0 div 0.0` and `0.0 mod 0` yield ONE item, not an empty
    // collection (testDivide7, testDiv6, testMod8, recorded `failed` by HAPI,
    // DEV-0017), because `Decimal.divide` answers `Decimal.zero()` for a zero
    // dividend before it checks the divisor (Decimal.java:631-635). The values
    // are derived in path.math.005, .006 and .007.
    assert_eq!(eval("0.0 / 0"), decimal("0"));
    assert_eq!(eval("0 div 0.0"), integer("0"));
    assert_eq!(eval("0.0 mod 0"), decimal("-0.0"));

    // The same zero-dividend rule: the dividend's scale is not kept, the
    // integer `div` arm still guards its divisor, and the `mod` zero is
    // negative exactly when its text has a `.` (path.math.016).
    assert_eq!(eval("0 / 5"), decimal("0"));
    assert_eq!(eval("0.00 / 5"), decimal("0"));
    assert_eq!(eval("0 / 0.0"), decimal("0"));
    assert_eq!(eval("0 div 0"), empty(), "the integer arm, :3051-3054");
    assert_eq!(eval("0 mod 2.5"), decimal("0"));
    assert_eq!(eval("0 mod 0.0"), decimal("0"));
    assert_eq!(eval("0.0 mod 2"), decimal("-0.0"));

    // `5.5 mod 0` and `5 mod 0.0` fail with an execution error (testMod6,
    // testMod7, recorded `failed` by HAPI), with path.math.007's messages.
    assert_eq!(
        expect_evaluation_error("5.5 mod 0"),
        "org.fhir.ucum.UcumException: Attempt to divide 5.5 by zero"
    );
    assert_eq!(
        expect_evaluation_error("5 mod 0.0"),
        "org.fhir.ucum.UcumException: Attempt to divide 5 by zero"
    );

    // A non-zero decimal or mixed dividend over a zero divisor is EMPTY under
    // `/` and `div`: both arms catch the UcumException and add nothing
    // (FHIRPathEngine.java:3009-3011, :3060-3063).
    for expr in ["5 / 0.0", "5.5 / 0", "5 div 0.0", "5.5 div 0"] {
        assert_eq!(eval(expr), empty(), "{expr}");
    }

    // The suite cases at the status HAPI records: `failed` for the five the
    // spec names, `passed` for their neighbours.
    for (group, name, want) in [
        ("testDivide", "testDivide6", CaseStatus::Passed),
        ("testDivide", "testDivide7", CaseStatus::Failed),
        ("testDiv", "testDiv5", CaseStatus::Passed),
        ("testDiv", "testDiv6", CaseStatus::Failed),
        ("testMod", "testMod5", CaseStatus::Passed),
        ("testMod", "testMod6", CaseStatus::Failed),
        ("testMod", "testMod7", CaseStatus::Failed),
        ("testMod", "testMod8", CaseStatus::Failed),
    ] {
        let (suite, report) = pinned();
        let case = suite
            .case(group, name)
            .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        assert_eq!(
            r.status,
            want,
            "{}: {:?} is {} -- {}",
            r.address,
            r.expression,
            r.status.as_str(),
            r.reason
        );
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

    // And the suite's own date-arithmetic cases, graded as HAPI ran them
    // (`hapi-8.12.0-results.json`): `testMinus5` and `testMinus6` pass, while
    // `testMinus7` (`@2015-02-04T10:00Z - 1 day = ...`) is `failed`, as are
    // `testPlusDate23` and `testPlusDate24`, with the message
    // `Invalid date/time string (datatype DateTimeType does not support MINUTE
    // precision)` (DEV-0017).
    expect_case("testMinus", "testMinus5");
    for (group, name) in [
        ("testMinus", "testMinus7"),
        ("testPlus", "testPlusDate23"),
        ("testPlus", "testPlusDate24"),
    ] {
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
            "{}: {:?} is {} -- {}",
            r.address,
            r.expression,
            r.status.as_str(),
            r.reason
        );
    }
    let message = expect_evaluation_error("@2015-02-04T10:00Z - 1 day = @2015-02-03T10:00Z");
    assert!(
        message.contains("does not support MINUTE precision"),
        "testMinus7: offset wording expected, got {message:?}"
    );
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
// path.math.015 -- `/` rounds to the significant digits its operands carry
// ---------------------------------------------------------------------------

/// `(dividend, divisor, quotient)`: the acceptance table path.math.015 states,
/// row for row, grouped by the rule that places each value
/// (`Decimal.divide`, Decimal.java:627-769).
const DIVIDE_TABLE: &[(&str, &str, &str)] = &[
    // Both whole and the long division ends within max(digits) + 40 digits
    // (:638, :670, :715-722): the exact quotient, trailing zeros dropped.
    ("1", "2", "0.5"),
    ("1", "8", "0.125"),
    ("3", "8", "0.375"),
    ("1", "64", "0.015625"),
    ("1", "1024", "0.0009765625"),
    ("7", "2", "3.5"),
    ("19", "20", "0.95"),
    ("123456789", "1000", "123456.789"),
    ("4", "2", "2"),
    // Both whole, non-terminating: P is the larger operand digit count
    // (:724-731), rounding half up on the first dropped digit (:732-759).
    ("1", "3", "0.3"),
    ("2", "3", "0.7"),
    ("5", "6", "0.8"),
    ("1", "7", "0.1"),
    ("1", "9", "0.1"),
    ("5", "9", "0.6"),
    ("1", "6", "0.2"),
    ("10", "3", "3.3"),
    ("20", "3", "6.7"),
    ("22", "7", "3.1"),
    ("100", "3", "33.3"),
    ("200", "3", "66.7"),
    ("1000", "3", "333.3"),
    ("1", "11", "0.091"),
    ("1", "30", "0.033"),
    ("1", "300", "0.00333"),
    // Only the dividend whole: P is the larger of the divisor's precision and
    // the quotient's integer-digit count.
    ("1", "3.0", "0.33"),
    ("2", "3.0", "0.67"),
    ("2", "3.00", "0.667"),
    ("10", "3.0", "3.3"),
    ("1", "8.0", "0.13"),
    ("1", "0.3", "3"),
    ("1", "0.9", "1"),
    ("8", "0.9", "9"),
    ("2", "0.6", "3"),
    ("3", "1.5", "2.0"),
    ("1", "0.125", "8.0"),
    // Only the divisor whole: P is the larger of the dividend's precision and
    // the quotient's integer-digit count.
    ("1.0", "3", "0.33"),
    ("1.00", "3", "0.333"),
    ("10.0", "3", "3.33"),
    ("10.00", "3", "3.333"),
    ("0.1", "3", "0.03"),
    ("0.10", "3", "0.033"),
    ("1.000", "7", "0.1429"),
    ("1.0", "8", "0.13"),
    ("1.0", "16", "0.063"),
    ("1.5", "3", "0.50"),
    ("1.9", "2", "0.95"),
    ("10.0", "4", "2.5"),
    ("9.99", "3", "3.33"),
    // Neither whole: P is the larger of the two precisions' minimum and the
    // quotient's integer-digit count. A division that stops short of P keeps
    // the digits the loop produced (:670-711): `4.0 / 2.0` is `2.0`.
    ("1.2", "1.8", "0.67"),
    ("4.0", "2.0", "2.0"),
    ("10.00", "3.0", "3.3"),
    ("0.5", "0.3", "2"),
    ("1.5", "0.5", "3"),
    ("7.5", "2.5", "3.0"),
    ("1.25", "0.5", "3"),
    // The rounding-carry defect: a carry out of the leading digit prepends
    // `1` AND increments the exponent counter, so the result is one tenth of
    // the correctly rounded value (:751-754, :764).
    ("1.9", "0.2", "1.0"),
    ("0.95", "0.1", "1.0"),
    ("3.8", "0.4", "1.0"),
    ("9.99", "1.0", "1.00"),
    ("99.9", "1.0", "10.0"),
    ("1.99", "2.0", "0.100"),
    // Nothing is rounded here, so no carry.
    ("19", "2.0", "9.5"),
    ("1.9", "0.20", "9.5"),
    ("95", "10.0", "9.5"),
];

// covers: path.math.015
#[test]
fn division_over_numbers_rounds_the_quotient_to_the_significant_digits_its_operands_carry() {
    // `opDivideBy` is
    // `new DecimalType(new Decimal(l.primitiveValue()).divide(new Decimal(r.primitiveValue())).asDecimal())`
    // (FHIRPathEngine.java:3004-3012). No live case grades the rounding (every
    // testDivide case terminates except testDivide5, whose `round(2)` hides
    // the digits), so the table is graded in-process against the values the
    // spec states. DEV-0030 records the rounding and the carry defect, and
    // DEBT-0020 the wrong answers.
    let mut wrong = Vec::new();
    for &(a, b, want) in DIVIDE_TABLE {
        let expr = format!("{a} / {b}");
        let got = eval(&expr);
        if got != decimal(want) {
            wrong.push(format!("{expr}: wanted decimal {want}, got {got:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} rows of path.math.015's table differ:\n  {}",
        wrong.len(),
        DIVIDE_TABLE.len(),
        wrong.join("\n  ")
    );

    // The sign is negative exactly when one operand is negative, applied
    // after rounding (:765).
    assert_eq!(eval("(0 - 1) / 3"), decimal("-0.3"));
    assert_eq!(eval("(0 - 2) / 3"), decimal("-0.7"));

    // An operand read from data in exponent form is read lower-cased through
    // `setValueScientific` (:76-83, :182-208), with its mantissa's precision.
    let sci = parameters_with_decimals("exponent-form", &["1e3", "1.5e-1"]);
    assert_eq!(
        eval_over("Parameters.parameter[0].value / 3", &sci),
        decimal("300")
    );
    assert_eq!(
        eval_over("Parameters.parameter[1].value / 3", &sci),
        decimal("0.050")
    );

    // The string value is observable through `toString()` and through
    // equality (path.equality.001's string rule).
    assert_eq!(eval("(1 / 3).toString()"), primitive("string", "0.3"));
    assert_eq!(eval("1 / 3 = 0.3"), boolean(true));
    assert_eq!(eval("1 / 3 = 0.333"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.math.016 -- `div` and `mod` with a decimal operand
// ---------------------------------------------------------------------------

// covers: path.math.016
#[test]
fn div_and_mod_with_a_decimal_operand_truncate_the_rounded_quotient() {
    // `div` with a decimal operand is `new IntegerType(Decimal.divInt(...))`:
    // `trunc` of path.math.015's rounded quotient (Decimal.java:785-790,
    // `trunc` :338-351; FHIRPathEngine.java:3060). Both `integer` is the Java
    // `int` arm (:3050-3054), which path.math.006 covers.
    for (expr, want) in [
        ("2.2 div 1.8", "1"),
        ("7.5 div 2", "3"),
        ("(0 - 7.5) div 2", "-3"),
        ("5.5 div 2", "2"),
        ("6.0 div 2", "3"),
        ("10 div 3.0", "3"),
        ("1 div 0.3", "3"),
        ("10 div 0.3", "33"),
        ("0.5 div 1", "0"),
        ("1 div 3.0", "0"),
        // The carry defect reaches `div`: the quotient is rounded first.
        ("1.9 div 0.2", "1"),
        ("0.95 div 0.1", "1"),
    ] {
        assert_eq!(eval(expr), integer(want), "{expr}");
    }

    // A truncated quotient outside the Java `int` range fails:
    // `new IntegerType(String)` parses with `Integer.parseInt` and rethrows as
    // `IllegalArgumentException(e)` (IntegerType.java:65-67, :86-92), whose
    // message is the cause's `toString()`; the engine catches only
    // UcumException (:3061).
    assert_eq!(
        expect_evaluation_error("3000000000.0 div 1"),
        "java.lang.NumberFormatException: For input string: \"3000000000\""
    );

    // `mod` with a decimal operand is `this - trunc(this / other) * other`
    // computed by `Decimal.subtract` and `Decimal.multiply`
    // (Decimal.java:792-798, :368-386, :535-625).
    for (expr, want) in [
        ("2.2 mod 1.8", "0.4"),
        ("5.5 mod 2", "1.5"),
        ("(0 - 5.5) mod 2", "-1.5"),
        ("5.5 mod (0 - 2)", "1.5"),
        ("7 mod 2.5", "2.0"),
        ("10 mod 3.0", "1.0"),
        ("1 mod 0.3", "0.1"),
        // The carry defect: trunc(1.9 / 0.2) is 1, so 1.9 - 0.2 is 1.7.
        ("1.9 mod 0.2", "1.7"),
        ("0 mod 2.5", "0"),
    ] {
        assert_eq!(eval(expr), decimal(want), "{expr}");
    }

    // A zero remainder is a negative zero whatever the operands' signs (every
    // sign path of `Decimal.subtract` leaves it negative, :368-386, :439-486);
    // `asDecimal` writes the `-` only when the zero's text contains a `.`
    // (:305-306, :157-164). The zero has as many fractional `0`s as the wider
    // fractional part of the dividend and of the trimmed product (:535-625).
    for expr in [
        "4.0 mod 2",
        "6.0 mod 2",
        "0.0 mod 2",
        "(0 - 4.0) mod 2",
        "4.0 mod (0 - 2)",
        "4 mod 2.0",
        "6 mod 3.0",
        "(0 - 6) mod 3.0",
        "4.0 mod 2.0",
        "5 mod 2.5",
        "7.5 mod 2.5",
        "10.0 mod 2.5",
        "3000000000.0 mod 1",
    ] {
        assert_eq!(eval(expr), decimal("-0.0"), "{expr}");
    }
    for expr in ["4.00 mod 2", "4 mod 2.00", "1.25 mod 0.25"] {
        assert_eq!(eval(expr), decimal("-0.00"), "{expr}");
    }
    // The products `10` and `12` are trimmed to whole numbers, so the zero's
    // text has no `.` and no `-` is written.
    for expr in ["10 mod 2.5", "12 mod 1.5"] {
        assert_eq!(eval(expr), decimal("0"), "{expr}");
    }

    // A zero divisor is path.math.008's: empty under `div`, an execution error
    // under `mod`.
    assert_eq!(eval("5.5 div 0"), empty());
    assert_eq!(
        expect_evaluation_error("5.5 mod 0"),
        "org.fhir.ucum.UcumException: Attempt to divide 5.5 by zero"
    );

    // Only testDiv4 and testMod4 grade this in the suite, both passed by HAPI.
    expect_case("testDiv", "testDiv4");
    expect_case("testMod", "testMod4");
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

    // Decimal rendering, differential-only: written against HAPI's recorded
    // expectations (`DecimalType.encode`, DecimalType.java:101-102, which is
    // `BigDecimal.toString`), not against any suite case. Plain digits when the
    // scale is zero or positive and the adjusted exponent is at least -6,
    // otherwise scientific with a signed exponent. `round` with a negative
    // digit count uses `setScale` (FHIRPathEngine.java:4412).
    assert_eq!(eval("12345.round(-2)"), primitive("decimal", "1.23E+4"));
    assert_eq!(eval("1234.5.round(-3)"), primitive("decimal", "1E+3"));
    // The `double`-based functions build the result with
    // `BigDecimal.valueOf(double)` (DecimalType.java:66-69), which parses
    // `Double.toString`. A pinned-engine rendering, not the specification's.
    assert_eq!(eval("10000000000.abs()"), primitive("decimal", "1.0E+10"));
    assert_eq!(eval("0.0000001.abs()"), primitive("decimal", "1.0E-7"));
    assert_eq!(eval("0.0001.abs()"), primitive("decimal", "0.00010"));
    assert_eq!(eval("12345678.abs()"), primitive("decimal", "12345678"));
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
    // Only `{}.exp()` is empty: `funcExp` guards an empty focus
    // (FHIRPathEngine.java:4177-4179), and it is the one the suite grades
    // (`testExp/testExp3`, `{}.exp().empty() = true`).
    assert_eq!(eval("{}.exp()"), empty());
    expect_case("testExp", "testExp3");

    // The other nine raise with the check skipped: each opens with
    // `if (focus.size() != 1) throw makeExceptionPlural(...)`, which covers
    // zero (funcAbs :4116, funcCeiling :4139, funcFloor :4157, funcSqrt :4096,
    // funcLn :4201, funcLog :4221, funcPower :4249, funcRound :4397,
    // funcTruncate :4273). The specification says empty; HAPI raises
    // (DEV-0019). No live case grades it, so it is differential-only.
    for expr in [
        "{}.abs()",
        "{}.ceiling()",
        "{}.floor()",
        "{}.sqrt()",
        "{}.ln()",
        "{}.log(2)",
        "{}.power(2)",
        "{}.round(2)",
        "{}.truncate()",
    ] {
        let mut engine = fresh_engine();
        let context = CaseContext::default();
        let parsed = engine
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
        match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
            Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
            Err(EngineError::Diagnostic { .. }) => {}
            Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
        }
    }
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
