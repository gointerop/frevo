//! Filtering, projection and subsetting: `where`, `select`, `repeat`,
//! `ofType`, `single`, `first`, `last`, `tail`, `skip`, `take`, `intersect`
//! and `exclude`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `funcWhere` at `:6249`,
//! `funcSelect` at `:6263`, `funcRepeat` at `:5261`, `funcOfType` at `:5198`,
//! `isKnownType` at `:1904`, `funcSingle` at `:5087`, `funcFirst` at `:6240`,
//! `funcLast` at `:6232`, `funcTail` at `:6112`, `funcSkip` at `:6101`,
//! `funcTake` at `:5024`, `funcIntersect` at `:5062` and `funcExclude` at
//! `:5074`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: type questions go through the `Model` trait
//! instead of `IWorkerContext.fetchTypeDefinition`.
//!
//! What is the pinned engine's, kept:
//!
//! - `single()` fails for a focus of size **0** as well as for one larger than
//!   1 (`DEV-0001`);
//! - `where` keeps an item only when its criteria is `True`; a single
//!   non-boolean criteria result counts as `True` (`asBool`);
//! - `repeat` evaluates its projection with `atEntry` false, and treats an
//!   item as new when it is not `equalsDeep` to anything already collected;
//! - `intersect` evaluates its argument against `$this`, not against the
//!   focus, and de-duplicates its result; `exclude` evaluates its argument
//!   against the focus and does not;
//! - `ofType` names a type that is not known as an execution error, and
//!   compares a primitive's own type name, never its base.
//!
//! `skip` and `take` are ported as written: a negative `skip` count and an
//! empty count for either raise, and a negative `take` count is empty.

use crate::seam::value::Value;
use crate::syntax::ast::NodeId;

use super::equality::{compare_deep, do_contains, primitive_text};
use super::navigate::fhir_type;
use super::{Context, EvalError, Run};

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

impl Run<'_> {
    /// `funcWhere`: keep the items whose criteria is `True`. Every item is
    /// tried, so a failure on a later item still surfaces.
    pub(super) fn func_where(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let p = self.param(id, 0)?;
        let mut result = Vec::new();
        for item in focus {
            let v = self.as_bool(&self.criteria(context, item, p)?, id)?;
            if v == super::collection::Tri::True {
                result.push(item.clone());
            }
        }
        Ok(result)
    }

    /// `funcSelect`: the projection of every item, flattened, with `$index`
    /// the item's position.
    pub(super) fn func_select(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let p = self.param(id, 0)?;
        let mut result = Vec::new();
        for (i, item) in focus.iter().enumerate() {
            let item_context = Context {
                index: i,
                ..context.change_this(item)
            };
            result.extend(self.execute(item_context, std::slice::from_ref(item), p, true)?);
        }
        Ok(result)
    }

    /// `funcRepeat`: apply the projection to what the last pass added until a
    /// pass adds nothing new. "New" is `equalsDeep` against everything
    /// collected so far, which is what makes a cyclic projection terminate.
    pub(super) fn func_repeat(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let p = self.param(id, 0)?;
        let offset = self.expr.node(id).start();
        let mut result: Vec<Value> = Vec::new();
        let mut current: Vec<Value> = focus.to_vec();
        loop {
            let mut added = Vec::new();
            for item in &current {
                added.extend(self.execute(
                    context.change_this(item),
                    std::slice::from_ref(item),
                    p,
                    false,
                )?);
            }
            current.clear();
            for b in added {
                let mut is_new = true;
                for t in &result {
                    if compare_deep(self.model, &b, t, offset)? {
                        is_new = false;
                    }
                }
                if is_new {
                    result.push(b.clone());
                    current.push(b);
                }
            }
            if current.is_empty() {
                return Ok(result);
            }
        }
    }

    /// `isKnownType(tn)` (`FHIRPathEngine.java:1904`) for a name that always
    /// contains a `.` -- `funcOfType` builds it as `namespace.name`.
    fn is_known_type(&self, tn: &str) -> bool {
        let parts: Vec<&str> = tn.split('.').collect();
        let [namespace, name] = parts.as_slice() else {
            return false;
        };
        match *namespace {
            "System" => SYSTEM_TYPES.contains(name),
            "FHIR" => self.model.is_type(name),
            "CDA" => self.model.is_type(&cda_url(name)),
            _ => false,
        }
    }

    /// `funcOfType`.
    pub(super) fn func_of_type(
        &self,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let pn = self.expr.node(self.param(id, 0)?);
        let head = pn.name().unwrap_or("");
        let tn = match pn.inner() {
            Some(inner) => format!("{head}.{}", self.expr.node(inner).name().unwrap_or("")),
            None => format!("FHIR.{head}"),
        };
        if !self.is_known_type(&tn) {
            return Err(EvalError::execution(
                format!("The type {tn} is not valid"),
                node.start(),
            ));
        }
        let mut result = Vec::new();
        for b in focus {
            let matched = if let Some(name) = tn.strip_prefix("System.") {
                // Only a value the engine itself produced -- a literal, which
                // upstream marks `noExtensions()` -- is a System value, and
                // `hasType` ignores case.
                !matches!(b, Value::Node(_)) && fhir_type(b).eq_ignore_ascii_case(name)
            } else if let Some(name) = tn.strip_prefix("FHIR.") {
                self.has_type_through_bases(b, name)
            } else if let Some(name) = tn.strip_prefix("CDA.") {
                self.has_type_through_bases(b, &cda_url(name))
            } else {
                false
            };
            if matched {
                result.push(b.clone());
            }
        }
        Ok(result)
    }

    /// The `FHIR.` and `CDA.` arms of `funcOfType`: the item's own type name
    /// equals `wanted`, or one of its base definitions' type does. A primitive
    /// has no base walk (`getKind() == PRIMITIVETYPE ? null`), so a `code` is
    /// not a `string` here.
    fn has_type_through_bases(&self, value: &Value, wanted: &str) -> bool {
        let own = fhir_type(value);
        if own == wanted {
            return true;
        }
        if self.model.is_primitive(own) {
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

    /// `funcSingle`: the focus itself when it has exactly one item.
    pub(super) fn func_single(&self, focus: &[Value], id: NodeId) -> Result<Vec<Value>, EvalError> {
        if focus.len() == 1 {
            return Ok(focus.to_vec());
        }
        Err(EvalError::execution(
            format!(
                "Error evaluating FHIRPath expression: The function single can only be used on a singleton value but found {}",
                focus.len()
            ),
            self.expr.node(id).start(),
        ))
    }

    /// The count argument of `skip` and `take`:
    /// `Integer.parseInt(n1.get(0).primitiveValue())`. An empty argument is
    /// `n1.get(0)` on an empty list, which raises.
    fn count_argument(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<i32, EvalError> {
        let p = self.param(id, 0)?;
        let n1 = self.execute(context, focus, p, true)?;
        let Some(first) = n1.first() else {
            return Err(EvalError::execution(
                "Index 0 out of bounds for length 0",
                self.expr.node(id).start(),
            ));
        };
        let text = primitive_text(self.model, first).unwrap_or_default();
        text.parse::<i32>().map_err(|_| {
            EvalError::execution(
                format!("For input string: \"{text}\""),
                self.expr.node(p).start(),
            )
        })
    }

    /// `funcSkip`. The loop starts at `i1`, so a negative count reaches
    /// `focus.get(-1)` and raises, even over an empty focus.
    pub(super) fn func_skip(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let n = self.count_argument(context, focus, id)?;
        let Ok(from) = usize::try_from(n) else {
            return Err(EvalError::execution(
                format!("Index {n} out of bounds for length {}", focus.len()),
                self.expr.node(id).start(),
            ));
        };
        Ok(focus.iter().skip(from).cloned().collect())
    }

    /// `funcTake`. `Math.min(focus.size(), i1)` bounds the loop, so a count of
    /// zero or less takes nothing.
    pub(super) fn func_take(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let n = self.count_argument(context, focus, id)?;
        let count = usize::try_from(n).unwrap_or(0);
        Ok(focus.iter().take(count).cloned().collect())
    }

    /// `funcIntersect`: items of the focus that are in the argument, once
    /// each. The argument is evaluated against `$this`.
    pub(super) fn func_intersect(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let this: Vec<Value> = context.this.cloned().into_iter().collect();
        let other = self.execute(context, &this, self.param(id, 0)?, true)?;
        let mut result: Vec<Value> = Vec::new();
        for item in focus {
            if !do_contains(self.model, &result, item, offset)?
                && do_contains(self.model, &other, item, offset)?
            {
                result.push(item.clone());
            }
        }
        Ok(result)
    }

    /// `funcExclude`: items of the focus that are not in the argument,
    /// duplicates kept. The argument is evaluated against the focus.
    pub(super) fn func_exclude(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let other = self.execute(context, focus, self.param(id, 0)?, true)?;
        let mut result = Vec::new();
        for item in focus {
            if !do_contains(self.model, &other, item, offset)? {
                result.push(item.clone());
            }
        }
        Ok(result)
    }
}

/// `Utilities.pathURL(Constants.NS_CDA_ROOT, "StructureDefinition", name)`.
fn cda_url(name: &str) -> String {
    format!("http://hl7.org/cda/stds/core/StructureDefinition/{name}")
}
