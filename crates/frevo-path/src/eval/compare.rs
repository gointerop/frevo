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
//! - comparing a `dateTime` element finer than a day rewrites its stored
//!   string to the Zulu form for the rest of the evaluation
//!   (`compareDateTimeElements`, `:635-648`; [`super::rewrite`],
//!   `path.compare.013`);
//! - two quantities whose units differ go through UCUM canonical form without a
//!   dimension check (`DEV-0005`).

use std::cmp::Ordering;

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Value};

use super::EvalError;
use super::error::At;
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

/// A `NullPointerException` upstream, which carries no position
/// (`path.errors.002`).
fn null_pointer(what: &str) -> EvalError {
    EvalError::plain(format!(
        "Cannot invoke \"{what}\" because the value is null"
    ))
}

/// `Double.valueOf(primitiveValue())`.
fn double_of(model: &dyn Model, value: &Value) -> Result<f64, EvalError> {
    let text = text_of(model, value).ok_or_else(|| null_pointer("java.lang.String.trim()"))?;
    text.trim()
        .parse::<f64>()
        .map_err(|_| EvalError::plain(format!("For input string: \"{text}\"")))
}

/// The decimal list `listChildrenByName("value")` yields for a quantity: one
/// item, or none.
fn value_list(q: &operand::Qty) -> Result<Vec<Value>, EvalError> {
    match &q.value {
        None => Ok(Vec::new()),
        Some(text) => Decimal::parse(text)
            .map(|d| vec![Value::Decimal(d)])
            .ok_or_else(|| EvalError::plain(format!("\"{text}\" is not a decimal value"))),
    }
}

/// `opLessThan`, `opGreater`, `opLessOrEqual` and `opGreaterOrEqual`.
///
/// `at` is where an error is attributed (`holder.getStart()`, which a
/// precedence group does not have, `path.errors.002`).
pub(super) fn op_ordered(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    kind: Ordered,
    at: At,
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
    at: At,
) -> Result<Vec<Value>, EvalError> {
    if has_type(l, &STRING_TYPES) && has_type(r, &STRING_TYPES) {
        let lt = text_of(model, l).ok_or_else(|| null_pointer("java.lang.String.compareTo"))?;
        let rt = text_of(model, r).ok_or_else(|| null_pointer("java.lang.String.compareTo"))?;
        let order = lt.encode_utf16().cmp(rt.encode_utf16());
        return Ok(boolean(kind.holds(order)));
    }
    let numeric: &[&str] = if kind == Ordered::Less {
        &["integer", "decimal"]
    } else {
        &["integer", "decimal", "unsignedInt", "positiveInt"]
    };
    if has_type(l, numeric) && has_type(r, numeric) {
        let (a, b) = (double_of(model, l)?, double_of(model, r)?);
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
        let (Some(a), Some(b)) = (temporal::from_value(l)?, temporal::from_value(r)?) else {
            return Ok(Vec::new());
        };
        // `compareDateTimeElements` rewrites an operand that is already a
        // `DateTimeType` to Zulu in place (`path.compare.013`).
        super::rewrite::rewrite_to_zulu(l);
        super::rewrite::rewrite_to_zulu(r);
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
    at: At,
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
                return Err(null_pointer("java.lang.String.equals(Object)"));
            }
            (Some(a), Some(b)) => a == b,
            (Some(_), None) => false,
        },
    };
    if same_units {
        let (lv, rv) = (value_list(&lq)?, value_list(&rq)?);
        return op_ordered(model, &lv, &rv, kind, at);
    }
    // `qtyToCanonicalDecimal` of each side, left first; it answers `null` for
    // a quantity outside the UCUM system or a code the service refuses, and
    // the recursive call then dereferences it.
    let dl = super::quantity::canonical_decimal(&lq)?;
    let dr = super::quantity::canonical_decimal(&rq)?;
    let (Some(a), Some(b)) = (dl, dr) else {
        // `left.get(0).isPrimitive()` (or `right.get(0)`) on the one-item
        // list holding that `null` (`FHIRPathEngine.java:2413`), worded as
        // the JVM words it.
        return Err(EvalError::plain(
            "Cannot invoke \"org.hl7.fhir.r4.model.Base.isPrimitive()\" because the return \
             value of \"java.util.List.get(int)\" is null",
        ));
    };
    // Two `DecimalType`s: `Double.valueOf` of each, with no check that the
    // canonical units agree (`DEV-0005`, `path.quantity.022`).
    let double = |t: &str| {
        t.trim()
            .parse::<f64>()
            .map_err(|_| EvalError::plain(format!("For input string: \"{t}\"")))
    };
    let (x, y) = (double(&a)?, double(&b)?);
    Ok(boolean(match kind {
        Ordered::Less => x < y,
        Ordered::Greater => x > y,
        Ordered::LessOrEqual => x <= y,
        Ordered::GreaterOrEqual => x >= y,
    }))
}
