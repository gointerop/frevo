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
//!   `Decimal` (`org.fhir.ucum.Decimal`, ported in `ucum_decimal.rs`), not
//!   `BigDecimal`: a quotient is rounded to the significant digits its operands
//!   carry, with that library's rounding-carry defect (`path.math.015`,
//!   `path.math.016`, `DEV-0030`); a zero **dividend** answers zero even when
//!   the divisor is zero, a non-zero dividend over a zero divisor is empty for
//!   `/` and `div` and an **error** for `mod`, and a zero remainder is a
//!   negative zero;
//! - `0 - q` for a quantity `q` is `q` with its value's absolute value, which
//!   is how a unary minus on a quantity literal is evaluated, and `q - q` is
//!   empty because the branch yields nothing unless the left operand is `0`;
//! - date and dateTime plus or minus a quantity is `dateAdd` (task 38,
//!   `temporal::date_add`); `*` and `/` of two quantities go through the UCUM
//!   service (`path.quantity.021`): a `UcumException` fails `*` at the
//!   operator and makes `/` empty.

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Value};

use super::EvalError;
use super::error::At;
use super::navigate::fhir_type;
use super::operand::{self, STRING_TYPES, has_type, int_of, is_primitive, is_quantity, text_of};
use super::quantity;
use super::temporal;
use super::ucum_algebra::UcumError;
use super::ucum_decimal::{Failure, UcumDecimal};

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

/// `makeException(holder, ...)`: the holder's start, when it has one, is the
/// position (`path.errors.002`).
fn error(message: String, at: At) -> EvalError {
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

/// `new Decimal(l.primitiveValue())` for both operands. A primitive with no
/// value is `null`, on which the constructor's `toLowerCase()` raises a
/// `NullPointerException` the engine does not catch, whose JVM message names
/// the constructor's parameter (`path.math.017`).
fn ucum_pair(
    model: &dyn Model,
    l: &Value,
    r: &Value,
) -> Result<Result<(UcumDecimal, UcumDecimal), Failure>, EvalError> {
    let read = |v: &Value| {
        text_of(model, v).ok_or_else(|| {
            EvalError::plain(
                "Cannot invoke \"String.toLowerCase()\" because \"value\" is null".to_string(),
            )
        })
    };
    let (lt, rt) = (read(l)?, read(r)?);
    Ok(UcumDecimal::new(&lt).and_then(|a| UcumDecimal::new(&rt).map(|b| (a, b))))
}

/// `Integer.parseInt(v.primitiveValue())` in the `integer` arms of `div` and
/// `mod` (`FHIRPathEngine.java:3051-3053`, `:3091-3093`). A primitive with no
/// value is `Integer.parseInt(null)`, whose `NumberFormatException` the engine
/// does not catch; its text is the JDK 21 and 25 one HAPI recorded, with no
/// position (`path.math.017`, `path.errors.002`).
fn parse_int(model: &dyn Model, v: &Value) -> Result<i32, EvalError> {
    if !matches!(v, Value::Integer(_)) && text_of(model, v).is_none() {
        return Err(EvalError::plain("Cannot parse null string"));
    }
    int_of(model, v)
}

/// A `java.lang.Error` out of `Decimal`, which no arm catches: its message,
/// with no position (`path.errors.002`).
fn java_error(message: &str) -> EvalError {
    EvalError::plain(message.to_string())
}

/// `new DecimalType(String)` of `Decimal.asDecimal()`.
fn ucum_value(d: &UcumDecimal, at: usize) -> Result<Vec<Value>, EvalError> {
    Ok(dec_value(
        Decimal::parse(&d.as_decimal()).ok_or_else(|| too_wide(at))?,
    ))
}

/// The preconditions every operator checks before it looks at types.
fn operands<'a>(
    model: &dyn Model,
    left: &'a [Value],
    right: &'a [Value],
    op: Arith,
    at: At,
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

fn incompatible(op: Arith, l: &Value, r: &Value, at: At) -> EvalError {
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
///
/// `at` is the holder's start, which `makeException` reports, and `op_at` its
/// operator's start, which `dateAdd`'s own constructions pass
/// (`holder.getOpStart()`, `FHIRPathEngine.java:2725`, `:2731`, `:2763`).
pub(super) fn op_arith(
    model: &dyn Model,
    left: &[Value],
    op: Arith,
    right: &[Value],
    at: At,
    op_at: At,
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
                let sum = int_of(model, l)?.wrapping_add(int_of(model, r)?);
                return Ok(int_value(sum));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l)?,
                    operand::decimal_of(model, r)?,
                );
                let (x, y, scale) = align(&a, &b, at.offset)?;
                let sum = x.checked_add(y).ok_or_else(|| too_wide(at.offset))?;
                return Ok(dec_value(make_decimal(sum, scale, at.offset)?));
            }
            if has_type(r, &["Quantity"]) && is_date_time_type(l) {
                let q = operand::qty_of(r)?;
                return Ok(vec![temporal::date_add(l, &q, false, op_at)?]);
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Minus => {
            if both(&["integer"]) {
                let diff = int_of(model, l)?.wrapping_sub(int_of(model, r)?);
                return Ok(int_value(diff));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l)?,
                    operand::decimal_of(model, r)?,
                );
                let (x, y, scale) = align(&a, &b, at.offset)?;
                let diff = x.checked_sub(y).ok_or_else(|| too_wide(at.offset))?;
                return Ok(dec_value(make_decimal(diff, scale, at.offset)?));
            }
            if has_type(l, &["decimal", "integer", "Quantity"]) && has_type(r, &["Quantity"]) {
                // Only `0 - q` produces anything: `qty.copy().setValue(abs)`.
                if text_of(model, l).as_deref() != Some("0") {
                    return Ok(Vec::new());
                }
                let q = operand::qty_of(r)?;
                let value = q.value.as_deref().unwrap_or("0");
                let magnitude = value.trim_start_matches(['-', '+']);
                let abs = Decimal::parse(magnitude).ok_or_else(|| too_wide(at.offset))?;
                return Ok(vec![q.to_value(abs)]);
            }
            if has_type(r, &["Quantity"]) && is_date_time_type(l) {
                let q = operand::qty_of(r)?;
                return Ok(vec![temporal::date_add(l, &q, true, op_at)?]);
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Times => {
            if both(&["integer"]) {
                let product = int_of(model, l)?.wrapping_mul(int_of(model, r)?);
                return Ok(int_value(product));
            }
            if both(&numeric) {
                let (a, b) = (
                    operand::decimal_of(model, l)?,
                    operand::decimal_of(model, r)?,
                );
                let mantissa = a
                    .mantissa()
                    .checked_mul(b.mantissa())
                    .ok_or_else(|| too_wide(at.offset))?;
                return Ok(dec_value(make_decimal(
                    mantissa,
                    a.scale() + b.scale(),
                    at.offset,
                )?));
            }
            if is_quantity(model, l) && is_quantity(model, r) {
                // `qtyToPair` of each side, then `UcumService.multiply`, which
                // dereferences a `null` pair (`o1` first). A `UcumException`
                // is rethrown at the operator (`:2800-2801`).
                let pl = quantity::to_pair(&operand::qty_of(l)?)?;
                let pr = quantity::to_pair(&operand::qty_of(r)?)?;
                let pl = pl.ok_or_else(|| quantity::null_pair("o1"))?;
                let pr = pr.ok_or_else(|| quantity::null_pair("o2"))?;
                let service = quantity::service()?;
                return match service.multiply(&pl.0, pl.1.as_deref(), &pr.0, pr.1.as_deref()) {
                    Ok(p) => Ok(vec![quantity::pair_to_value(p, at.offset)?]),
                    Err(UcumError::Ucum(m)) => Err(EvalError::execution(m, op_at)),
                    Err(e) => Err(quantity::uncaught(&e)),
                };
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::DivideBy => {
            let four = ["integer", "decimal", "unsignedInt", "positiveInt"];
            if both(&four) {
                // `catch (UcumException e)`: just return nothing.
                return match ucum_pair(model, l, r)?.and_then(|(a, b)| a.divide(&b)) {
                    Ok(q) => ucum_value(&q, at.offset),
                    Err(Failure::Ucum(_)) => Ok(Vec::new()),
                    Err(Failure::Error(m)) => Err(java_error(m)),
                };
            }
            if is_quantity(model, l) && is_quantity(model, r) {
                // `qtyToPair` of each side, then `UcumService.divideBy`, which
                // dereferences a `null` pair (`dividend` first). A
                // `UcumException` is swallowed (`:3020-3022`).
                let pl = quantity::to_pair(&operand::qty_of(l)?)?;
                let pr = quantity::to_pair(&operand::qty_of(r)?)?;
                let pl = pl.ok_or_else(|| quantity::null_pair("dividend"))?;
                let pr = pr.ok_or_else(|| quantity::null_pair("divisor"))?;
                let service = quantity::service()?;
                return match service.divide_by(&pl.0, pl.1.as_deref(), &pr.0, pr.1.as_deref()) {
                    Ok(p) => Ok(vec![quantity::pair_to_value(p, at.offset)?]),
                    Err(UcumError::Ucum(_)) => Ok(Vec::new()),
                    Err(e) => Err(quantity::uncaught(&e)),
                };
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Div => {
            if both(&["integer"]) {
                let divisor = parse_int(model, r)?;
                if divisor == 0 {
                    return Ok(Vec::new());
                }
                return Ok(int_value(parse_int(model, l)?.wrapping_div(divisor)));
            }
            if both(&numeric) {
                let q = match ucum_pair(model, l, r)?.and_then(|(a, b)| a.div_int(&b)) {
                    Ok(q) => q,
                    // `catch (UcumException e)`: just return nothing.
                    Err(Failure::Ucum(_)) => return Ok(Vec::new()),
                    Err(Failure::Error(m)) => return Err(java_error(m)),
                };
                // `new IntegerType(String)`: `Integer.parseInt`, whose
                // `NumberFormatException` is rethrown as
                // `IllegalArgumentException(e)` (`IntegerType.java:65-67`,
                // `:86-92`) with the cause's `toString()` as its message.
                let text = q.as_decimal();
                return match text.parse::<i32>() {
                    Ok(i) => Ok(int_value(i)),
                    Err(_) => Err(EvalError::plain(format!(
                        "java.lang.NumberFormatException: For input string: \"{text}\""
                    ))),
                };
            }
            Err(incompatible(op, l, r, at))
        }
        Arith::Mod => {
            if both(&["integer"]) {
                let modulus = parse_int(model, r)?;
                if modulus == 0 {
                    return Ok(Vec::new());
                }
                return Ok(int_value(parse_int(model, l)?.wrapping_rem(modulus)));
            }
            if both(&numeric) {
                return match ucum_pair(model, l, r)?.and_then(|(a, b)| a.modulo(&b)) {
                    Ok(m) => ucum_value(&m, at.offset),
                    // `new PathEngineException(e)` around the UcumException
                    // (`FHIRPathEngine.java:3102`): the cause's `toString()`,
                    // and no location, so no suffix and no offset
                    // (`path.errors.002`).
                    Err(Failure::Ucum(m)) => Err(EvalError::plain(format!(
                        "org.fhir.ucum.UcumException: {m}"
                    ))),
                    Err(Failure::Error(m)) => Err(java_error(m)),
                };
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
    at: At,
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
