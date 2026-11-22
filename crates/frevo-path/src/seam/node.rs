//! `Node` — navigation.
//!
//! `design.md` section 4.2 of `add-fhirpath-and-codegen-r4`. Definitions only.

use std::error::Error;
use std::fmt;

use super::value::{Primitive, Value};

/// Navigation failed. A decode error must propagate as an error rather than as
/// an empty collection, which is why every navigation method returns `Result`
/// and not `Option`: an `Option` return would make a malformed document
/// indistinguishable from an absent element, and FHIRPath would turn that into
/// an empty collection — the silent-wrong-answer mode (`path.errors.005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeError {
    /// The element path the failure happened at.
    pub path: String,
    /// The byte offset in the source document, when there is one.
    pub offset: Option<usize>,
    pub message: String,
}

impl NodeError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            offset: None,
            message: message.into(),
        }
    }

    pub fn at(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)?;
        if let Some(offset) = self.offset {
            write!(f, " (at byte {offset})")?;
        }
        Ok(())
    }
}

impl Error for NodeError {}

/// A FHIR element or resource, navigable without being materialised.
///
/// Four properties of this signature are load-bearing, and `model.nav.*`
/// grades each of them:
///
/// - **owned returns** — `Vec<Value>`, not `&dyn`. A borrow with the
///   receiver's lifetime would force the model to hold a materialised value to
///   lend a reference to, which ADR 0004 says would have ended the lazy codec;
/// - **`Result`, not `Option`** — see [`NodeError`];
/// - **`children(name)` takes a base name** — the engine never constructs
///   `valueQuantity`, so the polymorphic rule lives in the model's element
///   definitions and `Observation.valueQuantity` yields nothing at runtime;
/// - **no `resource()` / `root_resource()`** — `%resource` and `%rootResource`
///   are evaluation-context state, updated when traversal enters a node whose
///   [`Node::is_resource`] is true. Putting them on the node would require
///   every node to know its ancestors, which a span-based handle does not.
pub trait Node: fmt::Debug + Send + Sync {
    /// The FHIR type name of this node: `Patient`, `HumanName`, `code`.
    fn type_name(&self) -> &str;

    /// Whether this node is a resource. `%resource` tracking needs it.
    fn is_resource(&self) -> bool;

    /// The children under `name`, in document order, owned. `name` is an
    /// element name with no `[x]` suffix.
    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError>;

    /// Every child, paired with its element name, in document order. Backs
    /// `children()` and `descendants()`.
    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError>;

    /// This node's primitive value, or `None` when it has no value — which is
    /// exactly what `hasValue()` reports.
    fn primitive(&self) -> Option<Primitive>;
}
