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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use frevo_path::{
    CheckError, CheckFocus, CheckOptions, CheckOutcome, CheckRequest, Checker, ConstantMode,
    ElementInfo, EvalError, Evaluator, FunctionDetails, HostError, HostServices, Model, Node,
    NodeError, Primitive, TypeSet, Value,
};

use super::input::{self, Document, SuiteInput};
use super::value::{QuantityValue, ResultItem};
use super::{json, xml};

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
                "the stage or feature this case needs is not implemented by frevo-path yet \
                 -- every stage the crate exposes is bound, so this is the crate answering \
                 EvalError::Unsupported or CheckError::NotImplemented, a setting or surface \
                 the crate has no API for, or the pinned R4 type package being absent \
                 (crates/frevo-path/tests/oracle/engine.rs)",
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
    /// before every case, so `today()` and `now()` are not read in the
    /// machine's zone. `UTC+1100` is not a zone id Java recognises, so the
    /// zone set is `GMT`, offset 0, and that is what the harness fixes
    /// (`path.datetime.004`).
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

/// UTC, in minutes: offset 0. `FHIRPathTests.test` calls
/// `TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"))`, but `UTC+1100` is
/// not a zone id `java.util.TimeZone` recognises, so it returns `GMT`, raw
/// offset 0, which is the zone HAPI's recorded run used (`path.datetime.004`,
/// DEV-0018).
pub const SUITE_TIME_ZONE_OFFSET_MINUTES: i32 = 0;

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
/// resource root, which is why the 13 cases of six `isSemanticChecks` groups
/// -- byte-identical expressions whose expected outcomes differ only through
/// `context`/`contextType` -- are recorded `hapi-cannot-pass` rather than
/// `failed` (`path.oracle.016`).
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
/// real. Task 31 built the evaluator core, so `evaluate` is bound -- for what
/// the evaluator implements (navigation, and since tasks 32-34 the collection
/// predicates, filtering and projection, subsetting, combining and membership,
/// and since tasks 35-37 equality, equivalence, ordered comparison, boolean
/// logic, `iif`, `hasValue` and arithmetic) and nothing else: every function, operator and
/// environment variable it does not implement yet comes back as
/// [`frevo_path::EvalError::Unsupported`], which this seam turns into
/// [`EngineError::NotImplemented`]. Task 50 built the static check pass, so
/// `check` is bound too, over the same type model, with anything the pass
/// cannot type yet -- a host-supplied function, an extension definition's
/// element model -- answered as [`frevo_path::CheckError::NotImplemented`]
/// and so as `NotImplemented` here. `convert_to_boolean` is
/// `FHIRPathEngine.convertToBoolean`, which needs no evaluator.
///
/// All of that is `did-not-run` and never a pass. Returning an empty result or
/// a default from a stage or a feature that does not exist would turn a
/// thousand unrun cases into graded passes and failures that mean nothing,
/// which is the one thing this seam exists to prevent.
pub fn engine_under_test() -> Box<dyn SuiteEngine> {
    Box::new(FrontEndEngine {
        config: None,
        parsed: Vec::new(),
        trees: HashMap::new(),
        evaluator: host_base_evaluator(),
    })
}

/// The binding for the front end and the evaluator core.
struct FrontEndEngine {
    config: Option<EngineConfig>,
    /// Every expression this engine has parsed, kept so a [`ParsedHandle`]
    /// resolves and an expression is parsed once however often it is used
    /// (`path.parse.001`).
    parsed: Vec<frevo_path::Expression>,
    /// Each input file as a typed node tree, built once on first use and
    /// shared by every case that names the file, as `FHIRPathTests` caches
    /// its parsed resources.
    trees: HashMap<String, Value>,
    /// The one evaluator every case of the run goes through. It owns the
    /// `trace()` log that `has_log`/`take_log` read, so the log is the
    /// engine's and is cleared by taking it. `None` when the type package is
    /// absent.
    evaluator: Option<Evaluator<'static>>,
}

impl FrontEndEngine {
    /// The typed tree for `input`, built on first use.
    fn tree_for(&mut self, model: &'static SdModel, input: &SuiteInput) -> Option<Value> {
        if let Some(tree) = self.trees.get(&input.file) {
            return Some(tree.clone());
        }
        let builder = TreeBuilder { model };
        let tree = match &input.document {
            Document::Xml(d) => Some(builder.xml_resource(&d.root)),
            Document::Json(v) => builder.json_resource(v),
        }?;
        self.trees.insert(input.file.clone(), tree.clone());
        Some(tree)
    }
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
        _flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        // The strictness flags are read by evaluation's `as` and by nothing
        // in the check pass; `check_expression` is the binding itself.
        let expr = self
            .parsed
            .get(parsed.0)
            .ok_or_else(|| EngineError::Diagnostic {
                message: format!("no expression was parsed under handle {}", parsed.0),
                offset: None,
            })?;
        let config = self.config.unwrap_or_default();
        check_types(expr, context, &config).map(|_| ())
    }

    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        _context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        // The strictness flags are the two `as` rules, which the evaluator
        // takes through `with_as_rules`; the evaluator core does not read the
        // case context's types (see the `context` handling below).
        // `FHIRPathTests.test` evaluates with
        // `fp.evaluate(res, node)`: the input resource is the focus and
        // `$this`, and there is no application context.
        //
        // The type information comes from the pinned R4 definition package.
        // Without it no input can be typed, and that is a missing
        // prerequisite, reported `did-not-run` like a missing input tree
        // (`path.oracle.003`) rather than as a graded answer.
        let model = sd_model().ok_or(EngineError::NotImplemented)?;
        let root = match input {
            None => None,
            Some(i) => Some(
                self.tree_for(model, i)
                    .ok_or_else(|| EngineError::Diagnostic {
                        message: format!("{} has no resourceType to build a resource from", i.file),
                        offset: None,
                    })?,
            ),
        };
        let expr = self
            .parsed
            .get(parsed.0)
            .ok_or_else(|| EngineError::Diagnostic {
                message: format!("no expression was parsed under handle {}", parsed.0),
                offset: None,
            })?;
        // The evaluation time zone is the configured one (UTC+11 for the
        // suite), which `today()`, `now()` and offset-less dateTime
        // comparison read (`path.datetime.004`).
        let zone = self.config.map_or(SUITE_TIME_ZONE_OFFSET_MINUTES, |c| {
            c.time_zone_offset_minutes
        });
        // Frevo's run honours the case's `context` attribute, which the pinned
        // Java harness reads nowhere (`path.oracle.002`, `path.oracle.016`):
        // the expression is evaluated once per item the context path selects
        // from the resource, with that item as the focus and `$this`, and the
        // results are concatenated. Without a `context` the resource is the
        // focus, as upstream's `fp.evaluate(res, node)`.
        //
        // One evaluator serves every case of the run, so its `trace()` log is
        // the engine's log: `has_log`/`take_log` read it, and the harness
        // drains it after every case (`path.utility.001`).
        let base = self
            .evaluator
            .clone()
            .ok_or(EngineError::NotImplemented)?
            .with_time_zone_offset_minutes(zone)
            .with_as_rules(
                flags.enforce_as_singleton_rule,
                flags.enforce_as_case_sensitive,
            );
        // `FHIRPathTests.test` installs `FHIRPathTestEvaluationServices`
        // before every case (`FHIRPathTests.java:176`), so evaluation sees the
        // suite's host service whenever the configuration installs it.
        let install = self.config.unwrap_or_default().install_test_host_services;
        let host: Option<&dyn HostServices> = if install {
            Some(&SuiteHostServices)
        } else {
            None
        };
        let values =
            evaluate_host_aware(&base, expr, root.as_ref(), _context.context, host, false)?;
        Ok(values.iter().map(|v| result_item(model, v)).collect())
    }

    /// `FHIRPathEngine.convertToBoolean` (`FHIRPathEngine.java:914`): one
    /// boolean item is its value, anything else is whether the list is
    /// non-empty. A boolean element goes through `Boolean.valueOf(text)`,
    /// which is true only for `true` in any case.
    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError> {
        Ok(match items {
            [ResultItem::Primitive { fhir_type, value }] if fhir_type == "boolean" => {
                value.eq_ignore_ascii_case("true")
            }
            _ => !items.is_empty(),
        })
    }

    fn has_log(&self) -> bool {
        self.evaluator.as_ref().is_some_and(Evaluator::has_log)
    }

    /// The accumulated `trace()` text, **cleared by taking it**: the evaluator's
    /// own `takeLog`, shared by every evaluation this engine makes.
    fn take_log(&mut self) -> String {
        self.evaluator
            .as_ref()
            .map_or_else(String::new, Evaluator::take_log)
    }
}

/// The check stage as the harness binds it, answering the check pass's own
/// result: the inferred types and warnings, or the [`CheckError`] with its
/// message id.
///
/// It is a free function so that a test about the inferred **type**
/// (`path.check.003`, `004`), about a diagnostic's message id, or about a
/// context the suite does not carry (`path.check.005` to `012`) drives exactly
/// the binding the suite run grades, over the same model and host services,
/// rather than a second copy of it. The outer error is the one prerequisite
/// the pass itself cannot report -- no type model -- and is
/// [`EngineError::NotImplemented`]. A caller must treat
/// [`CheckError::NotImplemented`] the same way: did-not-run, never a pass.
pub fn check_expression(
    expr: &frevo_path::Expression,
    context: &CaseContext<'_>,
    config: &EngineConfig,
) -> Result<Result<CheckOutcome, CheckError>, EngineError> {
    {
        // The type information is the same pinned R4 package evaluation is
        // typed from; without it nothing can be checked, which is a missing
        // prerequisite and not a verdict.
        let model = sd_model().ok_or(EngineError::NotImplemented)?;

        // `FHIRPathTests.test` checks with `fp.check(null, null, null, node)`
        // when the case has no input, and otherwise with
        // `fp.check(res, "Resource", type, type, node)`: the root resource
        // type is the literal `Resource`, and the resource type is both the
        // `%resource` type and the context. Frevo's run also honours the
        // case's `context` and `contextType` (`path.oracle.016`):
        // a `contextType` list is `checkOnTypes`, a `context` path is
        // `check(..., context, ...)`.
        let context_types: Vec<&str> = context.context_types.iter().map(String::as_str).collect();
        let focus = if !context_types.is_empty() {
            CheckFocus::Types(&context_types)
        } else if let Some(c) = context.context {
            CheckFocus::Context(c)
        } else if let Some(rt) = context.resource_type {
            CheckFocus::Context(rt)
        } else {
            CheckFocus::None
        };
        let request = CheckRequest {
            root_resource_type: context.resource_type.map(|_| "Resource"),
            resource_type: context.resource_type,
            focus,
        };
        let mut checker = Checker::new(model).with_options(CheckOptions {
            allow_polymorphic_names: false,
            emit_sql_on_fhir_warning: config.emit_sql_on_fhir_warning,
        });
        if config.install_test_host_services {
            checker = checker.with_host_services(&SuiteHostServices);
        }
        Ok(checker.check_with_warnings(expr, &request))
    }
}

/// [`check_expression`] with the pass's answer folded into the harness's
/// error: a failure is a [`EngineError::Diagnostic`] and a not-implemented
/// answer is [`EngineError::NotImplemented`]. [`SuiteEngine::check`] is this
/// with the type set dropped.
pub fn check_types(
    expr: &frevo_path::Expression,
    context: &CaseContext<'_>,
    config: &EngineConfig,
) -> Result<TypeSet, EngineError> {
    match check_expression(expr, context, config)? {
        Ok(outcome) => Ok(outcome.types),
        Err(CheckError::NotImplemented { .. }) => Err(EngineError::NotImplemented),
        Err(CheckError::Failed {
            message, offset, ..
        }) => Err(EngineError::Diagnostic { message, offset }),
    }
}

/// The evaluation stage as the harness binds it, over a root value the caller
/// supplies.
///
/// [`SuiteEngine::evaluate`] is this over the input file's typed tree. A test
/// whose input no suite file carries (`path.navigation.013`, an in-memory
/// `Patient`) drives the same binding over a node it built. A feature the
/// evaluator does not implement yet is [`EngineError::NotImplemented`].
#[allow(dead_code)] // used by test binaries that drive the seam directly
pub fn evaluate_over(
    expr: &frevo_path::Expression,
    root: Option<&Value>,
) -> Result<Vec<Value>, EngineError> {
    evaluate_over_in_zone(expr, root, SUITE_TIME_ZONE_OFFSET_MINUTES)
}

/// [`evaluate_over`] in an explicit evaluation time zone, as minutes east of
/// UTC. The suite runs at UTC, offset 0, which is
/// `SUITE_TIME_ZONE_OFFSET_MINUTES` (`UTC+1100` is `GMT` in Java); `today()`,
/// `now()` and the placement of an offset-less `dateTime` read it
/// (`path.datetime.004`, DEV-0018).
pub fn evaluate_over_in_zone(
    expr: &frevo_path::Expression,
    root: Option<&Value>,
    time_zone_offset_minutes: i32,
) -> Result<Vec<Value>, EngineError> {
    let model = sd_model().ok_or(EngineError::NotImplemented)?;
    Evaluator::new(model)
        .with_time_zone_offset_minutes(time_zone_offset_minutes)
        .evaluate(expr, root)
        .map_err(engine_error)
}

// ---------------------------------------------------------------------------
// The host-aware seam
//
// `path_env.rs` and `path_utility.rs` drive evaluation with a host service the
// caller supplies, an application context flag, an evaluation that starts
// partway down a resource, and the engine's own `trace()` log. These bind that
// surface to the real evaluator over the same model the suite run uses.
// ---------------------------------------------------------------------------

/// An evaluator over the pinned model in the suite's time zone, with no host
/// service. `None` when the type package is absent: a missing prerequisite,
/// which the caller reports as `did-not-run`. Clones share the `trace()` log,
/// so one base evaluator held by an engine gives clear-on-take semantics across
/// every call that engine makes.
#[allow(dead_code)]
pub fn host_base_evaluator() -> Option<Evaluator<'static>> {
    let model = sd_model()?;
    Some(Evaluator::new(model).with_time_zone_offset_minutes(SUITE_TIME_ZONE_OFFSET_MINUTES))
}

/// Evaluate `expr` over `root` on a caller-configured `evaluator` and grade
/// the values as the suite run does. A test that varies an engine-wide setting
/// the harness's [`EngineConfig`] does not carry -- implicit string conversion
/// (`path.string.024`), say -- builds the evaluator from
/// [`host_base_evaluator`] and grades through the same result mapping.
#[allow(dead_code)]
pub fn evaluate_items(
    evaluator: &Evaluator<'_>,
    expr: &frevo_path::Expression,
    root: Option<&Value>,
) -> Result<Vec<ResultItem>, EngineError> {
    let model = sd_model().ok_or(EngineError::NotImplemented)?;
    let values = evaluator.evaluate(expr, root).map_err(engine_error)?;
    Ok(values.iter().map(|v| result_item(model, v)).collect())
}

/// A node that answers as `inner` does except for one child name, where it
/// reports `fault` -- the shape a decode error from `frevo-model` takes at the
/// navigation seam (`path.errors.005`).
#[derive(Debug)]
struct FaultingNode {
    inner: Arc<dyn Node>,
    child: String,
    fault: NodeError,
}

impl Node for FaultingNode {
    fn type_name(&self) -> &str {
        self.inner.type_name()
    }

    fn is_resource(&self) -> bool {
        self.inner.is_resource()
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        if name == self.child {
            Err(self.fault.clone())
        } else {
            self.inner.children(name)
        }
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Err(self.fault.clone())
    }

    fn primitive(&self) -> Option<Primitive> {
        self.inner.primitive()
    }
}

/// `root`, with its `child` element replaced by a decode error. `None` when
/// `root` is not a node.
#[allow(dead_code)]
pub fn faulting_root(root: &Value, child: &str, fault: NodeError) -> Option<Value> {
    let inner = root.as_node()?.clone();
    Some(Value::Node(Arc::new(FaultingNode {
        inner,
        child: child.to_string(),
        fault,
    })))
}

/// The typed node tree for a suite input file.
#[allow(dead_code)]
pub fn typed_input(input: &SuiteInput) -> Option<Value> {
    let model = sd_model()?;
    let builder = TreeBuilder { model };
    match &input.document {
        Document::Xml(d) => Some(builder.xml_resource(&d.root)),
        Document::Json(v) => builder.json_resource(v),
    }
}

/// An evaluation failure as the harness grades it.
#[allow(dead_code)]
pub fn eval_error(e: EvalError) -> EngineError {
    engine_error(e)
}

/// Parse with a host service consulted for function names the R4 table lacks.
#[allow(dead_code)]
pub fn parse_with_host(
    text: &str,
    host: Option<&dyn HostServices>,
) -> Result<frevo_path::Expression, EngineError> {
    frevo_path::Expression::parse_with(text, frevo_path::ParseOptions::default(), host).map_err(
        |e| EngineError::Diagnostic {
            message: e.message().to_string(),
            offset: Some(e.offset()),
        },
    )
}

/// The check pass against a named context type (or none), with an optional
/// host service.
#[allow(dead_code)]
pub fn check_in_type(
    expr: &frevo_path::Expression,
    context_type: Option<&str>,
    host: Option<&dyn HostServices>,
) -> Result<TypeSet, EngineError> {
    match check_outcome_in_type(expr, context_type, host, CheckOptions::default())? {
        Ok(outcome) => Ok(outcome.types),
        Err(CheckError::NotImplemented { .. }) => Err(EngineError::NotImplemented),
        Err(CheckError::Failed {
            message, offset, ..
        }) => Err(EngineError::Diagnostic { message, offset }),
    }
}

/// The check pass against a named context type (or none), with an optional
/// host service and explicit [`CheckOptions`], answering the pass's own
/// outcome: the inferred types **and** the warnings, or its [`CheckError`].
/// The outer error is the one prerequisite the pass cannot report itself --
/// no type model -- and is [`EngineError::NotImplemented`].
///
/// `path_check_warnings.rs` grades the warning list through this, and
/// `path_strictness.rs` the polymorphic-names and SQL-on-FHIR switches.
#[allow(dead_code)]
pub fn check_outcome_in_type(
    expr: &frevo_path::Expression,
    context_type: Option<&str>,
    host: Option<&dyn HostServices>,
    options: CheckOptions,
) -> Result<Result<CheckOutcome, CheckError>, EngineError> {
    let checker = checker_for(host, options)?;
    Ok(checker.check_with_warnings(expr, &request_in_type(context_type)))
}

/// [`check_outcome_in_type`] through the pass's **simple** surface,
/// [`Checker::check`], which answers the inferred types and drops the
/// warnings itself (`path.check.019`).
#[allow(dead_code)]
pub fn simple_check_in_type(
    expr: &frevo_path::Expression,
    context_type: Option<&str>,
    host: Option<&dyn HostServices>,
    options: CheckOptions,
) -> Result<Result<TypeSet, CheckError>, EngineError> {
    let checker = checker_for(host, options)?;
    Ok(checker.check(expr, &request_in_type(context_type)))
}

/// A checker over the pinned model; `NotImplemented` when the type package is
/// absent.
fn checker_for<'a>(
    host: Option<&'a dyn HostServices>,
    options: CheckOptions,
) -> Result<Checker<'a>, EngineError> {
    let model = sd_model().ok_or(EngineError::NotImplemented)?;
    let mut checker = Checker::new(model).with_options(options);
    if let Some(h) = host {
        checker = checker.with_host_services(h);
    }
    Ok(checker)
}

/// `fp.check(res, "Resource", type, type, node)` for a named context type, or
/// the null-context check without one.
fn request_in_type(context_type: Option<&str>) -> CheckRequest<'_> {
    CheckRequest {
        root_resource_type: context_type.map(|_| "Resource"),
        resource_type: context_type,
        focus: match context_type {
            Some(t) => CheckFocus::Context(t),
            None => CheckFocus::None,
        },
    }
}

/// Evaluate with a host service, an application-context flag and an optional
/// context path, on a clone of `base` (so the `trace()` log is the base's).
///
/// With a `context_path` the expression is evaluated once per node the path
/// selects from `root`, with that node as focus and `$this`, and `root` as
/// `%resource` and `%rootResource`.
#[allow(dead_code)]
pub fn evaluate_host_aware(
    base: &Evaluator<'static>,
    expr: &frevo_path::Expression,
    root: Option<&Value>,
    context_path: Option<&str>,
    host: Option<&dyn HostServices>,
    app_context: bool,
) -> Result<Vec<Value>, EngineError> {
    let mut ev: Evaluator<'_> = base.clone().with_application_context(app_context);
    if let Some(h) = host {
        ev = ev.with_host_services(h);
    }
    match (context_path, root) {
        (Some(path), Some(resource)) => {
            let context_expr = parse_with_host(path, None)?;
            let items = base
                .clone()
                .evaluate(&context_expr, Some(resource))
                .map_err(engine_error)?;
            let mut out = Vec::new();
            for item in &items {
                // `%resource` is the resource the focus IS (a contained
                // resource selected by the context path) or is inside;
                // `%rootResource` is always the outermost resource.
                let focus_resource = match item {
                    Value::Node(n) if n.is_resource() => item,
                    _ => resource,
                };
                out.extend(
                    ev.evaluate_in(expr, Some(item), Some(focus_resource), Some(resource))
                        .map_err(engine_error)?,
                );
            }
            Ok(out)
        }
        _ => ev.evaluate(expr, root).map_err(engine_error),
    }
}

/// `FHIRPathTests.FHIRPathTestEvaluationServices`
/// (`FHIRPathTests.java:51-117`), for the methods the **check pass** calls:
/// `resolve_constant_type` raises, as upstream's does, so an unknown `%name`
/// fails the check; `check_function` raises; `param_is_type` is `false`.
///
/// The evaluation-side methods answer as upstream's where upstream's answer
/// needs nothing the harness lacks — `conforms_to_profile`, `log` — and
/// otherwise raise as upstream raises. `resolve_value_set` answers as
/// upstream's does, from the worker context
/// (`TestingUtilities.context().fetchResource(ValueSet.class, url)`), which
/// here is the pinned package through [`Model::value_set`]. The seam's
/// `evaluate` installs it whenever `install_test_host_services` is set, as
/// `FHIRPathTests.test` does before every case (`FHIRPathTests.java:176`).
struct SuiteHostServices;

impl SuiteHostServices {
    fn not_done(what: &str) -> HostError {
        HostError::new(format!(
            "Not done yet (FHIRPathTestEvaluationServices.{what}), when item is element"
        ))
    }
}

impl HostServices for SuiteHostServices {
    fn resolve_constant(&self, _name: &str, _mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        Err(Self::not_done("resolveConstant"))
    }

    fn resolve_constant_type(
        &self,
        _name: &str,
        _mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        Err(Self::not_done("resolveConstantType"))
    }

    fn resolve_reference(
        &self,
        _url: &str,
        _identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        Err(Self::not_done("resolveReference"))
    }

    fn conforms_to_profile(&self, _item: &Value, url: &str) -> Result<bool, HostError> {
        match url {
            "http://hl7.org/fhir/StructureDefinition/Patient" => Ok(true),
            "http://hl7.org/fhir/StructureDefinition/Person" => Ok(false),
            _ => Err(HostError::new(format!("unknown profile {url}"))),
        }
    }

    fn resolve_value_set(&self, url: &str) -> Result<Option<Value>, HostError> {
        // `fetchResource` answers null for an unknown URL; only a missing
        // package -- no worker context at all -- is an error.
        sd_model().and_then(|m| m.value_set(url)).ok_or_else(|| {
            HostError::new(format!(
                "the harness has no worker context to resolve {url} from"
            ))
        })
    }

    fn resolve_function(&self, _name: &str) -> Option<FunctionDetails> {
        None
    }

    fn check_function(
        &self,
        _name: &str,
        _focus: &TypeSet,
        _params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        Err(Self::not_done("checkFunction"))
    }

    fn execute_function(
        &self,
        _name: &str,
        _focus: &[Value],
        _params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        Err(Self::not_done("executeFunction"))
    }

    fn param_is_type(&self, _name: &str, _index: usize) -> bool {
        false
    }

    fn log(&self, _name: &str, _focus: &[Value]) -> bool {
        false
    }
}

/// The binding used while `frevo-path` has no engine. Nothing constructs it
/// now that every stage is bound; it is kept as the reference for what an
/// unbound stage answers (`EngineError::NotImplemented`).
#[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// The evaluation adapter: suite inputs as typed nodes
//
// `frevo-path` evaluates over the `Node` and `Model` traits and may not name
// the generated model crate (`path.api.008`), so the harness supplies both
// itself. The type information is read from the same pinned R4 definition
// package `frevo-codegen` generates from -- the base StructureDefinitions only,
// profiles and logical models excluded -- and each input file, XML or JSON, is
// turned into a tree of typed nodes once.
//
// Children are listed in definition order, which is the order
// `Base.listChildren` gives `children()` upstream, and within one element in
// document order. An element the definitions do not name is not a child.
// ---------------------------------------------------------------------------

/// The pinned R4 definition package, extracted
/// (`crates/frevo-codegen/packages/PINS.md`).
const R4_PACKAGE_DIR: &str = "crates/frevo-codegen/packages/fetched/hl7.fhir.r4.core-4.0.1/package";

const FHIR_TYPE_EXTENSION: &str =
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-fhir-type";
const SYSTEM_TYPE_PREFIX: &str = "http://hl7.org/fhirpath/System.";

/// One element of a StructureDefinition snapshot.
#[derive(Debug)]
struct ElementDef {
    path: String,
    parent: String,
    /// The element name, `[x]` removed.
    name: String,
    choice: bool,
    /// Each type's working code: the `structuredefinition-fhir-type`
    /// extension's value for a System type code, else the code.
    types: Vec<String>,
    /// Each type's code as declared, a System type URL shortened to
    /// `System.String`.
    codes: Vec<String>,
    /// `ElementDefinition.unbounded()`: `max` is `*` or above 1.
    repeating: bool,
    /// The referenced element path, `#` removed.
    content_reference: Option<String>,
}

#[derive(Debug)]
struct TypeDef {
    kind: String,
    base: Option<String>,
    /// Whether `derivation` is `specialization`. `Resource` and `Element`
    /// have no derivation at all.
    specialization: bool,
    is_abstract: bool,
    elements: Vec<ElementDef>,
}

/// The R4 type model, from the base StructureDefinitions.
#[derive(Debug)]
struct SdModel {
    types: HashMap<String, TypeDef>,
    /// The canonical URL of every StructureDefinition in the package,
    /// profiles and extensions included, for `extension(url)`.
    urls: std::collections::HashSet<String>,
    /// The package's `ValueSet-*.json` and `CodeSystem-*.json` files, by the
    /// id in the file name.
    terminology_files: TerminologyIndex,
    /// The same files by declared canonical URL, read on first need: 2,400-odd
    /// files that no test without terminology reads.
    terminology_by_url: OnceLock<TerminologyIndex>,
}

/// A file per ValueSet and per CodeSystem, keyed by id or by canonical URL.
#[derive(Debug, Default)]
struct TerminologyIndex {
    value_sets: HashMap<String, PathBuf>,
    code_systems: HashMap<String, PathBuf>,
}

/// The canonical URL a terminology file declares and its parsed content, when
/// the file holds a `resource_type` resource.
fn declared_url(path: &PathBuf, resource_type: &str) -> Option<(String, json::Value)> {
    let text = std::fs::read_to_string(path).ok()?;
    let v = json::parse(&text).ok()?;
    if v.get("resourceType").and_then(json::Value::as_str) != Some(resource_type) {
        return None;
    }
    let url = v.get("url").and_then(json::Value::as_str)?.to_string();
    Some((url, v))
}

/// The model, loaded once per test binary; `None` when the package is not
/// present or cannot be read.
fn sd_model() -> Option<&'static SdModel> {
    static MODEL: OnceLock<Option<SdModel>> = OnceLock::new();
    MODEL.get_or_init(load_sd_model).as_ref()
}

fn load_sd_model() -> Option<SdModel> {
    let dir = input::repo_path(R4_PACKAGE_DIR);
    let mut types = HashMap::new();
    let mut urls = std::collections::HashSet::new();
    let mut terminology_files = TerminologyIndex::default();
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(id) = file
            .strip_prefix("ValueSet-")
            .and_then(|f| f.strip_suffix(".json"))
        {
            terminology_files
                .value_sets
                .insert(id.to_string(), entry.path());
            continue;
        }
        if let Some(id) = file
            .strip_prefix("CodeSystem-")
            .and_then(|f| f.strip_suffix(".json"))
        {
            terminology_files
                .code_systems
                .insert(id.to_string(), entry.path());
            continue;
        }
        let Some(id) = file
            .strip_prefix("StructureDefinition-")
            .and_then(|f| f.strip_suffix(".json"))
        else {
            continue;
        };
        // Every StructureDefinition in the core package has the canonical
        // URL `http://hl7.org/fhir/StructureDefinition/<id>`, and the package
        // names each file by its id, so the URL is known without parsing the
        // 400-odd profiles the pre-filter below skips.
        urls.insert(format!("http://hl7.org/fhir/StructureDefinition/{id}"));
        let text = std::fs::read_to_string(entry.path()).ok()?;
        // A cheap pre-filter for the 400-odd profiles and extensions; the
        // parsed check below is the one that decides.
        if text.contains("\"derivation\":\"constraint\"") || text.contains("\"kind\":\"logical\"") {
            continue;
        }
        let sd = json::parse(&text).ok()?;
        let kind = sd.get("kind").and_then(json::Value::as_str).unwrap_or("");
        let derivation = sd.get("derivation").and_then(json::Value::as_str);
        if kind == "logical" || derivation == Some("constraint") {
            continue;
        }
        let Some(type_name) = sd.get("type").and_then(json::Value::as_str) else {
            continue;
        };
        let base = sd
            .get("baseDefinition")
            .and_then(json::Value::as_str)
            .map(|b| b.rsplit('/').next().unwrap_or(b).to_string());
        let mut elements = Vec::new();
        if let Some(json::Value::Array(snapshot)) =
            sd.get("snapshot").and_then(|s| s.get("element"))
        {
            elements.extend(snapshot.iter().filter_map(element_def));
        }
        types.insert(
            type_name.to_string(),
            TypeDef {
                kind: kind.to_string(),
                base,
                specialization: derivation == Some("specialization"),
                is_abstract: matches!(sd.get("abstract"), Some(json::Value::Bool(true))),
                elements,
            },
        );
    }
    if types.is_empty() {
        None
    } else {
        Some(SdModel {
            types,
            urls,
            terminology_files,
            terminology_by_url: OnceLock::new(),
        })
    }
}

fn element_def(e: &json::Value) -> Option<ElementDef> {
    let path = e.get("path")?.as_str()?;
    let dot = path.rfind('.')?;
    let last = &path[dot + 1..];
    let (name, choice) = match last.strip_suffix("[x]") {
        Some(base) => (base, true),
        None => (last, false),
    };
    let mut types = Vec::new();
    let mut codes = Vec::new();
    if let Some(json::Value::Array(entries)) = e.get("type") {
        for t in entries {
            if let Some(code) = t.get("code").and_then(json::Value::as_str) {
                types.push(fhir_type_code(t, code));
                codes.push(match code.strip_prefix("http://hl7.org/fhirpath/") {
                    Some(system) => system.to_string(),
                    None => code.to_string(),
                });
            }
        }
    }
    let repeating = match e.get("max").and_then(json::Value::as_str) {
        Some("*") => true,
        Some(n) => n.parse::<u64>().is_ok_and(|n| n > 1),
        None => false,
    };
    Some(ElementDef {
        path: path.to_string(),
        parent: path[..dot].to_string(),
        name: name.to_string(),
        choice,
        types,
        codes,
        repeating,
        content_reference: e
            .get("contentReference")
            .and_then(json::Value::as_str)
            .map(|r| r.trim_start_matches('#').to_string()),
    })
}

/// A `System.*` type code names its FHIR type in the
/// `structuredefinition-fhir-type` extension beside it: `Extension.url` is a
/// `uri`, `Element.id` a `string`.
fn fhir_type_code(t: &json::Value, code: &str) -> String {
    let Some(system) = code.strip_prefix(SYSTEM_TYPE_PREFIX) else {
        return code.to_string();
    };
    if let Some(json::Value::Array(exts)) = t.get("extension") {
        for x in exts {
            if x.get("url").and_then(json::Value::as_str) != Some(FHIR_TYPE_EXTENSION) {
                continue;
            }
            if let Some(v) = x
                .get("valueUrl")
                .or_else(|| x.get("valueUri"))
                .and_then(json::Value::as_str)
            {
                return v.to_string();
            }
        }
    }
    let mut chars = system.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

impl SdModel {
    /// `worker.fetchResource(<type>.class, url)` over the package: the
    /// resource whose canonical URL is `url`, as a typed node, or `Some(None)`
    /// when the package has none.
    ///
    /// The core package names each file by the resource id, and a core
    /// canonical URL usually ends in that id, so the file the last URL segment
    /// names is tried first; its declared URL still decides. Only when that
    /// guess misses is every file of the type read, once, into an index.
    fn terminology_resource(&self, url: &str, value_set: bool) -> Option<Option<Value>> {
        let (resource_type, by_id) = if value_set {
            ("ValueSet", &self.terminology_files.value_sets)
        } else {
            ("CodeSystem", &self.terminology_files.code_systems)
        };
        let build = |v: &json::Value| TreeBuilder { model: self }.json_resource(v);
        let tail = url.rsplit('/').next().unwrap_or(url);
        if let Some((declared, v)) = by_id
            .get(tail)
            .and_then(|path| declared_url(path, resource_type))
        {
            if declared == url {
                return Some(build(&v));
            }
        }
        let index = self.terminology_by_url.get_or_init(|| {
            let read = |files: &HashMap<String, PathBuf>, rt: &str| {
                files
                    .values()
                    .filter_map(|p| declared_url(p, rt).map(|(u, _)| (u, p.clone())))
                    .collect()
            };
            TerminologyIndex {
                value_sets: read(&self.terminology_files.value_sets, "ValueSet"),
                code_systems: read(&self.terminology_files.code_systems, "CodeSystem"),
            }
        });
        let by_url = if value_set {
            &index.value_sets
        } else {
            &index.code_systems
        };
        match by_url.get(url) {
            None => Some(None),
            Some(path) => {
                let (_, v) = declared_url(path, resource_type)?;
                Some(build(&v))
            }
        }
    }

    /// The direct child definitions of `path` in StructureDefinition `sd`.
    fn children_of<'a>(
        &'a self,
        sd: &str,
        path: &'a str,
    ) -> impl Iterator<Item = &'a ElementDef> + 'a {
        self.types
            .get(sd)
            .map(|t| t.elements.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter(move |e| e.parent == path)
    }

    fn has_children(&self, sd: &str, path: &str) -> bool {
        self.children_of(sd, path).next().is_some()
    }

    /// The types navigating to `e` by name yields, named as `frevo-model`'s
    /// tables name them: a content reference as the referenced element's
    /// path, an `Element`, `BackboneElement` or other abstract non-resource
    /// type as the element's own path (upstream's `sd#path`), `Resource` as
    /// itself, and everything else by its working code
    /// (`getChildTypesByName`, `FHIRPathEngine.java:6483`-`:6507`).
    fn navigation_types(&self, e: &ElementDef) -> Vec<String> {
        if let Some(r) = &e.content_reference {
            return vec![r.clone()];
        }
        e.types
            .iter()
            .zip(&e.codes)
            .map(|(working, code)| {
                let abstract_data_type = self
                    .types
                    .get(code)
                    .is_some_and(|t| t.is_abstract && t.kind != "resource");
                if code == "Element" || code == "BackboneElement" || abstract_data_type {
                    e.path.clone()
                } else {
                    working.clone()
                }
            })
            .collect()
    }

    /// A backbone type is named by its path (`Patient.contact`), which is how
    /// `Base.fhirType()` names it upstream; its definitions live in the
    /// StructureDefinition the path starts with.
    fn context_of(type_name: &str) -> (&str, &str) {
        match type_name.find('.') {
            Some(dot) => (&type_name[..dot], type_name),
            None => (type_name, type_name),
        }
    }
}

impl Model for SdModel {
    fn is_type(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }

    fn base_type(&self, type_name: &str) -> Option<&str> {
        self.types.get(type_name)?.base.as_deref()
    }

    fn element_types(&self, type_name: &str, element: &str) -> Option<Vec<String>> {
        let (sd, path) = SdModel::context_of(type_name);
        self.children_of(sd, path)
            .find(|e| e.name == element)
            .map(|e| self.navigation_types(e))
    }

    fn elements(&self, type_name: &str) -> Option<Vec<ElementInfo>> {
        let (sd, path) = SdModel::context_of(type_name);
        Some(
            self.children_of(sd, path)
                .map(|e| ElementInfo {
                    name: e.name.clone(),
                    path: e.path.clone(),
                    types: self.navigation_types(e),
                    codes: if e.content_reference.is_some() {
                        Vec::new()
                    } else {
                        e.codes.clone()
                    },
                    choice: e.choice,
                    repeating: e.repeating,
                    content_reference: e.content_reference.clone(),
                })
                .collect(),
        )
    }

    fn specializations(&self, type_name: &str) -> Option<Vec<String>> {
        let mut out: Vec<String> = self
            .types
            .iter()
            .filter(|(_, t)| t.specialization && t.base.as_deref() == Some(type_name))
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        Some(out)
    }

    fn resource_type_names(&self) -> Option<Vec<String>> {
        let mut out: Vec<String> = self
            .types
            .iter()
            .filter(|(_, t)| t.specialization && t.kind == "resource")
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        Some(out)
    }

    fn type_names(&self) -> Option<Vec<String>> {
        let mut out: Vec<String> = self
            .types
            .iter()
            .filter(|(_, t)| t.specialization)
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        Some(out)
    }

    fn has_definition_url(&self, url: &str) -> Option<bool> {
        Some(self.urls.contains(url))
    }

    fn value_set(&self, url: &str) -> Option<Option<Value>> {
        self.terminology_resource(url, true)
    }

    fn code_system(&self, url: &str) -> Option<Option<Value>> {
        self.terminology_resource(url, false)
    }

    fn is_primitive(&self, type_name: &str) -> bool {
        self.types
            .get(type_name)
            .is_some_and(|t| t.kind == "primitive-type")
    }

    fn is_resource_type(&self, type_name: &str) -> bool {
        self.types
            .get(type_name)
            .is_some_and(|t| t.kind == "resource")
    }
}

/// A node of a typed input tree.
///
/// `by_name` indexes `children` by element name, so a navigation step costs
/// what the engine spends on it and not a scan of every sibling: without it
/// `Patient.birthDate` over a `Patient` with a thousand names reads a thousand
/// entries in this adapter, and `path.perf.004`'s ratio would measure the
/// harness rather than the engine.
#[derive(Debug)]
struct TreeNode {
    type_name: String,
    is_resource: bool,
    children: Vec<(String, Value)>,
    by_name: HashMap<String, Vec<Value>>,
    primitive: Option<Primitive>,
}

impl Node for TreeNode {
    fn type_name(&self) -> &str {
        &self.type_name
    }

    fn is_resource(&self) -> bool {
        self.is_resource
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(self.by_name.get(name).cloned().unwrap_or_default())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(self.children.clone())
    }

    fn primitive(&self) -> Option<Primitive> {
        self.primitive.clone()
    }
}

fn tree_node(
    type_name: &str,
    is_resource: bool,
    children: Vec<(String, Value)>,
    primitive: Option<Primitive>,
) -> Value {
    let mut by_name: HashMap<String, Vec<Value>> = HashMap::new();
    for (name, value) in &children {
        by_name.entry(name.clone()).or_default().push(value.clone());
    }
    Value::Node(Arc::new(TreeNode {
        type_name: type_name.to_string(),
        is_resource,
        children,
        by_name,
        primitive,
    }))
}

/// A primitive element: its value, and its `id` and extensions both as
/// children and on the [`Primitive`].
fn primitive_node(
    type_name: &str,
    lexical: Option<String>,
    id: Option<String>,
    extensions: Vec<Value>,
) -> Value {
    let mut children = Vec::new();
    if let Some(id) = &id {
        children.push((
            "id".to_string(),
            primitive_node("string", Some(id.clone()), None, Vec::new()),
        ));
    }
    for x in &extensions {
        children.push(("extension".to_string(), x.clone()));
    }
    let primitive = lexical.map(|lexical| Primitive {
        type_name: type_name.to_string(),
        lexical,
        id,
        extensions,
    });
    tree_node(type_name, false, children, primitive)
}

/// What a child element is, once its type is chosen.
enum Shape {
    Primitive(String),
    Complex {
        type_name: String,
        sd: String,
        path: String,
    },
    /// `Resource`-typed -- `contained`, `Bundle.entry.resource` -- whose
    /// actual type is in the content.
    Resource,
}

/// The type a typed choice name selects: `valueQuantity` -> `Quantity`.
fn choice_type<'e>(e: &'e ElementDef, key: &str) -> Option<&'e str> {
    let rest = key.strip_prefix(e.name.as_str())?;
    e.types.iter().map(String::as_str).find(|t| {
        let mut chars = t.chars();
        chars.next().is_some_and(|c| {
            let capitalised: String = c.to_uppercase().chain(chars).collect();
            capitalised == rest
        })
    })
}

struct TreeBuilder<'m> {
    model: &'m SdModel,
}

impl TreeBuilder<'_> {
    fn shape(&self, sd: &str, e: &ElementDef, chosen: &str) -> Shape {
        if let Some(r) = &e.content_reference {
            let (ref_sd, _) = SdModel::context_of(r);
            return Shape::Complex {
                type_name: r.clone(),
                sd: ref_sd.to_string(),
                path: r.clone(),
            };
        }
        if chosen == "Resource" {
            return Shape::Resource;
        }
        if matches!(chosen, "BackboneElement" | "Element") && self.model.has_children(sd, &e.path) {
            return Shape::Complex {
                type_name: e.path.clone(),
                sd: sd.to_string(),
                path: e.path.clone(),
            };
        }
        if self.model.is_primitive(chosen) {
            return Shape::Primitive(chosen.to_string());
        }
        Shape::Complex {
            type_name: chosen.to_string(),
            sd: chosen.to_string(),
            path: chosen.to_string(),
        }
    }

    fn extension(&self, children: Vec<(String, Value)>) -> Value {
        tree_node("Extension", false, children, None)
    }

    // -- XML --------------------------------------------------------------

    fn xml_resource(&self, el: &xml::Element) -> Value {
        let type_name = el.name.as_str();
        let children = self.xml_children(el, type_name, type_name, true);
        tree_node(type_name, true, children, None)
    }

    fn xml_children(
        &self,
        el: &xml::Element,
        sd: &str,
        path: &str,
        at_resource_root: bool,
    ) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for e in self.model.children_of(sd, path) {
            // Two elements are attributes in the XML form: `Element.id` (but
            // not `Resource.id`) and `Extension.url`.
            let is_attribute =
                (e.name == "id" && !at_resource_root) || (e.name == "url" && path == "Extension");
            if is_attribute {
                if el.has_attr(&e.name) {
                    let t = e.types.first().map(String::as_str).unwrap_or("string");
                    let lexical = Some(el.attr(&e.name).to_string());
                    out.push((e.name.clone(), primitive_node(t, lexical, None, Vec::new())));
                }
                continue;
            }
            for child in el.child_elements() {
                let chosen = if e.choice {
                    match choice_type(e, &child.name) {
                        Some(t) => t,
                        None => continue,
                    }
                } else if child.name == e.name {
                    e.types.first().map(String::as_str).unwrap_or("")
                } else {
                    continue;
                };
                if let Some(v) = self.xml_value(sd, e, chosen, child) {
                    out.push((e.name.clone(), v));
                }
            }
        }
        out
    }

    fn xml_value(
        &self,
        sd: &str,
        e: &ElementDef,
        chosen: &str,
        el: &xml::Element,
    ) -> Option<Value> {
        match self.shape(sd, e, chosen) {
            Shape::Resource => el.child_elements().first().map(|r| self.xml_resource(r)),
            Shape::Primitive(t) if t == "xhtml" => Some(primitive_node(
                &t,
                Some(el.text_content()),
                None,
                Vec::new(),
            )),
            Shape::Primitive(t) => {
                let lexical = el.has_attr("value").then(|| el.attr("value").to_string());
                let id = el.has_attr("id").then(|| el.attr("id").to_string());
                let extensions = el
                    .children_named("extension")
                    .into_iter()
                    .map(|x| self.extension(self.xml_children(x, "Extension", "Extension", false)))
                    .collect();
                Some(primitive_node(&t, lexical, id, extensions))
            }
            Shape::Complex {
                type_name,
                sd,
                path,
            } => Some(tree_node(
                &type_name,
                false,
                self.xml_children(el, &sd, &path, false),
                None,
            )),
        }
    }

    // -- JSON -------------------------------------------------------------

    fn json_resource(&self, obj: &json::Value) -> Option<Value> {
        let type_name = obj.get("resourceType")?.as_str()?;
        let children = self.json_children(obj, type_name, type_name);
        Some(tree_node(type_name, true, children, None))
    }

    fn json_children(&self, obj: &json::Value, sd: &str, path: &str) -> Vec<(String, Value)> {
        let json::Value::Object(members) = obj else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in self.model.children_of(sd, path) {
            for (key, value) in members {
                // A `_field` with no `field` beside it is a primitive that
                // carries only an id or extensions.
                let (base_key, is_sibling) = match key.strip_prefix('_') {
                    Some(k) => (k, true),
                    None => (key.as_str(), false),
                };
                let chosen = if e.choice {
                    match choice_type(e, base_key) {
                        Some(t) => t,
                        None => continue,
                    }
                } else if base_key == e.name {
                    e.types.first().map(String::as_str).unwrap_or("")
                } else {
                    continue;
                };
                if is_sibling {
                    if obj.get(base_key).is_none() {
                        self.json_push(sd, e, chosen, None, Some(value), &mut out);
                    }
                } else {
                    let sibling = obj.get(&format!("_{key}"));
                    self.json_push(sd, e, chosen, Some(value), sibling, &mut out);
                }
            }
        }
        out
    }

    fn json_push(
        &self,
        sd: &str,
        e: &ElementDef,
        chosen: &str,
        main: Option<&json::Value>,
        sibling: Option<&json::Value>,
        out: &mut Vec<(String, Value)>,
    ) {
        let sibling_at = |i: usize| match sibling {
            Some(json::Value::Array(items)) => items.get(i),
            _ => None,
        };
        match (main, sibling) {
            (Some(json::Value::Array(items)), _) => {
                for (i, item) in items.iter().enumerate() {
                    if let Some(v) = self.json_value(sd, e, chosen, Some(item), sibling_at(i)) {
                        out.push((e.name.clone(), v));
                    }
                }
            }
            (None, Some(json::Value::Array(items))) => {
                for s in items {
                    if let Some(v) = self.json_value(sd, e, chosen, None, Some(s)) {
                        out.push((e.name.clone(), v));
                    }
                }
            }
            _ => {
                if let Some(v) = self.json_value(sd, e, chosen, main, sibling) {
                    out.push((e.name.clone(), v));
                }
            }
        }
    }

    fn json_value(
        &self,
        sd: &str,
        e: &ElementDef,
        chosen: &str,
        main: Option<&json::Value>,
        sibling: Option<&json::Value>,
    ) -> Option<Value> {
        match self.shape(sd, e, chosen) {
            Shape::Resource => main.and_then(|v| self.json_resource(v)),
            Shape::Primitive(t) => {
                let lexical = match main {
                    Some(json::Value::String(s)) => Some(s.clone()),
                    Some(json::Value::Number(n)) => Some(n.clone()),
                    Some(json::Value::Bool(b)) => Some(b.to_string()),
                    _ => None,
                };
                let sibling = sibling.filter(|s| matches!(s, json::Value::Object(_)));
                if lexical.is_none() && sibling.is_none() {
                    return None;
                }
                let id = sibling
                    .and_then(|s| s.get("id"))
                    .and_then(json::Value::as_str)
                    .map(str::to_string);
                let extensions = match sibling.and_then(|s| s.get("extension")) {
                    Some(json::Value::Array(xs)) => xs
                        .iter()
                        .map(|x| self.extension(self.json_children(x, "Extension", "Extension")))
                        .collect(),
                    _ => Vec::new(),
                };
                Some(primitive_node(&t, lexical, id, extensions))
            }
            Shape::Complex {
                type_name,
                sd,
                path,
            } => {
                let v = main.filter(|v| matches!(v, json::Value::Object(_)))?;
                Some(tree_node(
                    &type_name,
                    false,
                    self.json_children(v, &sd, &path),
                    None,
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Results and failures, as the harness compares them
// ---------------------------------------------------------------------------

/// An evaluation failure as the harness grades it. A feature the evaluator
/// does not implement is `did-not-run`, never a graded answer.
fn engine_error(e: EvalError) -> EngineError {
    match e {
        EvalError::Unsupported { .. } => EngineError::NotImplemented,
        EvalError::Execution { message, offset } => EngineError::Diagnostic {
            message,
            // The offset is absent where HAPI's failing node has no start
            // (path.errors.002).
            offset,
        },
        EvalError::Node(e) => EngineError::Diagnostic {
            message: e.to_string(),
            offset: None,
        },
    }
}

fn child_lexical(node: &dyn Node, name: &str) -> Option<String> {
    node.children(name)
        .ok()?
        .first()?
        .as_node()?
        .primitive()
        .map(|p| p.lexical)
}

/// One evaluation result as the harness compares it: a literal by its FHIR
/// type, as upstream's literal objects are typed, and a node by the type the
/// model gave it.
fn result_item(model: &SdModel, value: &Value) -> ResultItem {
    let primitive = |fhir_type: &str, value: String| ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value,
    };
    match value {
        Value::Boolean(b) => primitive("boolean", b.to_string()),
        Value::Integer(i) => primitive("integer", i.to_string()),
        Value::Decimal(d) => primitive("decimal", d.lexical().to_string()),
        Value::String(s) => primitive("string", s.clone()),
        Value::Date(d) => primitive("date", d.lexical.clone()),
        Value::DateTime(d) => primitive("dateTime", d.lexical.clone()),
        Value::Time(t) => primitive("time", t.lexical.clone()),
        Value::Quantity(q) => ResultItem::Quantity(QuantityValue {
            value: q.value.lexical().to_string(),
            comparator: None,
            unit: q.unit.clone(),
            system: q.system.clone(),
            code: q.code.clone(),
            has_element_id: false,
            has_extensions: false,
        }),
        Value::Node(node) => {
            let type_name = node.type_name();
            if let Some(p) = node.primitive() {
                return primitive(type_name, p.lexical);
            }
            if type_name == "Quantity" || model.derives_from(type_name, "Quantity") {
                let node = node.as_ref();
                return ResultItem::Quantity(QuantityValue {
                    value: child_lexical(node, "value").unwrap_or_default(),
                    comparator: child_lexical(node, "comparator"),
                    unit: child_lexical(node, "unit"),
                    system: child_lexical(node, "system"),
                    code: child_lexical(node, "code"),
                    has_element_id: node.children("id").is_ok_and(|c| !c.is_empty()),
                    has_extensions: node.children("extension").is_ok_and(|c| !c.is_empty()),
                });
            }
            if model.is_primitive(type_name) {
                // A primitive element with no value -- only an id or
                // extensions -- is still a primitive on the value branch.
                return primitive(type_name, String::new());
            }
            ResultItem::Complex {
                fhir_type: type_name.to_string(),
            }
        }
    }
}
