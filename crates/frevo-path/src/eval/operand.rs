//! What the binary operators ask of an operand: its FHIR type, whether it is a
//! primitive, its text, and the fields of a quantity.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Base.java
//! (`hasType` at `:159`, `isPrimitive` at `:133`, `primitiveValue` at `:145`)
//! and org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`FHIR_TYPES_STRING` at `:2373`), at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: operands are [`Value`]s, so "is this a
//! `Quantity`" reads either the literal variant or an element's children.

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Quantity, Value};

use super::EvalError;
use super::equality::primitive_value_of;
use super::navigate::fhir_type;

/// `FHIR_TYPES_STRING`.
pub(super) const STRING_TYPES: [&str; 12] = [
    "string",
    "uri",
    "code",
    "oid",
    "id",
    "uuid",
    "sid",
    "markdown",
    "base64Binary",
    "canonical",
    "url",
    "xhtml",
];

pub(super) const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

/// `Base.hasType(names)`: a case-insensitive match on the FHIR type name.
pub(super) fn has_type(value: &Value, names: &[&str]) -> bool {
    let t = fhir_type(value);
    names.iter().any(|n| n.eq_ignore_ascii_case(t))
}

/// `Base.isPrimitive()`.
pub(super) fn is_primitive(model: &dyn Model, value: &Value) -> bool {
    primitive_value_of(model, value).is_some()
}

/// `Base.primitiveValue()`: `None` for a value that is not a primitive or has
/// no value.
pub(super) fn text_of(model: &dyn Model, value: &Value) -> Option<String> {
    primitive_value_of(model, value).flatten()
}

/// A decimal from a primitive that holds a number, or the error
/// `new BigDecimal(text)` raises.
pub(super) fn decimal_of(
    model: &dyn Model,
    value: &Value,
    offset: usize,
) -> Result<Decimal, EvalError> {
    if let Value::Decimal(d) = value {
        return Ok(d.clone());
    }
    let text = text_of(model, value).unwrap_or_default();
    Decimal::parse(&text).ok_or_else(|| {
        EvalError::execution(
            format!("Character array is missing \"exponent\" mark 'e' or 'E': {text}"),
            offset,
        )
    })
}

/// A 32-bit integer as `Integer.parseInt(primitiveValue())` reads it.
pub(super) fn int_of(model: &dyn Model, value: &Value, offset: usize) -> Result<i32, EvalError> {
    let text = match value {
        Value::Integer(i) => {
            return i32::try_from(*i).map_err(|_| number_format(&i.to_string(), offset));
        }
        other => text_of(model, other).unwrap_or_default(),
    };
    text.parse::<i32>()
        .map_err(|_| number_format(&text, offset))
}

fn number_format(text: &str, offset: usize) -> EvalError {
    EvalError::execution(format!("For input string: \"{text}\""), offset)
}

/// The fields of a quantity that `FHIRPathEngine` reads.
#[derive(Debug, Clone)]
pub(super) struct Qty {
    pub(super) value: Option<String>,
    pub(super) unit: Option<String>,
    pub(super) code: Option<String>,
    pub(super) system: Option<String>,
}

impl Qty {
    /// `Quantity.copy().setValue(...)` and the other places a quantity result
    /// is built from an operand.
    pub(super) fn to_value(&self, value: Decimal) -> Value {
        Value::Quantity(Quantity {
            value,
            unit: self.unit.clone(),
            system: self.system.clone(),
            code: self.code.clone(),
        })
    }
}

/// `instanceof Quantity`, which an `Age` or a `Duration` satisfies.
pub(super) fn is_quantity(model: &dyn Model, value: &Value) -> bool {
    let t = fhir_type(value);
    matches!(value, Value::Quantity(_)) || t == "Quantity" || model.derives_from(t, "Quantity")
}

/// The fields of a quantity value, whether a literal or an element.
pub(super) fn qty_of(value: &Value) -> Result<Qty, EvalError> {
    match value {
        Value::Quantity(q) => Ok(Qty {
            value: Some(q.value.lexical().to_string()),
            unit: q.unit.clone(),
            code: q.code.clone(),
            system: q.system.clone(),
        }),
        Value::Node(node) => {
            let field = |name: &str| -> Result<Option<String>, EvalError> {
                Ok(node
                    .children(name)?
                    .first()
                    .and_then(|v| v.as_node().and_then(|n| n.primitive()))
                    .map(|p| p.lexical))
            };
            Ok(Qty {
                value: field("value")?,
                unit: field("unit")?,
                code: field("code")?,
                system: field("system")?,
            })
        }
        _ => Ok(Qty {
            value: None,
            unit: None,
            code: None,
            system: None,
        }),
    }
}
