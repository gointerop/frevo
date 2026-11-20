//! The one place an evaluation writes to its input: an ordered comparison
//! rewrites a `dateTime` operand's stored string to its Zulu form.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`compareDateTimeElements` at `:635`) and
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/BaseDateTimeType.java
//! (`setTimeZoneZulu` at `:690`, `encode` at `:190`), both at
//! 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the input is never written. Upstream calls
//! `setTimeZoneZulu(true)` on the operand object itself when it is already a
//! `DateTimeType` finer than a day, so every later read of that element in the
//! same evaluation sees the Zulu string (`path.compare.013`). Here the
//! rewritten string is recorded in a per-evaluation overlay keyed by node
//! identity, and navigation hands out a node that reports it; the overlay is
//! dropped when the evaluation ends. Every collection a step of the evaluation
//! produces is passed through the overlay too ([`apply_all`]), which is how
//! the items `where` returns and `$this` read after the comparison carry the
//! rewrite (`funcWhere`, `funcSelect`, `changeThis`,
//! `FHIRPathEngine.java:6249-6273`, `:4726-4734`).
//!
//! What is kept and what is not:
//!
//! - only an **element** of FHIR type `dateTime` is rewritten. A `date`
//!   (`DateType`) and an `instant` (`InstantType`) are not `DateTimeType`s and
//!   are copied upstream, so they are never rewritten;
//! - a `dateTime` **literal** is rewritten for the rest of the evaluation that
//!   compared it, and only the value that evaluation of its node produced
//!   (`path.compare.015`). Upstream holds a `@` literal as its text and builds
//!   a new object from it every time the node is evaluated
//!   (`FHIRPathEngine.java:1335-1336`, `:1653-1736`), so a second occurrence of
//!   the same text, or a later evaluation of the same parse, starts from the
//!   text again. The object's identity is the value's occurrence number
//!   ([`new_occurrence`]), which every clone of the value shares;
//! - any other `dateTime` value an evaluation builds -- `toDateTime()`,
//!   `now()`, a boundary or arithmetic result -- is also a new `DateTimeType`
//!   upstream and would be rewritten the same way; no scenario covers it, so it
//!   carries no occurrence number and is not;
//! - identity is the node's allocation. A model that hands out a fresh node
//!   for the same element on every navigation does not see the rewrite; the
//!   harness model and an in-memory tree do.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::seam::node::{Node, NodeError};
use crate::seam::value::{Primitive, Value};

use super::navigate::fhir_type;
use super::temporal;

/// The rewrites recorded in one evaluation.
#[derive(Default)]
struct Rewrites {
    /// Node identity to the rewritten lexical form. The node is held so its
    /// allocation, and so its identity, cannot be reused while the overlay
    /// lives.
    by_node: HashMap<usize, (Arc<dyn Node>, String)>,
    /// A node this overlay handed out, to the identity of the node it stands
    /// for, so a second rewrite of the same element lands on the same entry.
    aliases: HashMap<usize, (Arc<dyn Node>, usize)>,
    /// A `@` literal's occurrence number to its rewritten lexical form.
    by_literal: HashMap<u64, String>,
}

impl Rewrites {
    fn is_empty(&self) -> bool {
        self.by_node.is_empty() && self.by_literal.is_empty()
    }
}

/// The next occurrence number. Process-wide, so a value that outlives its
/// evaluation -- returned to a caller and handed back by a host service, say
/// -- can never be taken for a literal of a later evaluation.
static NEXT_OCCURRENCE: AtomicU64 = AtomicU64::new(1);

/// A fresh identity for one evaluation of a `@` dateTime literal's node.
pub(super) fn new_occurrence() -> u64 {
    NEXT_OCCURRENCE.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    /// The overlay of the evaluation running on this thread, if any.
    static REWRITES: RefCell<Option<Rewrites>> = const { RefCell::new(None) };
}

/// Starts an empty overlay for its lifetime and restores the previous one.
pub(super) struct RewriteGuard(Option<Rewrites>);

impl RewriteGuard {
    pub(super) fn begin() -> Self {
        Self(REWRITES.with(|r| r.replace(Some(Rewrites::default()))))
    }
}

impl Drop for RewriteGuard {
    fn drop(&mut self) {
        let previous = self.0.take();
        REWRITES.with(|r| *r.borrow_mut() = previous);
    }
}

fn identity(node: &Arc<dyn Node>) -> usize {
    Arc::as_ptr(node).cast::<()>() as usize
}

/// A node as navigation should hand it out: the node itself, or a node that
/// reports the string an earlier ordered comparison in this evaluation wrote.
pub(super) fn apply(child: Value) -> Value {
    let Value::Node(node) = &child else {
        return child;
    };
    let key = identity(node);
    REWRITES.with(|r| {
        let mut slot = r.borrow_mut();
        let Some(rewrites) = slot.as_mut() else {
            return child.clone();
        };
        let Some((_, lexical)) = rewrites.by_node.get(&key) else {
            return child.clone();
        };
        let wrapper: Arc<dyn Node> = Arc::new(Rewritten {
            inner: Arc::clone(node),
            lexical: lexical.clone(),
        });
        rewrites
            .aliases
            .insert(identity(&wrapper), (Arc::clone(&wrapper), key));
        Value::Node(wrapper)
    })
}

/// `compareDateTimeElements`' `setTimeZoneZulu(true)` on an operand that is
/// already a `DateTimeType` finer than a day (`FHIRPathEngine.java:636-648`).
pub(super) fn rewrite_to_zulu(value: &Value) {
    if let Value::DateTime(d) = value {
        rewrite_literal(d.occurrence, &d.lexical);
        return;
    }
    let Value::Node(node) = value else {
        return;
    };
    if fhir_type(value) != "dateTime" {
        return;
    }
    let Some(lexical) = node.primitive().map(|p| p.lexical) else {
        return;
    };
    let Some(zulu) = temporal::zulu_lexical(&lexical) else {
        return;
    };
    let key = identity(node);
    REWRITES.with(|r| {
        let mut slot = r.borrow_mut();
        let Some(rewrites) = slot.as_mut() else {
            return;
        };
        let (held, key) = match rewrites.aliases.get(&key) {
            Some((wrapper, original)) => (Arc::clone(wrapper), *original),
            None => (Arc::clone(node), key),
        };
        let held = match rewrites.by_node.get(&key) {
            Some((original, _)) => Arc::clone(original),
            None => held,
        };
        rewrites.by_node.insert(key, (held, zulu));
    });
}

/// The literal half of [`rewrite_to_zulu`]: a `@` literal's value, by its
/// occurrence number. A value with none is not a literal and is compared as a
/// copy.
fn rewrite_literal(occurrence: u64, lexical: &str) {
    if occurrence == 0 {
        return;
    }
    let Some(zulu) = temporal::zulu_lexical(lexical) else {
        return;
    };
    REWRITES.with(|r| {
        if let Some(rewrites) = r.borrow_mut().as_mut() {
            rewrites.by_literal.insert(occurrence, zulu);
        }
    });
}

/// Every item of `values` as [`apply`] hands out a node, and a `@` literal's
/// value with the string an earlier ordered comparison in this evaluation
/// wrote. Nothing is looked up until a comparison has rewritten something.
pub(super) fn apply_all(values: &mut [Value]) {
    if values.is_empty() {
        return;
    }
    let pending = REWRITES.with(|r| r.borrow().as_ref().is_some_and(|w| !w.is_empty()));
    if !pending {
        return;
    }
    for value in values.iter_mut() {
        match value {
            Value::Node(_) => *value = apply(value.clone()),
            Value::DateTime(d) if d.occurrence != 0 => {
                let zulu = REWRITES.with(|r| {
                    r.borrow()
                        .as_ref()
                        .and_then(|w| w.by_literal.get(&d.occurrence).cloned())
                });
                if let Some(zulu) = zulu {
                    d.lexical = zulu;
                    d.offset_minutes = Some(0);
                }
            }
            _ => {}
        }
    }
}

/// A node whose primitive value is the rewritten string and which is
/// otherwise the node it stands for.
#[derive(Debug)]
struct Rewritten {
    inner: Arc<dyn Node>,
    lexical: String,
}

impl Node for Rewritten {
    fn type_name(&self) -> &str {
        self.inner.type_name()
    }

    fn is_resource(&self) -> bool {
        self.inner.is_resource()
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        self.inner.children(name)
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        self.inner.all_children()
    }

    fn primitive(&self) -> Option<Primitive> {
        self.inner.primitive().map(|mut p| {
            p.lexical.clone_from(&self.lexical);
            p
        })
    }
}
