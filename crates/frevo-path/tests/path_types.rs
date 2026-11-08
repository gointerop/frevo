//! Type operators and reflection: `path.types.001` through `path.types.013`.
//!
//! ```text
//! cargo test -p frevo-path --test path_types
//! ```
//!
//! Twelve of the thirteen scenarios name suite cases, which are driven through
//! the committed harness. The thirteenth, `path.types.011`, describes a static
//! check the pinned engine does not perform; that is recorded at the test.
//!
//! Provenance: `FHIRPathEngine.java` and `ExpressionNode.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`. The two namespaces are decided
//! in two places that do not share code: `opIs` (:1940-1966) for the `is`
//! operator and `funcIs` (:5045-5100) for the `is()` function, which is why
//! `path.types.010` exists at all.

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

/// The single expected `<output>` the pinned file records for a case.
fn sole_output(group: &str, name: &str) -> (String, String) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert_eq!(case.outputs.len(), 1, "{group}/{name} has one output");
    (
        case.outputs[0].type_name.clone(),
        case.outputs[0].body.clone(),
    )
}

fn expects_no_output(group: &str, name: &str) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert!(
        case.outputs.is_empty(),
        "{group}/{name} must expect nothing, but it expects {:?}",
        case.outputs
    );
}

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

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be evaluated over: {e}")
    })
}

fn context_for(inp: Option<&SuiteInput>) -> CaseContext<'_> {
    let resource_type = inp.and_then(|i| i.resource_type());
    CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    }
}

fn eval_over(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    engine
        .evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

fn eval(expr: &str) -> Vec<ResultItem> {
    eval_over(expr, None)
}

/// `expr` must parse and then fail the **static check** with a diagnostic.
///
/// Failing at evaluation instead is not what the scenario asks for, so the
/// check's success is an explicit failure of this helper.
fn expect_check_error(expr: &str, context_type: &str) -> String {
    let mut engine = fresh_engine();
    let context = CaseContext {
        root_resource_type: Some(context_type),
        resource_type: Some(context_type),
        ..CaseContext::default()
    };
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before the check can reject it: {e}"));
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Ok(()) => panic!("{expr:?} must fail the static check against {context_type}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
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

// ---------------------------------------------------------------------------
// path.types.001 -- the System namespace for a literal
// ---------------------------------------------------------------------------

// covers: path.types.001
#[test]
fn is_tests_the_system_namespace_for_a_literal() {
    for (group, name) in [
        ("testType", "testType7"),
        ("testType", "testType8"),
        ("testTypes", "testIntegerLiteralIsInteger"),
        ("testTypes", "testIntegerLiteralIsSystemInteger"),
    ] {
        let (ty, body) = sole_output(group, name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case(group, name);
    }

    // A literal's type lives in the System namespace, and the namespace may be
    // left implicit or written out. `funcIs` picks System for the nine
    // capitalised names it knows (FHIRPathEngine.java:5063) and FHIR for
    // anything else, so an unqualified `Boolean` is System and an unqualified
    // `boolean` is FHIR -- which `path.types.002` is the other half of.
    assert_eq!(eval("true is Boolean"), boolean(true));
    assert_eq!(eval("true is System.Boolean"), boolean(true));
    assert_eq!(eval("1.is(Integer)"), boolean(true));
    assert_eq!(eval("1.is(System.Integer)"), boolean(true));

    // And a literal of the wrong type is false rather than empty or an error.
    assert_eq!(eval("1 is System.Boolean"), boolean(false));
    assert_eq!(eval("'1'.is(Integer)"), boolean(false));
}

// ---------------------------------------------------------------------------
// path.types.002 -- the FHIR namespace for a model element
// ---------------------------------------------------------------------------

// covers: path.types.002
#[test]
fn is_tests_the_fhir_namespace_for_a_model_element() {
    for name in ["testType11", "testType12", "testType13", "testType14"] {
        let (ty, body) = sole_output("testType", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testType", name);
    }

    // A model element's type is in the FHIR namespace and not in System, even
    // when the two names differ only in case. This is the distinction a
    // single-namespace implementation would collapse.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.active.is(boolean)", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.active.is(FHIR.boolean)", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.active.is(Boolean)", Some(&patient)),
        boolean(false),
        "a FHIR boolean is not a System.Boolean"
    );
    assert_eq!(
        eval_over("Patient.active.is(System.Boolean)", Some(&patient)),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.types.003 -- a resource type
// ---------------------------------------------------------------------------

// covers: path.types.003
#[test]
fn is_on_a_resource_tests_the_resource_type() {
    for name in ["testType17", "testType18", "testType19"] {
        let (ty, body) = sole_output("testType", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
        expect_case("testType", name);
    }

    let patient = load_input("patient-example.xml");
    for expr in [
        "Patient.is(Patient)",
        "Patient.is(FHIR.Patient)",
        "Patient.is(FHIR.`Patient`)",
    ] {
        assert_eq!(eval_over(expr, Some(&patient)), boolean(true), "{expr}");
    }

    // A resource is not in the System namespace, which the suite grades as
    // testType22, and is not a different resource type.
    assert_eq!(
        eval_over("Patient.is(System.Patient)", Some(&patient)),
        boolean(false)
    );
    expect_case("testType", "testType22");
    assert_eq!(
        eval_over("Patient.is(Observation)", Some(&patient)),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.types.004 -- the FHIR type hierarchy
// ---------------------------------------------------------------------------

// covers: path.types.004
#[test]
fn is_follows_the_fhir_type_hierarchy() {
    for (name, expected) in [
        ("testFHIRPathIsFunction1", "true"),
        ("testFHIRPathIsFunction2", "true"),
        ("testFHIRPathIsFunction3", "false"),
    ] {
        let (ty, body) = sole_output("testInheritance", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", expected));
        expect_case("testInheritance", name);
    }

    // `code` derives from `string`, so a code *is* a string; `id` also derives
    // from `string` but is a sibling of `code`, so a code is not an id. The
    // walk is up the `baseDefinition` chain
    // (FHIRPathEngine.java:1953-1963), not a name match.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.gender.is(code)", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.gender.is(string)", Some(&patient)),
        boolean(true),
        "code derives from string"
    );
    assert_eq!(
        eval_over("Patient.gender.is(id)", Some(&patient)),
        boolean(false),
        "id is a sibling of code, not an ancestor"
    );
}

// ---------------------------------------------------------------------------
// path.types.005 -- `uri` against `url`
// ---------------------------------------------------------------------------

// covers: path.types.005
#[test]
fn is_distinguishes_uri_from_url() {
    for (name, expected) in [
        ("testFHIRPathIsFunction4", "true"),
        ("testFHIRPathIsFunction5", "false"),
    ] {
        let (ty, body) = sole_output("testInheritance", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", expected));
        expect_case("testInheritance", name);
    }

    // `Questionnaire.url` is declared `uri` in R4, and `url` derives from
    // `uri` -- so the element is a uri and is not a url. Getting this backwards
    // is the mistake a name-similarity match makes.
    let questionnaire = load_input("questionnaire-example.xml");
    assert_eq!(
        eval_over("Questionnaire.url.is(uri)", Some(&questionnaire)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Questionnaire.url.is(url)", Some(&questionnaire)),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.types.006 -- an extension's value
// ---------------------------------------------------------------------------

// covers: path.types.006
#[test]
fn is_on_an_extensions_value_resolves_the_concrete_choice_type() {
    for (name, expected) in [
        ("testFHIRPathIsFunction8", "true"),
        ("testFHIRPathIsFunction9", "true"),
        ("testFHIRPathIsFunction10", "false"),
    ] {
        let (ty, body) = sole_output("testInheritance", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", expected));
        expect_case("testInheritance", name);
    }

    // The extension's value is an `Age`, which derives from `Quantity`; a
    // `Duration` also derives from `Quantity` but is a sibling of `Age`. So the
    // concrete type has to be resolved from the data rather than from the
    // element definition, whose type is the whole choice.
    let observation = load_input("observation-example.xml");
    const AGE: &str =
        "Observation.extension('http://example.com/fhir/StructureDefinition/patient-age').value";
    assert_eq!(
        eval_over(&format!("{AGE} is Age"), Some(&observation)),
        boolean(true)
    );
    assert_eq!(
        eval_over(&format!("{AGE} is Quantity"), Some(&observation)),
        boolean(true),
        "Age derives from Quantity"
    );
    assert_eq!(
        eval_over(&format!("{AGE} is Duration"), Some(&observation)),
        boolean(false),
        "Duration is a sibling of Age, not an ancestor"
    );
}

// ---------------------------------------------------------------------------
// path.types.007 -- `as`
// ---------------------------------------------------------------------------

// covers: path.types.007
#[test]
fn as_yields_the_value_on_a_match_and_empty_otherwise() {
    let (ty, body) = sole_output("testInheritance", "testFHIRPathAsFunction12");
    assert_eq!(
        (ty.as_str(), body.as_str()),
        ("code", "male"),
        "the result keeps its own type, which is `code` and not the requested name"
    );
    expect_case("testInheritance", "testFHIRPathAsFunction12");
    for name in ["testFHIRPathAsFunction11", "testFHIRPathAsFunction13"] {
        expects_no_output("testInheritance", name);
        expect_case("testInheritance", name);
    }

    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.gender.as(code)", Some(&patient)),
        primitive("code", "male")
    );
    assert_eq!(
        eval_over("Patient.gender.as(string)", Some(&patient)),
        empty(),
        "`as` requires the exact type, unlike `is`, which follows the hierarchy"
    );
    assert_eq!(eval_over("Patient.gender.as(id)", Some(&patient)), empty());

    // Which is the asymmetry worth stating outright: `is(string)` is true for
    // the same element that `as(string)` declines.
    assert_eq!(
        eval_over("Patient.gender.is(string)", Some(&patient)),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.types.008 -- `as` on a string-typed element
// ---------------------------------------------------------------------------

// covers: path.types.008
#[test]
fn as_on_a_string_typed_element_does_not_match_code() {
    let (ty, body) = sole_output("testInheritance", "testFHIRPathAsFunction14");
    assert_eq!((ty.as_str(), body.as_str()), ("string", "20150622"));
    expect_case("testInheritance", "testFHIRPathAsFunction14");
    expects_no_output("testInheritance", "testFHIRPathAsFunction15");
    expect_case("testInheritance", "testFHIRPathAsFunction15");

    // The mirror of `path.types.007`: `ValueSet.version` is a `string`, so
    // `as(string)` matches and `as(code)` does not -- the relationship does not
    // run downwards any more than it runs upwards.
    let valueset = load_input("valueset-example-expansion.xml");
    assert_eq!(
        eval_over("ValueSet.version.as(string)", Some(&valueset)),
        primitive("string", "20150622")
    );
    assert_eq!(
        eval_over("ValueSet.version.as(code)", Some(&valueset)),
        empty()
    );
    assert_eq!(
        eval_over("ValueSet.version.is(code)", Some(&valueset)),
        boolean(false),
        "a string is not a code under `is` either, because code derives from \
         string and not the other way round"
    );
}

// ---------------------------------------------------------------------------
// path.types.009 -- `type()`
// ---------------------------------------------------------------------------

// covers: path.types.009
#[test]
fn type_reports_a_namespace_and_a_name() {
    // The eight the scenario names, each graded by the suite as an equality
    // against the expected string.
    for name in [
        "testType1",
        "testType1a",
        "testType2a",
        "testType4",
        "testType9",
        "testType10",
        "testType15",
        "testType16",
    ] {
        let (ty, body) = sole_output("testType", name);
        assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"), "{name}");
        expect_case("testType", name);
    }

    // Stated directly, so the eight values are readable here and not only as
    // equalities inside the oracle file. A literal is in System with a
    // capitalised name; a model element and a resource are in FHIR with the
    // name the model uses, which for a primitive is lower case.
    let patient = load_input("patient-example.xml");
    assert_eq!(eval("1.type().namespace"), primitive("string", "System"));
    assert_eq!(eval("1.type().name"), primitive("string", "Integer"));
    assert_eq!(eval("'1'.type().name"), primitive("string", "String"));
    assert_eq!(eval("true.type().name"), primitive("string", "Boolean"));
    assert_eq!(
        eval_over("Patient.active.type().namespace", Some(&patient)),
        primitive("string", "FHIR")
    );
    assert_eq!(
        eval_over("Patient.active.type().name", Some(&patient)),
        primitive("string", "boolean")
    );
    assert_eq!(
        eval_over("Patient.type().namespace", Some(&patient)),
        primitive("string", "FHIR")
    );
    assert_eq!(
        eval_over("Patient.type().name", Some(&patient)),
        primitive("string", "Patient")
    );
}

// ---------------------------------------------------------------------------
// path.types.010 -- the operator and the function agree
// ---------------------------------------------------------------------------

// covers: path.types.010
#[test]
fn the_is_operator_and_the_is_function_agree() {
    // They are two separate implementations -- `opIs` for the operator and
    // `funcIs` for the function -- so agreement is a property that has to be
    // tested rather than one that follows from the code.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.gender is code", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.gender.is(code)", Some(&patient)),
        boolean(true)
    );

    // Across the cases this file already has a stated answer for, in both
    // namespaces and on both sides of the hierarchy.
    for (operator, function, expected) in [
        (
            "Patient.gender is string",
            "Patient.gender.is(string)",
            true,
        ),
        ("Patient.gender is id", "Patient.gender.is(id)", false),
        (
            "Patient.active is boolean",
            "Patient.active.is(boolean)",
            true,
        ),
        (
            "Patient.active is System.Boolean",
            "Patient.active.is(System.Boolean)",
            false,
        ),
        ("Patient is Patient", "Patient.is(Patient)", true),
    ] {
        assert_eq!(
            eval_over(operator, Some(&patient)),
            boolean(expected),
            "{operator}"
        );
        assert_eq!(
            eval_over(function, Some(&patient)),
            boolean(expected),
            "{function}"
        );
    }

    // And on literals, where the namespace is System.
    assert_eq!(eval("true is Boolean"), eval("true.is(Boolean)"));
    assert_eq!(eval("1 is Integer"), eval("1.is(Integer)"));
    assert_eq!(eval("1 is Boolean"), eval("1.is(Boolean)"));
}

// ---------------------------------------------------------------------------
// path.types.011 -- an unknown type name
// ---------------------------------------------------------------------------

// covers: path.types.011
#[test]
fn is_with_an_unknown_type_name_is_a_semantic_error() {
    // NOTE FOR REVIEW -- scenario against pinned source.
    //
    // The pinned R4 engine performs no such check for `is`, at either stage:
    //
    //  - at check time the right operand of `Is` is not type-checked at all.
    //    `executeType`'s operator loop calls `executeTypeName` rather than
    //    `executeType` for `Is` and `As` (FHIRPathEngine.java:1626-1629), and
    //    `executeTypeName` just wraps the name in a TypeDetails
    //    (:1651-1653). `operateTypes`'s `case Is` then returns Boolean after a
    //    cardinality warning (:2013-2015). Nothing consults `isKnownType`.
    //  - at evaluation `opIs` compares the name against the focus's type and
    //    walks its ancestors, returning `false` when nothing matches
    //    (:1940-1966). `funcIs` does the same (:5045-5100).
    //
    // So `Patient.gender is NoSuchType` checks cleanly and evaluates to
    // `false` upstream. The three sites that *do* call `isKnownType` are
    // `opAs` (:1881-1883), `funcAs` (:5163-5165) and `funcOfType` (:5206-5208),
    // and all three raise at **evaluation**, which is why the suite's
    // `testInheritance/testFHIRPathAsFunction23` and `24` are
    // `invalid="execution"` and there is no `is` counterpart.
    //
    // Written to the scenario as stated.
    let message = expect_check_error("Patient.gender is NoSuchType", "Patient");
    assert!(
        message.contains("NoSuchType"),
        "the diagnostic must name the type it could not resolve: {message:?}"
    );

    // The function spelling must agree with the operator spelling, as
    // `path.types.010` requires.
    let function = expect_check_error("Patient.gender.is(NoSuchType)", "Patient");
    assert!(function.contains("NoSuchType"), "{function:?}");

    // A type name that does exist checks cleanly, so the rejection is about the
    // name and not about the operator.
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.gender is code", Some(&patient)),
        boolean(true)
    );

    // The `as` and `ofType` spellings are refused as well -- upstream at
    // evaluation, which the suite grades.
    expect_case("testInheritance", "testFHIRPathAsFunction23");
    expect_case("testInheritance", "testFHIRPathAsFunction24");
}

// ---------------------------------------------------------------------------
// path.types.012 -- an empty collection
// ---------------------------------------------------------------------------

// covers: path.types.012
#[test]
fn is_on_an_empty_collection_yields_empty() {
    // `opIs` returns an empty result when either side is empty
    // (FHIRPathEngine.java:1942-1943), and `funcIs` returns `makeNull()` for an
    // empty focus (:5046-5048). Empty, not false -- so `{} is Boolean` cannot
    // be used as a presence test.
    assert_eq!(eval("{} is Boolean"), empty());
    assert_eq!(eval("{}.is(Boolean)"), empty());
    assert_eq!(eval("({} is Boolean).empty()"), boolean(true));

    // And the same for a model element that is not present. patient-example.xml
    // carries `deceasedBoolean` but no `multipleBirth[x]`, so the first of
    // these is a boolean and the second is empty -- which separates "the
    // element is absent" from "the type does not match".
    let patient = load_input("patient-example.xml");
    assert_eq!(
        eval_over("Patient.deceased is boolean", Some(&patient)),
        boolean(true)
    );
    assert_eq!(
        eval_over("Patient.multipleBirth is boolean", Some(&patient)),
        empty(),
        "an absent choice element gives an empty focus, so `is` is empty"
    );
}

// ---------------------------------------------------------------------------
// path.types.013 -- the whole surface
// ---------------------------------------------------------------------------

// covers: path.types.013
#[test]
fn the_whole_types_type_and_inheritance_surface_holds() {
    // 155 cases between the three groups. The individual scenarios above pick
    // out the ones whose behaviour is stated in prose; this one is the rest of
    // the surface, so a regression anywhere in it is visible.
    expect_group("testTypes", 101);
    expect_group("testType", 30);
    expect_group("testInheritance", 24);
}
