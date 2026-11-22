//! The three-stage run, and the per-case report.
//!
//! Stages run in the order `FHIRPathTests.test` runs them — parse, check,
//! evaluate — and a case expecting a `syntax`, `semantic` or `execution`
//! failure passes only when the failure comes from the matching stage
//! (`path.oracle.005`, `path.check.014`).

use std::collections::BTreeMap;

use super::compare;
use super::engine::{CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine};
use super::input::{self, InputError, SuiteInput};
use super::suite::{Case, Stage, Suite};

/// The four per-case statuses, and nothing else (`path.oracle.010`).
///
/// `DidNotRun` and `SkippedForLicence` are not passes and are not failures.
/// Keeping them distinct is the reason an unrun case can never be read as a
/// passing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseStatus {
    Passed,
    Failed,
    DidNotRun,
    SkippedForLicence,
}

impl CaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CaseStatus::Passed => "passed",
            CaseStatus::Failed => "failed",
            CaseStatus::DidNotRun => "did-not-run",
            CaseStatus::SkippedForLicence => "skipped-for-licence",
        }
    }

    /// The only status that counts towards the gate.
    pub fn is_pass(self) -> bool {
        matches!(self, CaseStatus::Passed)
    }
}

/// One case's record.
#[derive(Debug, Clone)]
pub struct CaseReport {
    pub address: String,
    pub expression: String,
    pub status: CaseStatus,
    /// Human-readable reason, always populated for a non-pass.
    pub reason: String,
    /// Which stage the run actually failed at, if any.
    pub failed_stage: Option<Stage>,
    /// Which stage `expression/@invalid` said should fail, if any.
    pub expected_failure: Option<Stage>,
    /// `expression/@mode`, recorded so the attribute is handled rather than
    /// unread, and explicitly not used in grading (`path.oracle.017`).
    pub expression_mode: Option<String>,
    /// The flags in force for each stage, so a disagreement with HAPI can be
    /// attributed (`path.strictness.002`).
    pub check_flags: StrictnessFlags,
    pub evaluate_flags: StrictnessFlags,
    /// Set when the case tests one of the seven document functions the R4
    /// engine does not implement (`path.oracle.014`).
    pub out_of_scope_function: Option<String>,
    /// The engine's trace log for this case, drained after it ran
    /// (`path.utility.001`).
    pub trace_log: String,
}

/// The whole run.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub cases: Vec<CaseReport>,
    /// Every input file the run resolved, and whether it loaded.
    pub inputs: BTreeMap<String, Result<InputSummary, InputError>>,
    pub config: EngineConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSummary {
    pub is_xml: bool,
    pub byte_len: usize,
    pub resource_type: Option<String>,
}

impl RunReport {
    pub fn count(&self, status: CaseStatus) -> usize {
        self.cases.iter().filter(|c| c.status == status).count()
    }

    pub fn passed(&self) -> usize {
        self.count(CaseStatus::Passed)
    }

    pub fn get(&self, address: &str) -> Option<&CaseReport> {
        self.cases.iter().find(|c| c.address == address)
    }

    /// Every case whose status is not `passed`, for the gate's per-case list.
    pub fn not_passed(&self) -> Vec<&CaseReport> {
        self.cases.iter().filter(|c| !c.status.is_pass()).collect()
    }
}

/// Run every case in the suite against the bound engine.
pub fn run(suite: &Suite, engine: &mut dyn SuiteEngine, config: EngineConfig) -> RunReport {
    // A configuration failure is not a grading answer: it leaves every case
    // `did-not-run`, which is what the unbound engine produces today.
    let configured = engine.configure(&config);

    // Inputs are loaded once and shared, as `FHIRPathTests` caches them in its
    // `resources` map.
    let mut inputs: BTreeMap<String, Result<SuiteInput, InputError>> = BTreeMap::new();
    for file in suite.referenced_inputs() {
        let loaded = input::load(&file);
        inputs.insert(file, loaded);
    }

    let mut cases = Vec::with_capacity(suite.case_count());
    for case in suite.cases() {
        cases.push(run_case(case, engine, &inputs, configured.as_ref().err()));
    }

    let summaries = inputs
        .iter()
        .map(|(k, v)| {
            let summary = match v {
                Ok(i) => Ok(InputSummary {
                    is_xml: i.is_xml(),
                    byte_len: i.bytes.len(),
                    resource_type: i.resource_type().map(|s| s.to_string()),
                }),
                Err(e) => Err(e.clone()),
            };
            (k.clone(), summary)
        })
        .collect();

    RunReport {
        cases,
        inputs: summaries,
        config,
    }
}

fn did_not_run(case: &Case, reason: String, stage: Option<Stage>) -> CaseReport {
    report(case, CaseStatus::DidNotRun, reason, stage, String::new())
}

/// Drain the engine's trace buffer, as `FHIRPathTests.test` does with
/// `if (fp.hasLog()) { ... fp.takeLog(); }`. Taking the log clears it, which is
/// what stops one case's lines leaking into the next across a shared engine
/// (`path.utility.001`).
fn drain_log(engine: &mut dyn SuiteEngine) -> String {
    if engine.has_log() {
        engine.take_log()
    } else {
        String::new()
    }
}

fn report(
    case: &Case,
    status: CaseStatus,
    reason: String,
    failed_stage: Option<Stage>,
    trace_log: String,
) -> CaseReport {
    CaseReport {
        address: case.address.to_string(),
        expression: case.expression.clone(),
        status,
        reason,
        failed_stage,
        expected_failure: case.expected_failure,
        expression_mode: case.expression_mode.clone(),
        check_flags: StrictnessFlags::CHECK,
        evaluate_flags: StrictnessFlags::EVALUATE,
        out_of_scope_function: case.out_of_scope_function().map(|s| s.to_string()),
        trace_log,
    }
}

/// Classify an engine error that arrived at `stage`.
///
/// Three outcomes, and the first two never become `passed`:
/// `NotImplemented` is `did-not-run`, a licensed-terminology refusal is
/// `skipped-for-licence`, and only a real diagnostic is graded against the
/// case's `invalid` expectation.
enum Graded {
    NotRun(String),
    Licence(String),
    Diagnostic(String),
}

fn classify(err: &EngineError) -> Graded {
    match err {
        EngineError::NotImplemented => Graded::NotRun(err.to_string()),
        EngineError::RequiresLicensedTerminology { .. } => Graded::Licence(err.to_string()),
        EngineError::Diagnostic { .. } => Graded::Diagnostic(err.to_string()),
    }
}

fn run_case(
    case: &Case,
    engine: &mut dyn SuiteEngine,
    inputs: &BTreeMap<String, Result<SuiteInput, InputError>>,
    config_error: Option<&EngineError>,
) -> CaseReport {
    if let Some(e) = config_error {
        return did_not_run(
            case,
            format!("the engine could not be configured: {e}"),
            None,
        );
    }

    // The input, if the case names one. A missing input is `did-not-run`
    // (`path.oracle.003`).
    let input = match &case.inputfile {
        None => None,
        Some(file) => match inputs.get(file) {
            Some(Ok(i)) => Some(i),
            Some(Err(e)) => {
                return did_not_run(case, format!("inputfile {file}: {e}"), None);
            }
            None => {
                return did_not_run(case, format!("inputfile {file} was never resolved"), None);
            }
        },
    };

    let resource_type = input.and_then(|i| i.resource_type());
    let context = CaseContext {
        // `FHIRPathTests` passes the resource type as both the root resource
        // type and the resource type; Frevo's run adds `context`,
        // `contextType`, `mode` and `version`, which the pinned Java harness
        // reads for no case at all (`path.oracle.016`).
        root_resource_type: resource_type,
        resource_type,
        context: case.context.as_deref(),
        context_types: &case.context_types,
        version: case.version.as_deref(),
        test_mode: case.test_mode.as_deref(),
        check_ordered_functions: case.check_ordered_functions,
    };

    // ---- stage 1: parse --------------------------------------------------
    let parsed = match engine.parse(&case.expression) {
        Ok(p) => {
            if case.expected_failure == Some(Stage::Parse) {
                return report(
                    case,
                    CaseStatus::Failed,
                    format!(
                        "expected a syntax failure parsing {:?}, but it parsed",
                        case.expression
                    ),
                    None,
                    drain_log(engine),
                );
            }
            p
        }
        Err(e) => return graded_failure(case, engine, &e, Stage::Parse),
    };

    // ---- stage 2: check --------------------------------------------------
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Ok(()) => {
            if case.expected_failure == Some(Stage::Check) {
                return report(
                    case,
                    CaseStatus::Failed,
                    format!(
                        "expected a semantic failure checking {:?}, but the check passed",
                        case.expression
                    ),
                    None,
                    drain_log(engine),
                );
            }
        }
        Err(e) => return graded_failure(case, engine, &e, Stage::Check),
    }

    // ---- stage 3: evaluate -----------------------------------------------
    let mut outcome = match engine.evaluate(parsed, input, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => {
            if case.expected_failure == Some(Stage::Evaluate) {
                return report(
                    case,
                    CaseStatus::Failed,
                    format!(
                        "expected an execution failure evaluating {:?}, but it evaluated",
                        case.expression
                    ),
                    None,
                    drain_log(engine),
                );
            }
            items
        }
        Err(e) => return graded_failure(case, engine, &e, Stage::Evaluate),
    };

    // The log is drained per case; the engine is shared across the whole
    // suite, so a log that did not clear on take would leak into the next case
    // (`path.utility.001`).
    let trace_log = drain_log(engine);

    // ---- predicate conversion -------------------------------------------
    if case.predicate {
        match engine.convert_to_boolean(&outcome) {
            Ok(b) => {
                outcome = vec![super::value::ResultItem::Primitive {
                    fhir_type: "boolean".to_string(),
                    value: if b { "true" } else { "false" }.to_string(),
                }];
            }
            Err(e) => match classify(&e) {
                Graded::NotRun(r) => return did_not_run(case, r, None),
                Graded::Licence(r) => {
                    return report(case, CaseStatus::SkippedForLicence, r, None, trace_log);
                }
                Graded::Diagnostic(r) => {
                    return report(
                        case,
                        CaseStatus::Failed,
                        format!("predicate conversion failed: {r}"),
                        None,
                        trace_log,
                    );
                }
            },
        }
    }

    // ---- comparison ------------------------------------------------------
    let verdict = if case.unordered {
        compare::compare_unordered(&case.outputs, &outcome)
    } else {
        compare::compare_ordered(&case.outputs, &outcome)
    };

    match verdict {
        Ok(()) => report(case, CaseStatus::Passed, String::new(), None, trace_log),
        Err(why) => report(case, CaseStatus::Failed, why, None, trace_log),
    }
}

/// A stage returned an error. Decide the status and attribute it to the stage.
fn graded_failure(
    case: &Case,
    engine: &mut dyn SuiteEngine,
    err: &EngineError,
    stage: Stage,
) -> CaseReport {
    match classify(err) {
        Graded::NotRun(reason) => did_not_run(case, reason, Some(stage)),
        Graded::Licence(reason) => report(
            case,
            CaseStatus::SkippedForLicence,
            reason,
            Some(stage),
            drain_log(engine),
        ),
        Graded::Diagnostic(reason) => {
            let log = drain_log(engine);
            if case.expected_failure == Some(stage) {
                report(case, CaseStatus::Passed, String::new(), Some(stage), log)
            } else {
                let expected = match case.expected_failure {
                    None => "no failure".to_string(),
                    Some(s) => format!("a failure at the {} stage", s.as_str()),
                };
                report(
                    case,
                    CaseStatus::Failed,
                    format!(
                        "failed at the {} stage ({reason}), but the case expects {expected}",
                        stage.as_str()
                    ),
                    Some(stage),
                    log,
                )
            }
        }
    }
}

/// Load the pinned suite and run it with the harness's fixed configuration.
pub fn run_pinned_suite() -> (Suite, RunReport) {
    let suite = super::load_pinned_suite();
    let mut engine = super::engine::engine_under_test();
    let report = run(&suite, engine.as_mut(), EngineConfig::default());
    (suite, report)
}
