//! The UCUM unit algebra the engine calls: parsing a unit code, reducing it to
//! base units, and the `UcumEssenceService` operations over the result
//! (`path.quantity.020`, `.023`, `.024`).
//!
//! Derived from, under Ucum-java/src/main/java/org/fhir/ucum/:
//! `Lexer.java` (`consume`, `checkNumber`, `checkNumberOrSymbol`,
//! `checkBrackets`, `isValidSymbolChar`, `checkAnnotation`, `checkSingle`,
//! `nextChar`, `peekChar`, `error`, `getTokenAsInt`, `finished`, at
//! `:52-206`), `ExpressionParser.java` (`parse`, `parseTerm`, `parseComp`,
//! `parseSymbol`, at `:48-146`), `Converter.java` (`normalise` of a term and of
//! a symbol, `expandDefinedUnit`, at `:57-180`), `Canonical.java`
//! (`multiplyValue`, `divideValue`), `ExpressionComposer.java`
//! (`compose(Canonical, false)`, `:97-109`), `special/Registry.java` (`init`),
//! `special/HoldingHandler.java`, `special/CelsiusHandler.java`,
//! `special/FahrenheitHandler.java`, and `UcumEssenceService.java`
//! (`getCanonicalUnits` at `:232-240`, `getCanonicalForm` at `:266-285`,
//! `multiply` and `divideBy` at `:314-325`, `isComparable` at `:333-343`), at
//! db9d22ade3f0f4a667b45817b85a9ac4534e4224 (tag `1.0.10`; live HAPI 8.12.0
//! runs 1.0.9, identical in every class ported here, `path.math.015`).
//! Original work Copyright (c) 2006+, Health Intersections Pty Ltd, licensed
//! under the BSD 3-Clause License, whose text is reproduced in
//! `ucum_decimal.rs` and in `NOTICE`.
//! Modified by the Frevo authors: the term tree is a Rust enum rather than a
//! class hierarchy, a unit is referred to by its index in the loaded data
//! (upstream merges base units by object identity, which is the same thing),
//! exceptions are the arms of [`UcumError`], and the source is read as UTF-16
//! code units as Java reads it, so positions in messages are Java's.
//!
//! Where this differs from the Java, and why it is unreachable from a code
//! HAPI would have answered: a lone surrogate in a message is written as
//! U+FFFD, which a Rust string must; and the symbol `1`, which `parseSymbol`
//! accepts with no unit and `Converter` would then dereference, cannot be
//! lexed as a symbol (a run of digits is a number), so it is not handled.

use super::ucum::{UcumEssence, UnitRef};
use super::ucum_decimal::{Failure, UcumDecimal};

/// What the UCUM service throws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UcumError {
    /// `org.fhir.ucum.UcumException` with this message.
    Ucum(String),
    /// `java.lang.NumberFormatException` from `Lexer.getTokenAsInt` for a
    /// number outside the Java `int` range, with this message; not a
    /// `UcumException`, so no engine arm catches it (`path.quantity.024`).
    NumberFormat(String),
    /// `java.lang.NullPointerException` with this message, from `divideBy`
    /// on a pair with no code (`path.quantity.021`).
    NullPointer(String),
    /// `java.lang.Error` with this message, from `Decimal`'s arithmetic.
    Error(String),
}

impl UcumError {
    /// `getMessage()`.
    pub fn message(&self) -> &str {
        match self {
            UcumError::Ucum(m)
            | UcumError::NumberFormat(m)
            | UcumError::NullPointer(m)
            | UcumError::Error(m) => m,
        }
    }
}

impl std::fmt::Display for UcumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for UcumError {}

impl From<Failure> for UcumError {
    fn from(f: Failure) -> Self {
        match f {
            Failure::Ucum(m) => UcumError::Ucum(m),
            Failure::Error(m) => UcumError::Error(m.to_owned()),
        }
    }
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

/// `TokenType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    None,
    Number,
    Symbol,
    Solidus,
    Period,
    Open,
    Close,
    Annotation,
}

impl Tok {
    /// `TokenType.toString()`, the enum constant's name.
    fn name(self) -> &'static str {
        match self {
            Tok::None => "NONE",
            Tok::Number => "NUMBER",
            Tok::Symbol => "SYMBOL",
            Tok::Solidus => "SOLIDUS",
            Tok::Period => "PERIOD",
            Tok::Open => "OPEN",
            Tok::Close => "CLOSE",
            Tok::Annotation => "ANNOTATION",
        }
    }
}

/// `Lexer.NO_CHAR`: `Character.UNASSIGNED`, which is `0`.
const NO_CHAR: u16 = 0;

fn is_digit(ch: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&ch)
}

fn is(ch: u16, c: u8) -> bool {
    ch == u16::from(c)
}

/// A UTF-16 unit as a Java `char` prints.
fn show(ch: u16) -> String {
    String::from_utf16_lossy(&[ch])
}

struct Lexer<'a> {
    /// The code as given, for messages.
    text: &'a str,
    source: Vec<u16>,
    index: usize,
    token: String,
    ty: Tok,
    start: usize,
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str) -> Result<Self, UcumError> {
        let mut lexer = Lexer {
            text,
            source: text.encode_utf16().collect(),
            index: 0,
            token: String::new(),
            ty: Tok::None,
            start: 0,
        };
        lexer.consume()?;
        Ok(lexer)
    }

    fn consume(&mut self) -> Result<(), UcumError> {
        self.token.clear();
        self.ty = Tok::None;
        self.start = self.index;
        if self.index < self.source.len() {
            let ch = self.next_char();
            let matched = self.check_single(ch, b'/', Tok::Solidus)
                || self.check_single(ch, b'.', Tok::Period)
                || self.check_single(ch, b'(', Tok::Open)
                || self.check_single(ch, b')', Tok::Close)
                || self.check_annotation(ch)?
                || self.check_number(ch)?
                || self.check_number_or_symbol(ch)?;
            if !matched {
                return Err(UcumError::Ucum(format!(
                    "Error processing unit '{}': unexpected character '{}' at position {}",
                    self.text,
                    show(ch),
                    self.start
                )));
            }
        }
        Ok(())
    }

    fn check_number(&mut self, ch: u16) -> Result<bool, UcumError> {
        if !(is(ch, b'+') || is(ch, b'-')) {
            return Ok(false);
        }
        let mut token = vec![ch];
        let mut ch = self.peek_char();
        while is_digit(ch) {
            token.push(ch);
            self.index += 1;
            ch = self.peek_char();
        }
        if token.len() == 1 {
            return Err(UcumError::Ucum(format!(
                "Error processing unit'{}': unexpected character '{}' at position {}: a + or - \
                 must be followed by at least one digit",
                self.text,
                show(ch),
                self.start
            )));
        }
        self.token = String::from_utf16_lossy(&token);
        self.ty = Tok::Number;
        Ok(true)
    }

    fn check_number_or_symbol(&mut self, ch: u16) -> Result<bool, UcumError> {
        let mut is_symbol = false;
        let mut in_brackets = false;
        if !is_valid_symbol_char(ch, true, false) {
            return Ok(false);
        }
        let mut token = vec![ch];
        is_symbol = is_symbol || !is_digit(ch);
        in_brackets = self.check_brackets(ch, in_brackets)?;
        let mut ch = self.peek_char();
        in_brackets = self.check_brackets(ch, in_brackets)?;
        while is_valid_symbol_char(ch, !is_symbol || in_brackets, in_brackets) {
            token.push(ch);
            is_symbol = is_symbol || (ch != NO_CHAR && !is_digit(ch));
            self.index += 1;
            ch = self.peek_char();
            in_brackets = self.check_brackets(ch, in_brackets)?;
        }
        self.token = String::from_utf16_lossy(&token);
        self.ty = if is_symbol { Tok::Symbol } else { Tok::Number };
        Ok(true)
    }

    fn check_brackets(&self, ch: u16, in_brackets: bool) -> Result<bool, UcumError> {
        if is(ch, b'[') {
            if in_brackets {
                return Err(self.error("Nested ["));
            }
            return Ok(true);
        }
        if is(ch, b']') {
            if !in_brackets {
                return Err(self.error("] without ["));
            }
            return Ok(false);
        }
        Ok(in_brackets)
    }

    fn check_annotation(&mut self, ch: u16) -> Result<bool, UcumError> {
        if !is(ch, b'{') {
            return Ok(false);
        }
        let mut b = Vec::new();
        let mut ch = ch;
        while !is(ch, b'}') {
            ch = self.next_char();
            // `Utilities.isAsciiChar`: ' ' to '~'. The end of the input reads
            // as `NO_CHAR`, which is not, so the "unterminated" check after it
            // is never reached.
            if !(u16::from(b' ')..=u16::from(b'~')).contains(&ch) {
                return Err(UcumError::Ucum(format!(
                    "Error processing unit'{}': Annotation contains non-ascii characters",
                    self.text
                )));
            }
            b.push(ch);
        }
        self.token = String::from_utf16_lossy(&b);
        self.ty = Tok::Annotation;
        Ok(true)
    }

    fn check_single(&mut self, ch: u16, test: u8, ty: Tok) -> bool {
        if is(ch, test) {
            self.token = show(ch);
            self.ty = ty;
            return true;
        }
        false
    }

    fn next_char(&mut self) -> u16 {
        let res = self.source.get(self.index).copied().unwrap_or(NO_CHAR);
        self.index += 1;
        res
    }

    fn peek_char(&self) -> u16 {
        self.source.get(self.index).copied().unwrap_or(NO_CHAR)
    }

    /// `error(errMsg)`: the position is the start of the current token.
    fn error(&self, msg: &str) -> UcumError {
        UcumError::Ucum(format!(
            "Error processing unit '{}': {msg}' at position {}",
            self.text, self.start
        ))
    }

    /// `getTokenAsInt`: `Integer.parseInt`, after a leading `+`.
    fn token_as_int(&self) -> Result<i32, UcumError> {
        let text = self.token.strip_prefix('+').unwrap_or(&self.token);
        text.parse::<i32>()
            .map_err(|_| UcumError::NumberFormat(format!("For input string: \"{text}\"")))
    }

    fn finished(&self) -> bool {
        self.index == self.source.len()
    }
}

fn is_valid_symbol_char(ch: u16, allow_digits: bool, in_brackets: bool) -> bool {
    (allow_digits && is_digit(ch))
        || (u16::from(b'a')..=u16::from(b'z')).contains(&ch)
        || (u16::from(b'A')..=u16::from(b'Z')).contains(&ch)
        || b"[]%*^'\"_".iter().any(|&c| is(ch, c))
        || (in_brackets && is(ch, b'.'))
}

// ---------------------------------------------------------------------------
// ExpressionParser
// ---------------------------------------------------------------------------

/// `Operator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Multiplication,
    Division,
}

/// `Component`: a `Factor`, a `Symbol` or a parenthesised `Term`.
#[derive(Debug)]
enum Comp {
    Factor(i32),
    Symbol {
        unit: UnitRef,
        prefix: Option<usize>,
        exponent: i32,
    },
    Term(Box<Term>),
}

/// `Term`: `comp`, then `op` and the rest of the term.
#[derive(Debug, Default)]
struct Term {
    comp: Option<Comp>,
    op: Option<Op>,
    term: Option<Box<Term>>,
}

/// `ExpressionParser.parse`.
fn parse(model: &UcumEssence, code: &str) -> Result<Term, UcumError> {
    let mut lexer = Lexer::new(code)?;
    let res = parse_term(model, &mut lexer, true)?;
    if !lexer.finished() {
        return Err(UcumError::Ucum(
            "Expression was not parsed completely. Syntax Error?".to_owned(),
        ));
    }
    Ok(res)
}

fn parse_term(model: &UcumEssence, lexer: &mut Lexer<'_>, first: bool) -> Result<Term, UcumError> {
    let mut res = Term::default();
    if first && lexer.ty == Tok::None {
        res.comp = Some(Comp::Factor(1));
    } else if lexer.ty == Tok::Solidus {
        res.op = Some(Op::Division);
        lexer.consume()?;
        res.term = Some(Box::new(parse_term(model, lexer, false)?));
    } else {
        if lexer.ty == Tok::Annotation {
            // still lose the annotation
            res.comp = Some(Comp::Factor(1));
            lexer.consume()?;
        } else {
            res.comp = Some(parse_comp(model, lexer)?);
        }
        if lexer.ty != Tok::None && lexer.ty != Tok::Close {
            match lexer.ty {
                Tok::Solidus => {
                    res.op = Some(Op::Division);
                    lexer.consume()?;
                }
                Tok::Period => {
                    res.op = Some(Op::Multiplication);
                    lexer.consume()?;
                }
                // implicit
                Tok::Annotation => res.op = Some(Op::Multiplication),
                _ => return Err(lexer.error("Expected '/' or '.'")),
            }
            res.term = Some(Box::new(parse_term(model, lexer, false)?));
        }
    }
    Ok(res)
}

fn parse_comp(model: &UcumEssence, lexer: &mut Lexer<'_>) -> Result<Comp, UcumError> {
    match lexer.ty {
        Tok::Number => {
            let fact = Comp::Factor(lexer.token_as_int()?);
            lexer.consume()?;
            Ok(fact)
        }
        Tok::Symbol => parse_symbol(model, lexer),
        Tok::None => {
            Err(lexer.error("unexpected end of expression looking for a symbol or a number"))
        }
        Tok::Open => {
            lexer.consume()?;
            let res = parse_term(model, lexer, true)?;
            if lexer.ty == Tok::Close {
                lexer.consume()?;
            } else {
                return Err(lexer.error(&format!(
                    "Unexpected Token Type '{}' looking for a close bracket",
                    lexer.ty.name()
                )));
            }
            Ok(Comp::Term(Box::new(res)))
        }
        _ => Err(lexer.error("unexpected token looking for a symbol or a number")),
    }
}

fn parse_symbol(model: &UcumEssence, lexer: &mut Lexer<'_>) -> Result<Comp, UcumError> {
    let sym = lexer.token.clone();
    // now, can we pick a prefix that leaves behind a metric unit?
    let mut selected: Option<(usize, UnitRef)> = None;
    for (i, prefix) in model.prefixes().iter().enumerate() {
        if let Some(rest) = sym.strip_prefix(prefix.code.as_str()) {
            let unit = model.find_unit(rest);
            let usable = match unit {
                Some(UnitRef::Base(_)) => true,
                Some(UnitRef::Defined(u)) => model.units()[u].is_metric,
                None => false,
            };
            if let (true, Some(unit)) = (usable, unit) {
                selected = Some((i, unit));
                break;
            }
        }
    }
    let (unit, prefix) = match selected {
        Some((p, unit)) => (unit, Some(p)),
        None => match model.find_unit(&sym) {
            Some(unit) => (unit, None),
            None => return Err(lexer.error(&format!("The unit '{sym}' is unknown"))),
        },
    };
    lexer.consume()?;
    let exponent = if lexer.ty == Tok::Number {
        let e = lexer.token_as_int()?;
        lexer.consume()?;
        e
    } else {
        1
    };
    Ok(Comp::Symbol {
        unit,
        prefix,
        exponent,
    })
}

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/// `Canonical`: a value and base units with exponents.
struct Canonical {
    value: UcumDecimal,
    /// `(base-unit index, exponent)`.
    units: Vec<(usize, i32)>,
}

impl Canonical {
    /// `new Canonical(new Decimal("1.000000000000000000000000000000"))`.
    fn unit() -> Result<Self, UcumError> {
        Ok(Canonical {
            value: UcumDecimal::new("1.000000000000000000000000000000")?,
            units: Vec::new(),
        })
    }

    fn multiply_value(&mut self, by: &UcumDecimal) -> Result<(), UcumError> {
        self.value = self.value.multiply(by)?;
        Ok(())
    }

    fn divide_value(&mut self, by: &UcumDecimal) -> Result<(), UcumError> {
        self.value = self.value.divide(by)?;
        Ok(())
    }
}

/// A `special/Registry.java` handler: its units, its value and whether it
/// has an offset.
struct Handler {
    units: &'static str,
    value: fn() -> UcumDecimal,
    offset: bool,
}

/// `Registry.init`, keyed by code.
fn handler(code: &str) -> Option<Handler> {
    let holding = |units: &'static str| Handler {
        units,
        value: UcumDecimal::one,
        offset: false,
    };
    Some(match code {
        "Cel" => Handler {
            units: "K",
            value: UcumDecimal::one,
            offset: true,
        },
        // `new Decimal(5).divide(new Decimal(9))`, computed but unused, as
        // the offset check follows it.
        "[degF]" => Handler {
            units: "K",
            value: || {
                UcumDecimal::from_int(5)
                    .divide(&UcumDecimal::from_int(9))
                    .unwrap_or_default()
            },
            offset: true,
        },
        "[p'diop]" | "%[slope]" => holding("deg"),
        "[hp_X]" | "[hp_C]" | "Np" | "B" | "bit_s" => holding("1"),
        "[pH]" => holding("mol/l"),
        "B[SPL]" => Handler {
            units: "10*-5.Pa",
            value: || UcumDecimal::from_int(2),
            offset: false,
        },
        "B[V]" => holding("V"),
        "B[mV]" => holding("mV"),
        "B[uV]" => holding("uV"),
        "B[W]" => holding("W"),
        "B[kW]" => holding("kW"),
        _ => return None,
    })
}

/// `Converter.convert`: `normalise` of the term.
fn convert(model: &UcumEssence, term: &Term) -> Result<Canonical, UcumError> {
    normalise_term(model, term)
}

fn normalise_term(model: &UcumEssence, term: &Term) -> Result<Canonical, UcumError> {
    let mut result = Canonical::unit()?;
    let mut div = false;
    let mut t = Some(term);
    while let Some(cur) = t {
        match &cur.comp {
            Some(Comp::Term(inner)) => {
                let mut temp = normalise_term(model, inner)?;
                absorb(&mut result, &mut temp, div)?;
            }
            Some(Comp::Factor(f)) => {
                let f = UcumDecimal::from_int(*f);
                if div {
                    result.divide_value(&f)?;
                } else {
                    result.multiply_value(&f)?;
                }
            }
            Some(Comp::Symbol {
                unit,
                prefix,
                exponent,
            }) => {
                let mut temp = normalise_symbol(model, *unit, *prefix, *exponent)?;
                absorb(&mut result, &mut temp, div)?;
            }
            None => {}
        }
        div = cur.op == Some(Op::Division);
        t = cur.term.as_deref();
    }

    // Merge base units: each later one into the nearest earlier with the same
    // base (`st.getBase() == sf.getBase()`, an identity).
    let mut i = result.units.len();
    while i > 0 {
        i -= 1;
        let sf = result.units[i];
        for j in (0..i).rev() {
            if result.units[j].0 == sf.0 {
                result.units[j].1 = sf.1.wrapping_add(result.units[j].1);
                result.units.remove(i);
                break;
            }
        }
    }
    result.units.retain(|&(_, e)| e != 0);
    // `Collections.sort` (stable) by `String.compareTo` of the code, which
    // compares UTF-16 code units.
    result.units.sort_by(|a, b| {
        let ca = model.base_units()[a.0].code.encode_utf16();
        let cb = model.base_units()[b.0].code.encode_utf16();
        ca.cmp(cb)
    });
    Ok(result)
}

/// A normalised component: its value multiplies the result, or divides it
/// after a `/`, which also negates its exponents.
fn absorb(result: &mut Canonical, temp: &mut Canonical, div: bool) -> Result<(), UcumError> {
    if div {
        result.divide_value(&temp.value)?;
        for c in &mut temp.units {
            c.1 = 0i32.wrapping_sub(c.1);
        }
    } else {
        result.multiply_value(&temp.value)?;
    }
    result.units.append(&mut temp.units);
    Ok(())
}

fn normalise_symbol(
    model: &UcumEssence,
    unit: UnitRef,
    prefix: Option<usize>,
    exponent: i32,
) -> Result<Canonical, UcumError> {
    let mut result = Canonical::unit()?;
    match unit {
        UnitRef::Base(b) => result.units.push((b, exponent)),
        UnitRef::Defined(d) => {
            let mut can = expand_defined_unit(model, d)?;
            for c in &mut can.units {
                c.1 = c.1.wrapping_mul(exponent);
            }
            result.units.append(&mut can.units);
            if exponent > 0 {
                for _ in 0..exponent {
                    result.multiply_value(&can.value)?;
                }
            } else {
                let mut i = 0;
                while i > exponent {
                    result.divide_value(&can.value)?;
                    i -= 1;
                }
            }
        }
    }
    if let Some(p) = prefix {
        let value = model.prefix_value(p);
        if exponent > 0 {
            for _ in 0..exponent {
                result.multiply_value(value)?;
            }
        } else {
            let mut i = 0;
            while i > exponent {
                result.divide_value(value)?;
                i -= 1;
            }
        }
    }
    Ok(result)
}

fn expand_defined_unit(model: &UcumEssence, d: usize) -> Result<Canonical, UcumError> {
    let unit = &model.units()[d];
    let mut u: &str = &unit.value_unit;
    let mut v = model.unit_value(d).clone();
    if unit.is_special {
        let Some(h) = handler(&unit.code) else {
            return Err(UcumError::Ucum("Not handled yet (special unit)".to_owned()));
        };
        u = h.units;
        v = (h.value)();
        if h.offset {
            return Err(UcumError::Ucum(
                "Not handled yet (special unit with offset from 0 at intersect)".to_owned(),
            ));
        }
    }
    let t = parse(model, u)?;
    let mut result = normalise_term(model, &t)?;
    result.multiply_value(&v)?;
    Ok(result)
}

/// `ExpressionComposer.compose(can, false)`: the units joined by `.`, each
/// with its exponent unless that is `1`.
fn compose(model: &UcumEssence, can: &Canonical) -> String {
    let mut b = String::new();
    for (i, &(base, exponent)) in can.units.iter().enumerate() {
        if i > 0 {
            b.push('.');
        }
        b.push_str(&model.base_units()[base].code);
        if exponent != 1 {
            b.push_str(&exponent.to_string());
        }
    }
    b
}

// ---------------------------------------------------------------------------
// UcumEssenceService
// ---------------------------------------------------------------------------

impl UcumEssence {
    /// `UcumEssenceService.getCanonicalForm(new Pair(new Decimal(value), code))`
    /// (`UcumEssenceService.java:266-285`): the canonical value's
    /// `asDecimal()` and the canonical unit string, `""` for a dimensionless
    /// code. A whole `value` takes the `checkForCouldBeWholeNumber` step.
    pub fn canonical_form(&self, value: &str, code: &str) -> Result<(String, String), UcumError> {
        let value = UcumDecimal::new(value)?;
        self.canonical_pair(&value, code)
    }

    fn canonical_pair(
        &self,
        value: &UcumDecimal,
        code: &str,
    ) -> Result<(String, String), UcumError> {
        let term = parse(self, code)?;
        let c = convert(self, &term)?;
        let mut product = value.multiply(&c.value)?;
        if value.is_whole_number() {
            // whole numbers are tricky - they have implied infinite precision,
            // but we need to check for digit errors in the last couple of
            // digits
            product.check_for_could_be_whole_number();
        }
        Ok((product.as_decimal(), compose(self, &c)))
    }

    /// `UcumEssenceService.getCanonicalUnits` (`:232-240`): the canonical unit
    /// string, every `Exception` re-wrapped as a `UcumException` reading
    /// `Error processing <unit>: <message>`. A `java.lang.Error` is not an
    /// `Exception` and is not re-wrapped.
    pub fn canonical_units(&self, unit: &str) -> Result<String, UcumError> {
        let result = parse(self, unit).and_then(|term| {
            let c = convert(self, &term)?;
            Ok(compose(self, &c))
        });
        result.map_err(|e| match e {
            UcumError::Error(m) => UcumError::Error(m),
            other => UcumError::Ucum(format!("Error processing {unit}: {}", other.message())),
        })
    }

    /// `UcumEssenceService.isComparable` (`:333-343`): `false` for a missing
    /// unit, otherwise whether the canonical unit strings are equal; a unit
    /// that does not canonicalise raises.
    pub fn is_comparable(
        &self,
        units1: Option<&str>,
        units2: Option<&str>,
    ) -> Result<bool, UcumError> {
        let (Some(u1), Some(u2)) = (units1, units2) else {
            return Ok(false);
        };
        let u1 = self.canonical_units(u1)?;
        let u2 = self.canonical_units(u2)?;
        Ok(u1 == u2)
    }

    /// `UcumEssenceService.multiply` (`:314-317`): the values multiplied, the
    /// codes joined as `<code1>.<code2>` (a missing code spelled `null`), and
    /// the result canonicalised.
    pub fn multiply(
        &self,
        value1: &str,
        code1: Option<&str>,
        value2: &str,
        code2: Option<&str>,
    ) -> Result<(String, String), UcumError> {
        let (v1, v2) = (UcumDecimal::new(value1)?, UcumDecimal::new(value2)?);
        let value = v1.multiply(&v2)?;
        let code = format!("{}.{}", code1.unwrap_or("null"), code2.unwrap_or("null"));
        self.canonical_pair(&value, &code)
    }

    /// `UcumEssenceService.divideBy` (`:320-325`): the values divided first,
    /// so a zero divisor raises before a code is read; then each code is
    /// parenthesised when it holds `/` or `*`, a missing one dereferenced, and
    /// the joined `<dividend>/<divisor>` canonicalised.
    pub fn divide_by(
        &self,
        dividend_value: &str,
        dividend_code: Option<&str>,
        divisor_value: &str,
        divisor_code: Option<&str>,
    ) -> Result<(String, String), UcumError> {
        let (v1, v2) = (
            UcumDecimal::new(dividend_value)?,
            UcumDecimal::new(divisor_value)?,
        );
        let value = v1.divide(&v2)?;
        let side = |code: Option<&str>| match code {
            None => Err(UcumError::NullPointer(
                "Cannot invoke \"String.contains(java.lang.CharSequence)\" because the return \
                 value of \"org.fhir.ucum.Pair.getCode()\" is null"
                    .to_owned(),
            )),
            Some(c) if c.contains('/') || c.contains('*') => Ok(format!("({c})")),
            Some(c) => Ok(c.to_owned()),
        };
        let code = format!("{}/{}", side(dividend_code)?, side(divisor_code)?);
        self.canonical_pair(&value, &code)
    }
}
