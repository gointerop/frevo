//! Arithmetic: `+`, `-`, `*`, `/`, `div`, `mod` and `&`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `opPlus` at `:2679`,
//! `opTimes` at `:2768`, `opConcatenate` at `:2808`, `opMinus` at `:2939`,
//! `opDivideBy` at `:2982`, `opDiv` at `:3028`, `opMod` at `:3068`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: decimals are an `i128` mantissa and a scale
//! with checked arithmetic instead of `BigDecimal`, so a result wider than 38
//! digits is [`EvalError::Unsupported`] rather than arbitrary-precision.
//!
//! What the pinned engine does here and the specification does not say, and
//! what is kept:
//!
//! - integer arithmetic is 32-bit and **wraps**: `2147483647 + 1` is
//!   `-2147483648`, and still an `integer`;
//! - `/` answers a decimal, `div` an integer, for every operand pair, and
//!   `div` and `mod` over two integers answer empty for a zero divisor;
//! - the decimal arms of `/`, `div` and `mod` go through the UCUM library's own
//!   `Decimal` (`org.fhir.ucum.Decimal`), not `BigDecimal`: a zero **dividend**
//!   answers zero even when the divisor is zero, a non-zero dividend over a
//!   zero divisor is empty for `/` and `div` and an **error** for `mod`;
//! - `0 - q` for a quantity `q` is `q` with its value's absolute value, which
//!   is how a unary minus on a quantity literal is evaluated, and `q - q` is
//!   empty because the branch yields nothing unless the left operand is `0`;
//! - date and dateTime plus or minus a quantity is `dateAdd` (task 38,
//!   `temporal::date_add`); quantity arithmetic that goes through UCUM is
//!   task 43's and is [`EvalError::Unsupported`].
//!
//! The one place this module is not a port: `org.fhir.ucum.Decimal.divide`
//! decides how many digits a non-terminating quotient carries, and that
//! library is not in `reference/`. A quotient that terminates is exact and
//! written with no trailing zeros; one that does not is
//! [`EvalError::Unsupported`] rather than a guess at its digits.

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Value};

use super::EvalError;
use super::navigate::fhir_type;
use super::operand::{self, STRING_TYPES, has_type, int_of, is_primitive, is_quantity, text_of};
use super::temporal;

/// Which operator.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Arith {
    Plus,
    Minus,
    Times,
    DivideBy,
    Div,
    Mod,
}

impl Arith {
    fn symbol(self) -> &'static str {
        match self {
            Arith::Plus => "+",
            Arith::Minus => "-",
            Arith::Times => "*",
            Arith::DivideBy => "/",
            Arith::Div => "div",
            Arith::Mod => "mod",
        }
    }
}

fn error(message: String, at: usize) -> EvalError {
    EvalError::execution(
        format!("Error evaluating FHIRPath expression: {message}"),
        at,
    )
}

pub(super) fn too_wide(at: usize) -> EvalError {
    EvalError::unsupported("decimal arithmetic beyond 38 significant digits", at)
}

/// A decimal written out in plain notation (`BigDecimal.toPlainString`).
pub(super) fn make_decimal(mantissa: i128, scale: u32, at: usize) -> Result<Decimal, EvalError> {
    let digits = mantissa.unsigned_abs().to_string();
    let mut text = String::new();
    if mantissa < 0 {
        text.push('-');
    }
    let scale = scale as usize;
    if scale == 0 {
        text.push_str(&digits);
    } else {
        let padded = format!("{digits:0>width$}", width = scale + 1);
        let split = padded.len() - scale;
        text.push_str(&padded[..split]);
        text.push('.');
        text.push_str(&padded[split..]);
    }
    Decimal::parse(&text).ok_or_else(|| too_wide(at))
}

/// Both mantissas at the larger of the two scales.
fn align(a: &Decimal, b: &Decimal, at: usize) -> Result<(i128, i128, u32), EvalError> {
    let scale = a.scale().max(b.scale());
    let lift = |d: &Decimal| {
        10i128
            .checked_pow(scale - d.scale())
            .and_then(|f| d.mantissa().checked_mul(f))
            .ok_or_else(|| too_wide(at))
    };
    Ok((lift(a)?, lift(b)?, scale))
}

fn dec_value(d: Decimal) -> Vec<Value> {
    vec![Value::Decimal(d)]
}

fn int_value(i: i32) -> Vec<Value> {
    vec![Value::Integer(i64::from(i))]
}

/// What `org.fhir.ucum.Decimal.divide` gives, for the cases that do not need
/// its digit-count rules.
enum Quotient {
    /// The dividend is zero.
    Zero,
    /// A non-zero dividend over a zero divisor.
    ByZero,
    /// The quotient terminates; the value as a decimal.
    Exact(Decimal),
    /// The quotient does not terminate within the digits this port carries.
    Inexact,
}

fn divide(a: &Decimal, b: &Decimal, at: usize) -> Result<Quotient, EvalError> {
    if a.mantissa() == 0 {
        return Ok(Quotient::Zero);
    }
    if b.mantissa() == 0 {
        return Ok(Quotient::ByZero);
    }
    let negative = (a.mantissa() < 0) != (b.mantissa() < 0);
    let (num, den) = (a.mantissa().unsigned_abs(), b.mantissa().unsigned_abs());
    let whole = num / den;
    let mut rem = num % den;
    let mut fraction = String::new();
    while rem != 0 {
        if fraction.len() >= 36 {
            return Ok(Quotient::Inexact);
        }
        rem = rem.checked_mul(10).ok_or_else(|| too_wide(at))?;
        fraction.push(char::from(b'0' + u8::try_from(rem / den).unwrap_or(0)));
        rem %= den;
    }
    let digits = format!("{whole}{fraction}");
    let mut mantissa: i128 = digits.parse().map_err(|_| too_wide(at))?;
    // The quotient of the mantissas is scaled by `10^(sb - sa)`.
    let mut scale = i64::try_from(fraction.len()).unwrap_or(i64::MAX)
        - (i64::from(b.scale()) - i64::from(a.scale()));
    while scale < 0 {
        mantissa = mantissa.checked_mul(10).ok_or_else(|| too_wide(at))?;
        scale += 1;
    }
    if negative {
        mantissa = -mantissa;
    }
    let scale = u32::try_from(scale).map_err(|_| too_wide(at))?;
    Ok(Quotient::Exact(make_decimal(mantissa, scale, at)?))
}

/// `divInt` of the UCUM `Decimal`: the quotient truncated toward zero, or
/// `None` for a zero divisor.
fn div_int(a: &Decimal, b: &Decimal, at: usize) -> Result<Option<i128>, EvalError> {
    let (x, y, _) = align(a, b, at)?;
    if y == 0 {
        return Ok(None);
    }
    Ok(Some(x / y))
}

/// The preconditions every operator checks before it looks at types.
fn operands<'a>(
    model: &dyn Model,
    left: &'a [Value],
    right: &'a [Value],
    op: Arith,
    at: usize,
) -> Result<Option<(&'a Value, &'a Value)>, EvalError> {
    if left.is_empty() || right.is_empty() {
        return Ok(None);
    }
    let sym = op.symbol();
    if left.len() > 1 {
        return Err(error(
            format!(
                "left operand to {sym} can only have 1 value, but has {} values",
                left.len()
            ),
            at,
        ));
    }
    let l = &left[0];
    let left_ok = match op {
        Arith::Plus | Arith::Mod => is_primitive(model, l),
        Arith::Minus => is_primitive(model, l) || has_type(l, &["Quantity"]),
        Arith::Times | Arith::DivideBy | Arith::Div => {
            is_primitive(model, l) || is_quantity(model, l)
        }
    };
    if !left_ok {
        return Err(error(
            format!("left operand to {sym} has the wrong type {}", fhir_type(l)),
            at,
        ));
    }
    if right.len() > 1 {
        return Err(error(
            format!(
                "right operand to {sym} can only have 1 value, but has {} values",
                right.len()
            ),
            at,
        ));
    }
    let r = &right[0];
    let right_ok = is_primitive(model, r)
        || match op {
            Arith::Plus | Arith::Minus => {
                (has_type(l, &["date", "dateTime", "instant", "Quantity"])
                    || text_of(model, l).as_deref() == Some("0"))
                    && has_type(r, &["Quantity"])
            }
            Arith::Times | Arith::DivideBy | Arith::Div => is_quantity(model, r),
            Arith::Mod => false,
        };
    if !right_ok {
        return Err(error(
            format!("right operand to {sym} has the wrong type {}", fhir_type(r)),
            at,
        ));
    }
    Ok(Some((l, r)))
}

fn incompatible(op: Arith, l: &Value, r: &Value, at: usize) -> EvalError {
    EvalError::execution(
        format!(
            "Error evaluating FHIRPath expression {}: left and right operand have incompatible \
             or invalid types ({}, {})",
            op.symbol(),
            fhir_type(l),
            fhir_type(r)
        ),
        at,
    )
}

fn is_date_time_type(v: &Value) -> bool {
    has_type(v, &["date", "dateTime", "instant"])
}

/// `opPlus`, `opMinus`, `opTimes`, `opDivideBy`, `opDiv` and `opMod`.
pub(super) fn op_arith(
    model: &dyn Model,
    left: &[Value],
    op: Arith,
    right: &[Value],
    at: usize,
) -> Result<Vec<Value>, EvalError> {
    let Some((l, r)) = operands(model, left, right, op, at)? else {
        return Ok(Vec::new());
    };
    let both = |names: &[&str]| has_type(l, names) && has_type(r, names);
    let numeric = ["decimal", "integer"];
    match op {
        Arith::Plus => {
            if both(&STRING_TYPES) {
                let text = |v: &Value| text_of(model, v).unwrap_or_else(|| "null".to_string());
                return Ok(vec![Value::String(format!("{}{}", text(l), text(r)))]);
            }
            if both(&["integer"]) {
                let sum = int_of(model, l, at)?.wrapping_add(int_of(model, r, at)?);
                return Ok(int_value(sum));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                let (x, y, scale) = align(&a, &b, at)?;
                let sum = x.checked_add(y).ok_or_else(|| too_wide(at))?;
                return Ok(dec_value(make_decimal(sum, scale, at)?));
            }
            if has_type(r, &["Quantity"]) && is_date_time_type(l) {
                let q = operand::qty_of(r)?;
                return Ok(vec![temporal::date_add(l, &q, false, at)?]);
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Minus => {
            if both(&["integer"]) {
                let diff = int_of(model, l, at)?.wrapping_sub(int_of(model, r, at)?);
                return Ok(int_value(diff));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                let (x, y, scale) = align(&a, &b, at)?;
                let diff = x.checked_sub(y).ok_or_else(|| too_wide(at))?;
                return Ok(dec_value(make_decimal(diff, scale, at)?));
            }
            if has_type(l, &["decimal", "integer", "Quantity"]) && has_type(r, &["Quantity"]) {
                // Only `0 - q` produces anything: `qty.copy().setValue(abs)`.
                if text_of(model, l).as_deref() != Some("0") {
                    return Ok(Vec::new());
                }
                let q = operand::qty_of(r)?;
                let value = q.value.as_deref().unwrap_or("0");
                let magnitude = value.trim_start_matches(['-', '+']);
                let abs = Decimal::parse(magnitude).ok_or_else(|| too_wide(at))?;
                return Ok(vec![q.to_value(abs)]);
            }
            if has_type(r, &["Quantity"]) && is_date_time_type(l) {
                let q = operand::qty_of(r)?;
                return Ok(vec![temporal::date_add(l, &q, true, at)?]);
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Times => {
            if both(&["integer"]) {
                let product = int_of(model, l, at)?.wrapping_mul(int_of(model, r, at)?);
                return Ok(int_value(product));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                let mantissa = a
                    .mantissa()
                    .checked_mul(b.mantissa())
                    .ok_or_else(|| too_wide(at))?;
                return Ok(dec_value(make_decimal(
                    mantissa,
                    a.scale() + b.scale(),
                    at,
                )?));
            }
            if is_quantity(model, l) && is_quantity(model, r) {
                return Err(EvalError::unsupported(
                    "multiplying quantities through the UCUM unit algebra",
                    at,
                ));
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::DivideBy => {
            let four = ["integer", "decimal", "unsignedInt", "positiveInt"];
            if both(&four) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                return match divide(&a, &b, at)? {
                    Quotient::Zero => Ok(dec_value(make_decimal(0, 0, at)?)),
                    Quotient::ByZero => Ok(Vec::new()),
                    Quotient::Exact(d) => Ok(dec_value(d)),
                    Quotient::Inexact => Err(EvalError::unsupported(
                        "a non-terminating quotient (the digit count belongs to \
                         org.fhir.ucum.Decimal.divide, which is not in reference/)",
                        at,
                    )),
                };
            }
            if is_quantity(model, l) && is_quantity(model, r) {
                return Err(EvalError::unsupported(
                    "dividing quantities through the UCUM unit algebra",
                    at,
                ));
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Div => {
            if both(&["integer"]) {
                let divisor = int_of(model, r, at)?;
                if divisor == 0 {
                    return Ok(Vec::new());
                }
                return Ok(int_value(int_of(model, l, at)?.wrapping_div(divisor)));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                if a.mantissa() == 0 {
                    return Ok(int_value(0));
                }
                return Ok(match div_int(&a, &b, at)? {
                    None => Vec::new(),
                    // `new IntegerType(String)` of the truncated quotient.
                    Some(q) => int_value(q as i32),
                });
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Mod => {
            if both(&["integer"]) {
                let modulus = int_of(model, r, at)?;
                if modulus == 0 {
                    return Ok(Vec::new());
                }
                return Ok(int_value(int_of(model, l, at)?.wrapping_rem(modulus)));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l, at)?,
                    operand::decimal_of(model, r, at)?,
                );
                if a.mantissa() == 0 {
                    return Ok(dec_value(a));
                }
                let (x, y, scale) = align(&a, &b, at)?;
                if y == 0 {
                    return Err(EvalError::execution(
                        format!(
                            "org.fhir.ucum.UcumException: Attempt to divide {} by zero",
                            a.lexical()
                        ),
                        at,
                    ));
                }
                let quotient = x / y;
                let rem = x - quotient.checked_mul(y).ok_or_else(|| too_wide(at))?;
                return Ok(dec_value(make_decimal(rem, scale, at)?));
            }
            Err(incompatible(op, l, r, at))
        }
    }
}

/// `opConcatenate`: an empty operand is the empty string, never an empty
/// result.
pub(super) fn op_concatenate(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    at: usize,
) -> Result<Vec<Value>, EvalError> {
    if left.len() > 1 {
        return Err(error(
            format!(
                "left operand to & can only have 1 value, but has {} values",
                left.len()
            ),
            at,
        ));
    }
    if let Some(l) = left.first() {
        if !has_type(l, &STRING_TYPES) {
            return Err(error(
                format!("left operand to & has the wrong type {}", fhir_type(l)),
                at,
            ));
        }
    }
    if right.len() > 1 {
        return Err(error(
            format!(
                "right operand to & can only have 1 value, but has {} values",
                right.len()
            ),
            at,
        ));
    }
    if let Some(r) = right.first() {
        if !has_type(r, &STRING_TYPES) {
            return Err(error(
                format!("right operand to & has the wrong type {}", fhir_type(r)),
                at,
            ));
        }
    }
    let text = |items: &[Value]| match items.first() {
        None => String::new(),
        Some(v) => text_of(model, v).unwrap_or_else(|| "null".to_string()),
    };
    Ok(vec![Value::String(format!(
        "{}{}",
        text(left),
        text(right)
    ))])
}
