//! The evaluator core: a parsed [`Expression`] evaluated over a [`Value`].
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 — `evaluate(Base,
//! ExpressionNode)` at `:697`, `execute(ExecutionContext, List<Base>,
//! ExpressionNode, boolean)` at `:1467`, `ExecutionContext` at `:954`,
//! `funcItem` at `:6277`, `funcChildren` at `:4830` and `funcDescendants` at
//! `:4810`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the execution context is an immutable value
//! borrowed down the recursion rather than a mutable object copied at every
//! step; evaluation is over the `Node` and `Model` traits; and every function,
//! operator and environment variable this module does not implement yet is an
//! [`EvalError::Unsupported`] rather than a missing `case`.
//!
//! Task 31 of `add-fhirpath-and-codegen-r4`: navigation. What exists here is
//! the walk itself — names, the leading type name, flattening, groups,
//! literals, `$this`, `$index` and `$total`, the indexer, `children()` and
//! `descendants()`. Tasks 32-37 added the collection functions, equality,
//! equivalence, ordered comparison, boolean logic, `iif`, `hasValue` and
//! arithmetic; the unary sign is `0 - x` or `0 + x` through the arithmetic, as
//! upstream. Every other function and operator answers
//! [`EvalError::Unsupported`], which is **not** an empty collection: the
//! oracle grades that as a case that did not run, never as one that matched
//! nothing.
//!
//! Performance (`CLAUDE.md` section 6, `path.perf.002`): the expression is
//! parsed once and only borrowed here; a name step borrows its name from the
//! parsed form and allocates no string; a focus collection is borrowed by
//! every step that reads it.

mod arith;
mod collection;
mod combine;
mod compare;
mod convert;
mod equality;
mod error;
mod filter;
mod logic;
mod mathfn;
mod navigate;
mod operand;
mod strext;
mod strfn;
mod temporal;

#[cfg(test)]
mod tests;

pub use error::EvalError;

use self::arith::Arith;
use self::collection::Tri;
use self::compare::Ordered;
use crate::seam::model::Model;
use crate::seam::value::Value;
use crate::syntax::ast::{Constant, Expression, Function, Kind, NodeId, Operation};
use crate::syntax::literal::process_date_constant;

/// The evaluation state `ExecutionContext` carries that navigation reads.
///
/// `%resource`, `%rootResource`, `%context` and defined variables are the
/// environment-variable tasks' and are not represented: reading one is
/// [`EvalError::Unsupported`], so nothing can observe their absence as an
/// empty value.
#[derive(Clone, Copy)]
struct Context<'c> {
    /// `$this`. `None` where upstream's is `null`: an evaluation with no
    /// focus at all.
    this: Option<&'c Value>,
    /// `$index`. Upstream initialises it to 0 and only `select` moves it.
    index: usize,
    /// `$total`. Only `aggregate()` sets it; everywhere else it is `null`.
    total: Option<&'c [Value]>,
}

impl Context<'_> {
    /// `changeThis(context, newThis)` (`FHIRPathEngine.java:4719`): a context
    /// for one item of a per-item criteria. The index goes back to 0 and
    /// `$total` is not carried, because upstream builds a new
    /// `ExecutionContext` and copies only the defined variables.
    fn change_this<'n>(&self, item: &'n Value) -> Context<'n> {
        Context {
            this: Some(item),
            index: 0,
            total: None,
        }
    }
}

/// Evaluates parsed expressions against a model.
///
/// An `Evaluator` holds no per-evaluation state, so one can serve any number
/// of evaluations, and so can one [`Expression`]: neither is mutated.
#[derive(Clone, Copy)]
pub struct Evaluator<'m> {
    model: &'m dyn Model,
    /// The evaluation time zone, minutes east of UTC (`DEV-0018`).
    zone_minutes: i32,
    /// `setDoImplicitStringConversion`: off, as both upstream wrappers leave it.
    implicit_string_conversion: bool,
}

impl<'m> Evaluator<'m> {
    /// An evaluator that answers type questions from `model`, in UTC.
    pub fn new(model: &'m dyn Model) -> Self {
        Self {
            model,
            zone_minutes: 0,
            implicit_string_conversion: false,
        }
    }

    /// `FHIRPathEngine.setDoImplicitStringConversion`: when on, the string
    /// functions convert a non-string focus to its string value instead of
    /// answering empty (`path.string.024`).
    #[must_use]
    pub fn with_implicit_string_conversion(mut self, on: bool) -> Self {
        self.implicit_string_conversion = on;
        self
    }

    /// The evaluation time zone as a fixed offset in minutes east of UTC.
    /// `today()` and `now()` are read in it, and a `dateTime` with no offset
    /// is placed in it when compared with one that has an offset. The default
    /// is UTC; the pinned engine reads the JVM's default zone instead, which
    /// the suite sets to `UTC+1100`.
    #[must_use]
    pub fn with_time_zone_offset_minutes(mut self, minutes: i32) -> Self {
        self.zone_minutes = minutes;
        self
    }

    /// `FHIRPathEngine.evaluate(Base, ExpressionNode)`: evaluate `expr` with
    /// `focus` as both the focus collection and `$this`. `None` is upstream's
    /// `null` base: an empty focus.
    pub fn evaluate(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
    ) -> Result<Vec<Value>, EvalError> {
        let focus_collection: &[Value] = match focus {
            Some(v) => std::slice::from_ref(v),
            None => &[],
        };
        let context = Context {
            this: focus,
            index: 0,
            total: None,
        };
        let _zone = temporal::ZoneGuard::set(self.zone_minutes);
        Run {
            model: self.model,
            expr,
            implicit_string_conversion: self.implicit_string_conversion,
        }
        .execute(context, focus_collection, expr.root(), true)
    }
}

/// One evaluation of one expression.
struct Run<'r> {
    model: &'r dyn Model,
    expr: &'r Expression,
    implicit_string_conversion: bool,
}

impl Run<'_> {
    /// `execute(context, focus, exp, atEntry)` (`FHIRPathEngine.java:1467`).
    fn execute(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        at_entry: bool,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let mut work: Vec<Value> = match node.kind() {
            // `-x` is `0 - x`: the unary node contributes the zero and its
            // operation chain below applies the sign.
            Kind::Unary => vec![Value::Integer(0)],
            Kind::Name => {
                let name = node.name().unwrap_or("");
                if at_entry && name == "$this" {
                    context.this.into_iter().cloned().collect()
                } else if at_entry && name == "$total" {
                    match context.total {
                        Some(total) => total.to_vec(),
                        // Upstream reads a null list here and fails with a
                        // NullPointerException; Frevo returns the failure
                        // (`path.navigation.018`, `path.errors.003`).
                        None => {
                            return Err(EvalError::execution(
                                "$total is only defined inside the arguments of aggregate()",
                                node.start(),
                            ));
                        }
                    }
                } else if at_entry && name == "$index" {
                    vec![index_value(context.index)]
                } else {
                    let mut out = Vec::new();
                    for item in focus {
                        self.execute_name(item, name, at_entry, &mut out)?;
                    }
                    out
                }
            }
            Kind::Function => self.evaluate_function(context, focus, id)?,
            Kind::Constant => self.resolve_constant(id)?,
            Kind::Group => match node.group() {
                Some(group) => self.execute(context, focus, group, at_entry)?,
                None => Vec::new(),
            },
        };

        if let Some(inner) = node.inner() {
            work = self.execute(context, &work, inner, false)?;
        }

        if node.is_proximal() && node.operation().is_some() {
            let mut last = id;
            let mut next = node.op_next();
            while let Some(next_id) = next {
                let last_node = self.expr.node(last);
                let Some(op) = last_node.operation() else {
                    break;
                };
                if let Some(decided) = self.pre_operate(&work, op, id)? {
                    work = decided;
                } else if matches!(op, Operation::Is | Operation::As) {
                    // `executeTypeName` reads the right operand as a type
                    // name, not as an expression; the type operators are a
                    // later task's.
                    return Err(EvalError::unsupported(
                        format!("the operator {}", op.to_code()),
                        last_node.op_start(),
                    ));
                } else {
                    let right = self.execute(context, focus, next_id, true)?;
                    work = self.operate(&work, op, &right, last)?;
                }
                last = next_id;
                next = self.expr.node(next_id).op_next();
            }
        }
        Ok(work)
    }

    /// One name step over one item (`FHIRPathEngine.java:3182`).
    ///
    /// The two host-service constant lookups upstream makes at entry fire only
    /// with an application context and host services installed, and neither
    /// is part of this evaluator yet — so there is nothing to shadow a name
    /// and nothing to fall back to.
    fn execute_name(
        &self,
        item: &Value,
        name: &str,
        at_entry: bool,
        out: &mut Vec<Value>,
    ) -> Result<(), EvalError> {
        if at_entry && name.chars().next().is_some_and(char::is_uppercase) {
            if navigate::type_name_matches(self.model, item, name) {
                out.push(item.clone());
            }
            Ok(())
        } else {
            navigate::children_by_name(item, name, out).map_err(EvalError::from)
        }
    }

    /// `resolveConstant(context, constant, ...)` for the forms that resolve
    /// without an environment.
    fn resolve_constant(&self, id: NodeId) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        match node.constant() {
            None | Some(Constant::Empty) => Ok(Vec::new()),
            Some(Constant::Value(value)) => Ok(vec![value.clone()]),
            Some(Constant::Deferred(text)) => {
                if let Some(run) = text.strip_prefix('@') {
                    process_date_constant(run)
                        .map(|v| vec![v])
                        .map_err(|message| EvalError::execution(message, node.start()))
                } else {
                    Err(EvalError::unsupported(
                        format!("the environment variable {text}"),
                        node.start(),
                    ))
                }
            }
        }
    }

    /// `evaluateFunction` (`FHIRPathEngine.java:3918`), for the functions
    /// navigation owns.
    fn evaluate_function(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        match node.function() {
            Some(Function::Item) => self.func_item(context, focus, id),
            Some(Function::Children) => func_children(focus),
            Some(Function::Descendants) => func_descendants(focus),
            Some(Function::Empty) => self.func_empty(focus),
            Some(Function::Exists) => self.func_exists(context, focus, id),
            Some(Function::Not) => self.func_not(focus, id),
            Some(Function::All) => self.func_all(context, focus, id),
            Some(Function::AllTrue) => self.func_fold(context, focus, id, Tri::True, true),
            Some(Function::AnyTrue) => self.func_fold(context, focus, id, Tri::True, false),
            Some(Function::AllFalse) => self.func_fold(context, focus, id, Tri::False, true),
            Some(Function::AnyFalse) => self.func_fold(context, focus, id, Tri::False, false),
            Some(Function::SubsetOf) => self.func_subset_of(context, focus, id),
            Some(Function::SupersetOf) => self.func_superset_of(context, focus, id),
            Some(Function::IsDistinct) => self.func_is_distinct(focus, id),
            Some(Function::Distinct) => self.func_distinct(focus, id),
            Some(Function::Count) => Ok(self.func_count(focus)),
            Some(Function::Iif) => self.func_iif(context, focus, id),
            Some(Function::HasValue) => Ok(self.func_has_value(focus)),
            Some(Function::Union) => self.func_union(context, focus, id),
            Some(Function::Combine) => self.func_combine(context, focus, id),
            Some(Function::Where) => self.func_where(context, focus, id),
            Some(Function::Select) => self.func_select(context, focus, id),
            Some(Function::Repeat) => self.func_repeat(context, focus, id),
            Some(Function::OfType) => self.func_of_type(focus, id),
            Some(Function::Single) => self.func_single(focus, id),
            // `funcFirst`, `funcLast` and `funcTail`: one item at either end,
            // or everything after the first, with no error for a short focus.
            Some(Function::First) => Ok(focus.first().cloned().into_iter().collect()),
            Some(Function::Last) => Ok(focus.last().cloned().into_iter().collect()),
            Some(Function::Tail) => Ok(focus.iter().skip(1).cloned().collect()),
            Some(Function::Skip) => self.func_skip(context, focus, id),
            Some(Function::Take) => self.func_take(context, focus, id),
            Some(Function::Intersect) => self.func_intersect(context, focus, id),
            Some(Function::Exclude) => self.func_exclude(context, focus, id),
            // `funcToday` and `funcNow` read the clock on every call; no
            // evaluation instant is cached (`DEV-0008`).
            Some(
                f @ (Function::Abs
                | Function::Ceiling
                | Function::Floor
                | Function::Round
                | Function::Truncate
                | Function::Sqrt
                | Function::Exp
                | Function::Ln
                | Function::Log
                | Function::Power),
            ) => self.func_math(context, focus, id, f),
            Some(
                f @ (Function::Upper
                | Function::Lower
                | Function::ToChars
                | Function::IndexOf
                | Function::Substring
                | Function::StartsWith
                | Function::EndsWith
                | Function::Contains
                | Function::Length),
            ) => self.func_string(context, focus, id, f),
            Some(
                f @ (Function::Replace
                | Function::Trim
                | Function::Split
                | Function::Join
                | Function::Encode
                | Function::Decode
                | Function::Escape
                | Function::Unescape),
            ) => self.func_string_ext(context, focus, id, f),
            Some(
                f @ (Function::ToInteger
                | Function::ToDecimal
                | Function::ToString
                | Function::ToBoolean
                | Function::ToQuantity
                | Function::ToDateTime
                | Function::ToTime
                | Function::ConvertsToInteger
                | Function::ConvertsToDecimal
                | Function::ConvertsToString
                | Function::ConvertsToBoolean
                | Function::ConvertsToQuantity
                | Function::ConvertsToDateTime
                | Function::ConvertsToDate
                | Function::ConvertsToTime),
            ) => self.func_convert(focus, id, f),
            Some(Function::Today) => Ok(vec![temporal::today()]),
            Some(Function::Now) => Ok(vec![temporal::now()]),
            Some(Function::Custom) | None => Err(EvalError::unsupported(
                format!("the host function {}()", node.name().unwrap_or("?")),
                node.start(),
            )),
            Some(other) => Err(EvalError::unsupported(
                format!("the function {}()", other.to_code()),
                node.start(),
            )),
        }
    }

    /// The indexer, `name[i]` (`funcItem`, `FHIRPathEngine.java:6277`).
    ///
    /// The index is evaluated against the focus collection. Zero-based; an
    /// index at or past the end selects nothing; an empty index selects
    /// nothing. Two answers differ from the pinned engine and are written to
    /// the approved scenarios, with the conflict recorded in
    /// `tasks/blockers.md` and `docs/deviations.md` (`DEV-0012`, `DEV-0013`):
    ///
    /// - a **negative** index raises (`focus.get(-1)` throws
    ///   `IndexOutOfBoundsException` upstream, `DEV-0012`), as a typed error;
    /// - an index that is **not an integer** is an execution error
    ///   (`path.navigation.011`), where upstream's `Utilities.isInteger` test
    ///   fails and it returns empty — including for the string `'1'`, which
    ///   upstream would convert and accept.
    fn func_item(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let Some(&param) = node.parameters().first() else {
            return Err(EvalError::execution(
                "the indexer requires an index",
                node.start(),
            ));
        };
        let index = self.execute(context, focus, param, true)?;
        let position = match index.as_slice() {
            [] => return Ok(Vec::new()),
            [Value::Integer(i)] => *i,
            [single] => match integer_of(single) {
                Some(i) => i,
                None => {
                    return Err(EvalError::execution(
                        format!(
                            "the indexer requires an integer, but the index is a {}",
                            navigate::fhir_type(single)
                        ),
                        self.expr.node(param).start(),
                    ));
                }
            },
            many => {
                return Err(EvalError::execution(
                    format!(
                        "the indexer requires a single integer, but the index has {} items",
                        many.len()
                    ),
                    self.expr.node(param).start(),
                ));
            }
        };
        // `Integer.parseInt(s) < focus.size()` admits a negative index and
        // `focus.get(-1)` then raises; raise a typed error rather than panic.
        let Ok(position) = usize::try_from(position) else {
            return Err(EvalError::execution(
                format!("Index {position} out of bounds for length {}", focus.len()),
                self.expr.node(param).start(),
            ));
        };
        Ok(focus.get(position).cloned().into_iter().collect())
    }

    /// `operate(context, left, operation, right, holder)`
    /// (`FHIRPathEngine.java:1843`), for the operators implemented so far.
    /// `holder` is the node whose operator this is; a failure is attributed to
    /// its start, as `makeException(holder, ...)` does.
    ///
    /// `Is` and `As` take their right operand as a type name rather than an
    /// evaluated expression (`executeTypeName`), so they never reach here, and
    /// `MemberOf` needs a terminology service: both are
    /// [`EvalError::Unsupported`].
    ///
    /// The unary sign is `0 - x` or `0 + x` through `Minus` and `Plus`, as it
    /// is upstream, so `-5 'mg'` is the quantity's absolute value and `-'a'` is
    /// the same incompatible-types error `0 - 'a'` is.
    fn operate(
        &self,
        left: &[Value],
        op: Operation,
        right: &[Value],
        holder: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(holder);
        let (at, offset) = (node.start(), node.op_start());
        let model = self.model;
        match op {
            Operation::Equals => equality::op_equals(model, left, right, false, offset),
            Operation::NotEquals => equality::op_equals(model, left, right, true, offset),
            Operation::Equivalent => equality::op_equivalent(model, left, right, false, offset),
            Operation::NotEquivalent => equality::op_equivalent(model, left, right, true, offset),
            Operation::LessThan => compare::op_ordered(model, left, right, Ordered::Less, at),
            Operation::Greater => compare::op_ordered(model, left, right, Ordered::Greater, at),
            Operation::LessOrEqual => {
                compare::op_ordered(model, left, right, Ordered::LessOrEqual, at)
            }
            Operation::GreaterOrEqual => {
                compare::op_ordered(model, left, right, Ordered::GreaterOrEqual, at)
            }
            Operation::Union => self.op_union(left, right, offset),
            Operation::In => self.op_in(left, right, offset),
            Operation::Contains => self.op_contains(left, right, offset),
            Operation::And | Operation::Or | Operation::Xor | Operation::Implies => {
                self.op_logic(left, op, right, holder)
            }
            Operation::Plus => arith::op_arith(model, left, Arith::Plus, right, at),
            Operation::Minus => arith::op_arith(model, left, Arith::Minus, right, at),
            Operation::Times => arith::op_arith(model, left, Arith::Times, right, at),
            Operation::DivideBy => arith::op_arith(model, left, Arith::DivideBy, right, at),
            Operation::Div => arith::op_arith(model, left, Arith::Div, right, at),
            Operation::Mod => arith::op_arith(model, left, Arith::Mod, right, at),
            Operation::Concatenate => arith::op_concatenate(model, left, right, at),
            other => Err(EvalError::unsupported(
                format!("the operator {}", other.to_code()),
                offset,
            )),
        }
    }
}

/// `$index` is an `IntegerType` upstream.
fn index_value(index: usize) -> Value {
    Value::Integer(i64::try_from(index).unwrap_or(i64::MAX))
}

/// An integer-typed node's value: `integer`, `positiveInt`, `unsignedInt`.
fn integer_of(value: &Value) -> Option<i64> {
    let Value::Node(node) = value else {
        return None;
    };
    let primitive = node.primitive()?;
    match primitive.type_name.as_str() {
        "integer" | "positiveInt" | "unsignedInt" => primitive.lexical.parse().ok(),
        _ => None,
    }
}

/// `funcChildren` (`FHIRPathEngine.java:4830`): every child of every focus
/// item.
fn func_children(focus: &[Value]) -> Result<Vec<Value>, EvalError> {
    let mut result = Vec::new();
    for item in focus {
        navigate::children_by_name(item, navigate::ALL_CHILDREN, &mut result)?;
    }
    Ok(result)
}

/// `funcDescendants` (`FHIRPathEngine.java:4810`): `children()` repeated
/// until a generation is empty. The focus itself is not included, and nothing
/// is de-duplicated — upstream's loop adds every generation whole.
fn func_descendants(focus: &[Value]) -> Result<Vec<Value>, EvalError> {
    let mut result = Vec::new();
    let mut current: Vec<Value> = focus.to_vec();
    loop {
        let mut added = Vec::new();
        for item in &current {
            navigate::children_by_name(item, navigate::ALL_CHILDREN, &mut added)?;
        }
        if added.is_empty() {
            break;
        }
        result.extend(added.iter().cloned());
        current = added;
    }
    Ok(result)
}
