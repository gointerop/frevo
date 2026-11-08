//! Boolean logic: `path.logic.001` through `path.logic.010`.
//!
//! ```text
//! cargo test -p frevo-path --test path_logic
//! ```
//!
//! The four truth tables are graded by driving the suite's own groups, each of
//! which is exactly the nine true/false/empty combinations. The scenarios the
//! suite does not grade -- short-circuiting, and `iif` with an empty condition
//! or no else branch -- are driven at the engine seam.
//!
//! Short-circuiting is observable only through an error: the right operand of
//! each of the three short-circuiting operators is an expression that fails on
//! its own, so an engine that evaluates it reports that failure instead of the
//! answer. That is why `1 < 'a'` is asserted to fail in isolation first; without
//! that, a passing short-circuit test would prove nothing.
//!
//! Provenance: `FHIRPathEngine.preOperate` (:1541-1556) decides `and`, `or` and
//! `implies` from the left operand alone and returns null for everything else,
//! so `xor` evaluates both sides; `funcIif` (:3429-3444) is the `iif` arm. All
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
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

fn integer(value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: "integer".to_string(),
        value: value.to_string(),
    }]
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

/// The right operand used in every short-circuit scenario: an expression that
/// fails on its own, so an engine that evaluates it cannot hide the fact.
const FAILING_OPERAND: &str = "(1 < 'a')";

// ---------------------------------------------------------------------------
// path.logic.001 -- `and`
// ---------------------------------------------------------------------------

// covers: path.logic.001
#[test]
fn and_follows_the_three_valued_truth_table() {
    // The group is exactly the nine combinations of true, false and empty.
    expect_group("testBooleanLogicAnd", 9);

    // The three the table turns on, stated here as well so the scenario is
    // readable without the oracle file: `and` is false whenever either side is
    // false, even against empty, and empty otherwise.
    assert_eq!(eval("(false and {}) = false"), boolean(true));
    assert_eq!(eval("({} and false) = false"), boolean(true));
    assert_eq!(eval("({} and true).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.logic.002 -- `or`
// ---------------------------------------------------------------------------

// covers: path.logic.002
#[test]
fn or_follows_the_three_valued_truth_table() {
    expect_group("testBooleanLogicOr", 9);

    // `or` is true whenever either side is true, even against empty.
    assert_eq!(eval("(true or {}) = true"), boolean(true));
    assert_eq!(eval("({} or true) = true"), boolean(true));
    assert_eq!(eval("({} or false).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.logic.003 -- `xor`
// ---------------------------------------------------------------------------

// covers: path.logic.003
#[test]
fn xor_follows_the_three_valued_truth_table() {
    expect_group("testBooleanLogicXOr", 9);

    // `xor` has no absorbing value, so every combination involving empty is
    // empty -- which is also why it cannot short-circuit (`path.logic.007`).
    assert_eq!(eval("(true xor {}).empty()"), boolean(true));
    assert_eq!(eval("(false xor {}).empty()"), boolean(true));
    assert_eq!(eval("({} xor {}).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.logic.004 -- `implies`
// ---------------------------------------------------------------------------

// covers: path.logic.004
#[test]
fn implies_follows_the_three_valued_truth_table() {
    expect_group("testBooleanImplies", 9);

    // A false antecedent makes the implication true whatever follows, and a
    // true consequent makes it true whatever precedes.
    assert_eq!(eval("(false implies {}) = true"), boolean(true));
    assert_eq!(eval("({} implies true) = true"), boolean(true));
    assert_eq!(eval("({} implies false).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.logic.005 -- a singleton non-boolean operand of `and`
// ---------------------------------------------------------------------------

// covers: path.logic.005
#[test]
fn a_singleton_non_boolean_operand_of_and_is_treated_as_true() {
    let (suite, _) = pinned();
    let case = suite
        .case("from-Zulip", "from-zulip-1")
        .expect("from-Zulip/from-zulip-1 is addressable");
    assert_eq!(case.expression, "(true and 'foo').empty()");
    assert_eq!(case.outputs.len(), 1);
    assert_eq!(
        (
            case.outputs[0].type_name.as_str(),
            case.outputs[0].body.as_str()
        ),
        ("boolean", "false"),
        "the result is not empty, so 'foo' was read as a boolean rather than \
         making the operation undecidable"
    );
    expect_case("from-Zulip", "from-zulip-1");

    // Singleton evaluation of collections makes a non-empty singleton true, so
    // the conjunction is true rather than empty or an error.
    assert_eq!(eval("(true and 'foo')"), boolean(true));
    assert_eq!(eval("(true and 'foo').empty()"), boolean(false));

    // And the rule is about the operand being a singleton: with `false` on the
    // left the result is false by short-circuit, which `path.logic.006` covers.
    assert_eq!(eval("('foo' and true)"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.logic.006 -- `and` short-circuits on a false left operand
// ---------------------------------------------------------------------------

// covers: path.logic.006
#[test]
fn and_short_circuits_on_a_false_left_operand() {
    // First: the right operand really does fail on its own. Without this the
    // short-circuit assertion below would hold for an engine that simply never
    // raises.
    let message = expect_evaluation_error(FAILING_OPERAND);
    assert!(
        message.contains("integer") && message.contains("string"),
        "the operand must fail with the cannot-compare error: {message:?}"
    );

    // `preOperate`'s `case And` returns false from a false left operand without
    // the right operand being executed at all
    // (FHIRPathEngine.java:1546-1547, reached from the operator loop at
    // :1512-1516 which only calls `execute` on the right when `preOperate`
    // returned null).
    assert_eq!(
        eval(&format!("false and {FAILING_OPERAND}")),
        boolean(false),
        "a false left operand decides `and` before the right one is evaluated"
    );

    // With a left operand that does not decide it, the right one is evaluated
    // and the failure surfaces -- so the short circuit is a short circuit and
    // not a blanket suppression of errors.
    expect_evaluation_error(&format!("true and {FAILING_OPERAND}"));
    expect_evaluation_error(&format!("{{}} and {FAILING_OPERAND}"));
}

// ---------------------------------------------------------------------------
// path.logic.007 -- `or` and `implies` short-circuit; `xor` does not
// ---------------------------------------------------------------------------

// covers: path.logic.007
#[test]
fn or_and_implies_short_circuit_and_xor_does_not() {
    // `or` from a true left operand (FHIRPathEngine.java:1548-1549).
    assert_eq!(
        eval(&format!("true or {FAILING_OPERAND}")),
        boolean(true),
        "a true left operand decides `or`"
    );
    // `implies` from a false left operand (FHIRPathEngine.java:1550-1552).
    assert_eq!(
        eval(&format!("false implies {FAILING_OPERAND}")),
        boolean(true),
        "a false antecedent decides `implies`"
    );

    // `xor` has no such arm: `preOperate`'s `default` returns null
    // (FHIRPathEngine.java:1553-1554), so both operands are evaluated and the
    // right one's failure surfaces.
    let message = expect_evaluation_error(&format!("false xor {FAILING_OPERAND}"));
    assert!(
        message.contains("integer") && message.contains("string"),
        "the failure must be the right operand's own: {message:?}"
    );
    // Including with `true` on the left, since the asymmetry is about the
    // operator and not about the value.
    expect_evaluation_error(&format!("true xor {FAILING_OPERAND}"));

    // The mirror cases for `or` and `implies`: when the left operand does not
    // decide the answer, the right one is evaluated and does fail.
    expect_evaluation_error(&format!("false or {FAILING_OPERAND}"));
    expect_evaluation_error(&format!("true implies {FAILING_OPERAND}"));
}

// ---------------------------------------------------------------------------
// path.logic.008 -- `iif`
// ---------------------------------------------------------------------------

// covers: path.logic.008
#[test]
fn iif_selects_one_branch_and_leaves_the_other_unevaluated() {
    // The group includes `iif(true, true, (1 | 2).toString())` and its mirror,
    // whose unselected branch fails on its own -- which is how the suite
    // observes that the branch was not taken.
    expect_group("testIif", 11);

    // Stated at the seam too, with the failing operand this file uses
    // throughout: the unselected branch is not evaluated.
    assert_eq!(
        eval(&format!("iif(true, 1, {FAILING_OPERAND})")),
        integer("1")
    );
    assert_eq!(
        eval(&format!("iif(false, {FAILING_OPERAND}, 2)")),
        integer("2")
    );

    // And the selected branch is evaluated, so the above is not a blanket
    // suppression either.
    expect_evaluation_error(&format!("iif(true, {FAILING_OPERAND}, 2)"));
    expect_evaluation_error(&format!("iif(false, 1, {FAILING_OPERAND})"));
}

// ---------------------------------------------------------------------------
// path.logic.009 -- `iif` with an empty condition
// ---------------------------------------------------------------------------

// covers: path.logic.009
#[test]
fn iif_with_an_empty_condition_yields_the_else_branch() {
    // `funcIif` compares `asBool(n1, exp)` against `Equality.True` only, so an
    // empty condition takes the same path as a false one
    // (FHIRPathEngine.java:3435-3443).
    assert_eq!(eval("iif({}, 1, 2)"), integer("2"));

    // Which is the same answer a false condition gives, and a different one
    // from what a true condition gives.
    assert_eq!(eval("iif(false, 1, 2)"), integer("2"));
    assert_eq!(eval("iif(true, 1, 2)"), integer("1"));

    // The else branch is really evaluated rather than returned as a literal:
    // an expression there is computed.
    assert_eq!(eval("iif({}, 1, 1 + 1)"), integer("2"));
}

// ---------------------------------------------------------------------------
// path.logic.010 -- `iif` with no else branch
// ---------------------------------------------------------------------------

// covers: path.logic.010
#[test]
fn iif_with_no_else_branch_and_a_false_condition_yields_empty() {
    // `exp.getParameters().size() < 3` returns an empty list rather than
    // raising about the missing parameter (FHIRPathEngine.java:3439-3440).
    assert_eq!(eval("iif(false, 1)"), empty());
    assert_eq!(eval("iif({}, 1)"), empty());

    // The two-parameter form still returns the then branch when the condition
    // holds, so the arity is legal and not merely tolerated.
    assert_eq!(eval("iif(true, 1)"), integer("1"));

    // The suite grades the `.empty()` form of the same case.
    expect_case("testIif", "testIif5");
}
