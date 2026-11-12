//! Subsetting: `path.subset.001` through `path.subset.013`.
//!
//! ```text
//! cargo test -p frevo-path --test path_subset
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Eight scenarios name a suite group
//! and are driven through the committed harness; the rest state an expression
//! and its result, and two are marked differential-only by the delta itself:
//!
//! - `path.subset.003` -- `single()` on an **empty** focus throws. `funcSingle`
//!   (FHIRPathEngine.java:5087) returns the focus only when `focus.size() == 1`
//!   and throws `FHIRPATH_NO_COLLECTION` otherwise, so size 0 takes the throw.
//!   This is `DEV-0001`, resolved as HAPI's behaviour; `testSingle/testSingle2`
//!   covers only the more-than-one case;
//! - `path.subset.013` -- only `skip` is graded live, by
//!   `testDollar/testDollarOrderNotAllowed`; the other five of the six
//!   `checkOrdered` call sites are differential-only.
//!
//! Three cases in `testSkip` and two in `testTake` state behaviour the pinned
//! engine does not have, and the tests that drive those groups say so at the
//! assertion: `funcSkip` (:6101) and `funcTake` (:5024) read
//! `n1.get(0).primitiveValue()` without checking that `n1` is non-empty, and
//! `funcSkip` starts its loop at the given index without a lower bound.
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

/// `expr` must parse and check and then fail at **evaluation** with a real
/// diagnostic.
fn expect_evaluation_failure(expr: &str, inp: Option<&SuiteInput>) -> String {
    let mut engine = fresh_engine();
    let context = context_for(inp);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before it can fail at evaluation: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    match engine.evaluate(parsed, inp, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

/// `expr` must fail the **static check** over `inp` with a real diagnostic.
fn expect_check_failure(expr: &str, inp: &SuiteInput, flags: StrictnessFlags) -> String {
    let mut engine = fresh_engine();
    let context = context_for(Some(inp));
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before the check can reject it: {e}"));
    match engine.check(parsed, &context, flags) {
        Ok(()) => panic!("{expr:?} must fail the static check, but the check passed"),
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

fn integer(value: &str) -> Vec<ResultItem> {
    primitive("integer", value)
}

fn empty() -> Vec<ResultItem> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// path.subset.001 -- `single` returns the only item
// ---------------------------------------------------------------------------

// covers: path.subset.001
#[test]
fn single_returns_the_only_item() {
    expect_group("testSingle", 2);

    // `funcSingle` (FHIRPathEngine.java:5087) returns the focus unchanged when
    // its size is exactly 1, so the item and not a copy of its value.
    assert_eq!(
        eval_patient("Patient.name.first().single().exists()"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("Patient.name.first().single().family"),
        primitive("string", "Chalmers")
    );
    assert_eq!(eval_over("1.single()", None), integer("1"));
}

// ---------------------------------------------------------------------------
// path.subset.002 -- `single` on more than one item fails
// ---------------------------------------------------------------------------

// covers: path.subset.002
#[test]
fn single_on_a_collection_of_more_than_one_is_an_execution_error() {
    let patient = load_input("patient-example.xml");

    // Three names, so `single()` throws `FHIRPATH_NO_COLLECTION` with the
    // function name and the size (FHIRPathEngine.java:5091).
    let message = expect_evaluation_failure("Patient.name.single()", Some(&patient));
    assert!(
        message.contains("single"),
        "the error must name the function: {message}"
    );
    expect_case("testSingle", "testSingle2");

    // The suite case is `invalid="execution"`, so the stage is part of the
    // expectation and a check-stage rejection would not satisfy it.
    let (suite, _) = pinned();
    assert_eq!(
        suite
            .case("testSingle", "testSingle2")
            .expect("the case resolves")
            .expected_failure,
        Some(Stage::Evaluate)
    );
}

// ---------------------------------------------------------------------------
// path.subset.003 -- `single` on an empty collection fails (DEV-0001)
// ---------------------------------------------------------------------------

// covers: path.subset.003
#[test]
fn single_on_an_empty_collection_is_an_execution_error() {
    let patient = load_input("patient-example.xml");

    // `funcSingle` (FHIRPathEngine.java:5087-5092):
    //
    //     if (focus.size() == 1) { return focus; }
    //     throw makeException(expr, FHIRPATH_NO_COLLECTION, "single", focus.size());
    //
    // Size 0 takes the throw. The FHIRPath specification returns empty for an
    // empty focus; `docs/deviations.md` records the divergence as `DEV-0001`
    // and resolves it as HAPI's behaviour.
    //
    // Differential-only: `testSingle/testSingle2` covers only the
    // more-than-one case, which is asserted here so the gap is visible.
    let (suite, _) = pinned();
    let empty_focus_cases = suite
        .cases()
        .filter(|c| c.expression.contains(".single()") && c.expression.contains("prefix"))
        .count();
    assert_eq!(
        empty_focus_cases, 0,
        "no live case grades the empty-focus form; if one appears, this \
         scenario stops being differential-only"
    );

    let message = expect_evaluation_failure("Patient.name.prefix.single()", Some(&patient));
    assert!(
        message.contains("single"),
        "the error must name the function: {message}"
    );

    // Empty is not the answer, which is the whole of DEV-0001.
    expect_evaluation_failure("{}.single()", None);
}

// ---------------------------------------------------------------------------
// path.subset.004 -- `first` and `last` select the ends
// ---------------------------------------------------------------------------

// covers: path.subset.004
#[test]
fn first_and_last_select_the_ends() {
    expect_group("testFirstLast", 2);

    // `funcFirst` (:6240) and `funcLast` (:6232) each append one item when the
    // focus is non-empty and nothing when it is empty.
    assert_eq!(
        eval_patient("Patient.name.first().family"),
        primitive("string", "Chalmers")
    );
    assert_eq!(
        eval_patient("Patient.name.last().family"),
        primitive("string", "Windsor")
    );
    assert_eq!(eval_patient("Patient.name.prefix.first()"), empty());
    assert_eq!(eval_patient("Patient.name.prefix.last()"), empty());

    // The two ends differ, so neither could be satisfied by the other.
    assert_eq!(
        eval_patient("Patient.name.first().family = Patient.name.last().family"),
        boolean(false)
    );
}

// ---------------------------------------------------------------------------
// path.subset.005 -- `tail` drops the first item
// ---------------------------------------------------------------------------

// covers: path.subset.005
#[test]
fn tail_drops_the_first_item() {
    expect_group("testTail", 2);

    // `funcTail` (FHIRPathEngine.java:6112) starts its loop at index 1, so the
    // result is one shorter and the first item is the one removed.
    assert_eq!(eval_over("(0 | 1 | 2).tail().count()", None), integer("2"));
    assert_eq!(eval_patient("Patient.name.tail().count()"), integer("2"));
    // The second of the three names carries no `family`, so `use` is what
    // identifies the item `tail()` now starts at.
    assert_eq!(
        eval_patient("Patient.name.tail().first().use"),
        primitive("code", "usual")
    );
    assert_eq!(
        eval_patient("Patient.name.tail().last().family"),
        primitive("string", "Windsor")
    );

    // A one-item and an empty focus both tail to empty, with no error.
    assert_eq!(eval_patient("Patient.name.first().tail()"), empty());
    assert_eq!(eval_over("{}.tail()", None), empty());
}

// ---------------------------------------------------------------------------
// path.subset.006 -- `skip` drops the first n items
// ---------------------------------------------------------------------------

// covers: path.subset.006
#[test]
fn skip_drops_the_first_n_items() {
    expect_group("testSkip", 8);

    // The uncontroversial half: a positive count drops that many items from the
    // front (`funcSkip`, FHIRPathEngine.java:6101).
    assert_eq!(eval_over("(0 | 1 | 2).skip(1).count()", None), integer("2"));
    assert_eq!(eval_over("(0 | 1 | 2).skip(2).count()", None), integer("1"));
    assert_eq!(eval_patient("Patient.name.skip(1).count()"), integer("2"));

    // The rest of the group states behaviour the pinned engine does not have,
    // and the group assertion above therefore cannot pass on parity alone.
    // Recorded rather than accommodated:
    //
    // - `testSkip5` -- `skip(-1)` must return the input. `funcSkip`'s loop is
    //   `for (int i = i1; i < focus.size(); i++) result.add(focus.get(i))`
    //   (:6105-6107) with no lower bound, so `i1 = -1` reaches
    //   `focus.get(-1)` and raises `IndexOutOfBoundsException`;
    // - `testSkip7` and `testSkip8` -- `skip({})` must return empty.
    //   `funcSkip` reads `n1.get(0).primitiveValue()` (:6103) without testing
    //   `n1.isEmpty()`, so an empty count raises as well.
    //
    // HAPI therefore fails those three; Frevo passing them is `ahead-of-hapi`
    // under `path.oracle.012`.
    assert_eq!(
        eval_patient("Patient.name.skip(-1).count() = Patient.name.count()"),
        boolean(true)
    );
    assert_eq!(
        eval_patient("Patient.name.skip(0).count() = Patient.name.count()"),
        boolean(true)
    );
    assert_eq!(
        eval_over("(0 | 1 | 2).skip({}).empty()", None),
        boolean(true)
    );
    assert_eq!(eval_over("{}.skip({}).empty()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.subset.007 -- `skip` past the end yields empty
// ---------------------------------------------------------------------------

// covers: path.subset.007
#[test]
fn skip_past_the_end_yields_empty() {
    // Three names, so skipping three leaves nothing and the navigation after it
    // is empty rather than an error (`funcSkip`'s loop simply does not run).
    assert_eq!(eval_patient("Patient.name.skip(3).given"), empty());
    assert_eq!(eval_patient("Patient.name.skip(99).given"), empty());
    expect_case("testDollar", "testDollarOrderAllowedA");

    // Exactly at the boundary: skipping two leaves one.
    assert_eq!(eval_patient("Patient.name.skip(2).count()"), integer("1"));
    assert_eq!(eval_patient("Patient.name.skip(3).count()"), integer("0"));
    expect_case("testSkip", "testSkip4");
}

// ---------------------------------------------------------------------------
// path.subset.008 -- `take` keeps the first n items
// ---------------------------------------------------------------------------

// covers: path.subset.008
#[test]
fn take_keeps_the_first_n_items() {
    expect_group("testTake", 9);

    // `funcTake` (FHIRPathEngine.java:5024) iterates to
    // `Math.min(focus.size(), i1)`, so a count past the end is clamped rather
    // than an error.
    assert_eq!(eval_over("(0 | 1 | 2).take(1).count()", None), integer("1"));
    assert_eq!(eval_over("(0 | 1 | 2).take(2).count()", None), integer("2"));
    assert_eq!(
        eval_patient("Patient.name.take(3).given.count()"),
        integer("5")
    );
    assert_eq!(
        eval_patient("Patient.name.take(4).given.count()"),
        integer("5")
    );

    // Two cases in the group state behaviour the pinned engine does not have,
    // recorded rather than accommodated: `testTake8` and `testTake9` expect
    // `take({})` to be empty, while `funcTake` reads
    // `n1.get(0).primitiveValue()` (:5026) without testing `n1.isEmpty()` and
    // so raises. HAPI fails both; Frevo passing them is `ahead-of-hapi`.
    assert_eq!(
        eval_over("(0 | 1 | 2).take({}).empty()", None),
        boolean(true)
    );
    assert_eq!(eval_over("{}.take({}).empty()", None), boolean(true));
}

// ---------------------------------------------------------------------------
// path.subset.009 -- `take(0)` and `take(-1)` yield empty
// ---------------------------------------------------------------------------

// covers: path.subset.009
#[test]
fn take_zero_and_take_negative_yield_empty() {
    // `Math.min(focus.size(), i1)` is 0 for a count of 0 and negative for a
    // negative count, so in both cases the loop body never runs and the result
    // is empty. Unlike `skip`, `take` therefore has no lower-bound hazard.
    assert_eq!(eval_patient("Patient.name.take(0)"), empty());
    assert_eq!(eval_patient("Patient.name.take(-1)"), empty());
    assert_eq!(
        eval_patient("Patient.name.take(0).given.exists()"),
        boolean(false)
    );
    expect_case("testTake", "testTake7");

    // Empty and not an error, which is what distinguishes this from
    // `path.subset.002`.
    assert_eq!(eval_patient("Patient.name.take(-1).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.subset.010 -- `intersect` keeps items in both, without duplicates
// ---------------------------------------------------------------------------

// covers: path.subset.010
#[test]
fn intersect_keeps_items_present_in_both_without_duplicates() {
    expect_group("testIntersect", 4);

    // `funcIntersect` (FHIRPathEngine.java:5062) adds an item only when it is
    // in the argument **and** not already in the result, so duplicates in the
    // focus collapse: that is `1.combine(1).intersect(1).count() = 1`.
    assert_eq!(
        eval_over("(1 | 2 | 3).intersect(2 | 4).count()", None),
        integer("1")
    );
    assert_eq!(
        eval_over("(1 | 2).intersect(4).empty()", None),
        boolean(true)
    );
    assert_eq!(
        eval_over("(1 | 2).intersect({}).empty()", None),
        boolean(true)
    );
    assert_eq!(
        eval_over("1.combine(1).intersect(1).count()", None),
        integer("1")
    );
}

// ---------------------------------------------------------------------------
// path.subset.011 -- `exclude` removes items present in the argument
// ---------------------------------------------------------------------------

// covers: path.subset.011
#[test]
fn exclude_removes_items_present_in_the_argument() {
    expect_group("testExclude", 4);

    // `funcExclude` (FHIRPathEngine.java:5074) keeps an item when no item of
    // the argument is equal to it, so an argument that matches nothing is the
    // identity and an empty argument changes nothing.
    assert_eq!(
        eval_over("(1 | 2 | 3).exclude(2 | 4).count()", None),
        integer("2")
    );
    assert_eq!(eval_over("(1 | 2).exclude(4).count()", None), integer("2"));
    assert_eq!(eval_over("(1 | 2).exclude({}).count()", None), integer("2"));
    assert_eq!(
        eval_over("(1 | 2 | 3).exclude(2 | 4) = 1 | 3", None),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.subset.012 -- `exclude` keeps duplicates that are not excluded
// ---------------------------------------------------------------------------

// covers: path.subset.012
#[test]
fn exclude_keeps_duplicates_that_are_not_excluded() {
    // `exclude` filters and never deduplicates: an item survives when no item
    // of the argument equals it, however many times it appears. That is the
    // difference between `funcExclude` (FHIRPathEngine.java:5074) and
    // `funcIntersect` (:5062), which does deduplicate.
    //
    // The suite's own case states it with `combine`, and that form holds under
    // the pinned engine.
    assert_eq!(
        eval_over("1.combine(1).exclude(2).count()", None),
        integer("2")
    );
    expect_case("testExclude", "testExclude4");

    // Contradiction with the pinned engine, recorded rather than silently
    // accommodated. The scenario writes the duplicate with the **union**
    // operator, which deduplicates before `exclude` ever runs: `funcUnion`
    // (:5036) adds an item only when it is not already present, which is what
    // `testUnion/testUnion3` -- `(1|1).count() = 1` -- grades. So
    // `(1 | 1 | 2)` is two items and `(1 | 1 | 2).exclude(2)` is one, not two.
    assert_eq!(
        eval_over("(1 | 1 | 2).count()", None),
        integer("2"),
        "the union operator deduplicates, which is the premise of the conflict"
    );
    assert_eq!(
        eval_over("(1 | 1 | 2).exclude(2).count()", None),
        integer("2"),
        "written to path.subset.012 as approved; the pinned engine answers 1, because the union removed the duplicate first"
    );
}

// ---------------------------------------------------------------------------
// path.subset.013 -- an unordered focus is rejected for all six functions
// ---------------------------------------------------------------------------

// covers: path.subset.013
#[test]
fn an_unordered_focus_is_rejected_for_all_six_position_dependent_functions() {
    let patient = load_input("patient-example.xml");

    // `checkOrdered` (FHIRPathEngine.java:3845) throws
    // `FHIRPATH_ORDERED_ONLY` whenever the focus's collection status is
    // UNORDERED, and it has exactly six call sites -- one per function:
    //
    //   item  :3364   first :3408   last :3412
    //   tail  :3416   skip  :3420   take :3425
    //
    // `children()` is the unordered focus the delta names.
    let unordered = [
        ("item (the indexer)", "Patient.children()[0]", "item"),
        ("first", "Patient.children().first()", "first"),
        ("last", "Patient.children().last()", "last"),
        ("tail", "Patient.children().tail()", "tail"),
        ("skip", "Patient.children().skip(1)", "skip"),
        ("take", "Patient.children().take(1)", "take"),
    ];
    for (label, expr, named) in unordered {
        let message = expect_check_failure(expr, &patient, StrictnessFlags::CHECK);
        assert!(
            message.contains(named),
            "the ordered-only error for {label} must name {named}: {message}"
        );
    }

    // Unconditional: no engine setting disables it, so the same six are
    // rejected under the evaluation strictness configuration too.
    for (_, expr, _) in unordered {
        expect_check_failure(expr, &patient, StrictnessFlags::EVALUATE);
    }

    // Only `skip` is graded by a live case; the other five are
    // differential-only, which is asserted here so the gap stays visible.
    expect_case("testDollar", "testDollarOrderNotAllowed");
    let (suite, _) = pinned();
    let ordered_cases = suite.cases().filter(|c| c.check_ordered_functions).count();
    assert_eq!(
        ordered_cases, 1,
        "the pinned suite carries exactly one checkOrderedFunctions case, so \
         five of the six sites have no live coverage"
    );

    // And an ordered focus is accepted for all six, so the rejection is about
    // the collection status and not about the functions.
    assert_eq!(eval_patient("Patient.name.first().count()"), integer("1"));
    assert_eq!(eval_patient("Patient.name.take(1).count()"), integer("1"));
}
