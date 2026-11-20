//! Environment variables and the constant-resolution sites: `%name`,
//! `defineVariable`, and the two implicit host-service lookups a bare name
//! makes.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `resolveConstant(context,
//! Base, ...)` at `:1653`, `isSystemVariable` at `:1743`,
//! `resolveConstant(context, String, ...)` at `:1754`, `execute(context, Base
//! item, ...)` at `:3182`, `funcDefineVariable` at `:5635` and
//! `ExecutionContext.setDefinedVariable` at `:1001`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the execution context is immutable, so a
//! defined variable is a node in a list whose tail is the enclosing scope; a
//! host service's failure is an [`EvalError::Execution`]; and `%context` over
//! an evaluation with no focus is an empty collection, where upstream adds a
//! `null` to the list.
//!
//! The four constant-evaluation modes and exactly where each applies:
//!
//! - [`ConstantMode::Explicit`] -- a `%name` the expression wrote, which is
//!   every call [`Run::resolve_constant`] makes to the host service;
//! - [`ConstantMode::ImplicitBefore`] -- the entry of evaluation, before any
//!   navigation and before the leading-type-name match. A non-empty answer is
//!   returned at once, so a host constant **shadows a real element**
//!   (`DEV-0010`);
//! - [`ConstantMode::ImplicitAfter`] -- a fall-through, after navigation
//!   yielded nothing for that item;
//! - [`ConstantMode::NoValue`] -- the check pass's one site, not reached here.
//!
//! Both implicit modes are gated on a **non-null application context**, which
//! is the condition the engine tests, and not on the host service: a host
//! service installed with no application context never sees either.

use crate::seam::host::ConstantMode;
use crate::seam::value::Value;
use crate::syntax::ast::{Constant, NodeId};
use crate::syntax::literal::process_date_constant;

use super::collection::convert_to_string;
use super::equality::primitive_text;
use super::error::At;
use super::{Context, EvalError, Run, is_system_variable, navigate};

impl Run<'_> {
    /// One name step over one item (`execute(context, Base item, exp,
    /// atEntry)`, `FHIRPathEngine.java:3182`).
    pub(super) fn execute_name(
        &self,
        item: &Value,
        name: &str,
        at_entry: bool,
        out: &mut Vec<Value>,
    ) -> Result<(), EvalError> {
        let gated = at_entry && self.app_context;
        if gated {
            if let Some(host) = self.host {
                let known = host
                    .resolve_constant(name, ConstantMode::ImplicitBefore)
                    .map_err(|e| EvalError::plain(e.message))?;
                if !known.is_empty() {
                    out.extend(known);
                    return Ok(());
                }
            }
        }
        let before = out.len();
        if at_entry && name.chars().next().is_some_and(char::is_uppercase) {
            if navigate::type_name_matches(self.model, item, name) {
                out.push(item.clone());
            }
        } else {
            navigate::children_by_name(item, name, out)?;
        }
        if gated && out.len() == before {
            if let Some(host) = self.host {
                out.extend(
                    host.resolve_constant(name, ConstantMode::ImplicitAfter)
                        .map_err(|e| EvalError::plain(e.message))?,
                );
            }
        }
        Ok(())
    }

    /// `resolveConstant(context, constant, expr, mode)`
    /// (`FHIRPathEngine.java:1653`).
    pub(super) fn resolve_constant(
        &self,
        context: Context<'_>,
        id: NodeId,
        mode: ConstantMode,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        match node.constant() {
            None | Some(Constant::Empty) => Ok(Vec::new()),
            Some(Constant::Value(value)) => Ok(vec![value.clone()]),
            Some(Constant::Deferred(text)) => {
                if let Some(run) = text.strip_prefix('@') {
                    // A new value on every evaluation of the node, as
                    // `processDateConstant` builds a new object
                    // (`path.compare.015`).
                    process_date_constant(run)
                        .map(|mut v| {
                            if let Value::DateTime(d) = &mut v {
                                d.occurrence = super::rewrite::new_occurrence();
                            }
                            vec![v]
                        })
                        .map_err(EvalError::plain)
                } else if let Some(name) = text.strip_prefix('%') {
                    if let Some(defined) = context.defined_variable(name) {
                        return Ok(defined.to_vec());
                    }
                    self.resolve_environment(context, text, mode, At::node(node))
                } else {
                    Err(EvalError::execution(
                        format!(
                            "Error evaluating FHIRPath expression: Invalid FHIRPath Constant {text}"
                        ),
                        At::node(node),
                    ))
                }
            }
        }
    }

    /// `resolveConstant(context, String s, expr, mode)`
    /// (`FHIRPathEngine.java:1754`): the engine's own table, then the host
    /// service, then the unknown-constant error.
    fn resolve_environment(
        &self,
        _context: Context<'_>,
        s: &str,
        mode: ConstantMode,
        at: At,
    ) -> Result<Vec<Value>, EvalError> {
        let one = |text: String| Ok(vec![Value::String(text)]);
        match s {
            "%sct" => one("http://snomed.info/sct".to_string()),
            "%loinc" => one("http://loinc.org".to_string()),
            "%ucum" => one("http://unitsofmeasure.org".to_string()),
            "%resource" => match self.focus_resource {
                Some(resource) => Ok(vec![resource.clone()]),
                None => Err(EvalError::execution(
                    "Error evaluating FHIRPath expression: Cannot use %resource in this \
                     context because no focus resource",
                    at,
                )),
            },
            "%rootResource" => match self.root_resource {
                Some(resource) => Ok(vec![resource.clone()]),
                None => Err(EvalError::execution(
                    "Error evaluating FHIRPath expression: Cannot use %rootResource in this \
                     context because no focus rootResource",
                    at,
                )),
            },
            // Upstream adds `context.context` with no guard, which for an
            // evaluation with no base is a `null` in the list.
            "%context" => Ok(self.base.cloned().into_iter().collect()),
            "%us-zip" => one("[0-9]{5}(-[0-9]{4}){0,1}".to_string()),
            _ if s.starts_with("%`vs-") => one(format!(
                "http://hl7.org/fhir/ValueSet/{}",
                shorthand(s, "%`vs-")
            )),
            _ if s.starts_with("%`cs-") => {
                one(format!("http://hl7.org/fhir/{}", shorthand(s, "%`cs-")))
            }
            _ if s.starts_with("%`ext-") => one(format!(
                "http://hl7.org/fhir/StructureDefinition/{}",
                shorthand(s, "%`ext-")
            )),
            _ => match self.host {
                None => Err(EvalError::execution(
                    format!("Error evaluating FHIRPath expression: Invalid FHIRPath Constant {s}"),
                    at,
                )),
                Some(host) => host
                    .resolve_constant(&s[1..], mode)
                    .map_err(|e| EvalError::plain(e.message)),
            },
        }
    }

    /// `funcDefineVariable` (`FHIRPathEngine.java:5635`): the name, the value
    /// the variable takes and the focus, which is the function's result. The
    /// caller makes the variable visible to the rest of the chain.
    pub(super) fn func_define_variable(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<(String, Vec<Value>, Vec<Value>), EvalError> {
        let node = self.expr.node(id);
        let nl = self.execute(context, focus, self.param(id, 0)?, true)?;
        let Some(first) = nl.first() else {
            return Err(EvalError::plain("Index 0 out of bounds for length 0"));
        };
        let name = primitive_text(self.model, first).unwrap_or_else(|| convert_to_string(first));
        let value = match node.parameters().get(1) {
            Some(&p) => self.execute(context, focus, p, true)?,
            None => focus.to_vec(),
        };
        if is_system_variable(&name) || context.defined_variable(&name).is_some() {
            return Err(EvalError::plain(format!(
                "The variable '{name}' cannot be redefined"
            )));
        }
        Ok((name, value, focus.to_vec()))
    }
}

/// The part of a backtick shorthand between its prefix and the closing
/// backtick: `s.substring(prefix.length(), s.length()-1)`.
fn shorthand<'s>(s: &'s str, prefix: &str) -> &'s str {
    s.get(prefix.len()..s.len().saturating_sub(1)).unwrap_or("")
}
