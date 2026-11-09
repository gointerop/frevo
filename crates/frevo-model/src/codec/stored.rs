//! The stored form: a resource that is its bytes.
//!
//! Constitution rule 6.1 is the whole of this module: a read that returns a
//! resource unchanged must not parse or re-serialize it. So
//! [`Stored::write_to`] on an unedited resource is one `write_all` of the
//! buffer it was loaded with, and nothing on the path to it has looked at a
//! single JSON token.

use std::io;
use std::sync::{Arc, Mutex};

use super::doc::Doc;
use super::path::{join, parse_path};
use super::scan::{DecodeError, Meter, Span, escape_into};
use super::tables::{JsonForm, Tables, TypeKind};

/// Where an element's two spans live: its own JSON value and its `_field`
/// sibling. They are never merged, because a merged struct cannot represent
/// "element present, value absent" without an extra flag, and getting that flag
/// wrong is a silent wrong answer (`design.md` section 3).
#[derive(Debug, Clone)]
pub struct Located {
    /// The concrete FHIR type of the element's value.
    pub type_name: String,
    /// The span of the element's own JSON value, absent when the element exists
    /// only as a `_field` sibling.
    pub value: Option<Span>,
    /// Where the matching `_name` value is, or how to find it.
    pub sibling: Sibling,
    /// The indexed element path this is located at.
    pub path: String,
    /// The span of the whole repeating array, before indexing.
    pub array: Option<Span>,
    /// Whether the element repeats.
    pub repeating: bool,
}

/// The `_field` sibling, which is **not** looked up until something asks for
/// it.
///
/// Looking it up eagerly is the mistake that quietly destroys laziness: a
/// property that is not in an object can only be shown to be absent by scanning
/// the object to its closing brace, so an eager `_birthDate` lookup turns a read
/// of `Patient.birthDate` into a shallow scan of the whole `Patient` --
/// including the `address` array that `model.bytes.003` requires to stay
/// untouched. When the value *was* found nothing needs the sibling yet, so the
/// lookup is deferred to [`Stored::sibling_of`]; when the value was absent the
/// object has already been scanned to its end, so resolving the sibling then is
/// free.
#[derive(Debug, Clone)]
pub enum Sibling {
    /// Already known, because finding it cost nothing.
    Known(Option<Span>),
    /// Not looked up: the enclosing object, the element's base name, the
    /// choice type the value took when the element is a choice, and the index
    /// to take when it repeats.
    Deferred {
        owner: usize,
        base: &'static str,
        choice_type: Option<&'static str>,
        index: Option<usize>,
    },
}

impl Sibling {
    /// The span, when it is already known. `None` means "not looked up yet" as
    /// well as "absent", so a caller that needs the difference goes through
    /// [`Stored::sibling_of`].
    pub fn known(&self) -> Option<Span> {
        match self {
            Sibling::Known(span) => *span,
            Sibling::Deferred { .. } => None,
        }
    }
}

/// A resource backed by stored bytes, with a lazily built index and a list of
/// the spans a write has replaced.
#[derive(Debug)]
pub struct Stored {
    doc: Doc,
    tables: Tables,
    root_type: Mutex<Option<String>>,
    edits: Mutex<Vec<(Span, Vec<u8>)>>,
}

impl Stored {
    /// Take an already-owned buffer. No scan happens: this is the load step
    /// `model.bytes.002` requires to cost zero tokens.
    pub fn new(bytes: Arc<[u8]>, tables: Tables, meter: Arc<Meter>) -> Self {
        Self {
            doc: Doc::new(bytes, meter),
            tables,
            root_type: Mutex::new(None),
            edits: Mutex::new(Vec::new()),
        }
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    pub fn meter(&self) -> &Arc<Meter> {
        self.doc.meter()
    }

    pub fn is_modified(&self) -> bool {
        self.edits
            .lock()
            .map(|edits| !edits.is_empty())
            .unwrap_or(true)
    }

    /// The untouched stored buffer, when the resource is still exactly its
    /// bytes. `None` once a write has happened.
    pub fn untouched_bytes(&self) -> Option<Arc<[u8]>> {
        let edits = self.edits.lock().ok()?;
        if edits.is_empty() {
            Some(Arc::clone(self.doc.bytes()))
        } else {
            None
        }
    }

    /// The resource type, from `resourceType`. Reading it costs the two tokens
    /// of one property, and the answer is cached.
    pub fn root_type(&self) -> Result<String, DecodeError> {
        if let Ok(cached) = self.root_type.lock() {
            if let Some(name) = cached.as_ref() {
                return Ok(name.clone());
            }
        }
        let root = self.doc.root();
        if !self.doc.is_object(root) {
            return Err(DecodeError::new(
                "",
                root.start,
                "a FHIR resource must be a JSON object",
            ));
        }
        let span = self
            .doc
            .member(root.start, "resourceType")?
            .ok_or_else(|| DecodeError::new("", root.start, "resourceType is required"))?;
        let name = self.doc.lexical(span)?;
        if self.tables.type_def(&name).is_none() {
            return Err(DecodeError::new(
                "",
                span.start,
                format!("unknown resourceType: {name}"),
            ));
        }
        if let Ok(mut cached) = self.root_type.lock() {
            *cached = Some(name.clone());
        }
        Ok(name)
    }

    /// Resolve the declared type of a node to its concrete type. For the
    /// abstract `Resource` an element declares — a `contained` entry, or
    /// `Bundle.entry.resource` — that means reading the instance's
    /// `resourceType`.
    fn concrete_type(&self, declared: &str, span: Span) -> Result<String, DecodeError> {
        if self.tables.type_def(declared).map(|def| def.kind) != Some(TypeKind::AnyResource) {
            return Ok(declared.to_string());
        }
        let span = self
            .doc
            .member(span.start, "resourceType")?
            .ok_or_else(|| DecodeError::new("", span.start, "resourceType is required"))?;
        self.doc.lexical(span)
    }

    /// Locate the element at `path`. `Ok(None)` is an absent element, which is
    /// not an error (`model.nav.005`).
    pub fn locate(&self, path: &str) -> Result<Option<Located>, DecodeError> {
        let steps = parse_path(path)?;
        let root_type = self.root_type()?;
        let Some((first, rest)) = steps.split_first() else {
            return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
        };
        if first.name != root_type {
            return Err(DecodeError::new(
                path,
                0,
                format!(
                    "the path is rooted at {:?} but the resource is a {root_type:?}",
                    first.name
                ),
            ));
        }
        let root = self.doc.root();
        let mut located = Located {
            type_name: root_type,
            value: Some(root),
            sibling: Sibling::Known(None),
            path: first.name.clone(),
            array: None,
            repeating: false,
        };
        for step in rest {
            let Some(span) = located.value else {
                return Ok(None);
            };
            if !self.doc.is_object(span) {
                return Ok(None);
            }
            let owner = self.concrete_type(&located.type_name, span)?;
            let def = self.tables.element(&owner, &step.name).ok_or_else(|| {
                DecodeError::new(
                    path,
                    span.start,
                    format!("{owner} has no element named {:?}", step.name),
                )
            })?;

            let mut value = None;
            let mut value_type = def.types[0];
            let mut matched_choice: Option<&'static str> = None;
            if def.choice {
                let mut property = String::with_capacity(32);
                for candidate in def.types {
                    Tables::write_choice_property(def.name, candidate, &mut property);
                    if let Some(found) = self.doc.member(span.start, &property)? {
                        value = Some(found);
                        value_type = candidate;
                        matched_choice = Some(candidate);
                        break;
                    }
                }
            } else {
                value = self.doc.member_bytes(span.start, def.name.as_bytes())?;
            }
            let array = value.filter(|found| self.doc.is_array(*found));
            let mut index = None;
            let mut value_item = value;
            if def.repeating {
                let wanted = step.index.unwrap_or(0);
                index = Some(wanted);
                value_item = self.pick(value, wanted)?;
            }

            let sibling = if value.is_some() {
                Sibling::Deferred {
                    owner: span.start,
                    base: def.name,
                    choice_type: matched_choice,
                    index,
                }
            } else {
                let found = self.sibling_member(span.start, def.name, matched_choice)?;
                Sibling::Known(match index {
                    Some(wanted) => self.pick(found, wanted)?,
                    None => found,
                })
            };
            if value_item.is_none() && sibling.known().is_none() {
                return Ok(None);
            }

            located = Located {
                type_name: value_type.to_string(),
                value: value_item,
                sibling,
                path: join(&located.path, def.name, index),
                array,
                repeating: def.repeating,
            };
        }
        Ok(Some(located))
    }

    /// One item of a repeating element's JSON. A `null` hole means "nothing at
    /// this index" for that side (`design.md` section 3).
    fn pick(&self, value: Option<Span>, index: usize) -> Result<Option<Span>, DecodeError> {
        let Some(span) = value else {
            return Ok(None);
        };
        let item = if self.doc.is_array(span) {
            self.doc.item(span.start, index)?
        } else if index == 0 {
            Some(span)
        } else {
            None
        };
        Ok(item.filter(|found| !self.doc.is_null(*found)))
    }

    /// Read the primitive at `path`. Returns the concrete type name and the
    /// lexical value, which is `None` for an element that exists without a
    /// value — exactly what `hasValue()` reports.
    pub fn read(&self, path: &str) -> Result<Option<(String, Option<String>)>, DecodeError> {
        let Some(located) = self.locate(path)? else {
            return Ok(None);
        };
        let Some(span) = located.value else {
            return Ok(Some((located.type_name, None)));
        };
        if !self.tables.is_primitive(&located.type_name) {
            return Ok(Some((located.type_name, None)));
        }
        let lexical = self.doc.lexical(span)?;
        self.doc.meter().materialise(&located.path);
        Ok(Some((located.type_name, Some(lexical))))
    }

    /// How many repetitions `path` holds. Steps over the items; materialises
    /// none of them.
    pub fn count(&self, path: &str) -> Result<usize, DecodeError> {
        let Some(located) = self.locate(path)? else {
            return Ok(0);
        };
        match located.array {
            Some(array) => self.doc.item_count(array.start),
            None if located.value.is_some() => Ok(1),
            None => Ok(usize::from(self.sibling_of(&located)?.is_some())),
        }
    }

    /// Resolve an element's `_field` sibling, looking it up if that has not
    /// happened yet.
    pub fn sibling_of(&self, located: &Located) -> Result<Option<Span>, DecodeError> {
        match &located.sibling {
            Sibling::Known(span) => Ok(*span),
            Sibling::Deferred {
                owner,
                base,
                choice_type,
                index,
            } => {
                let found = self.sibling_member(*owner, base, *choice_type)?;
                match index {
                    Some(wanted) => self.pick(found, *wanted),
                    None => Ok(found),
                }
            }
        }
    }

    /// The `_name` property of an element, spelled with the choice type when
    /// the element is a choice.
    fn sibling_member(
        &self,
        owner: usize,
        base: &str,
        choice_type: Option<&str>,
    ) -> Result<Option<Span>, DecodeError> {
        let mut property = String::with_capacity(base.len() + 16);
        property.push('_');
        match choice_type {
            Some(concrete) => {
                let mut spelled = String::with_capacity(base.len() + concrete.len());
                Tables::write_choice_property(base, concrete, &mut spelled);
                property.push_str(&spelled);
            }
            None => property.push_str(base),
        }
        self.doc.member(owner, &property)
    }

    /// Replace the primitive at `path` with `lexical`. The replaced span is
    /// recorded; everything around it keeps its original bytes, which is why
    /// `model.perf.004` can require the 4 KiB and 64 KiB cases to be within
    /// 25% of each other.
    pub fn set(&self, path: &str, lexical: &str) -> Result<(), DecodeError> {
        let located = self
            .locate(path)?
            .ok_or_else(|| DecodeError::new(path, 0, "no such element to write"))?;
        let span = located
            .value
            .ok_or_else(|| DecodeError::new(path, 0, "the element has no value span to replace"))?;
        let mut encoded = Vec::new();
        match self.tables.json_form(&located.type_name) {
            JsonForm::Quoted => escape_into(lexical, &mut encoded),
            JsonForm::Boolean | JsonForm::Number => encoded.extend_from_slice(lexical.as_bytes()),
        }
        let mut edits = self
            .edits
            .lock()
            .map_err(|_| DecodeError::new(path, 0, "the write log lock was poisoned"))?;
        edits.retain(|(existing, _)| existing.start != span.start);
        edits.push((span, encoded));
        Ok(())
    }

    /// Decode the whole document, recording every element path it
    /// materialises. This is the `model-full-parse` workload.
    pub fn materialise_all(&self) -> Result<(), DecodeError> {
        let root = self.doc.root();
        let root_type = self.root_type()?;
        self.materialise_object(&root_type, root, &root_type.clone())
    }

    fn materialise_object(
        &self,
        type_name: &str,
        span: Span,
        path: &str,
    ) -> Result<(), DecodeError> {
        let concrete = self.concrete_type(type_name, span)?;
        if self.tables.type_def(&concrete).is_none() {
            // An unmodelled subtree is left as bytes rather than guessed at.
            return Ok(());
        }
        for (property, value) in self.doc.member_spans(span.start)? {
            let Some((element, element_type, _sibling)) = self
                .tables
                .resolve_property_bytes(&concrete, property.of(self.doc.bytes()))
            else {
                continue;
            };
            if element.repeating && self.doc.is_array(value) {
                let count = self.doc.item_count(value.start)?;
                for index in 0..count {
                    let Some(item) = self.doc.item(value.start, index)? else {
                        continue;
                    };
                    if self.doc.is_null(item) {
                        continue;
                    }
                    let child = self.child_path(path, element.name, Some(index));
                    self.materialise_value(element_type, item, &child)?;
                }
            } else {
                let child = self.child_path(path, element.name, element.repeating.then_some(0));
                self.materialise_value(element_type, value, &child)?;
            }
        }
        Ok(())
    }

    /// The path of a child, or the empty string when nothing is recording
    /// paths: formatting one costs an allocation per element, which is the
    /// whole of the gap between `model.lazy.*`'s instrumentation and
    /// `model.perf.003`'s decode budget.
    fn child_path(&self, parent: &str, name: &str, index: Option<usize>) -> String {
        if self.doc.meter().records_paths() {
            join(parent, name, index)
        } else {
            String::new()
        }
    }

    fn materialise_value(
        &self,
        type_name: &str,
        span: Span,
        path: &str,
    ) -> Result<(), DecodeError> {
        self.doc.meter().materialise(path);
        if self.tables.is_primitive(type_name) {
            // The value is decoded, but an `xhtml` div is carried as opaque
            // text: nothing here parses or validates it (`model.lazy.006`,
            // `model.json.008`).
            let _ = self.doc.lexical(span)?;
            return Ok(());
        }
        if self.doc.is_object(span) {
            self.materialise_object(type_name, span, path)?;
        }
        Ok(())
    }

    /// Exactly how many bytes [`Stored::write_to`] will emit. Knowing it lets a
    /// caller size its buffer once instead of growing it by doubling, which at
    /// 64 KiB is sixteen reallocations and about twice the bytes copied.
    pub fn stored_len(&self) -> usize {
        let base = self.doc.bytes().len();
        let Ok(edits) = self.edits.lock() else {
            return base;
        };
        edits.iter().fold(base, |total, (span, replacement)| {
            total + replacement.len() - span.len()
        })
    }

    /// Serialise in the stored form. With no write, this is the input buffer
    /// verbatim; with writes, it is the original bytes with each replaced span
    /// swapped out.
    pub fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        let bytes = self.doc.bytes();
        let edits = self
            .edits
            .lock()
            .map_err(|_| io::Error::other("the write log lock was poisoned"))?;
        if edits.is_empty() {
            return out.write_all(bytes);
        }
        let mut ordered: Vec<&(Span, Vec<u8>)> = edits.iter().collect();
        ordered.sort_by_key(|(span, _)| span.start);
        let mut cursor = 0usize;
        for (span, replacement) in ordered {
            if span.start < cursor {
                // Overlapping writes would make the output order-dependent;
                // refusing is better than emitting something plausible.
                return Err(io::Error::other(format!(
                    "two writes overlap at byte {}",
                    span.start
                )));
            }
            out.write_all(&bytes[cursor..span.start])?;
            out.write_all(replacement)?;
            cursor = span.end;
        }
        out.write_all(&bytes[cursor..])
    }

    /// Serialise canonically: every object's property names sorted
    /// lexicographically, no whitespace. Unlike the stored form this cannot be
    /// a byte copy, because the stored order is the generated element order.
    pub fn write_canonical(&self, out: &mut dyn io::Write) -> io::Result<()> {
        let mut buffer = Vec::new();
        let root = self.doc.root();
        self.canonicalise(root, &mut buffer)
            .map_err(|e| io::Error::other(e.to_string()))?;
        out.write_all(&buffer)
    }

    fn canonicalise(&self, span: Span, out: &mut Vec<u8>) -> Result<(), DecodeError> {
        if self.doc.is_object(span) {
            let mut members = self.doc.members(span.start)?;
            members.sort_by(|a, b| a.0.cmp(&b.0));
            out.push(b'{');
            for (position, (property, value)) in members.iter().enumerate() {
                if position > 0 {
                    out.push(b',');
                }
                escape_into(property, out);
                out.push(b':');
                self.canonicalise(*value, out)?;
            }
            out.push(b'}');
            return Ok(());
        }
        if self.doc.is_array(span) {
            let count = self.doc.item_count(span.start)?;
            out.push(b'[');
            for index in 0..count {
                if index > 0 {
                    out.push(b',');
                }
                match self.doc.item(span.start, index)? {
                    Some(item) => self.canonicalise(item, out)?,
                    None => out.extend_from_slice(b"null"),
                }
            }
            out.push(b']');
            return Ok(());
        }
        // A scalar keeps its presented form, so a decimal keeps its precision.
        out.extend_from_slice(span.of(self.doc.bytes()));
        Ok(())
    }

    /// The children of the node at `path`, as `(element name, concrete type,
    /// value span, sibling span)` in document order.
    pub fn children_of(&self, path: &str) -> Result<Vec<ChildSpan>, DecodeError> {
        let Some(located) = self.locate(path)? else {
            return Ok(Vec::new());
        };
        let Some(span) = located.value else {
            return Ok(Vec::new());
        };
        if !self.doc.is_object(span) {
            return Ok(Vec::new());
        }
        let owner = self.concrete_type(&located.type_name, span)?;
        let mut out = Vec::new();
        let mut siblings: Vec<String> = Vec::new();
        for (property, value) in self.doc.member_spans(span.start)? {
            let Some((element, element_type, is_sibling)) = self
                .tables
                .resolve_property_bytes(&owner, property.of(self.doc.bytes()))
            else {
                continue;
            };
            if is_sibling {
                siblings.push(element.name.to_string());
                continue;
            }
            if element.repeating && self.doc.is_array(value) {
                let count = self.doc.item_count(value.start)?;
                for index in 0..count {
                    let Some(item) = self.doc.item(value.start, index)? else {
                        continue;
                    };
                    out.push(ChildSpan {
                        name: element.name.to_string(),
                        type_name: element_type.to_string(),
                        value: (!self.doc.is_null(item)).then_some(item),
                        index: Some(index),
                    });
                }
            } else {
                out.push(ChildSpan {
                    name: element.name.to_string(),
                    type_name: element_type.to_string(),
                    value: Some(value),
                    index: element.repeating.then_some(0),
                });
            }
        }
        // An element that exists only as a `_field` sibling is still an
        // element, and still a child.
        for name in siblings {
            if out.iter().any(|child| child.name == name) {
                continue;
            }
            let Some(def) = self.tables.element(&owner, &name) else {
                continue;
            };
            out.push(ChildSpan {
                name,
                type_name: def.types[0].to_string(),
                value: None,
                index: def.repeating.then_some(0),
            });
        }
        // Document order is the order the members came back in: the generated
        // element order decides serialisation and nothing else. An element
        // that exists only as a `_field` sibling has no position among the
        // values, so it is appended.
        Ok(out)
    }

    /// Materialise a child's primitive value without recording it as a read of
    /// the parent.
    pub fn lexical_of(&self, span: Span) -> Result<String, DecodeError> {
        self.doc.lexical(span)
    }

    pub fn span_is_object(&self, span: Span) -> bool {
        self.doc.is_object(span)
    }

    pub fn resolve_concrete(&self, declared: &str, span: Span) -> Result<String, DecodeError> {
        self.concrete_type(declared, span)
    }
}

/// One child of a node, as spans rather than as a materialised value.
#[derive(Debug, Clone)]
pub struct ChildSpan {
    pub name: String,
    pub type_name: String,
    pub value: Option<Span>,
    pub index: Option<usize>,
}
