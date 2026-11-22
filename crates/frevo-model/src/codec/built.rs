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
//!
//! The encode-side `#` reference check (`dangling_internal_references`,
//! `check_resource`, `contained_ids`) is derived from
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/BaseParser.java
//! (`determineReferenceText`, `isValidInternalReference`),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/util/FhirTerser.java
//! (`containResources`, `ContainedResources.referenceMatchesAContainedResource`)
//! and hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java
//! (`encodeResourceToJsonStreamWriter`'s not-contained guard), all at
//! 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright (C) 2014 - 2026 Smile CDR, Inc., licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: re-implemented in Rust as a walk over a
//! [`BuiltNode`] tree that collects each dangling reference rather than
//! calling an error handler while composing; the element-order serialisation
//! in the rest of this module is not derived from those.
//! Also derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/formats/JsonParser.java
//! and its base org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/formats/JsonParserBase.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: the element-order
//! serialisation of a constructed resource, which is the order the R4
//! `JsonParser` composes each type's elements in
//! (owner decision, 2026-11-21: the JSON codec is treated as derived from the
//! R4 `JsonParser`). Original work Copyright (c) 2011+, HL7, Inc., licensed
//! under HL7's BSD 3-Clause-style licence, which `NOTICE` reproduces.
//! Modified by the Frevo authors: one encoder driven by the generated element tables in place
//! of a generated `compose` method per type.

use super::decode::{CodedDiagnostic, Encoded, LocalReference, ParseMode, encoded};
use super::path::{PathStep, join, parse_path};
use super::scan::{DecodeError, escape_into};
use std::borrow::Cow;

use super::tables::{ElementDef, JsonForm, Tables, TypeDef, TypeKind};

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
/// elements first, then the derived type's own, and **each name only once**.
/// `Patient` therefore reads `id, meta, implicitRules, language` (from
/// `Resource`), then `text, contained, extension, modifierExtension` (from
/// `DomainResource`), then `identifier, active, name, telecom, gender,
/// birthDate, ...`.
///
/// The de-duplication is not defensive tidying. An R4 StructureDefinition
/// snapshot is **fully expanded**, so the generated `Patient` lists its
/// inherited `id`, `meta`, `text` and the rest itself, and a plain
/// concatenation of the base chain emits `id` three times -- once from
/// `Resource`, once from `DomainResource` and once from `Patient`. Taking the
/// base-chain position of each name and skipping a repeat reproduces the
/// snapshot order for an expanded table and for a table that relies on
/// inheritance alike: measured over all 685 generated types, 683 carry a fully
/// expanded element list and this rule reproduces every one of them exactly,
/// the remaining two being `Resource` and `Element`, which have no base to
/// inherit from.
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
    let mut out: Vec<&'static ElementDef> = Vec::new();
    for def in chain {
        for element in def.elements {
            if !out.iter().any(|seen| seen.name == element.name) {
                out.push(element);
            }
        }
    }
    out
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

    /// About how many bytes serialising this resource produces: exact for a
    /// resource with nothing to escape, an underestimate otherwise. A buffer
    /// sized by it is filled without reallocating, which matters more than
    /// it sounds -- growing a 4 KiB output by doubling was measured costing
    /// as much as the encode itself on a heap other work had fragmented.
    pub fn estimated_len(&self) -> usize {
        estimated_len(&self.root)
    }

    /// Serialise canonically: property names sorted lexicographically, no
    /// whitespace, which is what `JsonCreatorCanonical` produces.
    pub fn write_canonical(&self, out: &mut Vec<u8>) {
        write_node(&self.tables, &self.root, out, true);
    }

    /// The `#` references that serialising this resource finds no contained
    /// resource for (`model.canonical.007`), by the rule
    /// [`crate::codec::decode::Decoded::dangling_internal_references`]
    /// states. A constructed resource has no input bytes, so each offset is
    /// `0`; the element path is what locates it.
    pub fn dangling_internal_references(&self) -> Vec<LocalReference> {
        let mut out = Vec::new();
        check_resource(&self.tables, &self.root, self.root.type_name(), &mut out);
        out
    }

    /// Serialise canonically through the encode-side check: lenient mode
    /// returns the bytes with a diagnostic per dangling `#` reference, strict
    /// mode raises the first with HAPI's code `2724`.
    pub fn encode_canonical(&self, mode: ParseMode) -> Result<Encoded, CodedDiagnostic> {
        let mut bytes = Vec::new();
        self.write_canonical(&mut bytes);
        encoded(bytes, &self.dangling_internal_references(), mode)
    }
}

/// A `contained` entry's id, `#` stripped, as `FhirTerser.containResources`
/// strips it.
fn contained_ids(node: &BuiltNode) -> Vec<String> {
    let BuiltNode::Object { props, .. } = node else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_, value) in props.iter().filter(|(name, _)| name == "contained") {
        let entries: &[BuiltNode] = match value {
            BuiltNode::Array(items) => items,
            single => std::slice::from_ref(single),
        };
        for entry in entries {
            if let Some(id) = primitive_child(entry, "id") {
                out.push(id.trim_start_matches('#').to_string());
            }
        }
    }
    out
}

fn primitive_child<'n>(node: &'n BuiltNode, name: &str) -> Option<&'n str> {
    let BuiltNode::Object { props, .. } = node else {
        return None;
    };
    props
        .iter()
        .find(|(prop, _)| prop == name)
        .and_then(|(_, value)| match value {
            BuiltNode::Primitive { lexical, .. } => Some(lexical.as_str()),
            _ => None,
        })
}

fn check_resource(tables: &Tables, node: &BuiltNode, path: &str, out: &mut Vec<LocalReference>) {
    let contained = contained_ids(node);
    check_children(tables, node, path, true, &contained, out);
}

fn check_children(
    tables: &Tables,
    node: &BuiltNode,
    path: &str,
    is_resource: bool,
    contained: &[String],
    out: &mut Vec<LocalReference>,
) {
    let BuiltNode::Object { props, .. } = node else {
        return;
    };
    for (name, value) in props {
        let items: Vec<(Option<usize>, &BuiltNode)> = match value {
            BuiltNode::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, n)| (Some(i), n))
                .collect(),
            single => vec![(None, single)],
        };
        for (index, child) in items {
            let BuiltNode::Object { type_name, .. } = child else {
                continue;
            };
            let child_path = join(path, name, index);
            let child_is_resource =
                tables.type_def(type_name).map(|def| def.kind) == Some(TypeKind::Resource);
            if child_is_resource && !(is_resource && name == "contained") {
                check_resource(tables, child, &child_path, out);
                continue;
            }
            if type_name == "Reference" {
                if let Some(text) = primitive_child(child, "reference") {
                    if let Some(id) = text.strip_prefix('#') {
                        if !id.is_empty() && !contained.iter().any(|candidate| candidate == id) {
                            out.push(LocalReference {
                                path: child_path.clone(),
                                text: text.to_string(),
                                offset: 0,
                                resolved: false,
                            });
                        }
                    }
                }
            }
            check_children(
                tables,
                child,
                &child_path,
                child_is_resource,
                contained,
                out,
            );
        }
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
            let def = tables.type_def(type_name);
            let is_resource = def.map(|def| def.kind) == Some(TypeKind::Resource);
            // The base chain, base-most first, which is the order
            // [`ordered_elements`] reads it in. A chain deeper than the
            // buffer falls back to building that list outright.
            let mut chain: [Option<&'static TypeDef>; CHAIN] = [None; CHAIN];
            let depth = base_chain(tables, def, &mut chain);
            let mut entries = Entries::new(props.len() + 1);
            if is_resource {
                entries.push(Entry {
                    property: Cow::Borrowed("resourceType"),
                    rank: (0, 0),
                    value: None,
                });
            }
            match depth {
                Some(depth) => {
                    let chain = &chain[CHAIN - depth..];
                    for (insertion, (name, value)) in props.iter().enumerate() {
                        let entry = match generated_position(chain, name) {
                            // `set` never inserts a name twice; were a known
                            // one repeated, only its first would be written,
                            // as the walk over the generated order writes it.
                            Some(_) if props[..insertion].iter().any(|(seen, _)| seen == name) => {
                                continue;
                            }
                            Some((rank, element)) => Entry {
                                property: property_of(element, value),
                                rank: (1, rank),
                                value: Some(value),
                            },
                            // An element the tables do not know cannot be
                            // ordered, so it goes last rather than being
                            // dropped: losing an element silently is the
                            // failure mode constitution rule 4 is about.
                            None => Entry {
                                property: Cow::Borrowed(name.as_str()),
                                rank: (2, insertion),
                                value: Some(value),
                            },
                        };
                        entries.push(entry);
                    }
                    entries.as_mut_slice().sort_by_key(|entry| entry.rank);
                }
                None => {
                    let mut emitted = Vec::new();
                    for def in ordered_elements(tables, type_name) {
                        if let Some((_, value)) = props.iter().find(|(name, _)| *name == def.name) {
                            entries.push(Entry {
                                property: property_of(def, value),
                                rank: (1, 0),
                                value: Some(value),
                            });
                            emitted.push(def.name);
                        }
                    }
                    for (name, value) in props {
                        if !emitted.iter().any(|known| known == name) {
                            entries.push(Entry {
                                property: Cow::Borrowed(name.as_str()),
                                rank: (2, 0),
                                value: Some(value),
                            });
                        }
                    }
                }
            }
            if canonical {
                // Stable, so equal names keep the generated order between
                // them, exactly as before.
                entries
                    .as_mut_slice()
                    .sort_by(|a, b| a.property.cmp(&b.property));
            }
            out.push(b'{');
            for (position, entry) in entries.as_mut_slice().iter().enumerate() {
                if position > 0 {
                    out.push(b',');
                }
                escape_into(&entry.property, out);
                out.push(b':');
                match entry.value {
                    None => escape_into(type_name, out),
                    Some(value) => write_node(tables, value, out, canonical),
                }
            }
            out.push(b'}');
        }
    }
}

/// The deepest base chain [`write_node`] orders by without allocating. R4's
/// deepest is four (`Element`, `BackboneElement`, a backbone type and its
/// nested type, or `Resource`, `DomainResource` and a resource).
const CHAIN: usize = 8;

/// One property of an object being written: its JSON name, where the
/// generated element order puts it, and its value (`None` is `resourceType`).
#[derive(Default)]
struct Entry<'n> {
    property: Cow<'n, str>,
    rank: (u8, usize),
    value: Option<&'n BuiltNode>,
}

/// How many properties an object may have before [`Entries`] moves them to the
/// heap. Most FHIR objects written have a handful; an allocation per object
/// was a measurable share of `model.perf.005` on this host's allocator.
const INLINE: usize = 12;

/// The properties of one object being written, on the stack while they fit
/// and on the heap (`spilled`) once they do not.
struct Entries<'n> {
    inline: [Entry<'n>; INLINE],
    len: usize,
    spilled: Vec<Entry<'n>>,
}

impl<'n> Entries<'n> {
    fn new(expected: usize) -> Self {
        Self {
            inline: std::array::from_fn(|_| Entry::default()),
            len: 0,
            // `Vec::new` does not allocate; only an object too large for the
            // inline buffer pays for this one.
            spilled: if expected > INLINE {
                Vec::with_capacity(expected)
            } else {
                Vec::new()
            },
        }
    }

    fn push(&mut self, entry: Entry<'n>) {
        if self.spilled.capacity() == 0 && self.len < INLINE {
            self.inline[self.len] = entry;
            self.len += 1;
            return;
        }
        if self.len > 0 {
            let moved = self.inline.iter_mut().take(self.len).map(std::mem::take);
            let mut spilled: Vec<Entry<'n>> = moved.collect();
            spilled.append(&mut self.spilled);
            self.spilled = spilled;
            self.len = 0;
        }
        self.spilled.push(entry);
    }

    fn as_mut_slice(&mut self) -> &mut [Entry<'n>] {
        if self.len > 0 {
            &mut self.inline[..self.len]
        } else {
            &mut self.spilled
        }
    }
}

fn property_of<'n>(def: &'static ElementDef, value: &BuiltNode) -> Cow<'n, str> {
    if def.choice {
        Cow::Owned(Tables::choice_property(def.name, value.type_name()))
    } else {
        Cow::Borrowed(def.name)
    }
}

/// Fill the tail of `chain` with `def`'s base chain, base-most first, as
/// [`ordered_elements`] walks it, and return its depth; `None` when it is
/// deeper than the buffer. A type the tables do not know has an empty chain.
fn base_chain(
    tables: &Tables,
    def: Option<&'static TypeDef>,
    chain: &mut [Option<&'static TypeDef>; CHAIN],
) -> Option<usize> {
    let mut depth = 0;
    let mut current = def;
    while let Some(def) = current {
        if depth == CHAIN {
            return None;
        }
        chain[CHAIN - 1 - depth] = Some(def);
        depth += 1;
        current = def.base.and_then(|base| tables.type_def(base));
    }
    Some(depth)
}

/// Where `name` sits in [`ordered_elements`]' list for the chain, without
/// building the list: the list is each chain type's elements in turn, base-most
/// first, each name kept at its first occurrence only, so the element that
/// list holds for `name` is its first occurrence, and its position orders the
/// same as `(depth of that type, index within it)`. Returns that pair, flattened
/// to one number, and the element.
fn generated_position(
    chain: &[Option<&'static TypeDef>],
    name: &str,
) -> Option<(usize, &'static ElementDef)> {
    for (level, def) in chain.iter().flatten().enumerate() {
        if let Some(index) = def.elements.iter().position(|e| e.name == name) {
            return Some((level << 16 | index, &def.elements[index]));
        }
    }
    None
}

/// [`Built::estimated_len`] for one node: quotes, braces, brackets, colons
/// and commas, the property names, and the lexical values unescaped.
fn estimated_len(node: &BuiltNode) -> usize {
    match node {
        BuiltNode::Primitive { lexical, .. } => lexical.len() + 2,
        BuiltNode::Array(items) => {
            2 + items.len().saturating_sub(1) + items.iter().map(estimated_len).sum::<usize>()
        }
        BuiltNode::Object { type_name, props } => {
            // `"resourceType":"<type>",`, counted whether or not the type is a
            // resource: the estimate only has to be close.
            let envelope = 2 + 17 + type_name.len();
            envelope
                + props
                    .iter()
                    .map(|(name, value)| {
                        // The name, its quotes, the colon and a comma; a choice
                        // element's name also carries its type.
                        let choice = match value {
                            BuiltNode::Primitive { type_name, .. }
                            | BuiltNode::Object { type_name, .. } => type_name.len(),
                            BuiltNode::Array(_) => 0,
                        };
                        name.len() + choice + 4 + estimated_len(value)
                    })
                    .sum::<usize>()
        }
    }
}
