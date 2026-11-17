//! The four ordered comparison operators.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `opLessThan` at `:2409`,
//! `opGreater` at `:2456`, `opLessOrEqual` at `:2500`, `opGreaterOrEqual` at
//! `:2548`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: one function takes the operator as a value
//! instead of four near-copies; the differences between the four -- which are
//! the pinned engine's, and are kept -- are spelled out where they occur.
//!
//! What the pinned engine does here and the specification does not say:
//!
//! - a multi-item operand, or two operands that are neither primitives nor
//!   quantities, gives **empty** rather than an error (`DEV-0002`);
//! - `<` accepts only `integer` and `decimal` where the other three also accept
//!   `unsignedInt` and `positiveInt` (`DEV-0003`);
//! - every numeric comparison is between two binary `double`s (`DEV-0004`);
//! - strings compare by UTF-16 code unit, as `String.compareTo` does;
//! - `<` compares quantities' `code`, `<=`, `>` and `>=` their `unit`, so a
//!   quantity literal such as `1 'cm'`, which has a code and no unit, compares
//!   as if it had no unit at all;
//! - two quantities whose units differ go through UCUM canonical form without a
//!   dimension check (`DEV-0005`), which needs task 43 and is
//!   [`EvalError::Unsupported`] until then.

use std::cmp::Ordering;

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Value};

use super::EvalError;
use super::navigate::fhir_type;
use super::operand::{self, STRING_TYPES, has_type, is_primitive, text_of};
use super::temporal;

/// Which of the four operators.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Ordered {
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
}

impl Ordered {
    fn holds(self, o: Ordering) -> bool {
        match self {
            Ordered::Less => o == Ordering::Less,
            Ordered::Greater => o == Ordering::Greater,
            Ordered::LessOrEqual => o != Ordering::Greater,
            Ordered::GreaterOrEqual => o != Ordering::Less,
        }
    }
}

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Boolean(b)]
}

fn null_pointer(what: &str, offset: usize) -> EvalError {
    EvalError::execution(
        format!("Cannot invoke \"{what}\" because the value is null"),
        offset,
    )
}

/// `Double.valueOf(primitiveValue())`.
fn double_of(model: &dyn Model, value: &Value, offset: usize) -> Result<f64, EvalError> {
    let text =
        text_of(model, value).ok_or_else(|| null_pointer("java.lang.String.trim()", offset))?;
    text.trim()
        .parse::<f64>()
        .map_err(|_| EvalError::execution(format!("For input string: \"{text}\""), offset))
}

/// The decimal list `listChildrenByName("value")` yields for a quantity: one
/// item, or none.
fn value_list(q: &operand::Qty, offset: usize) -> Result<Vec<Value>, EvalError> {
    match &q.value {
        None => Ok(Vec::new()),
        Some(text) => Decimal::parse(text)
            .map(|d| vec![Value::Decimal(d)])
            .ok_or_else(|| {
                EvalError::execution(format!("\"{text}\" is not a decimal value"), offset)
            }),
    }
}

/// `opLessThan`, `opGreater`, `opLessOrEqual` and `opGreaterOrEqual`.
///
/// `at` is where an error is attributed (`holder.getStart()`).
pub(super) fn op_ordered(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    kind: Ordered,
    at: usize,
) -> Result<Vec<Value>, EvalError> {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    let single = left.len() == 1 && right.len() == 1;
    if single && is_primitive(model, &left[0]) && is_primitive(model, &right[0]) {
        return primitives(model, &left[0], &right[0], kind, at);
    }
    if single && fhir_type(&left[0]) == "Quantity" && fhir_type(&right[0]) == "Quantity" {
        return quantities(model, &left[0], &right[0], kind, at);
    }
    Ok(Vec::new())
}

fn primitives(
    model: &dyn Model,
    l: &Value,
    r: &Value,
    kind: Ordered,
    at: usize,
) -> Result<Vec<Value>, EvalError> {
    if has_type(l, &STRING_TYPES) && has_type(r, &STRING_TYPES) {
        let lt = text_of(model, l).ok_or_else(|| null_pointer("java.lang.String.compareTo", at))?;
        let rt = text_of(model, r).ok_or_else(|| null_pointer("java.lang.String.compareTo", at))?;
        let order = lt.encode_utf16().cmp(rt.encode_utf16());
        return Ok(boolean(kind.holds(order)));
    }
    let numeric: &[&str] = if kind == Ordered::Less {
        &["integer", "decimal"]
    } else {
        &["integer", "decimal", "unsignedInt", "positiveInt"]
    };
    if has_type(l, numeric) && has_type(r, numeric) {
        let (a, b) = (double_of(model, l, at)?, double_of(model, r, at)?);
        let holds = match kind {
            Ordered::Less => a < b,
            Ordered::Greater => a > b,
            Ordered::LessOrEqual => a <= b,
            Ordered::GreaterOrEqual => a >= b,
        };
        return Ok(boolean(holds));
    }
    let dates = ["date", "dateTime", "instant"];
    if has_type(l, &dates) && has_type(r, &dates) {
        let (Some(a), Some(b)) = (temporal::from_value(l, at)?, temporal::from_value(r, at)?)
        else {
            return Ok(Vec::new());
        };
        return Ok(match temporal::compare_ordered(&a, &b) {
            None => Vec::new(),
            Some(o) => boolean(kind.holds(o)),
        });
    }
    if has_type(l, &["time"]) && has_type(r, &["time"]) {
        let lt = text_of(model, l).unwrap_or_default();
        let rt = text_of(model, r).unwrap_or_default();
        return Ok(match temporal::compare_time(&lt, &rt) {
            None => Vec::new(),
            Some(o) => boolean(kind.holds(o)),
        });
    }
    Err(EvalError::execution(
        format!(
            "Error evaluating FHIRPath expression: Unable to compare values of type {} and {}",
            fhir_type(l),
            fhir_type(r)
        ),
        at,
    ))
}

fn quantities(
    model: &dyn Model,
    l: &Value,
    r: &Value,
    kind: Ordered,
    at: usize,
) -> Result<Vec<Value>, EvalError> {
    let (lq, rq) = (operand::qty_of(l)?, operand::qty_of(r)?);
    let same_units = match kind {
        // `Base.compareDeep(lCode, rCode, true)`.
        Ordered::Less => lq.code == rq.code,
        // `Base.compareDeep(lUnit, rUnit, true)`.
        Ordered::Greater | Ordered::GreaterOrEqual => lq.unit == rq.unit,
        // `(lunit == null && runit == null) || lunit.equals(runit)`, which
        // dereferences a null `lunit` when only `runit` is present.
        Ordered::LessOrEqual => match (&lq.unit, &rq.unit) {
            (None, None) => true,
            (None, Some(_)) => {
                return Err(null_pointer("java.lang.String.equals(Object)", at));
            }
            (Some(a), Some(b)) => a == b,
            (Some(_), None) => false,
        },
    };
    if same_units {
        let (lv, rv) = (value_list(&lq, at)?, value_list(&rq, at)?);
        return op_ordered(model, &lv, &rv, kind, at);
    }
    // `qtyToCanonicalDecimal` answers `null` for a quantity that is not in the
    // UCUM system, and the recursive call then dereferences it.
    let ucum = Some(operand::UCUM_SYSTEM);
    if lq.system.as_deref() != ucum || rq.system.as_deref() != ucum {
        return Err(null_pointer("org.hl7.fhir.r4.model.Base.isPrimitive()", at));
    }
    Err(EvalError::unsupported(
        "ordering quantities through their UCUM canonical form",
        at,
    ))
}
