//! Parsing and execution errors: `path.parse.001`-`008` and
//! `path.errors.001`-`005`.
//!
//! ```text
//! cargo test -p frevo-path --test path_parse_errors
//! ```
//!
//! Two scenario families share a file because they share a subject: what the
//! engine does when an expression is wrong, and at which of the three stages it
//! says so. The harness distinguishes the stages
//! (`tests/oracle/run.rs::run_case`), and these tests use that: a syntax error
//! that surfaces at evaluation is not the behaviour the suite grades, so the
//! stage is asserted and not merely the failure.
//!
//! `EngineError::NotImplemented` is never accepted as a failure. Every
//! expectation of an error matches on `EngineError::Diagnostic`, which is why
//! an engine that refuses to parse anything fails these tests rather than
//! passing them.
//!
//! Provenance: `FHIRPathEngine.java` and `FHIRLexer.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`, plus the message texts in
//! `org.hl7.fhir.utilities/src/main/resources/Messages.properties`.

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{
    CaseContext, EngineConfig, EngineError, ParsedHandle, StrictnessFlags, SuiteEngine,
    engine_under_test,
};
use oracle::input::{self, SuiteInput};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::{self, Stage, Suite};
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| panic!("the engine must configure before anything is parsed: {e}"));
    engine
}

/// `expr` must fail to **parse**, with a real diagnostic.
fn expect_parse_error(expr: &str) -> (String, Option<usize>) {
    let mut engine = fresh_engine();
    match engine.parse(expr) {
        Ok(_) => panic!("{expr:?} must fail to parse, but it parsed"),
        Err(EngineError::Diagnostic { message, offset }) => (message, offset),
        Err(e) => panic!("{expr:?} must fail to parse with a diagnostic, not: {e}"),
    }
}

/// Parse and check `expr`, then require evaluation to fail with a diagnostic.
///
/// Both earlier stages must succeed: an expression the parser rejects is not
/// testing an execution error.
fn expect_evaluation_error(expr: &str) -> (String, Option<usize>) {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse before it can fail at evaluation: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
        Ok(items) => panic!("{expr:?} must fail at evaluation, but it returned {items:?}"),
        Err(EngineError::Diagnostic { message, offset }) => (message, offset),
        Err(e) => panic!("{expr:?} must fail with a diagnostic, not: {e}"),
    }
}

fn primitive(fhir_type: &str, value: &str) -> ResultItem {
    ResultItem::Primitive {
        fhir_type: fhir_type.to_string(),
        value: value.to_string(),
    }
}

/// Evaluate a literal-only expression through all three stages.
fn eval(expr: &str) -> Vec<ResultItem> {
    let mut engine = fresh_engine();
    let context = CaseContext::default();
    let parsed = engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    engine
        .check(parsed, &context, StrictnessFlags::CHECK)
        .unwrap_or_else(|e| panic!("{expr:?} must pass the static check: {e}"));
    engine
        .evaluate(parsed, None, &context, StrictnessFlags::EVALUATE)
        .unwrap_or_else(|e| panic!("{expr:?} must evaluate: {e}"))
}

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the suite input {file} must load before it can be evaluated over: {e}")
    })
}

// ---------------------------------------------------------------------------
// path.parse.001 -- parse once, evaluate many
// ---------------------------------------------------------------------------

// covers: path.parse.001
#[test]
fn an_expression_parses_once_and_evaluates_against_several_resources() {
    // Three different Patient resources from the suite's own input set, each
    // with a different first given name, so a result that came from the wrong
    // resource is visible rather than merely plausible.
    let files = [
        ("patient-example.xml", "Peter"),
        ("patient-example-xds.xml", "John"),
        ("patient-glossy-example.xml", "Henry"),
    ];
    let inputs: Vec<(SuiteInput, &str)> = files
        .iter()
        .map(|(f, given)| (load_input(f), *given))
        .collect();

    let mut engine = fresh_engine();

    // One parse. `IFhirPath.parse` returns a retained parsed form, and
    // `evaluate(IBase, IParsedExpression, Class)` takes it, so a caller inside
    // a write transaction never re-parses.
    let parsed: ParsedHandle = engine
        .parse("Patient.name.given.first()")
        .unwrap_or_else(|e| panic!("the expression must parse once: {e}"));

    for (inp, expected_given) in &inputs {
        let resource_type = inp.resource_type();
        assert_eq!(
            resource_type,
            Some("Patient"),
            "{} must be a Patient for this test to mean anything",
            inp.file
        );
        let context = CaseContext {
            root_resource_type: resource_type,
            resource_type,
            ..CaseContext::default()
        };
        engine
            .check(parsed, &context, StrictnessFlags::CHECK)
            .unwrap_or_else(|e| {
                panic!(
                    "the retained expression must check against {}: {e}",
                    inp.file
                )
            });
        let items = engine
            .evaluate(parsed, Some(inp), &context, StrictnessFlags::EVALUATE)
            .unwrap_or_else(|e| {
                panic!(
                    "the retained expression must evaluate against {} without being re-parsed: {e}",
                    inp.file
                )
            });
        assert_eq!(
            items,
            vec![primitive("string", expected_given)],
            "{} must yield its own first given name",
            inp.file
        );
    }

    // The handle is still the one the single parse returned, so the three
    // evaluations really did share one parsed form.
    let again = engine
        .evaluate(
            parsed,
            Some(&inputs[0].0),
            &CaseContext {
                root_resource_type: Some("Patient"),
                resource_type: Some("Patient"),
                ..CaseContext::default()
            },
            StrictnessFlags::EVALUATE,
        )
        .unwrap_or_else(|e| panic!("a fourth evaluation on the same handle must work: {e}"));
    assert_eq!(again, vec![primitive("string", "Peter")]);
}

// ---------------------------------------------------------------------------
// path.parse.002 -- precedence follows the engine's table
// ---------------------------------------------------------------------------

// covers: path.parse.002
#[test]
fn the_precedence_group_holds() {
    let (suite, report) = pinned();
    let group = suite
        .groups
        .iter()
        .find(|g| g.name == "testPrecedence")
        .expect("the pinned suite has a testPrecedence group");
    assert_eq!(
        group.cases.len(),
        6,
        "the pinned testPrecedence group is six cases"
    );

    let mut failures = Vec::new();
    for case in &group.cases {
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
        "{} of {} testPrecedence cases did not pass:\n  {}",
        failures.len(),
        group.cases.len(),
        failures.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// path.parse.003 -- parentheses override precedence
// ---------------------------------------------------------------------------

// covers: path.parse.003
#[test]
fn parentheses_override_precedence() {
    assert_eq!(eval("(1 + 2) * 3 = 9"), vec![primitive("boolean", "true")]);
    assert_eq!(eval("1 + 2 * 3 = 7"), vec![primitive("boolean", "true")]);

    // The pair is only meaningful if the two readings really differ, so the
    // other two readings are false.
    assert_eq!(eval("(1 + 2) * 3 = 7"), vec![primitive("boolean", "false")]);
    assert_eq!(eval("1 + 2 * 3 = 9"), vec![primitive("boolean", "false")]);
}

// ---------------------------------------------------------------------------
// path.parse.004 -- an unbalanced parenthesis
// ---------------------------------------------------------------------------

// covers: path.parse.004
#[test]
fn an_unbalanced_parenthesis_is_a_syntax_error() {
    // `parseExpression`'s Group arm raises `Found <token> expecting a ")"` when
    // the group is not closed (FHIRPathEngine.java:1135-1142).
    for expr in [
        "(1 + 2",
        "Patient.name.where(use = 'official'",
        "((1))(",
        "1 + (2 * (3 + 4)",
    ] {
        let (message, _) = expect_parse_error(expr);
        assert!(
            !message.is_empty(),
            "{expr:?} must report why it failed to parse"
        );
    }

    // The balanced forms parse, so the rule is about the balance.
    let mut engine = fresh_engine();
    for expr in ["(1 + 2)", "1 + (2 * (3 + 4))"] {
        engine
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    }
}

// ---------------------------------------------------------------------------
// path.parse.005 -- an unknown function name
// ---------------------------------------------------------------------------

// covers: path.parse.005
#[test]
fn an_unknown_function_name_is_a_parse_error_that_names_the_function() {
    // `parseExpression` consults `Function.fromCode` and, failing that, the
    // host service; with neither resolving the name it raises
    // "The name <n> is not a valid function name" (FHIRPathEngine.java:1158-1166).
    // The check is at parse time, so no evaluation happens at all.
    let (message, _) = expect_parse_error("Patient.nosuchfunction()");
    assert!(
        message.contains("nosuchfunction"),
        "the diagnostic must name the function: {message:?}"
    );

    // A name that is not followed by `(` is an element name and not a function,
    // so it must not be rejected here -- that is what makes this a statement
    // about the call and not about the identifier.
    let mut engine = fresh_engine();
    engine
        .parse("Patient.nosuchfunction")
        .unwrap_or_else(|e| panic!("a bare name is not a function call, so it parses: {e}"));
}

// ---------------------------------------------------------------------------
// path.parse.006 -- the wrong number of arguments
// ---------------------------------------------------------------------------

// covers: path.parse.006
#[test]
fn the_wrong_argument_count_is_reported_at_parse_time_with_the_arity() {
    // `checkParamCount` raises
    // "The function \"<n>\" requires <c> parameters" and the two-bound form
    // raises "requires between <min> and <max> parameters"
    // (FHIRPathEngine.java:1094-1106). `substring` is declared 1..2
    // (`case Substring: return checkParamCount(lexer, location, exp, 1, 2)`)
    // and `first` is declared 0.
    let (substring, _) = expect_parse_error("Patient.name.substring()");
    assert!(
        substring.contains("substring"),
        "the diagnostic must name the function: {substring:?}"
    );
    assert!(
        substring.contains('1') && substring.contains('2'),
        "and its declared arity, which for substring is between 1 and 2: {substring:?}"
    );

    let (first, _) = expect_parse_error("Patient.name.first(1)");
    assert!(
        first.contains("first"),
        "the diagnostic must name the function: {first:?}"
    );
    assert!(
        first.contains('0'),
        "and its declared arity, which for first is 0: {first:?}"
    );

    // The correct arities parse, so the rule is about the count.
    let mut engine = fresh_engine();
    for expr in [
        "Patient.name.substring(1)",
        "Patient.name.substring(1, 2)",
        "Patient.name.first()",
    ] {
        engine
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
    }
}

// ---------------------------------------------------------------------------
// path.parse.007 -- a parse error names the offset
// ---------------------------------------------------------------------------

// covers: path.parse.007
#[test]
fn a_parse_error_names_the_character_offset_it_was_found_at() {
    // The offset is a character offset into the expression, zero-based, at the
    // first character of the offending token. The harness's channel for it is
    // `EngineError::Diagnostic::offset`.
    //
    // Upstream reports a (line, column) pair instead, and its column
    // bookkeeping is approximate: `FHIRLexer.next` advances the column by the
    // token length only (FHIRLexer.java:286) after
    // `skipWhitespaceAndComments` has already moved the cursor past the
    // whitespace, so the skipped characters are never counted. Frevo reports
    // the offset, so this test states the position rather than reproducing the
    // drift.
    const PREFIX: &str = "(1) + ";
    const TAIL: &str = "1 + ) 2";

    let mut offsets = Vec::new();
    for k in 0..4 {
        let expr = format!("{}{TAIL}", PREFIX.repeat(k));
        let (_, offset) = expect_parse_error(&expr);
        let offset =
            offset.unwrap_or_else(|| panic!("{expr:?}: a syntax error must carry an offset"));
        assert!(
            offset <= expr.len(),
            "{expr:?}: the offset {offset} must be inside the expression"
        );
        assert!(
            expr.is_char_boundary(offset),
            "{expr:?}: the offset {offset} must be a character boundary"
        );
        assert!(
            expr[offset..].starts_with(") 2"),
            "{expr:?}: the offset {offset} must point at the offending `)`, \
             but it points at {:?}",
            &expr[offset..]
        );
        offsets.push(offset);
    }

    // And it moves with the token: each extra `(1) + ` prefix pushes the same
    // error exactly six characters further along.
    for k in 1..offsets.len() {
        assert_eq!(
            offsets[k] - offsets[k - 1],
            PREFIX.len(),
            "the reported offset must track the position of the error: {offsets:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// path.parse.008 -- the seven syntax-invalid oracle cases
// ---------------------------------------------------------------------------

// covers: path.parse.008
#[test]
fn every_syntax_invalid_case_in_the_pinned_suite_fails_to_parse() {
    let (s, report) = pinned();
    let cases: Vec<_> = s
        .cases()
        .filter(|c| c.expected_failure == Some(Stage::Parse))
        .collect();
    assert_eq!(
        cases.len(),
        suite::EXPECTED_SYNTAX_INVALID,
        "the pinned file has {} invalid=\"syntax\" cases",
        suite::EXPECTED_SYNTAX_INVALID
    );

    let mut failures = Vec::new();
    for case in &cases {
        let address = case.address.to_string();
        let r = report
            .get(&address)
            .unwrap_or_else(|| panic!("{address} must appear in the run report"));
        // Passing means the parse stage produced a diagnostic. A
        // `did-not-run` -- which is what `NotImplemented` yields -- is not a
        // pass, which is exactly why an engine that parses nothing cannot
        // satisfy this scenario by accident.
        if r.status != CaseStatus::Passed || r.failed_stage != Some(Stage::Parse) {
            failures.push(format!(
                "{address} ({:?}): {} at {:?} -- {}",
                case.expression,
                r.status.as_str(),
                r.failed_stage,
                r.reason
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of the {} syntax-invalid cases did not fail at the parse stage:\n  {}",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// path.errors.001 -- the 27 execution-invalid oracle cases
// ---------------------------------------------------------------------------

// covers: path.errors.001
#[test]
fn every_execution_invalid_case_parses_and_checks_and_then_fails_to_evaluate() {
    let (s, report) = pinned();
    let cases: Vec<_> = s
        .cases()
        .filter(|c| c.expected_failure == Some(Stage::Evaluate))
        .collect();
    assert_eq!(
        cases.len(),
        suite::EXPECTED_EXECUTION_INVALID,
        "the pinned file has {} invalid=\"execution\" cases",
        suite::EXPECTED_EXECUTION_INVALID
    );

    let mut failures = Vec::new();
    for case in &cases {
        let address = case.address.to_string();
        let r = report
            .get(&address)
            .unwrap_or_else(|| panic!("{address} must appear in the run report"));
        // The harness only reaches the evaluate stage after parse and check
        // have both returned Ok, so `failed_stage == Evaluate` is the
        // assertion that the two earlier stages succeeded.
        if r.status != CaseStatus::Passed || r.failed_stage != Some(Stage::Evaluate) {
            failures.push(format!(
                "{address} ({:?}): {} at {:?} -- {}",
                case.expression,
                r.status.as_str(),
                r.failed_stage,
                r.reason
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of the {} execution-invalid cases did not fail at the evaluate stage:\n  {}",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// path.errors.002 -- an error names the expression and the position
// ---------------------------------------------------------------------------

// covers: path.errors.002
#[test]
fn an_execution_error_names_the_sub_expression_and_its_offset() {
    // Upstream's `makeException` builds
    // `new PathEngineException(fmt, constName, holder.getStart(), holder.toString())`
    // (FHIRPathEngine.java:584-593), so the exception carries both the location
    // and a rendering of the failing sub-expression. The harness's
    // `EngineError::Diagnostic` has one text channel, so both must be in the
    // message.
    let (message, offset) = expect_evaluation_error("1 < 'a'");
    assert!(
        message.contains("integer") && message.contains("string"),
        "FHIRPATH_CANT_COMPARE names both operand types: {message:?}"
    );
    assert!(
        message.contains("1 < 'a'"),
        "the diagnostic must name the failing sub-expression: {message:?}"
    );
    let offset = offset.expect("an execution error must carry an offset");
    assert!(offset <= "1 < 'a'".len());

    // The offset is the position of the failing sub-expression inside the whole
    // expression, so padding the front of the expression moves it.
    const PREFIX: &str = "(1=1) and ";
    let mut offsets = Vec::new();
    for k in 0..3 {
        let expr = format!("{}1 < 'a'", PREFIX.repeat(k));
        let (message, offset) = expect_evaluation_error(&expr);
        assert!(
            message.contains("integer") && message.contains("string"),
            "{expr:?}: {message:?}"
        );
        let offset = offset.unwrap_or_else(|| panic!("{expr:?} must report an offset"));
        assert!(
            expr.is_char_boundary(offset) && expr[offset..].starts_with("1 < 'a'"),
            "{expr:?}: the offset {offset} must point at the failing sub-expression, \
             but it points at {:?}",
            &expr[offset.min(expr.len())..]
        );
        offsets.push(offset);
    }
    for k in 1..offsets.len() {
        assert_eq!(
            offsets[k] - offsets[k - 1],
            PREFIX.len(),
            "the offset must track the sub-expression's position: {offsets:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// path.errors.003 -- evaluation never panics
// ---------------------------------------------------------------------------

// covers: path.errors.003
#[test]
fn every_case_in_the_suite_including_the_invalid_ones_returns_a_status() {
    let (s, report) = pinned();
    assert_eq!(report.cases.len(), s.case_count());
    assert_eq!(report.cases.len(), suite::EXPECTED_CASES);

    // Reaching this line already shows that no case aborted the process. The
    // scenario asks for more: every expression in the suite is parsed, checked
    // and evaluated. A case reported `did-not-run` was never evaluated at all,
    // so it does not satisfy the premise -- and `did-not-run` is exactly what
    // `EngineError::NotImplemented` produces, which is why this assertion is
    // the one that keeps the scenario honest.
    let not_run: Vec<&str> = report
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::DidNotRun)
        .map(|c| c.address.as_str())
        .collect();
    assert!(
        not_run.is_empty(),
        "{} of {} cases were never evaluated; the first few are {:?}. \
         The common reason is reported as: {}",
        not_run.len(),
        report.cases.len(),
        &not_run[..not_run.len().min(5)],
        report
            .cases
            .iter()
            .find(|c| c.status == CaseStatus::DidNotRun)
            .map(|c| c.reason.as_str())
            .unwrap_or("")
    );

    // And every case that did not pass explains itself, because a failure that
    // reports nothing is indistinguishable from a crash.
    for c in report.not_passed() {
        assert!(
            !c.reason.is_empty(),
            "{} did not pass and must say why",
            c.address
        );
    }
}

// ---------------------------------------------------------------------------
// path.errors.004 -- evaluation is bounded for an adversarial expression
// ---------------------------------------------------------------------------

// covers: path.errors.004
#[test]
fn a_deeply_nested_expression_yields_a_limit_error_rather_than_a_stack_overflow() {
    // Ten thousand nested groups. Upstream recurses once per group in
    // `parseExpression`'s Group arm (FHIRPathEngine.java:1134-1143) with no
    // depth counter, so the JVM answers with a StackOverflowError. Frevo must
    // answer with a returned error, because `path.errors.003` forbids the
    // process-level failure mode.
    const DEPTH: usize = 10_000;
    let expr = format!("{}1{}", "(".repeat(DEPTH), ")".repeat(DEPTH));

    let mut engine = fresh_engine();
    let context = CaseContext::default();

    // Whichever stage declines it, the answer must be a returned diagnostic:
    // never a success, and never the absence of an answer.
    let message = match engine.parse(&expr) {
        Err(EngineError::Diagnostic { message, .. }) => message,
        Err(e) => panic!("the engine gave no graded answer for a 10,000-deep expression: {e}"),
        Ok(parsed) => match engine.check(parsed, &context, StrictnessFlags::CHECK) {
            Err(EngineError::Diagnostic { message, .. }) => message,
            Err(e) => panic!("the engine gave no graded answer at check: {e}"),
            Ok(()) => match engine.evaluate(parsed, None, &context, StrictnessFlags::EVALUATE) {
                Err(EngineError::Diagnostic { message, .. }) => message,
                Err(e) => panic!("the engine gave no graded answer at evaluate: {e}"),
                Ok(items) => panic!(
                    "a 10,000-deep expression must be refused with a depth or step-limit \
                     error, not accepted; it returned {} item(s)",
                    items.len()
                ),
            },
        },
    };
    assert!(
        !message.is_empty(),
        "the refusal must say which limit was reached"
    );

    // A shallow nest of the same shape is accepted, so the limit is a limit and
    // not a refusal to nest at all.
    let shallow = format!("{}1{}", "(".repeat(8), ")".repeat(8));
    assert_eq!(eval(&shallow), vec![primitive("integer", "1")]);
}

// ---------------------------------------------------------------------------
// path.errors.005 -- a model error is surfaced, not swallowed
// ---------------------------------------------------------------------------

/// A decode error the model reports for one element.
///
/// ADR 0004 names swallowing this into an empty collection as the worst
/// available failure mode, because an invariant written over the element then
/// silently passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDecodeError {
    /// The element path the model was asked for.
    pub path: String,
    pub message: String,
}

/// Navigation over a model that reports a decode error for a named element.
///
/// The shared harness cannot express this: its `SuiteInput` is a well-formed
/// document by construction, so a model fault has no way in. This is the same
/// shape `path_check_warnings.rs` uses -- a small oracle of its own, bound to
/// nothing until the engine exists.
pub trait ModelFaultOracle {
    /// Evaluate `expression` over a `Patient` whose `fault_at` element answers
    /// with `fault` instead of a value.
    fn evaluate_over_faulty_model(
        &mut self,
        expression: &str,
        fault_at: &str,
        fault: &ModelDecodeError,
    ) -> Result<Vec<ResultItem>, EngineError>;
}

fn model_fault_oracle_under_test() -> Box<dyn ModelFaultOracle> {
    Box::new(UnboundModelFault)
}

struct UnboundModelFault;

impl ModelFaultOracle for UnboundModelFault {
    fn evaluate_over_faulty_model(
        &mut self,
        _expression: &str,
        _fault_at: &str,
        _fault: &ModelDecodeError,
    ) -> Result<Vec<ResultItem>, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

// covers: path.errors.005
#[test]
fn a_decode_error_from_the_model_fails_the_evaluation_rather_than_yielding_empty() {
    let mut oracle = model_fault_oracle_under_test();
    let fault = ModelDecodeError {
        path: "Patient.birthDate".to_string(),
        message: "birthDate is not a valid date: '19740-12-25'".to_string(),
    };

    // Navigating to the faulty element must fail, and the engine's diagnostic
    // must carry the model's own message so the cause is not replaced by a
    // generic one.
    match oracle.evaluate_over_faulty_model("Patient.birthDate", "Patient.birthDate", &fault) {
        Err(EngineError::Diagnostic { message, .. }) => {
            assert!(
                message.contains(&fault.message),
                "the engine must surface the model's message, not substitute its own: {message:?}"
            );
            assert!(
                message.contains(&fault.path),
                "and name the element it was reading: {message:?}"
            );
        }
        Ok(items) => panic!(
            "a model decode error must fail the evaluation; it returned {items:?}. \
             An empty collection here is the failure mode ADR 0004 calls the worst \
             available one, because `Patient.birthDate.empty()` then reports true."
        ),
        Err(e) => panic!("the engine gave no graded answer: {e}"),
    }

    // And the same holds when the faulty element is reached through a
    // predicate, where an empty collection would quietly satisfy the
    // invariant instead of failing it.
    for expression in [
        "Patient.birthDate.empty()",
        "Patient.birthDate.exists()",
        "Patient.birthDate < @2000-01-01",
        "Patient.where(birthDate < @2000-01-01).exists()",
    ] {
        match oracle.evaluate_over_faulty_model(expression, "Patient.birthDate", &fault) {
            Err(EngineError::Diagnostic { .. }) => {}
            Ok(items) => panic!(
                "{expression:?} must fail when the model cannot decode birthDate; \
                 it returned {items:?}"
            ),
            Err(e) => panic!("{expression:?}: the engine gave no graded answer: {e}"),
        }
    }

    // An element the fault does not cover still evaluates, so the failure is
    // attributable to the faulty element and not to the whole resource.
    let unaffected = oracle
        .evaluate_over_faulty_model("Patient.active", "Patient.birthDate", &fault)
        .unwrap_or_else(|e| {
            panic!("a sound element must still be readable beside a faulty one: {e}")
        });
    assert_eq!(
        unaffected,
        vec![primitive("boolean", "true")],
        "the fault is scoped to the element the model could not decode"
    );
}
