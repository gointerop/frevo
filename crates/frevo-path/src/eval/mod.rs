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
mod boundary;
mod collection;
mod combine;
mod compare;
mod convert;
mod env;
mod equality;
mod error;
mod filter;
mod logic;
mod mathfn;
mod navigate;
mod operand;
mod strext;
mod strfn;
mod strregex;
mod temporal;
mod types;
mod utility;

#[cfg(test)]
mod env_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod utility_tests;

use std::sync::{Arc, Mutex};

pub use error::EvalError;

use self::arith::Arith;
use self::collection::Tri;
use self::compare::Ordered;
use crate::seam::host::{ConstantMode, HostServices};
use crate::seam::model::Model;
use crate::seam::value::Value;
use crate::syntax::ast::{Expression, Function, Kind, NodeId, Operation};

/// The evaluation state `ExecutionContext` carries that changes as the walk
/// goes down. What is fixed for a whole evaluation -- `%resource`,
/// `%rootResource`, `%context`, the host service and the application context
/// -- lives on [`Run`] instead.
#[derive(Clone, Copy)]
struct Context<'c> {
    /// `$this`. `None` where upstream's is `null`: an evaluation with no
    /// focus at all.
    this: Option<&'c Value>,
    /// `$index`. Upstream initialises it to 0 and only `select` moves it.
    index: usize,
    /// `$total`. Only `aggregate()` sets it; everywhere else it is `null`.
    total: Option<&'c [Value]>,
    /// The variables `defineVariable` has introduced on the way here, newest
    /// first.
    vars: Option<&'c VarNode<'c>>,
}

/// One variable `defineVariable` introduced. A context is copied, not shared,
/// at every node upstream, so a definition is visible to the rest of the
/// chain that follows it and to nothing else; an immutable list whose tail is
/// the enclosing scope has exactly that visibility.
struct VarNode<'p> {
    name: String,
    value: Vec<Value>,
    parent: Option<&'p VarNode<'p>>,
}

impl<'c> Context<'c> {
    /// `changeThis(context, newThis)` (`FHIRPathEngine.java:4726`): a context
    /// for one item of a per-item criteria. The index goes back to 0 and
    /// `$total` is not carried, because upstream builds a new
    /// `ExecutionContext` and copies only the defined variables.
    fn change_this<'n>(&self, item: &'n Value) -> Context<'n>
    where
        'c: 'n,
    {
        Context {
            this: Some(item),
            index: 0,
            total: None,
            vars: self.vars,
        }
    }

    /// `getDefinedVariable(name)`, or `None` where `hasDefinedVariable` is
    /// false.
    fn defined_variable(&self, name: &str) -> Option<&'c [Value]> {
        let mut at = self.vars;
        while let Some(var) = at {
            if var.name == name {
                return Some(&var.value);
            }
            at = var.parent;
        }
        None
    }
}

/// `isSystemVariable` (`FHIRPathEngine.java:1743`): the names
/// `defineVariable` may not take.
fn is_system_variable(name: &str) -> bool {
    matches!(
        name,
        "sct" | "loinc" | "ucum" | "resource" | "rootResource" | "context"
    )
}

/// Evaluates parsed expressions against a model.
///
/// An `Evaluator` holds one piece of state, the `trace()` log, which
/// [`Evaluator::evaluate`] starts afresh and [`Evaluator::take_log`] empties; a
/// clone shares it. Nothing else is kept between evaluations, so one evaluator
/// can serve any number of them, and so can one [`Expression`]: neither is
/// mutated.
#[derive(Clone)]
pub struct Evaluator<'m> {
    model: &'m dyn Model,
    /// The evaluation time zone, minutes east of UTC (`DEV-0018`).
    zone_minutes: i32,
    /// `setDoImplicitStringConversion`: off, as both upstream wrappers leave it.
    implicit_string_conversion: bool,
    /// `doNotEnforceAsSingletonRule`, inverted: `as` raises for a focus of
    /// more than one item. Enforced unless a caller says otherwise.
    enforce_as_singleton: bool,
    /// `doNotEnforceAsCaseSensitive`, inverted: `as` compares type names
    /// exactly. Enforced unless a caller says otherwise.
    enforce_as_case_sensitive: bool,
    /// The host service, if one is installed. Not having one is a state of its
    /// own, distinct from a host service that answers nothing.
    host: Option<&'m dyn HostServices>,
    /// Whether a non-null application context is passed. Both implicit
    /// constant modes are gated on this and not on the host service
    /// (`FHIRPathEngine.java:3184`, `:3213`).
    app_context: bool,
    /// The `trace()` log (`FHIRPathEngine.log`).
    log: Arc<Mutex<String>>,
}

impl<'m> Evaluator<'m> {
    /// An evaluator that answers type questions from `model`, in UTC.
    pub fn new(model: &'m dyn Model) -> Self {
        Self {
            model,
            zone_minutes: 0,
            implicit_string_conversion: false,
            enforce_as_singleton: true,
            enforce_as_case_sensitive: true,
            host: None,
            app_context: false,
            log: Arc::new(Mutex::new(String::new())),
        }
    }

    /// `hasLog()` (`FHIRPathEngine.java:6687`): whether a `trace()` has
    /// written to the log since the last evaluation started or the log was
    /// taken.
    pub fn has_log(&self) -> bool {
        !self.lock_log().is_empty()
    }

    /// `takeLog()` (`FHIRPathEngine.java:6692`): the log, which is **emptied**
    /// by taking it. A second call with no `trace()` between is the empty
    /// string, which is what stops one case's lines leaking into the next
    /// when one evaluator serves many.
    pub fn take_log(&self) -> String {
        std::mem::take(&mut *self.lock_log())
    }

    fn lock_log(&self) -> std::sync::MutexGuard<'_, String> {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Install a host service (`setHostServices`). Without one an unknown
    /// `%name` is an execution error.
    #[must_use]
    pub fn with_host_services(mut self, host: &'m dyn HostServices) -> Self {
        self.host = Some(host);
        self
    }

    /// Say that the evaluation carries a non-null application context, which
    /// is the condition under which a host service is asked about a bare name
    /// -- before navigation (`ConstantMode::ImplicitBefore`) and after it
    /// found nothing (`ConstantMode::ImplicitAfter`). A host service with no
    /// application context never sees either (`DEV-0010`).
    #[must_use]
    pub fn with_application_context(mut self, present: bool) -> Self {
        self.app_context = present;
        self
    }

    /// The two rules `as` enforces: that its focus is a singleton, and that
    /// the type name matches in case. Both are on by default, as in the
    /// pinned engine (`setDoNotEnforceAsSingletonRule` and
    /// `setDoNotEnforceAsCaseSensitive` both start `false`).
    #[must_use]
    pub fn with_as_rules(mut self, singleton: bool, case_sensitive: bool) -> Self {
        self.enforce_as_singleton = singleton;
        self.enforce_as_case_sensitive = case_sensitive;
        self
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
    /// `focus` as the focus collection, `$this` and `%context`. `None` is
    /// upstream's `null` base: an empty focus. `%resource` and `%rootResource`
    /// are the focus when it is a resource and absent otherwise.
    pub fn evaluate(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
    ) -> Result<Vec<Value>, EvalError> {
        let resource = focus.filter(|v| matches!(v, Value::Node(n) if n.is_resource()));
        self.evaluate_in(expr, focus, resource, resource)
    }

    /// `FHIRPathEngine.evaluate(appContext, focusResource, rootResource, base,
    /// expression)` (`:771`): the same, with `%resource` and `%rootResource`
    /// supplied by the caller. This is how an evaluation that starts partway
    /// down a resource -- an invariant on `Observation.effective`, say --
    /// still sees the resource it is inside.
    pub fn evaluate_in(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
        focus_resource: Option<&Value>,
        root_resource: Option<&Value>,
    ) -> Result<Vec<Value>, EvalError> {
        let focus_collection: &[Value] = match focus {
            Some(v) => std::slice::from_ref(v),
            None => &[],
        };
        let context = Context {
            this: focus,
            index: 0,
            total: None,
            vars: None,
        };
        let _zone = temporal::ZoneGuard::set(self.zone_minutes);
        // Every `evaluate` overload starts `log = new StringBuilder()`.
        self.lock_log().clear();
        Run {
            log: &self.log,
            model: self.model,
            expr,
            implicit_string_conversion: self.implicit_string_conversion,
            enforce_as_singleton: self.enforce_as_singleton,
            enforce_as_case_sensitive: self.enforce_as_case_sensitive,
            host: self.host,
            app_context: self.app_context,
            base: focus,
            focus_resource,
            root_resource,
        }
        .execute(context, focus_collection, expr.root(), true)
    }
}

/// One evaluation of one expression.
struct Run<'r> {
    model: &'r dyn Model,
    expr: &'r Expression,
    implicit_string_conversion: bool,
    enforce_as_singleton: bool,
    enforce_as_case_sensitive: bool,
    host: Option<&'r dyn HostServices>,
    app_context: bool,
    log: &'r Mutex<String>,
    /// `%context`: the node evaluation started from.
    base: Option<&'r Value>,
    /// `%resource`.
    focus_resource: Option<&'r Value>,
    /// `%rootResource`.
    root_resource: Option<&'r Value>,
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
        let mut defined: Option<VarNode<'_>> = None;
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
                        self.execute_name(item, name, at_entry, node.start(), &mut out)?;
                    }
                    out
                }
            }
            Kind::Function if node.function() == Some(Function::DefineVariable) => {
                let (name, value, out) = self.func_define_variable(context, focus, id)?;
                defined = Some(VarNode {
                    name,
                    value,
                    parent: context.vars,
                });
                out
            }
            Kind::Function => self.evaluate_function(context, focus, id)?,
            Kind::Constant => self.resolve_constant(context, id, ConstantMode::Explicit)?,
            Kind::Group => match node.group() {
                Some(group) => self.execute(context, focus, group, at_entry)?,
                None => Vec::new(),
            },
        };

        if let Some(inner) = node.inner() {
            // A variable `defineVariable` just introduced is visible to the
            // rest of this chain and to nothing beside it: the operator loop
            // below runs in the context the node was entered with.
            let inner_context = match &defined {
                Some(var) => Context {
                    vars: Some(var),
                    ..context
                },
                None => context,
            };
            work = self.execute(inner_context, &work, inner, false)?;
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
                    // name, not as an expression.
                    let tn = self.type_name_operand(next_id);
                    work = if op == Operation::Is {
                        self.op_is(&work, &tn)
                    } else {
                        self.op_as(&work, &tn, last)?
                    };
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
            Some(Function::Trace) => self.func_trace(context, focus, id),
            Some(Function::Check) => self.func_check(context, focus, id),
            Some(Function::Aggregate) => self.func_aggregate(context, focus, id),
            Some(Function::Sort) => self.func_sort(context, focus, id),
            Some(Function::Extension) => self.func_extension(context, focus, id),
            Some(Function::Is) => self.func_is(focus, id),
            Some(Function::As) => self.func_as(focus, id),
            Some(Function::Type) => Ok(self.func_type(focus)),
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
            Some(f @ (Function::Matches | Function::MatchesFull | Function::ReplaceMatches)) => {
                self.func_regex(context, focus, id, f)
            }
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
            Some(f @ (Function::LowBoundary | Function::HighBoundary)) => {
                self.func_boundary(context, focus, id, f)
            }
            Some(Function::Precision) => self.func_precision(focus, id),
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
