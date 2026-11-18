//! The walk: `executeType` over a parsed expression.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 — `executeType` at `:1574`,
//! `operateTypes` at `:1987`, `resolveConstantType` at `:3111`,
//! `executeContextType` at `:3224`, `executeType(String, ...)` at `:3231`,
//! `evaluateParameters` and `evaluateFunctionType` at `:3261`-`:3745`,
//! `checkType` at `:3767`, `checkParamTypes` at `:3820`,
//! `getChildTypesByName` at `:6322`, `ExecutionTypeContext` at `:1022`,
//! `case Extension` at `:3556` and `findExtensionDefinition` at `:3747`
//! (with `Utilities.isAbsoluteUrl`).
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the per-function `switch` is the table in
//! `signature.rs`; the type context is an immutable value cloned where
//! upstream copies it (`contextForParameter`, `changeThis`); the model is the
//! `Model` trait; and every path on which the pinned engine would dereference
//! a null type set, or throw a `java.lang.Error`, answers
//! `CheckError::NotImplemented` rather than a verdict.

use std::collections::HashMap;
use std::rc::Rc;

use super::error::{CheckError, CheckWarning, WarningId};
use super::signature::{self, Focus, Param, Params, Signature, Special, Yields};
use super::types::{
    CollectionStatus, SYSTEM_BOOLEAN, SYSTEM_CLASS_INFO, SYSTEM_DATE_TIME, SYSTEM_DECIMAL,
    SYSTEM_INTEGER, SYSTEM_QUANTITY, SYSTEM_SIMPLE_TYPE_INFO, SYSTEM_STRING, SYSTEM_TIME, Types,
    is_backbone, is_system, url_of,
};
use crate::seam::host::{ConstantMode, HostServices};
use crate::seam::model::{ElementInfo, Model};
use crate::seam::value::Value;
use crate::syntax::ast::{Constant, Expression, Function, Kind, NodeId, Operation};

type CheckResult<T> = Result<T, CheckError>;

/// The variable names `setDefinedVariable` refuses (`isSystemVariable`).
const SYSTEM_VARIABLES: &[&str] = &[
    "sct",
    "loinc",
    "ucum",
    "resource",
    "rootResource",
    "context",
];

/// A bound on `repeat()`'s fixed point. Upstream's loop terminates after two
/// rounds whenever the projection has a collection status, and never when it
/// has none; the bound turns the second into "not implemented" rather than a
/// hang.
const REPEAT_ROUNDS: usize = 16;

/// `ExecutionTypeContext`.
#[derive(Clone, Default)]
pub(crate) struct TypeContext {
    pub root_resource: Option<String>,
    pub resource: Option<String>,
    pub context: Option<Rc<Types>>,
    pub this_item: Option<Rc<Types>>,
    vars: Vec<(String, Rc<Types>)>,
}

impl TypeContext {
    /// The context a check starts in: no variables defined.
    pub(crate) fn new(
        root_resource: Option<&str>,
        resource: Option<&str>,
        focus: Option<Rc<Types>>,
    ) -> Self {
        TypeContext {
            root_resource: root_resource.map(str::to_string),
            resource: resource.map(str::to_string),
            context: focus.clone(),
            this_item: focus,
            vars: Vec::new(),
        }
    }

    fn defined(&self, name: &str) -> Option<&Rc<Types>> {
        self.vars.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    /// `changeThis`.
    fn change_this(&self, this: Option<&Types>) -> TypeContext {
        TypeContext {
            this_item: this.map(|t| Rc::new(t.clone())),
            ..self.clone()
        }
    }

    /// `setDefinedVariable`.
    fn define(&mut self, name: &str, value: Types, offset: usize) -> CheckResult<()> {
        if SYSTEM_VARIABLES.contains(&name) || self.defined(name).is_some() {
            return Err(CheckError::failed(
                "FHIRPATH_REDEFINE_VARIABLE",
                format!("Redefine of variable {name}"),
                offset,
            ));
        }
        self.vars.push((name.to_string(), Rc::new(value)));
        Ok(())
    }
}

/// One check of one expression.
pub(crate) struct Pass<'a> {
    pub model: &'a dyn Model,
    pub host: Option<&'a dyn HostServices>,
    pub expr: &'a Expression,
    pub allow_polymorphic_names: bool,
    pub emit_sql_on_fhir_warning: bool,
    pub warnings: Vec<CheckWarning>,
    descendants_cache: HashMap<String, Rc<Vec<String>>>,
    resource_names: Option<Rc<Vec<String>>>,
}

impl<'a> Pass<'a> {
    pub(crate) fn new(
        model: &'a dyn Model,
        host: Option<&'a dyn HostServices>,
        expr: &'a Expression,
    ) -> Self {
        Pass {
            model,
            host,
            expr,
            allow_polymorphic_names: false,
            emit_sql_on_fhir_warning: false,
            warnings: Vec::new(),
            descendants_cache: HashMap::new(),
            resource_names: None,
        }
    }

    fn warn(&mut self, id: WarningId, message: String) {
        self.warnings.push(CheckWarning { id, message });
    }

    fn text(&self, id: NodeId) -> String {
        self.expr.render(id)
    }

    fn start(&self, id: NodeId) -> usize {
        self.expr.node(id).start()
    }

    /// Where the pinned engine dereferences a null type set: a
    /// `NullPointerException`, which no caller can rely on, so not a verdict.
    fn null_focus(&self, what: &str, id: NodeId) -> CheckError {
        CheckError::not_implemented(
            format!(
                "{what} with no focus type, on which the pinned engine raises a \
                 NullPointerException"
            ),
            self.start(id),
        )
    }

    fn model_cannot(&self, what: &str, id: NodeId) -> CheckError {
        CheckError::not_implemented(
            format!("{what}, which the model given to the check pass cannot describe"),
            self.start(id),
        )
    }

    // -----------------------------------------------------------------
    // executeType
    // -----------------------------------------------------------------

    /// `executeType(ExecutionTypeContext, TypeDetails, ExpressionNode, ...)`.
    pub(crate) fn execute(
        &mut self,
        in_context: &TypeContext,
        focus: Option<&Types>,
        id: NodeId,
        at_entry: bool,
        can_be_none: bool,
        container: NodeId,
    ) -> CheckResult<Types> {
        // `contextForParameter`: a copy, so a variable defined here reaches
        // this node's inner chain and not its operation siblings.
        let mut context = in_context.clone();
        let node = self.expr.node(id);
        let mut result = Types::new(None);
        match node.kind() {
            Kind::Name => {
                let name = node.name().unwrap_or("");
                if at_entry && name == "$this" {
                    match &context.this_item {
                        Some(t) => result.update(t),
                        None => return Err(self.null_focus("$this", id)),
                    }
                } else if at_entry && name == "$total" {
                    result.update(&self.anything(Some(CollectionStatus::Unordered), id)?);
                } else if at_entry && name == "$index" {
                    result.add(SYSTEM_INTEGER);
                } else if at_entry && focus.is_none() {
                    result.update(&self.context_type(name, id)?);
                } else {
                    let Some(focus) = focus else {
                        return Err(self.null_focus(&format!("the name {name}"), id));
                    };
                    for s in focus.names().to_vec() {
                        let t = self.execute_type_name_on(&s, id, at_entry, focus)?;
                        result.update(&t);
                    }
                    if result.has_no_types() && !can_be_none {
                        return Err(CheckError::failed(
                            "FHIRPATH_UNKNOWN_NAME",
                            format!(
                                "Error evaluating FHIRPath expression: The name '{name}' is not \
                                 valid for any of the possible types: {}",
                                focus.describe()
                            ),
                            self.start(id),
                        ));
                    }
                }
                self.sql_on_fhir_check(&result, id);
            }
            Kind::Function => {
                let t = self.evaluate_function_type(&mut context, focus, id, container)?;
                result.update(&t);
            }
            Kind::Unary => {
                result.add(SYSTEM_INTEGER);
                result.add(SYSTEM_DECIMAL);
                result.add(SYSTEM_QUANTITY);
            }
            Kind::Constant => {
                let t = self.constant_type(&context, id)?;
                result.update(&t);
            }
            Kind::Group => {
                if let Some(group) = node.group() {
                    let t = self.execute(&context, focus, group, at_entry, can_be_none, id)?;
                    result.update(&t);
                }
            }
        }

        if let Some(inner) = node.inner() {
            result = self.execute(&context, Some(&result), inner, false, false, id)?;
        }

        if node.is_proximal() && node.operation().is_some() {
            let mut last = id;
            let mut next = node.op_next();
            while let Some(next_id) = next {
                let Some(op) = self.expr.node(last).operation() else {
                    break;
                };
                let work = if matches!(op, Operation::Is | Operation::As) {
                    // `executeTypeName`: the operand's name, unchecked.
                    match self.expr.node(next_id).name() {
                        Some(n) => Types::single(&[n]),
                        None => return Err(self.null_focus("a type operand", next_id)),
                    }
                } else {
                    self.execute(in_context, focus, next_id, at_entry, can_be_none, id)?
                };
                result = self.operate_types(&result, op, &work, last)?;
                last = next_id;
                next = self.expr.node(next_id).op_next();
            }
        }
        Ok(result)
    }

    /// `doSQLOnFHIRCheck`.
    fn sql_on_fhir_check(&mut self, focus: &Types, id: NodeId) {
        if !self.emit_sql_on_fhir_warning {
            return;
        }
        let inner_is_of_type = self
            .expr
            .node(id)
            .inner()
            .is_some_and(|i| self.expr.node(i).function() == Some(Function::OfType));
        if focus.is_choice() {
            if !inner_is_of_type {
                let m = format!(
                    "The expression {} refers to a choice element without a type specifier",
                    self.text(id)
                );
                self.warn(WarningId::ChoiceNoTypeSpecifier, m);
            }
        } else if inner_is_of_type {
            let m = format!(
                "The expression {} has a type specifier on an element that is not a choice",
                self.text(id)
            );
            self.warn(WarningId::ChoiceSpuriousTypeSpecifier, m);
        }
    }

    /// `executeContextType`: a name at the entry of an expression checked
    /// with no focus type at all, the one site of the no-value mode.
    fn context_type(&self, name: &str, id: NodeId) -> CheckResult<Types> {
        let Some(host) = self.host else {
            return Err(CheckError::failed(
                "FHIRPATH_HO_HOST_SERVICES",
                "Internal Error evaluating FHIRPath expression: No host services are provided \
                 (Context Reference)",
                self.start(id),
            ));
        };
        match host.resolve_constant_type(name, ConstantMode::NoValue) {
            Err(e) => Err(CheckError::failed(
                "HOST_SERVICE",
                e.message,
                self.start(id),
            )),
            Ok(Some(t)) => Ok(Types::single(
                &t.names().iter().map(String::as_str).collect::<Vec<_>>(),
            )),
            Ok(None) => Err(self.null_focus(
                &format!("the name {name}, for which the host service has no type"),
                id,
            )),
        }
    }

    /// `executeType(String type, ...)`: one name over one focus type.
    fn execute_type_name_on(
        &mut self,
        ty: &str,
        id: NodeId,
        at_entry: bool,
        focus: &Types,
    ) -> CheckResult<Types> {
        let name = self.expr.node(id).name().unwrap_or("");
        if at_entry
            && name.chars().next().is_some_and(char::is_uppercase)
            && (hash_tail(ty) == name || self.is_ancestor(ty, name))
        {
            return Ok(Types::single(&[ty]));
        }
        let mut result = Types::new(focus.status());
        self.child_types_by_name(ty, name, &mut result, id)?;
        Ok(result)
    }

    /// `isAncestor`: `stated` is `wanted` or one of its bases.
    fn is_ancestor(&self, wanted: &str, stated: &str) -> bool {
        if is_system(wanted) || is_backbone(wanted) || !self.model.is_type(wanted) {
            return false;
        }
        let mut current = Some(wanted);
        let mut guard = 0usize;
        while let Some(t) = current {
            if t == stated {
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

    // -----------------------------------------------------------------
    // getChildTypesByName
    // -----------------------------------------------------------------

    /// The type and every specialisation of it (`addTypeAndDescendents`).
    fn type_and_descendants(&mut self, ty: &str, id: NodeId) -> CheckResult<Rc<Vec<String>>> {
        if let Some(v) = self.descendants_cache.get(ty) {
            return Ok(Rc::clone(v));
        }
        let mut out = Vec::new();
        let mut stack = vec![ty.to_string()];
        while let Some(t) = stack.pop() {
            let Some(mut specs) = self.model.specializations(&t) else {
                return Err(self.model_cannot(&format!("the specialisations of {t}"), id));
            };
            out.push(t);
            specs.reverse();
            stack.extend(specs);
        }
        let out = Rc::new(out);
        self.descendants_cache
            .insert(ty.to_string(), Rc::clone(&out));
        Ok(out)
    }

    fn resource_names(&mut self, id: NodeId) -> CheckResult<Rc<Vec<String>>> {
        if let Some(r) = &self.resource_names {
            return Ok(Rc::clone(r));
        }
        let Some(names) = self.model.resource_type_names() else {
            return Err(self.model_cannot("the resource types", id));
        };
        let names = Rc::new(names);
        self.resource_names = Some(Rc::clone(&names));
        Ok(names)
    }

    fn elements_of(&self, ty: &str, id: NodeId) -> CheckResult<Vec<ElementInfo>> {
        self.model
            .elements(ty)
            .ok_or_else(|| self.model_cannot(&format!("the elements of {ty}"), id))
    }

    /// `getChildTypesByName`.
    fn child_types_by_name(
        &mut self,
        ty: &str,
        name: &str,
        result: &mut Types,
        id: NodeId,
    ) -> CheckResult<()> {
        if ty.is_empty() {
            return Err(CheckError::failed(
                "FHIRPATH_NO_TYPE",
                "Error evaluating FHIRPath expression: No type provided at getChildTypesByName",
                self.start(id),
            ));
        }
        if ty == "xhtml" {
            return Ok(());
        }
        if ty == SYSTEM_SIMPLE_TYPE_INFO || ty == SYSTEM_CLASS_INFO {
            if name == "namespace" || name == "name" {
                result.add(SYSTEM_STRING);
            }
            return Ok(());
        }
        if is_system(ty) {
            return Ok(());
        }
        if ty.contains("://") && !self.model.is_type(ty) {
            // An extension's profile, as `extension(url)` types it: navigated
            // as `Extension`, with `value` narrowed to the definition's value
            // types when the model carries them.
            if name == "value" {
                if let Some(types) = self.model.extension_value_types(ty) {
                    if types.len() > 1 {
                        result.set_choice();
                    }
                    for t in &types {
                        result.add_with_status(CollectionStatus::Singleton, t);
                    }
                    return Ok(());
                }
            }
            return self.child_types_by_name("Extension", name, result, id);
        }
        let (owner, tail) = if is_backbone(ty) {
            match ty.find('.') {
                Some(dot) => (&ty[..dot], &ty[dot..]),
                None => (ty, ""),
            }
        } else {
            (ty, "")
        };
        if !self.model.is_type(owner) {
            let url = url_of(ty);
            let url = url.split('#').next().unwrap_or(&url).to_string();
            return Err(CheckError::failed(
                "FHIRPATH_UNKNOWN_TYPE",
                format!(
                    "Error evaluating FHIRPath expression: The type '{url}' is unknown or not \
                     supported at getChildTypesByName#1"
                ),
                self.start(id),
            ));
        }
        let sdl = self.type_and_descendants(owner, id)?;
        for sdi in sdl.iter() {
            let here = format!("{sdi}{tail}");
            match name {
                "**" => self.all_descendant_types(&here, result, id)?,
                "*" => {
                    for e in self.elements_of(&here, id)? {
                        for code in &e.codes {
                            if code == "Element" || code == "BackboneElement" {
                                result.add(&e.path);
                            } else if code == "Resource" {
                                for rn in self.resource_names(id)?.iter() {
                                    result.add(rn);
                                }
                            } else {
                                result.add(code);
                            }
                        }
                    }
                }
                _ => self.named_child_types(&here, name, result, id)?,
            }
        }
        Ok(())
    }

    /// The by-name branch of `getChildTypesByName`, over one definition.
    fn named_child_types(
        &mut self,
        here: &str,
        name: &str,
        result: &mut Types,
        id: NodeId,
    ) -> CheckResult<()> {
        let elements = self.elements_of(here, id)?;
        // `getElementDefinition` returns the first element in snapshot order
        // that the path names: exactly, by a choice element's base name, or
        // -- only with polymorphic names allowed -- by a typed choice name.
        let mut found: Option<(ElementInfo, Option<String>)> = None;
        for e in elements {
            if e.name == name {
                found = Some((e, None));
                break;
            }
            if self.allow_polymorphic_names
                && e.choice
                && name.len() > e.name.len()
                && name.starts_with(e.name.as_str())
            {
                let rest = &name[e.name.len()..];
                let lowered = uncapitalize(rest);
                let fixed = if self.model.is_primitive(&lowered) {
                    lowered
                } else {
                    rest.to_string()
                };
                found = Some((e, Some(fixed)));
                break;
            }
        }
        let Some((e, fixed)) = found else {
            return Ok(());
        };
        if e.choice {
            result.set_choice();
        }
        let status = if e.repeating {
            CollectionStatus::Ordered
        } else {
            CollectionStatus::Singleton
        };
        if let Some(fixed) = fixed {
            result.add(&fixed);
        } else if let Some(target) = &e.content_reference {
            result.add_with_status(status, target);
        } else {
            for t in &e.types {
                if t == "Resource" {
                    for rn in self.resource_names(id)?.iter() {
                        result.add(rn);
                    }
                } else {
                    result.add_with_status(status, t);
                }
            }
        }
        Ok(())
    }

    /// The `**` branch of `getChildTypesByName`, over one definition: every
    /// element at any depth below `here`, in snapshot order, each new type
    /// added and then walked in turn.
    fn all_descendant_types(
        &mut self,
        here: &str,
        result: &mut Types,
        id: NodeId,
    ) -> CheckResult<()> {
        let mut flat = Vec::new();
        self.flatten_elements(here, &mut flat, id)?;
        for e in flat {
            if let Some(target) = &e.content_reference {
                if !result.has_type_worker(self.model, target, true) {
                    result.add(target);
                    self.child_types_by_name(target, "**", result, id)?;
                }
                continue;
            }
            for code in &e.codes {
                if code == "Resource" {
                    for rn in self.resource_names(id)?.iter() {
                        if !result.has_type_worker(self.model, rn, true) {
                            result.add(rn);
                            self.child_types_by_name(rn, "**", result, id)?;
                        }
                    }
                    continue;
                }
                let tn = if code == "Element" || code == "BackboneElement" || code == "Base" {
                    e.path.clone()
                } else {
                    code.clone()
                };
                if !result.has_type_worker(self.model, &tn, true) {
                    result.add(&tn);
                    self.child_types_by_name(&tn, "**", result, id)?;
                }
            }
        }
        Ok(())
    }

    /// The elements below `here` at any depth, pre-order: snapshot order.
    fn flatten_elements(
        &self,
        here: &str,
        out: &mut Vec<ElementInfo>,
        id: NodeId,
    ) -> CheckResult<()> {
        for e in self.elements_of(here, id)? {
            let nested = e.content_reference.is_none()
                && e.codes
                    .iter()
                    .any(|c| c == "Element" || c == "BackboneElement");
            let path = e.path.clone();
            out.push(e);
            if nested {
                self.flatten_elements(&path, out, id)?;
            }
        }
        Ok(())
    }

    /// `anything(status)`: every type.
    fn anything(&self, status: Option<CollectionStatus>, id: NodeId) -> CheckResult<Types> {
        let Some(names) = self.model.type_names() else {
            return Err(self.model_cannot("every type ($total, aggregate())", id));
        };
        let mut t = Types::new(status);
        for n in &names {
            t.add(n);
        }
        Ok(t)
    }

    // -----------------------------------------------------------------
    // constants
    // -----------------------------------------------------------------

    /// `resolveConstantType(context, constant, ...)`, explicit mode.
    fn constant_type(&self, context: &TypeContext, id: NodeId) -> CheckResult<Types> {
        let node = self.expr.node(id);
        let start = node.start();
        let text = match node.constant() {
            None | Some(Constant::Empty) => return Ok(Types::empty()),
            Some(Constant::Value(v)) => {
                return Ok(Types::single(&[match v {
                    Value::Boolean(_) => SYSTEM_BOOLEAN,
                    Value::Integer(_) => SYSTEM_INTEGER,
                    Value::Decimal(_) => SYSTEM_DECIMAL,
                    Value::Quantity(_) => SYSTEM_QUANTITY,
                    // Everything else is typed as a string, as upstream's
                    // final `else` does.
                    _ => SYSTEM_STRING,
                }]));
            }
            Some(Constant::Deferred(text)) => text.as_str(),
        };
        if text.starts_with("@T") {
            return Ok(Types::single(&[SYSTEM_TIME]));
        }
        if text.starts_with('@') {
            return Ok(Types::single(&[SYSTEM_DATE_TIME]));
        }
        match text {
            "%sct" | "%loinc" | "%ucum" | "%map-codes" | "%us-zip" => {
                return Ok(Types::single(&[SYSTEM_STRING]));
            }
            "%resource" => {
                return match &context.resource {
                    Some(r) => Ok(Types::single(&[r.as_str()])),
                    None => Err(cannot_use("%resource", "no focus resource", start)),
                };
            }
            "%rootResource" => {
                return match &context.root_resource {
                    Some(r) => Ok(Types::single(&[r.as_str()])),
                    None => Err(cannot_use("%rootResource", "no focus rootResource", start)),
                };
            }
            "%context" => {
                return match &context.context {
                    Some(c) => Ok((**c).clone()),
                    None => Err(self.null_focus("%context", id)),
                };
            }
            _ => {}
        }
        if text.starts_with("%`vs-") || text.starts_with("%`cs-") || text.starts_with("%`ext-") {
            return Ok(Types::single(&[SYSTEM_STRING]));
        }
        let var = text.strip_prefix('%').unwrap_or(text);
        if let Some(t) = context.defined(var) {
            return Ok((**t).clone());
        }
        let unknown = || {
            CheckError::failed(
                "FHIRPATH_UNKNOWN_CONSTANT",
                format!("Error evaluating FHIRPath expression: Invalid FHIRPath Constant {text}"),
                start,
            )
        };
        let Some(host) = self.host else {
            return Err(unknown());
        };
        match host.resolve_constant_type(text, ConstantMode::Explicit) {
            Err(e) => Err(CheckError::failed("HOST_SERVICE", e.message, start)),
            Ok(None) => Err(unknown()),
            Ok(Some(t)) => Ok(Types::single(
                &t.names().iter().map(String::as_str).collect::<Vec<_>>(),
            )),
        }
    }

    // -----------------------------------------------------------------
    // operateTypes
    // -----------------------------------------------------------------

    fn cardinality_same(&mut self, left: &Types, right: &Types, id: NodeId) {
        if left.is_list() && !right.is_list() {
            let m = self.collection_message("left", id);
            self.warn(WarningId::CollectionStatusOperationLeft, m);
        } else if !left.is_list() && right.is_list() {
            let m = self.collection_message("right", id);
            self.warn(WarningId::CollectionStatusOperationRight, m);
        }
    }

    fn cardinality_single(&mut self, left: &Types, right: &Types, id: NodeId) {
        if left.is_list() {
            let m = self.collection_message("left", id);
            self.warn(WarningId::CollectionStatusOperationLeft, m);
        }
        if right.is_list() {
            let m = self.collection_message("right", id);
            self.warn(WarningId::CollectionStatusOperationRight, m);
        }
    }

    fn collection_message(&self, side: &str, id: NodeId) -> String {
        format!(
            "The {side} side of the operation {} is a collection where a single item is expected",
            self.text(id)
        )
    }

    /// `operateTypes`.
    fn operate_types(
        &mut self,
        left: &Types,
        op: Operation,
        right: &Types,
        id: NodeId,
    ) -> CheckResult<Types> {
        let model = self.model;
        let boolean = || Types::single(&[SYSTEM_BOOLEAN]);
        let int = |t: &Types| t.has_type_worker(model, "integer", true);
        let num = |t: &Types| t.has_any_worker(model, &["integer", "decimal"]);
        Ok(match op {
            Operation::Equals
            | Operation::Equivalent
            | Operation::NotEquals
            | Operation::NotEquivalent => {
                self.cardinality_same(left, right, id);
                boolean()
            }
            Operation::LessThan
            | Operation::Greater
            | Operation::LessOrEqual
            | Operation::GreaterOrEqual
            | Operation::Is => {
                self.cardinality_single(left, right, id);
                boolean()
            }
            Operation::As => {
                self.cardinality_single(left, right, id);
                right.names_with_status(Some(CollectionStatus::Singleton))
            }
            Operation::Union => left.union(right),
            Operation::Or
            | Operation::And
            | Operation::Xor
            | Operation::Implies
            | Operation::In
            | Operation::MemberOf
            | Operation::Contains => boolean(),
            Operation::Times | Operation::Div | Operation::Mod => {
                self.cardinality_single(left, right, id);
                let mut r = Types::empty();
                if int(left) && int(right) {
                    r.add(SYSTEM_INTEGER);
                } else if num(left) && num(right) {
                    r.add(SYSTEM_DECIMAL);
                }
                r
            }
            Operation::DivideBy => {
                self.cardinality_single(left, right, id);
                let mut r = Types::empty();
                if num(left) && num(right) {
                    r.add(SYSTEM_DECIMAL);
                }
                r
            }
            Operation::Concatenate => {
                self.cardinality_single(left, right, id);
                Types::single(&[SYSTEM_STRING])
            }
            Operation::Plus | Operation::Minus => {
                self.cardinality_single(left, right, id);
                let mut r = Types::empty();
                let stringy = ["string", "id", "code", "uri"];
                let dated = ["date", "dateTime", "instant"];
                let quantity = |t: &Types| t.has_type_worker(model, "Quantity", true);
                if int(left) && int(right) {
                    r.add(SYSTEM_INTEGER);
                } else if num(left) && num(right) {
                    r.add(SYSTEM_DECIMAL);
                } else if op == Operation::Plus
                    && left.has_any_worker(model, &stringy)
                    && right.has_any_worker(model, &stringy)
                {
                    r.add(SYSTEM_STRING);
                } else if op == Operation::Minus && quantity(left) && quantity(right) {
                    r.add(SYSTEM_QUANTITY);
                } else if left.has_any_worker(model, &dated) {
                    if quantity(right) {
                        if let Some(first) = left.first() {
                            r.add(first);
                        }
                    } else {
                        let (id_, verb) = if op == Operation::Plus {
                            ("FHIRPATH_ARITHMETIC_PLUS", "add")
                        } else {
                            ("FHIRPATH_ARITHMETIC_MINUS", "subtract")
                        };
                        return Err(CheckError::failed(
                            id_,
                            format!(
                                "Error in date arithmetic: Unable to {verb} type {} to {}",
                                right.first().map(url_of).unwrap_or_else(|| "null".into()),
                                left.first().map(url_of).unwrap_or_else(|| "null".into()),
                            ),
                            self.expr.node(id).op_start(),
                        ));
                    }
                }
                r
            }
        })
    }

    // -----------------------------------------------------------------
    // evaluateFunctionType
    // -----------------------------------------------------------------

    /// `isExpressionParameter`.
    fn is_expression_parameter(function: Function, i: usize) -> bool {
        if function == Function::Sort {
            return true;
        }
        match i {
            0 => matches!(
                function,
                Function::Where
                    | Function::Exists
                    | Function::All
                    | Function::Select
                    | Function::Repeat
                    | Function::Aggregate
            ),
            1 => matches!(function, Function::Trace | Function::DefineVariable),
            _ => false,
        }
    }

    /// `evaluateParameters`.
    fn evaluate_parameters(
        &mut self,
        context: &TypeContext,
        focus: Option<&Types>,
        id: NodeId,
        function: Function,
        out: &mut Vec<Types>,
        can_be_none: bool,
    ) -> CheckResult<()> {
        let params = self.expr.node(id).parameters().to_vec();
        for (i, p) in params.into_iter().enumerate() {
            let t = if Self::is_expression_parameter(function, i) {
                let ctx = context.change_this(focus);
                self.execute(&ctx, focus, p, true, can_be_none, p)?
            } else {
                let this = context.this_item.clone();
                self.execute(context, this.as_deref(), p, true, can_be_none, p)?
            };
            out.push(t);
        }
        Ok(())
    }

    fn evaluate_function_type(
        &mut self,
        context: &mut TypeContext,
        focus: Option<&Types>,
        id: NodeId,
        container: NodeId,
    ) -> CheckResult<Types> {
        let node = self.expr.node(id);
        let Some(function) = node.function() else {
            return Err(CheckError::not_implemented(
                "a function node with no function",
                node.start(),
            ));
        };
        let Some(sig) = signature::signature(function) else {
            return Err(CheckError::not_implemented(
                format!(
                    "the function {}()",
                    node.name().unwrap_or(function.to_code())
                ),
                node.start(),
            ));
        };
        if focus.is_none() && sig.reads_focus() {
            return Err(self.null_focus(&format!("{}()", function.to_code()), id));
        }

        let mut params: Vec<Types> = Vec::new();
        match sig.params {
            Params::TypeName => params.push(Types::single(&[SYSTEM_STRING])),
            Params::Repeat => params.push(self.repeat_type(context, focus, id, function)?),
            Params::SingletonFocus => {
                let single = focus.map(|f| f.with_status(CollectionStatus::Singleton));
                self.evaluate_parameters(
                    context,
                    single.as_ref(),
                    id,
                    function,
                    &mut params,
                    false,
                )?;
            }
            Params::Standard => {
                self.evaluate_parameters(context, focus, id, function, &mut params, false)?;
            }
        }

        if sig.warns_not_a_collection
            && focus.is_some_and(|f| f.status() == Some(CollectionStatus::Singleton))
        {
            let m = format!(
                "The expression {} is not a collection, so the function is unnecessary",
                self.text(container)
            );
            self.warn(WarningId::NotACollection, m);
        }

        for rule in sig.focus {
            // `reads_focus` guarantees a focus whenever there is a rule.
            let Some(f) = focus else { continue };
            if let Focus::Singleton(name) = rule {
                // `checkSingleton` warns and never fails.
                if f.status() != Some(CollectionStatus::Singleton) {
                    let m = format!(
                        "The function {name} expects a single item, but {} may be a collection",
                        self.text(id)
                    );
                    self.warn(WarningId::CollectionStatusContext, m);
                }
            } else {
                self.check_focus(*rule, f, id)?;
            }
        }
        self.check_param_types(id, function.to_code(), &params, &sig, focus)?;
        self.yields(&sig, context, focus, &params, id, container)
    }

    /// `repeat()`'s loop: the projection typed against the focus, then
    /// against what it yielded, until the yield stops growing.
    fn repeat_type(
        &mut self,
        context: &TypeContext,
        focus: Option<&Types>,
        id: NodeId,
        function: Function,
    ) -> CheckResult<Types> {
        let mut base = Types::empty();
        let mut l_focus = focus.cloned();
        let mut params: Vec<Types> = Vec::new();
        for _ in 0..REPEAT_ROUNDS {
            self.evaluate_parameters(context, l_focus.as_ref(), id, function, &mut params, true)?;
            // Upstream reads `paramTypes.get(0)` every round while appending
            // to the list, so only the first round's type is ever compared.
            let Some(first) = params.first() else {
                return Ok(base);
            };
            if base.contains_all(first) {
                return Ok(base);
            }
            base.add_types(first);
            l_focus = Some(base.clone());
        }
        Err(CheckError::not_implemented(
            "repeat() over a projection with no collection status, on which the pinned engine \
             does not terminate",
            self.start(id),
        ))
    }

    fn check_focus(&self, rule: Focus, focus: &Types, id: NodeId) -> CheckResult<()> {
        let model = self.model;
        let start = self.start(id);
        let fail = |constant: &'static str, message: String| {
            Err(CheckError::failed(constant, message, start))
        };
        let prefix = "Error evaluating FHIRPath expression: The function";
        match rule {
            Focus::Ordered(name) => {
                if focus.status() == Some(CollectionStatus::Unordered) {
                    return fail(
                        "FHIRPATH_ORDERED_ONLY",
                        format!("{prefix} {name} can only be used on ordered collections"),
                    );
                }
            }
            Focus::Singleton(_) => {}
            Focus::String(name) => {
                let ok = focus.has_no_types()
                    || focus.has_any_worker(
                        model,
                        &["string", "code", "uri", "url", "canonical", "id"],
                    );
                if !ok {
                    return fail(
                        "FHIRPATH_STRING_SING_ONLY",
                        format!(
                            "{prefix} {name} can only be used on string, uri, url, code, id but \
                             found {}",
                            focus.describe()
                        ),
                    );
                }
            }
            Focus::Primitive(name, quantity) => {
                let ok = focus.has_no_types()
                    || focus.has_primitive_type(model)
                    || (quantity && focus.has_type_plain("Quantity"));
                if !ok {
                    return fail(
                        "FHIRPATH_PRIMITIVE_ONLY",
                        format!("{prefix} {name} can only be used on primitives"),
                    );
                }
            }
            Focus::Numerical(name) => {
                let ok = focus.has_no_types()
                    || ["integer", "decimal", "Quantity"]
                        .iter()
                        .any(|t| focus.has_type_plain(t));
                if !ok {
                    return fail(
                        "FHIRPATH_NUMERICAL_ONLY",
                        format!(
                            "{prefix} {name} can only be used on integer, decimal or Quantity but \
                             found {}",
                            focus.describe()
                        ),
                    );
                }
            }
            Focus::Decimal(name) => {
                let ok = focus.has_no_types()
                    || focus.has_type_plain("decimal")
                    || focus.has_type_plain("integer");
                if !ok {
                    return fail(
                        "FHIRPATH_DECIMAL_ONLY",
                        format!(
                            "{prefix} {name} can only be used on a decimal but found {}",
                            focus.describe()
                        ),
                    );
                }
            }
            Focus::Continuous(name, integer) => {
                let ok = focus.has_no_types()
                    || ["decimal", "date", "dateTime", "time", "Quantity"]
                        .iter()
                        .any(|t| focus.has_type_plain(t))
                    || (integer && focus.has_type_plain("integer"));
                if !ok {
                    return fail(
                        "FHIRPATH_CONTINUOUS_ONLY",
                        format!(
                            "{prefix} {name} can only be used on a decimal or date type but found \
                             {}",
                            focus.describe()
                        ),
                    );
                }
            }
            Focus::Reference(name) => {
                if !focus.has_any_worker(model, &["string", "uri", "url", "Reference", "canonical"])
                {
                    return fail(
                        "FHIRPATH_REFERENCE_ONLY",
                        format!(
                            "{prefix} {name} can only be used on ordered string, uri, canonical or \
                             Reference but found {}",
                            focus.describe()
                        ),
                    );
                }
            }
            Focus::Coded(name) => {
                if !focus.has_any_worker(
                    model,
                    &["string", "code", "uri", "Coding", "CodeableConcept"],
                ) {
                    return fail(
                        "FHIRPATH_CODED_ONLY",
                        format!(
                            "{prefix} {name} can only be used on ordered string, code, uri, \
                             Coding, CodeableConcept but found {}",
                            focus.describe()
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    /// `checkParamTypes`.
    fn check_param_types(
        &mut self,
        id: NodeId,
        func_name: &str,
        params: &[Types],
        sig: &Signature,
        focus: Option<&Types>,
    ) -> CheckResult<()> {
        let model = self.model;
        for (i, rule) in sig.param_types.iter().enumerate() {
            let Some(actual) = params.get(i) else {
                return Ok(());
            };
            let expected = match rule {
                Param::Types(status, names) => Types::of(*status, names),
                Param::FocusTypes(status) => match focus {
                    Some(f) => match status {
                        Some(s) => f.with_status(*s),
                        None => f.clone(),
                    },
                    None => return Err(self.null_focus(func_name, id)),
                },
            };
            for a in actual.names() {
                if !expected.has_type_worker(model, a, false) {
                    return Err(CheckError::failed(
                        "FHIRPATH_WRONG_PARAM_TYPE",
                        format!(
                            "Error evaluating FHIRPath expression: The parameter type {} is not \
                             legal for {func_name} parameter {}. expecting {}",
                            url_of(a),
                            i + 1,
                            render_details(&expected)
                        ),
                        self.start(id),
                    ));
                }
            }
            if actual.status() != Some(CollectionStatus::Singleton)
                && expected.status() == Some(CollectionStatus::Singleton)
            {
                let m = format!(
                    "The parameter {} of {func_name} in {} is a collection where a single item \
                     is expected",
                    i + 1,
                    self.text(id)
                );
                self.warn(WarningId::CollectionStatusParameter, m);
            }
        }
        Ok(())
    }

    fn yields(
        &mut self,
        sig: &Signature,
        context: &mut TypeContext,
        focus: Option<&Types>,
        params: &[Types],
        id: NodeId,
        container: NodeId,
    ) -> CheckResult<Types> {
        let param = |i: usize| -> CheckResult<Types> {
            params.get(i).cloned().ok_or_else(|| {
                CheckError::not_implemented("a function with a missing parameter", 0)
            })
        };
        let focus_or = |what: &str| focus.cloned().ok_or_else(|| self.null_focus(what, id));
        Ok(match sig.yields {
            Yields::Types(status, names) => Types::of(status, names),
            Yields::Focus => focus_or("the function")?,
            Yields::FocusAs(status) => focus_or("the function")?.with_status(status),
            Yields::TypesWithFocusStatus(names) => {
                let f = focus_or("the function")?;
                let mut t = Types::new(f.status());
                for n in names {
                    t.add(n);
                }
                t
            }
            Yields::Param(i) => param(i)?,
            Yields::UnionWithParam => focus_or("the function")?.union(&param(0)?),
            Yields::IntersectWithParam => focus_or("the function")?.intersect(&param(0)?),
            Yields::Special(special) => {
                return self.special(special, context, focus, params, id, container);
            }
        })
    }

    fn special(
        &mut self,
        special: Special,
        context: &mut TypeContext,
        focus: Option<&Types>,
        params: &[Types],
        id: NodeId,
        _container: NodeId,
    ) -> CheckResult<Types> {
        let Some(focus) = focus else {
            return Err(self.null_focus("the function", id));
        };
        match special {
            Special::TypeNamed => self.check_type(focus, id),
            Special::TypeInfo => {
                let (s, c) = focus.system_and_class();
                Ok(if s && c {
                    Types::single(&[SYSTEM_SIMPLE_TYPE_INFO, SYSTEM_CLASS_INFO])
                } else if s {
                    Types::single(&[SYSTEM_SIMPLE_TYPE_INFO])
                } else {
                    Types::single(&[SYSTEM_CLASS_INFO])
                })
            }
            Special::Iif => {
                let mut t = Types::new(None);
                if let Some(p) = params.get(1) {
                    t.update(p);
                }
                if let Some(p) = params.get(2) {
                    t.update(p);
                }
                Ok(t)
            }
            Special::Anything => self.anything(focus.status(), id),
            Special::Children | Special::Descendants => {
                let mask = if special == Special::Children {
                    "*"
                } else {
                    "**"
                };
                let mut result = Types::new(Some(CollectionStatus::Unordered));
                for f in focus.names().to_vec() {
                    self.child_types_by_name(&f, mask, &mut result, id)?;
                }
                Ok(result)
            }
            Special::DefineVariable => {
                let p0 = self.expr.node(id).parameters().first().copied();
                if let Some(p0) = p0 {
                    let pn = self.expr.node(p0);
                    if pn.kind() == Kind::Constant {
                        let name = match pn.constant() {
                            Some(Constant::Value(v)) => primitive_value(v),
                            Some(Constant::Deferred(text)) => Some(text.clone()),
                            None | Some(Constant::Empty) => None,
                        };
                        if let Some(name) = name {
                            let value = params.get(1).cloned().unwrap_or_else(|| focus.clone());
                            context.define(&name, value, pn.start())?;
                        }
                    }
                }
                Ok(focus.clone())
            }
            Special::Extension => self.extension_type(id, focus),
            Special::Boundary => Ok(
                if focus.has_type_plain("date")
                    || focus.has_type_plain("datetime")
                    || focus.has_type_plain("instant")
                {
                    // Upstream tests `datetime`, lower case, which no type
                    // set ever holds; ported as written.
                    Types::single(&[SYSTEM_DECIMAL, SYSTEM_DATE_TIME])
                } else if focus.has_type_plain("time") {
                    Types::single(&[SYSTEM_TIME])
                } else if focus.has_type_plain("decimal") || focus.has_type_plain("integer") {
                    Types::single(&[SYSTEM_DECIMAL])
                } else {
                    Types::single(&[SYSTEM_DATE_TIME])
                },
            ),
        }
    }

    /// `checkType`: the type `as()` and `ofType()` name, from the parameter
    /// as written.
    fn check_type(&mut self, focus: &Types, id: NodeId) -> CheckResult<Types> {
        let Some(&p0) = self.expr.node(id).parameters().first() else {
            return Err(CheckError::not_implemented(
                "a type function with no parameter",
                self.start(id),
            ));
        };
        let pn = self.expr.node(p0);
        let head = pn.name().unwrap_or("null");
        let written = match pn.inner() {
            Some(inner) => format!("{head}.{}", self.expr.node(inner).name().unwrap_or("null")),
            None => format!("FHIR.{head}"),
        };
        let tn = if let Some(rest) = written.strip_prefix("System.") {
            // Upstream strips `System.` and leaves a bare name, which its
            // `TypeDetails` then puts in the FHIR namespace: `String`, not
            // `System.String`.
            rest.to_string()
        } else if let Some(rest) = written.strip_prefix("FHIR.") {
            rest.to_string()
        } else if let Some(rest) = written.strip_prefix("CDA.") {
            format!("http://hl7.org/cda/stds/core/StructureDefinition/{rest}")
        } else {
            written
        };
        if self.type_cast_is_impossible(focus, &tn) {
            let m = format!(
                "The type {} can never be a {} (in {})",
                focus.describe(),
                url_of(&tn),
                self.text(id)
            );
            self.warn(WarningId::OfTypeImpossible, m);
        }
        Ok(Types::single(&[tn.as_str()]))
    }

    /// `typeCastIsImpossible`, for profile-free types: `tn` or one of its
    /// bases is in the focus, or the focus holds `tn` itself.
    fn type_cast_is_impossible(&self, focus: &Types, tn: &str) -> bool {
        for pt in focus.names() {
            let mut purl = Some(tn);
            let mut guard = 0usize;
            while let Some(p) = purl {
                if p == pt {
                    return false;
                }
                guard += 1;
                if guard > 64 {
                    break;
                }
                purl = if is_system(p) || is_backbone(p) {
                    None
                } else {
                    self.model.base_type(p)
                };
            }
        }
        !focus.contains(tn)
    }

    /// `extension(url)` with a literal URL (`:3556`-`:3591`). It never fails:
    /// a URL that names no definition warns and is typed a plain `Extension`;
    /// a known absolute URL is typed as the extension's profile, which
    /// `child_types_by_name` navigates as `Extension` with `value` narrowed to
    /// the definition's value types. Type profiles on the focus are not
    /// carried by this pass, so a relative URL finds none and warns.
    fn extension_type(&mut self, id: NodeId, _focus: &Types) -> CheckResult<Types> {
        let Some(&p0) = self.expr.node(id).parameters().first() else {
            return Err(CheckError::not_implemented(
                "extension() with no parameter",
                self.start(id),
            ));
        };
        let pn = self.expr.node(p0);
        let url = match (pn.kind(), pn.constant()) {
            (Kind::Constant, Some(Constant::Value(v))) => primitive_value(v),
            (Kind::Constant, Some(Constant::Deferred(text))) => Some(text.clone()),
            _ => None,
        };
        let Some(url) = url else {
            // Upstream's `case Extension` has no `return` on this path and
            // falls through into `case AnyTrue`.
            return Ok(Types::single(&[SYSTEM_BOOLEAN]));
        };
        let known = if is_absolute_url(&url) {
            match self.model.has_definition_url(&url) {
                None => return Err(self.model_cannot(&format!("whether {url} is defined"), id)),
                Some(known) => known,
            }
        } else {
            // Upstream reads the focus's `Extension` type profiles, or the
            // fixed values in the focus type's definition; this pass carries
            // neither, and a base definition has no such fixed value.
            false
        };
        if known {
            return Ok(Types::of(CollectionStatus::Ordered, &[url.as_str()]));
        }
        let m = format!(
            "Reference to an unknown extension - double check that the URL '{url}' is correct"
        );
        self.warn(WarningId::UnknownExtension, m);
        Ok(Types::single(&["Extension"]))
    }
}

/// `Utilities.isAbsoluteUrl`.
fn is_absolute_url(s: &str) -> bool {
    let Some((scheme, details)) = s.split_once(':') else {
        return false;
    };
    let token = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '[' | ']'))
        && scheme == scheme.to_lowercase();
    (matches!(scheme, "http" | "https" | "urn" | "file:") || token)
        && !details.is_empty()
        && !details.contains(' ')
}

/// `hashTail`: a URL's last segment, and nothing for a backbone.
fn hash_tail(ty: &str) -> &str {
    if is_backbone(ty) { "" } else { ty }
}

/// `Utilities.uncapitalize`.
fn uncapitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `Base.primitiveValue()` of a literal.
fn primitive_value(v: &Value) -> Option<String> {
    Some(match v {
        Value::String(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Decimal(d) => d.lexical().to_string(),
        Value::Date(d) => d.lexical.clone(),
        Value::DateTime(d) => d.lexical.clone(),
        Value::Time(t) => t.lexical.clone(),
        Value::Quantity(_) | Value::Node(_) => return None,
    })
}

fn cannot_use(what: &str, why: &str, offset: usize) -> CheckError {
    CheckError::failed(
        "FHIRPATH_CANNOT_USE",
        format!(
            "Error evaluating FHIRPath expression: Cannot use {what} in this context because {why}"
        ),
        offset,
    )
}

/// `TypeDetails.toString()`: the status, then the type URLs.
fn render_details(t: &Types) -> String {
    let status = match t.status() {
        None | Some(CollectionStatus::Singleton) => "SINGLETON",
        Some(CollectionStatus::Ordered) => "ORDERED",
        Some(CollectionStatus::Unordered) => "UNORDERED",
    };
    let urls: Vec<String> = t.names().iter().map(|n| url_of(n)).collect();
    format!("{status}[{}]", urls.join(", "))
}
