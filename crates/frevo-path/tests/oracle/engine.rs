//! The seam the oracle harness grades through.
//!
//! The harness talks to a trait object, so it compiles and runs before any
//! engine exists — which is the whole reason task 1 comes before the
//! implementation. Replacing [`engine_under_test`]'s body with the real
//! `frevo_path` engine is the only edit needed to point the suite at it.
//!
//! The unbound binding returns [`EngineError::NotImplemented`] from every
//! method. That is deliberately **not** a parse, check or evaluation failure: a
//! case whose stage returns `NotImplemented` is reported `did-not-run`, so the
//! seven `invalid="syntax"` cases cannot be made to look like passes by an
//! engine that simply refuses to parse anything. It is an error value and not a
//! panicking placeholder, because a panic there would abort the run and report
//! nothing at all.

use super::input::SuiteInput;
use super::value::ResultItem;

/// A handle to an expression the engine parsed and retained, so that a parsed
/// expression can be evaluated repeatedly without re-parsing
/// (`path.api.002`, `path.perf.002`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHandle(pub usize);

/// What went wrong, and whether it is a graded engine answer at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// A real diagnostic from the engine: the message, and the offset within
    /// the expression when the engine reports one (`path.errors.002`).
    Diagnostic {
        message: String,
        offset: Option<usize>,
    },
    /// The case needs a code system whose licence forbids redistribution
    /// (SNOMED CT, LOINC, ICD-10-CM, UMLS). Reported `skipped-for-licence`,
    /// a status deliberately distinct from `passed`.
    RequiresLicensedTerminology { system: String },
    /// No engine is bound yet. Always `did-not-run`, never a graded failure.
    NotImplemented,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Diagnostic { message, offset } => match offset {
                Some(o) => write!(f, "{message} (at offset {o})"),
                None => f.write_str(message),
            },
            EngineError::RequiresLicensedTerminology { system } => {
                write!(f, "requires licensed terminology: {system}")
            }
            EngineError::NotImplemented => f.write_str(
                "the stage this case needs is not bound to the oracle harness yet \
                 -- parse is bound, check and evaluate are not \
                 (crates/frevo-path/tests/oracle/engine.rs::engine_under_test)",
            ),
        }
    }
}

/// The two strictness switches `FHIRPathEngine` exposes, in the sense the
/// harness needs them: `true` means the rule **is** enforced, which is the
/// opposite polarity of the Java setters' `doNotEnforce...` naming.
///
/// `FHIRPathTests.test` calls `check(...)` first and only then
/// `setDoNotEnforceAsCaseSensitive(false)` and
/// `setDoNotEnforceAsSingletonRule(false)` before `evaluate(...)`. So the check
/// pass sees the engine's own pre-R5 initialisation — both rules **not**
/// enforced — and the evaluation sees both **enforced** (`path.strictness.002`).
/// The harness sets them per case rather than relying on a shared engine,
/// because the pinned Java engine is `static` and only its very first case sees
/// the initial values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictnessFlags {
    pub enforce_as_singleton_rule: bool,
    pub enforce_as_case_sensitive: bool,
}

impl StrictnessFlags {
    /// What the check pass runs with.
    pub const CHECK: StrictnessFlags = StrictnessFlags {
        enforce_as_singleton_rule: false,
        enforce_as_case_sensitive: false,
    };
    /// What the evaluation runs with.
    pub const EVALUATE: StrictnessFlags = StrictnessFlags {
        enforce_as_singleton_rule: true,
        enforce_as_case_sensitive: true,
    };
}

/// Engine-wide configuration the harness fixes for every case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineConfig {
    /// The evaluation time zone, as minutes east of UTC.
    ///
    /// `FHIRPathTests.test` sets `TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"))`
    /// before every case, so `today()` and `now()` are read in UTC+11:00 and
    /// not in the machine's zone (`path.datetime.004`).
    pub time_zone_offset_minutes: i32,
    /// `FHIRPathTests` installs `FHIRPathTestEvaluationServices`, whose
    /// `conformsToProfile` answers `true` for the `Patient` profile, `false`
    /// for `Person` and raises for anything else.
    ///
    /// One detail of that service decides the grading of 115 cases. Its
    /// `resolveFunction` **raises** rather than returning null, and the parser
    /// consults it only after `Function.fromCode` has already returned null
    /// (`FHIRPathEngine.java:1158-1166`). So an expression naming a function
    /// the R4 `ExpressionNode.Function` enum does not have — every expression
    /// in the `documentFunctions` group — fails at the **parse** stage, not at
    /// the check or the evaluation stage. That is why the four
    /// `invalid="syntax"` cases in that group are graded `passed` while the
    /// other 111 are graded `failed`.
    pub install_test_host_services: bool,
    /// The SQL-on-FHIR choice warnings, off by default
    /// (`path.strictness.008`).
    pub emit_sql_on_fhir_warning: bool,
}

/// UTC+1100, in minutes.
pub const SUITE_TIME_ZONE_OFFSET_MINUTES: i32 = 11 * 60;

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            time_zone_offset_minutes: SUITE_TIME_ZONE_OFFSET_MINUTES,
            install_test_host_services: true,
            emit_sql_on_fhir_warning: false,
        }
    }
}

/// The context a case is checked and evaluated in.
///
/// Frevo's run honours `context`, `contextType`, `mode` and `version`. The
/// pinned R4 Java harness reads none of them and always checks against the
/// resource root, which is why four `isSemanticChecks` pairs are recorded
/// `hapi-cannot-pass` rather than `failed` (`path.oracle.016`).
#[derive(Debug, Clone, Default)]
pub struct CaseContext<'a> {
    pub root_resource_type: Option<&'a str>,
    pub resource_type: Option<&'a str>,
    pub context: Option<&'a str>,
    pub context_types: &'a [String],
    pub version: Option<&'a str>,
    pub test_mode: Option<&'a str>,
    pub check_ordered_functions: bool,
}

/// What the harness needs of a FHIRPath engine.
pub trait SuiteEngine {
    /// Apply the engine-wide configuration. Called once before the run.
    fn configure(&mut self, config: &EngineConfig) -> Result<(), EngineError>;

    /// Stage 1.
    fn parse(&mut self, expression: &str) -> Result<ParsedHandle, EngineError>;

    /// Stage 2, the static type check.
    fn check(
        &mut self,
        parsed: ParsedHandle,
        context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<(), EngineError>;

    /// Stage 3.
    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError>;

    /// `FHIRPathEngine.convertToBoolean`, which is what `predicate="true"`
    /// applies to the outcome before comparison (`path.oracle.006`).
    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError>;

    /// `FHIRPathEngine.hasLog`.
    fn has_log(&self) -> bool;

    /// `FHIRPathEngine.takeLog`: returns the accumulated trace text **and
    /// clears it**. The harness drains the log after every case and shares one
    /// engine across all 1,191, so without the clear-on-take one case's lines
    /// leak into the next (`path.utility.001`).
    fn take_log(&mut self) -> String;
}

/// The engine the oracle grades.
///
/// Tasks 28, 29 and 30 built the front end, so `configure` and `parse` are
/// real and the remaining three stages are not. `check`, `evaluate` and
/// `convert_to_boolean` answer [`EngineError::NotImplemented`], which is
/// `did-not-run` and never a pass: of the 1,191 cases, the ones whose
/// expression the parser rejects are graded against their `invalid` attribute
/// and every other case is reported as having not run. Returning an empty
/// result or a default from a stage that does not exist would turn a thousand
/// unrun cases into graded passes and failures that mean nothing, which is the
/// one thing this seam exists to prevent.
pub fn engine_under_test() -> Box<dyn SuiteEngine> {
    Box::new(FrontEndEngine {
        config: None,
        parsed: Vec::new(),
        unbound: UnboundEngine,
    })
}

/// The binding for the front end: the lexer, the literal forms and the parser.
struct FrontEndEngine {
    config: Option<EngineConfig>,
    /// Every expression this engine has parsed, kept so a [`ParsedHandle`]
    /// resolves and an expression is parsed once however often it is used
    /// (`path.parse.001`).
    parsed: Vec<frevo_path::Expression>,
    /// The three stages that do not exist yet, answered by the binding below
    /// rather than by a second copy of the same `NotImplemented`, so there is
    /// one place in this file that says "not built yet".
    unbound: UnboundEngine,
}

impl SuiteEngine for FrontEndEngine {
    fn configure(&mut self, config: &EngineConfig) -> Result<(), EngineError> {
        // Nothing the front end does depends on the configuration: the
        // evaluation time zone, the host services and the SQL-on-FHIR warning
        // are all read by stages that do not exist yet. It is recorded rather
        // than ignored so that the stage which starts reading it finds it
        // here.
        self.config = Some(*config);
        Ok(())
    }

    fn parse(&mut self, expression: &str) -> Result<ParsedHandle, EngineError> {
        // No host services are passed, and that is not a shortcut: the suite
        // installs `FHIRPathTestEvaluationServices`, whose `resolveFunction`
        // **raises** for every name (`FHIRPathTests.java:72-76`), so no name
        // the R4 `Function` table lacks ever resolves as a custom function.
        // Upstream's raise and this parser's "not a valid function name" are
        // the same outcome at the same stage.
        match frevo_path::Expression::parse(expression) {
            Ok(parsed) => {
                self.parsed.push(parsed);
                Ok(ParsedHandle(self.parsed.len() - 1))
            }
            Err(e) => Err(EngineError::Diagnostic {
                message: e.message().to_string(),
                offset: Some(e.offset()),
            }),
        }
    }

    fn check(
        &mut self,
        parsed: ParsedHandle,
        context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        self.unbound.check(parsed, context, flags)
    }

    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        self.unbound.evaluate(parsed, input, context, flags)
    }

    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError> {
        self.unbound.convert_to_boolean(items)
    }

    fn has_log(&self) -> bool {
        self.unbound.has_log()
    }

    fn take_log(&mut self) -> String {
        self.unbound.take_log()
    }
}

/// The binding used while `frevo-path` has no engine.
struct UnboundEngine;

impl SuiteEngine for UnboundEngine {
    fn configure(&mut self, _config: &EngineConfig) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn parse(&mut self, _expression: &str) -> Result<ParsedHandle, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn check(
        &mut self,
        _parsed: ParsedHandle,
        _context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate(
        &mut self,
        _parsed: ParsedHandle,
        _input: Option<&SuiteInput>,
        _context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn convert_to_boolean(&mut self, _items: &[ResultItem]) -> Result<bool, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn has_log(&self) -> bool {
        false
    }

    fn take_log(&mut self) -> String {
        String::new()
    }
}
