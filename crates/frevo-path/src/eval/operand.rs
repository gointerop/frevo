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
use crate::seam::value::Value;

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

/// The fields of a quantity that `FHIRPathEngine` reads.
#[derive(Debug, Clone)]
pub(super) struct Qty {
    pub(super) value: Option<String>,
    pub(super) unit: Option<String>,
    pub(super) code: Option<String>,
    pub(super) system: Option<String>,
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
