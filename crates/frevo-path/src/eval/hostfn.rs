//! The host-service functions: `conformsTo`, both `memberOf` forms,
//! `resolve()` and a host-supplied (`Custom`) function.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `opMemberOf` at `:2602`,
//! the `Custom` arm of `evaluateFunction` at `:4020`-`:4040`, `funcMemberOf`
//! at `:4786`, `makeIdentifier` at `:5403`, `funcResolve` at `:5413` and
//! `funcConformsTo` at `:5972`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: `worker.validateCode` is the in-memory
//! subset in [`super::terminology`], and what that subset cannot decide is an
//! [`EvalError::Unsupported`]; a state in which upstream dereferences `null`
//! (a `Custom` function or a `#` reference with no host service or no root
//! resource) is a typed execution error.
//!
//! What is the pinned engine's, kept:
//!
//! - `conformsTo` with **no** host service raises before reading its argument
//!   (`path.utility.022`), with one answers `false` for a focus of other than
//!   one item, and propagates the host service's error (`path.utility.013`);
//! - `memberOf()` is empty unless the focus and the argument are singletons,
//!   empty for an unresolvable value set, and empty for a focus that is not a
//!   `code`, `string`, `uri`, `Coding` or `CodeableConcept`
//!   (`path.utility.016`, `026`); the `memberOf` operator ORs over the whole
//!   left collection and is `false` for an unresolvable value set
//!   (`path.utility.024`, `025`); both read the host service's value set when
//!   one is installed and the worker context's otherwise;
//! - `resolve()` resolves a `#id` against the **root** resource's `contained`
//!   itself and hands every other reference to the host service, swallowing
//!   its error (`path.utility.017`-`020`);
//! - a custom function's first argument is passed as a type name, unevaluated,
//!   when `param_is_type(name, 0)` says so, and only the first
//!   (`path.utility.027`).

use std::sync::Arc;

use crate::seam::node::{Node, NodeError};
use crate::seam::value::{Primitive, Value};
use crate::syntax::ast::NodeId;

use super::collection::{convert_list_to_string, convert_to_string};
use super::equality::primitive_text;
use super::navigate::{children_by_name, fhir_type};
use super::terminology::{self, CodeToCheck};
use super::{Context, EvalError, Run};

/// `I18nConstants.FHIRPATH_HO_HOST_SERVICES`.
fn no_host_services(what: &str, offset: usize) -> EvalError {
    EvalError::execution(
        format!(
            "Internal Error evaluating FHIRPath expression: No host services are provided ({what})"
        ),
        offset,
    )
}

/// A primitive the engine makes itself: the `CodeType` a type-name argument
/// of a custom function becomes.
#[derive(Debug)]
struct MadePrimitive {
    type_name: &'static str,
    value: String,
}

impl Node for MadePrimitive {
    fn type_name(&self) -> &str {
        self.type_name
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, _name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(Vec::new())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(Vec::new())
    }

    fn primitive(&self) -> Option<Primitive> {
        Some(Primitive {
            type_name: self.type_name.to_string(),
            lexical: self.value.clone(),
            id: None,
            extensions: Vec::new(),
        })
    }
}

/// `Base.isPrimitive()`: a literal, or a node that is a FHIR primitive.
fn is_primitive(run: &Run<'_>, item: &Value) -> bool {
    match item {
        Value::Node(n) => n.primitive().is_some() || run.model.is_primitive(n.type_name()),
        Value::Quantity(_) => false,
        _ => true,
    }
}

/// The resolved value set, or why there is none.
enum ValueSetAnswer {
    Found(Value),
    Absent,
}

impl Run<'_> {
    /// `hostServices != null ? hostServices.resolveValueSet(...) :
    /// worker.fetchResource(ValueSet.class, url)`.
    fn resolve_value_set(&self, url: &str, offset: usize) -> Result<ValueSetAnswer, EvalError> {
        let found = match self.host {
            Some(host) => host
                .resolve_value_set(url)
                .map_err(|e| EvalError::execution(e.message, offset))?,
            None => self.model.value_set(url).ok_or_else(|| {
                EvalError::unsupported(
                    format!("memberOf() against {url}: this model carries no value sets"),
                    offset,
                )
            })?,
        };
        Ok(match found {
            Some(vs) => ValueSetAnswer::Found(vs),
            None => ValueSetAnswer::Absent,
        })
    }

    /// The validation `funcMemberOf` and `opMemberOf` apply to one item:
    /// `None` for a type neither validates, and otherwise whether
    /// `worker.validateCode(...)` is ok.
    fn member_of_item(
        &self,
        item: &Value,
        vs: &Value,
        offset: usize,
    ) -> Result<Option<bool>, EvalError> {
        let codes = match fhir_type(item) {
            // `castToCoding` of a `code`, `string` or `uri` is a Coding with
            // that code and no system, validated with system guessing on.
            "code" | "string" | "uri" => vec![CodeToCheck {
                system: None,
                code: primitive_text(self.model, item),
                guess_system: true,
            }],
            "Coding" => vec![self.coding_of(item)?],
            "CodeableConcept" => {
                let mut codings = Vec::new();
                children_by_name(item, "coding", &mut codings)?;
                codings
                    .iter()
                    .map(|c| self.coding_of(c))
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Ok(None),
        };
        let mut ok = false;
        for code in &codes {
            if terminology::validate_code(self.model, code, vs, offset)? {
                ok = true;
                break;
            }
        }
        Ok(Some(ok))
    }

    /// A `Coding`'s system and code, validated without system guessing.
    fn coding_of(&self, coding: &Value) -> Result<CodeToCheck, EvalError> {
        let first_text = |name: &str| -> Result<Option<String>, EvalError> {
            let mut out = Vec::new();
            children_by_name(coding, name, &mut out)?;
            Ok(out.first().and_then(|v| primitive_text(self.model, v)))
        };
        Ok(CodeToCheck {
            system: first_text("system")?,
            code: first_text("code")?,
            guess_system: false,
        })
    }

    /// `funcMemberOf` (`FHIRPathEngine.java:4786`).
    pub(super) fn func_member_of(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let nl = self.execute(context, focus, self.param(id, 0)?, true)?;
        let ([url_item], [item]) = (nl.as_slice(), focus) else {
            return Ok(Vec::new());
        };
        let Some(url) = primitive_text(self.model, url_item) else {
            return Err(EvalError::unsupported(
                "memberOf() with an argument that has no primitive value",
                offset,
            ));
        };
        let vs = match self.resolve_value_set(&url, offset)? {
            ValueSetAnswer::Found(vs) => vs,
            ValueSetAnswer::Absent => return Ok(Vec::new()),
        };
        Ok(match self.member_of_item(item, &vs, offset)? {
            Some(ok) => vec![Value::Boolean(ok)],
            None => Vec::new(),
        })
    }

    /// `opMemberOf` (`FHIRPathEngine.java:2602`): one boolean for the whole
    /// left collection, `true` when any item validates.
    pub(super) fn op_member_of(
        &self,
        left: &[Value],
        right: &[Value],
        holder: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(holder).op_start();
        // `right.get(0).primitiveValue()` raises on an empty right operand.
        let Some(first) = right.first() else {
            return Err(EvalError::execution(
                "Index 0 out of bounds for length 0",
                offset,
            ));
        };
        let Some(url) = primitive_text(self.model, first) else {
            return Err(EvalError::unsupported(
                "the memberOf operator with a right operand that has no primitive value",
                offset,
            ));
        };
        let mut ans = false;
        if let ValueSetAnswer::Found(vs) = self.resolve_value_set(&url, offset)? {
            for l in left {
                if self.member_of_item(l, &vs, offset)? == Some(true) {
                    ans = true;
                }
            }
        }
        Ok(vec![Value::Boolean(ans)])
    }

    /// `funcConformsTo` (`FHIRPathEngine.java:5972`).
    pub(super) fn func_conforms_to(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let Some(host) = self.host else {
            return Err(no_host_services("conformsTo", offset));
        };
        let [item] = focus else {
            return Ok(vec![Value::Boolean(false)]);
        };
        let url =
            convert_list_to_string(&self.execute(context, focus, self.param(id, 0)?, true)?);
        let answer = host
            .conforms_to_profile(item, &url)
            .map_err(|e| EvalError::execution(e.message, offset))?;
        Ok(vec![Value::Boolean(answer)])
    }

    /// `funcResolve` (`FHIRPathEngine.java:5413`).
    pub(super) fn func_resolve(
        &self,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let mut result = Vec::new();
        for item in focus {
            let mut url: Option<String> = None;
            let mut identifier: Option<Value> = None;
            if fhir_type(item) == "Reference" {
                let mut refs = Vec::new();
                children_by_name(item, "reference", &mut refs)?;
                if let Some(r) = refs.first() {
                    url = Some(convert_to_string(r));
                }
                let mut ids = Vec::new();
                children_by_name(item, "identifier", &mut ids)?;
                // `makeIdentifier`: only an `Identifier` is one.
                identifier = ids
                    .into_iter()
                    .next()
                    .filter(|v| fhir_type(v) == "Identifier");
            } else if is_primitive(self, item) {
                url = primitive_text(self.model, item);
            }
            if url.is_none() && identifier.is_none() {
                continue;
            }
            let res = match url.as_deref() {
                Some(u) if u.starts_with('#') => self.resolve_contained(&u[1..], offset)?,
                _ => match self.host {
                    None => None,
                    Some(host) => {
                        let Some(u) = url.as_deref() else {
                            // Upstream passes a null URL with the identifier;
                            // the host-service seam takes a URL, so this case
                            // cannot be expressed through it.
                            return Err(EvalError::unsupported(
                                "resolve() of a Reference that carries only an identifier",
                                offset,
                            ));
                        };
                        // The one place an error becomes an absent value
                        // (`FHIRPathEngine.java:5449-5453`).
                        host.resolve_reference(u, identifier.as_ref(), Some(item))
                            .ok()
                            .flatten()
                    }
                },
            };
            if let Some(res) = res {
                result.push(res);
            }
        }
        Ok(result)
    }

    /// The `#id` branch: the first resource in the **root** resource's
    /// `contained` whose id is `id`.
    fn resolve_contained(&self, id: &str, offset: usize) -> Result<Option<Value>, EvalError> {
        let Some(Value::Node(root)) = self.root_resource else {
            // `context.rootResource.getChildByName(...)` on a null root.
            return Err(EvalError::execution(
                format!("resolve() of #{id} needs a root resource, and there is none"),
                offset,
            ));
        };
        for c in root.children("contained")? {
            let Value::Node(node) = &c else { continue };
            // `getIdBase()` is the id element's value as written.
            let matches = node
                .children("id")?
                .first()
                .and_then(|v| match v {
                    Value::Node(n) => n.primitive().map(|p| p.lexical),
                    other => Some(convert_to_string(other)),
                })
                .is_some_and(|v| v == id);
            if matches {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    /// The `Custom` arm of `evaluateFunction` (`FHIRPathEngine.java:4020`).
    pub(super) fn func_custom(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let name = node.name().unwrap_or("");
        let Some(host) = self.host else {
            return Err(no_host_services(&format!("{name}()"), node.start()));
        };
        let mut params: Vec<Vec<Value>> = Vec::new();
        if host.param_is_type(name, 0) {
            if let Some(&p) = node.parameters().first() {
                let pnode = self.expr.node(p);
                let pname = pnode.name().unwrap_or("");
                let tn = match pnode.inner() {
                    Some(inner) => {
                        format!("{pname}.{}", self.expr.node(inner).name().unwrap_or(""))
                    }
                    None => format!("FHIR.{pname}"),
                };
                params.push(vec![Value::Node(Arc::new(MadePrimitive {
                    type_name: "code",
                    value: tn,
                }))]);
            }
        } else {
            for &p in node.parameters() {
                params.push(self.execute(context, focus, p, true)?);
            }
        }
        host.execute_function(name, focus, &params)
            .map_err(|e| EvalError::execution(e.message, node.start()))
    }
}
