//! Combining and membership: the union operator, `union()`, `combine()`, `in`
//! and `contains`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `opUnion` at `:2831`,
//! `funcUnion` at `:5036`, `funcCombine` at `:5051`, `opIn` at `:2630` and
//! `opContains` at `:2655`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: results are built with the shared
//! `do_contains` over the `Model` trait.
//!
//! The empty-operand rules differ between the two membership operators, and
//! both are the pinned engine's:
//!
//! | | left empty | right empty |
//! |---|---|---|
//! | `in` | empty | **`false`** |
//! | `contains` | empty | empty |
//!
//! `union()` and `combine()` evaluate their argument against `$this`, not
//! against the focus, so `a.combine(b)` reads `b` from the item `$this` is
//! bound to -- the same item at the top of an expression, and the item being
//! tested inside `where`.

use crate::seam::value::Value;
use crate::syntax::ast::NodeId;

use super::equality::{do_contains, do_equals};
use super::{Context, EvalError, Run};

impl Run<'_> {
    /// `opUnion`: set union, in order of first appearance, de-duplicated with
    /// `doEquals` -- including within one operand.
    pub(super) fn op_union(
        &self,
        left: &[Value],
        right: &[Value],
        offset: usize,
    ) -> Result<Vec<Value>, EvalError> {
        let mut result: Vec<Value> = Vec::new();
        for item in left.iter().chain(right) {
            if !do_contains(self.model, &result, item, offset)? {
                result.push(item.clone());
            }
        }
        Ok(result)
    }

    /// `funcUnion`: [`Self::op_union`] with the argument evaluated against
    /// `$this`.
    pub(super) fn func_union(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let this: Vec<Value> = context.this.cloned().into_iter().collect();
        let other = self.execute(context, &this, self.param(id, 0)?, true)?;
        self.op_union(focus, &other, self.expr.node(id).start())
    }

    /// `funcCombine`: concatenation, duplicates kept.
    pub(super) fn func_combine(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let this: Vec<Value> = context.this.cloned().into_iter().collect();
        let other = self.execute(context, &this, self.param(id, 0)?, true)?;
        let mut result = focus.to_vec();
        result.extend(other);
        Ok(result)
    }

    /// `opIn`: every left item is `doEquals`-equal to some right item.
    pub(super) fn op_in(
        &self,
        left: &[Value],
        right: &[Value],
        offset: usize,
    ) -> Result<Vec<Value>, EvalError> {
        if left.is_empty() {
            return Ok(Vec::new());
        }
        if right.is_empty() {
            return Ok(vec![Value::Boolean(false)]);
        }
        Ok(vec![Value::Boolean(self.all_members(left, right, offset)?)])
    }

    /// `opContains`: every right item is `doEquals`-equal to some left item.
    pub(super) fn op_contains(
        &self,
        left: &[Value],
        right: &[Value],
        offset: usize,
    ) -> Result<Vec<Value>, EvalError> {
        if left.is_empty() || right.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![Value::Boolean(self.all_members(right, left, offset)?)])
    }

    /// Whether every item of `items` equals some item of `within`; a pair
    /// `doEquals` cannot compare is not a match.
    fn all_members(
        &self,
        items: &[Value],
        within: &[Value],
        offset: usize,
    ) -> Result<bool, EvalError> {
        for item in items {
            let mut found = false;
            for candidate in within {
                if do_equals(self.model, item, candidate, offset)? == Some(true) {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
