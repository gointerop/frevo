//! Type operators and reflection: `is`, `as`, `type()` and the evaluation
//! side of the type tests the function forms share with `ofType`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `executeTypeName` at
//! `:1535`, `opAs` at `:1875`, `compareTypeNames` at `:1893`, `isKnownType` at
//! `:1904`, `opIs` at `:1940`, `funcIs` at `:5095`, `funcAs` at `:5155` and
//! `funcType` at `:5252` -- and from `FHIRPathUtilityClasses.ClassTypeInfo` at
//! `:63`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: type questions go through the `Model` trait
//! instead of `IWorkerContext.fetchTypeDefinition`, and a System value is a
//! [`Value`] that is not a [`Value::Node`] where upstream asks
//! `Element.isDisallowExtensions()`.
//!
//! What is the pinned engine's, kept, and which the scenarios depend on:
//!
//! - the **operator** and the **function** are two implementations that do not
//!   share code. The operator compares the right operand's text, `name` or
//!   `name.inner`, with the focus's type name and then walks the type's
//!   `baseDefinition` chain; the function reads a namespace (`System` for the
//!   nine capitalised names it knows, `FHIR` for anything else) and a name;
//! - `is` follows the hierarchy all the way, so a `code` is a `string`; `as`
//!   and `ofType` stop at a primitive, so `as(string)` on a `code` is empty;
//! - `is` on an empty operand is empty, and on a collection of more than one
//!   item it is `false` (operator) or empty (function);
//! - an unknown type name is an execution error for `as` and `ofType`, and
//!   for `is` it is simply `false`: the pinned engine does not call
//!   `isKnownType` for `is` at either stage (`DEV` entry pending for
//!   `path.types.011`, see `tasks/blockers.md`).

use std::sync::Arc;

use crate::seam::node::{Node, NodeError};
use crate::seam::value::{Primitive, Value};
use crate::syntax::ast::{Kind, NodeId};

use super::navigate::fhir_type;
use super::{EvalError, Run};

/// The System types `isKnownType` accepts without a definition.
const SYSTEM_TYPES: [&str; 9] = [
    "String",
    "Boolean",
    "Integer",
    "Decimal",
    "Quantity",
    "DateTime",
    "Time",
    "SimpleTypeInfo",
    "ClassInfo",
];

/// The names `funcIs` reads as System when no namespace is written
/// (`FHIRPathEngine.java:5112`). `Date` is in this list and not in
/// [`SYSTEM_TYPES`], which is upstream's own inconsistency.
const FUNC_IS_SYSTEM_NAMES: [&str; 9] = [
    "Boolean",
    "Integer",
    "Decimal",
    "String",
    "DateTime",
    "Date",
    "Time",
    "SimpleTypeInfo",
    "ClassInfo",
];

/// `Utilities.capitalize`.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `ClassTypeInfo`: what `type()` returns for one item, with `name` and
/// `namespace` as its only properties.
#[derive(Debug)]
struct ClassInfo {
    namespace: &'static str,
    name: String,
}

impl Node for ClassInfo {
    fn type_name(&self) -> &str {
        "ClassInfo"
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(match name {
            "name" => vec![Value::String(self.name.clone())],
            "namespace" => vec![Value::String(self.namespace.to_string())],
            _ => Vec::new(),
        })
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        // `listChildren` is empty upstream: `children()` does not see them.
        Ok(Vec::new())
    }

    fn primitive(&self) -> Option<Primitive> {
        None
    }
}

impl Run<'_> {
    /// `executeTypeName` (`FHIRPathEngine.java:1535`): the right operand of
    /// `is` and `as` is read as a type name, never evaluated.
    pub(super) fn type_name_operand(&self, id: NodeId) -> String {
        let node = self.expr.node(id);
        let name = node.name().unwrap_or("");
        match node.inner() {
            Some(inner) => format!("{name}.{}", self.expr.node(inner).name().unwrap_or("")),
            None => name.to_string(),
        }
    }

    /// `isKnownType(tn)` (`FHIRPathEngine.java:1904`), which the operator
    /// `as` calls with a name that may or may not carry a namespace.
    fn is_known_type_name(&self, tn: &str) -> bool {
        if !tn.contains('.') {
            return SYSTEM_TYPES.contains(&tn) || self.model.is_type(tn);
        }
        self.is_known_type(tn)
    }

    /// Whether `wanted` is `own` or one of its base types, following the
    /// `baseDefinition` chain through primitives as well: the loop `opIs` and
    /// `funcIs` run, which has no primitive stop.
    fn derives_through_bases(&self, own: &str, wanted: &str) -> bool {
        if own == wanted {
            return true;
        }
        if !self.model.is_type(own) {
            return false;
        }
        let mut current = Some(own);
        let mut guard = 0usize;
        while let Some(t) = current {
            if t == wanted {
                return true;
            }
            guard += 1;
            if guard > 64 {
                return false;
            }
            current = self.model.base_type(t);
        }
        false
    }

    /// `compareTypeNames` (`FHIRPathEngine.java:1893`).
    fn compare_type_names(&self, left: &str, right: &str) -> bool {
        if self.enforce_as_case_sensitive {
            left == right
        } else {
            left.eq_ignore_ascii_case(right)
        }
    }

    /// The `FHIR.` arm of `funcAs`: the item's own type name equals `wanted`
    /// exactly, or `compareTypeNames` accepts the `type` of one of its base
    /// definitions. A primitive's definition is the last one looked at.
    fn as_type_matches(&self, item: &Value, wanted: &str) -> bool {
        let own = fhir_type(item);
        if own == wanted {
            return true;
        }
        if !self.model.is_type(own) {
            return false;
        }
        let mut current = Some(own);
        let mut guard = 0usize;
        while let Some(t) = current {
            if self.compare_type_names(wanted, t) {
                return true;
            }
            guard += 1;
            if guard > 64 || self.model.is_primitive(t) {
                return false;
            }
            current = self.model.base_type(t);
        }
        false
    }

    /// `opIs` (`FHIRPathEngine.java:1940`): `left is tn`.
    pub(super) fn op_is(&self, left: &[Value], tn: &str) -> Vec<Value> {
        match left {
            [] => Vec::new(),
            [item] => {
                let own = fhir_type(item);
                let answer = if !matches!(item, Value::Node(_)) {
                    // A value the engine produced itself: `isDisallowExtensions`.
                    let cap = capitalize(own);
                    cap == tn || format!("System.{cap}") == tn
                } else {
                    self.derives_through_bases(own, tn)
                };
                vec![Value::Boolean(answer)]
            }
            _ => vec![Value::Boolean(false)],
        }
    }

    /// `opAs` (`FHIRPathEngine.java:1875`): `left as tn`.
    pub(super) fn op_as(
        &self,
        left: &[Value],
        tn: &str,
        holder: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(holder).start();
        if !self.is_known_type_name(tn) {
            return Err(EvalError::execution(
                format!("The type {tn} is not valid"),
                offset,
            ));
        }
        if self.enforce_as_singleton && left.len() > 1 {
            return Err(EvalError::execution(
                format!(
                    "Attempt to use 'as()' on more than one item ('{}', '{}')",
                    left.len(),
                    self.expr.render(holder)
                ),
                offset,
            ));
        }
        Ok(left
            .iter()
            .filter(|item| self.compare_type_names(tn, fhir_type(item)))
            .cloned()
            .collect())
    }

    /// The type name of the parameter of `as`, `ofType` and `is`, as `as` and
    /// `ofType` build it: `namespace.name` as written, else `FHIR.name`.
    fn param_type_name(&self, param: NodeId) -> String {
        let pn = self.expr.node(param);
        let head = pn.name().unwrap_or("");
        match pn.inner() {
            Some(inner) => format!("{head}.{}", self.expr.node(inner).name().unwrap_or("")),
            None => format!("FHIR.{head}"),
        }
    }

    /// `funcIs` (`FHIRPathEngine.java:5095`).
    pub(super) fn func_is(&self, focus: &[Value], id: NodeId) -> Result<Vec<Value>, EvalError> {
        let [item] = focus else {
            return Ok(Vec::new());
        };
        let node = self.expr.node(id);
        let texp_id = self.param(id, 0)?;
        let texp = self.expr.node(texp_id);
        if texp.kind() != Kind::Name {
            return Err(EvalError::execution(
                format!(
                    "The expression type {:?} is not supported for parameter 0 on function is",
                    texp.kind()
                ),
                node.start(),
            ));
        }
        let (ns, n) = match texp.inner() {
            Some(inner_id) => {
                let inner = self.expr.node(inner_id);
                if inner.kind() != Kind::Name {
                    return Err(EvalError::execution(
                        format!(
                            "The expression type {:?} is not supported for parameter 1 on function is",
                            texp.kind()
                        ),
                        node.start(),
                    ));
                }
                (
                    texp.name().unwrap_or("").to_string(),
                    inner.name().unwrap_or("").to_string(),
                )
            }
            None => {
                let name = texp.name().unwrap_or("");
                if FUNC_IS_SYSTEM_NAMES.contains(&name) {
                    ("System".to_string(), name.to_string())
                } else {
                    ("FHIR".to_string(), name.to_string())
                }
            }
        };
        let answer = match ns.as_str() {
            "System" => {
                if matches!(item, Value::Node(_)) {
                    // A resource, or an element that allows extensions.
                    false
                } else {
                    let t = capitalize(fhir_type(item));
                    n == t || (t == "Date" && n == "DateTime")
                }
            }
            "FHIR" => self.derives_through_bases(fhir_type(item), &n),
            _ => false,
        };
        Ok(vec![Value::Boolean(answer)])
    }

    /// `funcAs` (`FHIRPathEngine.java:5155`).
    pub(super) fn func_as(&self, focus: &[Value], id: NodeId) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let tn = self.param_type_name(self.param(id, 0)?);
        if !self.is_known_type_name(&tn) {
            return Err(EvalError::execution(
                format!("The type {tn} is not valid"),
                node.start(),
            ));
        }
        if self.enforce_as_singleton && focus.len() > 1 {
            return Err(EvalError::execution(
                format!(
                    "Attempt to use 'as()' on more than one item ('{}', 'as({})')",
                    focus.len(),
                    self.expr.render(id)
                ),
                node.start(),
            ));
        }
        let mut result = Vec::new();
        for b in focus {
            let matched = if let Some(name) = tn.strip_prefix("System.") {
                !matches!(b, Value::Node(_)) && fhir_type(b).eq_ignore_ascii_case(name)
            } else if let Some(name) = tn.strip_prefix("FHIR.") {
                self.as_type_matches(b, name)
            } else {
                false
            };
            if matched {
                result.push(b.clone());
            }
        }
        Ok(result)
    }

    /// `funcType` (`FHIRPathEngine.java:5252`): one `ClassInfo` per item.
    pub(super) fn func_type(&self, focus: &[Value]) -> Vec<Value> {
        focus
            .iter()
            .map(|item| {
                let (namespace, name) = match item {
                    Value::Node(_) => ("FHIR", fhir_type(item).to_string()),
                    other => ("System", capitalize(fhir_type(other))),
                };
                Value::Node(Arc::new(ClassInfo { namespace, name }))
            })
            .collect()
    }
}
