//! The boundary functions: `lowBoundary`, `highBoundary` and `precision`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`funcLowBoundary` at `:4290`, `funcHighBoundary` at `:4334`,
//! `funcPrecision` at `:4378`),
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/Utilities.java
//! (`lowBoundaryForDecimal` at `:1631`, `applyPrecision` at `:1653`,
//! `minusOne` at `:1684`, `highBoundaryForDecimal` at `:1697`, `isZero` at
//! `:1718`, `getDecimalPrecision` at `:1722`) and
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/DateTimeUtil.java
//! (`lowBoundaryForDate` at `:65`, `lowBoundaryForTime` at `:100`,
//! `highBoundaryForTime` at `:115`, `applyDatePrecision` at `:130`,
//! `applyTimePrecision` at `:146`, `highBoundaryForDate` at `:156`,
//! `splitTimezone` at `:184`, `getDatePrecision` / `getTimePrecision` at
//! `:203`-`:209`, `dayCount` at `:211`), all at
//! 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the string algorithms work over a `Vec` of
//! characters; a Java exception (`StringIndexOutOfBoundsException`,
//! `NumberFormatException`, `FHIRException`) is an [`EvalError`]; the result
//! of `new DecimalType(s)` is a [`Decimal`] that keeps `s` as its lexical form,
//! as `DecimalType.getValueAsString()` does.
//!
//! The decimal boundary is computed on the **text** of the number, as
//! upstream, and the text is what the result keeps. What the pinned code does
//! that a numeric implementation would not, all kept:
//!
//! - there is no carry: rounding up a `9` yields the character after it, `:`,
//!   and `new DecimalType` then fails, so `9.95.highBoundary(1)` and
//!   `(-9).lowBoundary(0)` are execution errors;
//! - there is no borrow normalisation: `10.lowBoundary(0)` is `09`;
//! - only a lower-case `e` is split off as an exponent, and the exponent's
//!   digits are appended to the result with no `e` before them, so `9.5e2`
//!   has a low boundary at scale 0 of `92`;
//! - an out-of-range precision (below 0 or above 16) is empty for a decimal
//!   or an integer, but a quantity's value is not range-checked, and a text
//!   cut to nothing is a quantity with no value (`path.boundary.012`);
//! - precision 14 leaves `hh:mm:` before the offset, whose first two
//!   characters `new DateTimeType` reads as the seconds (`path.boundary.015`,
//!   in [`super::temporal`]);
//! - a date or dateTime boundary is always a `dateTime` (`new DateTimeType`),
//!   even where the text has no time;
//! - a focus of a type the function does not take is empty: the
//!   `makeException(...)` in the last arm is built and never thrown;
//! - `precision()` raises on an empty focus as well as on a many-item one.
//!
//! HAPI's recorded run of the pinned suite fails the same cases these produce
//! (`oracles/fhirpath/hapi-8.12.0-results.json:954-962`, `:987-995`). They
//! differ from what `path.boundary.001`, `.004`, `.005`, `.006`, `.007` and
//! `.009` expect; the conflict is recorded in `tasks/blockers.md`.

use std::sync::Arc;

use crate::seam::node::{Node, NodeError};
use crate::seam::value::{
    Decimal, PartialDateTime, PartialTime, Primitive, TemporalPrecision, Value,
};
use crate::syntax::ast::{Function, NodeId};

use super::arith::too_wide;
use super::hostfn::MadePrimitive;
use super::operand::{Qty, has_type, int_of, qty_of, text_of};
use super::temporal::Dt;
use super::{Context, EvalError, Run};

/// The fraction appended to a decimal with a point.
const HALF_AFTER_POINT: &str = "50000000000000000000000000000";
/// The fraction appended to a decimal without one.
const HALF_WITH_POINT: &str = ".50000000000000000000000000000";
/// `lowBoundaryForDecimal` of a zero.
const LOW_OF_ZERO: &str = "-0.5000000000000000000000000";
/// `highBoundaryForDecimal` of a zero.
const HIGH_OF_ZERO: &str = "0.50000000000000000000000000000";

fn focus_error(name: &str, count: usize, at: usize) -> EvalError {
    EvalError::execution(
        format!(
            "Error evaluating FHIRPath expression: focus for {name} can only have one value, \
             but has {count} values"
        ),
        at,
    )
}

/// The `NumberFormatException` `new BigDecimal(s)` raises for a character it
/// does not read.
fn not_a_decimal_character(bad: char, at: usize) -> EvalError {
    EvalError::execution(
        format!(
            "Character {bad} is neither a decimal digit number, decimal point, nor \"e\" \
             notation exponential mark."
        ),
        at,
    )
}

/// A `StringIndexOutOfBoundsException` from `charAt`.
fn out_of_bounds(index: i64, length: usize, at: usize) -> EvalError {
    EvalError::execution(
        format!("Index {index} out of bounds for length {length}"),
        at,
    )
}

/// `s.substring(from, to)` over characters.
fn substring(s: &[char], from: i64, to: i64, at: usize) -> Result<String, EvalError> {
    let len = i64::try_from(s.len()).unwrap_or(i64::MAX);
    let range = usize::try_from(from)
        .ok()
        .zip(usize::try_from(to).ok())
        .filter(|(f, t)| f <= t && *t <= s.len());
    match range {
        Some((f, t)) => Ok(s[f..t].iter().collect()),
        None => Err(EvalError::execution(
            format!("begin {from}, end {to}, length {len}"),
            at,
        )),
    }
}

/// `s.charAt(i)` over characters.
fn char_at(s: &[char], index: i64, at: usize) -> Result<char, EvalError> {
    usize::try_from(index)
        .ok()
        .and_then(|i| s.get(i).copied())
        .ok_or_else(|| out_of_bounds(index, s.len(), at))
}

/// `(char) (c + delta)`.
fn shift_char(c: char, delta: i32) -> char {
    let code = i64::from(u32::from(c)) + i64::from(delta);
    u32::try_from(code)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// Split off the part after the first lower-case `e`, as both boundary
/// functions do: `(mantissa, exponent digits)`.
fn split_exponent(value: &str) -> (&str, &str) {
    match value.find('e') {
        Some(i) => (&value[..i], &value[i + 1..]),
        None => (value, ""),
    }
}

/// `Utilities.isZero`.
fn is_zero(value: &str) -> bool {
    value.chars().all(|c| matches!(c, '.' | '-' | '0'))
}

/// `Utilities.getDecimalPrecision`: the length of the second element of
/// `value.split("\\.")`, the exponent cut off first.
fn decimal_precision(value: &str, at: usize) -> Result<i64, EvalError> {
    let value = split_exponent(value).0;
    if !value.contains('.') {
        return Ok(0);
    }
    // `String.split` drops trailing empty elements, so `"1."` has one part
    // and indexing the second one fails.
    let mut parts: Vec<&str> = value.split('.').collect();
    while parts.len() > 1 && parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    match parts.get(1) {
        Some(fraction) => Ok(i64::try_from(fraction.chars().count()).unwrap_or(i64::MAX)),
        None => Err(out_of_bounds(1, parts.len(), at)),
    }
}

/// `Utilities.minusOne`: decrement the last digit that is not `0`, turning
/// every `0` after it into `9`. A leading run of zeros is kept, which is what
/// makes `10` become `09`.
fn minus_one(value: &str) -> String {
    let mut s: Vec<char> = value.chars().collect();
    for c in s.iter_mut().rev() {
        if *c == '0' {
            *c = '9';
        } else if *c != '.' {
            *c = shift_char(*c, -1);
            break;
        }
    }
    s.into_iter().collect()
}

/// `Utilities.applyPrecision(v, p, down)`.
fn apply_precision(v: &str, p: i32, down: bool, at: usize) -> Result<String, EvalError> {
    let chars: Vec<char> = v.chars().collect();
    let dp = chars.iter().position(|c| *c == '.');
    // `nv.substring(0, dp) + nv.substring(dp + 1)`: the first point only.
    let nv: Vec<char> = chars
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != dp)
        .map(|(_, c)| *c)
        .collect();
    let d = i64::from(p) - decimal_precision(v, at)?;
    let mut s: String = if d == 0 {
        nv.iter().collect()
    } else if d > 0 {
        let mut s: String = nv.iter().collect();
        s.extend(std::iter::repeat_n('0', usize::try_from(d).unwrap_or(0)));
        s
    } else {
        let l = i64::try_from(chars.len()).unwrap_or(i64::MAX);
        let mut ld = l + d;
        if dp.is_some() {
            ld -= 1;
        }
        if char_at(&nv, ld, at)? >= '5' && !down {
            let mut s = substring(&nv, 0, ld - 1, at)?;
            s.push(shift_char(char_at(&nv, ld - 1, at)?, 1));
            s
        } else {
            substring(&nv, 0, ld, at)?
        }
    };
    if s.ends_with('.') {
        s.pop();
    }
    let s_chars: Vec<char> = s.chars().collect();
    Ok(match dp {
        Some(dp) if dp < s_chars.len() => {
            let mut out: String = s_chars[..dp].iter().collect();
            out.push('.');
            out.extend(s_chars[dp..].iter());
            out
        }
        _ => s,
    })
}

/// `Utilities.lowBoundaryForDecimal`.
fn low_boundary_for_decimal(
    value: Option<&str>,
    precision: i32,
    at: usize,
) -> Result<String, EvalError> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Err(EvalError::execution(
            "Unable to calculate lowBoundary for a null decimal string",
            at,
        ));
    };
    let (value, e) = split_exponent(value);
    if is_zero(value) {
        return apply_precision(LOW_OF_ZERO, precision, true, at);
    }
    let body = if let Some(rest) = value.strip_prefix('-') {
        format!("-{}", high_boundary_for_decimal(Some(rest), precision, at)?)
    } else if value.contains('.') {
        let v = format!("{}{HALF_AFTER_POINT}", minus_one(value));
        apply_precision(&v, precision, true, at)?
    } else {
        let v = format!("{}{HALF_WITH_POINT}", minus_one(value));
        apply_precision(&v, precision, true, at)?
    };
    Ok(format!("{body}{e}"))
}

/// `Utilities.highBoundaryForDecimal`.
fn high_boundary_for_decimal(
    value: Option<&str>,
    precision: i32,
    at: usize,
) -> Result<String, EvalError> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Err(EvalError::execution(
            "Unable to calculate highBoundary for a null decimal string",
            at,
        ));
    };
    let (value, e) = split_exponent(value);
    if is_zero(value) {
        return apply_precision(HIGH_OF_ZERO, precision, false, at);
    }
    let body = if let Some(rest) = value.strip_prefix('-') {
        format!("-{}", low_boundary_for_decimal(Some(rest), precision, at)?)
    } else if value.contains('.') {
        apply_precision(&format!("{value}{HALF_AFTER_POINT}"), precision, false, at)?
    } else {
        apply_precision(&format!("{value}{HALF_WITH_POINT}"), precision, false, at)?
    };
    Ok(format!("{body}{e}"))
}

/// `new DecimalType(s)`: `new BigDecimal(s)` must accept the text, and the
/// value keeps it verbatim.
fn decimal_type(s: &str, at: usize) -> Result<Decimal, EvalError> {
    if let Some(d) = Decimal::parse(s) {
        return Ok(d);
    }
    // A character `BigDecimal` does not read is named, whether or not a digit
    // came before it: `9` rounded up is `:`, and `-:` fails on the `:`.
    if let Some(bad) = s
        .chars()
        .find(|c| !(c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')))
    {
        return Err(not_a_decimal_character(bad, at));
    }
    // `applyPrecision` can cut the text to a bare sign, which `BigDecimal`
    // rejects outright.
    if !s.chars().any(|c| c.is_ascii_digit()) {
        return Err(EvalError::execution(
            format!("the boundary \"{s}\" is not a decimal"),
            at,
        ));
    }
    // A sign after the first character with no exponent mark before it, as
    // `1.45000000-3` has: `BigDecimal` names the sign.
    if !s.contains(['e', 'E'])
        && let Some(bad) = s.chars().skip(1).find(|c| matches!(c, '-' | '+'))
    {
        return Err(not_a_decimal_character(bad, at));
    }
    // Every character is one `BigDecimal` reads: a value wider than this
    // decimal type holds.
    Err(too_wide(at))
}

/// `base.copy()` with `setProperty("value", <a null DecimalType>)`: the
/// quantity with every field but its value. A [`Value::Quantity`] always has a
/// value, so the result is a `Quantity` node: an element's copy hides only its
/// `value`, a literal's is built from its unit, code and system.
fn valueless_quantity(base: &Value, q: Qty) -> Value {
    let node = match base {
        Value::Node(node) => ValuelessQuantity::Copy(Arc::clone(node)),
        _ => ValuelessQuantity::Literal(q),
    };
    Value::Node(Arc::new(node))
}

/// A quantity whose value is null.
#[derive(Debug)]
enum ValuelessQuantity {
    /// A `Quantity` (or `Age`, `Duration`, ...) element, less its `value`.
    Copy(Arc<dyn Node>),
    /// A quantity literal's fields.
    Literal(Qty),
}

impl ValuelessQuantity {
    fn literal_field(q: &Qty, name: &str) -> Option<Value> {
        let (type_name, value) = match name {
            "unit" => ("string", q.unit.as_ref()),
            "system" => ("uri", q.system.as_ref()),
            "code" => ("code", q.code.as_ref()),
            _ => return None,
        };
        value.map(|v| {
            Value::Node(Arc::new(MadePrimitive {
                type_name,
                value: v.clone(),
            }))
        })
    }
}

impl Node for ValuelessQuantity {
    fn type_name(&self) -> &str {
        match self {
            Self::Copy(node) => node.type_name(),
            Self::Literal(_) => "Quantity",
        }
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        match self {
            Self::Copy(_) if name == "value" => Ok(Vec::new()),
            Self::Copy(node) => node.children(name),
            Self::Literal(q) => Ok(Self::literal_field(q, name).into_iter().collect()),
        }
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        match self {
            Self::Copy(node) => Ok(node
                .all_children()?
                .into_iter()
                .filter(|(name, _)| name != "value")
                .collect()),
            Self::Literal(q) => Ok(["unit", "system", "code"]
                .into_iter()
                .filter_map(|name| Self::literal_field(q, name).map(|v| (name.to_string(), v)))
                .collect()),
        }
    }

    fn primitive(&self) -> Option<Primitive> {
        None
    }
}

/// `DateTimeUtil.splitTimezone`: `(value, zone)`.
fn split_timezone(value: &str) -> (&str, &str) {
    if let Some(i) = value.find('+') {
        return (&value[..i], &value[i..]);
    }
    if let (Some(t), Some(dash)) = (value.find('T'), value.rfind('-'))
        && dash > t
    {
        return (&value[..dash], &value[dash..]);
    }
    if let Some(i) = value.find('Z') {
        return (&value[..i], &value[i..]);
    }
    (value, "")
}

/// `DateTimeUtil.dayCount`.
fn day_count(y: i32, m: i32) -> &'static str {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => "31",
        2 if (y % 4 == 0) && (y % 400 == 0 || y % 100 != 0) => "29",
        2 => "28",
        _ => "30",
    }
}

/// `Integer.parseInt(text)`.
fn parse_int(text: &str, at: usize) -> Result<i32, EvalError> {
    text.parse::<i32>()
        .map_err(|_| EvalError::execution(format!("For input string: \"{text}\""), at))
}

/// `DateTimeUtil.applyDatePrecision`.
fn apply_date_precision(v: &str, precision: i32, at: usize) -> Result<String, EvalError> {
    let chars: Vec<char> = v.chars().collect();
    match precision {
        4 => substring(&chars, 0, 4, at),
        6 | 7 => substring(&chars, 0, 7, at),
        8 | 10 => substring(&chars, 0, 10, at),
        14 => substring(&chars, 0, 17, at),
        17 => Ok(v.to_string()),
        other => Err(EvalError::execution(
            format!("Unsupported Date precision for boundary operation: {other}"),
            at,
        )),
    }
}

/// `DateTimeUtil.applyTimePrecision`.
fn apply_time_precision(v: &str, precision: i32, at: usize) -> Result<String, EvalError> {
    let chars: Vec<char> = v.chars().collect();
    match precision {
        2 => substring(&chars, 0, 3, at),
        4 => substring(&chars, 0, 6, at),
        6 => substring(&chars, 0, 9, at),
        9 => Ok(v.to_string()),
        other => Err(EvalError::execution(
            format!("Unsupported Time precision for boundary operation: {other}"),
            at,
        )),
    }
}

/// `DateTimeUtil.lowBoundaryForDate` and `highBoundaryForDate`.
fn boundary_for_date(
    value: &str,
    precision: i32,
    high: bool,
    at: usize,
) -> Result<String, EvalError> {
    let (date, zone) = split_timezone(value);
    let mut b = date.to_string();
    let len = |b: &String| b.chars().count();
    if len(&b) == 4 {
        b.push_str(if high { "-12" } else { "-01" });
    }
    if len(&b) == 7 {
        if high {
            let chars: Vec<char> = b.chars().collect();
            let y = parse_int(&substring(&chars, 0, 4, at)?, at)?;
            let m = parse_int(&substring(&chars, 5, 7, at)?, at)?;
            b.push('-');
            b.push_str(day_count(y, m));
        } else {
            b.push_str("-01");
        }
    }
    if len(&b) == 10 {
        b.push_str(if high { "T23:59" } else { "T00:00" });
    }
    if len(&b) == 16 {
        b.push_str(if high { ":59" } else { ":00" });
    }
    if len(&b) == 19 {
        b.push_str(if high { ".999" } else { ".000" });
    }
    let tz = if precision <= 10 {
        ""
    } else if zone.is_empty() {
        // `defLowTimezone` is Kiribati's permanent zone, the earliest instant;
        // `defHighTimezone` the westernmost, the latest.
        if high { "-12:00" } else { "+14:00" }
    } else {
        zone
    };
    Ok(format!("{}{tz}", apply_date_precision(&b, precision, at)?))
}

/// `DateTimeUtil.lowBoundaryForTime` and `highBoundaryForTime`.
fn boundary_for_time(
    value: &str,
    precision: i32,
    high: bool,
    at: usize,
) -> Result<String, EvalError> {
    let (time, zone) = split_timezone(value);
    let mut b = time.to_string();
    let len = |b: &String| b.chars().count();
    if len(&b) == 2 {
        b.push_str(if high { ":59" } else { ":00" });
    }
    if len(&b) == 5 {
        b.push_str(if high { ":59" } else { ":00" });
    }
    if len(&b) == 8 {
        b.push_str(if high { ".999" } else { ".000" });
    }
    Ok(format!(
        "{}{zone}",
        apply_time_precision(&b, precision, at)?
    ))
}

/// `new DateTimeType(s)`: the text must be a date or dateTime, and the value
/// keeps it verbatim. Always a `dateTime`, whatever precision the text has.
fn date_time_type(s: String, at: usize) -> Result<Value, EvalError> {
    let Some(dt) = Dt::parse(&s, true) else {
        return Err(EvalError::execution(
            format!("Invalid date/time format: \"{s}\""),
            at,
        ));
    };
    Ok(Value::DateTime(PartialDateTime {
        lexical: s,
        precision: dt.precision,
        offset_minutes: dt.offset,
    }))
}

/// `new TimeType(s)`, which does not validate its text.
fn time_type(s: String) -> Value {
    let precision = match time_precision(&s) {
        0..=2 => TemporalPrecision::Hour,
        3..=4 => TemporalPrecision::Minute,
        5..=6 => TemporalPrecision::Second,
        _ => TemporalPrecision::Millisecond,
    };
    Value::Time(PartialTime {
        lexical: s,
        precision,
    })
}

/// `DateTimeUtil.getDatePrecision`.
fn date_precision(value: &str) -> usize {
    split_timezone(value)
        .0
        .chars()
        .filter(|c| !matches!(c, '-' | 'T' | ':' | '.'))
        .count()
}

/// `DateTimeUtil.getTimePrecision`.
fn time_precision(value: &str) -> usize {
    split_timezone(value)
        .0
        .chars()
        .filter(|c| !matches!(c, 'T' | ':' | '.'))
        .count()
}

impl Run<'_> {
    /// `funcLowBoundary` and `funcHighBoundary`.
    pub(super) fn func_boundary(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let at = node.start();
        let high = function == Function::HighBoundary;
        let base = match focus {
            [] => return Ok(Vec::new()),
            [base] => base,
            many => return Err(focus_error(function.to_code(), many.len(), at)),
        };
        let mut precision: Option<i32> = None;
        if let Some(&param) = node.parameters().first() {
            let values = self.execute(context, focus, param, true)?;
            let [one] = values.as_slice() else {
                // Both functions name `lowBoundary` here upstream.
                return Err(EvalError::execution(
                    "Error evaluating FHIRPath expression: The parameter type Multiple Values \
                     is not legal for lowBoundary parameter 0. expecting integer",
                    at,
                ));
            };
            precision = Some(int_of(self.model, one, at)?);
        }
        let decimal_boundary = |value: Option<&str>, p: i32| {
            if high {
                high_boundary_for_decimal(value, p, at)
            } else {
                low_boundary_for_decimal(value, p, at)
            }
        };

        if has_type(base, &["decimal"]) || has_type(base, &["integer"]) {
            if !precision.is_none_or(|p| (0..17).contains(&p)) {
                return Ok(Vec::new());
            }
            let text = text_of(self.model, base);
            let s = decimal_boundary(text.as_deref(), precision.unwrap_or(8))?;
            return Ok(vec![Value::Decimal(decimal_type(&s, at)?)]);
        }
        if has_type(base, &["date"]) || has_type(base, &["dateTime"]) {
            let default = if has_type(base, &["date"]) { 10 } else { 17 };
            let text = text_of(self.model, base).unwrap_or_default();
            let s = boundary_for_date(&text, precision.unwrap_or(default), high, at)?;
            return Ok(vec![date_time_type(s, at)?]);
        }
        if has_type(base, &["time"]) {
            let text = text_of(self.model, base).unwrap_or_default();
            let s = boundary_for_time(&text, precision.unwrap_or(9), high, at)?;
            return Ok(vec![time_type(s)]);
        }
        if has_type(base, &["Quantity"]) {
            // No range check on the precision here upstream.
            let q = qty_of(base)?;
            let s = decimal_boundary(q.value.as_deref(), precision.unwrap_or(8))?;
            // `new DecimalType(s)` of a blank text is a null value
            // (`DecimalType.setValueAsString`), and the copy keeps no value.
            if s.trim().is_empty() {
                return Ok(vec![valueless_quantity(base, q)]);
            }
            return Ok(vec![q.to_value(decimal_type(&s, at)?)]);
        }
        // `makeException(...)` built and not thrown.
        Ok(Vec::new())
    }

    /// `funcPrecision`.
    pub(super) fn func_precision(
        &self,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let at = self.expr.node(id).start();
        let [base] = focus else {
            // Upstream names `highBoundary` in this message.
            return Err(focus_error("highBoundary", focus.len(), at));
        };
        let count = if has_type(base, &["decimal"]) {
            let text = text_of(self.model, base).unwrap_or_default();
            decimal_precision(&text, at)?
        } else if has_type(base, &["date"]) || has_type(base, &["dateTime"]) {
            let text = text_of(self.model, base).unwrap_or_default();
            i64::try_from(date_precision(&text)).unwrap_or(i64::MAX)
        } else if has_type(base, &["time"]) {
            let text = text_of(self.model, base).unwrap_or_default();
            i64::try_from(time_precision(&text)).unwrap_or(i64::MAX)
        } else {
            // `makeException(...)` built and not thrown.
            return Ok(Vec::new());
        };
        Ok(vec![Value::Integer(count)])
    }
}
