//! `Node` and `Model` over the span-based codec.
//!
//! The traits are `frevo-path`'s (`design.md` section 4): the dependency edge
//! runs from here to there and never the other way, which is what
//! `path.api.008` and the tail of `model.nav.012` grade.
//!
//! A [`ResourceNode`] is the element handle of `design.md` section 3: an
//! `Arc<[u8]>`, the span of the element's own value, the span of its `_field`
//! sibling, and the element definition. Cloning one is an atomic increment plus
//! two ranges, which is why [`frevo_path::Node`]'s returns can be **owned** and
//! still cheap, and why a child outlives the node it came from
//! (`model.nav.007`).

use std::sync::Arc;

use frevo_path::{Model, Node, NodeError, Primitive, TypeSet, Value};

use super::scan::{DecodeError, Span};
use super::stored::Stored;
use super::tables::Tables;

fn to_node_error(path: &str, error: DecodeError) -> NodeError {
    NodeError::new(
        if error.path.is_empty() {
            path.to_string()
        } else {
            error.path.clone()
        },
        error.message,
    )
    .at(error.offset)
}

/// A navigable handle on one element or resource.
#[derive(Debug, Clone)]
pub struct ResourceNode {
    stored: Arc<Stored>,
    type_name: String,
    path: String,
    value: Option<Span>,
    sibling: Option<Span>,
}

impl ResourceNode {
    /// A handle on the root resource.
    pub fn root(stored: Arc<Stored>) -> Result<Self, DecodeError> {
        let type_name = stored.root_type()?;
        let located = stored
            .locate(&type_name)?
            .ok_or_else(|| DecodeError::new(&type_name, 0, "the root resource is absent"))?;
        let sibling = stored.sibling_of(&located)?;
        Ok(Self {
            stored,
            path: type_name,
            type_name: located.type_name,
            value: located.value,
            sibling,
        })
    }

    /// A handle on the element at `path`, which is rooted at the resource type.
    pub fn at(stored: Arc<Stored>, path: &str) -> Result<Option<Self>, DecodeError> {
        let Some(located) = stored.locate(path)? else {
            return Ok(None);
        };
        let concrete = match located.value {
            Some(span) if stored.span_is_object(span) => {
                stored.resolve_concrete(&located.type_name, span)?
            }
            _ => located.type_name.clone(),
        };
        let sibling = stored.sibling_of(&located)?;
        Ok(Some(Self {
            stored,
            type_name: concrete,
            path: located.path,
            value: located.value,
            sibling,
        }))
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn value_span(&self) -> Option<Span> {
        self.value
    }

    pub fn sibling_span(&self) -> Option<Span> {
        self.sibling
    }

    fn child_nodes(&self) -> Result<Vec<(String, ResourceNode)>, NodeError> {
        let children = self
            .stored
            .children_of(&self.path)
            .map_err(|e| to_node_error(&self.path, e))?;
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let concrete = match child.value {
                Some(span) if self.stored.span_is_object(span) => self
                    .stored
                    .resolve_concrete(&child.type_name, span)
                    .map_err(|e| to_node_error(&self.path, e))?,
                _ => child.type_name.clone(),
            };
            let path = match child.index {
                Some(index) => format!("{}.{}[{index}]", self.path, child.name),
                None => format!("{}.{}", self.path, child.name),
            };
            out.push((
                child.name,
                ResourceNode {
                    stored: Arc::clone(&self.stored),
                    type_name: concrete,
                    path,
                    value: child.value,
                    sibling: None,
                },
            ));
        }
        Ok(out)
    }
}

impl Node for ResourceNode {
    fn type_name(&self) -> &str {
        &self.type_name
    }

    fn is_resource(&self) -> bool {
        self.stored.tables().is_resource_type(&self.type_name)
    }

    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(self
            .child_nodes()?
            .into_iter()
            .filter(|(child_name, _)| child_name == name)
            .map(|(_, node)| Value::Node(Arc::new(node) as Arc<dyn Node>))
            .collect())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(self
            .child_nodes()?
            .into_iter()
            .map(|(name, node)| (name, Value::Node(Arc::new(node) as Arc<dyn Node>)))
            .collect())
    }

    fn primitive(&self) -> Option<Primitive> {
        if !self.stored.tables().is_primitive(&self.type_name) {
            return None;
        }
        let span = self.value?;
        let lexical = self.stored.lexical_of(span).ok()?;
        Some(Primitive {
            type_name: self.type_name.clone(),
            lexical,
            // `Element.id` and `Element.extension` from the `_field` sibling
            // arrive with task 22; the sibling span is already carried.
            id: None,
            extensions: Vec::new(),
        })
    }
}

/// The R4 model information: type hierarchy and element types.
#[derive(Debug, Clone, Copy)]
pub struct R4Model {
    tables: Tables,
}

impl R4Model {
    pub fn new(tables: Tables) -> Self {
        Self { tables }
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    /// The declared types of an element, as a [`TypeSet`] in declared order.
    pub fn element_type_set(&self, type_name: &str, element: &str) -> Option<TypeSet> {
        self.element_types(type_name, element)
            .map(TypeSet::from_iter_names)
    }
}

impl Default for R4Model {
    fn default() -> Self {
        Self::new(super::R4)
    }
}

impl Model for R4Model {
    fn is_type(&self, type_name: &str) -> bool {
        self.tables.type_def(type_name).is_some()
    }

    fn base_type(&self, type_name: &str) -> Option<&str> {
        self.tables.base_type(type_name)
    }

    fn element_types(&self, type_name: &str, element: &str) -> Option<Vec<String>> {
        self.tables
            .element(type_name, element)
            .map(|def| def.types.iter().map(|name| name.to_string()).collect())
    }

    fn is_primitive(&self, type_name: &str) -> bool {
        self.tables.is_primitive(type_name)
    }

    fn is_resource_type(&self, type_name: &str) -> bool {
        self.tables.is_resource_type(type_name)
    }

    fn derives_from(&self, type_name: &str, ancestor: &str) -> bool {
        self.tables.derives_from(type_name, ancestor)
    }
}
