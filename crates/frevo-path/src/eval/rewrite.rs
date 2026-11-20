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
//! dropped when the evaluation ends.
//!
//! What is kept and what is not:
//!
//! - only an **element** of FHIR type `dateTime` is rewritten. A `date`
//!   (`DateType`) and an `instant` (`InstantType`) are not `DateTimeType`s and
//!   are copied upstream, so they are never rewritten;
//! - a `dateTime` **literal** is a constant object of the parsed expression
//!   upstream and would be rewritten too, for every later evaluation of that
//!   parsed expression. A literal here is a value, not a node, so it is not;
//!   for a literal that already carries `Z` the two agree, and the difference
//!   is a literal with a non-Zulu offset or none, read back after an ordered
//!   comparison;
//! - identity is the node's allocation. A model that hands out a fresh node
//!   for the same element on every navigation does not see the rewrite; the
//!   harness model and an in-memory tree do.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

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
