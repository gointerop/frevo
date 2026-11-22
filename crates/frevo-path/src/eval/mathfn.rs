//! The math functions: `abs`, `ceiling`, `floor`, `round`, `truncate`,
//! `sqrt`, `exp`, `ln`, `log` and `power`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `funcSqrt` at `:4095`,
//! `funcAbs` at `:4115`, `funcCeiling` at `:4138`, `funcFloor` at `:4156`,
//! `funcExp` at `:4176`, `funcLn` at `:4200`, `funcLog` at `:4220`,
//! `funcPower` at `:4248`, `funcTruncate` at `:4272` and `funcRound` at
//! `:4396`; and `DecimalType(double)` from
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/DecimalType.java.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the binary `double` is Rust's `f64`; the
//! `BigDecimal` of `BigDecimal.valueOf(double)` is a [`Decimal`] written as
//! `BigDecimal.toString` writes it, including the `E+n` form; a negative scale
//! survives in that text only, the numeric fields holding the value at scale 0.
//!
//! What the pinned engine does here and the specification does not say, and
//! what is kept:
//!
//! - every function but `exp` raises on an **empty** focus as well as on a
//!   many-item one (`focus.size() != 1`); only `exp` answers empty;
//! - a focus that is not numeric answers **empty**: the `makeException(...)`
//!   in the `else` arm is built and never thrown;
//! - `abs`, `sqrt`, `exp`, `ln`, `log` and `power` answer a decimal computed
//!   through a binary `double`; a `NaN` or an infinity cannot be a
//!   `BigDecimal` and the swallowed exception makes the answer empty;
//! - `ceiling` and `floor` answer a 32-bit integer cast from the `double`
//!   (saturating); `truncate` cuts the text at the point and parses it as an
//!   integer; `round` is exact `BigDecimal.setScale(i, HALF_UP)`.

use crate::seam::unscaled::Unscaled;
use crate::seam::value::{Decimal, Value};
use crate::syntax::ast::{Function, NodeId};

use super::arith::too_wide;
use super::error::At;
use super::operand::{self, has_type, int_of, qty_of, text_of};
use super::{Context, EvalError, Run};

const NUMERIC: [&str; 4] = ["integer", "decimal", "unsignedInt", "positiveInt"];

/// `makeExceptionPlural(focus.size(), expr, FHIRPATH_FOCUS, ...)`, held by the
/// function node (`path.errors.002`).
fn focus_error(name: &str, count: usize, at: At) -> EvalError {
    EvalError::execution(
        format!(
            "Error evaluating FHIRPath expression: focus for {name} can only have one value, \
             but has {count} values"
        ),
        at,
    )
}

/// `makeException(expr, FHIRPATH_WRONG_PARAM_TYPE, ...)`, held by the function
/// node (`path.errors.002`).
fn wrong_param(name: &str, index: &str, got: &str, expecting: &str, at: At) -> EvalError {
    EvalError::execution(
        format!(
            "Error evaluating FHIRPath expression: The parameter type {got} is not legal for \
             {name} parameter {index}. expecting {expecting}"
        ),
        at,
    )
}

/// `Double.parseDouble(text)`.
/// A `NumberFormatException`, which carries no position (`path.errors.002`).
fn parse_double(text: &str) -> Result<f64, EvalError> {
    text.trim().parse::<f64>().map_err(|_| {
        EvalError::plain(if text.is_empty() {
            "empty String".to_string()
        } else {
            format!("For input string: \"{text}\"")
        })
    })
}

/// `new DecimalType(double)`, i.e. `BigDecimal.valueOf(d)`: the digits of
/// `Double.toString(d)` with the scale that string gives, written in plain
/// notation. `None` for a `NaN` or an infinity, which `BigDecimal` rejects and
/// the engine swallows.
fn decimal_of_double(d: f64, at: usize) -> Result<Option<Value>, EvalError> {
    if !d.is_finite() {
        return Ok(None);
    }
    // `new BigDecimal("-0.0")` has no negative zero.
    let d = if d == 0.0 { 0.0 } else { d };
    let magnitude = d.abs();
    // Shortest round-trip digits and the decimal exponent of the first digit.
    let scientific = format!("{magnitude:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if magnitude == 0.0 {
        digits = "0".to_string();
    }
    let n = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    // `Double.toString` is plain between 1e-3 and 1e7 and `d.dddE+n` outside.
    let plain_form = magnitude == 0.0 || (1e-3..1e7).contains(&magnitude);
    let (unscaled, scale) = if plain_form {
        if exponent >= 0 {
            let fraction = (n - 1 - exponent).max(1);
            let width = exponent + 1 + fraction;
            let pad = usize::try_from(width - n).unwrap_or(0);
            (format!("{digits}{}", "0".repeat(pad)), fraction)
        } else {
            (digits, n - 1 - exponent)
        }
    } else {
        if n == 1 {
            digits.push('0');
        }
        let n = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        (digits, n - 1 - exponent)
    };
    let signed = Unscaled::from_digits(d < 0.0, unscaled.as_bytes()).ok_or_else(|| too_wide(at))?;
    Decimal::from_unscaled(signed, scale)
        .map(|v| Some(Value::Decimal(v)))
        .ok_or_else(|| too_wide(at))
}

/// `BigDecimal.setScale(scale, RoundingMode.HALF_UP)`.
fn set_scale_half_up(d: &Decimal, scale: i32, at: usize) -> Result<Decimal, EvalError> {
    let current = i64::from(d.scale());
    let target = i64::from(scale);
    let mantissa = d.mantissa();
    if target >= current {
        let scaled = usize::try_from(target - current)
            .ok()
            .and_then(|shift| mantissa.shifted_left(shift))
            .ok_or_else(|| too_wide(at))?;
        return Decimal::from_unscaled(scaled, target).ok_or_else(|| too_wide(at));
    }
    let shift = usize::try_from(current - target).map_err(|_| too_wide(at))?;
    let rounded = mantissa.shifted_right_half_up(shift);
    // A negative scale renders in scientific form, as `BigDecimal` does.
    Decimal::from_unscaled(rounded, target).ok_or_else(|| too_wide(at))
}

impl Run<'_> {
    /// One math function over its focus.
    pub(super) fn func_math(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let at = node.start();
        let name = function.to_code();
        // `funcExp` alone answers an empty focus with an empty result.
        if focus.is_empty() && function == Function::Exp {
            return Ok(Vec::new());
        }
        let [base] = focus else {
            return Err(focus_error(name, focus.len(), At::node(node)));
        };
        // `abs` also takes a quantity; every other function needs a number,
        // and a non-number is an exception built and never thrown: empty.
        if function == Function::Abs && has_type(base, &["Quantity"]) {
            let q = qty_of(base)?;
            let value = q.value.as_deref().unwrap_or("0");
            let abs =
                Decimal::parse(value.trim_start_matches(['-', '+'])).ok_or_else(|| too_wide(at))?;
            return Ok(vec![q.to_value(abs)]);
        }
        if !has_type(base, &NUMERIC) {
            return Ok(Vec::new());
        }
        let text = text_of(self.model, base).unwrap_or_default();
        match function {
            Function::Truncate => {
                let integral = text.split('.').next().unwrap_or("");
                return integral
                    .parse::<i32>()
                    .map(|i| vec![Value::Integer(i64::from(i))])
                    .map_err(|_| EvalError::plain(format!("For input string: \"{integral}\"")));
            }
            Function::Round => return self.func_round(context, focus, id, base),
            _ => {}
        }
        let d = parse_double(&text)?;
        let result = match function {
            Function::Abs => decimal_of_double(d.abs(), at)?,
            Function::Ceiling => return Ok(vec![Value::Integer(i64::from(d.ceil() as i32))]),
            Function::Floor => return Ok(vec![Value::Integer(i64::from(d.floor() as i32))]),
            Function::Sqrt => decimal_of_double(d.sqrt(), at)?,
            Function::Exp => decimal_of_double(d.exp(), at)?,
            Function::Ln => decimal_of_double(d.ln(), at)?,
            Function::Log | Function::Power => {
                let argument = self.number_argument(context, focus, id, name)?;
                let e = parse_double(&argument)?;
                let computed = if function == Function::Log {
                    // `customLog(base, number)`: `Math.log(number) / Math.log(base)`.
                    d.ln() / e.ln()
                } else {
                    d.powf(e)
                };
                decimal_of_double(computed, at)?
            }
            other => {
                return Err(EvalError::unsupported(
                    format!("the function {}()", other.to_code()),
                    at,
                ));
            }
        };
        Ok(result.into_iter().collect())
    }

    /// `execute(context, focus, parameters.get(0), true)` that must be one
    /// item, as `log` and `power` read it; its `primitiveValue()`.
    fn number_argument(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        name: &str,
    ) -> Result<String, EvalError> {
        let node = self.expr.node(id);
        let Some(&param) = node.parameters().first() else {
            return Err(EvalError::plain(format!("{name}() requires a parameter")));
        };
        let values = self.execute(context, focus, param, true)?;
        match values.as_slice() {
            [one] => Ok(text_of(self.model, one).unwrap_or_default()),
            _ => Err(wrong_param(
                name,
                "0",
                "Multiple Values",
                "integer or decimal",
                At::node(node),
            )),
        }
    }

    /// `funcRound`: exact, HALF_UP, at the scale given (0 when none).
    fn func_round(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        base: &Value,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let at = node.start();
        let mut places = 0;
        if let [param] = node.parameters() {
            let values = self.execute(context, focus, *param, true)?;
            let [one] = values.as_slice() else {
                // Upstream's message names `power` here.
                return Err(wrong_param(
                    "power",
                    "0",
                    "Multiple Values",
                    "integer",
                    At::node(node),
                ));
            };
            places = int_of(self.model, one)?;
        }
        let decimal = operand::decimal_of(self.model, base)?;
        Ok(vec![Value::Decimal(set_scale_half_up(
            &decimal, places, at,
        )?)])
    }
}
