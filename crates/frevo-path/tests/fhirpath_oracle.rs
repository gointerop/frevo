//! The FHIRPath oracle suite, and the tests that assert the harness grades it
//! the way `org.hl7.fhir.r4.test.FHIRPathTests` does.
//!
//! Run it with:
//!
//! ```text
//! cargo nextest run -p frevo-path --test fhirpath_oracle
//! ```
//!
//! Every case that names an `inputfile` needs the suite's input files, which
//! are fetched and not vendored. Without `bash oracles/fetch.sh` those 1,101
//! cases report `did-not-run`, which is not a pass.
//!
//! Scenario coverage: `path.oracle.001`-`010`, `015`, `017` and
//! `path.datetime.004`. `path.oracle.011`-`014` and `016` are task 3 — HAPI's
//! own measured rate and the gate — and are not asserted here.

mod oracle;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, ParsedHandle, SUITE_TIME_ZONE_OFFSET_MINUTES,
    StrictnessFlags, SuiteEngine,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus};
use oracle::suite::{self, Output, Stage};
use oracle::value::{self, QuantityValue, ResultItem};

/// Address a case by `group/name`, failing loudly on an ambiguous address.
fn address_of(s: &suite::Suite, group: &str, name: &str) -> String {
    s.case(group, name)
        .unwrap_or_else(|| {
            panic!("{group}/{name} must resolve to exactly one case in the pinned suite")
        })
        .address
        .to_string()
}

fn expect_passed(report: &run::RunReport, address: &str, why: &str) {
    let r = report
        .get(address)
        .unwrap_or_else(|| panic!("{address} must appear in the run report"));
    assert_eq!(
        r.status,
        CaseStatus::Passed,
        "{address} must pass because {why}; it is {} ({})",
        r.status.as_str(),
        r.reason
    );
}

// ---------------------------------------------------------------------------
// path.oracle.001 -- the suite is read by XML parse
// ---------------------------------------------------------------------------

// covers: path.oracle.001
#[test]
fn harness_parses_every_case_in_the_pinned_suite() {
    let s = oracle::load_pinned_suite();

    assert_eq!(
        s.groups.len(),
        suite::EXPECTED_GROUPS,
        "the pinned suite must have {} groups; a different count means it was re-pinned \
         without the harness being updated",
        suite::EXPECTED_GROUPS
    );
    assert_eq!(
        s.case_count(),
        suite::EXPECTED_CASES,
        "the pinned suite must have {} cases",
        suite::EXPECTED_CASES
    );
    assert_eq!(s.output_count(), suite::EXPECTED_OUTPUTS);
    assert_eq!(
        s.invalid_count(Stage::Parse),
        suite::EXPECTED_SYNTAX_INVALID
    );
    assert_eq!(
        s.invalid_count(Stage::Check),
        suite::EXPECTED_SEMANTIC_INVALID
    );
    assert_eq!(
        s.invalid_count(Stage::Evaluate),
        suite::EXPECTED_EXECUTION_INVALID
    );

    // The commented-out block at tests-fhir-r4.xml:1576-1582 contributes
    // nothing: a text match counts it, an XML parse does not.
    let text = std::fs::read_to_string(input::repo_path(suite::SUITE_PATH)).unwrap();
    assert_eq!(
        text.matches("<group ").count(),
        suite::EXPECTED_GROUPS + 1,
        "a text match over the pinned file over-counts groups by one; if it no longer \
         does, the commented-out testDollarResource block has changed"
    );
    assert_eq!(
        text.matches("<test ").count(),
        suite::EXPECTED_CASES + 2,
        "a text match over the pinned file over-counts cases by two"
    );
    assert!(
        !s.groups.iter().any(|g| g.name == "testDollarResource"),
        "the commented-out testDollarResource group must not appear as a group"
    );

    // The `documentFunctions` group is 115 cases, not 114.
    let doc = s
        .groups
        .iter()
        .find(|g| g.name == "documentFunctions")
        .expect("the documentFunctions group exists");
    assert_eq!(doc.cases.len(), 115);
    assert_eq!(doc.mode.as_deref(), Some("custom"));
}

// covers: path.oracle.001
#[test]
fn a_case_count_that_differs_from_the_pin_is_not_the_pinned_suite() {
    // A re-pin that changes the counts must fail the run rather than grade
    // fewer cases than the harness reports. The constants are what makes that
    // a failure, so a one-case suite must not satisfy them.
    let shrunk = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="t"><expression>1</expression><output>1</output></test>"#,
        r#"</group></tests>"#
    );
    let s = suite::load(shrunk).expect("the synthetic suite is well-formed");
    assert_eq!(s.groups.len(), 1);
    assert_eq!(s.case_count(), 1);
    assert_ne!(s.groups.len(), suite::EXPECTED_GROUPS);
    assert_ne!(s.case_count(), suite::EXPECTED_CASES);
}

// ---------------------------------------------------------------------------
// path.oracle.002 -- every attribute the case carries is read
// ---------------------------------------------------------------------------

// covers: path.oracle.002
#[test]
fn harness_reads_every_attribute_the_pinned_suite_carries() {
    let s = oracle::load_pinned_suite();

    // The load succeeds only when every attribute was inside its element's
    // closed set, so a successful load *is* the zero-unhandled-attributes
    // assertion. Spelling the inventory out makes the claim auditable.
    let seen: Vec<&str> = s
        .attribute_inventory
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();

    let mut expected: Vec<String> = Vec::new();
    for (el, attrs) in [
        ("tests", suite::TESTS_ATTRS),
        ("group", suite::GROUP_ATTRS),
        ("test", suite::TEST_ATTRS),
        ("expression", suite::EXPRESSION_ATTRS),
        ("output", suite::OUTPUT_ATTRS),
        ("notes", suite::NOTES_ATTRS),
    ] {
        for a in attrs {
            expected.push(format!("{el}/@{a}"));
        }
    }
    expected.sort();

    assert_eq!(
        seen, expected,
        "the attributes the pinned file carries and the attributes the harness declares \
         must be the same set: nothing unread, nothing declared that is absent"
    );

    let count = |key: &str| {
        s.attribute_inventory
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    };

    // The three a thirteen-attribute inventory misses, and that would abort a
    // run which did not declare them.
    assert_eq!(count("tests/@reference"), 1);
    assert_eq!(
        s.reference.as_deref(),
        Some("http://hl7.org/fhirpath|2.0.0")
    );
    assert_eq!(count("group/@reference"), 1);
    let zulip = s
        .groups
        .iter()
        .find(|g| g.name == "from-Zulip")
        .expect("the from-Zulip group is at tests-fhir-r4.xml:1583");
    assert_eq!(
        zulip.reference.as_deref(),
        Some("http://hl7.org/fhirpath/#singleton-evaluation-of-collections")
    );
    assert_eq!(count("expression/@mode"), 1);

    // The remaining per-element counts, so a re-pin that moves an attribute
    // from one element to another is visible rather than silently absorbed.
    assert_eq!(count("tests/@name"), 1);
    assert_eq!(count("tests/@description"), 1);
    assert_eq!(count("group/@name"), 103);
    assert_eq!(count("group/@description"), 4);
    assert_eq!(count("group/@mode"), 2);
    assert_eq!(count("test/@name"), 1191);
    assert_eq!(count("test/@description"), 106);
    assert_eq!(count("test/@inputfile"), 1101);
    assert_eq!(count("test/@mode"), 11);
    assert_eq!(count("test/@context"), 38);
    assert_eq!(count("test/@contextType"), 35);
    assert_eq!(count("test/@version"), 31);
    assert_eq!(count("test/@testing"), 153);
    assert_eq!(count("test/@predicate"), 2);
    assert_eq!(count("test/@ordered"), 1);
    assert_eq!(count("test/@checkOrderedFunctions"), 1);
    assert_eq!(count("expression/@invalid"), 63);
    assert_eq!(count("output/@type"), 1032);

    // `name` and `description` are read, not merely counted.
    assert_eq!(s.name, "FHIRPathTestSuite");
    assert_eq!(s.description.as_deref(), Some("FHIRPath Test Suite"));
    let described: Vec<&str> = s
        .groups
        .iter()
        .filter_map(|g| g.description.as_deref().map(|_| g.name.as_str()))
        .collect();
    assert_eq!(
        described,
        [
            "comments",
            "testMiscellaneousAccessorTests",
            "testBasics",
            "documentFunctions"
        ]
    );
    let birth_date = s
        .case("testMiscellaneousAccessorTests", "testExtractBirthDate")
        .expect("addressable");
    assert_eq!(birth_date.description, "Extract birthDate");
    assert_eq!(
        s.cases().filter(|c| !c.description.is_empty()).count(),
        106,
        "every test/@description must be read"
    );

    // The group's mode reaches every case in it, and each case records where
    // in the file it came from, so a report line can be traced to a line.
    let semantic = s
        .case("isSemanticChecks", "isGuardOrBad")
        .expect("addressable");
    assert_eq!(semantic.group_mode.as_deref(), Some("semantic"));
    assert_eq!(birth_date.group_mode, None);
    let offsets: Vec<usize> = s.cases().map(|c| c.offset).collect();
    assert_eq!(offsets.len(), suite::EXPECTED_CASES);
    assert!(
        offsets.windows(2).all(|w| w[0] < w[1]),
        "cases must be recorded in document order, with a distinct offset each"
    );

    // `notes` is the sixth element the file carries. It has no attributes, so
    // it contributes nothing to the inventory, and it must not abort the load.
    assert!(suite::NOTES_ATTRS.is_empty());
    let notes: usize = s.groups.iter().map(|g| g.notes.len()).sum();
    assert_eq!(notes, 2, "two <notes> elements, at :1584 and :1819");
}

// covers: path.oracle.002
#[test]
fn an_attribute_an_element_does_not_carry_fails_the_run() {
    // `inputfile` is a <test> attribute. On a <group> it is unhandled, and an
    // unhandled attribute is a silently weakened oracle, so the load fails
    // rather than ignoring it.
    let src = concat!(
        r#"<tests name="x"><group name="g" inputfile="patient-example.xml">"#,
        r#"<test name="t"><expression>1</expression></test></group></tests>"#
    );
    let err = suite::load(src).expect_err("an unhandled attribute must fail the load");
    assert!(
        err.message.contains("inputfile"),
        "the failure must name the attribute: {}",
        err.message
    );

    // The same for an attribute nothing in the pinned file carries at all.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="t" sparkle="yes"><expression>1</expression></test></group></tests>"#
    );
    let err = suite::load(src).expect_err("an unknown attribute must fail the load");
    assert!(err.message.contains("sparkle"), "{}", err.message);

    // And an element the harness does not know about. `modeTest` is the one
    // `FHIRPathTests.data()` also collects; the pinned R4 file carries none,
    // so one appearing is a re-pin the harness has not been taught about.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<modeTest name="t"><expression>1</expression></modeTest></group></tests>"#
    );
    let err = suite::load(src).expect_err("an unknown element must fail the load");
    assert!(err.message.contains("modeTest"), "{}", err.message);
}

// covers: path.oracle.002
#[test]
fn an_attribute_value_outside_its_recorded_domain_fails_the_run() {
    let cases: &[(&str, &str)] = &[
        (
            r#"<tests name="x"><group name="g"><test name="t"><expression invalid="maybe">1</expression></test></group></tests>"#,
            "invalid",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t"><expression mode="lenient">1</expression></test></group></tests>"#,
            "mode",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t" mode="whatever"><expression>1</expression></test></group></tests>"#,
            "mode",
        ),
        (
            r#"<tests name="x"><group name="g" mode="whatever"><test name="t"><expression>1</expression></test></group></tests>"#,
            "mode",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t" predicate="maybe"><expression>1</expression></test></group></tests>"#,
            "predicate",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t" ordered="true"><expression>1</expression></test></group></tests>"#,
            "ordered",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t" checkOrderedFunctions="false"><expression>1</expression></test></group></tests>"#,
            "checkOrderedFunctions",
        ),
        (
            r#"<tests name="x"><group name="g"><test name="t" version="3.0.0"><expression>1</expression></test></group></tests>"#,
            "version",
        ),
    ];
    for (src, attr) in cases {
        match suite::load(src) {
            Ok(_) => panic!("{attr} outside its recorded domain must fail the load"),
            Err(e) => assert!(
                e.message.contains(attr),
                "the failure must name {attr}: {}",
                e.message
            ),
        }
    }

    // The domains themselves are the ones the pinned file uses.
    assert_eq!(
        suite::EXPRESSION_INVALID_DOMAIN,
        ["syntax", "semantic", "execution"]
    );
    assert_eq!(suite::EXPRESSION_MODE_DOMAIN, ["strict"]);
    assert_eq!(suite::TEST_MODE_DOMAIN, ["element", "strict"]);
    assert_eq!(suite::GROUP_MODE_DOMAIN, ["custom", "semantic"]);
    assert_eq!(suite::TEST_PREDICATE_DOMAIN, ["true", "false"]);
    assert_eq!(suite::TEST_ORDERED_DOMAIN, ["false"]);
    assert_eq!(suite::TEST_CHECK_ORDERED_FUNCTIONS_DOMAIN, ["true"]);
    assert_eq!(suite::TEST_VERSION_DOMAIN, ["2.1.0"]);
}

// covers: path.oracle.002
#[test]
fn the_attributes_with_no_stated_domain_are_recorded_and_reported() {
    // `test/@testing`, `@context` and `@contextType` carry no domain in the
    // spec delta, so the harness records them rather than aborting a load on a
    // value the delta never fixed. Their observed sets are asserted here
    // instead, so a re-pin that adds one is still visible.
    let s = oracle::load_pinned_suite();

    let mut testing: Vec<&str> = s.cases().filter_map(|c| c.testing.as_deref()).collect();
    testing.sort_unstable();
    testing.dedup();
    assert_eq!(testing, suite::OBSERVED_TESTING_VALUES);

    let mut contexts: Vec<&str> = s.cases().filter_map(|c| c.context.as_deref()).collect();
    contexts.sort_unstable();
    contexts.dedup();
    assert_eq!(contexts, ["Observation.effective"]);

    let mut ctypes: Vec<String> = s
        .cases()
        .filter(|c| !c.context_types.is_empty())
        .map(|c| c.context_types.join("|"))
        .collect();
    ctypes.sort();
    ctypes.dedup();
    assert_eq!(
        ctypes,
        ["Period", "Period|Timing", "Period|dateTime", "dateTime"]
    );

    // A multi-type contextType is split, not carried as one opaque name, so
    // the check pass can run against all of them (`path.check.012`).
    let both = s
        .case("isSemanticChecks", "isGuardOrBad")
        .expect("isGuardOrBad is addressable");
    assert_eq!(both.context_types, ["Period", "Timing"]);

    // The `testing` values that mark a case out of scope are exactly the seven
    // document functions, and `is` is not one of them.
    let out_of_scope = s
        .cases()
        .filter(|c| c.out_of_scope_function().is_some())
        .count();
    assert_eq!(out_of_scope, 115);
    assert_eq!(suite::UNIMPLEMENTED_DOCUMENT_FUNCTIONS.len(), 7);
    assert!(!suite::UNIMPLEMENTED_DOCUMENT_FUNCTIONS.contains(&"is"));
}

// covers: path.oracle.002
#[test]
fn a_case_address_is_unique_even_where_group_and_name_are_not() {
    let s = oracle::load_pinned_suite();

    // `group/name` is not unique in the pinned file: testEquivalent23 appears
    // twice, at tests-fhir-r4.xml:986 and :987, with two different
    // expressions. The ordinal is part of the address, as it is in
    // FHIRPathTests.getName.
    let dupes: Vec<&suite::Case> = s
        .cases()
        .filter(|c| c.address.group == "testEquivalent" && c.address.name == "testEquivalent23")
        .collect();
    assert_eq!(dupes.len(), 2);
    assert_ne!(dupes[0].expression, dupes[1].expression);
    assert_ne!(dupes[0].address, dupes[1].address);

    // An ambiguous `group/name` must not silently resolve to the first match.
    assert!(s.case("testEquivalent", "testEquivalent23").is_none());

    let mut all: Vec<String> = s.cases().map(|c| c.address.to_string()).collect();
    let total = all.len();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), total, "every case address must be unique");
}

// ---------------------------------------------------------------------------
// path.oracle.017 -- the expression-level mode changes no outcome
// ---------------------------------------------------------------------------

// covers: path.oracle.017
#[test]
fn expression_level_mode_is_recorded_and_changes_no_grading() {
    let (s, report) = run::run_pinned_suite();

    // Exactly one expression-level mode in the pinned file, on testIif6.
    let with_mode: Vec<&str> = s
        .cases()
        .filter(|c| c.expression_mode.is_some())
        .map(|c| c.address.name.as_str())
        .collect();
    assert_eq!(with_mode, ["testIif6"]);

    let case = s
        .case("testIif", "testIif6")
        .expect("testIif6 is addressable");
    assert_eq!(case.expression_mode.as_deref(), Some("strict"));
    assert_eq!(case.expected_failure, Some(Stage::Check));
    assert_eq!(
        case.expression,
        "iif('non boolean criteria', 'true-result', 'true-result')"
    );

    let address = case.address.to_string();
    let r = report.get(&address).expect("testIif6 is in the report");
    assert_eq!(
        r.expression_mode.as_deref(),
        Some("strict"),
        "the harness must record that it read mode=\"strict\" on the expression"
    );

    // The engine types iif's first parameter against a singleton boolean with
    // no mode gate -- the `case Iif` arm of executeType calls checkParamTypes
    // unconditionally -- so the case is a semantic failure in every
    // configuration the engine offers. The harness must therefore not
    // implement the attribute as "fail the check only in strict mode", because
    // that would let an engine which accepts a non-boolean iif condition in
    // the default configuration pass this case.
    expect_passed(
        &report,
        &address,
        "it must fail the check pass whatever mode says",
    );
    assert_eq!(
        report.get(&address).unwrap().failed_stage,
        Some(Stage::Check)
    );
}

// ---------------------------------------------------------------------------
// path.oracle.003 / 004 -- the suite's input files
// ---------------------------------------------------------------------------

// covers: path.oracle.003
#[test]
fn harness_loads_every_referenced_input_file() {
    let s = oracle::load_pinned_suite();
    let referenced = s.referenced_inputs();
    assert_eq!(
        referenced.len(),
        15,
        "the pinned suite references 15 distinct input files, got {referenced:?}"
    );

    for file in &referenced {
        let loaded = input::load(file).unwrap_or_else(|e| {
            panic!("inputfile {file} must load from {}: {e}", suite::INPUT_DIR)
        });
        assert!(!loaded.bytes.is_empty());
        assert!(
            loaded.resource_type().is_some(),
            "{file} must expose a resource type"
        );
    }

    // The run report records each input's disposition.
    let (_, report) = run::run_pinned_suite();
    assert_eq!(report.inputs.len(), 15);
    let failed: Vec<&String> = report
        .inputs
        .iter()
        .filter(|(_, v)| v.is_err())
        .map(|(k, _)| k)
        .collect();
    assert!(failed.is_empty(), "these inputs did not load: {failed:?}");

    // A missing input file is `did-not-run`, never `passed`.
    let err =
        input::load("there-is-no-such-file.xml").expect_err("a missing input file is an error");
    assert!(format!("{err}").contains("there-is-no-such-file.xml"));

    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="t" inputfile="there-is-no-such-file.xml">"#,
        r#"<expression>1</expression><output type="integer">1</output></test>"#,
        r#"</group></tests>"#
    );
    let synthetic = suite::load(src).unwrap();
    let mut stub = StubEngine::failing_at(None);
    let r = run::run(&synthetic, &mut stub, EngineConfig::default());
    assert_eq!(r.cases[0].status, CaseStatus::DidNotRun);
    assert!(!r.cases[0].status.is_pass());
}

// covers: path.oracle.004
#[test]
fn xml_and_json_inputs_both_become_navigable_nodes() {
    let xml = input::load("patient-example.xml").expect("patient-example.xml loads");
    let json =
        input::load("appointment-examplereq.json").expect("appointment-examplereq.json loads");
    assert!(xml.is_xml());
    assert!(!json.is_xml());
    assert_eq!(xml.resource_type(), Some("Patient"));
    assert_eq!(json.resource_type(), Some("Appointment"));

    // The same expression must evaluate identically over both forms. Each
    // input is a resource with a logical id, so `id.exists()` is `true` for
    // both, and the XML and JSON paths cannot disagree about navigability
    // without this failing.
    let mut engine = oracle::engine::engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .expect("the engine under test must configure");
    let parsed = engine.parse("id.exists()").expect("id.exists() parses");

    let mut results = Vec::new();
    for inp in [&xml, &json] {
        let rt = inp.resource_type();
        let ctx = CaseContext {
            root_resource_type: rt,
            resource_type: rt,
            ..CaseContext::default()
        };
        engine
            .check(parsed, &ctx, StrictnessFlags::CHECK)
            .expect("id.exists() checks");
        results.push(
            engine
                .evaluate(parsed, Some(inp), &ctx, StrictnessFlags::EVALUATE)
                .expect("id.exists() evaluates"),
        );
    }

    assert_eq!(
        results[0], results[1],
        "the same expression must give the same answer over the XML and the JSON input"
    );
    assert_eq!(
        results[0],
        vec![ResultItem::Primitive {
            fhir_type: "boolean".to_string(),
            value: "true".to_string()
        }]
    );
}

// ---------------------------------------------------------------------------
// path.oracle.005 -- three stages, attributed
// ---------------------------------------------------------------------------

// covers: path.oracle.005
#[test]
fn the_three_stages_run_in_order_and_failures_are_attributed() {
    // First the attribution rule itself, with a stub engine that fails at a
    // chosen stage: a failure at the wrong stage must not satisfy an
    // expectation for another stage.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="syn"><expression invalid="syntax">(</expression></test>"#,
        r#"<test name="sem"><expression invalid="semantic">Patient.nope</expression></test>"#,
        r#"<test name="exe"><expression invalid="execution">1 + 'a'</expression></test>"#,
        r#"<test name="ok"><expression>1</expression><output type="integer">1</output></test>"#,
        r#"</group></tests>"#
    );
    let synthetic = suite::load(src).unwrap();
    let stages = ["parse", "check", "evaluate"];

    for (fail_at, expected, reached) in [
        (Stage::Parse, ["passed", "failed", "failed", "failed"], 1),
        (Stage::Check, ["failed", "passed", "failed", "failed"], 2),
        (Stage::Evaluate, ["failed", "failed", "passed", "failed"], 3),
    ] {
        let mut stub = StubEngine::failing_at(Some(fail_at));
        let report = run::run(&synthetic, &mut stub, EngineConfig::default());
        let got: Vec<&str> = report.cases.iter().map(|c| c.status.as_str()).collect();
        assert_eq!(
            got, expected,
            "with the stub failing at {fail_at:?} the statuses must be {expected:?}"
        );
        assert_eq!(
            stub.order,
            stages[..reached].to_vec(),
            "the stages must run in order and stop at the failure"
        );
        for c in report.cases.iter() {
            // A case that reached the failing stage is attributed to it. The
            // cases that returned early because an expected failure did not
            // happen carry no stage, which is the other half of the rule: a
            // stage that succeeded is never reported as the failure.
            assert!(
                c.failed_stage == Some(fail_at) || c.failed_stage.is_none(),
                "{} is attributed to {:?}, but only {fail_at:?} failed",
                c.address,
                c.failed_stage
            );
            if c.status == CaseStatus::Passed {
                assert_eq!(
                    c.failed_stage,
                    Some(fail_at),
                    "{} passed, so its expected failure came from {fail_at:?}",
                    c.address
                );
            }
        }
    }

    // Then the real suite: each of the three expectations must be satisfied by
    // the matching stage of the engine under test, which is what lets the
    // report distinguish a check failure from an evaluation failure.
    let (s, report) = run::run_pinned_suite();
    for (group, name, stage) in [
        ("testLiterals", "testLiteralUnicodeInvalid", Stage::Parse),
        ("testIif", "testIif6", Stage::Check),
        ("from-Zulip", "from-zulip-2", Stage::Evaluate),
    ] {
        let case = s.case(group, name).expect("the probe case exists");
        assert_eq!(case.expected_failure, Some(stage));
        let address = case.address.to_string();
        expect_passed(
            &report,
            &address,
            "the failure must come from the matching stage",
        );
        assert_eq!(
            report.get(&address).unwrap().failed_stage,
            Some(stage),
            "{address} must be attributed to the {} stage",
            stage.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// path.oracle.006 -- predicate="true"
// ---------------------------------------------------------------------------

// covers: path.oracle.006
#[test]
fn predicate_true_converts_the_outcome_to_a_single_boolean() {
    let (s, report) = run::run_pinned_suite();

    // The pinned file carries the attribute twice: "false" on one case and
    // "true" on one. Only the second selects the conversion.
    let predicated: Vec<&str> = s
        .cases()
        .filter(|c| c.predicate)
        .map(|c| c.address.name.as_str())
        .collect();
    assert_eq!(predicated, ["testPatientHasBirthDate"]);

    let case = s
        .case("testMiscellaneousAccessorTests", "testPatientHasBirthDate")
        .expect("the predicate case is addressable");
    assert_eq!(case.outputs.len(), 1);
    assert_eq!(case.outputs[0].type_name, "boolean");
    assert_eq!(case.outputs[0].body, "true");

    // The conversion really is applied, not merely parsed: a stub returning a
    // one-item string collection still compares as one boolean `true`.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="p" predicate="true"><expression>name.given</expression>"#,
        r#"<output type="boolean">true</output></test>"#,
        r#"</group></tests>"#
    );
    let synthetic = suite::load(src).unwrap();
    let mut stub = StubEngine::returning(vec![ResultItem::Primitive {
        fhir_type: "string".to_string(),
        value: "Peter".to_string(),
    }]);
    let r = run::run(&synthetic, &mut stub, EngineConfig::default());
    assert_eq!(
        r.cases[0].status,
        CaseStatus::Passed,
        "predicate=\"true\" must fold the outcome to one boolean before comparison: {}",
        r.cases[0].reason
    );

    expect_passed(
        &report,
        &case.address.to_string(),
        "the outcome is converted through convertToBoolean before comparison",
    );
}

// ---------------------------------------------------------------------------
// path.oracle.007 -- ordered="false"
// ---------------------------------------------------------------------------

// covers: path.oracle.007
#[test]
fn ordered_false_selects_an_unordered_existential_comparison() {
    let (s, report) = run::run_pinned_suite();

    let unordered: Vec<String> = s
        .cases()
        .filter(|c| c.unordered)
        .map(|c| c.address.to_string())
        .collect();
    assert_eq!(
        unordered.len(),
        1,
        "exactly one ordered=\"false\" case in the pinned file, at tests-fhir-r4.xml:321"
    );
    let case = s
        .case("testLiterals", "testExpressions")
        .expect("the one ordered=\"false\" case is testLiterals/testExpressions");
    assert!(case.unordered);
    assert_eq!(case.outputs.len(), 5);
    assert_eq!(unordered[0], case.address.to_string());

    // The unordered branch compares the *plain* string value and so carries no
    // prefix, while the ordered branch compares the FHIRPath string value,
    // which prefixes a temporal. Unifying the two renderers is latent today
    // and would be a whole-suite failure the first time an unordered case
    // yields a date, so the divergence is asserted here directly.
    let date = ResultItem::Primitive {
        fhir_type: "date".to_string(),
        value: "1974-12-25".to_string(),
    };
    assert_eq!(
        value::fhirpath_string(&date).as_deref(),
        Some("@1974-12-25")
    );
    assert_eq!(value::plain_string(&date).as_deref(), Some("1974-12-25"));

    let unprefixed = [Output {
        type_name: "date".to_string(),
        body: "1974-12-25".to_string(),
    }];
    let one = std::slice::from_ref(&date);
    assert!(oracle::compare::compare_unordered(&unprefixed, one).is_ok());
    assert!(
        oracle::compare::compare_ordered(&unprefixed, one).is_err(),
        "the ordered branch must not accept an unprefixed temporal expectation"
    );

    // Each item is matched against *any* expectation, and an expectation may
    // be matched by more than one item because the match consumes nothing.
    let peter = ResultItem::Primitive {
        fhir_type: "string".to_string(),
        value: "Peter".to_string(),
    };
    let twice = [peter.clone(), peter.clone()];
    let two_expectations = [
        Output {
            type_name: "string".to_string(),
            body: "Peter".to_string(),
        },
        Output {
            type_name: "string".to_string(),
            body: "James".to_string(),
        },
    ];
    assert!(oracle::compare::compare_unordered(&two_expectations, &twice).is_ok());
    // But the counts must still match.
    assert!(
        oracle::compare::compare_unordered(&two_expectations, std::slice::from_ref(&peter))
            .is_err()
    );

    expect_passed(
        &report,
        &case.address.to_string(),
        "its five string results are matched against any expectation, not by index",
    );
}

// ---------------------------------------------------------------------------
// path.oracle.008 -- count always; type and value only when stated
// ---------------------------------------------------------------------------

// covers: path.oracle.008
#[test]
fn ordered_comparison_checks_count_always_and_type_and_value_when_stated() {
    let s = oracle::load_pinned_suite();

    // 71 of the 1,103 outputs carry no `type`; a harness that required one
    // would fail cases HAPI passes.
    let untyped = s
        .cases()
        .flat_map(|c| c.outputs.iter())
        .filter(|o| !o.checks_type())
        .count();
    assert_eq!(untyped, 71);
    assert_eq!(s.output_count(), suite::EXPECTED_OUTPUTS);

    let item = ResultItem::Primitive {
        fhir_type: "integer".to_string(),
        value: "1".to_string(),
    };
    let one = std::slice::from_ref(&item);
    // A count mismatch fails whatever the items are.
    assert!(oracle::compare::compare_ordered(&[], one).is_err());
    // An absent type means "do not check the type"; an absent body means "do
    // not check the value".
    let anything = [Output {
        type_name: String::new(),
        body: String::new(),
    }];
    assert!(oracle::compare::compare_ordered(&anything, one).is_ok());
    let wrong_type = [Output {
        type_name: "string".to_string(),
        body: String::new(),
    }];
    assert!(oracle::compare::compare_ordered(&wrong_type, one).is_err());
    let wrong_value = [Output {
        type_name: String::new(),
        body: "2".to_string(),
    }];
    assert!(oracle::compare::compare_ordered(&wrong_value, one).is_err());

    // The temporal prefixes, which are why the suite's expectations read
    // @1974-12-25. The prefix is a property of the comparison: it is never
    // stripped from an expectation and never normalised away on either side.
    for (ty, raw, want) in [
        ("date", "1974-12-25", "@1974-12-25"),
        (
            "dateTime",
            "1974-01-01T00:00:00.000+10:00",
            "@1974-01-01T00:00:00.000+10:00",
        ),
        ("instant", "2015-02-07T13:28:17Z", "@2015-02-07T13:28:17Z"),
        ("time", "10:30", "@T10:30"),
        ("string", "Peter", "Peter"),
        ("boolean", "true", "true"),
        ("integer", "4", "4"),
        ("decimal", "1.58700", "1.58700"),
    ] {
        let i = ResultItem::Primitive {
            fhir_type: ty.to_string(),
            value: raw.to_string(),
        };
        assert_eq!(value::fhirpath_string(&i).as_deref(), Some(want));
    }

    // Every temporal expectation in the pinned file carries its prefix, which
    // is what makes the prefixed comparison the only one that can pass.
    let temporal: Vec<&Output> = s
        .cases()
        .flat_map(|c| c.outputs.iter())
        .filter(|o| {
            matches!(
                o.type_name.as_str(),
                "date" | "dateTime" | "instant" | "time"
            ) && o.checks_value()
        })
        .collect();
    assert_eq!(temporal.len(), 21);
    assert!(temporal.iter().all(|o| o.body.starts_with('@')));

    // And the real cases.
    let (_, report) = run::run_pinned_suite();
    for (group, name) in [
        ("testMiscellaneousAccessorTests", "testExtractBirthDate"),
        ("testPlus", "testPlusDate3"),
    ] {
        expect_passed(
            &report,
            &address_of(&s, group, name),
            "a temporal result is compared through the FHIRPath string value",
        );
    }
}

// ---------------------------------------------------------------------------
// path.oracle.009 -- the Quantity branch comes from the result item's type
// ---------------------------------------------------------------------------

// covers: path.oracle.009
#[test]
fn quantity_comparison_is_chosen_by_the_result_items_type() {
    let s = oracle::load_pinned_suite();

    // There is exactly one <output type="Quantity"> in the pinned file, while
    // the boundary cases expect quantities from a bare <output>. So the branch
    // cannot be chosen from the expectation's type attribute.
    let typed_quantity = s
        .cases()
        .flat_map(|c| c.outputs.iter())
        .filter(|o| o.type_name == "Quantity")
        .count();
    assert_eq!(typed_quantity, 1);
    for (group, name, body) in [
        ("LowBoundary", "LowBoundaryQuantity", "1.58650000 'cm'"),
        ("HighBoundary", "HighBoundaryQuantity", "1.58750000 'm'"),
    ] {
        let c = s.case(group, name).expect("the boundary case exists");
        assert_eq!(c.outputs.len(), 1);
        assert_eq!(c.outputs[0].type_name, "", "{name} has a bare <output>");
        assert_eq!(c.outputs[0].body, body);
    }

    // parseQuantityString, then equalsDeep with BigDecimal's scale-sensitive
    // equality: 1.5865 is not 1.58650000, which is the whole point of the
    // boundary expectations.
    let want = value::parse_quantity_string("1.58650000 'cm'").expect("parses");
    assert_eq!(want, QuantityValue::from_ucum("1.58650000", "cm"));
    assert!(want.equals_deep(&QuantityValue::from_ucum("1.58650000", "cm")));
    assert!(!want.equals_deep(&QuantityValue::from_ucum("1.5865", "cm")));
    assert!(!want.equals_deep(&QuantityValue::from_ucum("1.58650000", "m")));
    assert_eq!(
        value::parse_quantity_string("1 '1'"),
        Some(QuantityValue::from_ucum("1", "1"))
    );
    assert_eq!(
        value::parse_quantity_string("4 days"),
        Some(QuantityValue::from_ucum("4", "d"))
    );
    assert_eq!(
        value::parse_quantity_string("4"),
        Some(QuantityValue::from_ucum("4", "1"))
    );
    assert_eq!(value::parse_quantity_string("not a quantity"), None);

    // A bare <output> whose body is a quantity string is compared deeply when
    // the item is a Quantity and as a primitive when it is not.
    let bare = [Output {
        type_name: String::new(),
        body: "1.58650000 'cm'".to_string(),
    }];
    let q = ResultItem::Quantity(QuantityValue::from_ucum("1.58650000", "cm"));
    assert!(oracle::compare::compare_ordered(&bare, std::slice::from_ref(&q)).is_ok());
    let as_string = ResultItem::Primitive {
        fhir_type: "string".to_string(),
        value: "1.58650000 'cm'".to_string(),
    };
    assert!(oracle::compare::compare_ordered(&bare, std::slice::from_ref(&as_string)).is_ok());
    // A result item that is neither a Quantity nor a primitive fails the case.
    let complex = ResultItem::Complex {
        fhir_type: "HumanName".to_string(),
    };
    let err = oracle::compare::compare_ordered(&bare, std::slice::from_ref(&complex))
        .expect_err("a complex item has no value to compare");
    assert!(err.contains("primitive"), "{err}");

    let (_, report) = run::run_pinned_suite();
    for (group, name) in [
        ("LowBoundary", "LowBoundaryQuantity"),
        ("HighBoundary", "HighBoundaryQuantity"),
        ("testLiterals", "testStringIntegerLiteralToQuantity"),
    ] {
        expect_passed(
            &report,
            &address_of(&s, group, name),
            "its result is a Quantity and is compared deeply",
        );
    }
}

// ---------------------------------------------------------------------------
// path.oracle.010 -- four statuses, and did-not-run is not a pass
// ---------------------------------------------------------------------------

// covers: path.oracle.010
#[test]
fn every_case_reports_one_of_four_distinct_statuses() {
    let (_, report) = run::run_pinned_suite();

    let mut names: Vec<&str> = [
        CaseStatus::Passed,
        CaseStatus::Failed,
        CaseStatus::DidNotRun,
        CaseStatus::SkippedForLicence,
    ]
    .iter()
    .map(|s| s.as_str())
    .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["did-not-run", "failed", "passed", "skipped-for-licence"]
    );

    assert_eq!(report.cases.len(), suite::EXPECTED_CASES);
    let total = report.count(CaseStatus::Passed)
        + report.count(CaseStatus::Failed)
        + report.count(CaseStatus::DidNotRun)
        + report.count(CaseStatus::SkippedForLicence);
    assert_eq!(
        total,
        suite::EXPECTED_CASES,
        "every case must carry exactly one of the four statuses"
    );

    // The invariant that makes the status set worth having: only `passed`
    // counts, so a case that did not run can never be read as a pass.
    assert!(CaseStatus::Passed.is_pass());
    assert!(!CaseStatus::Failed.is_pass());
    assert!(!CaseStatus::DidNotRun.is_pass());
    assert!(!CaseStatus::SkippedForLicence.is_pass());
    assert_eq!(
        report.passed(),
        report.count(CaseStatus::Passed),
        "the pass count counts passes and nothing else"
    );
    assert_eq!(
        report.not_passed().len(),
        suite::EXPECTED_CASES - report.passed()
    );
    for c in report.not_passed() {
        assert!(
            !c.reason.is_empty(),
            "{} is {} with no reason recorded",
            c.address,
            c.status.as_str()
        );
    }

    // Each record carries what the case was, which stage it expected to fail
    // at, the flags in force for each stage, whether it tests a function that
    // is out of scope, and the trace log drained after it ran.
    for c in report.cases.iter() {
        assert!(!c.expression.is_empty(), "{} has no expression", c.address);
        assert_eq!(c.check_flags, StrictnessFlags::CHECK);
        assert_eq!(c.evaluate_flags, StrictnessFlags::EVALUATE);
    }
    assert_eq!(
        report
            .cases
            .iter()
            .filter(|c| c.expected_failure.is_some())
            .count(),
        suite::EXPECTED_SYNTAX_INVALID
            + suite::EXPECTED_SEMANTIC_INVALID
            + suite::EXPECTED_EXECUTION_INVALID
    );
    assert_eq!(
        report
            .cases
            .iter()
            .filter(|c| c.out_of_scope_function.is_some())
            .count(),
        115,
        "the documentFunctions cases must be reported as out of scope"
    );
    assert!(
        report.cases.iter().all(|c| c.trace_log.is_empty()),
        "no case in the pinned suite that ran left an undrained trace log"
    );

    // `skipped-for-licence` is reachable and is not a pass. No case in the
    // pinned suite needs licensed terminology, so this is asserted through the
    // status the engine raises rather than through a suite case.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="t"><expression>'x'.memberOf('http://snomed.info/sct?fhir_vs')</expression>"#,
        r#"<output type="boolean">true</output></test>"#,
        r#"</group></tests>"#
    );
    let synthetic = suite::load(src).unwrap();
    let mut stub = StubEngine::licensed("SNOMED CT");
    let r = run::run(&synthetic, &mut stub, EngineConfig::default());
    assert_eq!(r.cases[0].status, CaseStatus::SkippedForLicence);
    assert!(!r.cases[0].status.is_pass());
    assert_eq!(r.passed(), 0);
}

// ---------------------------------------------------------------------------
// path.oracle.015 -- the harness does not modify the suite
// ---------------------------------------------------------------------------

// covers: path.oracle.015
#[test]
fn a_harness_run_leaves_the_suite_byte_identical() {
    let path = input::repo_path(suite::SUITE_PATH);
    let before = std::fs::read(&path).expect("the vendored suite is committed");
    let (_, report) = run::run_pinned_suite();
    assert_eq!(report.cases.len(), suite::EXPECTED_CASES);
    let after = std::fs::read(&path).expect("the vendored suite is still there");
    assert_eq!(
        before, after,
        "a harness run must leave oracles/fhirpath/tests-fhir-r4.xml byte-identical; \
         `bash oracles/fetch.sh --verify` checks the same thing against the pin"
    );

    // The suite's input files are read-only to the harness too.
    for file in ["patient-example.xml", "appointment-examplereq.json"] {
        let p = input::repo_path(&format!("{}/{file}", suite::INPUT_DIR));
        let a = std::fs::read(&p).expect("the input file is present");
        let loaded = input::load(file).expect("the input file loads");
        let b = std::fs::read(&p).expect("the input file is still present");
        assert_eq!(a, b);
        assert_eq!(
            loaded.bytes, a,
            "the harness keeps the input's bytes exactly as read"
        );
    }
}

// ---------------------------------------------------------------------------
// path.datetime.004 -- the configured time zone
// ---------------------------------------------------------------------------

// covers: path.datetime.004
#[test]
fn the_harness_fixes_the_evaluation_time_zone_at_utc_plus_1100() {
    // FHIRPathTests.test calls
    // TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100")) before every case,
    // so today() and now() are read in UTC+11:00 and never in the machine's
    // zone. The harness fixes the same offset explicitly instead of mutating
    // process-wide state.
    assert_eq!(SUITE_TIME_ZONE_OFFSET_MINUTES, 660);
    assert_eq!(EngineConfig::default().time_zone_offset_minutes, 660);

    let (s, report) = run::run_pinned_suite();
    assert_eq!(report.config.time_zone_offset_minutes, 660);

    // The configuration is handed to the engine rather than merely recorded.
    let src = concat!(
        r#"<tests name="x"><group name="g">"#,
        r#"<test name="t"><expression>today()</expression></test></group></tests>"#
    );
    let mut stub = StubEngine::failing_at(None);
    run::run(
        &suite::load(src).unwrap(),
        &mut stub,
        EngineConfig::default(),
    );
    assert_eq!(
        stub.configured_offset,
        Some(660),
        "the harness must hand the engine its configured time zone"
    );

    // And the four cases that read the clock must pass with it.
    for (group, name) in [
        ("testToday", "testToday1"),
        ("testToday", "testToday2"),
        ("testNow", "testNow1"),
        ("testNow", "testNow2"),
    ] {
        expect_passed(
            &report,
            &address_of(&s, group, name),
            "today() and now() read the engine's configured time zone",
        );
    }
}

// ---------------------------------------------------------------------------
// A stub engine, used only to prove the harness's own grading logic
// ---------------------------------------------------------------------------

/// An engine that succeeds at every stage except one, so the harness's stage
/// attribution, predicate conversion and status mapping can be asserted
/// without an implementation. It never grades a real suite case: every
/// assertion about the pinned suite above goes through
/// `oracle::engine::engine_under_test`.
struct StubEngine {
    fail_at: Option<Stage>,
    licensed: Option<String>,
    returns: Vec<ResultItem>,
    order: Vec<&'static str>,
    configured_offset: Option<i32>,
}

impl StubEngine {
    fn failing_at(fail_at: Option<Stage>) -> Self {
        StubEngine {
            fail_at,
            licensed: None,
            returns: Vec::new(),
            order: Vec::new(),
            configured_offset: None,
        }
    }

    fn returning(returns: Vec<ResultItem>) -> Self {
        StubEngine {
            returns,
            ..StubEngine::failing_at(None)
        }
    }

    fn licensed(system: &str) -> Self {
        StubEngine {
            licensed: Some(system.to_string()),
            ..StubEngine::failing_at(None)
        }
    }

    fn step(&mut self, stage: Stage, name: &'static str) -> Result<(), EngineError> {
        if !self.order.contains(&name) {
            self.order.push(name);
        }
        if let Some(system) = &self.licensed
            && stage == Stage::Evaluate
        {
            return Err(EngineError::RequiresLicensedTerminology {
                system: system.clone(),
            });
        }
        if self.fail_at == Some(stage) {
            return Err(EngineError::Diagnostic {
                message: format!("stub failure at the {} stage", stage.as_str()),
                offset: Some(0),
            });
        }
        Ok(())
    }
}

impl SuiteEngine for StubEngine {
    fn configure(&mut self, config: &EngineConfig) -> Result<(), EngineError> {
        self.configured_offset = Some(config.time_zone_offset_minutes);
        Ok(())
    }

    fn parse(&mut self, _expression: &str) -> Result<ParsedHandle, EngineError> {
        self.step(Stage::Parse, "parse")?;
        Ok(ParsedHandle(0))
    }

    fn check(
        &mut self,
        _parsed: ParsedHandle,
        _context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        assert_eq!(
            flags,
            StrictnessFlags::CHECK,
            "the harness runs the check pass with neither rule enforced"
        );
        self.step(Stage::Check, "check")
    }

    fn evaluate(
        &mut self,
        _parsed: ParsedHandle,
        _input: Option<&SuiteInput>,
        _context: &CaseContext<'_>,
        flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        assert_eq!(
            flags,
            StrictnessFlags::EVALUATE,
            "the harness runs the evaluation with both rules enforced"
        );
        self.step(Stage::Evaluate, "evaluate")?;
        Ok(self.returns.clone())
    }

    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError> {
        // FHIRPathEngine.convertToBoolean: a single boolean is itself,
        // otherwise non-empty is true.
        if let [ResultItem::Primitive { fhir_type, value }] = items
            && fhir_type == "boolean"
        {
            return Ok(value == "true");
        }
        Ok(!items.is_empty())
    }

    fn has_log(&self) -> bool {
        false
    }

    fn take_log(&mut self) -> String {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// The harness hands the engine the whole case context
// ---------------------------------------------------------------------------

/// A stub that records the context and input it was given, so the harness's
/// side of the context-aware run is assertable without an engine.
struct RecordingEngine {
    seen_context: Option<String>,
    seen_context_types: Vec<String>,
    seen_version: Option<String>,
    seen_test_mode: Option<String>,
    seen_check_ordered_functions: bool,
    seen_resource_type: Option<String>,
    seen_root_resource_type: Option<String>,
    seen_input_file: Option<String>,
}

impl RecordingEngine {
    fn new() -> Self {
        RecordingEngine {
            seen_context: None,
            seen_context_types: Vec::new(),
            seen_version: None,
            seen_test_mode: None,
            seen_check_ordered_functions: false,
            seen_resource_type: None,
            seen_root_resource_type: None,
            seen_input_file: None,
        }
    }

    fn record(&mut self, context: &CaseContext<'_>) {
        self.seen_context = context.context.map(|s| s.to_string());
        self.seen_context_types = context.context_types.to_vec();
        self.seen_version = context.version.map(|s| s.to_string());
        self.seen_test_mode = context.test_mode.map(|s| s.to_string());
        self.seen_check_ordered_functions = context.check_ordered_functions;
        self.seen_resource_type = context.resource_type.map(|s| s.to_string());
        self.seen_root_resource_type = context.root_resource_type.map(|s| s.to_string());
    }
}

impl SuiteEngine for RecordingEngine {
    fn configure(&mut self, _config: &EngineConfig) -> Result<(), EngineError> {
        Ok(())
    }

    fn parse(&mut self, _expression: &str) -> Result<ParsedHandle, EngineError> {
        Ok(ParsedHandle(0))
    }

    fn check(
        &mut self,
        _parsed: ParsedHandle,
        context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<(), EngineError> {
        self.record(context);
        Ok(())
    }

    fn evaluate(
        &mut self,
        _parsed: ParsedHandle,
        input: Option<&SuiteInput>,
        context: &CaseContext<'_>,
        _flags: StrictnessFlags,
    ) -> Result<Vec<ResultItem>, EngineError> {
        self.record(context);
        self.seen_input_file = input.map(|i| i.file.clone());
        Ok(Vec::new())
    }

    fn convert_to_boolean(&mut self, items: &[ResultItem]) -> Result<bool, EngineError> {
        Ok(!items.is_empty())
    }

    fn has_log(&self) -> bool {
        false
    }

    fn take_log(&mut self) -> String {
        String::new()
    }
}

// covers: path.oracle.002
#[test]
fn the_harness_hands_the_engine_every_attribute_that_changes_the_run() {
    // Frevo's run honours `context`, `contextType`, `mode` and `version`. The
    // pinned R4 Java harness reads none of them, which is why four
    // `isSemanticChecks` pairs are recorded `hapi-cannot-pass` rather than
    // `failed`; Frevo is graded on its own context-aware run instead.
    let src = concat!(
        r#"<tests name="x"><group name="g" mode="semantic">"#,
        r#"<test name="t" inputfile="observation-example.xml" "#,
        r#"context="Observation.effective" contextType="Period|Timing" "#,
        r#"version="2.1.0" mode="strict" checkOrderedFunctions="true">"#,
        r#"<expression>$this is Period</expression></test>"#,
        r#"</group></tests>"#
    );
    let synthetic = suite::load(src).expect("the synthetic case is well-formed");
    let mut engine = RecordingEngine::new();
    run::run(&synthetic, &mut engine, EngineConfig::default());

    assert_eq!(
        engine.seen_context.as_deref(),
        Some("Observation.effective")
    );
    assert_eq!(engine.seen_context_types, ["Period", "Timing"]);
    assert_eq!(engine.seen_version.as_deref(), Some("2.1.0"));
    assert_eq!(engine.seen_test_mode.as_deref(), Some("strict"));
    assert!(engine.seen_check_ordered_functions);

    // The resource type comes from the loaded input and is passed as both the
    // resource type and the root resource type, as FHIRPathTests does.
    assert_eq!(engine.seen_resource_type.as_deref(), Some("Observation"));
    assert_eq!(
        engine.seen_root_resource_type.as_deref(),
        Some("Observation")
    );
    assert_eq!(
        engine.seen_input_file.as_deref(),
        Some("observation-example.xml"),
        "the engine is handed the input it must evaluate over, by name"
    );
}
