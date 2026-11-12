//! Unit tests for the front end.
//!
//! These are the in-crate half of tasks 28, 29 and 30. The scenarios they
//! carry are also graded through `tests/path_lexer.rs`,
//! `tests/path_literals.rs` and `tests/path_parse_errors.rs`, but those run
//! every expression through the whole engine — parse, check, evaluate — so
//! until the check pass and the evaluator exist they cannot observe what a
//! literal **is** or what shape the parser gave an expression. That is what is
//! asserted here, against the same pinned source: `FHIRLexer.java`,
//! `ExpressionNode.java` and `FHIRPathEngine.java` at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`.

use super::ast::{Constant, Expression, Function, Kind, Operation};
use super::error::ParseError;
use super::literal::{UCUM_SYSTEM, process_date_constant};
use super::parser::{MAX_PARSE_DEPTH, ParseOptions};
use crate::seam::value::{TemporalPrecision, Value};

/// A single backslash, assembled rather than written into the escape
/// sequences below, so that nothing between this source and the compiler can
/// decode `*` into the character it denotes and leave an assertion that
/// checks nothing.
const BACKSLASH: char = '\\';

fn parse(expr: &str) -> Expression {
    Expression::parse(expr).unwrap_or_else(|e| panic!("{expr:?} must parse: {e}"))
}

fn rejected(expr: &str) -> ParseError {
    match Expression::parse(expr) {
        Err(e) => e,
        Ok(parsed) => panic!("{expr:?} must be rejected, but it parsed as {parsed}"),
    }
}

/// The root node's literal, for an expression that is one.
fn literal(expr: &str) -> Constant {
    let parsed = parse(expr);
    let root = parsed.node(parsed.root());
    assert_eq!(
        root.kind(),
        Kind::Constant,
        "{expr:?} must parse to a single literal, not {parsed}"
    );
    root.constant()
        .unwrap_or_else(|| panic!("{expr:?} has a constant"))
        .clone()
}

/// The literal's value, for a literal that has one.
fn literal_value(expr: &str) -> Value {
    match literal(expr) {
        Constant::Value(value) => value,
        other => panic!("{expr:?} must be a value literal, not {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Task 28 -- the lexer
// ---------------------------------------------------------------------------

// covers: path.lexer.001, path.lexer.002, path.lexer.003, path.lexer.005
#[test]
fn a_comment_contributes_nothing_to_the_parsed_form() {
    // Each of these carries the same expression with a comment somewhere in
    // it, and must parse to the same tree as the expression alone. The
    // rendering is the tree, so comparing renderings compares trees.
    let plain = parse("2 + 2").to_string();
    for expr in [
        "2 + 2 // This is a single-line comment + 4",
        "// comment\n// another\n2+2",
        "2 + 2 /* and + 2 in a block comment */",
        "/* a leading block comment */ 2+2",
        "2 + /* inline $@%^+ * */ 2",
    ] {
        assert_eq!(
            parse(expr).to_string(),
            plain,
            "{expr:?} must parse as if the comment were not there"
        );
    }
}

// covers: path.lexer.004
#[test]
fn text_after_a_closing_block_comment_is_still_expression() {
    // The distinguishing pair of `comments/testComment3` and
    // `comments/testComment4`: the `+2` inside the comment is dropped, the
    // `+2` after it is not.
    assert_eq!(parse("2 + 2 /* +2 */").to_string(), "2 + 2");
    assert_eq!(parse("2 + 2 /* comment */ +2").to_string(), "2 + 2 + 2");
}

// covers: path.lexer.006
#[test]
fn a_comment_may_sit_between_an_operand_and_its_operator() {
    // `comments/testComment6`: the `/` on the second line is the division
    // operator, so the comment separated an operand from its operator.
    let parsed = parse("2 // comment\n/ 2");
    let root = parsed.node(parsed.root());
    assert_eq!(root.operation(), Some(Operation::DivideBy));
    assert_eq!(parsed.to_string(), "2 / 2");
}

// covers: path.lexer.007
#[test]
fn an_inline_block_comment_is_ignored_mid_expression() {
    // `comments/testComment9`: the comment body carries a `+` and a `*`,
    // neither of which may be lexed as an operator, and a lone `*` inside the
    // comment must not close it.
    assert_eq!(
        parse("2 + /* inline $@%^+ * */ 2 = 4").to_string(),
        "(2 + 2) = 4"
    );
}

// covers: path.lexer.008
#[test]
fn a_trailing_operator_is_a_syntax_error() {
    // `comments/testComment7`. The `/` is lexed as the division operator —
    // `skipWhitespaceAndComments` only treats `//` as a comment when a
    // character follows it — and then the parser asks for a right operand and
    // the lexer is done (`FHIRPathEngine.java:1090`).
    let e = rejected("2 + 2 /");
    assert_eq!(e.message(), "Expression terminated unexpectedly");
    assert_eq!(e.offset(), "2 + 2 /".len());
}

// covers: path.lexer.009
#[test]
fn an_unterminated_block_comment_is_a_syntax_error() {
    // `comments/testComment8`. Upstream builds the "Unfinished comment"
    // exception and discards it (`FHIRLexer.java:315`), which leaves the last
    // character of the source as the next token; the parse then fails on that
    // token. The failure is the behaviour the scenario asks for, and the
    // mechanism is upstream's.
    let e = rejected("2 + 2 /* not finished");
    assert_eq!(
        e.message(),
        "Premature ExpressionNode termination at unexpected token \"d\""
    );

    // Closing the comment makes the same expression legal, so the rule is
    // about the termination and not about block comments.
    assert_eq!(parse("2 + 2 /* finished */").to_string(), "2 + 2");
}

// covers: path.lexer.010
#[test]
fn a_backtick_delimited_identifier_is_the_same_name_as_the_plain_one() {
    // `testType/testType19`. The backticks are a lexical device, so the two
    // spellings must produce the same tree.
    let delimited = parse("Patient.is(FHIR.`Patient`)");
    let plain = parse("Patient.is(FHIR.Patient)");
    assert_eq!(delimited.to_string(), plain.to_string());

    // And the name really is the undelimited one.
    let root = delimited.node(delimited.root());
    let call = delimited.node(root.inner().expect("the `is` call"));
    assert_eq!(call.function(), Some(Function::Is));
    let argument = delimited.node(call.parameters()[0]);
    assert_eq!(argument.name(), Some("FHIR"));
    assert_eq!(
        delimited
            .node(argument.inner().expect("the type name"))
            .name(),
        Some("Patient")
    );
}

// covers: path.lexer.011
#[test]
fn every_string_escape_the_lexer_knows_is_decoded() {
    // The ten escapes `processConstantString` accepts
    // (`FHIRPathEngine.java:1796-1829`).
    // `testLiterals/testLiteralStringEscapes` exercises all ten in one
    // literal, so a decoder missing any one of them fails to parse the
    // expression at all.
    let expr = format!(
        "'{b}{b} {b}/ {b}f {b}r {b}n {b}t {b}\" {b}` {b}' {b}u002a'",
        b = BACKSLASH
    );
    let decoded = match literal_value(&expr) {
        Value::String(s) => s,
        other => panic!("a quoted run is a string literal, not {other:?}"),
    };
    assert_eq!(decoded, "\\ / \u{000c} \r \n \t \" ` ' *");
}

// covers: path.lexer.012
#[test]
fn an_unknown_string_escape_is_a_syntax_error() {
    let e = rejected(&format!("'a{BACKSLASH}qb'"));
    assert_eq!(e.message(), "Unknown FHIRPath character escape \\q");
    assert_eq!(e.offset(), 2, "the offset names the escape itself");

    // A known escape in the same position parses, so the failure is about
    // `\q` and not about escapes in general.
    assert!(matches!(
        literal_value(&format!("'a{BACKSLASH}tb'")),
        Value::String(s) if s == "a\tb"
    ));
}

// covers: path.lexer.013
#[test]
fn a_unicode_escape_needs_four_hexadecimal_digits() {
    // Upstream takes `s.substring(i, i + 4)` unguarded and hands it to
    // `Integer.parseInt(.., 16)`, so `'\u00'` raises
    // StringIndexOutOfBoundsException, `'\u00zz'` raises NumberFormatException
    // and `'\u+041'` decodes to `A` because `parseInt` accepts a sign. All
    // four spellings below are refused here with a diagnostic (`DEV-0011`),
    // which is also what makes `testLiterals/testLiteralUnicodeInvalid` — an
    // `invalid="syntax"` case — fail at the parse stage as the suite expects.
    for expr in [r"'\u00'", r"'\u00zz'", r"'\u+041'", r"'\u-041'"] {
        let e = rejected(expr);
        assert!(
            e.message().contains("escape"),
            "{expr:?} must say what it refused: {}",
            e.message()
        );
    }

    // Four hexadecimal digits are decoded, so the rule is about the digits.
    assert!(matches!(
        literal_value(&format!("'{BACKSLASH}u002a'")),
        Value::String(s) if s == "*"
    ));
}

// covers: path.lexer.014
#[test]
fn an_unterminated_string_literal_is_a_syntax_error() {
    for expr in ["'abc", "1 = 'abc", "`abc", "Patient.name = 'abc"] {
        let e = rejected(expr);
        assert_eq!(
            e.message(),
            "Unterminated string",
            "{expr:?} must report the open delimiter"
        );
        assert!(
            expr.is_char_boundary(e.offset()) && expr[e.offset()..].starts_with(['\'', '`']),
            "{expr:?}: the offset {} must name the delimiter that was never closed",
            e.offset()
        );
    }

    // `'abc = 'def` is **not** this case: `'abc = '` is a terminated literal
    // and the `def` after it is left over, so it fails as a stray token. The
    // rule is about a delimiter the source runs out before closing.
    assert_eq!(
        rejected("'abc = 'def").message(),
        "Premature ExpressionNode termination at unexpected token \"def\""
    );

    assert_eq!(parse("'abc'").to_string(), "'abc'");
}

// covers: path.lexer.015
#[test]
fn a_double_quoted_string_is_a_syntax_error_unless_the_parser_is_told_otherwise() {
    // `allowDoubleQuotes` has no initialiser (`FHIRPathEngine.java:182`), so
    // it is off on every engine nothing sets it on — which is both HAPI
    // wrappers. With it off a double quote is not a delimiter, and the
    // resulting one-character token fails `isToken()`.
    assert!(!ParseOptions::default().allow_double_quotes);
    let e = rejected("\"foo\" = 'foo'");
    // Why that message: `FHIRPathConstant.isFHIRPathConstant` accepts a
    // leading double quote whether or not the lexer delimits on one, so with
    // the setting off the lone `\"` token is read as an **empty** string
    // constant and the `foo` after it is left over. Upstream fails exactly
    // here, with exactly this sentence.
    assert_eq!(
        e.message(),
        "Premature ExpressionNode termination at unexpected token \"foo\""
    );

    // The single-quoted form of the same expression parses, so the failure is
    // about the delimiter.
    assert_eq!(parse("'foo' = 'foo'").to_string(), "'foo' = 'foo'");

    // And with the setting on, the run is a string literal with the same
    // content as the single-quoted spelling — not a delimited identifier.
    let options = ParseOptions {
        allow_double_quotes: true,
    };
    let parsed = Expression::parse_with("\"foo\" = 'foo'", options, None)
        .expect("with double quotes allowed it parses");
    assert_eq!(parsed.to_string(), "'foo' = 'foo'");
}

// ---------------------------------------------------------------------------
// Task 29 -- literals
// ---------------------------------------------------------------------------

// covers: path.literal.001
#[test]
fn true_and_false_are_boolean_literals() {
    assert!(matches!(literal_value("true"), Value::Boolean(true)));
    assert!(matches!(literal_value("false"), Value::Boolean(false)));
}

// covers: path.literal.002
#[test]
fn a_single_quoted_run_is_a_string_literal() {
    match literal_value("'Peter'") {
        Value::String(s) => assert_eq!(s, "Peter", "the quotes are not part of the value"),
        other => panic!("not a string: {other:?}"),
    }
}

// covers: path.literal.003
#[test]
fn an_integer_literal_is_an_integer() {
    for (expr, value) in [("1", 1), ("0", 0), ("2147483647", 2147483647)] {
        match literal_value(expr) {
            Value::Integer(i) => assert_eq!(i, value),
            other => panic!("{expr:?} must be an integer, not {other:?}"),
        }
    }
}

// covers: path.literal.004
#[test]
fn a_parenthesised_negative_integer_is_a_unary_minus_over_the_literal() {
    // `(-1)` is a group holding a unary operation whose operand is the literal
    // `1`: `execute`'s `Unary` arm pushes an integer zero and the subtraction
    // then yields -1 (`FHIRPathEngine.java:1471`).
    let parsed = parse("(-1)");
    let group = parsed.node(parsed.root());
    assert_eq!(group.kind(), Kind::Group);
    let unary = parsed.node(group.group().expect("the group's content"));
    assert_eq!(unary.kind(), Kind::Unary);
    assert_eq!(unary.operation(), Some(Operation::Minus));
    let operand = parsed.node(unary.op_next().expect("the operand"));
    assert_eq!(operand.kind(), Kind::Constant);
    assert!(matches!(
        operand.constant(),
        Some(Constant::Value(Value::Integer(1)))
    ));
}

// covers: path.literal.005
#[test]
fn an_unparenthesised_negative_integer_before_a_call_parses_and_binds_to_the_call() {
    // `testLiterals/testLiteralIntegerNegative1Invalid` is
    // `invalid="execution"`, not `invalid="syntax"`: the expression is
    // syntactically fine, and the unary minus applies to what
    // `1.convertsToInteger()` returned, which is why it fails only when it is
    // evaluated.
    let parsed = parse("-1.convertsToInteger()");
    let unary = parsed.node(parsed.root());
    assert_eq!(unary.kind(), Kind::Unary);
    assert_eq!(unary.operation(), Some(Operation::Minus));
    let operand = parsed.node(unary.op_next().expect("the operand"));
    assert_eq!(operand.kind(), Kind::Constant);
    let call = parsed.node(operand.inner().expect("the call hangs off the literal"));
    assert_eq!(call.function(), Some(Function::ConvertsToInteger));
}

// covers: path.literal.006
#[test]
fn a_decimal_literal_keeps_the_precision_it_was_written_with() {
    match literal_value("1.58700") {
        Value::Decimal(d) => {
            assert_eq!(d.lexical(), "1.58700", "the trailing zeros are retained");
            assert_eq!(d.precision(), 5);
            assert_eq!(d.mantissa(), 158700);
        }
        other => panic!("not a decimal: {other:?}"),
    }
    // A shorter spelling of the same number reports a shorter precision,
    // which is what makes this a statement about the literal.
    match literal_value("1.587") {
        Value::Decimal(d) => assert_eq!(d.precision(), 3),
        other => panic!("not a decimal: {other:?}"),
    }

    // `1.58700.precision()` lexes as the decimal, a `.`, and the call: the
    // number arm of the lexer stops at the second dot.
    let parsed = parse("1.58700.precision()");
    let constant = parsed.node(parsed.root());
    assert!(matches!(
        constant.constant(),
        Some(Constant::Value(Value::Decimal(d))) if d.lexical() == "1.58700"
    ));
    let call = parsed.node(constant.inner().expect("the call"));
    assert_eq!(call.function(), Some(Function::Precision));
    assert!(call.parameters().is_empty());
}

// covers: path.literal.006
#[test]
fn a_decimal_with_a_leading_zero_run_is_refused_as_upstream_refuses_it() {
    // `processConstant` calls `isDecimal(value, false)`, which rejects a
    // leading zero (`Utilities.java:239-246`), so `01.5` is not a literal at
    // all. `0.5` and `0` are.
    let e = rejected("01.5");
    assert_eq!(e.message(), "Invalid Constant 01.5");
    assert!(matches!(literal_value("0.5"), Value::Decimal(_)));
    assert!(matches!(literal_value("0"), Value::Integer(0)));
}

// covers: path.literal.007
#[test]
fn date_literals_are_accepted_at_year_month_and_day_precision() {
    for (expr, lexical, precision) in [
        ("@2014", "2014", TemporalPrecision::Year),
        ("@2014-01", "2014-01", TemporalPrecision::Month),
        ("@2014-01-05", "2014-01-05", TemporalPrecision::Day),
    ] {
        // The parser keeps the run as written and leaves resolving it to the
        // evaluation, exactly as upstream's `FHIRConstant` does, because a
        // malformed temporal literal is an **execution** error there.
        assert!(
            matches!(literal(expr), Constant::Deferred(text) if text == expr),
            "{expr:?} is carried as written"
        );
        match process_date_constant(&expr[1..]).expect("a valid date") {
            Value::Date(date) => {
                assert_eq!(date.lexical, lexical);
                assert_eq!(date.precision, precision);
            }
            other => panic!("{expr:?} must denote a date, not {other:?}"),
        }
    }
}

// covers: path.literal.009
#[test]
fn datetime_literals_are_accepted_at_every_precision_and_offset_form() {
    for (expr, lexical, precision, offset) in [
        (
            "@2014-01-05T10:30",
            "2014-01-05T10:30",
            TemporalPrecision::Minute,
            None,
        ),
        (
            "@2014-01-05T10:30:00",
            "2014-01-05T10:30:00",
            TemporalPrecision::Second,
            None,
        ),
        (
            "@2014-01-05T10:30:00.000",
            "2014-01-05T10:30:00.000",
            TemporalPrecision::Millisecond,
            None,
        ),
        (
            "@2014-01-05T10:30:00Z",
            "2014-01-05T10:30:00Z",
            TemporalPrecision::Second,
            Some(0),
        ),
        (
            "@2014-01-05T10:30:00+10:00",
            "2014-01-05T10:30:00+10:00",
            TemporalPrecision::Second,
            Some(600),
        ),
    ] {
        // One token: the `@` arm of the lexer takes the digits, the
        // separators, the `T`, the `Z` and the offset sign, and the fractional
        // second only at the one position a fractional second can appear
        // (`FHIRLexer.java:335`).
        assert!(matches!(literal(expr), Constant::Deferred(text) if text == expr));
        match process_date_constant(&expr[1..]).expect("a valid dateTime") {
            Value::DateTime(dt) => {
                assert_eq!(dt.lexical, lexical, "the offset is kept, not normalised");
                assert_eq!(dt.precision, precision);
                assert_eq!(dt.offset_minutes, offset);
            }
            other => panic!("{expr:?} must denote a dateTime, not {other:?}"),
        }
    }

    // `@2015T` is a **date**: `"2015T".split("T")` drops the trailing empty
    // element upstream, so there is no time at all
    // (`FHIRPathEngine.java:1688`). `funcIs` still answers true for
    // `is(DateTime)` on a date (`FHIRPathEngine.java:5128`), which is why
    // `testLiterals/testLiteralDateTimeYear` passes upstream.
    assert!(matches!(process_date_constant("2015T"), Ok(Value::Date(_))));
}

// covers: path.literal.011
#[test]
fn time_literals_are_accepted_at_each_precision() {
    for (expr, lexical, precision) in [
        ("@T10", "10:00", TemporalPrecision::Minute),
        ("@T10:30", "10:30", TemporalPrecision::Minute),
        ("@T10:30:00", "10:30:00", TemporalPrecision::Second),
        (
            "@T10:30:00.000",
            "10:30:00.000",
            TemporalPrecision::Millisecond,
        ),
    ] {
        assert!(matches!(literal(expr), Constant::Deferred(text) if text == expr));
        match process_date_constant(&expr[1..]).expect("a valid time") {
            Value::Time(time) => {
                // An hour-only time is padded to `hh:00:00` and then
                // truncated to five characters by `setPrecision(MINUTE)`
                // (`TimeType.java:138`), so `@T10` is `10:00`.
                assert_eq!(time.lexical, lexical);
                assert_eq!(time.precision, precision);
            }
            other => panic!("{expr:?} must denote a time, not {other:?}"),
        }
    }

    // A time with an offset is an error, which is why the suite's two such
    // cases are `invalid="execution"` and carry no expected output.
    assert!(process_date_constant("T10:30:00Z").is_err());
}

// covers: path.literal.012
#[test]
fn a_quantity_literal_with_a_ucum_unit_is_a_quantity() {
    // `FHIRPathEngine.java:1132` sets the unit from the keyword and the code
    // from the quoted run, so a quoted unit leaves the unit text null.
    for (expr, value, code) in [("1 'cm'", "1", "cm"), ("10.1 'mg'", "10.1", "mg")] {
        match literal_value(expr) {
            Value::Quantity(q) => {
                assert_eq!(q.value.lexical(), value, "the value is kept as written");
                assert_eq!(q.unit, None, "a quoted unit leaves no unit text");
                assert_eq!(q.code.as_deref(), Some(code));
                assert_eq!(q.system.as_deref(), Some(UCUM_SYSTEM));
            }
            other => panic!("{expr:?} must be a quantity, not {other:?}"),
        }
    }
}

// covers: path.literal.013
#[test]
fn calendar_duration_keywords_are_quantity_literals() {
    // Sixteen keywords, eight singular and eight plural. `year` and `month`
    // map to no UCUM code at all — upstream declines them with "this is not
    // the UCUM year" — so those two carry neither a system nor a code.
    for (expr, value, unit, code) in [
        ("7 days", "7", "days", Some("d")),
        ("1 week", "1", "week", Some("wk")),
        ("1 weeks", "1", "weeks", Some("wk")),
        ("1 day", "1", "day", Some("d")),
        ("1 hour", "1", "hour", Some("h")),
        ("1 hours", "1", "hours", Some("h")),
        ("1 minute", "1", "minute", Some("min")),
        ("1 minutes", "1", "minutes", Some("min")),
        ("1 second", "1", "second", Some("s")),
        ("1 seconds", "1", "seconds", Some("s")),
        ("1 millisecond", "1", "millisecond", Some("ms")),
        ("1 milliseconds", "1", "milliseconds", Some("ms")),
        ("1 year", "1", "year", None),
        ("1 years", "1", "years", None),
        ("1 month", "1", "month", None),
        ("1 months", "1", "months", None),
    ] {
        match literal_value(expr) {
            Value::Quantity(q) => {
                assert_eq!(q.value.lexical(), value);
                assert_eq!(
                    q.unit.as_deref(),
                    Some(unit),
                    "{expr:?} keeps the literal word as its unit"
                );
                assert_eq!(q.code.as_deref(), code);
                assert_eq!(
                    q.system.as_deref(),
                    code.map(|_| UCUM_SYSTEM),
                    "{expr:?} carries a system exactly when it carries a code"
                );
            }
            other => panic!("{expr:?} must be a quantity, not {other:?}"),
        }
    }
}

// covers: path.literal.014
#[test]
fn the_empty_collection_literal_is_its_own_constant() {
    // `processConstant` returns a null constant for `{}`
    // (`FHIRPathEngine.java:1332`), which `resolveConstant` turns into an
    // empty collection — not into a one-item collection holding a null.
    assert!(matches!(literal("{}"), Constant::Empty));
    let parsed = parse("{}.count()");
    let root = parsed.node(parsed.root());
    assert!(root.is_null_set());
    let call = parsed.node(root.inner().expect("the call"));
    assert_eq!(call.function(), Some(Function::Count));
}

// covers: path.literal.015
#[test]
fn an_integer_literal_beyond_the_32_bit_range_is_lexed_as_a_decimal() {
    // `Utilities.isInteger` bounds-checks — "check bounds
    // -2,147,483,648..2,147,483,647" — and returns false for a ten-digit
    // value above the maximum, so the next arm of `processConstant` reads the
    // literal as a decimal. It is neither an integer nor an error.
    match literal_value("2147483648") {
        Value::Decimal(d) => assert_eq!(d.lexical(), "2147483648"),
        other => panic!("past the boundary the literal is a decimal, not {other:?}"),
    }
    // One below the boundary is still an integer, so the boundary is where
    // `isInteger` puts it and not somewhere else.
    assert!(matches!(
        literal_value("2147483647"),
        Value::Integer(2147483647)
    ));
    // And well past it, too.
    assert!(matches!(literal_value("99999999999999"), Value::Decimal(_)));
}

// ---------------------------------------------------------------------------
// Task 30 -- the parser and precedence
// ---------------------------------------------------------------------------

// covers: path.parse.001
#[test]
fn the_parsed_form_is_reusable_and_carries_no_state() {
    // What makes "parse once, evaluate many" possible: the parsed form owns
    // its text and its nodes, is cloneable, and reading it does not change
    // it. `Sync` is what lets one parsed expression be shared across the
    // threads of a write transaction.
    fn assert_shareable<T: Send + Sync + Clone>(_: &T) {}

    let parsed = parse("Patient.name.given.first()");
    assert_shareable(&parsed);
    assert_eq!(parsed.text(), "Patient.name.given.first()");
    let once = parsed.to_string();
    let twice = parsed.to_string();
    assert_eq!(once, twice, "rendering it must not consume it");
    assert_eq!(once, "Patient.name.given.first()");
    assert_eq!(parsed.clone().to_string(), once);
    assert_eq!(parsed.node_count(), 4);
}

// covers: path.parse.002
#[test]
fn precedence_follows_the_engines_table() {
    // `organisePrecedence` regroups the flat chain the parse produced, one
    // level at a time, tightest first (`FHIRPathEngine.java:1226-1236`). The
    // rendering shows the grouping it introduced, and a group it did not need
    // to introduce is absent — which is why the right-hand sides below differ
    // in shape as well as in text.
    for (expr, grouped) in [
        // `*`, `/`, `div`, `mod` bind tighter than `+`, `-`, `&`.
        ("1 + 2 * 3", "1 + (2 * 3)"),
        ("1 * 2 + 3", "(1 * 2) + 3"),
        ("1 - 2 div 3", "1 - (2 div 3)"),
        ("1 + 2 mod 3", "1 + (2 mod 3)"),
        // `+`/`-` bind tighter than a comparison, which binds tighter than
        // equality.
        ("1 + 2 < 4", "(1 + 2) < 4"),
        ("1 < 2 = true", "(1 < 2) = true"),
        // `is` and `as` bind tighter than `|`.
        ("1 is Integer | 2", "(1 is Integer) | 2"),
        // `in`, `contains` and `memberOf` bind tighter than `and`.
        ("1 in 2 and true", "(1 in 2) and true"),
        // `and` binds tighter than `or` and `xor`.
        ("true and false or true", "(true and false) or true"),
        ("true or false and true", "true or (false and true)"),
        ("true and false xor true", "(true and false) xor true"),
        // `implies` is last: upstream gathers no group for it, so everything
        // else binds tighter.
        ("true implies false or true", "true implies (false or true)"),
        ("true or false implies true", "(true or false) implies true"),
        // Same-level operators stay a flat left-to-right chain.
        ("1 + 2 + 3", "1 + 2 + 3"),
        ("1 * 2 / 3", "1 * 2 / 3"),
    ] {
        assert_eq!(
            parse(expr).to_string(),
            grouped,
            "{expr:?} must be grouped as {grouped:?}"
        );
    }
}

// covers: path.parse.002
#[test]
fn an_indexer_and_a_path_step_build_the_shape_the_evaluator_walks() {
    // `name[0]` is an implicit `item()` call that the following `.` hangs off,
    // so `Patient.name[0].given` reads "the given of the first name" and not
    // "the first of name.given" (`FHIRPathEngine.java:1188-1204`).
    let parsed = parse("Patient.name[0].given");
    let patient = parsed.node(parsed.root());
    assert_eq!(patient.name(), Some("Patient"));
    let name = parsed.node(patient.inner().expect("name"));
    assert_eq!(name.name(), Some("name"));
    let item = parsed.node(name.inner().expect("the indexer"));
    assert_eq!(item.function(), Some(Function::Item));
    let given = parsed.node(item.inner().expect("given"));
    assert_eq!(given.name(), Some("given"));
    assert!(given.inner().is_none());
    assert_eq!(parsed.to_string(), "Patient.name[0].given");
}

// covers: path.parse.002
#[test]
fn the_three_variables_whose_names_begin_with_a_dollar_are_the_only_ones() {
    // `checkName` (`ExpressionNode.java:516`).
    for expr in ["$this", "$index", "$total"] {
        assert_eq!(parse(expr).to_string(), expr);
    }
    let e = rejected("$nope");
    assert_eq!(e.message(), "Found $nope expecting a valid token name");
}

// covers: path.parse.003
#[test]
fn parentheses_override_precedence() {
    // The written group is kept as a `Group` node, so it is the operand of the
    // multiplication rather than being flattened into the chain.
    assert_eq!(parse("(1 + 2) * 3").to_string(), "(1 + 2) * 3");
    assert_eq!(parse("1 + 2 * 3").to_string(), "1 + (2 * 3)");
    let parsed = parse("(1 + 2) * 3");
    let root = parsed.node(parsed.root());
    assert_eq!(root.kind(), Kind::Group);
    assert_eq!(root.operation(), Some(Operation::Times));
    assert_eq!(parse("1 + (2 * (3 + 4))").to_string(), "1 + (2 * (3 + 4))");
}

// covers: path.parse.004
#[test]
fn an_unbalanced_parenthesis_is_a_syntax_error() {
    // The Group arm raises `Found <token> expecting a ")"` when the group is
    // not closed (`FHIRPathEngine.java:1139-1141`); when the source ran out
    // inside a sub-expression, the inner `parseExpression` gets there first
    // and says so.
    for (expr, message) in [
        // The group's content parsed, and then there was no `)`.
        ("(1 + 2", "Found  expecting a \")\""),
        ("1 + (2 * (3 + 4)", "Found  expecting a \")\""),
        // Inside an argument list the argument parsed, and what follows is
        // neither the separator nor the closing parenthesis.
        (
            "Patient.name.where(use = 'official'",
            "The token  is not expected here - either a \",\" or a \")\" expected",
        ),
        // Here the whole expression parsed and the stray `(` is left over.
        (
            "((1))(",
            "Premature ExpressionNode termination at unexpected token \"(\"",
        ),
    ] {
        let e = rejected(expr);
        assert_eq!(e.message(), message, "{expr:?}");
        assert!(e.offset() <= expr.len());
    }

    // The balanced forms parse, so the rule is about the balance.
    for expr in ["(1 + 2)", "1 + (2 * (3 + 4))", "((1))"] {
        parse(expr);
    }
}

// covers: path.parse.005
#[test]
fn an_unknown_function_name_is_a_parse_error_that_names_the_function() {
    // `parseExpression` consults `Function.fromCode` and then the host
    // service, and raises when neither resolves the name
    // (`FHIRPathEngine.java:1158-1166`). The check is at parse time, so no
    // evaluation happens at all.
    let e = rejected("Patient.nosuchfunction()");
    assert_eq!(
        e.message(),
        "The name nosuchfunction is not a valid function name"
    );
    assert_eq!(
        e.offset(),
        "Patient.".len(),
        "the offset names the function, not the expression"
    );

    // `toDate()` is one real example: the R4 enum has `ConvertsToDate` and no
    // `ToDate`, so an expression using it does not parse (`DEV-0009`).
    assert_eq!(
        rejected("'2015'.toDate()").message(),
        "The name toDate is not a valid function name"
    );

    // A name that is not followed by `(` is an element name and not a
    // function, so it must not be rejected here.
    assert_eq!(
        parse("Patient.nosuchfunction").to_string(),
        "Patient.nosuchfunction"
    );
}

// covers: path.parse.006
#[test]
fn the_wrong_argument_count_is_reported_at_parse_time_with_the_arity() {
    // `checkParamCount` has a one-bound and a two-bound form
    // (`FHIRPathEngine.java:1348-1360`); `substring` is declared 1..2 and
    // `first` is declared 0.
    let e = rejected("Patient.name.substring()");
    assert_eq!(
        e.message(),
        "The function \"substring\" requires between 1 and 2 parameters"
    );
    assert_eq!(e.offset(), "Patient.name.".len());

    let e = rejected("Patient.name.first(1)");
    assert_eq!(e.message(), "The function \"first\" requires 0 parameters");

    let e = rejected("Patient.name.substring(1, 2, 3)");
    assert_eq!(
        e.message(),
        "The function \"substring\" requires between 1 and 2 parameters"
    );

    // The correct arities parse, so the rule is about the count.
    for expr in [
        "Patient.name.substring(1)",
        "Patient.name.substring(1, 2)",
        "Patient.name.first()",
    ] {
        parse(expr);
    }
}

// covers: path.parse.007
#[test]
fn a_parse_error_names_the_byte_offset_it_was_found_at() {
    // The offset is a byte offset into the expression, zero-based, at the
    // first byte of the offending token. Upstream reports a (line, column)
    // pair whose column bookkeeping never counts the whitespace
    // `skipWhitespaceAndComments` consumed (`FHIRLexer.java:285`), so this is
    // a position rather than a reproduction of that drift.
    const PREFIX: &str = "(1) + ";
    const TAIL: &str = "1 + ) 2";

    let mut offsets = Vec::new();
    for k in 0..4 {
        let expr = format!("{}{TAIL}", PREFIX.repeat(k));
        let e = rejected(&expr);
        let offset = e.offset();
        assert!(expr.is_char_boundary(offset));
        assert!(
            expr[offset..].starts_with(") 2"),
            "{expr:?}: the offset {offset} must point at the offending `)`, \
             but it points at {:?}",
            &expr[offset..]
        );
        offsets.push(offset);
    }
    for k in 1..offsets.len() {
        assert_eq!(offsets[k] - offsets[k - 1], PREFIX.len());
    }

    // A multi-byte character before the error moves the offset by its length
    // in bytes, and the offset stays on a character boundary.
    let expr = "'é' + ) 2";
    let e = rejected(expr);
    assert!(expr.is_char_boundary(e.offset()) && expr[e.offset()..].starts_with(") 2"));
}

// covers: path.errors.004
#[test]
fn a_deeply_nested_expression_is_refused_rather_than_overflowing_the_stack() {
    // Upstream recurses once per group with no counter
    // (`FHIRPathEngine.java:1134-1143`), so the JVM answers 10,000 nested
    // groups with a StackOverflowError. The parser returns an error instead.
    let expr = format!("{}1{}", "(".repeat(10_000), ")".repeat(10_000));
    let e = rejected(&expr);
    assert!(
        e.message().contains("depth limit"),
        "the refusal must say which limit was reached: {}",
        e.message()
    );

    // A shallow nest of the same shape is accepted, so the limit is a limit
    // and not a refusal to nest at all.
    let shallow = format!("{}1{}", "(".repeat(8), ")".repeat(8));
    assert_eq!(parse(&shallow).to_string(), shallow);

    // And just below the limit still parses, so the limit is where it says it
    // is. Each written group costs one level of recursion.
    let deep = MAX_PARSE_DEPTH - 2;
    let at_limit = format!("{}1{}", "(".repeat(deep), ")".repeat(deep));
    parse(&at_limit);
}
