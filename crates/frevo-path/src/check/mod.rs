//! The static type-check pass: `FHIRPathEngine.check` and `checkOnTypes`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 — `check` at `:405`-`:463`
//! and `checkOnTypes` at `:477`-`:544`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the pass is separate from evaluation and
//! shares nothing with it but the parsed [`Expression`], which it only
//! borrows; the per-function rules are a table (`signature.rs`); type
//! information comes through the [`Model`] trait; and a question the pass or
//! its model cannot answer is [`CheckError::NotImplemented`], never a pass.
//!
//! Task 50 of `add-fhirpath-and-codegen-r4` (`path.check.001`-`004`, `013`,
//! `014`). The warning list is collected as upstream collects it
//! ([`Checker::check_with_warnings`]); grading it is task 50a. Type-guard
//! narrowing (task 51) is **not** here, because the pinned engine has none:
//! its `executeType` checks every operand of `implies`, `and`, `or` and
//! `iif` against the same focus.
//!
//! What makes a check fail is upstream's: an unknown element name, a
//! function over a focus it does not accept, a parameter of the wrong type,
//! an ordered function over an unordered collection, date arithmetic with a
//! non-quantity, an unknown or unusable constant, an unknown context.
//!
//! Performance (`path.perf.001`): one walk over the parsed form, with no
//! evaluation and no resource. The specialisations of a type and the resource
//! names are computed once per check and shared.

mod error;
mod pass;
mod signature;
mod types;

pub use error::{CheckError, CheckWarning, WarningId};
pub use types::CollectionStatus;

use std::rc::Rc;

use pass::{Pass, TypeContext};
use types::{Types, is_backbone};

use crate::seam::host::HostServices;
use crate::seam::model::{ElementInfo, Model, TypeSet};
use crate::syntax::ast::Expression;

/// What the expression is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckFocus<'a> {
    /// No focus type at all: `check(appContext, root, sd, expr)` with a null
    /// context. A leading name is then a host-service constant in the
    /// no-value mode.
    #[default]
    None,
    /// A context, as `check(..., context, expr)` takes it: a type name
    /// (`Patient`) or an element path (`Observation.effective`), whose
    /// declared types are the focus.
    Context(&'a str),
    /// A list of type names, as `checkOnTypes` takes it — which is what a
    /// suite case's `contextType="Period|Timing"` means.
    Types(&'a [&'a str]),
}

/// One check request.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckRequest<'a> {
    /// The type `%rootResource` has.
    pub root_resource_type: Option<&'a str>,
    /// The type `%resource` has.
    pub resource_type: Option<&'a str>,
    pub focus: CheckFocus<'a>,
}

/// The settings the check pass reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckOptions {
    /// `allowPolymorphicNames`, off by default: `valueQuantity` is then an
    /// unknown name.
    pub allow_polymorphic_names: bool,
    /// `emitSQLonFHIRWarning`, off by default (`path.check.017`).
    pub emit_sql_on_fhir_warning: bool,
}

/// A successful check: the inferred types and the warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub types: TypeSet,
    pub collection_status: Option<CollectionStatus>,
    pub warnings: Vec<CheckWarning>,
}

/// The static type-check pass over one model, with or without host services.
///
/// A `Checker` holds no per-check state: the warning list is built fresh by
/// every check, so nothing leaks from one expression to the next.
#[derive(Clone, Copy)]
pub struct Checker<'a> {
    model: &'a dyn Model,
    host: Option<&'a dyn HostServices>,
    options: CheckOptions,
}

impl<'a> Checker<'a> {
    pub fn new(model: &'a dyn Model) -> Self {
        Checker {
            model,
            host: None,
            options: CheckOptions::default(),
        }
    }

    /// Install host services, which the pass asks for the type of a constant
    /// it does not know.
    pub fn with_host_services(mut self, host: &'a dyn HostServices) -> Self {
        self.host = Some(host);
        self
    }

    pub fn with_options(mut self, options: CheckOptions) -> Self {
        self.options = options;
        self
    }

    /// Check `expr`, answering the inferred type set. Warnings are dropped
    /// (`path.check.019`).
    pub fn check(
        &self,
        expr: &Expression,
        request: &CheckRequest<'_>,
    ) -> Result<TypeSet, CheckError> {
        self.check_with_warnings(expr, request).map(|o| o.types)
    }

    /// Check `expr`, answering the inferred types and the warnings.
    pub fn check_with_warnings(
        &self,
        expr: &Expression,
        request: &CheckRequest<'_>,
    ) -> Result<CheckOutcome, CheckError> {
        let types = self.focus_types(expr, request.focus)?;
        let types = types.map(Rc::new);
        let context = TypeContext::new(
            request.root_resource_type,
            request.resource_type,
            types.clone(),
        );
        let mut pass = Pass::new(self.model, self.host, expr);
        pass.allow_polymorphic_names = self.options.allow_polymorphic_names;
        pass.emit_sql_on_fhir_warning = self.options.emit_sql_on_fhir_warning;
        let root = expr.root();
        let result = pass.execute(&context, types.as_deref(), root, true, false, root)?;
        Ok(CheckOutcome {
            types: result.to_type_set(),
            collection_status: result.status(),
            warnings: pass.warnings,
        })
    }

    /// The focus type set a request describes (`check`'s and
    /// `checkOnTypes`'s preamble).
    fn focus_types(
        &self,
        expr: &Expression,
        focus: CheckFocus<'_>,
    ) -> Result<Option<Types>, CheckError> {
        let offset = expr.node(expr.root()).start();
        match focus {
            CheckFocus::None => Ok(None),
            CheckFocus::Context(context) => {
                let mut types = Types::new(Some(CollectionStatus::Singleton));
                self.add_context_types(context, &mut types, offset, "FHIRPATH_UNKNOWN_CONTEXT")?;
                Ok(Some(types))
            }
            CheckFocus::Types(list) => {
                let mut types = Types::new(Some(CollectionStatus::Singleton));
                for t in list {
                    self.add_context_types(t, &mut types, offset, "FHIRPATH_UNKNOWN_CONTEXT")?;
                }
                Ok(Some(types))
            }
        }
    }

    /// One context: a type name, or an element path whose declared types
    /// are added (`getElementDefinition(sd, context, true, expr)`).
    fn add_context_types(
        &self,
        context: &str,
        types: &mut Types,
        offset: usize,
        unknown: &'static str,
    ) -> Result<(), CheckError> {
        let unknown_context = || {
            CheckError::failed(
                unknown,
                format!("Unknown context evaluating FHIRPath expression: {context}"),
                offset,
            )
        };
        if context.contains('#') || context.contains("://") {
            return Err(CheckError::not_implemented(
                format!("a context given as a URL or a fragment ({context})"),
                offset,
            ));
        }
        let mut segments = context.split('.');
        let head = segments.next().unwrap_or("");
        if !self.model.is_type(head) {
            return Err(unknown_context());
        }
        let rest: Vec<&str> = segments.collect();
        if rest.is_empty() {
            types.add(head);
            return Ok(());
        }
        let unknown_element = || {
            CheckError::failed(
                "FHIRPATH_UNKNOWN_CONTEXT_ELEMENT",
                format!("Unknown context element evaluating FHIRPath expression: {context}"),
                offset,
            )
        };
        let mut current = head.to_string();
        let mut last: Option<(ElementInfo, Option<String>)> = None;
        for (i, segment) in rest.iter().enumerate() {
            let Some(elements) = self.model.elements(&current) else {
                return Err(CheckError::not_implemented(
                    format!(
                        "the elements of {current}, which the model given to the check pass \
                         cannot describe"
                    ),
                    offset,
                ));
            };
            let found = match_context_element(elements, segment, self.model);
            let Some((e, fixed)) = found else {
                return Err(unknown_element());
            };
            if i + 1 < rest.len() {
                // Walking further needs exactly one type to walk into.
                let next = match (&fixed, e.types.as_slice()) {
                    (Some(f), _) => f.clone(),
                    (None, [only]) => only.clone(),
                    _ => {
                        return Err(CheckError::not_implemented(
                            format!("a context path through a multi-typed element ({context})"),
                            offset,
                        ));
                    }
                };
                current = next;
            }
            last = Some((e, fixed));
        }
        let Some((e, fixed)) = last else {
            return Err(unknown_element());
        };
        if let Some(fixed) = fixed {
            types.add(&fixed);
        } else if e.content_reference.is_some()
            || e.codes.is_empty()
            || (e.codes.len() == 1 && (e.codes[0] == "Element" || e.codes[0] == "BackboneElement"))
        {
            // No type, or an abstract one: the element itself is the type.
            match e.types.first() {
                Some(t) if is_backbone(t) => types.add(t),
                _ => types.add(&e.path),
            }
        } else {
            for code in &e.codes {
                types.add(code);
            }
        }
        Ok(())
    }
}

/// The element a context path segment names, with the fixed type a typed
/// choice name selects (`valueQuantity` -> `Quantity`).
fn match_context_element(
    elements: Vec<ElementInfo>,
    segment: &str,
    model: &dyn Model,
) -> Option<(ElementInfo, Option<String>)> {
    for e in elements {
        if e.name == segment {
            return Some((e, None));
        }
        if e.choice && segment.len() > e.name.len() && segment.starts_with(e.name.as_str()) {
            let rest = &segment[e.name.len()..];
            let mut chars = rest.chars();
            let lowered: String = match chars.next() {
                Some(c) => c.to_lowercase().chain(chars).collect(),
                None => String::new(),
            };
            let fixed = if model.is_primitive(&lowered) {
                lowered
            } else {
                rest.to_string()
            };
            return Some((e, Some(fixed)));
        }
    }
    None
}
