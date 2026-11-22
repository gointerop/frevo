//! Collection predicates: `empty`, `exists`, `not`, `all`, the four boolean
//! folds, `subsetOf`, `supersetOf`, `isDistinct`, `distinct` and `count`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `funcEmpty` at `:6286`,
//! `funcExists` at `:5383`, `funcNot` at `:6292`, `funcAll` at `:4696`,
//! `funcAllTrue` at `:5552`, `funcAnyTrue` at `:5585`, `funcAllFalse` at
//! `:5484`, `funcAnyFalse` at `:5518`, `funcSubsetOf` at `:5360`,
//! `funcSupersetOf` at `:5337`, `funcIsDistinct` at `:5313`, `funcDistinct` at
//! `:5659`, `funcCount` at `:6095`, `asBool` at `:6886` and `:6926`,
//! `canConvertToBoolean` at `:5619`, `convertToString` at `:887`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: a criteria is evaluated with an immutable
//! [`Context`] built per item instead of a copied `ExecutionContext`, the
//! `Equality` tri-state is [`Tri`], and a failure is an [`EvalError`] carrying
//! the offset of the function that raised it.
//!
//! Two things here are the pinned engine's and are not what the FHIRPath
//! specification says, and both are kept:
//!
//! - `not()` over an empty collection is **empty**, not `true`: `asBool`
//!   answers `Null` and `funcNot` appends nothing for `Null`;
//! - `distinct()` keeps the **last** occurrence of each value, because
//!   `funcDistinct` drops an item when an *equal item follows it*.

use crate::seam::value::Value;
use crate::syntax::ast::NodeId;

use super::equality::{compare_deep, do_equals, is_empty};
use super::error::At;
use super::navigate::fhir_type;
use super::{Context, EvalError, Run};

/// `Equality` (`FHIRPathEngine.java`): the three-valued answer `asBool` gives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tri {
    True,
    False,
    Null,
}

impl Tri {
    fn of(b: bool) -> Tri {
        if b { Tri::True } else { Tri::False }
    }
}

/// `Base.isBooleanPrimitive()`.
fn is_boolean_primitive(value: &Value) -> bool {
    matches!(value, Value::Boolean(_)) || fhir_type(value) == "boolean"
}

/// `asBool(item, true)` (`FHIRPathEngine.java:6926`) with `narrow` true, which
/// is the only form any collection function calls: a boolean is itself, a
/// boolean element with an unreadable value is `Null`, and anything else is
/// `False`.
fn as_bool_item(value: &Value) -> Tri {
    match value {
        Value::Boolean(b) => Tri::of(*b),
        Value::Node(node) if node.type_name() == "boolean" => match node.primitive() {
            Some(p) if p.lexical == "true" => Tri::True,
            Some(p) if p.lexical == "false" => Tri::False,
            _ => Tri::Null,
        },
        _ => Tri::False,
    }
}

/// `convertToString(Base)` (`FHIRPathEngine.java:887`), for a message.
pub(super) fn convert_to_string(value: &Value) -> String {
    match value {
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Decimal(d) => d.lexical().to_string(),
        Value::String(s) => s.clone(),
        Value::Date(d) => d.lexical.clone(),
        Value::DateTime(d) => d.lexical.clone(),
        Value::Time(t) => t.lexical.clone(),
        Value::Quantity(q) => match (&q.unit, &q.code) {
            (Some(unit), _) if q.system.is_none() || q.system.as_deref() == UCUM => {
                format!("{} {unit}", q.value.lexical())
            }
            // `"'"+q.getCode()+"'"`, so a missing code prints `'null'`
            // (`path.quantity.021`).
            (_, code) if q.system.as_deref() == UCUM => {
                format!(
                    "{} '{}'",
                    q.value.lexical(),
                    code.as_deref().unwrap_or("null")
                )
            }
            _ => "Quantity".to_string(),
        },
        Value::Node(node) => match node.primitive() {
            Some(p) => p.lexical,
            None => node.type_name().to_string(),
        },
    }
}

const UCUM: Option<&str> = Some("http://unitsofmeasure.org");

/// `convertToString(List<Base>)` (`FHIRPathEngine.java:872`).
pub(super) fn convert_list_to_string(items: &[Value]) -> String {
    items
        .iter()
        .map(convert_to_string)
        .collect::<Vec<_>>()
        .join(",")
}

impl Run<'_> {
    /// The `n`th parameter of the function at `id`.
    pub(super) fn param(&self, id: NodeId, n: usize) -> Result<NodeId, EvalError> {
        let node = self.expr.node(id);
        node.parameters().get(n).copied().ok_or_else(|| {
            EvalError::plain(format!(
                "{}() requires parameter {}",
                node.name().unwrap_or("the function"),
                n + 1
            ))
        })
    }

    /// `asBool(List<Base>, expr)` (`FHIRPathEngine.java:6886`): empty is
    /// `Null`, a single boolean is itself, **any other single item is
    /// `True`**, and more than one item is an error.
    pub(super) fn as_bool(&self, items: &[Value], at: NodeId) -> Result<Tri, EvalError> {
        match items {
            [] => Ok(Tri::Null),
            [one] if is_boolean_primitive(one) => Ok(as_bool_item(one)),
            [_] => Ok(Tri::True),
            many => Err(EvalError::execution(
                // `FHIRPATH_UNABLE_BOOLEAN`.
                format!(
                    "Unable to evaluate as a boolean: {}",
                    convert_list_to_string(many)
                ),
                At::node(self.expr.node(at)),
            )),
        }
    }

    /// One criteria evaluated for one item, with `$this` rebound to it
    /// (`execute(changeThis(context, item), pc, param, true)`).
    pub(super) fn criteria(
        &self,
        context: Context<'_>,
        item: &Value,
        param: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        self.execute(
            context.change_this(item),
            std::slice::from_ref(item),
            param,
            true,
        )
    }

    /// `funcEmpty`: `ElementUtil.isEmpty(List)`, which is true when **every
    /// item is itself empty**, so a collection holding only an empty string is
    /// empty.
    pub(super) fn func_empty(&self, focus: &[Value]) -> Result<Vec<Value>, EvalError> {
        let mut all_empty = true;
        for item in focus {
            if !is_empty(item)? {
                all_empty = false;
                break;
            }
        }
        Ok(vec![Value::Boolean(all_empty)])
    }

    /// `funcExists`. With a criteria every item is tried -- the loop does not
    /// stop at the first match, so a failure on a later item still surfaces.
    pub(super) fn func_exists(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let params = self.expr.node(id).parameters();
        let mut empty = true;
        for f in focus {
            if let Some(&p) = params.first().filter(|_| params.len() == 1) {
                let v = self.as_bool(&self.criteria(context, f, p)?, id)?;
                if v == Tri::True {
                    empty = false;
                }
            } else if !is_empty(f)? {
                empty = false;
            }
        }
        Ok(vec![Value::Boolean(!empty)])
    }

    /// `funcNot`: nothing is appended when `asBool` is `Null`.
    pub(super) fn func_not(&self, focus: &[Value], id: NodeId) -> Result<Vec<Value>, EvalError> {
        Ok(match self.as_bool(focus, id)? {
            Tri::Null => Vec::new(),
            v => vec![Value::Boolean(v != Tri::True)],
        })
    }

    /// `funcAll`: `true` for an empty focus, whatever the criteria.
    pub(super) fn func_all(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let params = self.expr.node(id).parameters();
        let mut all = true;
        for item in focus {
            let eq = match params.first().filter(|_| params.len() == 1) {
                Some(&p) => self.as_bool(&self.criteria(context, item, p)?, id)?,
                None => as_bool_item(item),
            };
            if eq != Tri::True {
                all = false;
                break;
            }
        }
        Ok(vec![Value::Boolean(all)])
    }

    /// The four boolean folds (`funcAllTrue`, `funcAnyTrue`, `funcAllFalse`,
    /// `funcAnyFalse`). With a criteria it is evaluated against `context` as
    /// it stands -- `$this` is **not** rebound, unlike `where`. Without one
    /// each item must be a boolean: anything else is an execution error naming
    /// it.
    ///
    /// `want` is the value a fold looks for; `all` selects the `all*` form,
    /// which fails on the first item that is not `want`, over the `any*` form,
    /// which succeeds on the first that is.
    pub(super) fn func_fold(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        want: Tri,
        all: bool,
    ) -> Result<Vec<Value>, EvalError> {
        let params = self.expr.node(id).parameters();
        let criteria = params.first().filter(|_| params.len() == 1).copied();
        let mut result = all;
        for item in focus {
            let v = match criteria {
                Some(p) => {
                    let res = self.execute(context, std::slice::from_ref(item), p, true)?;
                    self.as_bool(&res, id)?
                }
                None => {
                    if !is_boolean_primitive(item) {
                        return Err(EvalError::plain(format!(
                            "Unable to convert '{}' to a boolean",
                            convert_to_string(item)
                        )));
                    }
                    as_bool_item(item)
                }
            };
            if all && v != want {
                result = false;
                break;
            }
            if !all && v == want {
                result = true;
                break;
            }
        }
        Ok(vec![Value::Boolean(result)])
    }

    /// `funcSubsetOf`: every focus item is deeply equal to some target item.
    /// This is `compareDeep`, not `doEquals`.
    pub(super) fn func_subset_of(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let target = self.execute(context, focus, self.param(id, 0)?, true)?;
        let valid = self.all_found(focus, &target, id)?;
        Ok(vec![Value::Boolean(valid)])
    }

    /// `funcSupersetOf`: the mirror of [`Self::func_subset_of`].
    pub(super) fn func_superset_of(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let target = self.execute(context, focus, self.param(id, 0)?, true)?;
        let valid = self.all_found(&target, focus, id)?;
        Ok(vec![Value::Boolean(valid)])
    }

    /// Whether every item of `items` is deeply equal to some item of `within`.
    fn all_found(&self, items: &[Value], within: &[Value], id: NodeId) -> Result<bool, EvalError> {
        let offset = self.expr.node(id).start();
        for item in items {
            let mut found = false;
            for t in within {
                if compare_deep(self.model, item, t, offset)? {
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

    /// `funcIsDistinct`: size 0 or 1 is distinct; a pair `doEquals` cannot
    /// compare makes the answer empty.
    pub(super) fn func_is_distinct(
        &self,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        if focus.len() <= 1 {
            return Ok(vec![Value::Boolean(true)]);
        }
        let offset = self.expr.node(id).start();
        let mut distinct = true;
        for i in 0..focus.len() {
            for j in (i + 1)..focus.len() {
                match do_equals(self.model, &focus[j], &focus[i], offset)? {
                    None => return Ok(Vec::new()),
                    // Upstream's `break` leaves only the inner loop, and the
                    // outer one keeps going; the answer cannot change back.
                    Some(true) => {
                        distinct = false;
                        break;
                    }
                    Some(false) => {}
                }
            }
        }
        Ok(vec![Value::Boolean(distinct)])
    }

    /// `funcDistinct`: an item is dropped when an equal item **follows** it,
    /// so the last occurrence of each value is the one kept.
    pub(super) fn func_distinct(
        &self,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        if focus.len() <= 1 {
            return Ok(focus.to_vec());
        }
        let offset = self.expr.node(id).start();
        let mut result = Vec::new();
        for i in 0..focus.len() {
            let mut found = false;
            for j in (i + 1)..focus.len() {
                match do_equals(self.model, &focus[j], &focus[i], offset)? {
                    None => return Ok(Vec::new()),
                    Some(true) => {
                        found = true;
                        break;
                    }
                    Some(false) => {}
                }
            }
            if !found {
                result.push(focus[i].clone());
            }
        }
        Ok(result)
    }

    /// `funcCount`: always exactly one integer.
    pub(super) fn func_count(&self, focus: &[Value]) -> Vec<Value> {
        vec![Value::Integer(
            i64::try_from(focus.len()).unwrap_or(i64::MAX),
        )]
    }
}
