//! Ordered comparison and equality: `path.compare.001`-`012` and
//! `path.equality.001`-`012`.
//!
//! ```text
//! cargo test -p frevo-path --test path_compare_equality
//! ```
//!
//! The two families share a file because they share the question the suite
//! cannot answer: which operand types each operator accepts. The suite's
//! `testLessThan`, `testLessOrEqual`, `testGreaterThan` and
//! `testGreatorOrEqual` groups are 122 cases between them and not one of them
//! compares a `positiveInt` or an `unsignedInt`, so the asymmetry recorded as
//! `DEV-0003` is invisible to it. Four of the tests here reach for resources
//! outside the suite's own input list for exactly that reason.
//!
//! The deviations this file pins, each resolved as HAPI's behaviour:
//!
//! - `DEV-0002` -- a multi-item operand yields empty rather than an error.
//! - `DEV-0003` -- `<` accepts a narrower numeric type set than the other three.
//! - `DEV-0004` -- all four comparisons go through `Double.valueOf`.
//!
//! Provenance: `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, `opLessThan` (:2409-2454),
//! `opGreater` (:2456-2498), `opLessOrEqual` (:2500-2546),
//! `opGreaterOrEqual` (:2548-2594), `opEquals` (:2140-2168),
//! `opEquivalent` (:2111-2131), `doEquals` (:2189-2202) and
//! `BaseDateTimeType.compareTimes` / `equalsUsingFhirPathRules`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseReport, CaseStatus, RunReport};
use oracle::suite::Suite;
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

/// Assert every case in a suite group passed, and that the group still has the
/// size the scenario was written against.
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

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be evaluated over: {e}")
    })
}

/// Evaluate over an optional resource, through all three stages.
fn eval_over(expr: &str, input: Option<&SuiteInput>) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let resource_type = input.and_then(|i| i.resource_type());
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
        .evaluate(parsed, input, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

fn eval(expr: &str) -> Vec<ResultItem> {
    eval_over(expr, None)
}

/// `expr` must parse and check, then fail at evaluation with a diagnostic.
fn expect_evaluation_error_over(expr: &str, input: Option<&SuiteInput>) -> String {
    let mut engine = fresh_engine();
    let resource_type = input.and_then(|i| i.resource_type());
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
    match engine.evaluate(parsed, input, &context, StrictnessFlags::EVALUATE) {
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

fn decimal(value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: "decimal".to_string(),
        value: value.to_string(),
    }]
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

/// The four ordered comparison operators.
const ORDERED: [&str; 4] = ["<", "<=", ">", ">="];

// ---------------------------------------------------------------------------
// path.compare.001 -- `<` accepts a narrower numeric type set
// ---------------------------------------------------------------------------

/// The twelve string-like FHIR types all four comparisons accept:
/// `FHIRPathEngine.FHIR_TYPES_STRING` at FHIRPathEngine.java:2407.
const FHIR_TYPES_STRING: [&str; 12] = [
    "string",
    "uri",
    "code",
    "oid",
    "id",
    "uuid",
    "sid",
    "markdown",
    "base64Binary",
    "canonical",
    "url",
    "xhtml",
];

// covers: path.compare.001
#[test]
fn the_less_than_operator_accepts_a_narrower_numeric_type_set_than_the_other_three() {
    // NOTE FOR REVIEW: one of these 33, `testLessThan/testLessThan33`, cannot
    // pass against the pinned R4 engine. `opLessThan` identifies a quantity's
    // unit by its `code` child while its three siblings use `unit`, and the
    // case's left operand has unit text and no code -- so the units are judged
    // to differ, `qtyToCanonicalDecimal` returns null for a quantity with no
    // UCUM system (FHIRPathEngine.java:2301-2303), and the null is
    // dereferenced at :2413. The full evidence is in the module note of
    // `path_quantity.rs`, whose `path.quantity.013` owns that case.
    expect_group("testLessThan", 33);

    // The list the operator's string branch accepts is the same twelve for all
    // four operators, and it is a closed list rather than "anything primitive".
    assert_eq!(FHIR_TYPES_STRING.len(), 12);

    // Four of the twelve are reachable from the suite's own inputs, so the
    // string branch is exercised and not merely named. The remaining eight --
    // oid, uuid, sid, markdown, base64Binary, canonical, url and xhtml -- have
    // no instance in the fetched input set and are differential-only.
    let patient = load_input("patient-example.xml");
    let questionnaire = load_input("questionnaire-example.xml");
    let valueset = load_input("valueset-example-expansion.xml");

    // code vs string, and id vs string.
    assert_eq!(
        eval_over("Patient.gender < 'n'", Some(&patient)),
        boolean(true),
        "a code compares against a string literal by code point: 'male' < 'n'"
    );
    assert_eq!(
        eval_over("Patient.id < 'z'", Some(&patient)),
        boolean(true),
        "an id compares against a string literal: 'example' < 'z'"
    );
    // uri vs string.
    assert_eq!(
        eval_over("Questionnaire.url < 'i'", Some(&questionnaire)),
        boolean(true),
        "a uri compares against a string literal: 'http://...' < 'i'"
    );
    // string vs string, through a model element rather than two literals.
    assert_eq!(
        eval_over("ValueSet.version < '3'", Some(&valueset)),
        boolean(true),
        "'20150622' < '3'"
    );

    // The numeric branch: `integer` and `decimal` only. `opLessThan`'s guard is
    // `(l.hasType("integer") || l.hasType("decimal")) && (r.hasType(...))`
    // (FHIRPathEngine.java:2417-2418), where the other three operators list
    // `"integer", "decimal", "unsignedInt", "positiveInt"`. `path.compare.011`
    // is the scenario for what that costs.
    assert_eq!(eval("1 < 2"), boolean(true));
    assert_eq!(eval("1.5 < 2"), boolean(true));
    assert_eq!(eval("1 < 1.5"), boolean(true));

    // And the temporal and quantity branches.
    assert_eq!(eval("@2014-01-01 < @2014-01-02"), boolean(true));
    assert_eq!(
        eval("@2014-01-01T10:00:00 < @2014-01-01T11:00:00"),
        boolean(true)
    );
    assert_eq!(eval("@T10:30 < @T11:30"), boolean(true));
    assert_eq!(eval("1 'cm' < 2 'cm'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.compare.002 -- `<=`
// ---------------------------------------------------------------------------

// covers: path.compare.002
#[test]
fn the_less_or_equal_operator_accepts_the_extended_numeric_type_set() {
    expect_group("testLessOrEqual", 30);

    // Its numeric branch lists all four numeric types
    // (FHIRPathEngine.java:2512-2513), so a positiveInt operand is compared
    // rather than refused. `path.compare.011` states the asymmetry against `<`.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over(
            "Patient.telecom.rank.first() <= Patient.telecom.rank.last()",
            Some(&patient)
        ),
        boolean(true),
        "two positiveInt values compare under <=: rank 1 <= rank 2"
    );

    assert_eq!(eval("1 <= 1"), boolean(true));
    assert_eq!(eval("2 <= 1"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.compare.003 -- `>`
// ---------------------------------------------------------------------------

// covers: path.compare.003
#[test]
fn the_greater_than_operator_accepts_the_extended_numeric_type_set() {
    expect_group("testGreaterThan", 30);

    // FHIRPathEngine.java:2468-2469.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over(
            "Patient.telecom.rank.last() > Patient.telecom.rank.first()",
            Some(&patient)
        ),
        boolean(true),
        "two positiveInt values compare under >: rank 2 > rank 1"
    );

    assert_eq!(eval("2 > 1"), boolean(true));
    assert_eq!(eval("1 > 1"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.compare.004 -- `>=`
// ---------------------------------------------------------------------------

// covers: path.compare.004
#[test]
fn the_greater_or_equal_operator_accepts_the_extended_numeric_type_set() {
    expect_group("testGreatorOrEqual", 29);

    // FHIRPathEngine.java:2560-2561.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over(
            "Patient.telecom.rank.first() >= Patient.telecom.rank.first()",
            Some(&patient)
        ),
        boolean(true),
        "a positiveInt compares against itself under >="
    );

    assert_eq!(eval("1 >= 1"), boolean(true));
    assert_eq!(eval("1 >= 2"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.compare.005 -- comparing different types
// ---------------------------------------------------------------------------

// covers: path.compare.005
#[test]
fn comparing_operands_of_different_types_is_an_execution_error() {
    // `opLessThan` falls through its four branches to
    // `throw makeException(expr, FHIRPATH_CANT_COMPARE, l.fhirType(), r.fhirType())`
    // (FHIRPathEngine.java:2434-2436). The message names both types.
    let message = expect_evaluation_error_over("1 < 'a'", None);
    assert!(
        message.contains("integer") && message.contains("string"),
        "the diagnostic must name both operand types: {message:?}"
    );

    // All four operators behave the same way here: the type mismatch is not
    // specific to `<`.
    for op in ORDERED {
        let expr = format!("1 {op} 'a'");
        let message = expect_evaluation_error_over(&expr, None);
        assert!(
            message.contains("integer") && message.contains("string"),
            "{expr}: {message:?}"
        );
    }

    // And a boolean operand, which is primitive but in none of the branches.
    expect_evaluation_error_over("true < false", None);
}

// ---------------------------------------------------------------------------
// path.compare.006 -- an empty operand
// ---------------------------------------------------------------------------

// covers: path.compare.006
#[test]
fn comparison_with_an_empty_operand_yields_empty() {
    // Each operator's first statement is
    // `if (left.size() == 0 || right.size() == 0) return new ArrayList<Base>()`
    // (FHIRPathEngine.java:2410-2411 and the three mirrors).
    for op in ORDERED {
        assert_eq!(eval(&format!("{{}} {op} 1")), empty(), "{{}} {op} 1");
        assert_eq!(eval(&format!("1 {op} {{}}")), empty(), "1 {op} {{}}");
        assert_eq!(eval(&format!("{{}} {op} {{}}")), empty(), "{{}} {op} {{}}");
    }

    // Empty, not false: `({} < 1).empty()` and `({} < 1) = false` separate the
    // two readings.
    assert_eq!(eval("({} < 1).empty()"), boolean(true));
    assert_eq!(eval("({} < 1) = false"), empty());
}

// ---------------------------------------------------------------------------
// path.compare.007 -- a multi-item operand (DEV-0002)
// ---------------------------------------------------------------------------

// covers: path.compare.007
#[test]
fn comparison_of_a_collection_of_more_than_one_yields_empty_rather_than_an_error() {
    // DEV-0002, resolved as HAPI's behaviour. Each operator handles the
    // empty-operand case and the singleton-and-singleton cases and then falls
    // off the end of the method into `return new ArrayList<Base>()`
    // (FHIRPathEngine.java:2453, :2497, :2545, :2593). The FHIRPath
    // specification makes a multi-item operand an error; upstream returns
    // empty, and no case in the pinned suite grades it.
    for op in ORDERED {
        assert_eq!(
            eval(&format!("(1 | 2) {op} 3")),
            empty(),
            "(1 | 2) {op} 3 must be empty, not an error and not a boolean"
        );
        assert_eq!(
            eval(&format!("3 {op} (1 | 2)")),
            empty(),
            "3 {op} (1 | 2) must be empty too"
        );
        assert_eq!(
            eval(&format!("(1 | 2) {op} (3 | 4)")),
            empty(),
            "both sides multi-item is still empty"
        );
    }

    // Three or more items behaves the same way, so the rule is "not a
    // singleton" and not "exactly two".
    assert_eq!(eval("(1 | 2 | 3) < 4"), empty());

    // A one-item collection written as a union is a singleton and does compare,
    // which is what makes this a statement about cardinality.
    assert_eq!(eval("(1 | 1) < 2"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.compare.008 -- strings compare by code point
// ---------------------------------------------------------------------------

// covers: path.compare.008
#[test]
fn strings_compare_by_code_point() {
    // `l.primitiveValue().compareTo(r.primitiveValue())`
    // (FHIRPathEngine.java:2416-2417): Java's `String.compareTo` is a UTF-16
    // code-unit comparison, so it is case sensitive and uppercase sorts first.
    assert_eq!(eval("'a' < 'b'"), boolean(true));
    assert_eq!(eval("'A' < 'a'"), boolean(true));

    // Not a case-insensitive or locale-aware collation: both of these would be
    // false under one.
    assert_eq!(eval("'B' < 'a'"), boolean(true));
    assert_eq!(eval("'a' < 'B'"), boolean(false));

    // A prefix sorts before its extension, and equal strings are neither less
    // nor greater.
    assert_eq!(eval("'abc' < 'abcd'"), boolean(true));
    assert_eq!(eval("'abc' < 'abc'"), boolean(false));
    assert_eq!(eval("'abc' <= 'abc'"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.compare.009 -- dates of insufficient precision
// ---------------------------------------------------------------------------

// covers: path.compare.009
#[test]
fn dates_of_insufficient_precision_compare_to_empty() {
    // `compareDateTimeElements` delegates to `BaseDateTimeType.compareTimes`
    // with a null default, and that returns the default as soon as the two
    // sides disagree about precision at a component whose values are equal. So
    // `@2014` against `@2014-01-05` is undecidable and the operator returns
    // `makeNull()`.
    assert_eq!(eval("@2014 < @2014-01-05"), empty());
    for op in ORDERED {
        assert_eq!(
            eval(&format!("@2014 {op} @2014-01-05")),
            empty(),
            "@2014 {op} @2014-01-05"
        );
    }

    // Where the coarser component already decides the answer there is nothing
    // undecidable, so the comparison does produce a boolean.
    assert_eq!(eval("@2013 < @2014-01-05"), boolean(true));
    assert_eq!(eval("@2015 < @2014-01-05"), boolean(false));

    // And two dates of the same precision compare normally.
    assert_eq!(eval("@2014-01-04 < @2014-01-05"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.compare.010 -- a Period invariant, raw and with boundaries
// ---------------------------------------------------------------------------

// covers: path.compare.010
#[test]
fn a_raw_period_invariant_and_a_boundary_period_invariant_disagree() {
    let (suite, _) = pinned();

    // The pair is the point: the same intent, written two ways, over the same
    // resource, gives different answers -- which is why the boundary functions
    // exist.
    let old = suite
        .case("period", "testPeriodInvariantOld")
        .expect("period/testPeriodInvariantOld is addressable");
    let new = suite
        .case("period", "testPeriodInvariantNew")
        .expect("period/testPeriodInvariantNew is addressable");
    assert_eq!(old.inputfile.as_deref(), Some("patient-example-period.xml"));
    assert_eq!(new.inputfile.as_deref(), Some("patient-example-period.xml"));
    assert_eq!(old.outputs.len(), 1);
    assert_eq!(new.outputs.len(), 1);
    assert_eq!(
        (old.outputs[0].body.as_str(), new.outputs[0].body.as_str()),
        ("false", "true"),
        "the raw comparison fails the invariant and the boundary form passes it"
    );

    expect_case("period", "testPeriodInvariantOld");
    expect_case("period", "testPeriodInvariantNew");

    // Stated at the seam as well, so the disagreement is attributable to the
    // comparison and not to the `all()` wrapper.
    let period = load_input("patient-example-period.xml");
    assert_eq!(
        eval_over(
            "Patient.identifier.period.all(start.hasValue().not() or \
             end.hasValue().not() or (start <= end))",
            Some(&period)
        ),
        boolean(false)
    );
    assert_eq!(
        eval_over(
            "Patient.identifier.period.all(start.empty() or end.empty() or \
             (start.lowBoundary() < end.highBoundary()))",
            Some(&period)
        ),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.compare.011 -- positiveInt and unsignedInt (DEV-0003)
// ---------------------------------------------------------------------------

// covers: path.compare.011
#[test]
fn a_positive_or_unsigned_int_compares_under_three_operators_and_fails_under_less_than() {
    // DEV-0003, resolved as HAPI's behaviour and asymmetric on purpose:
    // `opLessThan`'s numeric guard is `integer || decimal`
    // (FHIRPathEngine.java:2417-2418) while `opGreater` (:2468),
    // `opLessOrEqual` (:2512) and `opGreaterOrEqual` (:2560) each list
    // `"integer", "decimal", "unsignedInt", "positiveInt"`. No case in the
    // pinned suite compares either type, so this is differential-only.

    // positiveInt: `ContactPoint.rank` (ContactPoint.java:503), which
    // patient-example.xml carries twice, with the values 1 and 2.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.telecom.rank", Some(&patient)),
        vec![
            ResultItem::Primitive {
                fhir_type: "positiveInt".to_string(),
                value: "1".to_string()
            },
            ResultItem::Primitive {
                fhir_type: "positiveInt".to_string(),
                value: "2".to_string()
            },
        ],
        "the element really is typed positiveInt, which is what the asymmetry turns on"
    );

    let message = expect_evaluation_error_over(
        "Patient.telecom.rank.first() < Patient.telecom.rank.last()",
        Some(&patient),
    );
    assert!(
        message.contains("positiveInt"),
        "the cannot-compare error must name both FHIR types: {message:?}"
    );

    for (op, expected) in [("<=", true), (">", false), (">=", false)] {
        assert_eq!(
            eval_over(
                &format!("Patient.telecom.rank.first() {op} Patient.telecom.rank.last()"),
                Some(&patient)
            ),
            boolean(expected),
            "rank 1 {op} rank 2 returns a boolean under {op}"
        );
    }

    // unsignedInt: `Appointment.priority` (Appointment.java:1307), which
    // `examples/appointment-example.json` carries once, with the value 5.
    let appointment = load_input("examples/appointment-example.json");
    assert_eq!(
        appointment.resource_type(),
        Some("Appointment"),
        "the input must be an Appointment for this half of the scenario"
    );
    assert_eq!(
        eval_over("Appointment.priority", Some(&appointment)),
        vec![ResultItem::Primitive {
            fhir_type: "unsignedInt".to_string(),
            value: "5".to_string()
        }],
        "the element really is typed unsignedInt"
    );

    let message = expect_evaluation_error_over(
        "Appointment.priority < Appointment.priority",
        Some(&appointment),
    );
    assert!(
        message.contains("unsignedInt"),
        "the cannot-compare error must name both FHIR types: {message:?}"
    );

    for (op, expected) in [("<=", true), (">", false), (">=", true)] {
        assert_eq!(
            eval_over(
                &format!("Appointment.priority {op} Appointment.priority"),
                Some(&appointment)
            ),
            boolean(expected),
            "priority {op} priority returns a boolean under {op}"
        );
    }
}

// ---------------------------------------------------------------------------
// path.compare.012 -- comparison goes through binary floating point (DEV-0004)
// ---------------------------------------------------------------------------

// covers: path.compare.012
#[test]
fn numeric_comparison_goes_through_binary_floating_point() {
    // DEV-0004, resolved as HAPI's behaviour. All four operators compare
    // `Double.valueOf(left) OP Double.valueOf(right)`
    // (FHIRPathEngine.java:2418, :2469, :2513, :2561), so two decimals that
    // differ only beyond what an IEEE-754 double can hold compare as equal.
    //
    // 1e-17 is below half an ulp at 1.0, so both of these round to exactly 1.0.
    const LO: &str = "1.00000000000000001";
    const HI: &str = "1.00000000000000002";

    // As exact decimals LO is less than HI. As doubles they are the same value,
    // and the comparison says so.
    assert_eq!(
        eval(&format!("{LO} < {HI}")),
        boolean(false),
        "exact decimal semantics would make this true"
    );
    assert_eq!(eval(&format!("{HI} > {LO}")), boolean(false));
    assert_eq!(eval(&format!("{LO} <= {HI}")), boolean(true));
    assert_eq!(eval(&format!("{LO} >= {HI}")), boolean(true));

    // The two really are different decimals, which `=` shows: `doEquals` sends
    // decimals to `decEqual`, a trailing-zero-stripped string comparison
    // (FHIRPathEngine.java:2180-2184), not to a double.
    assert_eq!(eval(&format!("{LO} = {HI}")), boolean(false));

    // The same double round trip applies to the eight math functions that parse
    // their focus with `Double.parseDouble`: `abs` here
    // (FHIRPathEngine.java:4121-4123) returns `BigDecimal.valueOf(1.0)`, whose
    // lexical form is "1.0" and not the seventeen-digit input.
    assert_eq!(eval(&format!("{LO}.abs()")), decimal("1.0"));
    assert_eq!(eval(&format!("{HI}.abs()")), decimal("1.0"));

    // And it does not apply to the exact-decimal operators: adding zero to LO
    // returns LO with its precision intact, because `opPlus`'s decimal arm uses
    // `BigDecimal.add` (FHIRPathEngine.java:2702-2703).
    assert_eq!(eval(&format!("{LO} + 0")), decimal(LO));
    assert_eq!(eval(&format!("{LO} - 0")), decimal(LO));

    // `truncate` and `round` are exact too, so they are not in the double set.
    assert_eq!(eval(&format!("{LO}.round(17)")), decimal(LO));
}

// ---------------------------------------------------------------------------
// path.equality.001 -- `=`
// ---------------------------------------------------------------------------

// covers: path.equality.001
#[test]
fn the_equality_operator_compares_values_of_the_same_type() {
    expect_group("testEquality", 40);
}

// ---------------------------------------------------------------------------
// path.equality.002 -- `!=`
// ---------------------------------------------------------------------------

// covers: path.equality.002
#[test]
fn the_inequality_operator_is_the_negation_of_equality() {
    expect_group("testNEquality", 27);

    // Negation includes the empty case: where `=` is empty, `!=` is empty too
    // rather than true.
    assert_eq!(eval("(1 = {}).empty()"), boolean(true));
    assert_eq!(eval("(1 != {}).empty()"), boolean(true));
    assert_eq!(eval("(@2014 = @2014-01).empty()"), boolean(true));
    assert_eq!(eval("(@2014 != @2014-01).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.equality.003 -- an empty operand
// ---------------------------------------------------------------------------

// covers: path.equality.003
#[test]
fn equality_with_an_empty_operand_yields_empty() {
    // `opEquals` returns an empty list for either side empty
    // (FHIRPathEngine.java:2141-2143), which is the behaviour that makes
    // equality unusable as a presence test.
    assert_eq!(eval("{} = 1"), empty());
    assert_eq!(eval("1 = {}"), empty());
    assert_eq!(eval("{} = {}"), empty());

    // Empty and not false, which is the whole distinction against
    // `path.equality.006`'s equivalence.
    assert_eq!(eval("({} = 1).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.equality.004 -- `~`
// ---------------------------------------------------------------------------

// covers: path.equality.004
#[test]
fn the_equivalence_operator_ignores_case_and_whitespace_for_strings() {
    expect_group("testEquivalent", 31);

    // `doEquivalent`'s string branch goes through `Utilities.equivalent`
    // (FHIRPathEngine.java:2223-2225), which folds case and collapses
    // whitespace -- neither of which `=` does.
    assert_eq!(eval("'Abc' ~ 'abc'"), boolean(true));
    assert_eq!(eval("'Abc' = 'abc'"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.equality.005 -- `!~`
// ---------------------------------------------------------------------------

// covers: path.equality.005
#[test]
fn the_non_equivalence_operator_is_the_negation_of_equivalence() {
    expect_group("testNotEquivalent", 24);

    // Including on the empty cases, where both answers are booleans.
    assert_eq!(eval("{} !~ {}"), boolean(false));
    assert_eq!(eval("{} !~ 1"), boolean(true));
    assert_eq!(eval("'Abc' !~ 'abc'"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.equality.006 -- equivalence with an empty operand
// ---------------------------------------------------------------------------

// covers: path.equality.006
#[test]
fn equivalence_with_an_empty_operand_yields_a_boolean_and_never_empty() {
    // `opEquivalent` has no empty-operand guard at all: it compares the two
    // sizes first (FHIRPathEngine.java:2112-2114), so two empties are
    // equivalent and an empty against a singleton is not. This is the one place
    // equivalence and equality differ in kind rather than in strictness.
    assert_eq!(eval("{} ~ {}"), boolean(true));
    assert_eq!(eval("{} ~ 1"), boolean(false));
    assert_eq!(eval("1 ~ {}"), boolean(false));

    // Never empty, which is what makes `~` usable where `=` is not.
    for expr in ["{} ~ {}", "{} ~ 1", "1 ~ {}"] {
        assert_eq!(
            eval(&format!("({expr}).empty()")),
            boolean(false),
            "({expr}) must be a boolean"
        );
    }
}

// ---------------------------------------------------------------------------
// path.equality.007 -- collections compare item by item in order
// ---------------------------------------------------------------------------

// covers: path.equality.007
#[test]
fn equality_on_collections_compares_item_by_item_in_order() {
    // `opEquals` walks both lists by index (FHIRPathEngine.java:2149-2158), so
    // the same items in a different order are not equal.
    assert_eq!(eval("(1 | 2 | 3) = (1 | 2 | 3)"), boolean(true));
    assert_eq!(eval("(1 | 2 | 3) = (3 | 2 | 1)"), boolean(false));

    // Different sizes are false rather than empty
    // (FHIRPathEngine.java:2145-2147) -- but only once both sides are non-empty,
    // which `path.equality.003` covers.
    assert_eq!(eval("(1 | 2) = (1 | 2 | 3)"), boolean(false));

    // Equivalence is the unordered counterpart, so the reordered pair that `=`
    // calls false is equivalent.
    assert_eq!(eval("(1 | 2 | 3) ~ (3 | 2 | 1)"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.equality.008 -- decimals compare by value
// ---------------------------------------------------------------------------

// covers: path.equality.008
#[test]
fn equality_on_decimals_compares_by_value_and_not_by_lexical_form() {
    // `doEquals` routes anything decimal to `decEqual`, which strips trailing
    // zeros from both sides before comparing the strings
    // (FHIRPathEngine.java:2180-2184).
    assert_eq!(eval("1.0 = 1"), boolean(true));
    assert_eq!(eval("1.10 = 1.1"), boolean(true));
    assert_eq!(eval("1.000 = 1.0"), boolean(true));

    // And a genuinely different value is still not equal, so the stripping is
    // not a blanket tolerance.
    assert_eq!(eval("1.01 = 1.1"), boolean(false));
    assert_eq!(eval("1.1 = 1.10000000000000001"), boolean(false));

    // Note that this is a different rule from the one `path.compare.012`
    // records for the ordered operators: equality is exact on the digits,
    // comparison is not.
    assert_eq!(eval("1.10000000000000001 <= 1.1"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.equality.009 -- complex elements compare every child
// ---------------------------------------------------------------------------

// covers: path.equality.009
#[test]
fn equality_on_complex_elements_compares_every_child() {
    // `doEquals`'s last arm is `Base.compareDeep(left, right, false)`
    // (FHIRPathEngine.java:2200-2201), so two HumanName elements are equal only
    // when every child matches.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.name[0] = Patient.name[0]", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.name[0] = Patient.name[1]", Some(&patient)),
        boolean(false),
        "name[0] is the official Chalmers name and name[1] the usual Jim one"
    );

    // A difference in one child is enough, and sameness across every child is
    // required: name[0] and name[2] share both given names and differ in `use`
    // and `family`.
    assert_eq!(
        eval_over("Patient.name[0] = Patient.name[2]", Some(&patient)),
        boolean(false)
    );
    assert_eq!(
        eval_over(
            "Patient.name[0].given = Patient.name[2].given",
            Some(&patient)
        ),
        boolean(true),
        "the given names alone do match, so the inequality above comes from the \
         other children"
    );
}

// ---------------------------------------------------------------------------
// path.equality.010 -- dates of differing precision under `=`
// ---------------------------------------------------------------------------

// covers: path.equality.010
#[test]
fn equality_on_dates_of_differing_precision_yields_empty_where_undecidable() {
    // `datesEqual` calls `BaseDateTimeType.equalsUsingFhirPathRules`, which --
    // with both sides lacking a timezone -- defers to
    // `compareTimes(left, right, null)`. That returns the null default as soon
    // as one side runs out of precision at a component the two agree on, and
    // `opEquals` turns a null comparison into an empty result
    // (FHIRPathEngine.java:2159-2165).
    assert_eq!(eval("@2014 = @2014-01"), empty());
    assert_eq!(eval("@2014-01 = @2014-01-05"), empty());
    assert_eq!(eval("@2014 = @2014-01-05"), empty());

    // Where the coarser component already settles the question there is nothing
    // undecidable, so the answer is a boolean.
    assert_eq!(eval("@2014 = @2015-01"), boolean(false));

    // And equal precision compares normally.
    assert_eq!(eval("@2014-01 = @2014-01"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.equality.011 -- dates of differing precision under `~`
// ---------------------------------------------------------------------------

// covers: path.equality.011
#[test]
fn equivalence_on_dates_of_differing_precision_is_what_the_pinned_engine_computes() {
    // What the pinned engine computes, which is the behaviour the scenario
    // defers to: `false`.
    //
    // `doEquivalent`'s temporal branch calls
    // `compareDateTimeElements(left, right, true)`
    // (FHIRPathEngine.java:2217-2222), and that branch is
    // `return left.equalsUsingFhirPathRules(right) == Boolean.TRUE ? 0 : 1`
    // (FHIRPathEngine.java:639-641). For `@2014` against `@2014-01` the rules
    // answer `null`, `null != Boolean.TRUE`, so the comparison is 1 and the
    // equivalence is false.
    //
    // Note for review: the scenario's title says equivalence "compares at the
    // lower precision", which would make this `true`, and is what the FHIRPath
    // specification requires. The scenario body defers to the engine, so this
    // test asserts the engine's answer. The conflict is between the title and
    // the pinned source, not between this test and either.
    assert_eq!(eval("@2014 ~ @2014-01"), boolean(false));
    assert_eq!(eval("@2014-01 ~ @2014-01-05"), boolean(false));

    // Equal precision is equivalent, so the false above is about the precision
    // mismatch and not about temporal equivalence in general.
    assert_eq!(eval("@2014 ~ @2014"), boolean(true));
    assert_eq!(eval("@2014-01-05 ~ @2014-01-05"), boolean(true));

    // And unlike `=`, `~` never answers empty here.
    assert_eq!(eval("(@2014 ~ @2014-01).empty()"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.equality.012 -- `hasValue()`
// ---------------------------------------------------------------------------

// covers: path.equality.012
#[test]
fn has_value_distinguishes_a_primitive_with_a_value_from_one_without() {
    expect_group("testHasValue", 3);

    // The distinguishing pair from that group, stated here: a primitive with a
    // value has one, and a complex element does not.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("name.first().given.first().hasValue()", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("name.first().hasValue()", Some(&patient)),
        boolean(false),
        "a HumanName is not a primitive, so it has no value"
    );
}
