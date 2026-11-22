//! Boolean logic: `and`, `or`, `xor`, `implies`, their short-circuits and
//! `iif`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `preOperate` at `:1542`,
//! `opAnd` at `:2866`, `isBoolean` at `:2889`, `opOr` at `:2893`, `opXor` at
//! `:2907`, `opImplies` at `:2926`, `funcIif` at `:5007`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the three-valued answer is [`Tri`], and an
//! operand is reduced to it with [`Run::as_bool`].
//!
//! Three facts that are the pinned engine's:
//!
//! - an operand that is a single item that is **not** a boolean counts as
//!   `true`, so `true and 'foo'` is `true`;
//! - `and`, `or` and `implies` are short-circuited by `preOperate` **before**
//!   the right operand is evaluated, but only when the left is a boolean
//!   *value* (`instanceof BooleanType`), not a boolean element of a resource;
//! - `xor` is never short-circuited.

use crate::seam::value::Value;
use crate::syntax::ast::{NodeId, Operation};

use super::collection::Tri;
use super::error::At;
use super::{Context, EvalError, Run};

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Boolean(b)]
}

/// `isBoolean(list, b)`: exactly one item, a boolean value equal to `b`.
fn is_boolean(list: &[Value], b: bool) -> bool {
    matches!(list, [Value::Boolean(v)] if *v == b)
}

impl Run<'_> {
    /// `preOperate(left, operation, expr)`: the answer when the left operand
    /// alone decides the operation, so the right is never evaluated.
    pub(super) fn pre_operate(
        &self,
        left: &[Value],
        op: Operation,
        at: NodeId,
    ) -> Result<Option<Vec<Value>>, EvalError> {
        if left.is_empty() {
            return Ok(None);
        }
        Ok(match op {
            Operation::And if is_boolean(left, false) => Some(boolean(false)),
            Operation::Or if is_boolean(left, true) => Some(boolean(true)),
            Operation::Implies if self.as_bool(left, at)? == Tri::False => Some(boolean(true)),
            _ => None,
        })
    }

    /// `opAnd`, `opOr`, `opXor` and `opImplies`.
    pub(super) fn op_logic(
        &self,
        left: &[Value],
        op: Operation,
        right: &[Value],
        at: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let l = self.as_bool(left, at)?;
        if op == Operation::Implies {
            return self.implies(l, right, at);
        }
        let r = self.as_bool(right, at)?;
        Ok(match (op, l, r) {
            (Operation::And, Tri::False, _) | (Operation::And, Tri::Null, Tri::False) => {
                boolean(false)
            }
            (Operation::And, Tri::True, Tri::False) => boolean(false),
            (Operation::And, Tri::True, Tri::True) => boolean(true),
            (Operation::And, _, _) => Vec::new(),

            (Operation::Or, Tri::True, _) | (Operation::Or, Tri::Null, Tri::True) => boolean(true),
            (Operation::Or, Tri::False, Tri::True) => boolean(true),
            (Operation::Or, Tri::False, Tri::False) => boolean(false),
            (Operation::Or, _, _) => Vec::new(),

            (Operation::Xor, Tri::True, Tri::False) | (Operation::Xor, Tri::False, Tri::True) => {
                boolean(true)
            }
            (Operation::Xor, Tri::True, Tri::True) | (Operation::Xor, Tri::False, Tri::False) => {
                boolean(false)
            }
            (Operation::Xor, _, _) => Vec::new(),

            _ => Vec::new(),
        })
    }

    fn implies(&self, l: Tri, right: &[Value], at: NodeId) -> Result<Vec<Value>, EvalError> {
        if l == Tri::False {
            return Ok(boolean(true));
        }
        if right.is_empty() {
            return Ok(Vec::new());
        }
        Ok(match self.as_bool(right, at)? {
            Tri::False if l == Tri::Null => Vec::new(),
            Tri::False => boolean(false),
            Tri::Null => Vec::new(),
            Tri::True => boolean(true),
        })
    }

    /// `funcIif`: the condition is evaluated against the single focus item
    /// when there is one; only the chosen branch is evaluated; an empty or
    /// false condition with no third parameter is empty.
    pub(super) fn func_iif(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        if focus.len() > 1 {
            return Err(EvalError::execution(
                format!(
                    "Error evaluating FHIRPath expression: The function iif can only be used on a singleton value but found {}",
                    focus.len()
                ),
                At::node(node),
            ));
        }
        let condition = self.param(id, 0)?;
        let evaluated = match focus.first() {
            None => self.execute(context, focus, condition, true)?,
            Some(item) => self.execute(context.change_this(item), focus, condition, true)?,
        };
        if self.as_bool(&evaluated, id)? == Tri::True {
            let then = self.param(id, 1)?;
            self.execute(context, focus, then, true)
        } else if node.parameters().len() < 3 {
            Ok(Vec::new())
        } else {
            let otherwise = self.param(id, 2)?;
            self.execute(context, focus, otherwise, true)
        }
    }
}
