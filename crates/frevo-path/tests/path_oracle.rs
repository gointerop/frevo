//! The oracle gate: `path.oracle.011`, `012`, `013`, `014` and `016`.
//!
//! ```text
//! cargo test -p frevo-path --test path_oracle
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. `path.oracle.001`-`010`, `015` and
//! `017` are the harness itself and are graded by the committed
//! `fhirpath_oracle.rs`. The five here are the **gate**: Frevo's pass count
//! against HAPI 8.12.0's, per case, which task 3 measures and wires in.
//!
//! So these tests have two reasons to fail today, and both are the capability
//! being absent rather than a broken test:
//!
//! 1. no HAPI record exists at [`HAPI_RESULTS`], so there is nothing to compare
//!    against; and
//! 2. no engine is bound, so Frevo's own side of the comparison is 1,191
//!    `did-not-run`s.
//!
//! The record's location and shape are stated here because the test is what
//! defines them: task 3 writes the file, this file reads it, and a disagreement
//! between the two is a test failure rather than a silent skip.
//!
//! Provenance for HAPI's side: `org.hl7.fhir.r4.test.FHIRPathTests` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, run against the same pinned
//! suite file.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use oracle::input;
use oracle::json::{self, Value};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;

// ---------------------------------------------------------------------------
// HAPI's measured result for the same 1,191 cases
// ---------------------------------------------------------------------------

/// Where task 3 records what HAPI 8.12.0 did with each case.
///
/// Shape, one entry per case, addressed exactly as the harness addresses a case
/// (`group/name[ordinal]`, because `group/name` is not unique --
/// `testEquivalent/testEquivalent23` appears twice):
///
/// ```json
/// {
///   "engine": "org.hl7.fhir.r4.test.FHIRPathTests",
///   "sha": "73c432b5f43ddbce979978a760ec4aa81e929094",
///   "suite": "oracles/fhirpath/tests-fhir-r4.xml",
///   "cases": [
///     { "address": "testBasics/testSimple[1]", "status": "passed" },
///     { "address": "isSemanticChecks/isGuardOrBad[25]", "status": "hapi-cannot-pass",
///       "reason": "reads neither context nor contextType" }
///   ]
/// }
/// ```
const HAPI_RESULTS: &str = "oracles/fhirpath/hapi-8.12.0-results.json";

/// The three statuses HAPI's side may carry. `HapiCannotPass` is distinct from
/// `Failed` on purpose: it marks a case the pinned R4 harness cannot express at
/// all, so it is not evidence about HAPI's correctness (`path.oracle.016`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HapiStatus {
    Passed,
    Failed,
    HapiCannotPass,
}

impl HapiStatus {
    fn parse(s: &str) -> Option<HapiStatus> {
        match s {
            "passed" => Some(HapiStatus::Passed),
            "failed" => Some(HapiStatus::Failed),
            "hapi-cannot-pass" => Some(HapiStatus::HapiCannotPass),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            HapiStatus::Passed => "passed",
            HapiStatus::Failed => "failed",
            HapiStatus::HapiCannotPass => "hapi-cannot-pass",
        }
    }
}

/// HAPI's record, keyed by case address.
fn hapi_results() -> BTreeMap<String, (HapiStatus, Option<String>)> {
    let path = input::repo_path(HAPI_RESULTS);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{HAPI_RESULTS} does not exist ({e}). Task 3 runs \
             org.hl7.fhir.r4.test.FHIRPathTests at pin 73c432b5 against the \
             same suite file and records each case's status there. Until it \
             does, the gate has nothing to compare against -- which is not the \
             same as the gate passing."
        )
    });
    let doc = json::parse(&text).unwrap_or_else(|e| panic!("{HAPI_RESULTS} is not JSON: {e}"));

    let cases = match doc.get("cases") {
        Some(Value::Array(items)) => items,
        other => panic!("{HAPI_RESULTS} must carry a \"cases\" array, got {other:?}"),
    };
    let mut out = BTreeMap::new();
    for entry in cases {
        let address = entry
            .get("address")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("every entry needs an \"address\": {entry:?}"));
        let status = entry
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("every entry needs a \"status\": {entry:?}"));
        let status = HapiStatus::parse(status).unwrap_or_else(|| {
            panic!("{address}: {status:?} is not one of passed, failed, hapi-cannot-pass")
        });
        let reason = entry
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string);
        if out.insert(address.to_string(), (status, reason)).is_some() {
            panic!("{HAPI_RESULTS} records {address} twice");
        }
    }
    out
}

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// The six `isSemanticChecks` groups, 13 cases, whose expressions are
/// byte-identical, which share an `inputfile`, which differ only by `context`
/// and `contextType`, and whose expected outcomes differ (`path.oracle.016`,
/// task 3). The pinned R4 harness reads neither attribute, so it gives every
/// member of a group the same outcome and necessarily fails at least one.
const HAPI_CANNOT_PASS_GROUPS: [&[&str]; 6] = [
    &["isEffectOnLogic2Base", "isEffectOnLogic3Base"],
    &["isGuardOr", "isGuardOr2", "isGuardOrBad"],
    &["isGuardXorBad", "isGuardXorBad2"],
    &["isGuardOrInnerImplies", "isGuardOrInnerImpliesBad"],
    &["isGuardOrTypesImpliesBad", "isGuardOrTypesImpliesAll"],
    &["isGuardNotImplies", "isGuardNotImpliesBad"],
];

/// The two groups that also share an expression and differ only in
/// `context`/`contextType`, but whose expected outcomes are identical, so HAPI
/// is not forced to fail any member and they are **not** `hapi-cannot-pass`.
const SAME_OUTCOME_GROUPS: [[&str; 2]; 2] = [
    ["isEffectOnLogic1", "isGuardImpliesEmpty"],
    ["isEffectOnLogic2", "isEffectOnLogic3"],
];

/// Frevo's graded status for the 13, from its own context-aware run, as
/// `path.oracle.016` breaks it down. `None` is a case that passes the check and
/// is graded by evaluation: no status is asserted for it here, only that it is
/// not a plain check-stage failure.
const FREVO_HAPI_CANNOT_PASS_STATUS: [(&str, Option<CaseStatus>); 13] = [
    // Expect success, end at the check stage graded `failed`: the check pass
    // does not narrow (`path.check.005`, `path.check.009`).
    ("isGuardOr", Some(CaseStatus::Failed)),
    ("isGuardOrInnerImplies", Some(CaseStatus::Failed)),
    ("isGuardNotImplies", Some(CaseStatus::Failed)),
    // `invalid="semantic"`, fail the check, graded `passed`.
    ("isEffectOnLogic3Base", Some(CaseStatus::Passed)),
    ("isGuardOrBad", Some(CaseStatus::Passed)),
    ("isGuardXorBad", Some(CaseStatus::Passed)),
    ("isGuardOrInnerImpliesBad", Some(CaseStatus::Passed)),
    ("isGuardOrTypesImpliesBad", Some(CaseStatus::Passed)),
    ("isGuardNotImpliesBad", Some(CaseStatus::Passed)),
    // Expect success, pass the check, graded by evaluation.
    ("isEffectOnLogic2Base", None),
    ("isGuardOr2", None),
    ("isGuardOrTypesImpliesAll", None),
    ("isGuardXorBad2", None),
];

// ---------------------------------------------------------------------------
// path.oracle.011 -- HAPI's result is recorded per case
// ---------------------------------------------------------------------------

// covers: path.oracle.011
#[test]
fn the_harness_records_hapis_result_for_the_same_case() {
    let (suite, report) = pinned();
    let hapi = hapi_results();

    // Every case the harness runs has a HAPI status beside it, so the gate can
    // be evaluated per case and not only in aggregate.
    assert_eq!(
        hapi.len(),
        suite.case_count(),
        "HAPI's record must cover all {} cases",
        suite.case_count()
    );
    let mut missing = Vec::new();
    for case in &report.cases {
        if !hapi.contains_key(&case.address) {
            missing.push(case.address.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "{} case(s) have no HAPI status; the first few: {:?}",
        missing.len(),
        &missing[..missing.len().min(5)]
    );

    // The status is one of exactly three values, which the loader enforces, and
    // all three are actually used -- a record in which `hapi-cannot-pass` never
    // appears would mean `path.oracle.016` was not applied.
    let mut seen: Vec<HapiStatus> = hapi.values().map(|(s, _)| *s).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen,
        [
            HapiStatus::Passed,
            HapiStatus::Failed,
            HapiStatus::HapiCannotPass
        ],
        "all three statuses must appear in the record"
    );

    // A `hapi-cannot-pass` entry carries its reason, because the status is a
    // claim about the harness and not a measurement.
    for (address, (status, reason)) in &hapi {
        if *status == HapiStatus::HapiCannotPass {
            assert!(
                reason.as_ref().is_some_and(|r| !r.is_empty()),
                "{address} is hapi-cannot-pass and must say why"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// path.oracle.012 -- the gate is pass rate at or above HAPI's
// ---------------------------------------------------------------------------

// covers: path.oracle.012
#[test]
fn the_gate_is_pass_rate_at_or_above_hapis_on_the_same_cases() {
    let (_, report) = pinned();
    let hapi = hapi_results();

    let hapi_passed = hapi
        .values()
        .filter(|(s, _)| *s == HapiStatus::Passed)
        .count();
    let frevo_passed = report.passed();

    // The per-case lists, which the gate report carries regardless of the
    // aggregate.
    let mut behind = Vec::new();
    let mut ahead = Vec::new();
    for case in &report.cases {
        let (hapi_status, _) = hapi
            .get(&case.address)
            .unwrap_or_else(|| panic!("{} has no HAPI status", case.address));
        match (hapi_status, case.status) {
            (HapiStatus::Passed, s) if s != CaseStatus::Passed => behind.push(format!(
                "{} ({:?}): HAPI passed, Frevo {} -- {}",
                case.address,
                case.expression,
                s.as_str(),
                case.reason
            )),
            (HapiStatus::Failed | HapiStatus::HapiCannotPass, CaseStatus::Passed) => {
                ahead.push(format!(
                    "{}: ahead-of-hapi (HAPI {})",
                    case.address,
                    hapi_status.as_str()
                ))
            }
            _ => {}
        }
    }

    // A case HAPI passes that Frevo fails is listed individually, regardless of
    // the aggregate -- so this assertion comes first and is not subsumed by the
    // count.
    assert!(
        behind.is_empty(),
        "{} case(s) HAPI passes and Frevo does not; the first few:\n  {}",
        behind.len(),
        behind[..behind.len().min(10)].join("\n  ")
    );

    // The aggregate condition.
    assert!(
        frevo_passed >= hapi_passed,
        "Frevo passed {frevo_passed} of the suite, HAPI passed {hapi_passed}"
    );

    // A surplus is reported under its own status, so the aggregate cannot be
    // read as parity when it is in fact a surplus. The list is printed rather
    // than asserted empty: being ahead is allowed.
    if !ahead.is_empty() {
        println!(
            "ahead-of-hapi on {} case(s):\n  {}",
            ahead.len(),
            ahead[..ahead.len().min(10)].join("\n  ")
        );
    }

    // And `did-not-run` is never counted towards the gate, which is the
    // property that makes the comparison meaningful at all.
    assert_eq!(
        report.count(CaseStatus::DidNotRun),
        0,
        "a run with did-not-run cases cannot be compared with HAPI's"
    );
}

// ---------------------------------------------------------------------------
// path.oracle.013 -- the same cases through helios-fhirpath
// ---------------------------------------------------------------------------

/// Where the differential reference lives. Outside the workspace on purpose
/// (`design.md` section 9): it is the only place `haste-fhirpath`'s wildcard
/// dependencies can be resolved without entering the product graph.
const COMPARE_DIR: &str = "harness/fhirpath-compare";

/// The three-way comparison the reference produces.
const COMPARE_RESULTS: &str = "harness/fhirpath-compare/results.json";

// covers: path.oracle.013
#[test]
fn the_harness_can_run_the_same_cases_through_helios_fhirpath() {
    // Task 2 builds this. Until it does, the differential reference does not
    // exist -- which is this test's reason to fail.
    let dir = input::repo_path(COMPARE_DIR);
    assert!(
        dir.is_dir(),
        "{COMPARE_DIR} does not exist. Task 2 creates it, listed in \
         [workspace] exclude, and runs all 1,191 cases through \
         helios-fhirpath 0.2.7 and haste-fhirpath 0.129.3."
    );

    // It names helios-fhirpath 0.2.7 specifically, because the measurement ADR
    // 0004 rests on is version-pinned.
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml"))
        .expect("the comparison harness has a manifest");
    assert!(
        manifest.contains("helios-fhirpath"),
        "the comparison harness must depend on helios-fhirpath:\n{manifest}"
    );
    assert!(
        manifest.contains("0.2.7"),
        "pinned at 0.2.7, which is the version ADR 0004 measures:\n{manifest}"
    );

    // The report is a three-way comparison: the same case addresses, with an
    // outcome from each engine.
    let text = std::fs::read_to_string(input::repo_path(COMPARE_RESULTS))
        .unwrap_or_else(|e| panic!("{COMPARE_RESULTS} must record the comparison: {e}"));
    let doc = json::parse(&text).unwrap_or_else(|e| panic!("{COMPARE_RESULTS} is not JSON: {e}"));
    let cases = match doc.get("cases") {
        Some(Value::Array(items)) => items,
        other => panic!("{COMPARE_RESULTS} must carry a \"cases\" array, got {other:?}"),
    };
    let (suite, _) = pinned();
    assert_eq!(
        cases.len(),
        suite.case_count(),
        "the comparison must cover all {} cases",
        suite.case_count()
    );
    for entry in cases {
        for key in ["address", "frevo", "helios", "haste"] {
            assert!(
                entry.get(key).is_some(),
                "every entry needs a {key:?} member: {entry:?}"
            );
        }
    }

    // And it appears in no shipped crate's dependency graph: the workspace
    // excludes the directory, so `cargo metadata` over the workspace can never
    // see it.
    let root = std::fs::read_to_string(input::repo_path("Cargo.toml"))
        .expect("the workspace manifest is readable");
    assert!(
        root.contains("exclude"),
        "the workspace must exclude the comparison harness:\n{root}"
    );
    assert!(
        root.contains(COMPARE_DIR),
        "and name it in the exclude list:\n{root}"
    );
    for crate_dir in shipped_crate_manifests() {
        let text = std::fs::read_to_string(&crate_dir).expect("a manifest is readable");
        assert!(
            !text.contains("helios-fhirpath") && !text.contains("haste-fhirpath"),
            "{} must not depend on a reference engine",
            crate_dir.display()
        );
    }
}

/// Every shipped crate's manifest.
fn shipped_crate_manifests() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(input::repo_path("crates")).expect("crates/ is readable") {
        let path = entry.expect("a directory entry is readable").path();
        let manifest = path.join("Cargo.toml");
        if manifest.is_file() {
            out.push(manifest);
        }
    }
    assert!(!out.is_empty(), "the workspace has crates");
    out
}

// ---------------------------------------------------------------------------
// path.oracle.014 -- documentFunctions is out of scope, not passing
// ---------------------------------------------------------------------------

// covers: path.oracle.014
#[test]
fn the_document_functions_group_is_reported_as_out_of_scope_not_as_passing() {
    let (suite, report) = pinned();
    let group = suite
        .groups
        .iter()
        .find(|g| g.name == "documentFunctions")
        .expect("the pinned suite has a documentFunctions group");
    assert_eq!(group.cases.len(), 115);
    assert_eq!(group.mode.as_deref(), Some("custom"));

    // 111 of the 115 are reported `failed` with the out-of-scope reason. The
    // other four are `invalid="syntax"` and therefore **pass**: an unknown
    // function is a parse-time error, which is exactly what those four expect.
    let syntax_invalid: Vec<&str> = group
        .cases
        .iter()
        .filter(|c| c.expected_failure == Some(oracle::suite::Stage::Parse))
        .map(|c| c.address.name.as_str())
        .collect();
    assert_eq!(
        syntax_invalid,
        [
            "testSection13",
            "testEntry12",
            "testSameSubject16",
            "testHasCoding10"
        ],
        "four cases in the group expect a syntax failure"
    );

    let mut failed = 0usize;
    let mut passed = 0usize;
    for case in &group.cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        if syntax_invalid.contains(&case.address.name.as_str()) {
            assert_eq!(
                r.status,
                CaseStatus::Passed,
                "{} expects a syntax failure and so passes: {}",
                r.address,
                r.reason
            );
            passed += 1;
        } else {
            assert_eq!(
                r.status,
                CaseStatus::Failed,
                "{} must be reported failed, not {}: {}",
                r.address,
                r.status.as_str(),
                r.reason
            );
            // And the reason names the function, so "out of scope" is reported
            // rather than inferred.
            let f = case
                .out_of_scope_function()
                .unwrap_or_else(|| panic!("{} must name the function it tests", case.address));
            assert!(
                [
                    "section",
                    "entry",
                    "entryIn",
                    "resources",
                    "resourcesIn",
                    "sameSubject",
                    "hasCoding"
                ]
                .contains(&f),
                "{} tests {f}, which is not one of the seven",
                case.address
            );
            failed += 1;
        }
    }
    assert_eq!((failed, passed), (111, 4));

    // Never `did-not-run`: out of scope is a graded answer.
    assert_eq!(
        group
            .cases
            .iter()
            .filter(|c| report
                .get(&c.address.to_string())
                .is_some_and(|r| r.status == CaseStatus::DidNotRun))
            .count(),
        0
    );

    // HAPI's recorded status for the same 115 is the same split, so the
    // aggregate gate is unaffected.
    let hapi = hapi_results();
    for case in &group.cases {
        let (status, _) = hapi
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} has no HAPI status", case.address));
        let expected = if syntax_invalid.contains(&case.address.name.as_str()) {
            HapiStatus::Passed
        } else {
            HapiStatus::Failed
        };
        assert_eq!(
            *status,
            expected,
            "{}: HAPI must be recorded {} for this case",
            case.address,
            expected.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// path.oracle.016 -- a case HAPI structurally cannot pass
// ---------------------------------------------------------------------------

// covers: path.oracle.016
#[test]
fn a_case_hapi_structurally_cannot_pass_is_recorded_as_such() {
    let (suite, report) = pinned();
    let hapi = hapi_results();

    let resolve = |name: &str| {
        suite
            .case("isSemanticChecks", name)
            .unwrap_or_else(|| panic!("isSemanticChecks/{name} must resolve to one case"))
    };
    // What a case expects: its `invalid` stage and its outputs.
    let expected_outcome = |case: &oracle::suite::Case| {
        (
            case.expected_failure,
            case.outputs
                .iter()
                .map(|o| (o.type_name.clone(), o.body.clone()))
                .collect::<Vec<_>>(),
        )
    };
    let hapi_status = |case: &oracle::suite::Case| {
        hapi.get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} has no HAPI status", case.address))
            .0
    };

    // The rule selects exactly six groups and 13 cases.
    let members: Vec<&str> = HAPI_CANNOT_PASS_GROUPS
        .iter()
        .flat_map(|g| g.iter().copied())
        .collect();
    assert_eq!(members.len(), 13, "six groups, 13 cases");
    let mut distinct = members.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), 13, "no case is in two groups");

    for group in HAPI_CANNOT_PASS_GROUPS {
        let cases: Vec<_> = group.iter().map(|n| resolve(n)).collect();
        let first = cases[0];

        for case in &cases[1..] {
            // The premise: the expressions are byte-identical, on the same
            // input...
            assert_eq!(
                case.expression, first.expression,
                "{group:?} must be byte-identical expressions"
            );
            assert_eq!(
                case.inputfile, first.inputfile,
                "{group:?} share an inputfile"
            );
            // ...and they differ only by `context` and `contextType`.
            assert_eq!(case.context, first.context, "{group:?}");
        }
        for (i, a) in cases.iter().enumerate() {
            for b in &cases[i + 1..] {
                assert_ne!(
                    a.context_types, b.context_types,
                    "{} and {} must differ by contextType",
                    a.address.name, b.address.name
                );
            }
        }
        // ...and their expected outcomes differ, which is what forces HAPI's
        // context-blind harness to fail at least one of them.
        let outcomes: Vec<_> = cases.iter().map(|c| expected_outcome(c)).collect();
        assert!(
            outcomes.iter().any(|o| *o != outcomes[0]),
            "{group:?} must have differing expected outcomes: {outcomes:?}"
        );

        // So every member is recorded `hapi-cannot-pass`, a status distinct
        // from `failed` -- and none of them as a plain failure.
        let statuses: Vec<HapiStatus> = cases.iter().map(|c| hapi_status(c)).collect();
        assert!(
            statuses.iter().all(|s| *s == HapiStatus::HapiCannotPass),
            "every member of {group:?} must be recorded hapi-cannot-pass, not {:?}",
            statuses.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        );
        assert!(
            statuses.iter().all(|s| *s != HapiStatus::Failed),
            "no member of {group:?} may be recorded as a plain failure"
        );
    }

    // Frevo's status comes from its own context-aware run, which honours all
    // four attributes. It is **not** `passed` for every member: 3 end at the
    // check stage `failed` because the check pass does not narrow, 6 are
    // semantic-invalid and graded `passed` at the check stage, and 4 pass the
    // check and are graded by evaluation.
    assert_eq!(
        {
            let mut names: Vec<&str> = FREVO_HAPI_CANNOT_PASS_STATUS
                .iter()
                .map(|(n, _)| *n)
                .collect();
            names.sort_unstable();
            names
        },
        distinct,
        "the breakdown covers exactly the 13"
    );
    for (name, expected) in FREVO_HAPI_CANNOT_PASS_STATUS {
        let case = resolve(name);
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        match expected {
            Some(status) => assert_eq!(
                (r.status, r.failed_stage),
                (status, Some(oracle::suite::Stage::Check)),
                "{}: Frevo must end at the check stage graded {} -- {}",
                r.address,
                status.as_str(),
                r.reason
            ),
            None => {
                // Graded by evaluation: no status is asserted until evaluation
                // exists, but the case must pass the check and must not be a
                // plain failure.
                assert!(
                    !matches!(
                        r.failed_stage,
                        Some(oracle::suite::Stage::Parse | oracle::suite::Stage::Check)
                    ),
                    "{}: must pass the check and reach evaluation, stopped at {:?} -- {}",
                    r.address,
                    r.failed_stage.map(|s| s.as_str()),
                    r.reason
                );
                assert_ne!(
                    r.status,
                    CaseStatus::Failed,
                    "{}: must not be recorded as a plain failure -- {}",
                    r.address,
                    r.reason
                );
            }
        }
    }

    // The two groups that share an expression and differ only by
    // `context`/`contextType` but expect the same outcome are excluded: HAPI is
    // not forced to fail any of them, so none is `hapi-cannot-pass`.
    for [a, b] in SAME_OUTCOME_GROUPS {
        let (a, b) = (resolve(a), resolve(b));
        assert_eq!(a.expression, b.expression);
        assert_eq!(a.inputfile, b.inputfile);
        assert_ne!(a.context_types, b.context_types);
        assert_eq!(
            expected_outcome(a),
            expected_outcome(b),
            "{} and {} expect the same outcome",
            a.address.name,
            b.address.name
        );
        for case in [a, b] {
            assert_ne!(
                hapi_status(case),
                HapiStatus::HapiCannotPass,
                "{} expects the same outcome as its twin and is not hapi-cannot-pass",
                case.address
            );
        }
    }

    // `path.check.002` is graded against that same context-aware run, so
    // requiring the semantic-invalid cases to fail the check is consistent with
    // the aggregate gate rather than stricter than it: every one of them is
    // `passed` for Frevo.
    let semantic_passed = suite
        .cases()
        .filter(|c| c.expected_failure == Some(oracle::suite::Stage::Check))
        .filter(|c| {
            report
                .get(&c.address.to_string())
                .is_some_and(|r| r.status == CaseStatus::Passed)
        })
        .count();
    assert_eq!(semantic_passed, 29);
}
