//! String functions: `path.string.001` through `path.string.024`.
//!
//! ```text
//! cargo test -p frevo-path --test path_string
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`. Fifteen scenarios name a suite
//! group and are driven through the committed harness; the rest state an
//! expression and its result.
//!
//! The type guard is the substance of this requirement, and it is narrower than
//! the delta's prose. `FHIR_TYPES_STRING` (FHIRPathEngine.java:2407) lists
//! exactly twelve types, and the guard
//! `focus.get(0).hasType(FHIR_TYPES_STRING) || doImplicitStringConversion`
//! appears on exactly **thirteen** functions:
//!
//!   replace :4849   replaceMatches :4875   endsWith :4903   matches :5691
//!   matchesFull :5716   contains :5750   length :5769   startsWith :5798
//!   lower :5811   upper :5822   toChars :5833   indexOf :5853
//!   substring :5877
//!
//! `trim` (:4523), `split` (:4532) and `join` (:4547) -- which the delta's
//! `path.string.020` lists among the guarded functions -- carry **no** such
//! guard: each reads `primitiveValue()` behind a size check only. That is
//! recorded at `path.string.020` rather than quietly implemented either way.
//!
//! `DEV-0006` resolves the guard's effect as HAPI's behaviour: a string
//! function over a non-string focus returns empty rather than failing.
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
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// The twelve string-like FHIR types
// ---------------------------------------------------------------------------

/// `FHIR_TYPES_STRING`, FHIRPathEngine.java:2407, verbatim and in order.
const FHIR_TYPES_STRING: [&str; 12] = [
    "string",
    "uri",
    "code",
    "oid",
    "id",
    "uuid",
    "sid",
    "markdown",
    "base64Binary",
    "canonical",
    "url",
    "xhtml",
];

// ---------------------------------------------------------------------------
// The settings seam `path.string.024` needs
//
// `doImplicitStringConversion` is an engine-wide switch, and the harness's
// `EngineConfig` does not carry it: it is off for every suite case and for both
// HAPI wrappers (`path.strictness.009`). So the one scenario that needs it on
// drives a local seam, with the engine still unbound.
// ---------------------------------------------------------------------------

/// The engine settings this file varies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StringSettings {
    /// `FHIRPathEngine.setDoImplicitStringConversion`. Off by default, which is
    /// what both HAPI wrappers leave it at.
    implicit_string_conversion: bool,
}

trait ConfigurableEngine {
    fn evaluate_with(
        &mut self,
        expr: &str,
        inp: Option<&SuiteInput>,
        settings: StringSettings,
    ) -> Result<Vec<ResultItem>, EngineError>;
}

fn configurable_engine_under_test() -> Box<dyn ConfigurableEngine> {
    Box::new(UnboundConfigurable)
}

struct UnboundConfigurable;

impl ConfigurableEngine for UnboundConfigurable {
    fn evaluate_with(
        &mut self,
        _expr: &str,
        _inp: Option<&SuiteInput>,
        _settings: StringSettings,
    ) -> Result<Vec<ResultItem>, EngineError> {
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

/// Assert the group's cases are graded as HAPI's own recorded run grades them
/// (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` except the named
/// cases, which must be `failed` (parity under `path.oracle.012`). `did-not-run`
/// satisfies neither, so an unbuilt feature never passes vacuously.
fn expect_group_except(group: &str, expected: usize, failing: &[&str]) {
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
    for name in failing {
        assert!(
            g.cases.iter().any(|c| c.address.name == *name),
            "{group}/{name} must be a case of the group"
        );
    }
    let mut problems = Vec::new();
    for case in &g.cases {
        let r = report
            .get(&case.address.to_string())
            .unwrap_or_else(|| panic!("{} must appear in the run report", case.address));
        let want = if failing.iter().any(|n| case.address.name == *n) {
            CaseStatus::Failed
        } else {
            CaseStatus::Passed
        };
        if r.status != want {
            problems.push(format!(
                "{} ({:?}): {} (wanted {}) -- {}",
                case.address,
                case.expression,
                r.status.as_str(),
                want.as_str(),
                r.reason
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {} cases in {group} are not graded as HAPI grades them; the first few:\n  {}",
        problems.len(),
        g.cases.len(),
        problems[..problems.len().min(5)].join("\n  ")
    );
}

/// `expr` evaluated with the static check **skipped**.
fn eval_unchecked(expr: &str) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = context_for(None);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .evaluate(parsed, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate with the check skipped: {e}"))
}

/// `expr` must fail to **parse** with a real diagnostic.
fn expect_parse_error(expr: &str) -> String {
    let mut engine = fresh_engine();
    match engine.parse(expr) {
        Ok(_) => panic!("{expr:?} must fail to parse, but it parsed"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail to parse with a diagnostic, not: {e}"),
    }
}

/// `expr` must fail the static check with a real diagnostic.
fn expect_check_failure(expr: &str) -> String {
    let mut engine = fresh_engine();
    let context = context_for(None);
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    match engine.check(parsed, &context, StrictnessFlags::CHECK) {
        Ok(()) => panic!("{expr:?} must fail the static check, but the check passed"),
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("{expr:?} must fail the check with a diagnostic, not: {e}"),
    }
}

/// Assert every case in `group` whose name contains `needle` passed, and that
/// the subset still has the size this test was written against.
fn expect_cases_matching(group: &str, needle: &str, expected: usize) {
    let (suite, report) = pinned();
    let g = suite
        .groups
        .iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("the pinned suite has a group named {group}"));
    let selected: Vec<_> = g
        .cases
        .iter()
        .filter(|c| c.address.name.contains(needle))
        .collect();
    assert_eq!(
        selected.len(),
        expected,
        "{group} must have {expected} case(s) whose name contains {needle:?}"
    );
    for case in selected {
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

fn eval(expr: &str) -> Vec<ResultItem> {
    eval_over(expr, None)
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

fn primitive(fhir_type: &str, value: &str) -> Vec<ResultItem> {
    vec![ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }]
}

fn string(value: &str) -> Vec<ResultItem> {
    primitive("string", value)
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
// path.string.001 -- `upper` and `lower`
// ---------------------------------------------------------------------------

// covers: path.string.001
#[test]
fn upper_and_lower_change_case() {
    expect_group("testCase", 4);

    // `funcUpper` :5820 and `funcLower` :5809, both behind the string guard.
    assert_eq!(eval("'t'.upper()"), string("T"));
    assert_eq!(eval("'T'.lower()"), string("t"));
    assert_eq!(eval("'T'.upper()"), string("T"));
    assert_eq!(eval("'t'.lower()"), string("t"));

    // The result is a `string` whatever the focus's string-like type was, which
    // is what makes `Patient.gender.upper()` comparable with a literal.
    assert_eq!(eval_patient("Patient.gender.upper()"), string("MALE"));
}

// ---------------------------------------------------------------------------
// path.string.002 -- `toChars`
// ---------------------------------------------------------------------------

// covers: path.string.002
#[test]
fn to_chars_splits_a_string_into_single_characters() {
    expect_group("testToChars", 1);

    // `funcToChars` (FHIRPathEngine.java:5831) appends one `string` per
    // character, so the count is the length and each item is one character.
    assert_eq!(eval("'t2'.toChars() = 't' | '2'"), boolean(true));
    assert_eq!(
        eval("'abc'.toChars()"),
        vec![
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "a".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "b".into()
            },
            ResultItem::Primitive {
                fhir_type: "string".into(),
                value: "c".into()
            },
        ]
    );
    assert_eq!(eval("'abc'.toChars().count()"), integer("3"));
    assert_eq!(eval("''.toChars()"), empty());

    // One item per UTF-16 code unit (`for (char c : s.toCharArray())`,
    // FHIRPathEngine.java:5835): `'a😀'` (U+1F600) yields THREE items, `a`, the
    // high surrogate U+D83D and the low surrogate U+DE00, each a one-unit
    // string. A lone surrogate cannot be held in a Rust `String`, so the two
    // halves are asserted through FHIRPath: three items, the first is `a`, and
    // the two halves are distinct one-unit strings.
    assert_eq!(eval("'a😀'.toChars().count()"), integer("3"));
    assert_eq!(eval("'a😀'.toChars()[0]"), string("a"));
    assert_eq!(eval("'a😀'.toChars()[1].length()"), integer("1"));
    assert_eq!(eval("'a😀'.toChars()[2].length()"), integer("1"));
    assert_eq!(
        eval("'a😀'.toChars()[1] = 'a😀'.toChars()[2]"),
        boolean(false),
        "the high and low surrogate halves are different one-unit strings"
    );
}

// ---------------------------------------------------------------------------
// path.string.003 -- `indexOf`
// ---------------------------------------------------------------------------

// covers: path.string.003
#[test]
fn index_of_returns_a_zero_based_position_or_minus_one() {
    expect_group("testIndexOf", 6);

    // `funcIndexOf` (FHIRPathEngine.java:5842): zero-based, -1 when absent, and
    // 0 for the empty needle -- all three are graded by the group.
    assert_eq!(eval("'LogicalModel-Person'.indexOf('-')"), integer("12"));
    assert_eq!(eval("'LogicalModel-Person'.indexOf('z')"), integer("-1"));
    assert_eq!(eval("'LogicalModel-Person'.indexOf('')"), integer("0"));

    // An empty focus or an empty argument is empty, not -1: the group grades
    // all three combinations.
    assert_eq!(eval("'LogicalModel-Person'.indexOf({})"), empty());
    assert_eq!(eval("{}.indexOf('-')"), empty());
    assert_eq!(eval("{}.indexOf({})"), empty());
}

// ---------------------------------------------------------------------------
// path.string.004 -- `substring`
// ---------------------------------------------------------------------------

// covers: path.string.004
#[test]
fn substring_takes_a_start_and_an_optional_length() {
    // HAPI passes 8 of 16 (`hapi-8.12.0-results.json:352-359`): required
    // `failed` (parity, `path.oracle.012`) for `testSubstring11` to `18`,
    // `passed` for the rest (DEV-0019).
    expect_group_except(
        "testSubstring",
        16,
        &[
            "testSubstring11",
            "testSubstring12",
            "testSubstring13",
            "testSubstring14",
            "testSubstring15",
            "testSubstring16",
            "testSubstring17",
            "testSubstring18",
        ],
    );

    // The eight failures, in `funcSubstring` (FHIRPathEngine.java:5864-5895).
    // 11, 12: the start argument has no emptiness guard (:5867), an
    // index-out-of-bounds failure, so evaluation fails.
    expect_evaluation_failure("{}.substring({})", None);
    expect_evaluation_failure("'string'.substring({})", None);
    // 13, 14, 15: the end is `min(length, start + count)` with an `int` sum
    // (:5884), which falls before the start or overflows, so evaluation fails.
    expect_evaluation_failure("'abcdefg'.substring(3, -1)", None);
    expect_evaluation_failure("'abcdefg'.substring(0, -1)", None);
    expect_evaluation_failure("'abcdefg'.substring(1, 2147483647)", None);
    // 16: a zero count's empty result is dropped (:5888), so the answer is
    // empty and `= ''` is empty, not true.
    assert_eq!(eval("'abcdefg'.substring(3, 0)"), empty());
    assert_eq!(eval("'abcdefg'.substring(3, 0) = ''"), empty());
    // 17, 18: an empty length argument returns empty (:5871-5872), not the rest
    // of the string.
    assert_eq!(eval("'abcdefg'.substring(3, {})"), empty());
    assert_eq!(eval("'abcdefg'.substring(0, {})"), empty());

    // `funcSubstring` (FHIRPathEngine.java:5864): one argument takes the rest
    // of the string, two arguments clamp the end at the string's length.
    assert_eq!(eval("'12345'.substring(2)"), string("345"));
    assert_eq!(eval("'12345'.substring(2, 1)"), string("3"));
    assert_eq!(eval("'12345'.substring(2, 5)"), string("345"));
    assert_eq!(eval("'12345'.substring(0)"), string("12345"));

    // A non-integer or empty length yields empty rather than being ignored
    // (:5871-5873).
    assert_eq!(eval("'12345'.substring(2, {})"), empty());
}

// ---------------------------------------------------------------------------
// path.string.005 -- `substring` with an out-of-range start
// ---------------------------------------------------------------------------

// covers: path.string.005
#[test]
fn substring_with_an_out_of_range_start_yields_empty() {
    // `if (i1 < 0 || i1 >= sw.length()) { return new ArrayList<Base>(); }`
    // (FHIRPathEngine.java:5879), so both ends are empty rather than clamped
    // and neither is an error.
    assert_eq!(eval("'abc'.substring(5)"), empty());
    assert_eq!(eval("'abc'.substring(-1)"), empty());

    // The boundary: index 2 is the last character, index 3 is out of range.
    assert_eq!(eval("'abc'.substring(2)"), string("c"));
    assert_eq!(eval("'abc'.substring(3)"), empty());
    assert_eq!(eval("'abc'.substring(5).empty()"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.string.006 -- `startsWith` and `endsWith`
// ---------------------------------------------------------------------------

// covers: path.string.006
#[test]
fn starts_with_and_ends_with_test_affixes() {
    expect_group("testStartsWith", 12);
    expect_group("testEndsWith", 10);

    // `funcStartsWith` :5787 and `funcEndsWith` :4892.
    assert_eq!(eval("'12345'.startsWith('1')"), boolean(true));
    assert_eq!(eval("'12345'.startsWith('2')"), boolean(false));
    assert_eq!(eval("'12345'.endsWith('5')"), boolean(true));
    assert_eq!(eval("'12345'.endsWith('4')"), boolean(false));

    // The empty affix is true, a longer affix is false, and an empty focus is
    // empty rather than false -- the three edges the two groups grade.
    assert_eq!(eval("'12345'.startsWith('')"), boolean(true));
    assert_eq!(eval("'12345'.startsWith('123456')"), boolean(false));
    assert_eq!(eval("{}.startsWith('1')"), empty());
    assert_eq!(eval("''.startsWith('')"), boolean(true));

    // One case in `testStartsWith` is `invalid="semantic"`: a non-string focus
    // is rejected by the check pass, which is a different stage from the empty
    // answer `path.string.020` is about.
    let (suite, _) = pinned();
    assert!(
        suite
            .case("testStartsWith", "testStartsWithNonString1")
            .expect("the case resolves")
            .expected_failure
            .is_some()
    );
}

// ---------------------------------------------------------------------------
// path.string.007 -- `contains` tests substring presence
// ---------------------------------------------------------------------------

// covers: path.string.007
#[test]
fn contains_tests_substring_presence() {
    expect_group("testContainsString", 10);

    // `funcContains` (FHIRPathEngine.java:5739) is the *string* function, and
    // it is reached only when the focus is a string-like type: the collection
    // operator `contains` is `opContains` (:2655) and a different code path
    // entirely (`path.combine.005`).
    assert_eq!(eval("'12345'.contains('3')"), boolean(true));
    assert_eq!(eval("'12345'.contains('6')"), boolean(false));
    assert_eq!(eval("'12345'.contains('')"), boolean(true));
    assert_eq!(eval("{}.contains('3')"), empty());
    assert_eq!(eval("'12345'.contains({})"), empty());
}

// ---------------------------------------------------------------------------
// path.string.008 -- `matches` is unanchored
// ---------------------------------------------------------------------------

// covers: path.string.008
#[test]
fn matches_tests_a_regular_expression_unanchored() {
    expect_group("testMatches", 16);

    // `funcMatches` (FHIRPathEngine.java:5683) calls `Pattern.matcher(...)
    // .find()`, so the pattern may match anywhere in the string.
    assert_eq!(eval("'12345'.matches('^[0-9]+$')"), boolean(true));
    assert_eq!(eval("'abc'.matches('b')"), boolean(true));
    assert_eq!(eval("'abc'.matches('^b$')"), boolean(false));
    assert_eq!(eval("{}.matches('b')"), empty());

    // `matches` prefixes the pattern with `(?s)` (FHIRPathEngine.java:5698), so
    // `.` matches a line terminator.
    assert_eq!(eval("'a\\nb'.matches('a.b')"), boolean(true));

    // The edges of `funcMatches` (:5683-5707). An empty pattern COLLECTION
    // yields an empty collection (:5688); an empty pattern STRING yields
    // `false` (:5705-5706); an empty focus STRING yields `false` (:5693-5694).
    assert_eq!(eval("'abc'.matches({})"), empty());
    assert_eq!(eval("'abc'.matches('')"), boolean(false));
    assert_eq!(eval("''.matches('a')"), boolean(false));
    // A focus that is not string-like, with a non-empty pattern, is empty
    // (DEV-0006); evaluated with the check skipped, which rejects the focus.
    assert_eq!(eval_unchecked("1.matches('1')"), empty());
    // A non-string pattern is converted to its string: the pattern `1` against
    // `abc` is false.
    assert_eq!(eval_unchecked("'abc'.matches(1)"), boolean(false));
    // A focus of more than one item is `false`.
    assert_eq!(eval_unchecked("('a' | 'b').matches('a')"), boolean(false));
    // A pattern Java cannot compile is an execution error (an unbalanced
    // parenthesis is one in HAPI too). HAPI's 500 ms regex timeout
    // (`Timeout evaluating regex`, :5699-5700) is not testable here: Frevo has
    // no wall-clock timeout (DEV-0023), so that edge is differential-only.
    expect_evaluation_failure("'a'.matches('(a')", None);
}

// ---------------------------------------------------------------------------
// path.string.009 -- `matchesFull` requires the whole string
// ---------------------------------------------------------------------------

// covers: path.string.009
#[test]
fn matches_full_requires_the_whole_string_to_match() {
    // `funcMatchesFull` (FHIRPathEngine.java:5711) calls `matcher(...)
    // .matches()` rather than `.find()`, which is the whole difference between
    // the two functions.
    assert_eq!(eval("'abc'.matches('b')"), boolean(true));
    assert_eq!(eval("'abc'.matchesFull('b')"), boolean(false));

    // The full pattern matches under both.
    assert_eq!(eval("'abc'.matchesFull('abc')"), boolean(true));
    assert_eq!(eval("'abc'.matches('abc')"), boolean(true));
    assert_eq!(eval("'abc'.matchesFull('a.c')"), boolean(true));

    // `funcMatchesFull` (:5711-5735) has NO empty-propagation branch, unlike
    // `matches`: `{}.matchesFull('b')` is `false`, not empty, because
    // `focus.size() == 1` is false and the else arm adds `false` (:5733-5734).
    // The FHIRPath specification yields empty; DEV-0023.
    assert_eq!(eval("{}.matchesFull('b')"), boolean(false));
    // An empty pattern collection and an empty pattern string are `false` too,
    // as is an empty focus string.
    assert_eq!(eval("'abc'.matchesFull({})"), boolean(false));
    assert_eq!(eval("'abc'.matchesFull('')"), boolean(false));
    assert_eq!(eval("''.matchesFull('a')"), boolean(false));
    // `matchesFull` prefixes `(?s)` too (RegexTimeout.matches, :5725).
    assert_eq!(eval("'a\\nb'.matchesFull('a.b')"), boolean(true));
    // A non-string focus is empty (DEV-0006); the check rejects the focus.
    assert_eq!(eval_unchecked("1.matchesFull('1')"), empty());
}

// ---------------------------------------------------------------------------
// path.string.010 -- `replaceMatches`
// ---------------------------------------------------------------------------

// covers: path.string.010
#[test]
fn replace_matches_substitutes_regular_expression_matches() {
    expect_group("testReplaceMatches", 7);

    // `funcReplaceMatches` (FHIRPathEngine.java:4865) is
    // `String.replaceAll(regex, replacement)`, so the pattern is a regular
    // expression and every match is replaced.
    assert_eq!(eval("'12345'.replaceMatches('234', 'x')"), string("1x5"));
    assert_eq!(eval("'a1b2'.replaceMatches('[0-9]', 'x')"), string("axbx"));
    assert_eq!(eval("{}.replaceMatches('2', 'x')"), empty());

    // `funcReplaceMatches` (:4865-4888): an empty pattern or replacement
    // collection is empty (:4872); an empty pattern STRING returns the focus
    // unchanged (`testReplaceMatches2`, :4885-4886); a focus of more than one
    // item returns the FIRST item's string with no error; a non-string focus
    // with a non-empty pattern is empty (DEV-0006).
    assert_eq!(eval("'abc'.replaceMatches({}, 'x')"), empty());
    assert_eq!(eval("'abc'.replaceMatches('a', {})"), empty());
    assert_eq!(eval("'abc'.replaceMatches('', 'x')"), string("abc"));
    assert_eq!(
        eval_unchecked("('a' | 'b').replaceMatches('a', 'x')"),
        string("a")
    );
    assert_eq!(eval_unchecked("1.replaceMatches('1', 'x')"), empty());
    // `replaceMatches` does NOT add the `(?s)` prefix, so `.` does not match a
    // line terminator in it (:4879): the string is unchanged.
    assert_eq!(eval("'a\\nb'.replaceMatches('a.b', 'x')"), string("a\nb"));
}

// ---------------------------------------------------------------------------
// path.string.011 -- `replace` substitutes literal substrings
// ---------------------------------------------------------------------------

// covers: path.string.011
#[test]
fn replace_substitutes_literal_substrings() {
    expect_group("testReplace", 6);

    // `funcReplace` (FHIRPathEngine.java:4839) replaces a literal substring, so
    // a regular-expression metacharacter in the pattern is matched literally --
    // which is the whole difference from `replaceMatches`.
    assert_eq!(eval("'12345'.replace('234', 'x')"), string("1x5"));
    assert_eq!(eval("'a.b'.replace('.', 'x')"), string("axb"));
    assert_eq!(eval("'a.b'.replaceMatches('.', 'x')"), string("xxx"));
    assert_eq!(eval("{}.replace('2', 'x')"), empty());

    // Java's `String.replace` (FHIRPathEngine.java:4854), no pattern: an empty
    // target inserts the replacement between every character and at both ends.
    assert_eq!(eval("'abc'.replace('', '-')"), string("-a-b-c-"));
    // An empty focus string yields `''` whatever the arguments (:4851-4852).
    assert_eq!(eval("''.replace('a', 'b')"), string(""));
    // An empty focus collection or an empty argument yields empty (:4846).
    assert_eq!(eval("'abc'.replace({}, 'x')"), empty());
    assert_eq!(eval("'abc'.replace('a', {})"), empty());
    // A focus of more than one item is an execution error (:4859).
    expect_evaluation_failure("('a' | 'b').replace('a', 'x')", None);
}

// ---------------------------------------------------------------------------
// path.string.012 -- `length` counts characters
// ---------------------------------------------------------------------------

// covers: path.string.012
#[test]
fn length_counts_characters() {
    expect_group("testLength", 6);

    // `funcLength` (FHIRPathEngine.java:5767) is `convertToString(focus).length()`
    // behind the string guard.
    assert_eq!(eval("'123456'.length()"), integer("6"));
    assert_eq!(eval("''.length()"), integer("0"));
    assert_eq!(eval("{}.length()"), empty());

    // Zero and empty are different answers, which is what the group's last two
    // cases separate.
    assert_ne!(eval("{}.length()"), integer("0"));
}

// ---------------------------------------------------------------------------
// path.string.013 -- `trim`
// ---------------------------------------------------------------------------

// covers: path.string.013
#[test]
fn trim_removes_leading_and_trailing_whitespace() {
    expect_group("testTrim", 6);

    // `funcTrim` (FHIRPathEngine.java:4523) is `String.trim()` behind a size
    // check -- and, unlike the thirteen guarded functions, with no
    // `FHIR_TYPES_STRING` test at all (recorded at `path.string.020`).
    assert_eq!(eval("' 123456 '.trim()"), string("123456"));
    assert_eq!(eval("'123 456'.trim()"), string("123 456"));
    assert_eq!(eval("'      '.trim()"), string(""));
    assert_eq!(eval("{}.trim()"), empty());

    // Interior whitespace is kept, so the length after trimming is 7 and not 6.
    assert_eq!(eval("'123 456'.trim().length()"), integer("7"));

    // The characters removed are those Java's `String.trim` removes: every
    // character up to and including U+0020 (FHIRPathEngine.java:4527), so a
    // U+00A0 no-break space is NOT removed.
    assert_eq!(eval("'\t\n x \r'.trim()"), string("x"));
    assert_eq!(
        eval("'\u{a0}x\u{a0}'.trim().length()"),
        integer("3"),
        "a no-break space is not trimmed"
    );
}

// ---------------------------------------------------------------------------
// path.string.014 -- `split` keeps empty segments
// ---------------------------------------------------------------------------

// covers: path.string.014
#[test]
fn split_keeps_empty_segments() {
    expect_group("testSplit", 8);

    // `funcSplit` (FHIRPathEngine.java:4532) uses `Utilities.simpleSplit`,
    // which keeps an empty segment for every adjacent pair of separators and
    // for a leading or trailing one. `'A,,C'` is therefore three segments, and
    // joining them back reproduces the input.
    assert_eq!(eval("'A,,C'.split(',').count()"), integer("3"));
    assert_eq!(eval("'A,,C'.split(',').join(',') = 'A,,C'"), boolean(true));

    // The nine-segment `[stop]` case: a multi-character separator, with leading,
    // adjacent and trailing occurrences.
    assert_eq!(
        eval(
            "'[stop]ONE[stop][stop]TWO[stop][stop][stop]THREE[stop][stop]'.split('[stop]').count()"
        ),
        integer("9")
    );
    expect_case("testSplit", "testSplit3");
    expect_case("testSplit", "testSplit4");

    // And the ordinary cases: five names, and a separator that occurs once.
    assert_eq!(
        eval("'Peter,James,Jim,Peter,James'.split(',').count()"),
        integer("5")
    );
    assert_eq!(eval("'fr-BE'.split('-').first()"), string("fr"));
    assert_eq!(eval("'fr-BE'.split('-').last()"), string("BE"));

    // `split` is `Utilities.simpleSplit` (Utilities.java:1978-1996): the
    // separator is a literal, not a pattern, and a trailing separator yields a
    // trailing empty segment.
    assert_eq!(eval("'a.b'.split('.').count()"), integer("2"));
    assert_eq!(eval("'a,'.split(',').count()"), integer("2"));

    // An empty separator: HAPI never terminates (`simpleSplit` appends an empty
    // segment at a zero-length match without advancing, Utilities.java:
    // 1985-1994), so there is no HAPI result. Frevo returns an execution error
    // as `path.errors.004` and `path.errors.003` require (DEV-0022); a
    // differential test grades Frevo's error alone.
    let message = expect_evaluation_failure("'abc'.split('')", None);
    assert!(
        !message.is_empty(),
        "the empty-separator error must carry a message"
    );
}

// ---------------------------------------------------------------------------
// path.string.015 -- `join`
// ---------------------------------------------------------------------------

// covers: path.string.015
#[test]
fn join_concatenates_with_a_separator_and_is_empty_for_an_empty_collection() {
    // `testJoinEmpty` is `failed` in HAPI (`hapi-8.12.0-results.json:457`) and
    // required `failed` here (parity, `path.oracle.012`); `testJoin` passes.
    expect_group_except("testJoin", 2, &["testJoinEmpty"]);

    assert_eq!(
        eval_patient("name.given.join(',')"),
        string("Peter,James,Jim,Peter,James")
    );

    // `funcJoin` (FHIRPathEngine.java:4547-4565) ends with
    // `result.add(new StringType(b.toString()))` outside any emptiness test, so
    // an empty focus yields ONE empty string, not an empty collection. The
    // FHIRPath specification yields empty; DEV-0022, resolved as HAPI's
    // behaviour. A Frevo pass on `testJoinEmpty` would be `ahead-of-hapi`.
    assert_eq!(eval_patient("name.prefix.join(',')"), string(""));
    assert_ne!(eval_patient("name.prefix.join(',')"), empty());

    // A separator of more than one character, and the no-argument form.
    assert_eq!(eval("('a' | 'b').join('--')"), string("a--b"));
    assert_eq!(eval("('a' | 'b').join()"), string("ab"));

    // `join` takes zero or one argument: `checkParameters` allows between 0 and
    // 1 (FHIRPathEngine.java:1427, `checkParamCount` :1355-1359), so a second
    // argument is a PARSE failure, never reaching the check or evaluation. The
    // branch inside `funcJoin` for a second argument (:4553-4556) is
    // unreachable.
    let message = expect_parse_error("(1 | 2 | 3).join(',', ' and ')");
    assert!(
        message.contains("The function \"join\" requires between 0 and 1 parameters"),
        "unexpected parse message: {message:?}"
    );
}

// ---------------------------------------------------------------------------
// path.string.016 -- `encode`
// ---------------------------------------------------------------------------

// covers: path.string.016
#[test]
fn encode_supports_base64_urlbase64_and_hex() {
    expect_cases_matching("testEncodeDecode", "Encode", 4);

    // `funcEncode` (FHIRPathEngine.java:4440) has exactly three arms: `hex`,
    // `base64` and `urlbase64`.
    assert_eq!(eval("'test'.encode('base64')"), string("dGVzdA=="));
    assert_eq!(eval("'test'.encode('hex')"), string("74657374"));

    // `base64` and `urlbase64` differ in the 62nd and 63rd alphabet
    // characters, which is why the suite grades the same input through both:
    // `/` becomes `_`.
    assert_eq!(
        eval("'subjects?_d'.encode('base64')"),
        string("c3ViamVjdHM/X2Q=")
    );
    assert_eq!(
        eval("'subjects?_d'.encode('urlbase64')"),
        string("c3ViamVjdHM_X2Q=")
    );

    // Hex is upper case: `bytesToHex` (FHIRPathEngine.java:4425) indexes
    // `"0123456789ABCDEF"`.
    assert_eq!(eval("'z'.encode('hex')"), string("7A"));
}

// ---------------------------------------------------------------------------
// path.string.017 -- `decode`
// ---------------------------------------------------------------------------

// covers: path.string.017
#[test]
fn decode_reverses_encode_for_each_supported_form() {
    expect_cases_matching("testEncodeDecode", "Decode", 4);

    assert_eq!(eval("'dGVzdA=='.decode('base64')"), string("test"));
    assert_eq!(eval("'74657374'.decode('hex')"), string("test"));
    assert_eq!(
        eval("'c3ViamVjdHM/X2Q='.decode('base64')"),
        string("subjects?_d")
    );
    assert_eq!(
        eval("'c3ViamVjdHM_X2Q='.decode('urlbase64')"),
        string("subjects?_d")
    );

    // Round-tripping is the property the pairing exists to state.
    assert_eq!(
        eval("'subjects?_d'.encode('urlbase64').decode('urlbase64') = 'subjects?_d'"),
        boolean(true)
    );
}

// ---------------------------------------------------------------------------
// path.string.018 -- an unsupported encode/decode format
// ---------------------------------------------------------------------------

// covers: path.string.018
#[test]
fn an_unsupported_encode_format_behaves_as_the_pinned_engine_does() {
    // The scenario asks for the pinned engine's behaviour and for the harness
    // to record which behaviour that is. It is **empty, not an error**:
    // `funcEncode` (FHIRPathEngine.java:4446-4457) tests the parameter against
    // `hex`, `base64` and `urlbase64` and falls through to `return result` with
    // nothing appended when none matches. `funcDecode` (:4461) has the same
    // shape.
    assert_eq!(eval("'x'.encode('rot13')"), empty());
    assert_eq!(eval("'x'.decode('rot13')"), empty());
    assert_eq!(eval("'x'.encode('')"), empty());

    // `'!!'.decode('base64')` fails with an execution error:
    // `Base64.Decoder.decode` raises `IllegalArgumentException` on malformed
    // input. Text is converted to bytes with the JVM default charset in HAPI;
    // Frevo uses UTF-8, identical wherever the default is UTF-8.
    expect_evaluation_failure("'!!'.decode('base64')", None);

    // Empty specifically, and not a failure: the distinction is the whole
    // scenario, so an implementation that raised here would be wrong even
    // though the result is unusable either way.
    assert_eq!(eval("'x'.encode('rot13').empty()"), boolean(true));

    // `escape`/`unescape` have the same fall-through, with four arms rather
    // than three (`html`, `json`, `url`, `md`; :4481 and :4502).
    assert_eq!(eval("'x'.escape('rot13')"), empty());
    assert_eq!(eval("'x'.unescape('rot13')"), empty());
}

// ---------------------------------------------------------------------------
// path.string.019 -- `escape` and `unescape`
// ---------------------------------------------------------------------------

// covers: path.string.019
#[test]
fn escape_and_unescape_support_html_and_json() {
    // HAPI fails three of the seven (`hapi-8.12.0-results.json:439-441`):
    // required `failed` (parity, `path.oracle.012`), `passed` for the rest
    // (DEV-0022).
    expect_group_except(
        "testEscapeUnescape",
        7,
        &[
            "testUnescapeJson2",
            "testUnescapeJson3",
            "testUnescapeJson4",
        ],
    );

    // `funcEscape` :4481 delegates to `Utilities.escapeXml` for `html` and
    // `Utilities.escapeJson` for `json`; `funcUnescape` :4502 is the inverse
    // pair.
    assert_eq!(
        eval("'\"1<2\"'.escape('html')"),
        string("&quot;1&lt;2&quot;")
    );
    assert_eq!(
        eval("'&quot;1&lt;2&quot;'.unescape('html')"),
        string("\"1<2\"")
    );

    // `testUnescapeJson` (the literal `'\"1<2\"'`, which the lexer has already
    // turned into `"1<2"`) passes.
    assert_eq!(eval("'\\\"1<2\\\"'.unescape('json')"), string("\"1<2\""));

    // The three failures come from `Utilities.unescapeJson`
    // (Utilities.java:481-525), and Frevo matches them (DEV-0022).
    //
    // `case '"'` appends the letter `b`, not a quote (:492-494): the escaped
    // form `\"1<2\"` unescapes to `b1<2b`. That is `testUnescapeJson2`, whose
    // escape step produces exactly that string, and `testUnescapeJson4`, whose
    // FHIRPath literal `'\\"1<2\\"'` is that string.
    assert_eq!(
        eval("'\"1<2\"'.escape('json').unescape('json')"),
        string("b1<2b")
    );
    assert_eq!(eval("'\\\\\"1<2\\\\\"'.unescape('json')"), string("b1<2b"));
    // After `\u` the cursor advances one place and not past the four hex
    // digits (:516-519, :525), so they are appended as text: the FHIRPath
    // literal `'\\u0041B'` (a backslash then `u0041B`) unescapes to `A0041B`,
    // not `AB` (`testUnescapeJson3`).
    assert_eq!(eval("'\\\\u0041B'.unescape('json')"), string("A0041B"));
    // An unknown escape such as `\x` raises `Unknown JSON escape` (:521), and a
    // trailing lone backslash raises an index error: execution errors.
    expect_evaluation_failure("'\\\\x'.unescape('json')", None);
    expect_evaluation_failure("'a\\\\'.unescape('json')", None);

    // `url` and `md` are also accepted (FHIRPathEngine.java:4492-4495,
    // 4513-4516) and any other format yields empty. No pinned suite case grades
    // either, so everything below is DIFFERENTIAL-ONLY, written against the
    // spec's statement of the pinned source.
    //
    // `escape('url')` is Java's `URLEncoder.encode(s, "UTF-8")`: letters,
    // digits and `.` `-` `*` `_` are kept, a space is `+`, everything else is
    // its UTF-8 bytes as upper-case `%XX` (`~` is encoded).
    assert_eq!(
        eval("'a b&c=d/\u{e9}~'.escape('url')"),
        string("a+b%26c%3Dd%2F%C3%A9%7E")
    );
    // `unescape('url')` is `URLDecoder.decode`: `+` is a space, `%XX` decodes
    // as UTF-8, and a malformed `%` sequence is an execution error.
    assert_eq!(eval("'a+b%20c'.unescape('url')"), string("a b c"));
    expect_evaluation_failure("'%zz'.unescape('url')", None);
    // `escape('md')` inserts a backslash before exactly seven characters, `*`
    // `&` `#` `[` `>` `<` and the backtick, and before nothing else.
    assert_eq!(
        eval("'a*b&c#d[e>f<g`h]_()'.escape('md')"),
        string("a\\*b\\&c\\#d\\[e\\>f\\<g\\`h]_()")
    );
    // `unescape('md')` drops a backslash only when the next character is one
    // of those seven and the backslash is not the last character.
    assert_eq!(eval("'a\\\\*b'.unescape('md')"), string("a*b"));
    assert_eq!(eval("'a\\\\xb'.unescape('md')"), string("a\\xb"));
    assert_eq!(eval("'a\\\\'.unescape('md')"), string("a\\"));
    assert_eq!(eval("'\\\\\\\\*'.unescape('md')"), string("\\*"));
}

// ---------------------------------------------------------------------------
// path.string.020 -- a string function on a non-string yields empty (DEV-0006)
// ---------------------------------------------------------------------------

// covers: path.string.020
#[test]
fn a_string_function_on_a_non_string_focus_yields_empty() {
    // `FHIR_TYPES_STRING` is exactly these twelve, in this order
    // (FHIRPathEngine.java:2407). The list is the scenario's, so it is asserted
    // rather than described.
    assert_eq!(FHIR_TYPES_STRING.len(), 12);
    assert_eq!(
        FHIR_TYPES_STRING,
        [
            "string",
            "uri",
            "code",
            "oid",
            "id",
            "uuid",
            "sid",
            "markdown",
            "base64Binary",
            "canonical",
            "url",
            "xhtml"
        ]
    );

    // An integer focus is not one of them, so the function appends nothing.
    // `DEV-0006` resolves this as HAPI's behaviour: the FHIRPath specification
    // makes a type-mismatched focus an error.
    // Evaluated with the static check skipped: the check itself rejects a
    // string function over an integer focus (`checkContextString`), so the
    // empty answer is the evaluation's.
    assert_eq!(eval_unchecked("1.startsWith('1')"), empty());
    assert_eq!(eval_unchecked("1.startsWith('1').empty()"), boolean(true));

    // The thirteen functions that carry the guard, each over an integer focus.
    for expr in [
        "1.upper()",
        "1.lower()",
        "1.contains('1')",
        "1.endsWith('1')",
        "1.substring(0)",
        "1.indexOf('1')",
        "1.replace('1', 'x')",
        "1.length()",
        "1.toChars()",
        "1.startsWith('1')",
    ] {
        assert_eq!(
            eval_unchecked(expr),
            empty(),
            "{expr} must yield empty with the check skipped"
        );
    }

    // A string-like but non-`string` focus **is** accepted: `code` is in the
    // list, which is why `Patient.gender.upper()` works.
    assert_eq!(
        eval_patient("Patient.gender.startsWith('m')"),
        boolean(true)
    );
}

// covers: path.string.020
#[test]
fn the_regex_functions_also_yield_empty_on_a_non_string_focus() {
    // The three regex-dependent functions of the same guard (the regular
    // expression engine is task 41a): kept in a test of their own so that a
    // did-not-run here cannot hide the other `path.string.020` assertions.
    for expr in [
        "1.matches('1')",
        "1.matchesFull('1')",
        "1.replaceMatches('1', 'x')",
    ] {
        assert_eq!(
            eval_unchecked(expr),
            empty(),
            "{expr} must yield empty with the check skipped"
        );
    }
}

// covers: path.string.020
#[test]
fn trim_split_and_join_do_not_check_the_focus_type() {
    // `trim`, `split` and `join` do NOT check the focus type
    // (FHIRPathEngine.java:4523-4529, 4532-4545, 4547-4565): they use the
    // item's primitive value, so an integer focus is converted, not rejected.
    assert_eq!(eval("1.trim()"), string("1"));
    assert_eq!(eval("1.split(',')"), string("1"));
    assert_eq!(eval("1.join(',')"), string("1"));

    // On a COMPLEX (non-primitive) focus the primitive value is null, and the
    // static check passes it (:3622-3630):
    let patient = load_input("patient-example.xml");
    //  - `trim` fails at evaluation with a NullPointerException on
    //    `cnt.trim()` (:4527): an execution error in Frevo;
    expect_evaluation_failure("Patient.name.first().trim()", Some(&patient));
    //  - `split` yields an empty collection (`simpleSplit` returns no segments
    //    for a null string, Utilities.java:1979-1981);
    assert_eq!(
        eval_over("Patient.name.first().split(',')", Some(&patient)),
        empty()
    );
    //  - `join` yields one empty string: each null item is skipped, with no
    //    separator added for it.
    assert_eq!(
        eval_over("Patient.name.join(',')", Some(&patient)),
        string("")
    );
}

// ---------------------------------------------------------------------------
// path.string.021 -- a string function on an empty collection
// ---------------------------------------------------------------------------

// covers: path.string.021
#[test]
fn a_string_function_on_an_empty_collection_yields_empty() {
    // Every guarded function begins `if (focus.size() == 1 && ...)`, so an
    // empty focus never reaches the body. The answer is empty for all three
    // forms the scenario names.
    assert_eq!(eval("{}.upper()"), empty());
    assert_eq!(eval("{}.length()"), empty());
    assert_eq!(eval("{}.substring(0)"), empty());

    // Empty rather than a failure, and empty rather than a zero or an empty
    // string -- the three answers an implementation could confuse.
    assert_ne!(eval("{}.length()"), integer("0"));
    assert_ne!(eval("{}.upper()"), string(""));

    // And the same through a real navigation that happens to be absent.
    assert_eq!(eval_patient("Patient.name.prefix.upper()"), empty());
}

// ---------------------------------------------------------------------------
// path.string.022 -- UTF-16 code units, not bytes
// ---------------------------------------------------------------------------

// covers: path.string.022
#[test]
fn string_functions_operate_on_unicode_scalar_values() {
    // `patient-example.xml:122-130` carries `du March&#xE9;` and
    // `B&#xE9;n&#xE9;dicte` inside `Patient.contact.name`, so the input itself
    // has non-ASCII characters to measure.
    //
    // `funcLength` is `String.length()` on a Java string, which counts UTF-16
    // code units; every character here is in the Basic Multilingual Plane, so
    // code units and scalar values agree and the count is the character count.
    assert_eq!(eval("'Bénédicte'.length()"), integer("9"));
    assert_eq!(eval("'du Marché'.length()"), integer("9"));

    // A slice must not split a character: taking one character off the end of
    // `Marché` leaves `March`, and the accented character survives a slice that
    // includes it.
    assert_eq!(eval("'Marché'.substring(0, 5)"), string("March"));
    assert_eq!(eval("'Marché'.substring(5)"), string("é"));
    assert_eq!(eval("'Marché'.substring(5).length()"), integer("1"));

    // `toChars` yields one item per character, not per byte: `é` is two bytes
    // in UTF-8 and one character.
    assert_eq!(eval("'é'.toChars().count()"), integer("1"));
    assert_eq!(eval("'Marché'.toChars().count()"), integer("6"));
    assert_eq!(
        eval_patient("Patient.contact.name.family.length()"),
        integer("9")
    );

    // Above the Basic Multilingual Plane a character is two UTF-16 code units
    // (FHIRPathEngine.java:5771, 5884): `'a😀b'` (U+1F600) is four units.
    assert_eq!(eval("'a😀b'.length()"), integer("4"));
    assert_eq!(eval("'a😀b'.indexOf('b')"), integer("3"));
    // `substring` slices by unit and can split a surrogate pair (:5884-5886):
    // `substring(1, 1)` is the one-unit high surrogate U+D83D and
    // `substring(2)` is the low surrogate U+DE00 followed by `b`. A lone
    // surrogate cannot be held in a Rust `String`, so these are asserted
    // through FHIRPath: lengths 1 and 2, the tail after the low surrogate is
    // `b`, and the two halves are distinct (a lossy replacement would make them
    // equal). A byte count is never used.
    assert_eq!(eval("'a😀b'.substring(1, 1).length()"), integer("1"));
    assert_eq!(eval("'a😀b'.substring(2).length()"), integer("2"));
    assert_eq!(eval("'a😀b'.substring(3)"), string("b"));
    assert_eq!(
        eval("'a😀b'.substring(1, 1) = 'a😀b'.substring(2, 1)"),
        boolean(false),
        "the high and low surrogate halves are different one-unit strings"
    );
}

// ---------------------------------------------------------------------------
// path.string.023 -- the functions that do reject a bad focus
// ---------------------------------------------------------------------------

// covers: path.string.023
#[test]
fn the_functions_that_reject_a_bad_focus_reject_it_with_an_error() {
    // A focus whose size is not 1 raises, which is the shape of rejection the
    // string functions do **not** have: `funcRound` :4397, `funcTruncate`
    // :4273 and `funcSingle` :5087 all test the size first and throw.
    for expr in ["(1 | 2).round(2)", "(1 | 2).truncate()", "(1 | 2).single()"] {
        let message = expect_evaluation_failure(expr, None);
        assert!(
            !message.is_empty(),
            "{expr} must fail with a message naming the function"
        );
    }

    // `'abc'.round(2)` is rejected by the **check** on the focus type:
    // `checkContextDecimal` (FHIRPathEngine.java:3690, 3890-3892) raises
    // `FHIRPATH_DECIMAL_ONLY`, "The function round can only be used on a
    // decimal but found [System.String]".
    let message = expect_check_failure("'abc'.round(2)");
    assert!(
        message.contains("round") && message.contains("can only be used on a decimal"),
        "the check must name the function and the wanted type: {message}"
    );
    // With the check skipped it does NOT fail at evaluation: `funcRound`'s else
    // arm builds an exception and never throws it (:4414), so the result is
    // empty. (`funcTruncate` has the same defect at :4285.)
    assert_eq!(eval_unchecked("'abc'.round(2)"), empty());
}

// ---------------------------------------------------------------------------
// path.string.024 -- implicit string conversion
// ---------------------------------------------------------------------------

// covers: path.string.024
#[test]
fn implicit_string_conversion_changes_the_outcome_of_every_string_function() {
    // The guard is `focus.get(0).hasType(FHIR_TYPES_STRING) ||
    // doImplicitStringConversion`, so the setting is the second half of the
    // same condition on all thirteen guarded functions.
    //
    // Off is the default, and it is what both HAPI wrappers leave it at
    // (`path.strictness.009`).
    assert!(!StringSettings::default().implicit_string_conversion);

    let mut engine = configurable_engine_under_test();
    let off = StringSettings::default();
    let on = StringSettings {
        implicit_string_conversion: true,
    };

    let with_off = engine
        .evaluate_with("1.startsWith('1')", None, off)
        .expect("evaluating with the setting off must succeed and answer empty");
    assert_eq!(with_off, empty(), "off: a non-string focus yields empty");

    let with_on = engine
        .evaluate_with("1.startsWith('1')", None, on)
        .expect("evaluating with the setting on must succeed");
    assert_eq!(
        with_on,
        boolean(true),
        "on: the focus is converted to its string value before the test"
    );

    // The setting is engine-wide, so it moves every guarded function and not
    // just `startsWith`.
    for (expr, expected) in [
        ("1.length()", integer("1")),
        ("1.upper()", string("1")),
        ("1.contains('1')", boolean(true)),
        ("123.substring(1)", string("23")),
    ] {
        assert_eq!(
            engine
                .evaluate_with(expr, None, off)
                .expect("off evaluates"),
            empty(),
            "{expr} must be empty with implicit string conversion off"
        );
        assert_eq!(
            engine.evaluate_with(expr, None, on).expect("on evaluates"),
            expected,
            "{expr} must convert with implicit string conversion on"
        );
    }

    // And the setting is reachable from the public surface, which is what makes
    // both outcomes testable at all.
    assert_ne!(off, on);
}

// ---------------------------------------------------------------------------
// path.string.025 -- constructs whose meaning would silently differ (DEV-0023)
// ---------------------------------------------------------------------------

// covers: path.string.025
#[test]
fn a_pattern_construct_whose_meaning_would_differ_from_javas_is_an_execution_error() {
    // DIFFERENTIAL-ONLY: no pinned suite case grades any of these. HAPI accepts
    // every construct in `DEPARTURES` with Java's meaning; Frevo's engine is the
    // `regex` crate behind a compatibility layer (ADR 0005), which raises an
    // execution error rather than answer differently (DEV-0023). Each is tried
    // through `matches`, `matchesFull` and `replaceMatches`.
    //
    // FHIRPath literals double the backslash, so `'\\v'` hands the engine `\v`.
    const DEPARTURES: [&str; 8] = [
        "(?u)a",
        "(?U)a",
        "(?x)a",
        "\\\\v",
        "a*+",
        "\\\\p{Alpha}",
        "[a-z--b]",
        "[a-z~~b]",
    ];
    for pattern in DEPARTURES {
        for expr in [
            format!("'a'.matches('{pattern}')"),
            format!("'a'.matchesFull('{pattern}')"),
            format!("'a'.replaceMatches('{pattern}', 'x')"),
        ] {
            let message = expect_evaluation_failure(&expr, None);
            assert!(
                !message.is_empty(),
                "{expr} must fail with a message naming the construct"
            );
        }
    }

    // Backreferences and lookaround are the other way round: Java accepts
    // `'aa'.matches('(a)\\1')` (true) and `a(?=b)`, atomic groups and `\Q..\E`;
    // the `regex` crate has no such construct, so Frevo raises (DEV-0023).
    expect_evaluation_failure("'aa'.matches('(a)\\\\1')", None);
    expect_evaluation_failure("'ab'.matches('a(?=b)')", None);

    // NOT departures: an unbalanced parenthesis and an invalid range
    // (`[a--b]`) are errors in Java too.
    expect_evaluation_failure("'a'.matches('(a')", None);
    expect_evaluation_failure("'a'.matches('[a--b]')", None);
}

// ---------------------------------------------------------------------------
// path.string.026 -- ASCII \d \w \s \b
// ---------------------------------------------------------------------------

// covers: path.string.026
#[test]
fn d_w_s_and_b_mean_their_ascii_sets() {
    // Java's default (non-Unicode-class) meaning; DIFFERENTIAL-ONLY.
    assert_eq!(eval("'\u{663}'.matches('\\\\d')"), boolean(false));
    assert_eq!(eval("'\u{e9}'.matches('\\\\w')"), boolean(false));
    assert_eq!(eval("'\u{e9}'.matches('\\\\W')"), boolean(true));
    assert_eq!(eval("'a\u{a0}b'.matches('a\\\\sb')"), boolean(false));
    assert_eq!(eval("'7'.matches('\\\\d')"), boolean(true));
    assert_eq!(eval("'_'.matches('\\\\w')"), boolean(true));
    assert_eq!(eval("'a b'.matches('a\\\\sb')"), boolean(true));
    assert_eq!(eval("'ab'.matches('\\\\bab\\\\b')"), boolean(true));
    // The same inside a character class.
    assert_eq!(eval("'\u{e9}'.matches('[\\\\w]')"), boolean(false));
    // `\D`, `\W`, `\S` and `\B` are the complements.
    assert_eq!(eval("'a'.matches('\\\\D')"), boolean(true));
    assert_eq!(eval("'7'.matches('\\\\D')"), boolean(false));
    assert_eq!(eval("'a'.matches('\\\\S')"), boolean(true));
    assert_eq!(eval("' '.matches('\\\\S')"), boolean(false));
    assert_eq!(eval("'ab'.matches('a\\\\Bb')"), boolean(true));
}

// ---------------------------------------------------------------------------
// path.string.027 -- replacement syntax
// ---------------------------------------------------------------------------

// covers: path.string.027
#[test]
fn the_replace_matches_replacement_uses_javas_syntax() {
    // DIFFERENTIAL-ONLY. `$n`, `${name}` and backslash, as `Matcher.replaceAll`.
    assert_eq!(eval("'abc'.replaceMatches('(b)', '[$1]')"), string("a[b]c"));
    assert_eq!(
        eval("'abc'.replaceMatches('(?<x>b)', '<${x}>')"),
        string("a<b>c")
    );
    // The literals pass `\$` and two backslashes to the engine.
    assert_eq!(eval("'abc'.replaceMatches('b', '\\\\$')"), string("a$c"));
    assert_eq!(
        eval("'a.c'.replaceMatches('\\\\.', '\\\\\\\\')"),
        string("a\\c")
    );
    // `$n` takes further digits only while the number still names a group.
    assert_eq!(
        eval("'abcdefghij'.replaceMatches('(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)', '$10')"),
        string("j")
    );
    assert_eq!(eval("'a'.replaceMatches('(a)', '$10')"), string("a0"));
    // A group that did not participate contributes nothing.
    assert_eq!(
        eval("'ac'.replaceMatches('(a)(b)?c', '[$2]')"),
        string("[]")
    );
    // A missing group, `${nope}`, a trailing `$` and a trailing `\` are
    // execution errors, and so is the engine's own `$$` (an error in Java, not
    // `a$c`).
    for expr in [
        "'abc'.replaceMatches('(b)', '$2')",
        "'abc'.replaceMatches('b', '${nope}')",
        "'abc'.replaceMatches('b', 'x$')",
        "'abc'.replaceMatches('b', 'x\\\\')",
        "'abc'.replaceMatches('b', '$$')",
    ] {
        expect_evaluation_failure(expr, None);
    }
}

// ---------------------------------------------------------------------------
// path.string.028 -- empty-match iteration
// ---------------------------------------------------------------------------

// covers: path.string.028
#[test]
fn replace_matches_finds_empty_matches_as_javas_matcher_find_does() {
    // DIFFERENTIAL-ONLY. An empty match is found at every position including the
    // end, and after one the search steps one UTF-16 code UNIT forward.
    assert_eq!(eval("'abc'.replaceMatches('x*', '-')"), string("-a-b-c-"));
    assert_eq!(eval("'abc'.replaceMatches('b*', '-')"), string("-a--c-"));
    assert_eq!(eval("'abc'.replaceMatches('', 'x')"), string("abc"));

    // That step can split a surrogate pair. `'😀'` (U+1F600) is two units, so
    // the result is FIVE units: `-`, the high surrogate U+D83D, `-`, the low
    // surrogate U+DE00, `-`. A lone surrogate cannot be held in a Rust
    // `String`, so the units are asserted by length and by position: the
    // separators are `-`, and the two halves are one-unit strings that differ.
    let smile = "'\u{1F600}'.replaceMatches('x*', '-')";
    assert_eq!(eval(&format!("{smile}.length()")), integer("5"));
    for at in [0, 2, 4] {
        assert_eq!(
            eval(&format!("{smile}.substring({at}, 1)")),
            string("-"),
            "unit {at} is a separator"
        );
    }
    assert_eq!(
        eval(&format!("{smile}.substring(1, 1).length()")),
        integer("1")
    );
    assert_eq!(
        eval(&format!("{smile}.substring(3, 1).length()")),
        integer("1")
    );
    assert_eq!(
        eval(&format!(
            "{smile}.substring(1, 1) = {smile}.substring(3, 1)"
        )),
        boolean(false),
        "the high and low surrogate halves are different one-unit strings"
    );

    // `'a😀'`: `-`, `a`, `-`, U+D83D, `-`, U+DE00, `-` -- seven units.
    let a_smile = "'a\u{1F600}'.replaceMatches('x*', '-')";
    assert_eq!(eval(&format!("{a_smile}.length()")), integer("7"));
    for at in [0, 2, 4, 6] {
        assert_eq!(
            eval(&format!("{a_smile}.substring({at}, 1)")),
            string("-"),
            "unit {at} is a separator"
        );
    }
    assert_eq!(eval(&format!("{a_smile}.substring(1, 1)")), string("a"));
    assert_eq!(
        eval(&format!(
            "{a_smile}.substring(3, 1) = {a_smile}.substring(5, 1)"
        )),
        boolean(false),
        "the two surrogate halves differ"
    );
}

// ---------------------------------------------------------------------------
// path.string.029 -- compiled-size and nesting limits
// ---------------------------------------------------------------------------

// covers: path.string.029
#[test]
fn a_pattern_over_the_size_or_nesting_limit_is_an_execution_error() {
    // DIFFERENTIAL-ONLY. HAPI compiles both with `java.util.regex` and answers;
    // Frevo's compiled-program `size_limit` (2 MiB) and `nest_limit` (100) make
    // each an execution error, never a panic (`path.errors.003`; DEV-0023).
    expect_evaluation_failure("'a'.matches('(?:a{1000}){1000}')", None);
    let deep = format!("{}a{}", "(".repeat(200), ")".repeat(200));
    expect_evaluation_failure(&format!("'a'.matches('{deep}')"), None);

    // A pattern nested 50 deep and a small pattern evaluate normally.
    let fifty = format!("{}a{}", "(".repeat(50), ")".repeat(50));
    assert_eq!(eval(&format!("'a'.matches('{fifty}')")), boolean(true));
    assert_eq!(eval("'a'.matches('a')"), boolean(true));
}
