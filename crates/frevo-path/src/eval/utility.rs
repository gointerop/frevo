//! The utility functions: `trace` with its retrievable log, `check`,
//! `aggregate`, `sort` and `extension(url)`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `log` at `:927`,
//! `hasLog`/`takeLog` at `:6687`, `funcAggregate` at `:5295`, `funcExtension`
//! at `:5464`, `funcTrace` at `:5623`, `funcCheck` at `:5649`,
//! `FHIRPathBaseSorter` at `:6121`, `funcSort` at `:6224` and `convertToBoolean`
//! at `:912`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the sort is a stable merge sort over a
//! fallible comparator rather than `Collections.sort` with a comparator that
//! throws, and a sort key is evaluated once per item and parameter rather than
//! once per comparison; the trace log is a shared buffer behind the
//! [`Evaluator`](super::Evaluator).
//!
//! What is the pinned engine's, kept:
//!
//! - the log is `name`, `: `, then the traced items' string values joined by
//!   `,`, with entries joined by `; `; `trace` is the only writer; a host
//!   service whose `log` answers `true` suppresses the entry;
//! - `aggregate` evaluates its first argument with an **empty focus**, a
//!   `$total` that is the previous result (empty for the first item without an
//!   initial value) and a `$index` that is already 1 for the first item;
//! - `sort` with a key that returns more than one value is an error, an item
//!   with no key sorts last, and a descending key is a leading `-`.

use std::cmp::Ordering;

use crate::seam::value::Value;
use crate::syntax::ast::{Kind, NodeId};

use super::collection::{convert_list_to_string, convert_to_string};
use super::equality::primitive_text;
use super::error::At;
use super::navigate::{children_by_name, fhir_type};
use super::{Context, EvalError, Run};

/// `convertToBoolean` (`FHIRPathEngine.java:912`): one boolean is its value,
/// one boolean element is `Boolean.valueOf` of its text, and anything else is
/// whether the list is non-empty.
fn convert_to_boolean(items: &[Value]) -> bool {
    match items {
        [Value::Boolean(b)] => *b,
        [Value::Node(n)] if n.type_name() == "boolean" => n
            .primitive()
            .is_some_and(|p| p.lexical.eq_ignore_ascii_case("true")),
        other => !other.is_empty(),
    }
}

/// The type names `isComparableTypes` accepts for a sort.
const SORTABLE: [&str; 17] = [
    "integer",
    "unsignedInt",
    "positiveInt",
    "decimal",
    "boolean",
    "date",
    "dateTime",
    "time",
    "instant",
    "string",
    "code",
    "url",
    "uri",
    "uuid",
    "oid",
    "canonical",
    "integer64",
];

fn is_integer_type(t: &str) -> bool {
    matches!(t, "integer" | "unsignedInt" | "positiveInt" | "integer64")
}

fn is_text_type(t: &str) -> bool {
    matches!(
        t,
        "string" | "code" | "url" | "uri" | "uuid" | "oid" | "canonical"
    )
}

/// `isComparableTypes` (`FHIRPathEngine.java:6178`).
fn is_comparable_types(t1: &str, t2: &str) -> bool {
    if !SORTABLE.contains(&t1) || !SORTABLE.contains(&t2) {
        return false;
    }
    (is_integer_type(t1) && is_integer_type(t2))
        || (t1 == "boolean" && t2 == "boolean")
        || (t1 == "decimal" && t2 == "decimal")
        || (t1 == "time" && t2 == "time")
        || (matches!(t1, "date" | "dateTime" | "instant")
            && matches!(t2, "date" | "dateTime" | "instant"))
        || (is_text_type(t1) && is_text_type(t2))
}

/// One sort key per parameter, evaluated on first use.
struct SortKeys {
    keys: Vec<Vec<Option<Option<Value>>>>,
}

impl Run<'_> {
    /// `log(name, contents)` (`FHIRPathEngine.java:927`).
    fn log_entry(&self, name: &str, contents: &[Value]) {
        if self.host.is_some_and(|h| h.log(name, contents)) {
            return;
        }
        let mut log = self
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !log.is_empty() {
            log.push_str("; ");
        }
        log.push_str(name);
        log.push_str(": ");
        log.push_str(&convert_list_to_string(contents));
    }

    /// The first value of an evaluated argument as text, as
    /// `nl.get(0).primitiveValue()` reads it: an empty list raises.
    fn first_text(&self, items: &[Value]) -> Result<String, EvalError> {
        match items.first() {
            Some(first) => {
                Ok(primitive_text(self.model, first).unwrap_or_else(|| "null".to_string()))
            }
            None => Err(EvalError::plain("Index 0 out of bounds for length 0")),
        }
    }

    /// `funcTrace` (`FHIRPathEngine.java:5623`): log the focus -- or, with a
    /// second argument, that argument evaluated against the focus -- and
    /// return the focus unchanged.
    pub(super) fn func_trace(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let nl = self.execute(context, focus, self.param(id, 0)?, true)?;
        let name = self.first_text(&nl)?;
        match self.expr.node(id).parameters().get(1) {
            Some(&p) => {
                let n2 = self.execute(context, focus, p, true)?;
                self.log_entry(&name, &n2);
            }
            None => self.log_entry(&name, focus),
        }
        Ok(focus.to_vec())
    }

    /// `funcCheck` (`FHIRPathEngine.java:5649`): the focus when the condition
    /// holds, and an execution error carrying the caller's text when not.
    pub(super) fn func_check(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let n1 = self.execute(context, focus, self.param(id, 0)?, true)?;
        if !convert_to_boolean(&n1) {
            let n2 = self.execute(context, focus, self.param(id, 1)?, true)?;
            let name = self.first_text(&n2)?;
            return Err(EvalError::execution(
                format!("Error evaluating FHIRPath expression: Check failed with message '{name}'"),
                At::node(self.expr.node(id)),
            ));
        }
        Ok(focus.to_vec())
    }

    /// `funcAggregate` (`FHIRPathEngine.java:5295`).
    pub(super) fn func_aggregate(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let mut total: Vec<Value> = match node.parameters().get(1) {
            Some(&p) => self.execute(context, focus, p, false)?,
            None => Vec::new(),
        };
        let p0 = self.param(id, 0)?;
        for item in focus {
            let c = Context {
                index: 1,
                total: Some(&total),
                ..context.change_this(item)
            };
            // The criteria runs over an empty focus: `$this` and `$total`
            // reach it through the context.
            let next = self.execute(c, &[], p0, true)?;
            total = next;
        }
        Ok(total)
    }

    /// `funcExtension` (`FHIRPathEngine.java:5464`): the focus items'
    /// `extension` and `modifierExtension` children whose `url` is the
    /// argument.
    pub(super) fn func_extension(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let nl = self.execute(context, focus, self.param(id, 0)?, true)?;
        let Some(first) = nl.first() else {
            return Err(EvalError::plain("Index 0 out of bounds for length 0"));
        };
        let url = primitive_text(self.model, first);
        let mut result = Vec::new();
        for item in focus {
            let mut ext = Vec::new();
            children_by_name(item, "extension", &mut ext)?;
            children_by_name(item, "modifierExtension", &mut ext)?;
            for ex in ext {
                let mut vl = Vec::new();
                children_by_name(&ex, "url", &mut vl)?;
                if url.as_deref() == Some(convert_list_to_string(&vl).as_str()) {
                    result.push(ex);
                }
            }
        }
        Ok(result)
    }

    /// `funcSort` (`FHIRPathEngine.java:6224`): a stable sort of a copy of the
    /// focus, by natural order or by the keys the parameters give.
    pub(super) fn func_sort(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let params = self.expr.node(id).parameters().to_vec();
        let mut keys = SortKeys {
            keys: vec![vec![None; params.len()]; focus.len()],
        };
        let mut order: Vec<usize> = (0..focus.len()).collect();
        self.merge_sort(&mut order, &mut |a, b| {
            self.compare_items(context, focus, &params, &mut keys, a, b)
        })?;
        Ok(order.into_iter().map(|i| focus[i].clone()).collect())
    }

    /// `FHIRPathBaseSorter.compare` for the items at `a` and `b`.
    fn compare_items(
        &self,
        context: Context<'_>,
        focus: &[Value],
        params: &[NodeId],
        keys: &mut SortKeys,
        a: usize,
        b: usize,
    ) -> Result<Ordering, EvalError> {
        if params.is_empty() {
            return self.compare_natural(Some(&focus[a]), Some(&focus[b]), None);
        }
        for (at, &p) in params.iter().enumerate() {
            let pnode = self.expr.node(p);
            let (reversed, pf) = if pnode.kind() == Kind::Unary {
                (true, pnode.op_next().unwrap_or(p))
            } else {
                (false, p)
            };
            let k1 = self.sort_key(context, focus, keys, a, at, pf)?;
            let k2 = self.sort_key(context, focus, keys, b, at, pf)?;
            let sorter = self.expr.render(pf);
            let res = self.compare_natural(k1.as_ref(), k2.as_ref(), Some(&sorter))?;
            if res != Ordering::Equal {
                return Ok(if reversed { res.reverse() } else { res });
            }
        }
        Ok(Ordering::Equal)
    }

    /// `executeExpression(o, pf)`: the key of one item, evaluated once.
    fn sort_key(
        &self,
        context: Context<'_>,
        focus: &[Value],
        keys: &mut SortKeys,
        item: usize,
        param: usize,
        pf: NodeId,
    ) -> Result<Option<Value>, EvalError> {
        if let Some(known) = &keys.keys[item][param] {
            return Ok(known.clone());
        }
        let b = &focus[item];
        let out = self.execute(context.change_this(b), std::slice::from_ref(b), pf, true)?;
        let key = match out.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            _ => {
                return Err(EvalError::plain(format!(
                    "Multiple values found in return from sort expression '{}'",
                    self.expr.render(pf)
                )));
            }
        };
        keys.keys[item][param] = Some(key.clone());
        Ok(key)
    }

    /// `compareNatural` (`FHIRPathEngine.java:6159`): an item with no key
    /// sorts after one that has a key.
    fn compare_natural(
        &self,
        o1: Option<&Value>,
        o2: Option<&Value>,
        sorter: Option<&str>,
    ) -> Result<Ordering, EvalError> {
        let (o1, o2) = match (o1, o2) {
            (None, None) => return Ok(Ordering::Equal),
            (None, Some(_)) => return Ok(Ordering::Greater),
            (Some(_), None) => return Ok(Ordering::Less),
            (Some(a), Some(b)) => (a, b),
        };
        let (t1, t2) = (fhir_type(o1), fhir_type(o2));
        if !is_comparable_types(t1, t2) {
            let message = match sorter {
                None => format!(
                    "Cannot sort a list that contains both a '{t1}' and a '{t2}' using natural sort order"
                ),
                Some(s) => format!(
                    "Cannot sort the list because the sort expression return both a '{t1}' and a '{t2}' (expression = '{s}')"
                ),
            };
            return Err(EvalError::plain(message));
        }
        let s1 = primitive_text(self.model, o1).unwrap_or_else(|| convert_to_string(o1));
        let s2 = primitive_text(self.model, o2).unwrap_or_else(|| convert_to_string(o2));
        let bad = |s: &str| EvalError::plain(format!("For input string: \"{s}\""));
        Ok(if is_integer_type(t1) {
            let a: i32 = s1.parse().map_err(|_| bad(&s1))?;
            let b: i32 = s2.parse().map_err(|_| bad(&s2))?;
            a.cmp(&b)
        } else if t1 == "decimal" {
            let a: f64 = s1.parse().map_err(|_| bad(&s1))?;
            let b: f64 = s2.parse().map_err(|_| bad(&s2))?;
            a.total_cmp(&b)
        } else if t1 == "boolean" {
            // The order is deliberately reversed upstream.
            s2.encode_utf16().cmp(s1.encode_utf16())
        } else {
            s1.encode_utf16().cmp(s2.encode_utf16())
        })
    }

    /// A stable top-down merge sort of `items`, with a comparator that may
    /// fail. `slice::sort_by` cannot be used: an error has to stop the sort,
    /// and a comparator that gives up half way is not a total order, which
    /// the standard sort is allowed to panic on.
    fn merge_sort(
        &self,
        items: &mut [usize],
        cmp: &mut dyn FnMut(usize, usize) -> Result<Ordering, EvalError>,
    ) -> Result<(), EvalError> {
        let n = items.len();
        if n < 2 {
            return Ok(());
        }
        let mid = n / 2;
        self.merge_sort(&mut items[..mid], cmp)?;
        self.merge_sort(&mut items[mid..], cmp)?;
        let left = items[..mid].to_vec();
        let right = items[mid..].to_vec();
        let (mut i, mut j, mut k) = (0, 0, 0);
        while i < left.len() && j < right.len() {
            // Take from the right only when it is strictly smaller, which is
            // what keeps the sort stable.
            if cmp(right[j], left[i])? == Ordering::Less {
                items[k] = right[j];
                j += 1;
            } else {
                items[k] = left[i];
                i += 1;
            }
            k += 1;
        }
        while i < left.len() {
            items[k] = left[i];
            i += 1;
            k += 1;
        }
        while j < right.len() {
            items[k] = right[j];
            j += 1;
            k += 1;
        }
        Ok(())
    }
}
