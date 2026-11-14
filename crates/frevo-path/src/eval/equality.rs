//! The equality and emptiness primitives the collection functions share.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`doEquals` at `:2190`, `decEqual` at `:2180`, `removeTrailingZeros` at
//! `:2161`), org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Base.java
//! (`compareDeep` at `:281`, `isEmpty` at `:238`), both at
//! 73c432b5f43ddbce979978a760ec4aa81e929094, and
//! hapi-fhir-base/src/main/java/ca/uhn/fhir/util/ElementUtil.java
//! (`isEmpty(List)` at `:92`) at 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright HL7, Inc. and contributors, and Smile CDR, Inc.,
//! licensed under the Apache License, Version 2.0.
//! Modified by the Frevo authors: values are the `Value` enum rather than
//! `Base` objects, so "is this a `DecimalType`" is a question about the value's
//! FHIR type name; `equalsDeep` is one structural comparison over `Node`
//! instead of one generated method per class; and the two equality rules this
//! module does not implement yet -- quantities and dates, which are task 35's
//! -- are [`EvalError::Unsupported`] and never a guessed answer.
//!
//! `doEquals` answers `Boolean` upstream, where `null` means "cannot be
//! compared". Only the quantity and date rules can produce it, and they are not
//! here, so every answer this module gives is `Some`. The `Option` is kept so
//! that the callers already carry the `null` branch `opEquals`, `funcDistinct`
//! and `funcIsDistinct` have.

use crate::seam::model::Model;
use crate::seam::value::Value;

use super::EvalError;
use super::navigate::fhir_type;

/// `StringUtils.isBlank`.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// `Base.primitiveValue()` for a value that is a primitive:
/// `Some(Some(text))` for one with a value, `Some(None)` for a primitive
/// element that carries only an id or extensions, `None` for anything that is
/// not a primitive at all.
fn primitive_value(model: &dyn Model, value: &Value) -> Option<Option<String>> {
    match value {
        Value::Boolean(b) => Some(Some(b.to_string())),
        Value::Integer(i) => Some(Some(i.to_string())),
        Value::Decimal(d) => Some(Some(d.lexical().to_string())),
        Value::String(s) => Some(Some(s.clone())),
        Value::Date(d) => Some(Some(d.lexical.clone())),
        Value::DateTime(d) => Some(Some(d.lexical.clone())),
        Value::Time(t) => Some(Some(t.lexical.clone())),
        Value::Quantity(_) => None,
        Value::Node(node) => match node.primitive() {
            Some(p) => Some(Some(p.lexical)),
            None if model.is_primitive(node.type_name()) => Some(None),
            None => None,
        },
    }
}

/// `left instanceof DecimalType`, which for an element-model node is its FHIR
/// type.
fn is_decimal(value: &Value) -> bool {
    matches!(value, Value::Decimal(_)) || fhir_type(value) == "decimal"
}

/// `Base.isDateTime()`: a `BaseDateTimeType`, which is `date`, `dateTime` and
/// `instant` and is not `time`.
fn is_date_time(value: &Value) -> bool {
    matches!(value, Value::Date(_) | Value::DateTime(_))
        || matches!(fhir_type(value), "date" | "dateTime" | "instant")
}

/// `instanceof Quantity`, which `Age`, `Duration` and the other quantity
/// profiles satisfy.
fn is_quantity(model: &dyn Model, value: &Value) -> bool {
    let t = fhir_type(value);
    matches!(value, Value::Quantity(_)) || t == "Quantity" || model.derives_from(t, "Quantity")
}

/// `removeTrailingZeros` (`FHIRPathEngine.java:2152`), as written: it strips
/// trailing `0`s from a number that has **no** decimal point too, so `100`
/// becomes `1`. That is the pinned engine's behaviour and it is kept.
fn remove_trailing_zeros(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = chars.len() - 1;
    let mut done = false;
    let mut dot = false;
    while i > 0 && !done {
        if chars[i] == '.' {
            i -= 1;
            dot = true;
        } else if !dot && chars[i] == '0' {
            i -= 1;
        } else {
            done = true;
        }
    }
    chars[..=i].iter().collect()
}

/// `decEqual` (`FHIRPathEngine.java:2178`).
fn dec_equal(left: &str, right: &str) -> bool {
    remove_trailing_zeros(left) == remove_trailing_zeros(right)
}

/// `doEquals(left, right)` (`FHIRPathEngine.java:2190`).
///
/// `offset` is where the failure is attributed if a rule this engine does not
/// implement yet is needed.
pub(super) fn do_equals(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    offset: usize,
) -> Result<Option<bool>, EvalError> {
    if is_quantity(model, left) && is_quantity(model, right) {
        return Err(EvalError::unsupported("quantity equality", offset));
    }
    if is_date_time(left) && is_date_time(right) {
        return Err(EvalError::unsupported(
            "date and dateTime equality (FHIRPath precision rules)",
            offset,
        ));
    }
    let left_primitive = primitive_value(model, left);
    let right_primitive = primitive_value(model, right);
    if is_decimal(left) || is_decimal(right) {
        let text = |p: &Option<Option<String>>| p.clone().flatten().unwrap_or_default();
        return Ok(Some(dec_equal(
            &text(&left_primitive),
            &text(&right_primitive),
        )));
    }
    if let (Some(l), Some(r)) = (&left_primitive, &right_primitive) {
        return Ok(Some(l == r));
    }
    compare_deep(model, left, right, offset).map(Some)
}

/// `opEquals` (`FHIRPathEngine.java:2103`) and `opNotEquals` (`:2130`):
/// either side empty is empty, a size mismatch is a plain answer, and a pair
/// `doEquals` cannot compare makes the whole answer empty unless another pair
/// already decided it.
///
/// Only `=` and `!=` are here, because `where(given = 'Jim')` is the criteria
/// most of the collection scenarios are written with; the rest of task 35
/// (`~`, `!~` and the ordered operators) is not.
pub(super) fn op_equals(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    negate: bool,
    offset: usize,
) -> Result<Vec<Value>, EvalError> {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    if left.len() != right.len() {
        return Ok(vec![Value::Boolean(negate)]);
    }
    let mut res = true;
    let mut nil = false;
    for (l, r) in left.iter().zip(right) {
        match do_equals(model, l, r, offset)? {
            None => nil = true,
            Some(eq) if eq == negate => {
                res = false;
                break;
            }
            Some(_) => {}
        }
    }
    Ok(if !res {
        vec![Value::Boolean(false)]
    } else if nil {
        Vec::new()
    } else {
        vec![Value::Boolean(true)]
    })
}

/// `doContains(list, item)` (`FHIRPathEngine.java:2846`): an item is contained
/// when `doEquals` answers `true`; a `null` answer is not a match.
pub(super) fn do_contains(
    model: &dyn Model,
    list: &[Value],
    item: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    for test in list {
        if do_equals(model, test, item, offset)? == Some(true) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `Base.primitiveValue()`: the text of a primitive that has a value, and
/// `None` for anything else.
pub(super) fn primitive_text(model: &dyn Model, value: &Value) -> Option<String> {
    primitive_value(model, value).flatten()
}

/// The Java class of the value a primitive holds, which is what
/// `PrimitiveType.equalsDeep` compares: an `integer` is never equal to the
/// `string` `"1"`, and a `code` is a `string` because `CodeType` extends
/// `StringType`.
fn value_class(value: &Value) -> u8 {
    match fhir_type(value) {
        "integer" | "unsignedInt" | "positiveInt" => 0,
        "decimal" => 1,
        "boolean" => 2,
        "date" | "dateTime" | "instant" => 3,
        "time" => 4,
        _ => 5,
    }
}

/// `Base.compareDeep(left, right, false)` for two values (`Base.java:281`),
/// which is `left.equalsDeep(right)`.
pub(super) fn compare_deep(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    if let (Value::Quantity(l), Value::Quantity(r)) = (left, right) {
        return Ok(l == r);
    }
    if matches!(left, Value::Quantity(_)) || matches!(right, Value::Quantity(_)) {
        // A literal quantity against a quantity element: the field-by-field
        // comparison needs the element's children, which task 35 owns.
        if is_quantity(model, left) && is_quantity(model, right) {
            return Err(EvalError::unsupported("quantity equality", offset));
        }
        return Ok(false);
    }
    let left_primitive = primitive_value(model, left);
    let right_primitive = primitive_value(model, right);
    match (left_primitive, right_primitive) {
        (Some(l), Some(r)) => {
            if value_class(left) != value_class(right) || l != r {
                return Ok(false);
            }
            // `Element.equalsDeep`: the `id` and the extensions too.
            let (Value::Node(ln), Value::Node(rn)) = (left, right) else {
                return Ok(true);
            };
            let (lp, rp) = (ln.primitive(), rn.primitive());
            let l_id = lp.as_ref().and_then(|p| p.id.as_deref());
            let r_id = rp.as_ref().and_then(|p| p.id.as_deref());
            if l_id != r_id {
                return Ok(false);
            }
            let l_ext = lp.map(|p| p.extensions).unwrap_or_default();
            let r_ext = rp.map(|p| p.extensions).unwrap_or_default();
            lists_deep_equal(model, &l_ext, &r_ext, offset)
        }
        // A primitive is never deeply equal to a complex value.
        (Some(_), None) | (None, Some(_)) => Ok(false),
        (None, None) => {
            let (Value::Node(ln), Value::Node(rn)) = (left, right) else {
                return Ok(false);
            };
            // `other instanceof HumanName`: the same class.
            if ln.type_name() != rn.type_name() {
                return Ok(false);
            }
            let lc = ln.all_children()?;
            let rc = rn.all_children()?;
            let mut names: Vec<&str> = Vec::new();
            for (name, _) in lc.iter().chain(rc.iter()) {
                if !names.contains(&name.as_str()) {
                    names.push(name);
                }
            }
            for name in names {
                let l: Vec<Value> = lc
                    .iter()
                    .filter(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .collect();
                let r: Vec<Value> = rc
                    .iter()
                    .filter(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .collect();
                if !lists_deep_equal(model, &l, &r, offset)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

/// `Base.compareDeep(List, List, true)` (`Base.java:263`): two empty lists are
/// equal, an empty and a non-empty one are not, and otherwise the lists match
/// item for item.
fn lists_deep_equal(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    offset: usize,
) -> Result<bool, EvalError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (l, r) in left.iter().zip(right) {
        if !compare_deep(model, l, r, offset)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `Base.isEmpty()` (`Base.java:238` and its overrides): a primitive is empty
/// when it has no non-blank value, no `id` and no extensions; a complex value
/// is empty when every child is.
pub(super) fn is_empty(value: &Value) -> Result<bool, EvalError> {
    match value {
        Value::String(s) => Ok(is_blank(s)),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Decimal(_)
        | Value::Date(_)
        | Value::DateTime(_)
        | Value::Time(_)
        | Value::Quantity(_) => Ok(false),
        Value::Node(node) => {
            if let Some(p) = node.primitive() {
                if !is_blank(&p.lexical) || p.id.is_some() || !p.extensions.is_empty() {
                    return Ok(false);
                }
            }
            for (_, child) in node.all_children()? {
                if !is_empty(&child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}
