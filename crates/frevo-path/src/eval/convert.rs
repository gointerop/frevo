//! Type conversion: the `toX` and `convertsToX` functions.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `funcToString` at `:4914`,
//! `funcToBoolean` at `:4928`, `funcToQuantity` at `:4957`, `funcToDateTime`
//! at `:4976`, `funcToTime` at `:4983`, `funcToDecimal` at `:4991`,
//! `funcToInteger` at `:5895`, `funcIsInteger` at `:5908`, `funcIsBoolean` at
//! `:5924`, `funcIsDateTime` at `:5942`, `funcIsDate` at `:5957`,
//! `funcIsTime` at `:5986`, `funcIsString` at `:6001`, `funcIsQuantity` at
//! `:6013`, `parseQuantityString` at `:6032` and `funcIsDecimal` at `:6077`;
//! and org.hl7.fhir.utilities `Utilities.isInteger`, `Utilities.isDecimal` /
//! `checkDecimal` and `RegexConstants.DATE_TIME_REGEX`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the three regular expressions the engine
//! matches (`DATE_TIME_REGEX` for `convertsToDate` and `convertsToDateTime`, and
//! the time pattern for `convertsToTime`) are hand-written matchers over the
//! same language, since the crate takes no regular-expression dependency; a
//! digit is an ASCII digit where Java's `Character.isDigit` also takes other
//! Unicode decimal digits; and a decimal is built from its digits and scale so
//! it renders as `BigDecimal.toString` does.
//!
//! What the pinned engine does here and the specification does not say, and
//! what is kept (`DEV-0009` and the blockers entry for task 42):
//!
//! - there is no `toDate`, and `toDateTime` and `toTime` are stubs that raise
//!   `FHIRPATH_NOT_IMPLEMENTED` (`:4976-4989`);
//! - `convertsToDate` uses the **dateTime** pattern (`:5962`, with a `FIXME`
//!   upstream), so `'2015-02-04T14'.convertsToDate()` is `true`;
//! - `convertsToString` is `false` only for a dateTime or time, so a `date` is
//!   convertible (`:6004`);
//! - `toInteger`, `toDecimal` read `convertToString(focus)` of the whole list,
//!   so a multi-item focus yields empty rather than an error; `toString` is the
//!   one that raises for more than one;
//! - `Utilities.isDecimal` admits a bare `+` or `-`, which `new BigDecimal`
//!   then refuses: `'-'.toDecimal()` is an execution error.

use crate::seam::value::{Decimal, Quantity, Value};
use crate::syntax::ast::{Function, NodeId};

use super::collection::{convert_list_to_string, convert_to_string};
use super::navigate::fhir_type;
use super::operand::{UCUM_SYSTEM, decimal_of, has_type, int_of, is_quantity, text_of};
use super::{EvalError, Run};

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Boolean(b)]
}

/// `instanceof IntegerType`: `integer`, `positiveInt` and `unsignedInt`.
fn is_integer_type(value: &Value) -> bool {
    has_type(value, &["integer", "positiveInt", "unsignedInt"])
}

/// `instanceof StringType`: `string`, `code` and `markdown`.
fn is_string_type(value: &Value) -> bool {
    has_type(value, &["string", "code", "markdown"])
}

fn is_boolean_type(value: &Value) -> bool {
    fhir_type(value) == "boolean"
}

fn is_decimal_type(value: &Value) -> bool {
    fhir_type(value) == "decimal"
}

/// `Utilities.isBlank`.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// `Utilities.isInteger`.
fn is_integer(s: &str) -> bool {
    if is_blank(s) {
        return false;
    }
    let value = s.strip_prefix('-').unwrap_or(s);
    if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) || value.len() > 10 {
        return false;
    }
    // The bound is a string comparison of the whole text, as upstream.
    if s.starts_with('-') {
        !(value.len() == 10 && s > "2147483648")
    } else {
        !(value.len() == 10 && s > "2147483647")
    }
}

/// `Utilities.isDecimal(value, allowExponent)`: true for `OK` and `RANGE`.
fn is_decimal(value: &str, allow_exponent: bool) -> bool {
    if is_blank(value) {
        return false;
    }
    if !allow_exponent_leading_zero_ok(value) {
        return false;
    }
    if value.ends_with('.') {
        return false;
    }
    let (mut period, mut exponent, mut sign, mut digits) = (false, false, false, false);
    for c in value.chars() {
        match c {
            '.' => {
                if !digits || period || exponent {
                    return false;
                }
                period = true;
            }
            '-' | '+' => {
                if digits || sign {
                    return false;
                }
                sign = true;
            }
            'e' | 'E' => {
                if !digits || exponent || !allow_exponent {
                    return false;
                }
                exponent = true;
                sign = false;
                digits = false;
            }
            c if c.is_ascii_digit() => digits = true,
            _ => return false,
        }
    }
    !(exponent && !digits)
}

/// The leading-zero check `checkDecimal` makes when `allowLeadingZero` is
/// false, which both two-argument `isDecimal` callers here are.
fn allow_exponent_leading_zero_ok(value: &str) -> bool {
    for sign in ["", "-", "+"] {
        let zero = format!("{sign}0");
        if value.starts_with(&zero) && value != zero && !value.starts_with(&format!("{zero}.")) {
            return false;
        }
    }
    true
}

/// `new BigDecimal(text)` for text `is_decimal` accepted, rendered as
/// `BigDecimal.toString` does. `None` when it is not a number (a bare sign) or
/// does not fit.
fn big_decimal(text: &str) -> Option<Decimal> {
    let (negative, rest) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i64>().ok()?),
        None => (rest, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let digits = format!("{int}{frac}");
    let unscaled: i128 = digits.parse().ok()?;
    let scale = i64::try_from(frac.len()).ok()? - exponent;
    Decimal::from_unscaled(if negative { -unscaled } else { unscaled }, scale)
}

/// `compareTo(BigDecimal.ZERO) == 0`.
fn decimal_is_zero(d: &Decimal) -> bool {
    d.mantissa() == 0
}

/// `compareTo(BigDecimal.ONE) == 0`.
fn decimal_is_one(d: &Decimal) -> bool {
    10i128
        .checked_pow(d.scale())
        .is_some_and(|one| d.mantissa() == one)
}

/// `RegexConstants.DATE_TIME_REGEX`, matched against the whole string.
fn matches_date_time(s: &str) -> bool {
    let b = s.as_bytes();
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    if !(0..4).all(digit) || &b[..4] == b"0000" {
        return false;
    }
    if b.len() == 4 {
        return true;
    }
    // -MM, with 01..12.
    let two = |i: usize| -> Option<u32> {
        (digit(i) && digit(i + 1)).then(|| u32::from(b[i] - b'0') * 10 + u32::from(b[i + 1] - b'0'))
    };
    if b[4] != b'-' || !two(5).is_some_and(|m| (1..=12).contains(&m)) {
        return false;
    }
    if b.len() == 7 {
        return true;
    }
    if b[7] != b'-' || !two(8).is_some_and(|d| (1..=31).contains(&d)) {
        return false;
    }
    if b.len() == 10 {
        return true;
    }
    b[10] == b'T' && time_tail(b, 11)
}

/// The time pattern `(T)?HH(:MM(:SS)?)?(.f+)?(zone)?`, whole string.
fn matches_time(s: &str) -> bool {
    let b = s.as_bytes();
    time_tail(b, usize::from(b.first() == Some(&b'T')))
}

/// `([01][0-9]|2[0-3])(:[0-5][0-9](:([0-5][0-9]|60))?)?(\.[0-9]+)?(zone)?`
/// from `at` to the end of `b`.
fn time_tail(b: &[u8], mut at: usize) -> bool {
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    let value = |i: usize| -> Option<u32> {
        (digit(i) && digit(i + 1)).then(|| u32::from(b[i] - b'0') * 10 + u32::from(b[i + 1] - b'0'))
    };
    if !value(at).is_some_and(|h| h <= 23) {
        return false;
    }
    at += 2;
    if b.get(at) == Some(&b':') {
        if !value(at + 1).is_some_and(|m| m <= 59) {
            return false;
        }
        at += 3;
        if b.get(at) == Some(&b':') {
            if !value(at + 1).is_some_and(|s| s <= 60) {
                return false;
            }
            at += 3;
        }
    }
    if b.get(at) == Some(&b'.') {
        let start = at + 1;
        at = start;
        while digit(at) {
            at += 1;
        }
        if at == start {
            return false;
        }
    }
    match b.get(at) {
        None => true,
        Some(b'Z') => at + 1 == b.len(),
        Some(b'+' | b'-') => {
            let zone = &b[at + 1..];
            let ok = |h: &[u8]| {
                h.len() == 5
                    && h[2] == b':'
                    && h[3] <= b'5'
                    && h[3] >= b'0'
                    && h[4].is_ascii_digit()
                    && ((h[0] == b'0' && h[1].is_ascii_digit())
                        || (h[0] == b'1' && (b'0'..=b'3').contains(&h[1])))
            };
            ok(zone) || zone == b"14:00"
        }
        Some(_) => false,
    }
}

/// `parseQuantityString` (`:6032`): `Ok(None)` is upstream's `null`; an `Err`
/// is the `StringIndexOutOfBoundsException` a lone quote raises.
fn parse_quantity_string(s: &str, at: usize) -> Result<Option<Quantity>, EvalError> {
    let s = s.trim_matches(|c: char| c <= ' ');
    let ucum = |v: &str, code: &str| -> Option<Quantity> {
        Some(Quantity {
            value: big_decimal(v)?,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some(code.to_string()),
        })
    };
    if let Some(space) = s.find(' ') {
        let v = s[..space].trim_matches(|c: char| c <= ' ');
        let unit = s[space..].trim_matches(|c: char| c <= ' ');
        if !is_decimal(v, false) {
            return Ok(None);
        }
        if unit.starts_with('\'') && unit.ends_with('\'') {
            if unit.len() < 2 {
                return Err(EvalError::execution(
                    format!("begin 1, end 0, length {}", unit.len()),
                    at,
                ));
            }
            return Ok(ucum(v, &unit[1..unit.len() - 1]));
        }
        let code = match unit {
            "year" | "years" => "a",
            "month" | "months" => "mo_s",
            "week" | "weeks" => "wk",
            "day" | "days" => "d",
            "hour" | "hours" => "h",
            "minute" | "minutes" => "min",
            "second" | "seconds" => "s",
            "millisecond" | "milliseconds" => "ms",
            _ => return Ok(None),
        };
        Ok(ucum(v, code))
    } else if is_decimal(s, true) {
        Ok(big_decimal(s).map(|value| Quantity {
            value,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("1".to_string()),
        }))
    } else {
        Ok(None)
    }
}

impl Run<'_> {
    /// `evaluateFunction` for the conversion functions.
    pub(super) fn func_convert(
        &self,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        let at = self.expr.node(id).start();
        match function {
            Function::ToInteger => Ok(to_integer(focus)),
            Function::ToDecimal => to_decimal(focus, at),
            Function::ToString => to_string(focus, at),
            Function::ToBoolean => self.to_boolean(focus, at),
            Function::ToQuantity => self.to_quantity(focus, at),
            Function::ToDateTime | Function::ToTime => Err(EvalError::execution(
                format!(
                    "Error evaluating FHIRPath expression: The function {} is not implemented",
                    function.to_code()
                ),
                at,
            )),
            other => self.converts_to(focus, other, at),
        }
    }

    fn to_boolean(&self, focus: &[Value], at: usize) -> Result<Vec<Value>, EvalError> {
        let [one] = focus else {
            return Ok(Vec::new());
        };
        if is_boolean_type(one) {
            return Ok(vec![one.clone()]);
        }
        if is_integer_type(one) {
            return Ok(match int_of(self.model, one, at)? {
                0 => vec![Value::Boolean(false)],
                1 => vec![Value::Boolean(true)],
                _ => Vec::new(),
            });
        }
        if is_decimal_type(one) {
            let d = decimal_of(self.model, one, at)?;
            return Ok(if decimal_is_zero(&d) {
                vec![Value::Boolean(false)]
            } else if decimal_is_one(&d) {
                vec![Value::Boolean(true)]
            } else {
                Vec::new()
            });
        }
        if is_string_type(one) {
            let text = text_of(self.model, one).unwrap_or_default();
            if text.eq_ignore_ascii_case("true") {
                return Ok(boolean(true));
            }
            if text.eq_ignore_ascii_case("false") {
                return Ok(boolean(false));
            }
        }
        Ok(Vec::new())
    }

    fn to_quantity(&self, focus: &[Value], at: usize) -> Result<Vec<Value>, EvalError> {
        let [one] = focus else {
            return Ok(Vec::new());
        };
        if is_quantity(self.model, one) {
            return Ok(vec![one.clone()]);
        }
        if is_string_type(one) {
            let text = text_of(self.model, one).unwrap_or_default();
            return Ok(parse_quantity_string(&text, at)?
                .map(Value::Quantity)
                .into_iter()
                .collect());
        }
        if is_integer_type(one) || is_decimal_type(one) {
            let text = text_of(self.model, one).unwrap_or_else(|| convert_to_string(one));
            return Ok(big_decimal(&text)
                .map(|value| {
                    Value::Quantity(Quantity {
                        value,
                        unit: None,
                        system: Some(UCUM_SYSTEM.to_string()),
                        code: Some("1".to_string()),
                    })
                })
                .into_iter()
                .collect());
        }
        Ok(Vec::new())
    }

    /// The `funcIsX` family: `false` unless the focus is a single item of a
    /// convertible kind.
    fn converts_to(
        &self,
        focus: &[Value],
        function: Function,
        at: usize,
    ) -> Result<Vec<Value>, EvalError> {
        let [one] = focus else {
            return Ok(boolean(false));
        };
        let string_of = || convert_to_string(one);
        let answer = match function {
            Function::ConvertsToInteger => {
                is_integer_type(one)
                    || is_boolean_type(one)
                    || (is_string_type(one) && is_integer(&string_of()))
            }
            Function::ConvertsToDecimal => {
                is_integer_type(one)
                    || is_boolean_type(one)
                    || is_decimal_type(one)
                    || (is_string_type(one) && is_decimal(&string_of(), true))
            }
            Function::ConvertsToString => !has_type(one, &["dateTime", "time"]),
            Function::ConvertsToBoolean => {
                if is_integer_type(one) {
                    (0..=1).contains(&int_of(self.model, one, at)?)
                } else if is_decimal_type(one) {
                    let d = decimal_of(self.model, one, at)?;
                    decimal_is_zero(&d) || decimal_is_one(&d)
                } else if is_boolean_type(one) {
                    true
                } else if is_string_type(one) {
                    matches!(string_of().to_lowercase().as_str(), "true" | "false")
                } else {
                    false
                }
            }
            Function::ConvertsToQuantity => {
                is_integer_type(one)
                    || is_decimal_type(one)
                    || is_quantity(self.model, one)
                    || is_boolean_type(one)
                    || (is_string_type(one)
                        && parse_quantity_string(
                            &text_of(self.model, one).unwrap_or_default(),
                            at,
                        )?
                        .is_some())
            }
            // `funcIsDate` tests the dateTime pattern too.
            Function::ConvertsToDateTime | Function::ConvertsToDate => {
                has_type(one, &["dateTime", "date"])
                    || (is_string_type(one) && matches_date_time(&string_of()))
            }
            Function::ConvertsToTime => {
                has_type(one, &["time"]) || (is_string_type(one) && matches_time(&string_of()))
            }
            other => {
                return Err(EvalError::unsupported(
                    format!("the function {}()", other.to_code()),
                    at,
                ));
            }
        };
        Ok(boolean(answer))
    }
}

fn to_integer(focus: &[Value]) -> Vec<Value> {
    let s = convert_list_to_string(focus);
    if is_integer(&s) {
        // `new IntegerType(String)` is `Integer.parseInt`, which `is_integer`
        // has already bounded.
        s.parse::<i64>().map(Value::Integer).into_iter().collect()
    } else if s == "true" {
        vec![Value::Integer(1)]
    } else if s == "false" {
        vec![Value::Integer(0)]
    } else {
        Vec::new()
    }
}

fn to_decimal(focus: &[Value], at: usize) -> Result<Vec<Value>, EvalError> {
    let s = convert_list_to_string(focus);
    let mut result = Vec::new();
    if is_decimal(&s, true) {
        let Some(d) = big_decimal(&s) else {
            return Err(EvalError::execution(
                format!(
                    "Character {s} is neither a decimal digit number, decimal point, nor \"e\" notation exponential mark."
                ),
                at,
            ));
        };
        result.push(Value::Decimal(d));
    }
    if s == "true" {
        result.extend(Decimal::parse("1").map(Value::Decimal));
    }
    if s == "false" {
        result.extend(Decimal::parse("0").map(Value::Decimal));
    }
    Ok(result)
}

fn to_string(focus: &[Value], at: usize) -> Result<Vec<Value>, EvalError> {
    let result: Vec<Value> = focus
        .iter()
        .map(|item| Value::String(convert_to_string(item)))
        .collect();
    if result.len() > 1 {
        return Err(EvalError::execution(
            format!(
                "Error evaluating FHIRPath expression: The function toString can only be used on a singleton value but found {}",
                result.len()
            ),
            at,
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: path.convert.001
    #[test]
    fn integers_are_bounded_as_utilities_is_integer_bounds_them() {
        assert!(is_integer("007"));
        assert!(is_integer("-2147483648"));
        assert!(is_integer("2147483647"));
        assert!(!is_integer("2147483648"));
        assert!(!is_integer("1.0"));
        assert!(!is_integer("-"));
        assert!(!is_integer(" "));
    }

    // covers: path.convert.002
    #[test]
    fn decimals_follow_check_decimal() {
        assert!(is_decimal("1e5", true));
        assert!(!is_decimal("1e5", false));
        assert!(is_decimal("+1.50", true));
        assert!(is_decimal("-", true), "a bare sign passes the syntax check");
        assert!(!is_decimal("1.", true));
        assert!(!is_decimal("01", false));
        assert!(is_decimal("0.1", false));
        assert!(!is_decimal("1.a", true));
        assert_eq!(big_decimal("+1.50").unwrap().lexical(), "1.50");
        assert_eq!(big_decimal("1e5").unwrap().lexical(), "1E+5");
        assert!(big_decimal("-").is_none());
    }

    // covers: path.convert.004, path.convert.005, path.convert.006
    #[test]
    fn the_date_time_and_time_patterns_match_whole_strings() {
        for s in [
            "2015",
            "2015-02",
            "2015-02-04",
            "2015-02-04T14",
            "2015-02-04T14:34:28.123",
            "2015-02-04T14:34:28Z",
            "2015-02-04T14:34:28+10:00",
            "2015-02-04T14:34:28+14:00",
        ] {
            assert!(matches_date_time(s), "{s}");
        }
        for s in [
            "abc",
            "0000",
            "2015-13",
            "2015-02-32",
            "2015-02-04T24",
            "2015-02-04T14:34:28+14:30",
            "2015-02-04T",
            "2015-02-04T14Z1",
        ] {
            assert!(!matches_date_time(s), "{s}");
        }
        for s in ["14", "T14", "14:34", "14:34:60", "14:34:28.123Z"] {
            assert!(matches_time(s), "{s}");
        }
        for s in ["24", "14:60", "abc", "14:34:28.", ""] {
            assert!(!matches_time(s), "{s}");
        }
    }

    // covers: path.convert.008
    #[test]
    fn quantity_strings_follow_parse_quantity_string() {
        let q = |s: &str| parse_quantity_string(s, 0).unwrap();
        assert_eq!(q("1 day").unwrap().code.as_deref(), Some("d"));
        assert_eq!(q("1 'wk'").unwrap().code.as_deref(), Some("wk"));
        assert_eq!(q("1.5").unwrap().code.as_deref(), Some("1"));
        assert!(q("1 wk").is_none());
        assert!(q("1.a").is_none());
        assert!(q("01 day").is_none());
        assert!(parse_quantity_string("1 '", 0).is_err());
    }
}
