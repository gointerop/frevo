//! The static type-check pass: `path.check.001` through `path.check.014`.
//!
//! ```text
//! cargo test -p frevo-path --test path_check
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. `path.check.015` through
//! `path.check.019` are the warning channel and are graded by the committed
//! in-process oracle in `path_check_warnings.rs`; this file is the pass/fail
//! verdict and the type inference, graded from the suite's 29
//! `invalid="semantic"` cases and the `isSemanticChecks` group.
//!
//! Two facts about those 29 shape several tests here:
//!
//! - **27 of them fail at the check stage, not 29.**
//!   `documentFunctions/testResourcesAbstract3` and
//!   `documentFunctions/testHasCoding11` name `resources()` and `hasCoding()`,
//!   which the R4 `ExpressionNode.Function` enum does not have. The parser
//!   consults the host service only after `Function.fromCode` returns null
//!   (FHIRPathEngine.java:1158-1166) and `FHIRPathTestEvaluationServices`
//!   **raises** from `resolveFunction`, so those two fail at the **parse**
//!   stage. They are still graded `passed`, because the harness grades a
//!   `semantic` expectation against the stage the failure came from
//!   (`path.oracle.005`) -- which is why this file asserts the stage and not
//!   only the verdict.
//! - **four `isSemanticChecks` pairs are byte-identical apart from `context`
//!   and `contextType`.** HAPI's pinned R4 harness reads neither attribute, so
//!   it necessarily fails one of each pair (`path.oracle.016`); Frevo is graded
//!   on its own context-aware run.
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use frevo_path::TypeSet;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::{Case, Stage, Suite};
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// The type-returning check seam
//
// `SuiteEngine::check` answers pass or fail, which is all the suite needs. Four
// scenarios here are about the inferred **type**, so they drive this seam
// instead. Its unbound binding answers `NotImplemented`, never an empty type
// set.
// ---------------------------------------------------------------------------

trait TypeCheckEngine {
    /// Check `expr` against a focus described by `context` and `context_types`
    /// and return the inferred type set.
    ///
    /// `context_types` may name several types, which is what the suite's
    /// `contextType="Period|Timing"` means (`path.check.012`).
    fn check_types(
        &mut self,
        expr: &str,
        root_type: &str,
        context: Option<&str>,
        context_types: &[&str],
    ) -> Result<TypeSet, EngineError>;
}

fn type_check_engine_under_test() -> Box<dyn TypeCheckEngine> {
    Box::new(UnboundTypeCheck)
}

struct UnboundTypeCheck;

impl TypeCheckEngine for UnboundTypeCheck {
    fn check_types(
        &mut self,
        _expr: &str,
        _root_type: &str,
        _context: Option<&str>,
        _context_types: &[&str],
    ) -> Result<TypeSet, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

/// The inferred type set for `expr`, requiring the check to succeed.
fn inferred(expr: &str, root_type: &str) -> TypeSet {
    let mut engine = type_check_engine_under_test();
    engine
        .check_types(expr, root_type, None, &[])
        .unwrap_or_else(|e| panic!("checking {expr:?} against {root_type} must succeed: {e}"))
}

/// The inferred type set for `expr` checked at `context` with `context_types`.
fn inferred_at(expr: &str, root_type: &str, context: &str, context_types: &[&str]) -> TypeSet {
    let mut engine = type_check_engine_under_test();
    engine
        .check_types(expr, root_type, Some(context), context_types)
        .unwrap_or_else(|e| {
            panic!("checking {expr:?} at {context} as {context_types:?} must succeed: {e}")
        })
}

/// `expr` must fail the check at `context` with `context_types`, with a real
/// diagnostic.
fn check_failure_at(expr: &str, root_type: &str, context: &str, context_types: &[&str]) -> String {
    let mut engine = type_check_engine_under_test();
    match engine.check_types(expr, root_type, Some(context), context_types) {
        Ok(t) => panic!("checking {expr:?} at {context} as {context_types:?} must fail, got {t}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// Every case whose `expression/@invalid` is `semantic`.
fn semantic_cases() -> Vec<&'static Case> {
    let (suite, _) = pinned();
    suite
        .cases()
        .filter(|c| c.expected_failure == Some(Stage::Check))
        .collect()
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

fn load_input(file: &str) -> SuiteInput {
    input::load(file)
        .unwrap_or_else(|e| panic!("the input {file} must load before it can be checked over: {e}"))
}

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is checked: {e}"));
    engine
}

fn context_for<'a>(inp: Option<&'a SuiteInput>) -> CaseContext<'a> {
    let resource_type = inp.and_then(|i| i.resource_type());
    CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    }
}

fn primitive(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

// ---------------------------------------------------------------------------
// path.check.001 -- the check pass is separate from evaluation
// ---------------------------------------------------------------------------

// covers: path.check.001
#[test]
fn the_check_pass_is_separate_from_evaluation() {
    let patient = load_input("patient-example.xml");
    let context = context_for(Some(&patient));

    // Checking reports its verdict without evaluating: the engine is asked to
    // check and nothing else, and it answers.
    let mut engine = fresh_engine();
    let parsed = engine
        .parse("Patient.name.given")
        .expect("the expression parses");
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .expect("the check reports its result on its own");

    // The parsed expression is still evaluable afterwards, so checking did not
    // consume it -- which is what lets a caller check once and evaluate many
    // times (`path.api.002`).
    let values = engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("and evaluation is a separate call");
    assert_eq!(values.len(), 5);

    // An expression that fails the check is not evaluated: the harness's
    // three-stage run returns at the check stage, so the report attributes the
    // failure to `check` and never reaches the evaluator
    // (`tests/oracle/run.rs::run_case`).
    let (suite, report) = pinned();
    let address = suite
        .case("testBasics", "testSimpleFail")
        .expect("the case resolves")
        .address
        .to_string();
    let case = report
        .get(&address)
        .expect("the case appears in the run report");
    assert_eq!(case.expected_failure, Some(Stage::Check));
    assert_eq!(
        case.failed_stage,
        Some(Stage::Check),
        "the run must stop at the check stage: {}",
        case.reason
    );
    assert_eq!(case.status, CaseStatus::Passed, "{}", case.reason);
}

// ---------------------------------------------------------------------------
// path.check.002 -- the semantic-invalid oracle cases fail the check
// ---------------------------------------------------------------------------

// covers: path.check.002
#[test]
fn the_semantic_invalid_oracle_cases_all_fail_the_check() {
    let (_, report) = pinned();
    let cases = semantic_cases();
    assert_eq!(
        cases.len(),
        29,
        "the pinned file carries 29 invalid=\"semantic\" cases"
    );

    // Every one is graded `passed`, which is the scenario's requirement.
    for case in &cases {
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

    // 27 fail at the **check** stage. The other two are
    // `documentFunctions/testResourcesAbstract3` and
    // `documentFunctions/testHasCoding11`, which name functions the R4
    // `Function` enum does not have and so fail at the **parse** stage: the
    // parser asks the host service only after `Function.fromCode` returns null
    // (FHIRPathEngine.java:1158-1166).
    let at_check = cases
        .iter()
        .filter(|c| {
            report
                .get(&c.address.to_string())
                .is_some_and(|r| r.failed_stage == Some(Stage::Check))
        })
        .count();
    let at_parse: Vec<&str> = cases
        .iter()
        .filter(|c| {
            report
                .get(&c.address.to_string())
                .is_some_and(|r| r.failed_stage == Some(Stage::Parse))
        })
        .map(|c| c.address.name.as_str())
        .collect();
    assert_eq!(at_check, 27, "27 of the 29 are check-stage failures");
    assert_eq!(
        at_parse,
        ["testResourcesAbstract3", "testHasCoding11"],
        "and these two are parse-stage failures"
    );

    // The check reports which element path or type caused the failure, which is
    // asserted on the two shapes the group covers: an unknown element name and
    // an impossible type.
    let patient = load_input("patient-example.xml");
    let mut engine = fresh_engine();
    let parsed = engine.parse("name.given1").expect("parses");
    match engine.check(parsed, &context_for(Some(&patient)), StrictnessFlags::CHECK) {
        Err(EngineError::Diagnostic { message, .. }) => assert!(
            message.contains("given1"),
            "the diagnostic must name the element: {message}"
        ),
        Err(e) => panic!("the check must answer with a diagnostic, not: {e}"),
        Ok(()) => panic!("`name.given1` must fail the check"),
    }

    // Each case is checked with the `context` and `contextType` it carries, so
    // this is graded against Frevo's context-aware run. 27 of the 29 carry at
    // least one of the two.
    let with_context = cases
        .iter()
        .filter(|c| c.context.is_some() || !c.context_types.is_empty())
        .count();
    assert_eq!(
        with_context, 15,
        "the isSemanticChecks cases carry context and contextType; HAPI reads \
         neither (path.oracle.016)"
    );
}

// ---------------------------------------------------------------------------
// path.check.003 -- a non-existent element is a semantic error
// ---------------------------------------------------------------------------

// covers: path.check.003
#[test]
fn navigating_a_non_existent_element_is_a_semantic_error() {
    // `executeType`'s name node raises `FHIRPATH_UNKNOWN_NAME` when the focus
    // yields no types for the name and the node may not be none
    // (FHIRPathEngine.java:1593).
    let message = check_failure_at("Patient.nosuchelement", "Patient", "Patient", &["Patient"]);
    assert!(
        message.contains("nosuchelement"),
        "the error must name the element: {message}"
    );

    // The same for a nested element, so the rule is not only about the leading
    // name.
    let message = check_failure_at(
        "Patient.name.nosuchpart",
        "Patient",
        "Patient",
        &["Patient"],
    );
    assert!(message.contains("nosuchpart"), "{message}");

    // A real element checks cleanly, so the failure is about the name.
    assert!(!inferred("Patient.name.given", "Patient").is_empty());
    expect_case("testBasics", "testSimpleFail");
}

// ---------------------------------------------------------------------------
// path.check.004 -- the inferred type of a choice element
// ---------------------------------------------------------------------------

// covers: path.check.004
#[test]
fn the_check_infers_a_choice_elements_type_as_the_declared_set() {
    // `Observation.value[x]` is declared over many types, and the check pass
    // propagates the whole set rather than collapsing it to one -- which is
    // what makes `Observation.value is Quantity` and `is Period` both
    // answerable (`path.types.*`).
    let types = inferred("Observation.value", "Observation");
    assert!(
        types.len() > 1,
        "the inferred type must be the declared choice set, not one type: {types}"
    );
    assert!(types.contains("Quantity"), "{types}");
    assert!(types.contains("Period"), "{types}");
    assert!(types.contains("string"), "{types}");

    // Declared order is preserved, because it is observable in diagnostics:
    // `Quantity` is the first of `value[x]`'s types in R4.
    assert_eq!(
        types.names().first().map(String::as_str),
        Some("Quantity"),
        "the declared order is kept: {types}"
    );

    // A non-choice element infers exactly one type, so the set is not always
    // wide.
    let one = inferred("Observation.status", "Observation");
    assert_eq!(one.names(), ["code"]);

    // And `ofType` narrows the set to the named type.
    let narrowed = inferred("Observation.value.ofType(Quantity)", "Observation");
    assert_eq!(narrowed.names(), ["Quantity"]);
}

// ---------------------------------------------------------------------------
// path.check.005 -- the check narrows after an `is` guard in `implies`
// ---------------------------------------------------------------------------

// covers: path.check.005
#[test]
fn the_check_narrows_the_type_after_an_is_guard_in_implies() {
    // `$this is dateTime implies ...` checks the right operand assuming the
    // guard is true, so `toString()` is checked against `dateTime` and not
    // against the declared `Period`.
    let types = inferred_at(
        "$this is dateTime implies $this.toString().length() > 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(
        !types.is_empty(),
        "the guarded form checks cleanly: {types}"
    );
    expect_case("isSemanticChecks", "isEffectOnLogic3");

    // The guard narrows to the named type, not to anything: a guard on a type
    // the focus cannot be leaves the operand unreachable rather than erroneous
    // (`path.check.011`).
    expect_case("isSemanticChecks", "isGuardImpliesEmpty");
}

// ---------------------------------------------------------------------------
// path.check.006 -- the same expression without the guard fails
// ---------------------------------------------------------------------------

// covers: path.check.006
#[test]
fn the_same_expression_without_the_guard_fails_the_check() {
    // Without the guard, `$this.toString()` is checked against `Period`, which
    // has no `toString()` of its own -- so the check fails. This is the control
    // for `path.check.005`: the two cases are byte-identical apart from the
    // guard.
    let message = check_failure_at(
        "$this.toString().length() > 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(
        !message.is_empty(),
        "the semantic error must carry a message"
    );
    expect_case("isSemanticChecks", "isEffectOnLogic3Base");

    // And with no `contextType` at all the same expression checks cleanly,
    // because the focus is then the whole `effective[x]` choice set -- which is
    // `isEffectOnLogic1Base`.
    expect_case("isSemanticChecks", "isEffectOnLogic1Base");
    let (suite, _) = pinned();
    assert!(
        suite
            .case("isSemanticChecks", "isEffectOnLogic1Base")
            .expect("resolves")
            .context_types
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// path.check.007 -- the `is()` function narrows as the operator does
// ---------------------------------------------------------------------------

// covers: path.check.007
#[test]
fn the_is_function_narrows_the_type_as_the_is_operator_does() {
    // The guard may be written either way round, and both narrow: the suite
    // grades the function form with `isGuardFnImplies` and the operator form
    // with `isEffectOnLogic3`.
    let types = inferred_at(
        "$this.is(dateTime) implies $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");
    expect_case("isSemanticChecks", "isGuardFnImplies");

    // And the bad form fails either way round too, so the function spelling is
    // not a way around the rule.
    check_failure_at(
        "$this.is(Period) implies $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
    expect_case("isSemanticChecks", "isGuardFnImpliesBad");
    expect_case("isSemanticChecks", "isGuardImpliesBad");
}

// ---------------------------------------------------------------------------
// path.check.008 -- `and` narrows the right operand
// ---------------------------------------------------------------------------

// covers: path.check.008
#[test]
fn and_narrows_the_right_operand_assuming_the_left_is_true() {
    // `and` is the one operator where the right operand may assume the left,
    // and the group grades both polarities plus the case where the guard sits
    // on the other side of an unrelated condition.
    for name in [
        "isGuardAnd",
        "isGuardAndOtherImplies",
        "isGuardOtherAndImplies",
    ] {
        expect_case("isSemanticChecks", name);
    }
    expect_case("isSemanticChecks", "isGuardAndBad");

    // Stated directly: with `Period` as the context type, the guarded form
    // checks and the unguarded form does not.
    let types = inferred_at(
        "$this is dateTime and $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");
    check_failure_at(
        "$this is Period and $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
}

// ---------------------------------------------------------------------------
// path.check.009 -- `or` narrows assuming the left is false
// ---------------------------------------------------------------------------

// covers: path.check.009
#[test]
fn or_narrows_the_right_operand_assuming_the_left_is_false() {
    // `$this is Period or ...` lets the right operand assume the focus is
    // **not** a Period, which with a single `Period` context type narrows to
    // nothing and so cannot be wrong (`isGuardOr`), while with
    // `Period|Timing` it narrows to `Timing` and the operand is wrong
    // (`isGuardOrBad`).
    for name in [
        "isGuardOr",
        "isGuardOr2",
        "isGuardOrBoth",
        "isGuardOrInnerImplies",
    ] {
        expect_case("isSemanticChecks", name);
    }
    for name in [
        "isGuardOrBad",
        "isGuardOrTypesImpliesBad",
        "isGuardOrFnTypesImpliesBad",
        "isGuardOrOtherImpliesBad",
        "isGuardOrOtherImpliesNoParensBad",
        "isGuardOrInnerImpliesBad",
    ] {
        expect_case("isSemanticChecks", name);
    }

    // The two that differ only by `contextType` are one of the four
    // `hapi-cannot-pass` pairs (`path.oracle.016`), so the context really is
    // read.
    let types = inferred_at(
        "$this is Period or $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");
    check_failure_at(
        "$this is Period or $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );

    // `xor` narrows nothing at all, which is why `isGuardXorBad` fails where
    // the `or` form passes.
    expect_case("isSemanticChecks", "isGuardXorBad");
    expect_case("isSemanticChecks", "isGuardXorBad2");
}

// ---------------------------------------------------------------------------
// path.check.010 -- `iif` narrows its branches by the condition's polarity
// ---------------------------------------------------------------------------

// covers: path.check.010
#[test]
fn iif_narrows_its_branches_by_the_conditions_polarity() {
    // The then-branch assumes the condition is true and the else-branch assumes
    // it is false, so the same guard makes one branch safe and the other not.
    expect_case("isSemanticChecks", "isGuardIifTrue");
    expect_case("isSemanticChecks", "isGuardIifElse");
    expect_case("isSemanticChecks", "isGuardIifBad");

    // then-branch under a true `is dateTime` guard: safe.
    let types = inferred_at(
        "iif($this is dateTime, $this.toString().length() >= 10, true)",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");

    // else-branch under a false `is Period` guard: also safe, and it is the
    // *other* branch that is narrowed.
    let types = inferred_at(
        "iif($this is Period, true, $this.toString().length() >= 10)",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");

    // ...and putting the unsafe operand in the branch the guard does not
    // protect fails.
    check_failure_at(
        "iif($this is Period, $this.toString().length() >= 10, true)",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
}

// ---------------------------------------------------------------------------
// path.check.011 -- a guard narrowing to the empty type set
// ---------------------------------------------------------------------------

// covers: path.check.011
#[test]
fn a_guard_that_narrows_to_the_empty_type_set_is_not_an_error() {
    // The group's own note is the specification here: when a guard narrows the
    // focus to no type at all, the guarded operand is unreachable, and
    // unreachable is **not** wrong. `isGuardImpliesEmpty` is the case:
    // `$this is dateTime` over a `Period|Timing` focus can never be true.
    let types = inferred_at(
        "$this is dateTime implies $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
    assert!(
        !types.is_empty(),
        "an unreachable operand must not fail the check: {types}"
    );
    expect_case("isSemanticChecks", "isGuardImpliesEmpty");

    // The group carries its note in the pinned file, and the harness reads it,
    // so the specification is traceable to the oracle rather than only to the
    // delta.
    let (suite, _) = pinned();
    let group = suite
        .groups
        .iter()
        .find(|g| g.name == "isSemanticChecks")
        .expect("the group exists");
    assert_eq!(group.mode.as_deref(), Some("semantic"));
    assert_eq!(group.cases.len(), 38);

    // The same shape with a guard that *is* reachable fails, so the exemption
    // is specifically about the empty narrowing.
    check_failure_at(
        "$this is Period implies $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
}

// ---------------------------------------------------------------------------
// path.check.012 -- a contextType naming several types
// ---------------------------------------------------------------------------

// covers: path.check.012
#[test]
fn a_context_type_naming_several_types_checks_against_all_of_them() {
    // `contextType="Period|Timing"` splits on `|`, and the check considers the
    // focus to be **either** type -- which is why a guard that excludes only
    // one of them leaves the other reachable and the operand wrong.
    let (suite, _) = pinned();
    let case = suite
        .case("isSemanticChecks", "isGuardOrBad")
        .expect("the case resolves");
    assert_eq!(case.context_types, ["Period", "Timing"]);

    // With both types the operand is reachable through `Timing`, so the check
    // fails...
    check_failure_at(
        "$this is Period or $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
    // ...and with only `Period` it is not, so the check passes. The two differ
    // in nothing but the context type list.
    let types = inferred_at(
        "$this is Period or $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period"],
    );
    assert!(!types.is_empty(), "{types}");

    // Excluding both types makes it unreachable again, which is `isGuardOrBoth`.
    let types = inferred_at(
        "$this is Period or $this is Timing or $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
    assert!(!types.is_empty(), "{types}");

    // Exactly four cases in the pinned file carry a multi-type `contextType`,
    // so a re-pin that adds one is visible.
    let multi = suite.cases().filter(|c| c.context_types.len() > 1).count();
    assert_eq!(multi, 11, "eleven cases carry a `|`-separated contextType");
}

// ---------------------------------------------------------------------------
// path.check.013 -- the check pass can be skipped
// ---------------------------------------------------------------------------

// covers: path.check.013
#[test]
fn the_check_pass_can_be_skipped() {
    let patient = load_input("patient-example.xml");
    let context = context_for(Some(&patient));

    // `FhirPathR4` never calls `check` at all: its `evaluate` goes straight
    // from `parse` to `myEngine.evaluate`. So evaluation must proceed with no
    // check pass having run.
    let mut engine = fresh_engine();
    let parsed = engine.parse("Patient.name.given").expect("parses");
    let values = engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("evaluation must not require the check pass to have run");
    assert_eq!(values.len(), 5);

    // And an expression the check pass **would** reject still evaluates when
    // the check is skipped, which is what makes skipping observable rather than
    // merely allowed. `Observation.valueQuantity` is a semantic error
    // (`path.navigation.007`) and navigates to nothing at runtime.
    let mut engine = fresh_engine();
    let parsed = engine.parse("Patient.nosuchelement").expect("parses");
    let values = engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("with no check pass, an unknown element is simply empty");
    assert!(
        values.is_empty(),
        "skipping the check leaves the runtime answer empty: {values:?}"
    );

    // The check itself still rejects it, so the two passes disagree by design.
    let mut engine = fresh_engine();
    let parsed = engine.parse("Patient.nosuchelement").expect("parses");
    assert!(
        engine
            .check(parsed, &context, StrictnessFlags::CHECK)
            .is_err(),
        "the check pass rejects what the evaluator tolerates"
    );
}

// ---------------------------------------------------------------------------
// path.check.014 -- the harness records which stage failed
// ---------------------------------------------------------------------------

// covers: path.check.014
#[test]
fn the_harness_records_which_stage_each_failure_came_from() {
    let (suite, report) = pinned();

    // Every case that expects a failure must be graded against the stage the
    // failure came from, so a `syntax`, `semantic` or `execution` expectation
    // cannot be satisfied by the wrong kind of failure.
    for (stage, expected) in [
        (Stage::Parse, 7usize),
        (Stage::Check, 29),
        (Stage::Evaluate, 27),
    ] {
        assert_eq!(
            suite.invalid_count(stage),
            expected,
            "the pinned file carries {expected} {} cases",
            stage.as_str()
        );
    }

    // For each of them the report carries both the expectation and the stage
    // the run actually failed at, and they agree. The run must have graded the
    // case first: a `did-not-run` records a stage too, and comparing it would
    // report "the wrong stage" when the real reason is that nothing ran.
    let mut mismatched = Vec::new();
    for case in suite.cases().filter(|c| c.expected_failure.is_some()) {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        assert_ne!(
            r.status,
            CaseStatus::DidNotRun,
            "{}: the stage attribution is only meaningful for a case that ran -- {}",
            r.address,
            r.reason
        );
        assert_eq!(r.expected_failure, case.expected_failure);
        // The two `documentFunctions` semantic cases are the recorded exception
        // (`path.check.002`): they fail at parse because the function does not
        // exist.
        let allowed = case.address.group == "documentFunctions";
        if r.failed_stage != case.expected_failure && !allowed {
            mismatched.push(format!(
                "{}: expected a {} failure, got {:?} -- {}",
                case.address,
                case.expected_failure.expect("filtered").as_str(),
                r.failed_stage.map(|s| s.as_str()),
                r.reason
            ));
        }
    }
    assert!(
        mismatched.is_empty(),
        "{} case(s) failed at the wrong stage; the first few:\n  {}",
        mismatched.len(),
        mismatched[..mismatched.len().min(5)].join("\n  ")
    );

    // And the report distinguishes a check failure from an evaluation failure
    // for a case that expects neither: a passing case records no stage at all.
    let clean = report
        .get(
            &suite
                .case("testBasics", "testSimple")
                .expect("resolves")
                .address
                .to_string(),
        )
        .expect("appears in the report");
    assert_eq!(clean.status, CaseStatus::Passed, "{}", clean.reason);
    assert_eq!(clean.failed_stage, None);

    // The flags in force for each stage are recorded too, so a disagreement
    // with HAPI can be attributed (`path.strictness.002`).
    assert!(!clean.check_flags.enforce_as_singleton_rule);
    assert!(clean.evaluate_flags.enforce_as_singleton_rule);
    assert!(eval_is_available());
}

/// The evaluator must be reachable for the stage attribution above to mean
/// anything: a run in which nothing evaluated would record no stages at all.
fn eval_is_available() -> bool {
    let patient = load_input("patient-example.xml");
    let mut engine = fresh_engine();
    let parsed = engine.parse("1 + 1").expect("parses");
    let context = context_for(Some(&patient));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .expect("checks");
    engine
        .evaluate(parsed, Some(&patient), &context, StrictnessFlags::EVALUATE)
        .expect("evaluates")
        == primitive("integer", "2")
}
