//! The `path-check-warnings` in-process oracle.
//!
//! Task 4a of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo nextest run -p frevo-path --test path_check_warnings
//! ```
//!
//! Why an oracle of its own. The check pass's warning channel is reachable by
//! neither of the other two:
//!
//! - the vendored suite's `invalid="semantic"` selector grades only whether the
//!   check **fails**, and a warning never fails a check;
//! - the HTTP differential harness never sees the list at all, because a live
//!   HAPI server does not expose it.
//!
//! So the case selector here is the **warning message id**, each of which is a
//! stable constant in the upstream engine. Provenance for every expression and
//! every id below is
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`, with the constants themselves
//! in `org.hl7.fhir.utilities/.../i18n/I18nConstants.java`.
//!
//! The oracle runs against a trait object, so it compiles and runs before the
//! check pass exists. The unbound binding returns
//! [`CheckError::NotImplemented`] from every surface, which is an error value
//! and not a panicking placeholder, so a run reports what is missing instead of
//! aborting.
//!
//! Scenario coverage: `path.check.015` through `path.check.019`.

// ---------------------------------------------------------------------------
// The nine warning ids
// ---------------------------------------------------------------------------

/// Every warning the check pass can emit. Nine, and a tenth or a missing one
/// fails the oracle.
///
/// These are the nine `typeWarnings.add(...)` sites in `FHIRPathEngine.java`:
/// lines 1646 and 1649 (`doSQLOnFHIRCheck`), 1972/1980 and 1974/1983
/// (`checkCardinalityForComparabilitySame` and `checkCardinalityForSingle`),
/// 3302 (the ordered-function focus check), 3579 and 3588
/// (`extension(url)` type resolution), 3783 (`typeCastIsImpossible`), 3834
/// (`checkParamTypes`) and 3841 (`checkSingleton`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningId {
    ChoiceNoTypeSpecifier,
    ChoiceSpuriousTypeSpecifier,
    CollectionStatusOperationLeft,
    CollectionStatusOperationRight,
    CollectionStatusParameter,
    CollectionStatusContext,
    NotACollection,
    UnknownExtension,
    OfTypeImpossible,
}

impl WarningId {
    /// The complete set. The validator will key on these, so they are stable.
    pub const ALL: [WarningId; 9] = [
        WarningId::ChoiceNoTypeSpecifier,
        WarningId::ChoiceSpuriousTypeSpecifier,
        WarningId::CollectionStatusOperationLeft,
        WarningId::CollectionStatusOperationRight,
        WarningId::CollectionStatusParameter,
        WarningId::CollectionStatusContext,
        WarningId::NotACollection,
        WarningId::UnknownExtension,
        WarningId::OfTypeImpossible,
    ];

    /// The stable message id, which is this oracle's case selector.
    pub fn as_str(self) -> &'static str {
        match self {
            WarningId::ChoiceNoTypeSpecifier => "choice-no-type-specifier",
            WarningId::ChoiceSpuriousTypeSpecifier => "choice-spurious-type-specifier",
            WarningId::CollectionStatusOperationLeft => "collection-status-operation-left",
            WarningId::CollectionStatusOperationRight => "collection-status-operation-right",
            WarningId::CollectionStatusParameter => "collection-status-parameter",
            WarningId::CollectionStatusContext => "collection-status-context",
            WarningId::NotACollection => "not-a-collection",
            WarningId::UnknownExtension => "unknown-extension",
            WarningId::OfTypeImpossible => "of-type-impossible",
        }
    }

    /// The upstream `I18nConstants` name the id corresponds to, recorded so the
    /// mapping is traceable rather than asserted from memory.
    pub fn upstream_constant(self) -> &'static str {
        match self {
            WarningId::ChoiceNoTypeSpecifier => "FHIRPATH_CHOICE_NO_TYPE_SPECIFIER",
            WarningId::ChoiceSpuriousTypeSpecifier => "FHIRPATH_CHOICE_SPURIOUS_TYPE_SPECIFIER",
            WarningId::CollectionStatusOperationLeft => "FHIRPATH_COLLECTION_STATUS_OPERATION_LEFT",
            WarningId::CollectionStatusOperationRight => {
                "FHIRPATH_COLLECTION_STATUS_OPERATION_RIGHT"
            }
            WarningId::CollectionStatusParameter => "FHIRPATH_COLLECTION_STATUS_PARAMETER",
            WarningId::CollectionStatusContext => "FHIRPATH_COLLECTION_STATUS_CONTEXT",
            WarningId::NotACollection => "FHIRPATH_NOT_A_COLLECTION",
            WarningId::UnknownExtension => "FHIRPATH_UNKNOWN_EXTENSION",
            WarningId::OfTypeImpossible => "FHIRPATH_OFTYPE_IMPOSSIBLE",
        }
    }
}

/// One warning: a human-readable message and the stable id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckWarning {
    pub id: WarningId,
    pub message: String,
}

/// What the warning-returning check surface answers with: the inferred type
/// **and** the warnings, separately from success or failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    /// The inferred type set, as type names.
    pub inferred_types: Vec<String>,
    pub warnings: Vec<CheckWarning>,
}

impl CheckOutcome {
    pub fn ids(&self) -> Vec<WarningId> {
        let mut v: Vec<WarningId> = self.warnings.iter().map(|w| w.id).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    pub fn has(&self, id: WarningId) -> bool {
        self.warnings.iter().any(|w| w.id == id)
    }
}

/// A check failure. `NotImplemented` is not a check verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The check genuinely failed: this is what the suite's
    /// `invalid="semantic"` cases select for.
    Failed { message: String },
    /// No check pass is bound to this oracle yet.
    NotImplemented,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Failed { message } => f.write_str(message),
            CheckError::NotImplemented => f.write_str(
                "no check pass is bound to the path-check-warnings oracle yet \
                 (crates/frevo-path/tests/path_check_warnings.rs::oracle_under_test)",
            ),
        }
    }
}

/// The engine settings this oracle varies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckSettings {
    /// `FHIRPathEngine.emitSQLonFHIRWarning`, off by default
    /// (`path.strictness.008`). It gates both choice warnings and nothing else.
    pub emit_sql_on_fhir_warning: bool,
}

impl CheckSettings {
    pub const SQL_ON_FHIR: CheckSettings = CheckSettings {
        emit_sql_on_fhir_warning: true,
    };
}

/// The two check surfaces this oracle grades.
pub trait CheckOracle {
    /// The surface that accepts a warning list: it returns the inferred type
    /// **and** the warnings. Must clear its accumulator at entry, so warnings
    /// never leak between expressions on a reused engine.
    fn check_with_warnings(
        &mut self,
        expression: &str,
        context_type: &str,
        settings: CheckSettings,
    ) -> Result<CheckOutcome, CheckError>;

    /// The simple surface, which is the one the FHIRPath oracle harness calls.
    /// Same pass/fail verdict, warnings discarded.
    fn check(
        &mut self,
        expression: &str,
        context_type: &str,
        settings: CheckSettings,
    ) -> Result<Vec<String>, CheckError>;
}

/// The check pass this oracle grades.
///
/// Task 50a replaces this body with the real one, over the generated R4 model.
fn oracle_under_test() -> Box<dyn CheckOracle> {
    Box::new(UnboundCheck)
}

struct UnboundCheck;

impl CheckOracle for UnboundCheck {
    fn check_with_warnings(
        &mut self,
        _expression: &str,
        _context_type: &str,
        _settings: CheckSettings,
    ) -> Result<CheckOutcome, CheckError> {
        Err(CheckError::NotImplemented)
    }

    fn check(
        &mut self,
        _expression: &str,
        _context_type: &str,
        _settings: CheckSettings,
    ) -> Result<Vec<String>, CheckError> {
        Err(CheckError::NotImplemented)
    }
}

// ---------------------------------------------------------------------------
// The cases: at least one expression per message id
// ---------------------------------------------------------------------------

/// One oracle case. `expected` is the id the expression must raise; `forbidden`
/// are ids it must **not** raise, which is what keeps a case from passing by
/// emitting every warning at once.
struct WarningCase {
    expected: WarningId,
    context_type: &'static str,
    expression: &'static str,
    settings: CheckSettings,
    /// Why this expression reaches that site, with the upstream line.
    why: &'static str,
}

const CASES: &[WarningCase] = &[
    WarningCase {
        expected: WarningId::ChoiceNoTypeSpecifier,
        context_type: "Observation",
        expression: "Observation.value",
        settings: CheckSettings::SQL_ON_FHIR,
        why: "doSQLOnFHIRCheck at FHIRPathEngine.java:1646 — the `value` name node's \
              inferred type is a choice and its inner node is not an ofType() call",
    },
    WarningCase {
        expected: WarningId::ChoiceSpuriousTypeSpecifier,
        context_type: "Patient",
        expression: "Patient.name.ofType(HumanName)",
        settings: CheckSettings::SQL_ON_FHIR,
        why: "doSQLOnFHIRCheck at FHIRPathEngine.java:1649 — the `name` name node's \
              inferred type is not a choice, yet its inner node is an ofType() call",
    },
    WarningCase {
        expected: WarningId::CollectionStatusOperationLeft,
        context_type: "Patient",
        expression: "Patient.name.given < 'Z'",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "checkCardinalityForSingle at FHIRPathEngine.java:1980, reached from \
              operateTypes' LessThan arm: the left operand is a list where a singleton \
              is expected",
    },
    WarningCase {
        expected: WarningId::CollectionStatusOperationRight,
        context_type: "Patient",
        expression: "'Z' < Patient.name.given",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "checkCardinalityForSingle at FHIRPathEngine.java:1983 — the mirror case, \
              with the collection on the right",
    },
    WarningCase {
        expected: WarningId::CollectionStatusParameter,
        context_type: "Patient",
        expression: "Patient.birthDate.iif((true | false), 'a', 'b')",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "checkParamTypes at FHIRPathEngine.java:3834 — iif's first parameter is \
              declared a singleton boolean, and a union of two booleans is a collection \
              of boolean; the focus is a singleton, so no context warning is raised",
    },
    WarningCase {
        expected: WarningId::CollectionStatusContext,
        context_type: "Patient",
        expression: "Patient.name.given.iif(true, 'a', 'b')",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "checkSingleton at FHIRPathEngine.java:3841, whose only call site is the \
              `case Iif` arm at :3443 — the focus is not a singleton; the parameter is, \
              so no parameter warning is raised",
    },
    WarningCase {
        expected: WarningId::NotACollection,
        context_type: "Patient",
        expression: "Patient.birthDate.first()",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "FHIRPathEngine.java:3302 — first(), last(), tail(), skip() and take() \
              warn when the focus's collection status is already SINGLETON",
    },
    WarningCase {
        expected: WarningId::UnknownExtension,
        context_type: "Patient",
        expression: "Patient.extension('http://example.org/not-a-known-extension')",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "FHIRPathEngine.java:3588 — findExtensionDefinition returned nothing for \
              the URL, so the result type falls back to plain Extension with a warning",
    },
    WarningCase {
        expected: WarningId::OfTypeImpossible,
        context_type: "Patient",
        expression: "Patient.name.ofType(Quantity)",
        settings: CheckSettings {
            emit_sql_on_fhir_warning: false,
        },
        why: "typeCastIsImpossible at FHIRPathEngine.java:3783 — a HumanName focus can \
              never be a Quantity. The SQL-on-FHIR warning is off here so that the \
              spurious-type-specifier warning does not also fire",
    },
];

// ---------------------------------------------------------------------------
// path.check.015 — the surface, the id set, and a warning never failing
// ---------------------------------------------------------------------------

// covers: path.check.015
#[test]
fn the_check_surface_returns_warnings_beside_the_inferred_type() {
    // The complete id set is exactly nine. A tenth, or a missing one, fails.
    assert_eq!(WarningId::ALL.len(), 9);
    let mut ids: Vec<&str> = WarningId::ALL.iter().map(|i| i.as_str()).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "every message id must be distinct");
    assert_eq!(
        ids,
        [
            "choice-no-type-specifier",
            "choice-spurious-type-specifier",
            "collection-status-context",
            "collection-status-operation-left",
            "collection-status-operation-right",
            "collection-status-parameter",
            "not-a-collection",
            "of-type-impossible",
            "unknown-extension",
        ]
    );
    // And each maps to exactly one upstream constant.
    let mut upstream: Vec<&str> = WarningId::ALL
        .iter()
        .map(|i| i.upstream_constant())
        .collect();
    upstream.sort_unstable();
    upstream.dedup();
    assert_eq!(upstream.len(), 9);

    // The oracle's case table covers every id, so no id is specified without
    // an expression that reaches it.
    let mut covered: Vec<WarningId> = CASES.iter().map(|c| c.expected).collect();
    covered.sort_unstable();
    covered.dedup();
    assert_eq!(
        covered.len(),
        9,
        "every message id needs at least one expression: {covered:?}"
    );

    // Each case's check must SUCCEED and return the warning beside the
    // inferred type. A warning is not a failure, so the expression is still
    // evaluated afterwards.
    let mut oracle = oracle_under_test();
    for case in CASES {
        let outcome = oracle
            .check_with_warnings(case.expression, case.context_type, case.settings)
            .unwrap_or_else(|e| {
                panic!(
                    "checking {:?} against {} must succeed with a warning, not fail: {e}\n  {}",
                    case.expression, case.context_type, case.why
                )
            });
        assert!(
            outcome.has(case.expected),
            "{:?} must raise {} ({}); it raised {:?}\n  {}",
            case.expression,
            case.expected.as_str(),
            case.expected.upstream_constant(),
            outcome.ids(),
            case.why
        );
        for w in &outcome.warnings {
            assert!(
                !w.message.is_empty(),
                "{} must carry a message as well as an id",
                w.id.as_str()
            );
        }
        assert!(
            !outcome.inferred_types.is_empty(),
            "{:?} must still infer a type: a warning does not stop the check",
            case.expression
        );
    }
}

// ---------------------------------------------------------------------------
// path.check.016 — collection status against a singleton
// ---------------------------------------------------------------------------

// covers: path.check.016
#[test]
fn comparing_a_collection_with_a_singleton_warns_rather_than_fails() {
    let mut oracle = oracle_under_test();
    let off = CheckSettings::default();

    // The operand sides are identified separately, so an implementation cannot
    // satisfy both with one id.
    let left = oracle
        .check_with_warnings("Patient.name.given < 'Z'", "Patient", off)
        .expect("a collection on the left is a warning, not a check failure");
    assert_eq!(
        left.ids(),
        [WarningId::CollectionStatusOperationLeft],
        "only the left-operand id, so the sides are not conflated"
    );

    let right = oracle
        .check_with_warnings("'Z' < Patient.name.given", "Patient", off)
        .expect("a collection on the right is a warning, not a check failure");
    assert_eq!(right.ids(), [WarningId::CollectionStatusOperationRight]);

    // Both sides at once raises both.
    let both = oracle
        .check_with_warnings("Patient.name.given < Patient.name.family", "Patient", off)
        .expect("two collections is still a warning");
    assert!(both.has(WarningId::CollectionStatusOperationLeft));
    assert!(both.has(WarningId::CollectionStatusOperationRight));

    // A function parameter whose declared collection status is singleton, and
    // a focus whose status is not singleton, each have their own id.
    let param = oracle
        .check_with_warnings(
            "Patient.birthDate.iif((true | false), 'a', 'b')",
            "Patient",
            off,
        )
        .expect("a collection parameter is a warning");
    assert_eq!(param.ids(), [WarningId::CollectionStatusParameter]);

    let focus = oracle
        .check_with_warnings("Patient.name.given.iif(true, 'a', 'b')", "Patient", off)
        .expect("a non-singleton focus is a warning");
    assert_eq!(focus.ids(), [WarningId::CollectionStatusContext]);

    // And a singleton on both sides raises nothing at all, so the warning is
    // about the collection status and not about the operator.
    let clean = oracle
        .check_with_warnings("Patient.birthDate < @2000-01-01", "Patient", off)
        .expect("a singleton comparison checks cleanly");
    assert!(
        clean.warnings.is_empty(),
        "a singleton comparison must raise no warning: {:?}",
        clean.ids()
    );
}

// ---------------------------------------------------------------------------
// path.check.017 — the SQL-on-FHIR gate
// ---------------------------------------------------------------------------

// covers: path.check.017
#[test]
fn the_choice_warnings_appear_only_when_the_sql_on_fhir_check_is_enabled() {
    let mut oracle = oracle_under_test();

    // Off is the default, and with it the list is empty.
    assert!(!CheckSettings::default().emit_sql_on_fhir_warning);
    let off = oracle
        .check_with_warnings("Observation.value", "Observation", CheckSettings::default())
        .expect("navigating a choice element is not a check failure");
    assert!(
        off.warnings.is_empty(),
        "with the SQL-on-FHIR check off the warning list is empty: {:?}",
        off.ids()
    );

    // On, the same expression warns.
    let on = oracle
        .check_with_warnings(
            "Observation.value",
            "Observation",
            CheckSettings::SQL_ON_FHIR,
        )
        .expect("still not a failure");
    assert_eq!(on.ids(), [WarningId::ChoiceNoTypeSpecifier]);

    // ofType() on the choice silences it...
    let typed = oracle
        .check_with_warnings(
            "Observation.value.ofType(Quantity)",
            "Observation",
            CheckSettings::SQL_ON_FHIR,
        )
        .expect("still not a failure");
    assert!(
        typed.warnings.is_empty(),
        "ofType() on a choice element must produce no warning: {:?}",
        typed.ids()
    );

    // ...and ofType() on a non-choice element is the spurious case.
    let spurious = oracle
        .check_with_warnings(
            "Patient.name.ofType(HumanName)",
            "Patient",
            CheckSettings::SQL_ON_FHIR,
        )
        .expect("still not a failure");
    assert_eq!(spurious.ids(), [WarningId::ChoiceSpuriousTypeSpecifier]);

    // The setting gates the two choice warnings and nothing else: an unrelated
    // warning is unaffected by it.
    let unrelated_off = oracle
        .check_with_warnings(
            "Patient.birthDate.first()",
            "Patient",
            CheckSettings::default(),
        )
        .expect("still not a failure");
    let unrelated_on = oracle
        .check_with_warnings(
            "Patient.birthDate.first()",
            "Patient",
            CheckSettings::SQL_ON_FHIR,
        )
        .expect("still not a failure");
    assert_eq!(unrelated_off.ids(), [WarningId::NotACollection]);
    assert_eq!(unrelated_on.ids(), [WarningId::NotACollection]);
}

// ---------------------------------------------------------------------------
// path.check.018 — unknown extension, impossible ofType, not a collection
// ---------------------------------------------------------------------------

// covers: path.check.018
#[test]
fn an_unknown_extension_an_impossible_oftype_and_a_non_collection_each_warn() {
    let mut oracle = oracle_under_test();
    let off = CheckSettings::default();

    let unknown = oracle
        .check_with_warnings(
            "Patient.extension('http://example.org/not-a-known-extension')",
            "Patient",
            off,
        )
        .expect("an unknown extension URL is a warning, not a check failure");
    assert_eq!(unknown.ids(), [WarningId::UnknownExtension]);
    assert!(
        unknown.warnings.iter().any(|w| w
            .message
            .contains("http://example.org/not-a-known-extension")),
        "the warning must name the URL: {:?}",
        unknown.warnings
    );

    let impossible = oracle
        .check_with_warnings("Patient.name.ofType(Quantity)", "Patient", off)
        .expect("an impossible ofType is a warning, not a check failure");
    assert_eq!(impossible.ids(), [WarningId::OfTypeImpossible]);

    let not_a_collection = oracle
        .check_with_warnings("Patient.birthDate.first()", "Patient", off)
        .expect("a position-dependent function on a singleton is a warning");
    assert_eq!(not_a_collection.ids(), [WarningId::NotACollection]);

    // The same warning for each of the five position-dependent functions the
    // engine checks, and for none of the others.
    for f in ["first()", "last()", "tail()", "skip(1)", "take(1)"] {
        let expr = format!("Patient.birthDate.{f}");
        let outcome = oracle
            .check_with_warnings(&expr, "Patient", off)
            .unwrap_or_else(|e| panic!("{expr} must check: {e}"));
        assert_eq!(
            outcome.ids(),
            [WarningId::NotACollection],
            "{expr} must warn that its focus is not a collection"
        );
    }
    let on_a_collection = oracle
        .check_with_warnings("Patient.name.given.first()", "Patient", off)
        .expect("first() on a real collection checks cleanly");
    assert!(
        on_a_collection.warnings.is_empty(),
        "first() on a collection must raise no warning: {:?}",
        on_a_collection.ids()
    );
}

// ---------------------------------------------------------------------------
// path.check.019 — the simple surface, and clearing between checks
// ---------------------------------------------------------------------------

// covers: path.check.019
#[test]
fn the_simple_surface_discards_warnings_and_the_list_is_cleared_between_checks() {
    let mut oracle = oracle_under_test();
    let off = CheckSettings::default();

    // The simple surface returns the same verdict while discarding warnings.
    for case in CASES {
        let simple = oracle
            .check(case.expression, case.context_type, case.settings)
            .unwrap_or_else(|e| {
                panic!(
                    "the simple surface must reach the same verdict for {:?}: {e}",
                    case.expression
                )
            });
        let rich = oracle
            .check_with_warnings(case.expression, case.context_type, case.settings)
            .unwrap_or_else(|e| panic!("{:?}: {e}", case.expression));
        assert_eq!(
            simple, rich.inferred_types,
            "{:?}: both surfaces must infer the same type",
            case.expression
        );
        assert!(!rich.warnings.is_empty());
    }

    // A genuine semantic failure fails on both surfaces, so discarding the
    // warnings does not change the pass/fail answer either.
    let bad = "Patient.thereIsNoSuchElement";
    let simple_err = oracle
        .check(bad, "Patient", off)
        .expect_err("a non-existent element is a semantic failure");
    let rich_err = oracle
        .check_with_warnings(bad, "Patient", off)
        .expect_err("and it fails on the warning-returning surface too");
    assert!(matches!(simple_err, CheckError::Failed { .. }));
    assert!(matches!(rich_err, CheckError::Failed { .. }));

    // The accumulator is cleared at the start of every check, so warnings
    // cannot leak between expressions on a reused engine. Upstream this is the
    // `typeWarnings.clear()` at the head of each checkOnTypes overload
    // (FHIRPathEngine.java:478, :533, :540).
    let first = oracle
        .check_with_warnings("Patient.birthDate.first()", "Patient", off)
        .expect("checks");
    assert_eq!(first.ids(), [WarningId::NotACollection]);
    let second = oracle
        .check_with_warnings("Patient.name.given < 'Z'", "Patient", off)
        .expect("checks");
    assert_eq!(
        second.ids(),
        [WarningId::CollectionStatusOperationLeft],
        "the second check must not carry the first check's warnings"
    );
    let third = oracle
        .check_with_warnings("Patient.birthDate", "Patient", off)
        .expect("checks");
    assert!(
        third.warnings.is_empty(),
        "a clean expression after two warning ones must return an empty list: {:?}",
        third.ids()
    );

    // Including when a check in between went through the simple surface, which
    // accumulates warnings it never returns.
    let _ = oracle.check("Patient.birthDate.first()", "Patient", off);
    let after_simple = oracle
        .check_with_warnings("Patient.birthDate", "Patient", off)
        .expect("checks");
    assert!(
        after_simple.warnings.is_empty(),
        "warnings accumulated by the simple surface must not surface later: {:?}",
        after_simple.ids()
    );
}
