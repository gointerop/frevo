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
//! Three facts shape several tests here:
//!
//! - **27 of them fail at the check stage, not 29.**
//!   `documentFunctions/testResourcesAbstract3` and
//!   `documentFunctions/testHasCoding11` name `resources()` and `hasCoding()`,
//!   which the R4 `ExpressionNode.Function` enum does not have. The parser
//!   consults the host service only after `Function.fromCode` returns null
//!   (FHIRPathEngine.java:1158-1166) and `FHIRPathTestEvaluationServices`
//!   **raises** from `resolveFunction`, so those two fail at the **parse**
//!   stage. The harness grades a `semantic` expectation against the stage the
//!   failure came from (`path.oracle.005`, `tests/oracle/run.rs::graded_failure`),
//!   so a parse-stage failure of either is graded `failed` -- as HAPI's own run
//!   grades it (`FHIRPathTests.java:201`). `path.check.002` as written requires
//!   all 29 to fail the check; that contradiction is open in
//!   `tasks/blockers.md` and the test below is left as the spec states it.
//! - **six `isSemanticChecks` groups, 13 cases, are byte-identical apart from
//!   `context` and `contextType`, with differing expected outcomes.** HAPI's
//!   pinned R4 harness reads neither attribute, so it necessarily fails at
//!   least one of each group (`path.oracle.016`); Frevo is graded on its own
//!   context-aware run.
//! - **the check pass does not narrow on a type guard**, because the pinned
//!   engine does not (`path.check.005` to `.012`, owner ruling of 2026-11-12):
//!   the 14 `isSemanticChecks` cases that expect success but guard a
//!   `toString()` over a non-primitive focus end at the check stage graded
//!   `failed`, which is parity with HAPI.
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use frevo_path::{CheckError, CheckOutcome, TypeSet, WarningId};

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
// `SuiteEngine::check` answers pass or fail, which is all the suite needs.
// Several scenarios here are about the inferred **type**, or about a context the
// suite does not carry, so they drive this seam instead. It is bound over the
// oracle harness's own check binding (`oracle::engine::check_expression`), so it
// checks with exactly the model, host services and focus rules the suite run
// grades with. Anything the pass cannot answer yet comes back
// `NotImplemented`, which the helpers below report as a failure to run --
// never as a pass and never as an empty type set.
// ---------------------------------------------------------------------------

/// Why a type check produced no type set.
#[derive(Debug)]
enum CheckFailure {
    /// A real check verdict: upstream's message constant and its text.
    Diagnostic { id: &'static str, message: String },
    /// The pass, or the model it was given, cannot answer yet. Not a verdict.
    NotRun(String),
}

impl std::fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckFailure::Diagnostic { id, message } => write!(f, "{id}: {message}"),
            CheckFailure::NotRun(why) => write!(f, "did not run: {why}"),
        }
    }
}

trait TypeCheckEngine {
    /// Check `expr` against a focus described by `context` and `context_types`
    /// and return the inferred type set and the warnings.
    ///
    /// `context_types` may name several types, which is what the suite's
    /// `contextType="Period|Timing"` means (`path.check.012`).
    fn check_types(
        &mut self,
        expr: &str,
        root_type: &str,
        context: Option<&str>,
        context_types: &[&str],
    ) -> Result<CheckOutcome, CheckFailure>;
}

fn type_check_engine_under_test() -> Box<dyn TypeCheckEngine> {
    Box::new(HarnessTypeCheck)
}

/// The check pass, through the same binding the oracle harness grades with.
///
/// `root_type` is the case's resource type: `%resource` is typed as it and
/// `%rootResource` as `Resource`, as `FHIRPathTests` checks
/// (`fp.check(res, "Resource", type, type, node)`). With a `context_types`
/// list the focus is those types (`checkOnTypes`); otherwise it is `context`,
/// or with neither the resource type itself.
struct HarnessTypeCheck;

impl TypeCheckEngine for HarnessTypeCheck {
    fn check_types(
        &mut self,
        expr: &str,
        root_type: &str,
        context: Option<&str>,
        context_types: &[&str],
    ) -> Result<CheckOutcome, CheckFailure> {
        // A parse failure is not a check verdict, so it must never satisfy
        // `check_failure_at`: every expression here is one the suite parses.
        let parsed = frevo_path::Expression::parse(expr).unwrap_or_else(|e| {
            panic!(
                "{expr:?} must parse before it is checked: {} (at offset {})",
                e.message(),
                e.offset()
            )
        });
        let context_types: Vec<String> = context_types.iter().map(|t| t.to_string()).collect();
        let case_context = CaseContext {
            root_resource_type: Some(root_type),
            resource_type: Some(root_type),
            context,
            context_types: &context_types,
            ..CaseContext::default()
        };
        match oracle::engine::check_expression(&parsed, &case_context, &EngineConfig::default()) {
            Err(e) => Err(CheckFailure::NotRun(e.to_string())),
            Ok(Ok(outcome)) => Ok(outcome),
            Ok(Err(e @ CheckError::NotImplemented { .. })) => {
                Err(CheckFailure::NotRun(e.to_string()))
            }
            Ok(Err(CheckError::Failed { message, id, .. })) => {
                Err(CheckFailure::Diagnostic { id, message })
            }
        }
    }
}

/// The inferred type set for `expr`, requiring the check to succeed.
fn inferred(expr: &str, root_type: &str) -> TypeSet {
    let mut engine = type_check_engine_under_test();
    engine
        .check_types(expr, root_type, None, &[])
        .unwrap_or_else(|e| panic!("checking {expr:?} against {root_type} must succeed: {e}"))
        .types
}

/// The check outcome for `expr` checked at `context` with `context_types`,
/// requiring the check to succeed.
fn outcome_at(expr: &str, root_type: &str, context: &str, context_types: &[&str]) -> CheckOutcome {
    let mut engine = type_check_engine_under_test();
    engine
        .check_types(expr, root_type, Some(context), context_types)
        .unwrap_or_else(|e| {
            panic!("checking {expr:?} at {context} as {context_types:?} must succeed: {e}")
        })
}

/// `expr` must fail the check at `context` with `context_types`, with a real
/// diagnostic; answers its message id and its message.
fn check_failure_at(
    expr: &str,
    root_type: &str,
    context: &str,
    context_types: &[&str],
) -> (&'static str, String) {
    let mut engine = type_check_engine_under_test();
    match engine.check_types(expr, root_type, Some(context), context_types) {
        Ok(o) => panic!(
            "checking {expr:?} at {context} as {context_types:?} must fail, got {}",
            o.types
        ),
        Err(CheckFailure::Diagnostic { id, message }) => (id, message),
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
    }
}

/// The `FHIRPATH_PRIMITIVE_ONLY` diagnostic for `toString`, exactly: the
/// template at `Messages.properties:353` with `{0}` bound to `toString`, which
/// `makeException` passes through unchanged and suffixes with no location,
/// because neither `FHIRPathTests` nor the harness sets one
/// (`FHIRPathEngine.java:584-594`). `path.check.005` states it, and
/// `path.check.006` to `.012` refer to it.
const PRIMITIVE_ONLY_ID: &str = "FHIRPATH_PRIMITIVE_ONLY";
const PRIMITIVE_ONLY_TO_STRING: &str =
    "Error evaluating FHIRPath expression: The function toString can only be used on primitives";

/// `expr`, checked at `Observation.effective` as `context_types`, must fail
/// with exactly the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for `toString`.
fn fails_primitive_only(expr: &str, context_types: &[&str]) {
    let (id, message) =
        check_failure_at(expr, "Observation", "Observation.effective", context_types);
    assert_eq!(
        (id, message.as_str()),
        (PRIMITIVE_ONLY_ID, PRIMITIVE_ONLY_TO_STRING),
        "{expr:?} as {context_types:?} must fail with the primitive-only diagnostic for toString"
    );
}

/// `expr`, checked at `Observation.effective` as `context_types`, must pass
/// the check; answers the outcome. An empty `context_types` is a case with no
/// `contextType`, whose focus is the whole `effective[x]` choice set.
fn passes_at_effective(expr: &str, context_types: &[&str]) -> CheckOutcome {
    outcome_at(expr, "Observation", "Observation.effective", context_types)
}

/// The check verdict for `expr` at `Observation.effective` as `context_types`:
/// `None` when it passes, the message id and message when it fails. Panics
/// when the pass did not run, so "did not run" can never compare equal to a
/// verdict.
fn verdict_at_effective(expr: &str, context_types: &[&str]) -> Option<(&'static str, String)> {
    let mut engine = type_check_engine_under_test();
    match engine.check_types(
        expr,
        "Observation",
        Some("Observation.effective"),
        context_types,
    ) {
        Ok(_) => None,
        Err(CheckFailure::Diagnostic { id, message }) => Some((id, message)),
        Err(e) => panic!("{expr:?} as {context_types:?} must be checked: {e}"),
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

/// The run report's entry for `group/name`.
fn report_for(group: &str, name: &str) -> &'static run::CaseReport {
    let (suite, report) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    report
        .get(&case.address.to_string())
        .unwrap_or_else(|| panic!("{} must appear in the run report", case.address))
}

/// `group/name` must end at the **check** stage with `status`: `Failed` for a
/// case that expects success, `Passed` for an `invalid="semantic"` one.
fn expect_at_check(group: &str, name: &str, status: CaseStatus) {
    let r = report_for(group, name);
    assert_eq!(
        (r.status, r.failed_stage),
        (status, Some(Stage::Check)),
        "{}: {:?} must end at the check stage graded {}, but is {} at {:?} -- {}",
        r.address,
        r.expression,
        status.as_str(),
        r.status.as_str(),
        r.failed_stage.map(|s| s.as_str()),
        r.reason
    );
}

/// `group/name` passes the check, so its graded status is decided by
/// evaluation. Only that much is asserted: the run report must not attribute
/// it to the parse or the check stage. What evaluation then gives it is that
/// scenario's business, not the check pass's.
fn expect_decided_by_evaluation(group: &str, name: &str) {
    let r = report_for(group, name);
    assert!(
        !matches!(r.failed_stage, Some(Stage::Parse | Stage::Check)),
        "{}: {:?} must pass the check and reach evaluation, but stopped at {:?} ({}) -- {}",
        r.address,
        r.expression,
        r.failed_stage.map(|s| s.as_str()),
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
    let (_, message) =
        check_failure_at("Patient.nosuchelement", "Patient", "Patient", &["Patient"]);
    assert!(
        message.contains("nosuchelement"),
        "the error must name the element: {message}"
    );

    // The same for a nested element, so the rule is not only about the leading
    // name.
    let (_, message) = check_failure_at(
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
// path.check.005 -- an `is` guard in `implies` does not narrow
// ---------------------------------------------------------------------------

// covers: path.check.005
#[test]
fn an_is_guard_in_implies_does_not_narrow_the_type_the_other_operand_is_checked_against() {
    // HAPI 8.12.0's `executeType` types every operand of an operator chain
    // against the same focus (FHIRPathEngine.java:1621-1637); `is` and
    // `implies` type to `Boolean` and leave the focus alone (`:2013-2015`,
    // `:2027`). So `toString()` on the right is checked against the declared
    // `Period`, and fails exactly as the unguarded expression does.
    let guarded = "$this is dateTime implies $this.toString().length() > 10";
    fails_primitive_only(guarded, &["Period"]);

    // So the three implies-guard cases the suite expects to pass the check end
    // at the check stage graded `failed` -- parity with HAPI, whose harness
    // checks them against the `Observation` root and fails them too.
    for name in [
        "isEffectOnLogic3",
        "isGuardOrTypesImplies",
        "isGuardNotImplies",
    ] {
        expect_at_check("isSemanticChecks", name, CaseStatus::Failed);
    }
    fails_primitive_only(
        "($this is dateTime or $this is instant) implies $this.toString().length() >= 10",
        &["Period"],
    );
    fails_primitive_only(
        "($this is Period).not() implies $this.toString().length() >= 10",
        &["Period"],
    );

    // The same guard passes the check when the focus includes a primitive:
    // with `dateTime` as the context type, and with no context type, where the
    // focus is the whole `effective[x]` choice set. Those cases' graded status
    // is decided by evaluation.
    passes_at_effective(guarded, &["dateTime"]);
    passes_at_effective(guarded, &[]);
    passes_at_effective(
        "$this is dateTime implies $this.toString().length() >= 10",
        &[],
    );
    passes_at_effective(
        "$this is Period implies ($this is dateTime implies $this.toString().length() >= 10)",
        &["dateTime"],
    );
    passes_at_effective(
        "($this is dateTime or $this is Period) implies $this.toString().length() >= 10",
        &[],
    );
    for name in [
        "isEffectOnLogic2",
        "isEffectOnLogic1",
        "isGuardOrTypesImpliesAll",
    ] {
        expect_decided_by_evaluation("isSemanticChecks", name);
    }

    // Every implies-guard case the suite expects to fail the check fails it
    // and is graded `passed`.
    for name in [
        "isGuardImpliesBad",
        "isGuardOrTypesImpliesBad",
        "isGuardOrFnTypesImpliesBad",
        "isGuardNotImpliesBad",
        "isGuardSupertypeBad",
        "isGuardOtherExpressionBad",
    ] {
        expect_at_check("isSemanticChecks", name, CaseStatus::Passed);
    }
    // HAPI's recorded status for these cases (`hapi-cannot-pass` for
    // `isGuardNotImplies`, `isGuardOrTypesImpliesAll`,
    // `isGuardOrTypesImpliesBad` and `isGuardNotImpliesBad`) is asserted, with
    // the rest of the 13, by `path_oracle.rs`'s `path.oracle.016` test.
}

// ---------------------------------------------------------------------------
// path.check.006 -- the same expression without the guard fails
// ---------------------------------------------------------------------------

// covers: path.check.006
#[test]
fn the_same_expression_without_the_guard_fails_the_check() {
    // Without the guard, `$this.toString()` is checked against `Period`, which
    // is not a primitive -- so the check fails, with the same
    // `FHIRPATH_PRIMITIVE_ONLY` diagnostic as the guarded form of
    // `path.check.005`.
    let unguarded = "$this.toString().length() > 10";
    let guarded = "$this is dateTime implies $this.toString().length() > 10";
    fails_primitive_only(unguarded, &["Period"]);
    expect_at_check(
        "isSemanticChecks",
        "isEffectOnLogic3Base",
        CaseStatus::Passed,
    );

    // With `dateTime` as the context type (`isEffectOnLogic2Base`) or with no
    // context type at all (`isEffectOnLogic1Base`), where the focus is the
    // whole `effective[x]` choice set, the same expression passes the check.
    passes_at_effective(unguarded, &["dateTime"]);
    passes_at_effective(unguarded, &[]);
    expect_decided_by_evaluation("isSemanticChecks", "isEffectOnLogic2Base");
    expect_decided_by_evaluation("isSemanticChecks", "isEffectOnLogic1Base");
    // `isEffectOnLogic1Base` carries no `contextType`, and with the context
    // honoured it evaluates `true` over the example's `dateTime`.
    expect_case("isSemanticChecks", "isEffectOnLogic1Base");
    let (suite, _) = pinned();
    assert!(
        suite
            .case("isSemanticChecks", "isEffectOnLogic1Base")
            .expect("resolves")
            .context_types
            .is_empty()
    );

    // For each of the three context types the guarded and the unguarded
    // expression get the same check outcome, message for message: the guard
    // changes nothing the check pass observes.
    for context_types in [&["Period"][..], &["dateTime"][..], &[][..]] {
        assert_eq!(
            verdict_at_effective(guarded, context_types),
            verdict_at_effective(unguarded, context_types),
            "the guard must not change the check outcome as {context_types:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// path.check.007 -- the `is()` function does not narrow; `ofType` and `as` do
// ---------------------------------------------------------------------------

// covers: path.check.007
#[test]
fn the_is_function_does_not_narrow_and_only_of_type_and_as_retype_the_focus() {
    // The `is()` function types to `Boolean` and leaves the focus unchanged
    // (FHIRPathEngine.java:3401-3404), exactly as the operator does, so the
    // function spelling of the guard narrows nothing either.
    fails_primitive_only(
        "$this.is(dateTime) implies $this.toString().length() >= 10",
        &["Period"],
    );
    expect_at_check("isSemanticChecks", "isGuardFnImplies", CaseStatus::Failed);

    // The bad form fails the check and is graded `passed`.
    check_failure_at(
        "$this.is(Period) implies $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
    expect_at_check(
        "isSemanticChecks",
        "isGuardFnImpliesBad",
        CaseStatus::Passed,
    );
    expect_at_check("isSemanticChecks", "isGuardImpliesBad", CaseStatus::Passed);

    // `ofType()` types its result as the named type whatever the focus
    // (`:3377-3384`), and `as` as the type named on its right (`:2016-2021`),
    // so these two pass the check over the same `Period` focus --
    // `isGuardOfType` and `isGuardAs`.
    let of_type = passes_at_effective(
        "$this.ofType(dateTime).toString().length() >= 10",
        &["Period"],
    );
    passes_at_effective("($this as dateTime).toString().length() >= 10", &["Period"]);
    expect_decided_by_evaluation("isSemanticChecks", "isGuardOfType");
    expect_decided_by_evaluation("isSemanticChecks", "isGuardAs");

    // The `ofType(dateTime)` form also yields an of-type-impossible warning,
    // because a `Period` focus can never be a `dateTime` (`:3782-3784`); the
    // warning does not fail the check (`path.check.015`).
    assert!(
        of_type
            .warnings
            .iter()
            .any(|w| w.id == WarningId::OfTypeImpossible),
        "ofType(dateTime) over Period must warn of-type-impossible: {:?}",
        of_type.warnings
    );
}

// ---------------------------------------------------------------------------
// path.check.008 -- `and` checks its right operand against the unnarrowed focus
// ---------------------------------------------------------------------------

// covers: path.check.008
#[test]
fn and_does_not_narrow_its_right_operand() {
    // `and` types to `Boolean` (FHIRPathEngine.java:2025) and its right
    // operand is typed against the same focus as its left (`:1621-1637`).
    fails_primitive_only(
        "$this is dateTime and $this.toString().length() >= 10",
        &["Period"],
    );

    // So the three `and`-guard cases the suite expects to pass end at the check
    // stage graded `failed`, as they are for HAPI. Their
    // `%resource.status = 'final'` operand types cleanly and does not change
    // which diagnostic is raised.
    for name in [
        "isGuardAnd",
        "isGuardAndOtherImplies",
        "isGuardOtherAndImplies",
    ] {
        expect_at_check("isSemanticChecks", name, CaseStatus::Failed);
    }
    fails_primitive_only(
        "($this is dateTime and %resource.status = 'final') implies $this.toString().length() >= 10",
        &["Period"],
    );
    fails_primitive_only(
        "(%resource.status = 'final' and $this is dateTime) implies $this.toString().length() >= 10",
        &["Period"],
    );

    // `isGuardAndBad` fails the check and is graded `passed`.
    expect_at_check("isSemanticChecks", "isGuardAndBad", CaseStatus::Passed);
    check_failure_at(
        "$this is Period and $this.toString().length() >= 10",
        "Observation",
        "Observation.effective",
        &["Period", "Timing"],
    );
}

// ---------------------------------------------------------------------------
// path.check.009 -- `or` and `xor` check their right operand unnarrowed
// ---------------------------------------------------------------------------

// covers: path.check.009
#[test]
fn or_and_xor_do_not_narrow_their_right_operand() {
    // `or` types to `Boolean` (FHIRPathEngine.java:2024) and does not assume
    // its left operand false when typing its right.
    let or = "$this is Period or $this.toString().length() >= 10";
    let xor = "$this is dateTime xor $this.toString().length() >= 10";
    fails_primitive_only(or, &["Period"]);

    // So the three `or`-guard cases the suite expects to pass end at the check
    // stage, and Frevo grades all three `failed`.
    for name in ["isGuardOr", "isGuardOrBoth", "isGuardOrInnerImplies"] {
        expect_at_check("isSemanticChecks", name, CaseStatus::Failed);
    }
    fails_primitive_only(
        "$this is Period or (%resource.status = 'final' implies $this.toString().length() >= 10)",
        &["Period"],
    );

    // `isGuardOr` and `isGuardOrInnerImplies` are `hapi-cannot-pass` because
    // each belongs to a `path.oracle.016` group of byte-identical expressions
    // that differ only by `contextType`: `isGuardOr` with `isGuardOr2` and
    // `isGuardOrBad`, `isGuardOrInnerImplies` with `isGuardOrInnerImpliesBad`.
    // HAPI's statuses are asserted by `path_oracle.rs`; the grouping is
    // asserted here.
    let (suite, _) = pinned();
    let expression = |name: &str| {
        suite
            .case("isSemanticChecks", name)
            .unwrap_or_else(|| panic!("isSemanticChecks/{name} resolves"))
            .expression
            .clone()
    };
    for name in ["isGuardOr", "isGuardOr2", "isGuardOrBad"] {
        assert_eq!(expression(name), or, "{name} shares the `or` expression");
    }
    assert_eq!(
        expression("isGuardOrInnerImplies"),
        expression("isGuardOrInnerImpliesBad")
    );

    // The `or` and `xor` cases the suite expects to fail the check fail it,
    // and Frevo grades them `passed`.
    for name in [
        "isGuardOrBad",
        "isGuardOrOtherImpliesBad",
        "isGuardOrOtherImpliesNoParensBad",
        "isGuardOrInnerImpliesBad",
        "isGuardXorBad",
        "isGuardOrTypesImpliesBad",
        "isGuardOrFnTypesImpliesBad",
    ] {
        expect_at_check("isSemanticChecks", name, CaseStatus::Passed);
    }
    fails_primitive_only(xor, &["Period"]);
    fails_primitive_only(or, &["Period", "Timing"]);

    // With `Period|dateTime` the same `or` expression (`isGuardOr2`) and its
    // `xor` counterpart (`isGuardXorBad2`) pass the check, because the focus
    // then includes a primitive (`path.check.012`); their graded status is
    // decided by evaluation.
    passes_at_effective(or, &["Period", "dateTime"]);
    passes_at_effective(xor, &["Period", "dateTime"]);
    assert_eq!(expression("isGuardXorBad2"), xor);
    assert_eq!(expression("isGuardXorBad"), xor);
    expect_decided_by_evaluation("isSemanticChecks", "isGuardOr2");
    expect_decided_by_evaluation("isSemanticChecks", "isGuardXorBad2");
}

// ---------------------------------------------------------------------------
// path.check.010 -- `iif` checks both branches against the unnarrowed focus
// ---------------------------------------------------------------------------

// covers: path.check.010
#[test]
fn iif_does_not_narrow_either_branch() {
    // `iif`'s arguments are not expression parameters
    // (FHIRPathEngine.java:3804-3816), so each is typed against the same
    // `$this` (`:3261-3271`), and its result is the union of its branch types
    // with no reference to the condition (`:3441-3449`). The diagnostic is the
    // same whichever branch holds `toString()`.
    fails_primitive_only(
        "iif($this is dateTime, $this.toString().length() >= 10, true)",
        &["Period"],
    );
    fails_primitive_only(
        "iif($this is Period, true, $this.toString().length() >= 10)",
        &["Period"],
    );
    fails_primitive_only(
        "iif($this is Period, $this.toString().length() >= 10, true)",
        &["Period"],
    );

    // So `isGuardIifTrue` and `isGuardIifElse` end at the check stage graded
    // `failed`, as they are for HAPI, and `isGuardIifBad` fails the check and
    // is graded `passed`.
    expect_at_check("isSemanticChecks", "isGuardIifTrue", CaseStatus::Failed);
    expect_at_check("isSemanticChecks", "isGuardIifElse", CaseStatus::Failed);
    expect_at_check("isSemanticChecks", "isGuardIifBad", CaseStatus::Passed);
}

// ---------------------------------------------------------------------------
// path.check.011 -- a guard that can never be true makes nothing unreachable
// ---------------------------------------------------------------------------

// covers: path.check.011
#[test]
fn a_guard_that_can_never_be_true_leaves_the_other_operand_checked_as_usual() {
    // `$this is dateTime` over a `Period|Timing` focus can never be true, but
    // the pinned engine has no notion of an unreachable operand: the right
    // operand is checked as usual and fails. The group's note asking for an
    // exemption (tests-fhir-r4.xml:1695-1702) describes behaviour HAPI 8.12.0
    // does not have.
    fails_primitive_only(
        "$this is dateTime implies $this.toString().length() >= 10",
        &["Period", "Timing"],
    );
    fails_primitive_only(
        "$this is Period implies ($this is dateTime implies $this.toString().length() >= 10)",
        &["Period", "Timing"],
    );

    // So `isGuardImpliesEmpty` and `isGuardNested` end at the check stage
    // graded `failed`, as they are for HAPI.
    expect_at_check(
        "isSemanticChecks",
        "isGuardImpliesEmpty",
        CaseStatus::Failed,
    );
    expect_at_check("isSemanticChecks", "isGuardNested", CaseStatus::Failed);

    // The group carries its note in the pinned file, and the harness reads the
    // group, so what the note asks for is traceable to the oracle.
    let (suite, _) = pinned();
    let group = suite
        .groups
        .iter()
        .find(|g| g.name == "isSemanticChecks")
        .expect("the group exists");
    assert_eq!(group.mode.as_deref(), Some("semantic"));
    assert_eq!(group.cases.len(), 38);

    // The same shape with a guard that *can* be true fails identically, so
    // whether the guard is satisfiable changes nothing the check observes.
    assert_eq!(
        verdict_at_effective(
            "$this is Period implies $this.toString().length() >= 10",
            &["Period", "Timing"],
        ),
        verdict_at_effective(
            "$this is dateTime implies $this.toString().length() >= 10",
            &["Period", "Timing"],
        ),
    );
}

// ---------------------------------------------------------------------------
// path.check.012 -- a contextType naming several types
// ---------------------------------------------------------------------------

// covers: path.check.012
#[test]
fn a_context_type_naming_several_types_checks_against_all_of_them() {
    // `contextType="Period|Timing"` splits on `|`, and the focus is the set of
    // all the named types, as `checkOnTypes` builds it from a type list
    // (FHIRPathEngine.java:477-485).
    let focus = passes_at_effective("$this", &["Period", "Timing"]).types;
    assert!(
        focus.len() == 2 && focus.contains("Period") && focus.contains("Timing"),
        "the focus must be exactly Period and Timing: {focus}"
    );

    // A focus rule is satisfied when **any** type in the set satisfies it
    // (TypeDetails.java:436-448): `$this.toString()` passes with
    // `Period|dateTime` and fails with `Period|Timing`, where neither type is a
    // primitive.
    let or = "$this is Period or $this.toString().length() >= 10";
    let xor = "$this is dateTime xor $this.toString().length() >= 10";
    passes_at_effective("$this.toString()", &["Period", "dateTime"]);
    passes_at_effective(or, &["Period", "dateTime"]);
    passes_at_effective(xor, &["Period", "dateTime"]);
    fails_primitive_only("$this.toString()", &["Period", "Timing"]);
    fails_primitive_only(or, &["Period", "Timing"]);
    fails_primitive_only(
        "$this is Period or $this is Timing or $this.toString().length() >= 10",
        &["Period", "Timing"],
    );

    // Every `Period|Timing` case in the suite, and how each is graded.
    let (suite, _) = pinned();
    let period_timing: [(&str, CaseStatus); 9] = [
        ("isGuardOrBad", CaseStatus::Passed),
        ("isGuardImpliesBad", CaseStatus::Passed),
        ("isGuardFnImpliesBad", CaseStatus::Passed),
        ("isGuardAndBad", CaseStatus::Passed),
        ("isGuardNotImpliesBad", CaseStatus::Passed),
        ("isGuardOrInnerImpliesBad", CaseStatus::Passed),
        ("isGuardImpliesEmpty", CaseStatus::Failed),
        ("isGuardOrBoth", CaseStatus::Failed),
        ("isGuardNested", CaseStatus::Failed),
    ];
    for (name, status) in period_timing {
        let case = suite
            .case("isSemanticChecks", name)
            .unwrap_or_else(|| panic!("isSemanticChecks/{name} resolves"));
        assert_eq!(case.context_types, ["Period", "Timing"], "{name}");
        expect_at_check("isSemanticChecks", name, status);
    }
    // ...and every `Period|dateTime` one, both of which pass the check.
    for name in ["isGuardOr2", "isGuardXorBad2"] {
        let case = suite
            .case("isSemanticChecks", name)
            .unwrap_or_else(|| panic!("isSemanticChecks/{name} resolves"));
        assert_eq!(case.context_types, ["Period", "dateTime"], "{name}");
        expect_decided_by_evaluation("isSemanticChecks", name);
    }
    let in_group = suite
        .cases()
        .filter(|c| c.address.group == "isSemanticChecks" && c.context_types.len() > 1)
        .count();
    assert_eq!(
        in_group, 11,
        "the nine Period|Timing and two Period|dateTime cases"
    );

    // Exactly eleven cases in the pinned file carry a multi-type
    // `contextType`, so a re-pin that adds one is visible.
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
