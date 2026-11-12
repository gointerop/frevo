//! Strictness configuration: `path.strictness.001` through
//! `path.strictness.010`.
//!
//! ```text
//! cargo test -p frevo-path --test path_strictness
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. HAPI configures one engine two
//! ways and both are observable, so neither is a deviation: both are HAPI.
//!
//! | Switch | Field | Engine default (pre-R5) | `FhirPathR4` |
//! |---|---|---|---|
//! | as-singleton rule | `doNotEnforceAsSingletonRule` :180 | not enforced (:214) | not enforced |
//! | `as` case sensitivity | `doNotEnforceAsCaseSensitive` :181 | not enforced (:213) | not enforced |
//! | polymorphic names | `allowPolymorphicNames` :177 | off | left off |
//! | implicit string conversion | `doImplicitStringConversion` :178 | off | left off |
//! | liquid mode | `liquidMode` :179 | off | left off |
//! | legacy mode | `legacyMode` :173 | off | left off |
//! | SQL-on-FHIR warning | `emitSQLonFHIRWarning` :184 | off | left off |
//!
//! `initFlags()` (FHIRPathEngine.java:211-216) is the whole of the first two
//! rows: `if (!VersionUtilities.isR5Plus(worker.getVersion()))` sets both
//! `doNotEnforce...` fields to `true`, which is "not enforced".
//!
//! The polarity is worth stating once: the Java fields are named
//! `doNotEnforce...`, and this file -- like the committed harness's
//! `StrictnessFlags` -- names them `enforce_...`, so `true` here means the rule
//! **is** enforced.
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! and
//! `reference/hapi-fhir/hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// The settings seam
//
// The committed harness carries the two `as` switches and the SQL-on-FHIR one;
// the other five are engine-wide fields with public getters and setters that
// nothing in the harness models. So the whole set is driven through this seam,
// whose unbound binding answers `NotImplemented` -- never a default.
// ---------------------------------------------------------------------------

/// The seven switches `FHIRPathEngine` exposes, in this crate's polarity:
/// `true` means the rule **is** enforced or the feature **is** on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Settings {
    enforce_as_singleton_rule: bool,
    enforce_as_case_sensitive: bool,
    allow_polymorphic_names: bool,
    implicit_string_conversion: bool,
    liquid_mode: bool,
    legacy_mode: bool,
    emit_sql_on_fhir_warning: bool,
}

impl Settings {
    /// What `initFlags()` leaves a pre-R5 engine at, with every other switch
    /// off. This is the value `path.strictness.001` asserts.
    const PRE_R5_DEFAULT: Settings = Settings {
        enforce_as_singleton_rule: false,
        enforce_as_case_sensitive: false,
        allow_polymorphic_names: false,
        implicit_string_conversion: false,
        liquid_mode: false,
        legacy_mode: false,
        emit_sql_on_fhir_warning: false,
    };

    fn enforcing_as(self) -> Settings {
        Settings {
            enforce_as_singleton_rule: true,
            enforce_as_case_sensitive: true,
            ..self
        }
    }
}

trait ConfiguredEngine {
    /// The settings the engine reports, as created.
    fn settings(&self) -> Result<Settings, EngineError>;

    fn evaluate_with(
        &mut self,
        expr: &str,
        inp: Option<&SuiteInput>,
        settings: Settings,
    ) -> Result<Vec<ResultItem>, EngineError>;

    fn check_with(
        &mut self,
        expr: &str,
        inp: Option<&SuiteInput>,
        settings: Settings,
    ) -> Result<(), EngineError>;
}

/// An engine created for `version` with no explicit configuration, which is
/// what `path.strictness.001` and `path.strictness.007` are about.
fn engine_for_version(_version: &str) -> Box<dyn ConfiguredEngine> {
    Box::new(UnboundConfigured)
}

/// An engine created the way `FhirPathR4`'s constructor creates one.
fn server_facing_engine() -> Box<dyn ConfiguredEngine> {
    Box::new(UnboundConfigured)
}

struct UnboundConfigured;

impl ConfiguredEngine for UnboundConfigured {
    fn settings(&self) -> Result<Settings, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate_with(
        &mut self,
        _expr: &str,
        _inp: Option<&SuiteInput>,
        _settings: Settings,
    ) -> Result<Vec<ResultItem>, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn check_with(
        &mut self,
        _expr: &str,
        _inp: Option<&SuiteInput>,
        _settings: Settings,
    ) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }
}

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
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
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be evaluated over: {e}")
    })
}

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is evaluated: {e}"));
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

fn boolean(value: bool) -> Vec<ResultItem> {
    primitive("boolean", if value { "true" } else { "false" })
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

/// The settings a freshly created engine reports, requiring the seam to answer.
fn settings_for(version: &str) -> Settings {
    engine_for_version(version)
        .settings()
        .unwrap_or_else(|e| panic!("an engine created for {version} must report its settings: {e}"))
}

/// Evaluate `expr` with `settings`, requiring success.
fn eval_with(expr: &str, inp: Option<&SuiteInput>, settings: Settings) -> Vec<ResultItem> {
    engine_for_version("4.0.1")
        .evaluate_with(expr, inp, settings)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate under {settings:?}: {e}"))
}

/// `expr` must fail at evaluation under `settings`, with a real diagnostic.
fn eval_failure_with(expr: &str, inp: Option<&SuiteInput>, settings: Settings) -> String {
    match engine_for_version("4.0.1").evaluate_with(expr, inp, settings) {
        Ok(items) => panic!("{expr:?} must fail under {settings:?}, but returned {items:?}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

// ---------------------------------------------------------------------------
// path.strictness.001 -- the default configuration
// ---------------------------------------------------------------------------

// covers: path.strictness.001
#[test]
fn the_default_configuration_is_non_strict_for_every_pre_r5_version() {
    // `initFlags()` (FHIRPathEngine.java:211-216) sets both `as` switches for
    // **any** worker context below R5, so the R4 default is not enforced.
    let r4 = settings_for("4.0.1");
    assert!(
        !r4.enforce_as_singleton_rule,
        "the as-singleton rule is not enforced by default"
    );
    assert!(
        !r4.enforce_as_case_sensitive,
        "`as` is not case-sensitive by default"
    );
    assert_eq!(r4, Settings::PRE_R5_DEFAULT);

    // Every pre-R5 version, not just R4: the condition is `!isR5Plus`.
    for version in ["1.0.2", "3.0.2", "4.0.1", "4.3.0"] {
        let s = settings_for(version);
        assert!(
            !s.enforce_as_singleton_rule && !s.enforce_as_case_sensitive,
            "{version} is pre-R5, so neither rule is enforced: {s:?}"
        );
    }

    // `FhirPathR4`'s constructor re-asserts the same two values --
    // `setDoNotEnforceAsCaseSensitive(true)` and
    // `setDoNotEnforceAsSingletonRule(true)` -- rather than changing them, so
    // the server-facing default and the engine default coincide.
    let server = server_facing_engine()
        .settings()
        .expect("the server-facing engine must report its settings");
    assert_eq!(
        server, r4,
        "FhirPathR4 re-asserts the engine default rather than changing it"
    );
}

// ---------------------------------------------------------------------------
// path.strictness.002 -- the harness's per-stage flags
// ---------------------------------------------------------------------------

// covers: path.strictness.002
#[test]
fn the_oracle_harness_checks_non_strict_and_evaluates_strict() {
    // `FHIRPathTests.test` calls `check(...)` first and only then
    // `setDoNotEnforceAsCaseSensitive(false)` and
    // `setDoNotEnforceAsSingletonRule(false)` before `evaluate(...)`. So the
    // check pass sees the engine's own initialisation -- both rules **not**
    // enforced -- and the evaluation sees both **enforced**.
    const { assert!(!StrictnessFlags::CHECK.enforce_as_singleton_rule) };
    const { assert!(!StrictnessFlags::CHECK.enforce_as_case_sensitive) };
    const { assert!(StrictnessFlags::EVALUATE.enforce_as_singleton_rule) };
    const { assert!(StrictnessFlags::EVALUATE.enforce_as_case_sensitive) };

    // The harness sets them per case rather than relying on a shared engine: in
    // the pinned Java harness the engine is `static`, so only the very first
    // case's check pass sees the initial values and every later one sees
    // `false`. Frevo's run must behave identically for the first case and the
    // last, which is asserted by reading the recorded flags of both.
    let (suite, report) = pinned();
    let first = suite.cases().next().expect("the suite has cases");
    let last = suite.cases().last().expect("the suite has cases");
    for case in [first, last] {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        assert_eq!(
            r.check_flags,
            StrictnessFlags::CHECK,
            "{} must record the check flags",
            r.address
        );
        assert_eq!(
            r.evaluate_flags,
            StrictnessFlags::EVALUATE,
            "{} must record the evaluation flags",
            r.address
        );
    }

    // And the run really graded both, so the recorded flags describe something
    // that happened.
    assert_eq!(
        report.count(CaseStatus::DidNotRun),
        0,
        "the flags are only meaningful for a case that ran"
    );
}

// ---------------------------------------------------------------------------
// path.strictness.003 -- the as-singleton rule, enforced
// ---------------------------------------------------------------------------

// covers: path.strictness.003
#[test]
fn the_as_singleton_rule_rejects_a_multi_item_focus_when_enforced() {
    let patient = load_input("patient-example.xml");

    // Two sites, one per spelling: `opAs` (FHIRPathEngine.java:1884) and
    // `funcAs` (:5166), both
    //
    //     if (!doNotEnforceAsSingletonRule && left.size() > 1)
    //         throw ... FHIRPATH_AS_COLLECTION
    //
    // so with the rule enforced a three-item focus raises.
    let enforcing = Settings::PRE_R5_DEFAULT.enforcing_as();
    let message = eval_failure_with("Patient.name as HumanName", Some(&patient), enforcing);
    assert!(
        !message.is_empty(),
        "the as-collection error must carry a message"
    );
    eval_failure_with("Patient.name.as(HumanName)", Some(&patient), enforcing);

    // A singleton focus is fine under the same setting, so the rejection is
    // about the size and not about `as`.
    assert_eq!(
        eval_with("Patient.gender as code", Some(&patient), enforcing),
        primitive("code", "male")
    );
}

// ---------------------------------------------------------------------------
// path.strictness.004 -- the as-singleton rule, not enforced
// ---------------------------------------------------------------------------

// covers: path.strictness.004
#[test]
fn the_as_singleton_rule_allows_a_multi_item_focus_when_not_enforced() {
    let patient = load_input("patient-example.xml");

    // Not enforced is the default, and then the guard is skipped and the loop
    // tests each item: `for (Base nextLeft : left)` (:1887).
    let lenient = Settings::PRE_R5_DEFAULT;
    assert!(!lenient.enforce_as_singleton_rule);

    let values = eval_with("Patient.name as HumanName", Some(&patient), lenient);
    assert_eq!(
        values.len(),
        3,
        "each of the three names is tested and kept: {values:?}"
    );
    assert_eq!(
        eval_with(
            "Patient.name.as(HumanName).count()",
            Some(&patient),
            lenient
        ),
        primitive("integer", "3")
    );

    // Each item is tested, so a type that matches none of them yields empty
    // rather than everything.
    assert_eq!(
        eval_with("Patient.name as Quantity", Some(&patient), lenient),
        empty()
    );
}

// ---------------------------------------------------------------------------
// path.strictness.005 -- a mis-cased type name
// ---------------------------------------------------------------------------

// covers: path.strictness.005
#[test]
fn a_mis_cased_type_name_resolves_only_when_case_is_not_enforced() {
    let patient = load_input("patient-example.xml");

    // `compareTypeNames` (FHIRPathEngine.java:1896-1902):
    //
    //     if (doNotEnforceAsCaseSensitive) return left.equalsIgnoreCase(right);
    //     else                             return left.equals(right);
    //
    // So with case **not** enforced -- the server-facing default -- `Code`
    // matches the FHIR type `code`.
    let lenient = Settings::PRE_R5_DEFAULT;
    assert_eq!(
        eval_with("Patient.gender.as(Code)", Some(&patient), lenient),
        primitive("code", "male")
    );

    // With case enforced -- the harness's evaluation configuration -- `Code`
    // does not name a FHIR type and the result is empty. Empty, not an error:
    // the guard that raises is the singleton one, not this.
    let strict = Settings::PRE_R5_DEFAULT.enforcing_as();
    assert_eq!(
        eval_with("Patient.gender.as(Code)", Some(&patient), strict),
        empty()
    );

    // Upstream corroboration, and the reason the lenient default exists:
    // `org.hl7.fhir.r4.utils.FhirPathEngineR4Test.testAs` asserts that
    // `Observation.value.as(String)` and `as(FHIR.String)` both resolve through
    // `FhirPathR4`, with the comment "Allow for bad casing on primitive type
    // names - this is a common mistake and even some 4.0.1 SPs use it".
    let observation = load_input("observation-example.xml");
    assert_eq!(
        eval_with(
            "Observation.value.as(Quantity).unit",
            Some(&observation),
            lenient
        ),
        primitive("string", "lbs")
    );
    assert_eq!(
        eval_with(
            "Observation.value.as(quantity).unit",
            Some(&observation),
            lenient
        ),
        primitive("string", "lbs"),
        "mis-cased the other way round, which is the same rule"
    );
    assert_eq!(
        eval_with("Observation.value.as(quantity)", Some(&observation), strict),
        empty()
    );

    // No live case grades either state, so both are differential-only: every
    // suite case is evaluated with case **enforced** and checked with it not
    // enforced, and none of them mis-cases a type name.
    let (suite, _) = pinned();
    assert_eq!(
        suite
            .cases()
            .filter(|c| c.expression.contains("as(Code)") || c.expression.contains("as(quantity)"))
            .count(),
        0
    );
}

// ---------------------------------------------------------------------------
// path.strictness.006 -- lenient polymorphic names
// ---------------------------------------------------------------------------

// covers: path.strictness.006
#[test]
fn lenient_polymorphic_names_are_supported_by_the_engine_and_left_unset_by_hapi() {
    let observation = load_input("observation-example.xml");

    // Off is the default and what both HAPI wrappers leave it at, and then the
    // typed spelling is a semantic error (`path.navigation.007`).
    let off = Settings::PRE_R5_DEFAULT;
    assert!(!off.allow_polymorphic_names);
    match engine_for_version("4.0.1").check_with(
        "Observation.valueQuantity.exists()",
        Some(&observation),
        off,
    ) {
        Err(EngineError::Diagnostic { message, .. }) => assert!(
            message.contains("valueQuantity"),
            "the semantic error must name the element: {message}"
        ),
        Err(e) => panic!("the check must answer with a diagnostic, not: {e}"),
        Ok(()) => panic!("with polymorphic names off, the typed spelling must fail the check"),
    }
    expect_case("polymorphics", "testPolymorphicsB");
    expect_case("testObservations", "testPolymorphismB");

    // On, the check passes and the expression evaluates: the field is read both
    // when resolving children during evaluation and when resolving an element
    // definition during the check (`allowPolymorphicNames`,
    // FHIRPathEngine.java:177).
    let on = Settings {
        allow_polymorphic_names: true,
        ..off
    };
    engine_for_version("4.0.1")
        .check_with("Observation.valueQuantity.exists()", Some(&observation), on)
        .expect("with polymorphic names allowed the check must pass");
    assert_eq!(
        eval_with("Observation.valueQuantity.exists()", Some(&observation), on),
        boolean(true)
    );
    assert_eq!(
        eval_with("Observation.valueQuantity.unit", Some(&observation), on),
        primitive("string", "lbs")
    );

    // The suite's comment that lenient polymorphic mode "is not supported in
    // R4" describes two `modeTest` elements that are themselves commented out
    // and are not among the 1,191; it does not describe the engine, which has
    // the field, a public getter and setter, and honours it. Asserted as the
    // case count, which the harness counts by XML parse.
    let (suite, _) = pinned();
    assert_eq!(suite.case_count(), 1191);

    // No live case grades the allowed state, so it is differential-only: every
    // case that writes a typed polymorphic name expects a semantic failure.
    let typed: Vec<_> = suite
        .cases()
        .filter(|c| c.expression.contains("valueQuantity"))
        .collect();
    assert_eq!(typed.len(), 2, "two cases write the typed spelling");
    assert!(
        typed
            .iter()
            .all(|c| c.expected_failure == Some(oracle::suite::Stage::Check)),
        "all three expect a semantic failure, so none grades the allowed state"
    );
}

// ---------------------------------------------------------------------------
// path.strictness.007 -- legacy mode is not enabled for R4
// ---------------------------------------------------------------------------

// covers: path.strictness.007
#[test]
fn legacy_mode_is_not_enabled_for_r4() {
    // `legacyMode` (FHIRPathEngine.java:173) exists because "some R2 and R3
    // constraints assume that != is valid for empty sets". It is never set by
    // either HAPI configuration, so an R4 engine has it off.
    let r4 = settings_for("4.0.1");
    assert!(!r4.legacy_mode);
    let server = server_facing_engine()
        .settings()
        .expect("the server-facing engine reports its settings");
    assert!(!server.legacy_mode);

    // The observable consequence is at `opEquals`-adjacent code
    // (FHIRPathEngine.java:2133):
    //
    //     if (!legacyMode && (left.size() == 0 || right.size() == 0))
    //         return makeNull();
    //
    // So with legacy mode off, a comparison with an empty operand is **empty**
    // rather than a boolean.
    let off = Settings::PRE_R5_DEFAULT;
    assert_eq!(eval_with("{} != 1", None, off), empty());
    assert_eq!(eval_with("1 != {}", None, off), empty());
    assert_eq!(eval_with("({} != 1).empty()", None, off), boolean(true));

    // With legacy mode on it would not be empty, which is what makes the
    // default observable rather than merely declared.
    let on = Settings {
        legacy_mode: true,
        ..off
    };
    assert_ne!(eval_with("{} != 1", None, on), empty());
}

// ---------------------------------------------------------------------------
// path.strictness.008 -- the SQL-on-FHIR warning is off by default
// ---------------------------------------------------------------------------

// covers: path.strictness.008
#[test]
fn the_sql_on_fhir_warning_is_off_by_default() {
    // `emitSQLonFHIRWarning` (FHIRPathEngine.java:184) gates
    // `doSQLOnFHIRCheck` (:1642), and neither HAPI configuration sets it.
    assert!(!settings_for("4.0.1").emit_sql_on_fhir_warning);
    assert!(
        !server_facing_engine()
            .settings()
            .expect("reports settings")
            .emit_sql_on_fhir_warning
    );

    // The harness's own configuration has it off too, which is what makes the
    // suite's 1,191 cases silent on the choice warnings.
    assert!(!EngineConfig::default().emit_sql_on_fhir_warning);

    // Checking a choice element without `ofType()` must therefore produce no
    // warning and -- the part that matters here -- no failure either. The
    // warning list itself is graded by `path.check.017` in the committed
    // `path_check_warnings` oracle.
    let observation = load_input("observation-example.xml");
    engine_for_version("4.0.1")
        .check_with(
            "Observation.value",
            Some(&observation),
            Settings::PRE_R5_DEFAULT,
        )
        .expect("navigating a choice element must check cleanly with the warning off");

    let mut engine = fresh_engine();
    let parsed = engine.parse("Observation.value").expect("parses");
    engine
        .check(
            parsed,
            &context_for(Some(&observation)),
            StrictnessFlags::CHECK,
        )
        .expect("and through the suite's own configuration too");
}

// ---------------------------------------------------------------------------
// path.strictness.009 -- implicit string conversion is off by default
// ---------------------------------------------------------------------------

// covers: path.strictness.009
#[test]
fn implicit_string_conversion_is_off_by_default() {
    // `doImplicitStringConversion` (FHIRPathEngine.java:178) is the second half
    // of the guard on all thirteen string functions, and neither HAPI
    // configuration sets it.
    assert!(!settings_for("4.0.1").implicit_string_conversion);
    assert!(
        !server_facing_engine()
            .settings()
            .expect("reports settings")
            .implicit_string_conversion
    );

    // So a string function over a non-string focus yields empty, which is
    // `path.string.020` and `DEV-0006`.
    let off = Settings::PRE_R5_DEFAULT;
    assert_eq!(eval_with("1.startsWith('1')", None, off), empty());
    assert_eq!(eval_with("1.length()", None, off), empty());

    // And with it on the same expressions answer, which is what makes the
    // default observable (`path.string.024`).
    let on = Settings {
        implicit_string_conversion: true,
        ..off
    };
    assert_eq!(eval_with("1.startsWith('1')", None, on), boolean(true));
    assert_eq!(eval_with("1.length()", None, on), primitive("integer", "1"));
}

// ---------------------------------------------------------------------------
// path.strictness.010 -- liquid mode is out of scope and must be off
// ---------------------------------------------------------------------------

// covers: path.strictness.010
#[test]
fn liquid_mode_is_out_of_scope_and_must_be_off() {
    // `liquidMode` (FHIRPathEngine.java:179): "in liquid mode, || terminates
    // the expression and hands the parser back to the host". Nothing this
    // change ships sets it.
    assert!(!settings_for("4.0.1").liquid_mode);
    assert!(
        !server_facing_engine()
            .settings()
            .expect("reports settings")
            .liquid_mode
    );
    const { assert!(!Settings::PRE_R5_DEFAULT.liquid_mode) };

    // With liquid mode off, `||` terminates nothing: it is not a FHIRPath
    // token at all, so an expression containing it is a parse error rather than
    // a truncated expression.
    let off = Settings::PRE_R5_DEFAULT;
    match engine_for_version("4.0.1").evaluate_with("1 || 2", None, off) {
        Err(EngineError::Diagnostic { .. }) => {}
        Err(e) => panic!("`1 || 2` must be a diagnostic, not: {e}"),
        Ok(items) => panic!("`1 || 2` must not evaluate with liquid mode off: {items:?}"),
    }

    // And a valid expression followed by `||` is rejected rather than silently
    // truncated to its valid prefix, which is the failure mode liquid mode
    // would introduce.
    match engine_for_version("4.0.1").evaluate_with("1 + 1 || junk", None, off) {
        Err(EngineError::Diagnostic { .. }) => {}
        Err(e) => panic!("the trailing `||` must be a diagnostic, not: {e}"),
        Ok(items) => panic!("the expression must not be truncated: {items:?}"),
    }

    // `add-narrative` owns the enabled state, so no scenario here turns it on.
    let on = Settings {
        liquid_mode: true,
        ..off
    };
    assert_ne!(off, on, "the setting exists and is distinguishable");
}
