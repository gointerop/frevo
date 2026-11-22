//! The stored form: a resource that is its bytes.
//!
//! Constitution rule 6.1 is the whole of this module: a read that returns a
//! resource unchanged must not parse or re-serialize it. So
//! [`Stored::write_to`] on an unedited resource is one `write_all` of the
//! buffer it was loaded with, and nothing on the path to it has looked at a
//! single JSON token.

use std::borrow::Cow;
use std::io;
use std::sync::{Arc, Mutex, OnceLock};

use super::doc::{Doc, Tape};
use super::path::{PathStep, join, parse_path};
use super::scan::{DecodeError, Meter, Span, escape_into};
use super::tables::{ElementDef, JsonForm, Tables, TypeDef, TypeKind};

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
    /// The span of the innermost resource below the root that the element
    /// sits inside, not counting the element itself: a `contained` entry or a
    /// `Bundle.entry.resource`. A write there re-encodes that resource
    /// (`model.mutate.006`).
    pub boundary: Option<Span>,
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

/// One recorded write: the input range it replaces and what replaces it.
///
/// `design.md` section 3, "Mutation and splicing": a write replaces the
/// written subtree with an owned value and records the span it replaced, and
/// serialisation emits the new encoding at that span and copies everything
/// else.
#[derive(Debug, Clone)]
struct Edit {
    /// The input range replaced. Empty for an insertion, whose position is
    /// `span.start`.
    span: Span,
    /// The bytes written in its place. Empty for a removal.
    bytes: Vec<u8>,
    /// The innermost resource below the root that the edit falls inside: a
    /// `contained` entry or a `Bundle.entry.resource`. Such a resource is
    /// re-encoded whole, so the splice is at its boundary
    /// (`model.mutate.006`); `None` for an edit of the root resource's own
    /// elements, which splices at the edit itself.
    boundary: Option<Span>,
    /// Set when the edit creates a repeating element that was absent, so a
    /// second append to the same element extends this array rather than
    /// creating a second property of the same name.
    created: Option<Created>,
    /// Set when the edit removes items of an array: the array's offset and
    /// the indices removed, so a later removal from the same array is
    /// computed against every index removed so far rather than overlapping
    /// this one.
    removed: Option<(usize, Vec<usize>)>,
}

impl Edit {
    fn new(span: Span, bytes: Vec<u8>, boundary: Option<Span>) -> Self {
        Self {
            span,
            bytes,
            boundary,
            created: None,
            removed: None,
        }
    }
}

/// A repeating element an append created: the owning object's offset, the
/// property name and the items appended so far.
#[derive(Debug, Clone)]
struct Created {
    owner: usize,
    property: String,
    items: Vec<Vec<u8>>,
    comma_before: bool,
    comma_after: bool,
}

impl Created {
    fn render(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.comma_before {
            out.push(b',');
        }
        escape_into(&self.property, &mut out);
        out.extend_from_slice(b":[");
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                out.push(b',');
            }
            out.extend_from_slice(item);
        }
        out.push(b']');
        if self.comma_after {
            out.push(b',');
        }
        out
    }
}

/// Where one run of serialised output came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Copied verbatim from the input buffer: the input range, and the offset
    /// in the output at which the copy begins.
    Spliced { input: Span, output_at: usize },
    /// Newly encoded: the range of the output it occupies.
    Encoded { output: Span },
}

/// One run of output, as the serialiser produces it.
enum Chunk<'a> {
    Original(Span),
    New(&'a [u8]),
}

/// What [`Stored::owner_of`] answers: the owning object's span, its concrete
/// type, the element's definition, the path's last step and the re-encoding
/// boundary.
type Owner<'s> = (
    Span,
    String,
    &'static ElementDef,
    &'s PathStep,
    Option<Span>,
);

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// An element path from its steps: `Patient.contained[0]`.
fn render_path(steps: &[PathStep]) -> String {
    let mut out = String::new();
    for (position, step) in steps.iter().enumerate() {
        if position > 0 {
            out.push('.');
        }
        out.push_str(&step.name);
        if let Some(index) = step.index {
            out.push_str(&format!("[{index}]"));
        }
    }
    out
}

/// Apply `edits`, given as ranges of `source`, into `out`.
fn splice_into(source: &[u8], mut edits: Vec<(Span, &[u8])>, out: &mut Vec<u8>) -> io::Result<()> {
    edits.sort_by_key(|(span, _)| (span.start, span.end));
    let mut cursor = 0usize;
    for (span, replacement) in edits {
        if span.start < cursor || span.end > source.len() {
            return Err(io::Error::other(format!(
                "two writes overlap at byte {}",
                span.start
            )));
        }
        out.extend_from_slice(&source[cursor..span.start]);
        out.extend_from_slice(replacement);
        cursor = span.end;
    }
    out.extend_from_slice(&source[cursor..]);
    Ok(())
}

/// A resource backed by stored bytes, with a lazily built index and a list of
/// the spans a write has replaced.
#[derive(Debug)]
pub struct Stored {
    doc: Doc,
    tables: Tables,
    root_type: OnceLock<&'static TypeDef>,
    edits: Mutex<Vec<Edit>>,
}

impl Stored {
    /// Take an already-owned buffer. No scan happens: this is the load step
    /// `model.bytes.002` requires to cost zero tokens.
    pub fn new(bytes: Arc<[u8]>, tables: Tables, meter: Arc<Meter>) -> Self {
        Self {
            doc: Doc::new(bytes, meter),
            tables,
            root_type: OnceLock::new(),
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
        self.root_def().map(|def| def.name.to_string())
    }

    /// The root's type definition, read once from `resourceType` and then
    /// cached. Its `name` is the resource type, so a caller that only needs to
    /// compare against it or look things up by it pays for no `String`.
    fn root_def(&self) -> Result<&'static TypeDef, DecodeError> {
        if let Some(def) = self.root_type.get() {
            return Ok(def);
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
        let name = self.doc.lexical_borrowed(span)?;
        let Some(def) = self.tables.type_def(&name) else {
            return Err(DecodeError::new(
                "",
                span.start,
                format!("unknown resourceType: {name}"),
            ));
        };
        Ok(*self.root_type.get_or_init(|| def))
    }

    /// Resolve the declared type of a node to its concrete type. For the
    /// abstract `Resource` an element declares — a `contained` entry, or
    /// `Bundle.entry.resource` — that means reading the instance's
    /// `resourceType`.
    fn concrete_type(&self, declared: &str, span: Span) -> Result<String, DecodeError> {
        self.concrete_type_borrowed(declared, span)
            .map(Cow::into_owned)
    }

    /// [`Stored::concrete_type`], borrowing `declared` when it is already
    /// concrete, which is every step but one into a contained resource.
    fn concrete_type_borrowed<'d>(
        &'d self,
        declared: &'d str,
        span: Span,
    ) -> Result<Cow<'d, str>, DecodeError> {
        if self.tables.type_def(declared).map(|def| def.kind) != Some(TypeKind::AnyResource) {
            return Ok(Cow::Borrowed(declared));
        }
        let span = self
            .doc
            .member(span.start, "resourceType")?
            .ok_or_else(|| DecodeError::new("", span.start, "resourceType is required"))?;
        self.doc.lexical_borrowed(span)
    }

    /// Locate the element at `path`. `Ok(None)` is an absent element, which is
    /// not an error (`model.nav.005`).
    pub fn locate(&self, path: &str) -> Result<Option<Located>, DecodeError> {
        let steps = parse_path(path)?;
        let root_type = self.root_def()?.name;
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
            type_name: root_type.to_string(),
            value: Some(root),
            sibling: Sibling::Known(None),
            path: first.name.clone(),
            array: None,
            repeating: false,
            boundary: None,
        };
        for step in rest {
            let Some(span) = located.value else {
                return Ok(None);
            };
            if !self.doc.is_object(span) {
                return Ok(None);
            }
            let owner = self.concrete_type_borrowed(&located.type_name, span)?;
            // The concrete type differs from the declared one exactly when the
            // declared one is the abstract `Resource`: the step is entering a
            // resource below the root.
            let boundary = if owner != located.type_name {
                Some(span)
            } else {
                located.boundary
            };
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
                boundary,
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
        if !self.tables.is_primitive(&located.type_name) {
            return Err(DecodeError::new(
                path,
                span.start,
                format!(
                    "{} is a complex element; only a primitive takes a lexical value",
                    located.type_name
                ),
            ));
        }
        let mut encoded = Vec::new();
        self.encode_primitive(&located.type_name, lexical, &mut encoded);
        self.record(Edit::new(span, encoded, located.boundary), path)
    }

    fn encode_primitive(&self, type_name: &str, lexical: &str, out: &mut Vec<u8>) {
        match self.tables.json_form(type_name) {
            JsonForm::Quoted => escape_into(lexical, out),
            JsonForm::Boolean | JsonForm::Number => out.extend_from_slice(lexical.as_bytes()),
        }
    }

    /// Append one repetition to the repeating element at `path`, which names
    /// the element and not a repetition: `Patient.name`. `values` are the new
    /// repetition's primitive sub-elements, by path relative to it
    /// (`("family", "Delta")`); a repetition of a primitive takes exactly one
    /// value, at the empty path.
    ///
    /// The new repetition is inserted after the array's last item, so every
    /// existing entry keeps its bytes (`model.mutate.002`). An absent element
    /// is created at its position in the generated element order.
    pub fn append(&self, path: &str, values: &[(&str, &str)]) -> Result<(), DecodeError> {
        let steps = parse_path(path)?;
        let (owner, owner_type, def, last, boundary) = self.owner_of(&steps, path)?;
        if last.index.is_some() {
            return Err(DecodeError::new(
                path,
                0,
                "an append names the repeating element, not one of its repetitions",
            ));
        }
        if !def.repeating {
            return Err(DecodeError::new(
                path,
                owner.start,
                format!("{owner_type}.{} does not repeat", def.name),
            ));
        }
        let item_type = def.types[0];
        let item = self.encode_item(item_type, values, path)?;
        match self.doc.member_bytes(owner.start, def.name.as_bytes())? {
            Some(array) if self.doc.is_array(array) => {
                self.insert_item(array, item, boundary, path)?;
                if self.tables.is_primitive(item_type) {
                    // Keep an index-aligned `_field` array aligned, rather
                    // than leaving it one shorter than its values.
                    let sibling = format!("_{}", def.name);
                    if let Some(side) = self.doc.member(owner.start, &sibling)? {
                        if self.doc.is_array(side) {
                            self.insert_item(side, b"null".to_vec(), boundary, path)?;
                        }
                    }
                }
                Ok(())
            }
            Some(other) => Err(DecodeError::new(
                path,
                other.start,
                "the element repeats but its JSON value is not an array",
            )),
            None => self.create_element(owner, &owner_type, def.name, item, boundary, path),
        }
    }

    /// Insert one item at the end of `array`: just after its last item, or
    /// just inside the bracket of an empty one.
    fn insert_item(
        &self,
        array: Span,
        item: Vec<u8>,
        boundary: Option<Span>,
        path: &str,
    ) -> Result<(), DecodeError> {
        let count = self.doc.item_count(array.start)?;
        let at = match count.checked_sub(1) {
            Some(last) => {
                self.doc
                    .item(array.start, last)?
                    .ok_or_else(|| DecodeError::new(path, array.start, "an array item vanished"))?
                    .end
            }
            None => array.start + 1,
        };
        let pending = self
            .lock_edits(path)?
            .iter()
            .any(|edit| edit.span.is_empty() && edit.span.start == at);
        let mut bytes = Vec::with_capacity(item.len() + 1);
        if count > 0 || pending {
            bytes.push(b',');
        }
        bytes.extend_from_slice(&item);
        self.record(Edit::new(Span::new(at, at), bytes, boundary), path)
    }

    /// Create an absent repeating element holding `item`, positioned among
    /// the owner's members by the generated element order.
    fn create_element(
        &self,
        owner: Span,
        owner_type: &str,
        name: &'static str,
        item: Vec<u8>,
        boundary: Option<Span>,
        path: &str,
    ) -> Result<(), DecodeError> {
        {
            let mut edits = self.lock_edits(path)?;
            if let Some(edit) = edits.iter_mut().find(|edit| {
                edit.created
                    .as_ref()
                    .is_some_and(|created| created.owner == owner.start && created.property == name)
            }) {
                if let Some(created) = edit.created.as_mut() {
                    created.items.push(item);
                    edit.bytes = created.render();
                }
                return Ok(());
            }
        }
        let order = super::built::ordered_elements(&self.tables, owner_type);
        let position = |element: &str| order.iter().position(|def| def.name == element);
        let mine = position(name);
        let members = self.doc.member_spans(owner.start)?;
        let mut before = None;
        for (key, _) in &members {
            let Some((def, _, _)) = self
                .tables
                .resolve_property_bytes(owner_type, key.of(self.doc.bytes()))
            else {
                continue;
            };
            if let (Some(theirs), Some(mine)) = (position(def.name), mine) {
                if theirs > mine {
                    before = Some(*key);
                    break;
                }
            }
        }
        let (at, comma_before, comma_after) = match (before, members.last()) {
            // Before the first member that comes later: at its opening quote.
            (Some(key), _) => (key.start - 1, false, true),
            (None, Some((_, value))) => (value.end, true, false),
            (None, None) => (owner.start + 1, false, false),
        };
        let created = Created {
            owner: owner.start,
            property: name.to_string(),
            items: vec![item],
            comma_before,
            comma_after,
        };
        let mut edit = Edit::new(Span::new(at, at), created.render(), boundary);
        edit.created = Some(created);
        self.record(edit, path)
    }

    /// Encode a new repetition of `item_type` from its sub-element values, in
    /// the generated element order.
    fn encode_item(
        &self,
        item_type: &str,
        values: &[(&str, &str)],
        path: &str,
    ) -> Result<Vec<u8>, DecodeError> {
        let mut out = Vec::new();
        if self.tables.is_primitive(item_type) {
            let [("", lexical)] = values else {
                return Err(DecodeError::new(
                    path,
                    0,
                    "a repetition of a primitive takes exactly one value, at the empty path",
                ));
            };
            self.encode_primitive(item_type, lexical, &mut out);
            return Ok(out);
        }
        if self.tables.type_def(item_type).map(|def| def.kind) == Some(TypeKind::AnyResource) {
            return Err(DecodeError::new(
                path,
                0,
                "appending a resource is not implemented: the element declares the abstract \
                 Resource, so the new entry's type cannot be taken from the tables",
            ));
        }
        let mut item = super::built::Built::new(item_type, self.tables);
        for (relative, lexical) in values {
            item.set(&format!("{item_type}.{relative}"), lexical)?;
        }
        item.write(&mut out);
        Ok(out)
    }

    /// Remove the element at `path`: one repetition when the path is indexed
    /// (`Patient.name[1]`), the whole element otherwise. Only the removed
    /// bytes and one separating comma are dropped, so every other entry keeps
    /// its bytes and its order (`model.mutate.003`). An index-aligned
    /// `_field` array loses the same index, and an array left empty is
    /// removed with its property.
    pub fn remove(&self, path: &str) -> Result<(), DecodeError> {
        let steps = parse_path(path)?;
        let (owner, _owner_type, def, last, boundary) = self.owner_of(&steps, path)?;
        let property = if def.choice {
            let located = self
                .locate(path)?
                .ok_or_else(|| DecodeError::new(path, owner.start, "no such element to remove"))?;
            Tables::choice_property(def.name, &located.type_name)
        } else {
            def.name.to_string()
        };
        let sibling = format!("_{property}");
        let members = self.doc.member_spans(owner.start)?;
        let member_index = |name: &str| -> Result<Option<usize>, DecodeError> {
            for (position, (key, _)) in members.iter().enumerate() {
                if self.doc.property_name(*key)? == name {
                    return Ok(Some(position));
                }
            }
            Ok(None)
        };
        let sides = [member_index(&property)?, member_index(&sibling)?];
        if sides.iter().all(Option::is_none) {
            return Err(DecodeError::new(
                path,
                owner.start,
                "no such element to remove",
            ));
        }

        let mut whole_members = Vec::new();
        let mut found = false;
        for position in sides.into_iter().flatten() {
            let value = members[position].1;
            match (def.repeating, last.index) {
                (true, Some(index)) if self.doc.is_array(value) => {
                    let count = self.doc.item_count(value.start)?;
                    if index >= count {
                        continue;
                    }
                    found = true;
                    let mut items = Vec::with_capacity(count);
                    for at in 0..count {
                        items.push(self.doc.item(value.start, at)?.ok_or_else(|| {
                            DecodeError::new(path, value.start, "an array item vanished")
                        })?);
                    }
                    let emptied =
                        self.remove_indices(value.start, &items, index, false, boundary, path)?;
                    if emptied {
                        whole_members.push(position);
                    }
                }
                (true, Some(index)) if index > 0 => {}
                _ => {
                    found = true;
                    whole_members.push(position);
                }
            }
        }
        if !found {
            return Err(DecodeError::new(
                path,
                owner.start,
                "no such repetition to remove",
            ));
        }
        let entries: Vec<Span> = members
            .iter()
            .map(|(key, value)| Span::new(key.start - 1, value.end))
            .collect();
        for position in whole_members {
            self.remove_indices(owner.start, &entries, position, true, boundary, path)?;
        }
        Ok(())
    }

    /// Remove entry `index` of a container -- an array's items or an
    /// object's members, given as `entries` -- together with every entry of
    /// the same container removed before, as non-overlapping removals that
    /// each take one separating comma. Paths address the document as it was
    /// loaded, so removing `name[0]` and then `name[1]` removes both.
    ///
    /// Returns `true`, recording nothing, when an **array** would be left
    /// empty: the caller removes the element's property instead. An object
    /// left empty is written as `{}`.
    fn remove_indices(
        &self,
        container: usize,
        entries: &[Span],
        index: usize,
        is_object: bool,
        boundary: Option<Span>,
        path: &str,
    ) -> Result<bool, DecodeError> {
        let mut removed = vec![index];
        {
            let mut edits = self.lock_edits(path)?;
            for edit in edits.iter() {
                if let Some((owner, indices)) = &edit.removed {
                    if *owner == container {
                        removed.extend_from_slice(indices);
                    }
                }
            }
            removed.sort_unstable();
            removed.dedup();
            if !is_object && removed.len() == entries.len() {
                return Ok(true);
            }
            edits.retain(|edit| {
                edit.removed
                    .as_ref()
                    .is_none_or(|(owner, _)| *owner != container)
            });
        }
        let mut at = 0;
        while at < removed.len() {
            let first = removed[at];
            let mut last = first;
            while at + 1 < removed.len() && removed[at + 1] == last + 1 {
                at += 1;
                last = removed[at];
            }
            at += 1;
            let span = if first > 0 {
                // From the end of the entry before the run.
                Span::new(entries[first - 1].end, entries[last].end)
            } else if let Some(next) = entries.get(last + 1) {
                // A leading run: up to the start of the next entry.
                Span::new(entries[first].start, next.start)
            } else {
                Span::new(entries[first].start, entries[last].end)
            };
            let mut edit = Edit::new(span, Vec::new(), boundary);
            edit.removed = Some((container, removed.clone()));
            self.record(edit, path)?;
        }
        Ok(false)
    }

    /// The object an element path's last step is a member of, its concrete
    /// type, the element's definition, the last step itself, and the
    /// innermost resource below the root that the element sits inside -- the
    /// owner itself when the owner is such a resource (`model.mutate.006`).
    fn owner_of<'s>(&self, steps: &'s [PathStep], path: &str) -> Result<Owner<'s>, DecodeError> {
        let Some((last, parents)) = steps.split_last() else {
            return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
        };
        if parents.is_empty() {
            return Err(DecodeError::new(
                path,
                0,
                "a path with no element steps names the resource, not an element",
            ));
        }
        let parent_path = render_path(parents);
        let parent = self
            .locate(&parent_path)?
            .ok_or_else(|| DecodeError::new(path, 0, "the element's parent does not exist"))?;
        let owner = parent
            .value
            .filter(|span| self.doc.is_object(*span))
            .ok_or_else(|| DecodeError::new(path, 0, "the element's parent is not an object"))?;
        let owner_type = self.concrete_type(&parent.type_name, owner)?;
        let def = self
            .tables
            .element(&owner_type, &last.name)
            .ok_or_else(|| {
                DecodeError::new(
                    path,
                    owner.start,
                    format!("{owner_type} has no element named {:?}", last.name),
                )
            })?;
        let boundary = if owner_type != parent.type_name {
            Some(owner)
        } else {
            parent.boundary
        };
        Ok((owner, owner_type, def, last, boundary))
    }

    fn lock_edits(&self, path: &str) -> Result<std::sync::MutexGuard<'_, Vec<Edit>>, DecodeError> {
        self.edits
            .lock()
            .map_err(|_| DecodeError::new(path, 0, "the write log lock was poisoned"))
    }

    /// Record one edit. A second write to the same span replaces the first; a
    /// write that encloses earlier ones supersedes them; a write inside a span
    /// an earlier write already replaced or removed is refused, because the
    /// bytes it addresses are no longer in the output.
    fn record(&self, edit: Edit, path: &str) -> Result<(), DecodeError> {
        let mut edits = self.lock_edits(path)?;
        let reach = |edit: &Edit| edit.boundary.unwrap_or(edit.span);
        for existing in edits.iter() {
            if existing.span.is_empty() {
                continue;
            }
            let target = if existing.boundary == edit.boundary {
                edit.span
            } else if existing.boundary.is_none() {
                reach(&edit)
            } else {
                continue;
            };
            let inside = if target.is_empty() {
                existing.span.start < target.start && target.start < existing.span.end
            } else {
                existing.span != target && contains(existing.span, target)
            };
            if inside {
                return Err(DecodeError::new(
                    path,
                    edit.span.start,
                    "an earlier write already replaced or removed the bytes this one addresses",
                ));
            }
        }
        if !edit.span.is_empty() {
            edits.retain(|existing| {
                let (own, theirs) = if existing.boundary == edit.boundary {
                    (edit.span, existing.span)
                } else if edit.boundary.is_none() {
                    // A write of the root's own elements that encloses a
                    // re-encoded resource supersedes every write inside it.
                    (edit.span, reach(existing))
                } else {
                    return true;
                };
                if own == theirs {
                    return false;
                }
                let interior = if theirs.is_empty() {
                    own.start < theirs.start && theirs.start < own.end
                } else {
                    contains(own, theirs)
                };
                !interior
            });
        }
        edits.push(edit);
        Ok(())
    }

    /// Decode the whole document, recording every element path it
    /// materialises. This is the `model-full-parse` workload.
    pub fn materialise_all(&self) -> Result<(), DecodeError> {
        let root = self.doc.root();
        let root_type = self.root_def()?.name;
        // The root's member scan steps over every byte of the document once;
        // the tape keeps what it learns on the way, so every container below
        // the root is entered from it rather than scanned again. See
        // [`super::doc::Tape`] for why no observable changes.
        let mut tape = Tape::for_document(self.doc.bytes().len());
        let result = self.materialise_object(root_type, root, root_type, &mut tape, true);
        self.doc.install_tape(tape);
        result
    }

    /// The concrete type of a node declared as `declared`, looked up once:
    /// [`Stored::concrete_type`] and then its definition, without the second
    /// table search or the `String` a type name would cost. `None` is a type
    /// the tables do not model.
    fn concrete_def(
        &self,
        declared_def: Option<&'static TypeDef>,
        span: Span,
    ) -> Result<Option<&'static TypeDef>, DecodeError> {
        if declared_def.map(|def| def.kind) != Some(TypeKind::AnyResource) {
            return Ok(declared_def);
        }
        let span = self
            .doc
            .member(span.start, "resourceType")?
            .ok_or_else(|| DecodeError::new("", span.start, "resourceType is required"))?;
        let concrete = self.doc.lexical_borrowed(span)?;
        Ok(self.tables.type_def(&concrete))
    }

    /// Decode the object at `span`. Its members come from the tape when the
    /// root's scan stepped over it, and from the index otherwise: the root
    /// itself (`root` records into the tape as it is scanned), and an object
    /// declared as the abstract `Resource`, whose `resourceType` is looked up
    /// through the index exactly as before.
    fn materialise_object(
        &self,
        type_name: &str,
        span: Span,
        path: &str,
        tape: &mut Tape,
        root: bool,
    ) -> Result<(), DecodeError> {
        let declared = self.tables.type_def(type_name);
        let Some(concrete) = self.concrete_def(declared, span)? else {
            // An unmodelled subtree is left as bytes rather than guessed at.
            return Ok(());
        };
        let from_tape = match declared.map(|def| def.kind) {
            _ if root => None,
            Some(TypeKind::AnyResource) => None,
            _ => tape.find(span.start).filter(|&node| tape.is_object(node)),
        };
        match from_tape {
            Some(node) => {
                for at in tape.enter(node, self.doc.meter()) {
                    let (property, value) = tape.entry_at(at);
                    self.materialise_member(concrete, property, value, path, tape)?;
                }
            }
            None => {
                let members = if root {
                    self.doc.member_spans_recording(span.start, tape)?
                } else {
                    self.doc.member_spans(span.start)?
                };
                for (property, value) in members {
                    self.materialise_member(concrete, property, value, path, tape)?;
                }
            }
        }
        Ok(())
    }

    fn materialise_member(
        &self,
        concrete: &'static TypeDef,
        property: Span,
        value: Span,
        path: &str,
        tape: &mut Tape,
    ) -> Result<(), DecodeError> {
        let Some((element, element_type, _sibling)) = self
            .tables
            .resolve_property_in(concrete, property.of(self.doc.bytes()))
        else {
            return Ok(());
        };
        if element.repeating && self.doc.is_array(value) {
            match tape.find(value.start).filter(|&node| !tape.is_object(node)) {
                Some(node) => {
                    for (index, at) in tape.enter(node, self.doc.meter()).enumerate() {
                        let (_, item) = tape.entry_at(at);
                        self.materialise_item(element, element_type, item, index, path, tape)?;
                    }
                }
                None => {
                    for (index, item) in self.doc.items(value.start)?.into_iter().enumerate() {
                        self.materialise_item(element, element_type, item, index, path, tape)?;
                    }
                }
            }
        } else {
            let child = self.child_path(path, element.name, element.repeating.then_some(0));
            self.materialise_value(element_type, value, &child, tape)?;
        }
        Ok(())
    }

    fn materialise_item(
        &self,
        element: &ElementDef,
        element_type: &str,
        item: Span,
        index: usize,
        path: &str,
        tape: &mut Tape,
    ) -> Result<(), DecodeError> {
        if self.doc.is_null(item) {
            return Ok(());
        }
        let child = self.child_path(path, element.name, Some(index));
        self.materialise_value(element_type, item, &child, tape)
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
        tape: &mut Tape,
    ) -> Result<(), DecodeError> {
        self.doc.meter().materialise(path);
        if self.tables.is_primitive(type_name) {
            // The value is decoded, but an `xhtml` div is carried as opaque
            // text: nothing here parses or validates it (`model.lazy.006`,
            // `model.json.008`). It is decoded in place: a value with no
            // escape is checked and read where it lies, not copied out.
            let _ = self.doc.lexical_borrowed(span)?;
            return Ok(());
        }
        if self.doc.is_object(span) {
            self.materialise_object(type_name, span, path, tape, false)?;
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
        if edits.iter().all(|edit| edit.boundary.is_none()) {
            return edits.iter().fold(base, |total, edit| {
                total + edit.bytes.len() - edit.span.len()
            });
        }
        drop(edits);
        // A re-encoded resource's length is known only once it is encoded.
        let mut total = 0usize;
        let counted = self.render(&mut |chunk| {
            total += match chunk {
                Chunk::Original(span) => span.len(),
                Chunk::New(bytes) => bytes.len(),
            };
            Ok(())
        });
        if counted.is_ok() { total } else { base }
    }

    /// Serialise in the stored form. With no write, this is the input buffer
    /// verbatim; with writes, it is the original bytes with each replaced span
    /// swapped out.
    pub fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        let unedited = self
            .edits
            .lock()
            .map_err(|_| io::Error::other("the write log lock was poisoned"))?
            .is_empty();
        if unedited {
            return out.write_all(self.doc.bytes());
        }
        let bytes = self.doc.bytes();
        self.render(&mut |chunk| match chunk {
            Chunk::Original(span) => out.write_all(span.of(bytes)),
            Chunk::New(new) => out.write_all(new),
        })
    }

    /// Serialise in the stored form into `out`, and report where every byte
    /// of it came from: the runs copied verbatim from the input, with their
    /// input ranges, and the runs newly encoded. Together they account for
    /// the whole output, in order (`model.mutate.001` .. `.007`).
    pub fn write_traced(&self, out: &mut Vec<u8>) -> io::Result<Vec<Provenance>> {
        let bytes = self.doc.bytes();
        let mut trace = Vec::new();
        self.render(&mut |chunk| {
            let at = out.len();
            match chunk {
                Chunk::Original(span) => {
                    out.extend_from_slice(span.of(bytes));
                    trace.push(Provenance::Spliced {
                        input: span,
                        output_at: at,
                    });
                }
                Chunk::New(new) => {
                    out.extend_from_slice(new);
                    trace.push(Provenance::Encoded {
                        output: Span::new(at, at + new.len()),
                    });
                }
            }
            Ok(())
        })?;
        Ok(trace)
    }

    /// Walk the output as runs of original bytes and new encodings. Edits
    /// inside a resource below the root are first collapsed into one
    /// re-encoding of that resource, so the splice is at its boundary.
    fn render(&self, sink: &mut dyn FnMut(Chunk<'_>) -> io::Result<()>) -> io::Result<()> {
        let bytes = self.doc.bytes();
        let edits = self
            .edits
            .lock()
            .map_err(|_| io::Error::other("the write log lock was poisoned"))?;

        // Group by boundary, keeping first-recorded order within a group.
        let mut groups: Vec<(Span, Vec<&Edit>)> = Vec::new();
        let mut plan: Vec<(Span, std::borrow::Cow<'_, [u8]>)> = Vec::new();
        for edit in edits.iter() {
            match edit.boundary {
                None => plan.push((edit.span, std::borrow::Cow::Borrowed(&edit.bytes))),
                Some(boundary) => match groups.iter_mut().find(|(span, _)| *span == boundary) {
                    Some((_, members)) => members.push(edit),
                    None => groups.push((boundary, vec![edit])),
                },
            }
        }
        for (boundary, members) in groups {
            let mut local = Vec::with_capacity(boundary.len());
            let shifted: Vec<(Span, &[u8])> = members
                .iter()
                .map(|edit| {
                    (
                        Span::new(
                            edit.span.start - boundary.start,
                            edit.span.end - boundary.start,
                        ),
                        edit.bytes.as_slice(),
                    )
                })
                .collect();
            splice_into(boundary.of(bytes), shifted, &mut local)?;
            let reencoded =
                super::decode::decode_with(&local, super::decode::ParseMode::Lenient, self.tables)
                    .map_err(|e| io::Error::other(e.to_string()))?
                    .reencoded;
            plan.push((boundary, std::borrow::Cow::Owned(reencoded)));
        }
        // Stable, so insertions at one point keep the order they were made
        // in, and an insertion sorts before a replacement starting there.
        plan.sort_by_key(|(span, _)| (span.start, span.end));

        let mut cursor = 0usize;
        for (span, replacement) in &plan {
            if span.start < cursor {
                // Overlapping writes would make the output order-dependent;
                // refusing is better than emitting something plausible.
                return Err(io::Error::other(format!(
                    "two writes overlap at byte {}",
                    span.start
                )));
            }
            if span.start > cursor {
                sink(Chunk::Original(Span::new(cursor, span.start)))?;
            }
            if !replacement.is_empty() {
                sink(Chunk::New(replacement))?;
            }
            cursor = span.end;
        }
        if cursor < bytes.len() {
            sink(Chunk::Original(Span::new(cursor, bytes.len())))?;
        }
        Ok(())
    }

    /// Serialise canonically: every object's property names sorted
    /// lexicographically, no whitespace. Unlike the stored form this cannot be
    /// a byte copy, because the stored order is the generated element order.
    pub fn write_canonical(&self, out: &mut dyn io::Write) -> io::Result<()> {
        if self.is_modified() {
            // The recorded writes live beside the original bytes, so the
            // canonical form of an edited resource is the canonical form of
            // its stored form, not of the bytes it was loaded with.
            let mut stored = Vec::with_capacity(self.stored_len());
            self.write_to(&mut stored)?;
            let edited = Stored::new(
                Arc::from(stored.into_boxed_slice()),
                self.tables,
                Arc::clone(self.meter()),
            );
            return edited.write_canonical(out);
        }
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
