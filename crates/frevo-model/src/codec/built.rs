//! The constructed form: a resource that was never bytes.
//!
//! `model.bytes.005` is the scenario this module exists for. A resource built
//! in memory has no stored byte order to preserve, so serialising it has to
//! consult the **generated element order** — the position of an element in its
//! type's StructureDefinition snapshot — and nothing else. The oracle supplies
//! the properties in reverse order on purpose, so an implementation that
//! emitted insertion order would be caught.
//!
//! `model.bytes.006` is the other half: a reader cannot tell this form from the
//! stored one.

use super::path::{PathStep, parse_path};
use super::scan::{DecodeError, escape_into};
use super::tables::{ElementDef, JsonForm, Tables, TypeKind};

/// One node of a constructed resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltNode {
    Primitive {
        type_name: String,
        lexical: String,
    },
    Object {
        type_name: String,
        /// Keyed by **element base name**, in insertion order. The order is
        /// kept only so a diagnostic can report it; serialisation never uses
        /// it.
        props: Vec<(String, BuiltNode)>,
    },
    Array(Vec<BuiltNode>),
}

impl BuiltNode {
    pub fn type_name(&self) -> &str {
        match self {
            BuiltNode::Primitive { type_name, .. } | BuiltNode::Object { type_name, .. } => {
                type_name
            }
            BuiltNode::Array(items) => items.first().map(BuiltNode::type_name).unwrap_or(""),
        }
    }

    fn object_props(&mut self) -> Option<&mut Vec<(String, BuiltNode)>> {
        match self {
            BuiltNode::Object { props, .. } => Some(props),
            _ => None,
        }
    }
}

/// Every element of `type_name`, in generated element order: the base type's
/// elements first, then the derived type's own. `Patient` therefore reads
/// `id, meta, implicitRules, language` (from `Resource`), then
/// `text, contained, extension, modifierExtension` (from `DomainResource`),
/// then `identifier, active, name, telecom, gender, birthDate, ...`.
pub fn ordered_elements(tables: &Tables, type_name: &str) -> Vec<&'static ElementDef> {
    let mut chain = Vec::new();
    let mut current = Some(type_name);
    while let Some(name) = current {
        match tables.type_def(name) {
            Some(def) => {
                chain.push(def);
                current = def.base;
            }
            None => break,
        }
    }
    chain.reverse();
    chain.iter().flat_map(|def| def.elements.iter()).collect()
}

/// A resource under construction.
#[derive(Debug, Clone)]
pub struct Built {
    root: BuiltNode,
    tables: Tables,
}

impl Built {
    pub fn new(resource_type: &str, tables: Tables) -> Self {
        Self {
            root: BuiltNode::Object {
                type_name: resource_type.to_string(),
                props: Vec::new(),
            },
            tables,
        }
    }

    pub fn root_type(&self) -> &str {
        self.root.type_name()
    }

    pub fn root(&self) -> &BuiltNode {
        &self.root
    }

    /// Set the primitive at `path` to `lexical`, creating the intermediate
    /// elements the path names. `path` is rooted at the resource type.
    pub fn set(&mut self, path: &str, lexical: &str) -> Result<(), DecodeError> {
        let steps = parse_path(path)?;
        let Some((first, rest)) = steps.split_first() else {
            return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
        };
        if first.name != self.root.type_name() {
            return Err(DecodeError::new(
                path,
                0,
                format!(
                    "the path is rooted at {:?} but the resource is a {:?}",
                    first.name,
                    self.root.type_name()
                ),
            ));
        }
        if rest.is_empty() {
            return Err(DecodeError::new(
                path,
                0,
                "a path with no element steps names the resource, not a value",
            ));
        }
        let tables = self.tables;
        Self::set_in(&tables, &mut self.root, rest, lexical, path)
    }

    fn set_in(
        tables: &Tables,
        node: &mut BuiltNode,
        steps: &[PathStep],
        lexical: &str,
        path: &str,
    ) -> Result<(), DecodeError> {
        let Some((step, rest)) = steps.split_first() else {
            return Ok(());
        };
        let owner = node.type_name().to_string();
        let def = tables.element(&owner, &step.name).ok_or_else(|| {
            DecodeError::new(
                path,
                0,
                format!("{owner} has no element named {:?}", step.name),
            )
        })?;
        // A constructed value is given by lexical form, so a choice element
        // takes its first declared type unless the path spelled one.
        let concrete = def.types[0];
        let leaf = rest.is_empty();
        let props = node.object_props().ok_or_else(|| {
            DecodeError::new(path, 0, format!("{owner} is not an object to set into"))
        })?;

        if !props.iter().any(|(name, _)| *name == def.name) {
            let fresh = if def.repeating {
                BuiltNode::Array(Vec::new())
            } else {
                Self::fresh(tables, concrete, leaf, lexical)
            };
            props.push((def.name.to_string(), fresh));
        }
        let slot = props
            .iter_mut()
            .find(|(name, _)| *name == def.name)
            .map(|(_, value)| value)
            .ok_or_else(|| DecodeError::new(path, 0, "the property vanished"))?;

        let target = if def.repeating {
            let index = step.index.unwrap_or(0);
            let BuiltNode::Array(items) = slot else {
                return Err(DecodeError::new(
                    path,
                    0,
                    format!("{}.{} is repeating but is not an array", owner, def.name),
                ));
            };
            while items.len() <= index {
                items.push(Self::fresh(tables, concrete, leaf, lexical));
            }
            &mut items[index]
        } else {
            slot
        };

        if leaf {
            *target = BuiltNode::Primitive {
                type_name: concrete.to_string(),
                lexical: lexical.to_string(),
            };
            return Ok(());
        }
        if !matches!(target, BuiltNode::Object { .. }) {
            *target = BuiltNode::Object {
                type_name: concrete.to_string(),
                props: Vec::new(),
            };
        }
        Self::set_in(tables, target, rest, lexical, path)
    }

    fn fresh(tables: &Tables, concrete: &str, leaf: bool, lexical: &str) -> BuiltNode {
        if leaf && tables.is_primitive(concrete) {
            BuiltNode::Primitive {
                type_name: concrete.to_string(),
                lexical: lexical.to_string(),
            }
        } else {
            BuiltNode::Object {
                type_name: concrete.to_string(),
                props: Vec::new(),
            }
        }
    }

    /// The primitive at `path`, with the type it was declared as. `Ok(None)`
    /// is an absent element, which is not an error (`model.nav.005`).
    pub fn read(&self, path: &str) -> Result<Option<(String, Option<String>)>, DecodeError> {
        let steps = parse_path(path)?;
        let Some((first, rest)) = steps.split_first() else {
            return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
        };
        if first.name != self.root.type_name() {
            return Ok(None);
        }
        let mut node = &self.root;
        for step in rest {
            let owner = node.type_name();
            let Some(def) = self.tables.element(owner, &step.name) else {
                return Err(DecodeError::new(
                    path,
                    0,
                    format!("{owner} has no element named {:?}", step.name),
                ));
            };
            let BuiltNode::Object { props, .. } = node else {
                return Ok(None);
            };
            let Some((_, value)) = props.iter().find(|(name, _)| *name == def.name) else {
                return Ok(None);
            };
            node = match value {
                BuiltNode::Array(items) => match items.get(step.index.unwrap_or(0)) {
                    Some(item) => item,
                    None => return Ok(None),
                },
                other => other,
            };
        }
        match node {
            BuiltNode::Primitive { type_name, lexical } => {
                Ok(Some((type_name.clone(), Some(lexical.clone()))))
            }
            other => Ok(Some((other.type_name().to_string(), None))),
        }
    }

    /// How many repetitions `path` holds.
    pub fn count(&self, path: &str) -> Result<usize, DecodeError> {
        let steps = parse_path(path)?;
        let Some((first, rest)) = steps.split_first() else {
            return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
        };
        if first.name != self.root.type_name() {
            return Ok(0);
        }
        let mut node = &self.root;
        for (position, step) in rest.iter().enumerate() {
            let owner = node.type_name();
            let Some(def) = self.tables.element(owner, &step.name) else {
                return Err(DecodeError::new(
                    path,
                    0,
                    format!("{owner} has no element named {:?}", step.name),
                ));
            };
            let BuiltNode::Object { props, .. } = node else {
                return Ok(0);
            };
            let Some((_, value)) = props.iter().find(|(name, _)| *name == def.name) else {
                return Ok(0);
            };
            let last = position + 1 == rest.len();
            if last {
                return Ok(match value {
                    BuiltNode::Array(items) => items.len(),
                    _ => 1,
                });
            }
            node = match value {
                BuiltNode::Array(items) => match items.get(step.index.unwrap_or(0)) {
                    Some(item) => item,
                    None => return Ok(0),
                },
                other => other,
            };
        }
        Ok(0)
    }

    /// Serialise in generated element order.
    pub fn write(&self, out: &mut Vec<u8>) {
        write_node(&self.tables, &self.root, out, false);
    }

    /// Serialise canonically: property names sorted lexicographically, no
    /// whitespace, which is what `JsonCreatorCanonical` produces.
    pub fn write_canonical(&self, out: &mut Vec<u8>) {
        write_node(&self.tables, &self.root, out, true);
    }
}

fn write_node(tables: &Tables, node: &BuiltNode, out: &mut Vec<u8>, canonical: bool) {
    match node {
        BuiltNode::Primitive { type_name, lexical } => match tables.json_form(type_name) {
            JsonForm::Quoted => escape_into(lexical, out),
            JsonForm::Boolean | JsonForm::Number => out.extend_from_slice(lexical.as_bytes()),
        },
        BuiltNode::Array(items) => {
            out.push(b'[');
            for (position, item) in items.iter().enumerate() {
                if position > 0 {
                    out.push(b',');
                }
                write_node(tables, item, out, canonical);
            }
            out.push(b']');
        }
        BuiltNode::Object { type_name, props } => {
            let mut entries: Vec<(String, Option<&BuiltNode>)> = Vec::new();
            let is_resource =
                tables.type_def(type_name).map(|def| def.kind) == Some(TypeKind::Resource);
            if is_resource {
                entries.push((String::from("resourceType"), None));
            }
            let mut emitted = Vec::new();
            for def in ordered_elements(tables, type_name) {
                if let Some((_, value)) = props.iter().find(|(name, _)| *name == def.name) {
                    let property = if def.choice {
                        Tables::choice_property(def.name, value.type_name())
                    } else {
                        def.name.to_string()
                    };
                    entries.push((property, Some(value)));
                    emitted.push(def.name);
                }
            }
            // An element the tables do not know cannot be ordered, so it goes
            // last rather than being dropped: losing an element silently is
            // the failure mode constitution rule 4 is about.
            for (name, value) in props {
                if !emitted.iter().any(|known| known == name) {
                    entries.push((name.clone(), Some(value)));
                }
            }
            if canonical {
                entries.sort_by(|a, b| a.0.cmp(&b.0));
            }
            out.push(b'{');
            for (position, (property, value)) in entries.iter().enumerate() {
                if position > 0 {
                    out.push(b',');
                }
                escape_into(property, out);
                out.push(b':');
                match value {
                    None => escape_into(type_name, out),
                    Some(value) => write_node(tables, value, out, canonical),
                }
            }
            out.push(b'}');
        }
    }
}
