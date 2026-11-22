//! Filtering and projection: `path.filter.001` through `path.filter.009`.
//!
//! ```text
//! cargo test -p frevo-path --test path_filter
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Six of the nine scenarios name a
//! suite group or case, so the real case is driven through the committed
//! harness; the rest state an expression and its result.
//!
//! `path.filter.003` states that a non-boolean `where()` criteria is rejected
//! by the **check**, before evaluation, and that with the check skipped (as
//! `FhirPathR4` does, `path.check.013`) it does not fail. `path.filter.002`
//! states `DEV-0015`: an operator after an indexer on the first term is dropped.
//!
//! Provenance:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
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
use oracle::suite::{Stage, Suite};
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

fn expect_group(group: &str, expected: usize) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    assert_eq!(
        g.cases.len(),
        expected,
        "{group} must have {expected} case(s), not {}",
        g.cases.len()
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

fn context_for<'a>(inp: Option<&'a SuiteInput>) -> CaseContext<'a> {
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

fn eval_patient(expr: &str) -> Vec<ResultItem> {
    let inp = load_input("patient-example.xml");
    eval_over(expr, Some(&inp))
}

/// The stage at which the engine reported a diagnostic for `expr`, with the
/// message. `NotImplemented` panics, so a missing engine is never read as the
/// failure a scenario asked for.
fn expect_failure(expr: &str, inp: Option<&SuiteInput>) -> (Stage, String) {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = match engine.parse(expr) {
        Err(EngineError::Diagnostic { message, .. }) => return (Stage::Parse, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at parse: {e}"),
        Ok(p) => p,
    };
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Err(EngineError::Diagnostic { message, .. }) => return (Stage::Check, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at check: {e}"),
        Ok(()) => {}
    }
    match engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE) {
        Err(EngineError::Diagnostic { message, .. }) => (Stage::Evaluate, message),
        Err(e) => panic!("{expr:?}: the engine gave no graded answer at evaluate: {e}"),
        Ok(items) => panic!("{expr:?} must fail, but it evaluated to {items:?}"),
    }
}

/// Parse and evaluate `expr` with the static check **skipped**, as
/// `FhirPathR4` does (`path.check.013`).
fn eval_unchecked(expr: &str, inp: Option<&SuiteInput>) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

/// `expr` must parse and then fail at **evaluation** with the check skipped.
fn expect_unchecked_evaluation_failure(expr: &str, inp: Option<&SuiteInput>) -> String {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    match engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
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

fn integer(value: &str) -> Vec<ResultItem> {
    primitive("integer", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// path.filter.001 -- `where` keeps items whose criteria is true
// ---------------------------------------------------------------------------

// covers: path.filter.001
#[test]
fn where_keeps_items_whose_criteria_is_true() {
    expect_group("testWhere", 4);

    // `funcWhere` (FHIRPathEngine.java:6249) keeps an item only when `asBool`
    // of its criteria is `Equality.True`, with `$this` rebound per item.
    assert_eq!(
        eval_patient("Patient.name.where(given = 'Jim').count()"),
        integer("1")
    );
    assert_eq!(
        eval_patient("Patient.name.where(given = 'X').count()"),
        integer("0")
    );
    assert_eq!(
        eval_patient("Patient.name.where($this.given = 'Jim').count()"),
        integer("1")
    );

    // The kept item is the whole focus item and not the criteria's value. The
    // name whose given name is `Jim` is the second of the three and carries no
    // `family`, so `use` is what identifies it.
    assert_eq!(
        eval_patient("Patient.name.where(given = 'Jim').use"),
        primitive("code", "usual")
    );
    assert_eq!(
        eval_patient("Patient.name.where(given = 'Jim').family"),
        empty()
    );
}

// ---------------------------------------------------------------------------
// path.filter.002 -- `where` drops items whose criteria is empty
// ---------------------------------------------------------------------------

// covers: path.filter.002
#[test]
fn where_drops_items_whose_criteria_is_empty() {
    // No name in `patient-example.xml` has a prefix, so `prefix = 'Dr'` is
    // empty for every item. `asBool` answers `Equality.Null` for an empty
    // collection (FHIRPathEngine.java:6887), which is not `True`, so the item
    // is dropped rather than kept.
    assert_eq!(eval_patient("Patient.name.where(prefix = 'Dr')"), empty());
    assert_eq!(
        eval_patient("Patient.name.where(prefix = 'Dr').count()"),
        integer("0")
    );

    // Dropped, not failed: an empty criteria is the specified way to exclude an
    // item, which is why this is distinct from `path.filter.003`.
    assert_eq!(
        eval_patient("Patient.name.where(prefix = 'Dr').empty()"),
        boolean(true)
    );

    // An operator that follows an **indexer on the first term** is dropped by
    // the pinned parser (`DEV-0015`; FHIRPathEngine.java:1188-1213, :1508), so
    // `where(given[1] = 'James')` is not a comparison. The check pass fails it,
    // because `where` parameter 1 is a `string` and not a boolean.
    let patient = load_input("patient-example.xml");
    let (stage, message) = expect_failure("Patient.name.where(given[1] = 'James')", Some(&patient));
    assert_eq!(stage, Stage::Check, "{message}");
    assert!(
        message.contains("where"),
        "the check must name where: {message}"
    );

    // With the check skipped the criteria evaluates as `given[1]`, and `asBool`
    // (:6891-6892) answers `True` for a single non-boolean item, so `name[0]`
    // and `name[2]` (each `Peter, James`) are kept and `name[1]` (`Jim`, no
    // second given, so empty) is dropped: two names, as for `path.filter.003`.
    assert_eq!(
        eval_unchecked(
            "Patient.name.where(given[1] = 'James').count()",
            Some(&patient)
        ),
        integer("2")
    );

    // The first-term cases are each observable: the operator is dropped, so
    // `name[0].given = 'Peter'` yields the given names, not a boolean, and
    // `(1 | 2)[0] = 5` yields the integer `1`, not `false`.
    assert_eq!(
        eval_unchecked("name[0].given = 'Peter'", Some(&patient)),
        vec![
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "Peter".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "James".into()
            },
        ]
    );
    assert_eq!(eval_unchecked("(1 | 2)[0] = 5", None), integer("1"));

    // `$this` is the first term there, the indexer is on a later one, and the
    // comparison is real: `name[0]` and `name[2]` match, `name[1]` is empty.
    assert_eq!(
        eval_patient("Patient.name.where($this.given[1] = 'James').count()"),
        integer("2")
    );

    // An indexer on a later term is not affected: these cases evaluate the
    // operator and pass.
    expect_case("testIndexer", "testIndexer1");
    expect_case("testIndexer", "testIndexer2");
    expect_case("testSort", "testSort9");
    expect_case("testQuantityUnitComparison", "testQuantityUnitTextEqual");
}

// ---------------------------------------------------------------------------
// path.filter.003 -- `where` with a non-boolean criteria is rejected by the check
// ---------------------------------------------------------------------------

// covers: path.filter.003
#[test]
fn where_with_a_non_boolean_criteria_is_rejected_by_the_check_before_evaluation() {
    let patient = load_input("patient-example.xml");

    // The **check** fails: `case Where` (FHIRPathEngine.java:3330) calls
    // `checkParamTypes` against a singleton boolean and raises
    // `FHIRPATH_WRONG_PARAM_TYPE` for a `string`.
    let (stage, message) = expect_failure("Patient.name.where('x')", Some(&patient));
    assert_eq!(stage, Stage::Check, "{message}");

    // The expression is not evaluated; evaluated with the check skipped, as
    // `FhirPathR4` does (`path.check.013`), it does **not** fail: `asBool`
    // (:6887-6894) answers `True` for any single non-boolean item, so all three
    // names are kept.
    assert_eq!(
        eval_unchecked("Patient.name.where('x').count()", Some(&patient)),
        integer("3")
    );

    // Not stated by the scenario text, kept from the earlier assertion: the
    // multi-item form fails at evaluation through `asBool`'s `else` arm
    // (:6893, FHIRPATH_UNABLE_BOOLEAN) once the check is skipped.
    expect_unchecked_evaluation_failure("Patient.name.where('x' | 'y')", Some(&patient));
}

// ---------------------------------------------------------------------------
// path.filter.004 -- `select` projects each item and flattens
// ---------------------------------------------------------------------------

// covers: path.filter.004
#[test]
fn select_projects_each_item_and_flattens_the_result() {
    expect_group("testSelect", 3);

    // `funcSelect` (FHIRPathEngine.java:6263) calls `result.addAll(...)` per
    // item, so a projection yielding several values contributes all of them to
    // one flat collection: three names, five given names.
    assert_eq!(
        eval_patient("Patient.name.select(given).count()"),
        integer("5")
    );
    assert_eq!(
        eval_patient("Patient.name.select(given | family).count()"),
        integer("7")
    );

    // Flattened, not nested: the result of a projection is indistinguishable
    // from the same navigation written directly.
    assert_eq!(
        eval_patient("Patient.name.select(given)"),
        eval_patient("Patient.name.given")
    );
}

// ---------------------------------------------------------------------------
// path.filter.005 -- `select` returning empty contributes nothing
// ---------------------------------------------------------------------------

// covers: path.filter.005
#[test]
fn select_returning_empty_for_an_item_contributes_nothing() {
    // Every name's `prefix` is absent, so every projection is empty and the
    // flattened result is empty: `addAll` of an empty list adds nothing, and
    // nothing is substituted for the missing value.
    assert_eq!(eval_patient("Patient.name.select(prefix)"), empty());
    assert_eq!(
        eval_patient("Patient.name.select(prefix).count()"),
        integer("0")
    );

    // The count is the number of projected values and not the number of focus
    // items, which is the property that makes this distinct from `where`.
    assert_eq!(eval_patient("Patient.name.count()"), integer("3"));
    assert_eq!(
        eval_patient("Patient.name.select(given[1]).count()"),
        integer("2")
    );
}

// ---------------------------------------------------------------------------
// path.filter.006 -- `repeat` applies the projection transitively
// ---------------------------------------------------------------------------

// covers: path.filter.006
#[test]
fn repeat_applies_the_projection_transitively() {
    expect_group("testRepeat", 5);

    // `funcRepeat` (FHIRPathEngine.java:5261) keeps applying the projection to
    // the newly added items until a pass adds nothing, so a nested structure is
    // walked to its full depth.
    let valueset = load_input("valueset-example-expansion.xml");
    assert_eq!(
        eval_over(
            "ValueSet.expansion.repeat(contains).count() = 10",
            Some(&valueset)
        ),
        boolean(true)
    );

    let questionnaire = load_input("questionnaire-example.xml");
    assert_eq!(
        eval_over(
            "Questionnaire.repeat(item).code.count() = 11",
            Some(&questionnaire)
        ),
        boolean(true)
    );

    // A projection that is a literal rather than a navigation adds its value
    // once and then stops, which is `testRepeat5`.
    assert_eq!(
        eval_patient("Patient.name.repeat('test')"),
        primitive("string", "test")
    );
}

// ---------------------------------------------------------------------------
// path.filter.007 -- `repeat` terminates on a cyclic projection
// ---------------------------------------------------------------------------

// covers: path.filter.007
#[test]
fn repeat_terminates_on_a_cyclic_projection() {
    // `questionnaire-example.xml` nests `item` inside `item`, so `repeat(item)`
    // is the self-referential case. Termination is the assertion: the test
    // returning at all is half of it, and the count being finite and equal to
    // the number of nested items is the other half.
    let questionnaire = load_input("questionnaire-example.xml");

    // `repeat` does not seed its result with the focus (`funcRepeat`,
    // FHIRPathEngine.java:5261-5290, starts with an empty `result`), so a
    // second `repeat(item)` returns only the items at depth two or deeper: its
    // `count()` is strictly smaller, and the equality is `false`.
    assert_eq!(
        eval_over(
            "Questionnaire.repeat(item).repeat(item).count() < Questionnaire.repeat(item).count()",
            Some(&questionnaire)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Questionnaire.repeat(item).count() = Questionnaire.repeat(item).repeat(item).count()",
            Some(&questionnaire)
        ),
        boolean(false)
    );

    // And the items really are distinct, so termination is not achieved by
    // dropping items that should have been returned.
    assert_eq!(
        eval_over(
            "Questionnaire.repeat(item).isDistinct()",
            Some(&questionnaire)
        ),
        boolean(true)
    );
    assert_eq!(
        eval_over(
            "Questionnaire.repeat(item).code.count() = 11",
            Some(&questionnaire)
        ),
        boolean(true)
    );
    expect_case("testRepeat", "testRepeat2");
}

// ---------------------------------------------------------------------------
// path.filter.008 -- `ofType` keeps only items of the named type
// ---------------------------------------------------------------------------

// covers: path.filter.008
#[test]
fn of_type_keeps_only_items_of_the_named_type() {
    // `Patient.gender` is a `code`. `funcOfType` (FHIRPathEngine.java:5198)
    // compares the item's own FHIR type name, so the declared type matches and
    // its *base* type does not: this is the one place `code` is not a `string`.
    assert_eq!(
        eval_patient("Patient.gender.ofType(code)"),
        primitive("code", "male")
    );
    assert_eq!(eval_patient("Patient.gender.ofType(string)"), empty());

    expect_case("testInheritance", "testFHIRPathAsFunction17");
    expect_case("testInheritance", "testFHIRPathAsFunction16");

    // `is` answers the inheritance question instead, so the two are not the
    // same test and an implementation cannot share one answer between them
    // (`path.types.004`).
    assert_eq!(eval_patient("Patient.gender is string"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.filter.009 -- `ofType` accepts a namespace-qualified type name
// ---------------------------------------------------------------------------

// covers: path.filter.009
#[test]
fn of_type_accepts_a_namespace_qualified_type_name() {
    assert_eq!(
        eval_patient("Patient.ofType(FHIR.Patient).type().name"),
        primitive("string", "Patient")
    );

    // The three spellings the `testType` group grades -- bare, qualified and
    // qualified with the name in backticks -- must all resolve to the same
    // type.
    for name in ["testType20", "testType21", "testType23"] {
        expect_case("testType", name);
    }
    assert_eq!(
        eval_patient("Patient.ofType(Patient).type().name"),
        primitive("string", "Patient")
    );
    assert_eq!(
        eval_patient("Patient.ofType(FHIR.`Patient`).type().name"),
        primitive("string", "Patient")
    );

    // `System.Patient` **raises** `FHIRPATH_INVALID_TYPE` rather than yielding
    // empty: `funcOfType` (FHIRPathEngine.java:5198-5207) rejects a name for
    // which `isKnownType` (:1904) is false, and the `System` namespace admits
    // only `String`, `Boolean`, `Integer`, `Decimal`, `Quantity`, `DateTime`,
    // `Time`, `SimpleTypeInfo` and `ClassInfo`.
    let patient = load_input("patient-example.xml");
    let (stage, message) = expect_failure("Patient.ofType(System.Patient)", Some(&patient));
    assert_eq!(stage, Stage::Evaluate, "{message}");
}
