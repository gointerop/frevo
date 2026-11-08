//! Lexing: `path.lexer.001` through `path.lexer.016`.
//!
//! Run it with:
//!
//! ```text
//! cargo test -p frevo-path --test path_lexer
//! ```
//!
//! Eleven of the sixteen scenarios name a case in the vendored suite, so they
//! are driven through the committed harness rather than hand-rolled: the case's
//! expression, input file and expected output come from
//! `oracles/fhirpath/tests-fhir-r4.xml` and the grading from
//! `tests/oracle/run.rs`. The five that do not -- the two double-quote
//! scenarios and the three malformed-string ones -- go straight at the engine
//! seam.
//!
//! The load-bearing property throughout: `EngineError::NotImplemented` is
//! **not** a parse failure. A scenario that expects a syntax error asserts the
//! error is an `EngineError::Diagnostic`, so an engine that refuses to parse
//! anything fails the test instead of passing it. In the suite harness the same
//! property is the `did-not-run` status, which is never a pass.
//!
//! Provenance for every behaviour asserted here is
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRLexer.java`
//! and `FHIRPathEngine.java` at `73c432b5f43ddbce979978a760ec4aa81e929094`,
//! cited per test.

// `dead_code` is allowed for the shared harness module, and for nothing else in
// this file: `tests/oracle/mod.rs:18-23` records that the oracle is a surface
// shared by several test binaries and by the tests this task adds, so an
// accessor this binary does not happen to call is not dead code.
#[allow(dead_code)]
mod oracle;

use std::sync::OnceLock;

use oracle::engine::{EngineConfig, EngineError, SuiteEngine, engine_under_test};
use oracle::run::{self, CaseReport, CaseStatus, RunReport};
use oracle::suite::{Stage, Suite};
use oracle::value::ResultItem;

/// A single backslash.
///
/// Escape sequences in this file are assembled from it rather than written
/// literally, so that no editor, patch or serialisation step on the way into
/// this source can decode `\u002a` into the character it denotes and leave an
/// assertion that checks nothing.
const BACKSLASH: char = '\\';

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

/// The pinned suite, loaded and run once per test binary.
fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

/// One suite case's report, addressed by `group/name`.
fn case_report(group: &str, name: &str) -> &'static CaseReport {
    let (suite, report) = pinned();
    let case = suite.case(group, name).unwrap_or_else(|| {
        panic!("{group}/{name} must resolve to exactly one case in the pinned suite")
    });
    let address = case.address.to_string();
    report
        .get(&address)
        .unwrap_or_else(|| panic!("{address} must appear in the run report"))
}

/// Assert a suite case passed, naming the status and the reason so a
/// `did-not-run` is never read as a wrong answer.
fn expect_case(group: &str, name: &str) -> &'static CaseReport {
    let r = case_report(group, name);
    assert_eq!(
        r.status,
        CaseStatus::Passed,
        "{}: {:?} is {} -- {}",
        r.address,
        r.expression,
        r.status.as_str(),
        r.reason
    );
    r
}

/// The single expected `<output>` the pinned file records for a case, so a
/// re-pin that changes the expectation is visible here too.
fn sole_output(group: &str, name: &str) -> (String, String) {
    let (suite, _) = pinned();
    let case = suite
        .case(group, name)
        .unwrap_or_else(|| panic!("{group}/{name} must resolve to exactly one case"));
    assert_eq!(
        case.outputs.len(),
        1,
        "{group}/{name} must have exactly one expected output"
    );
    (
        case.outputs[0].type_name.clone(),
        case.outputs[0].body.clone(),
    )
}

/// A configured engine from the harness seam.
fn fresh_engine() -> Box<dyn SuiteEngine> {
    let mut engine = engine_under_test();
    engine
        .configure(&EngineConfig::default())
        .unwrap_or_else(|e| {
            panic!("the engine must configure before it can be lexed through: {e}")
        });
    engine
}

/// `expr` must fail to **parse**, with a real diagnostic.
///
/// Matching on the variant rather than calling `is_err` is what stops an engine
/// that answers `NotImplemented` to everything from passing every syntax-error
/// scenario in this file.
fn expect_parse_error(expr: &str) -> (String, Option<usize>) {
    let mut engine = fresh_engine();
    match engine.parse(expr) {
        Ok(_) => panic!("{expr:?} must fail to parse, but it parsed"),
        Err(EngineError::Diagnostic { message, offset }) => (message, offset),
        Err(e) => panic!(
            "{expr:?} must fail to parse with a diagnostic; the engine answered instead: {e}"
        ),
    }
}

/// `expr` must parse.
fn expect_parses(expr: &str) {
    let mut engine = fresh_engine();
    engine
        .parse(expr)
        .unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"));
}

// ---------------------------------------------------------------------------
// path.lexer.001 -- a `//` comment runs to end of line
// ---------------------------------------------------------------------------

// covers: path.lexer.001
#[test]
fn a_line_comment_runs_to_the_end_of_the_line() {
    // The `+ 4` inside the comment must not reach the parser.
    let (ty, body) = sole_output("comments", "testComment1");
    assert_eq!((ty.as_str(), body.as_str()), ("integer", "4"));
    expect_case("comments", "testComment1");
}

// ---------------------------------------------------------------------------
// path.lexer.002 -- several `//` comment lines are ignored
// ---------------------------------------------------------------------------

// covers: path.lexer.002
#[test]
fn consecutive_line_comments_are_all_ignored() {
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment2")
        .expect("comments/testComment2 is addressable");
    // Two leading `//` lines and `2+2` on the third: the lexer must skip to the
    // third line rather than stopping after the first.
    assert_eq!(
        case.expression.lines().filter(|l| l.contains("//")).count(),
        2,
        "the pinned case must still be the two-comment-line one: {:?}",
        case.expression
    );
    let (ty, body) = sole_output("comments", "testComment2");
    assert_eq!((ty.as_str(), body.as_str()), ("integer", "4"));
    expect_case("comments", "testComment2");
}

// ---------------------------------------------------------------------------
// path.lexer.003 -- a block comment after the expression is ignored
// ---------------------------------------------------------------------------

// covers: path.lexer.003
#[test]
fn a_trailing_block_comment_contributes_nothing() {
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment3")
        .expect("comments/testComment3 is addressable");
    assert!(
        case.expression.contains("/*") && case.expression.contains("+2"),
        "the pinned case must still carry a `+2` inside a block comment: {:?}",
        case.expression
    );
    let (ty, body) = sole_output("comments", "testComment3");
    assert_eq!((ty.as_str(), body.as_str()), ("integer", "4"));
    expect_case("comments", "testComment3");
}

// ---------------------------------------------------------------------------
// path.lexer.004 -- text after a closing block comment is expression
// ---------------------------------------------------------------------------

// covers: path.lexer.004
#[test]
fn text_after_a_closing_block_comment_is_still_expression() {
    // The distinguishing pair: testComment3 and testComment4 differ only in
    // whether the `+2` is inside or after the comment, and they yield 4 and 6.
    let (ty3, body3) = sole_output("comments", "testComment3");
    let (ty4, body4) = sole_output("comments", "testComment4");
    assert_eq!((ty3.as_str(), body3.as_str()), ("integer", "4"));
    assert_eq!(
        (ty4.as_str(), body4.as_str()),
        ("integer", "6"),
        "the `+2` after the comment must be added"
    );
    expect_case("comments", "testComment4");
}

// ---------------------------------------------------------------------------
// path.lexer.005 -- a leading block comment is ignored
// ---------------------------------------------------------------------------

// covers: path.lexer.005
#[test]
fn a_leading_block_comment_is_ignored() {
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment5")
        .expect("comments/testComment5 is addressable");
    assert!(
        case.expression.trim_start().starts_with("/*"),
        "the pinned case must still open with a block comment: {:?}",
        case.expression
    );
    let (ty, body) = sole_output("comments", "testComment5");
    assert_eq!((ty.as_str(), body.as_str()), ("integer", "4"));
    expect_case("comments", "testComment5");
}

// ---------------------------------------------------------------------------
// path.lexer.006 -- a comment can separate an operator from its operand
// ---------------------------------------------------------------------------

// covers: path.lexer.006
#[test]
fn a_comment_may_sit_between_an_operand_and_its_operator() {
    // `2 // comment` on one line and `/ 2` on the next: the `/` is the division
    // operator and the result is a decimal, not an integer, because
    // `opDivideBy`'s numeric branch always produces a DecimalType
    // (FHIRPathEngine.java:3022-3026).
    let (ty, body) = sole_output("comments", "testComment6");
    assert_eq!((ty.as_str(), body.as_str()), ("decimal", "1"));
    expect_case("comments", "testComment6");
}

// ---------------------------------------------------------------------------
// path.lexer.007 -- an inline block comment is ignored mid-expression
// ---------------------------------------------------------------------------

// covers: path.lexer.007
#[test]
fn an_inline_block_comment_is_ignored_mid_expression() {
    // `2 + /* inline $@%^+ * */ 2 = 4` -- the comment body contains a `+` and a
    // `*`, neither of which may be lexed as an operator, and a lone `*` inside
    // the comment must not close it.
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment9")
        .expect("comments/testComment9 is addressable");
    assert!(
        case.expression.contains("$@%^+"),
        "the pinned case must still carry operator characters inside the comment: {:?}",
        case.expression
    );
    let (ty, body) = sole_output("comments", "testComment9");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("comments", "testComment9");
}

// ---------------------------------------------------------------------------
// path.lexer.008 -- a trailing operator is a syntax error
// ---------------------------------------------------------------------------

// covers: path.lexer.008
#[test]
fn a_trailing_operator_is_a_syntax_error() {
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment7")
        .expect("comments/testComment7 is addressable");
    assert_eq!(
        case.expected_failure,
        Some(Stage::Parse),
        "the pinned case must still be invalid=\"syntax\""
    );
    assert!(case.expression.trim_end().ends_with('/'));

    // The suite's grading: a case expecting a syntax failure passes only when
    // the parse stage produced a diagnostic. `NotImplemented` is did-not-run.
    let r = expect_case("comments", "testComment7");
    assert_eq!(
        r.failed_stage,
        Some(Stage::Parse),
        "{} must be attributed to the parse stage, not to a later one",
        r.address
    );

    // And directly at the seam, so the stage is not inferred from the harness.
    // Upstream: after `/` the parser asks for a right operand, the lexer is
    // done, and `parseExpression` raises "Found  expecting a token name"
    // (FHIRPathEngine.java:1144-1146).
    let (message, _) = expect_parse_error("2 + 2 /");
    assert!(
        !message.is_empty(),
        "a syntax error must carry a message naming what was expected"
    );
}

// ---------------------------------------------------------------------------
// path.lexer.009 -- an unterminated block comment is a syntax error
// ---------------------------------------------------------------------------

// covers: path.lexer.009
#[test]
fn an_unterminated_block_comment_is_a_syntax_error() {
    let (suite, _) = pinned();
    let case = suite
        .case("comments", "testComment8")
        .expect("comments/testComment8 is addressable");
    assert_eq!(case.expected_failure, Some(Stage::Parse));
    assert!(case.expression.contains("/*") && !case.expression.contains("*/"));

    let r = expect_case("comments", "testComment8");
    assert_eq!(r.failed_stage, Some(Stage::Parse));

    // FHIRLexer's comment arm raises "Unfinished comment" when the source runs
    // out before `*/`, so an unterminated comment is a diagnostic and not a
    // silently swallowed tail.
    let (message, _) = expect_parse_error("2 + 2 /* not finished");
    assert!(!message.is_empty());

    // Closing it makes the same expression legal, so the rule is about the
    // termination and not about block comments.
    expect_parses("2 + 2 /* finished */");
}

// ---------------------------------------------------------------------------
// path.lexer.010 -- a backtick-delimited identifier is accepted
// ---------------------------------------------------------------------------

// covers: path.lexer.010
#[test]
fn a_backtick_delimited_identifier_is_accepted() {
    let (suite, _) = pinned();
    let case = suite
        .case("testType", "testType19")
        .expect("testType/testType19 is addressable");
    assert_eq!(
        case.expression, "Patient.is(FHIR.`Patient`)",
        "the pinned case must still be the backtick-delimited one"
    );
    assert_eq!(case.inputfile.as_deref(), Some("patient-example.xml"));
    let (ty, body) = sole_output("testType", "testType19");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testType", "testType19");

    // The plain and the delimited spellings must agree, which is what makes the
    // backticks a lexical device and not a different name: FHIRLexer reads the
    // delimited run in its own arm and `processFixedName` decodes its escapes.
    expect_case("testType", "testType18");
}

// ---------------------------------------------------------------------------
// path.lexer.011 -- string escape sequences are decoded
// ---------------------------------------------------------------------------

// covers: path.lexer.011
#[test]
fn every_string_escape_the_lexer_knows_is_decoded() {
    let (suite, _) = pinned();
    let case = suite
        .case("testLiterals", "testLiteralStringEscapes")
        .expect("testLiterals/testLiteralStringEscapes is addressable");

    // The ten escapes `FHIRLexer.processConstant(String)` accepts, at
    // FHIRLexer.java:425-472. The case exercises all ten in one literal, so a
    // lexer missing any one of them fails to parse the expression at all.
    for escape in [
        r"\\", r"\/", r"\f", r"\r", r"\n", r"\t", r#"\""#, r"\`", r"\'",
    ] {
        assert!(
            case.expression.contains(escape),
            "the pinned case must still exercise {escape}: {:?}",
            case.expression
        );
    }
    // The tenth is the unicode escape. It is assembled rather than written out
    // so that nothing between this source and the compiler can decode it into
    // the character it denotes, which would make the assertion vacuous.
    let unicode_escape = format!("{BACKSLASH}u002a");
    assert!(
        case.expression.contains(&unicode_escape),
        "the pinned case must still exercise {unicode_escape}: {:?}",
        case.expression
    );
    let (ty, body) = sole_output("testLiterals", "testLiteralStringEscapes");
    assert_eq!((ty.as_str(), body.as_str()), ("boolean", "true"));
    expect_case("testLiterals", "testLiteralStringEscapes");
}

// ---------------------------------------------------------------------------
// path.lexer.012 -- an unknown escape sequence is a syntax error
// ---------------------------------------------------------------------------

// covers: path.lexer.012
#[test]
fn an_unknown_string_escape_is_a_syntax_error() {
    // `processConstant`'s `default:` arm raises
    // "Unknown FHIRPath character escape \q" (FHIRLexer.java:466-467). The
    // escape is unknown, not ignored, and not passed through as a bare `q`.
    let (message, _) = expect_parse_error(r"'a\qb'");
    assert!(
        message.contains('q'),
        "the diagnostic must name the offending escape: {message:?}"
    );

    // A known escape in the same position parses, so the failure is about `\q`
    // and not about escapes in general.
    expect_parses(r"'a\tb'");
}

// ---------------------------------------------------------------------------
// path.lexer.013 -- a short unicode escape is a syntax error
// ---------------------------------------------------------------------------

// covers: path.lexer.013
#[test]
fn a_unicode_escape_with_fewer_than_four_hex_digits_is_a_syntax_error() {
    // `processConstant`'s `case 'u'` takes `s.substring(i, i + 4)` unguarded
    // (FHIRLexer.java:459-463), so upstream raises an unchecked
    // StringIndexOutOfBoundsException here rather than a FHIRLexerException.
    // The scenario requires a syntax error and `path.errors.003` forbids a
    // panic, so Frevo must report it as a parse diagnostic.
    let (message, _) = expect_parse_error(r"'\u00'");
    assert!(
        !message.is_empty(),
        "the diagnostic must say something about the escape"
    );

    // Four hex digits parse, so the rule is about the digit count.
    expect_parses(&format!("'{BACKSLASH}u002a'"));

    // And four non-hexadecimal characters fail for the same reason: upstream's
    // `Integer.parseInt(..., 16)` cannot read them.
    expect_parse_error(r"'\u00zz'");
}

// ---------------------------------------------------------------------------
// path.lexer.014 -- an unterminated string literal is a syntax error
// ---------------------------------------------------------------------------

// covers: path.lexer.014
#[test]
fn an_unterminated_string_literal_is_a_syntax_error() {
    // FHIRLexer's single-quote and backtick arms both raise "Unterminated
    // string" when the cursor reaches the end of the source with the delimiter
    // still open.
    for expr in ["'abc", "'abc = 'def", "1 = 'abc", "`abc"] {
        let (message, _) = expect_parse_error(expr);
        assert!(
            !message.is_empty(),
            "{expr:?} must report why it failed to parse"
        );
    }

    // A terminated literal in the same shape parses.
    expect_parses("'abc'");
}

// ---------------------------------------------------------------------------
// path.lexer.015 -- double quotes are a syntax error by default
// ---------------------------------------------------------------------------

// covers: path.lexer.015
#[test]
fn a_double_quoted_string_is_a_syntax_error_by_default() {
    // `FHIRPathEngine.allowDoubleQuotes` is a plain boolean field with no
    // initialiser (FHIRPathEngine.java:182), so it is false on every engine
    // that nothing sets it on -- which is both HAPI wrappers. The lexer's
    // double-quote arm is gated on it (FHIRLexer.java:209), so with it off a
    // double quote is not a string delimiter and the resulting token fails
    // `isToken()`.
    let (message, _) = expect_parse_error("\"foo\" = 'foo'");
    assert!(
        !message.is_empty(),
        "the diagnostic must report the unexpected character: {message:?}"
    );

    // The single-quoted form of the same expression parses, so the failure is
    // about the delimiter and not about the expression.
    expect_parses("'foo' = 'foo'");

    // And the default of the setting really is off.
    assert!(
        !DoubleQuoteSettings::default().allow_double_quotes,
        "double quotes must be off by default"
    );
}

// ---------------------------------------------------------------------------
// path.lexer.016 -- double quotes when the engine is told to allow them
// ---------------------------------------------------------------------------

/// The one engine setting this file varies.
///
/// The shared harness's `EngineConfig` deliberately does not carry it: no case
/// in the pinned suite uses a double-quoted string, so the suite run fixes it
/// off and the oracle has no reason to expose it. `path.lexer.016` is the
/// scenario for the enabled state, so it needs a seam of its own -- the same
/// shape `path_check_warnings.rs` uses for the check pass's warning channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoubleQuoteSettings {
    /// `FHIRPathEngine.setAllowDoubleQuotes` (FHIRPathEngine.java:6996), which
    /// is threaded into the lexer at construction (FHIRPathEngine.java:334 and
    /// :371) and read at FHIRLexer.java:209.
    pub allow_double_quotes: bool,
}

impl DoubleQuoteSettings {
    pub const ALLOWED: DoubleQuoteSettings = DoubleQuoteSettings {
        allow_double_quotes: true,
    };
}

/// Parsing and evaluation with the double-quote setting varied.
pub trait DoubleQuoteOracle {
    /// The public setting must be readable as well as writable, so a caller can
    /// tell which lexer it is getting.
    fn allow_double_quotes(&self) -> bool;

    fn parse_with(
        &mut self,
        expression: &str,
        settings: DoubleQuoteSettings,
    ) -> Result<(), EngineError>;

    fn evaluate_with(
        &mut self,
        expression: &str,
        settings: DoubleQuoteSettings,
    ) -> Result<Vec<ResultItem>, EngineError>;
}

/// The binding graded until `frevo-path` exposes the setting.
fn double_quote_oracle_under_test() -> Box<dyn DoubleQuoteOracle> {
    Box::new(UnboundDoubleQuotes)
}

struct UnboundDoubleQuotes;

impl DoubleQuoteOracle for UnboundDoubleQuotes {
    fn allow_double_quotes(&self) -> bool {
        false
    }

    fn parse_with(
        &mut self,
        _expression: &str,
        _settings: DoubleQuoteSettings,
    ) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn evaluate_with(
        &mut self,
        _expression: &str,
        _settings: DoubleQuoteSettings,
    ) -> Result<Vec<ResultItem>, EngineError> {
        Err(EngineError::NotImplemented)
    }
}

// covers: path.lexer.016
#[test]
fn double_quotes_parse_and_evaluate_when_the_engine_is_configured_to_allow_them() {
    let mut oracle = double_quote_oracle_under_test();

    // The setting is public and readable, and off is what a fresh engine
    // reports.
    assert!(
        !oracle.allow_double_quotes(),
        "a fresh engine must report that double quotes are off"
    );

    // With it on, the expression `path.lexer.015` rejects parses.
    oracle
        .parse_with("\"foo\" = 'foo'", DoubleQuoteSettings::ALLOWED)
        .unwrap_or_else(|e| {
            panic!("with setAllowDoubleQuotes(true) a double-quoted run is a string literal: {e}")
        });

    // And it evaluates to `true`, because the lexer then treats the run as a
    // string literal with the same content as the single-quoted form -- not as
    // a delimited identifier, which would make the comparison false or an
    // error.
    let items = oracle
        .evaluate_with("\"foo\" = 'foo'", DoubleQuoteSettings::ALLOWED)
        .unwrap_or_else(|e| panic!("it must evaluate, not merely parse: {e}"));
    assert_eq!(
        items,
        vec![ResultItem::Primitive {
            fhir_type: "boolean".to_string(),
            value: "true".to_string(),
        }],
        "a double-quoted literal must compare equal to the single-quoted one"
    );

    // Escapes are decoded inside a double-quoted run too: that arm of the lexer
    // tracks a backslash exactly as the single-quote arm does
    // (FHIRLexer.java:209-221).
    let escaped = oracle
        .evaluate_with(r#""a\tb" = 'a\tb'"#, DoubleQuoteSettings::ALLOWED)
        .unwrap_or_else(|e| panic!("escapes must be decoded in a double-quoted literal: {e}"));
    assert_eq!(
        escaped,
        vec![ResultItem::Primitive {
            fhir_type: "boolean".to_string(),
            value: "true".to_string(),
        }]
    );

    // With the setting off the same expression still fails, so the setting is
    // what decides and not the expression.
    match oracle.parse_with("\"foo\" = 'foo'", DoubleQuoteSettings::default()) {
        Err(EngineError::Diagnostic { .. }) => {}
        Ok(()) => panic!("with the setting off a double-quoted run must not parse"),
        Err(e) => panic!("with the setting off it must fail with a diagnostic, not: {e}"),
    }
}
