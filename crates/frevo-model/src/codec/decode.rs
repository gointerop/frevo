//! Eager JSON decoding: structure, and primitive values.
//!
//! Tasks 19 and 20 of `add-fhirpath-and-codegen-r4`. This is the **other**
//! half of the codec from [`super::stored`]: where that one answers "what is
//! at this path" without looking at anything else, this one decodes a whole
//! document into an owned tree and can re-emit it in the generated element
//! order. Both exist because the oracles grade different things —
//! `model.bytes.*` and `model.lazy.*` grade that a read costs nothing it did
//! not need, and `model.json.*` grades that a full materialise-and-re-emit
//! reproduces the input byte for byte. Handing back the stored buffer is not
//! an answer to the second.
//!
//! Which upstream parser governs what is `design.md` section 3's table. The
//! **shape** decoded here — `resourceType` dispatch, the choice property
//! names, contained resources, the `_field` sibling form — is
//! `org.hl7.fhir.r4.formats.JsonParser` at pin `73c432b5`; what the server
//! **accepts** is `ca.uhn.fhir.parser.JsonParser` plus the handler
//! `FhirContext` installs, at pin `7ab4c7c7`, and that is the lenient one.
//!
//! Three properties are deliberate and are what the scenarios turn on:
//!
//! 1. **A primitive is re-emitted from the bytes it was read from.** A
//!    decimal's lexical form, a `base64Binary`'s padding and an `xhtml` div's
//!    escaping all survive because nothing re-renders them — the token's own
//!    bytes are copied out (`model.json.008`, `.009`, `.010`, `.017`).
//! 2. **A value and its `_field` sibling are two independent sides of one
//!    element**, never merged, with array lengths and `null` holes recorded on
//!    each side separately, because that is the only representation that can
//!    distinguish "no value" from "no element" and put the holes back where
//!    they were (`design.md` section 3).
//! 3. **Nesting is bounded.** A recursive-descent decoder with no bound turns
//!    an adversarial document into a stack overflow, which aborts the process
//!    rather than returning an error, so the bound is part of the decoder
//!    rather than a later hardening pass.
//!
//! What is **not** here, by task boundary: the unknown-property path, the JSON
//! type mismatches, the duplicate key, the `null`-in-a-complex-array report,
//! the invalid-primitive-value rejection and the extension-with-both report
//! are task 21's, because each one's outcome is what distinguishes the two
//! parse modes. The two contained-resource diagnostics are task 21a's. This
//! module reports the conditions tasks 19 and 20 name — a missing or unknown
//! `resourceType`, and a numeric value outside its declared type's range — and
//! implements the rule that turns a diagnostic into an error in strict mode,
//! because a decoder that cannot answer `decode(bytes, Strict)` at all cannot
//! be called.

use super::built::ordered_elements;
use super::scan::{Span, skip_ws, unescape};
use super::tables::{ElementDef, Tables, TypeKind};

/// How deep a document may nest before decoding refuses it.
///
/// R4's deepest core structure nests a handful of levels; `Questionnaire.item`
/// recurses but no real instance approaches this. The number is a bound on
/// recursion depth and therefore on stack use: at roughly a few hundred bytes
/// per frame, 512 frames is well inside the smallest thread stack this crate
/// runs on, which is what makes a depth error reportable rather than fatal to
/// the process.
pub const MAX_DEPTH: usize = 512;

/// The two parse modes, as `FhirContext.setParserErrorHandler` selects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// `LenientErrorHandler`: decoding succeeds and returns its diagnostics
    /// beside the resource. This is the handler `FhirContext` installs, so it
    /// is what the HTTP surface uses.
    Lenient,
    /// `StrictErrorHandler`: every diagnostic is raised as an error instead.
    Strict,
}

/// The mode a decoder created with no explicit mode is in.
///
/// `FhirContext` declares
/// `private volatile IParserErrorHandler myParserErrorHandler = new LenientErrorHandler();`,
/// so the default is lenient and a body HAPI 8.12.0 accepts is accepted.
pub const DEFAULT_PARSE_MODE: ParseMode = ParseMode::Lenient;

/// What a decode report is about. One variant per condition the spec delta
/// names, so a caller can match the condition rather than the message prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// `resourceType` is absent (`model.json.002`).
    MissingResourceType,
    /// `resourceType` names no known type (`model.json.003`).
    UnknownResourceType,
    /// `IParserErrorHandler.unknownElement` / `unknownAttribute`
    /// (`model.json.004`).
    UnknownProperty,
    /// `IParserErrorHandler.incorrectJsonType` (`model.json.013`, `.014`).
    /// Also carries the malformed-JSON case, which has no handler callback
    /// because Jackson would have refused the document before HAPI saw it.
    IncorrectJsonType,
    /// `IParserErrorHandler.unexpectedRepeatingElement` (`model.json.015`).
    UnexpectedRepeatingElement,
    /// A repeated JSON property name (`model.json.016`).
    DuplicateProperty,
    /// `null` inside an array of a non-primitive element (`model.json.018`).
    NullInNonPrimitiveArray,
    /// A numeric value outside its declared type's range (`model.json.011`,
    /// `model.json.012`).
    ValueOutOfRange,
    /// `IParserErrorHandler.invalidValue` (`model.json.023`).
    InvalidValue,
    /// `IParserErrorHandler.extensionContainsValueAndNestedExtensions`
    /// (`model.json.022`).
    ExtensionHasValueAndNestedExtensions,
    /// The input bytes are not valid UTF-8 (`model.json.019`).
    NotUtf8,
    /// A `_field` array and its value array disagree in length
    /// (`model.primext.006`).
    PrimitiveExtensionLengthMismatch,
    /// A `_field` property on a non-primitive element (`model.primext.009`).
    UnexpectedUnderscoreProperty,
    /// Nesting beyond [`MAX_DEPTH`] (`model.errors.005`).
    DepthLimit,
}

/// One report: a diagnostic in lenient mode, the same thing raised as an error
/// in strict mode (`model.json.021`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: ReportKind,
    /// The FHIR element path, not a JSON pointer: `Patient.name[0].given[1]`.
    pub path: String,
    /// The byte offset in the input at which the problem was found.
    pub offset: usize,
    pub message: String,
    /// The type the element declares, where the condition has one.
    pub expected_type: Option<String>,
    /// The JSON type actually found, where the condition has one.
    pub found_type: Option<String>,
}

impl Diagnostic {
    fn new(kind: ReportKind, path: &str, offset: usize, message: String) -> Self {
        Self {
            kind,
            path: path.to_string(),
            offset,
            message,
            expected_type: None,
            found_type: None,
        }
    }

    fn expecting(mut self, type_name: &str) -> Self {
        self.expected_type = Some(type_name.to_string());
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}: {} at {} (byte {})",
            self.kind, self.message, self.path, self.offset
        )
    }
}

impl std::error::Error for Diagnostic {}

/// A decimal read back from the model: the lexical form as it appeared, and
/// the exact value rendered positionally.
///
/// The two are kept apart so that `model.json.010` can be asserted: an `f64`
/// holds about 17 significant digits, so a `full_precision` that still carries
/// 40 of them is proof that the value never went through a binary float.
/// Nothing in this module parses a decimal into a float; `full_precision` is
/// produced by moving the decimal point of the input's own digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal {
    pub lexical: String,
    pub full_precision: String,
    pub significant_digits: u32,
}

/// One element occurrence of a decoded resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedElement {
    /// FHIR element path with `[i]` for each repetition of a repeating
    /// element, choice elements under their base name.
    pub path: String,
    /// The concrete FHIR type name: `Patient`, `HumanName`, `string`,
    /// `Quantity`.
    pub type_name: String,
    /// The primitive value's lexical form, or `None` when the element has no
    /// value — which is exactly what `hasValue()` reports.
    pub value: Option<String>,
    /// The element's own `id`: from the `_field` sibling for a primitive, from
    /// the object's `id` property otherwise (`model.json.020`).
    pub element_id: Option<String>,
    /// The `url` of each extension on this element, in document order.
    pub extension_urls: Vec<String>,
    /// Set when the element's concrete type is `decimal`.
    pub decimal: Option<Decimal>,
    /// Whether this element is a resource (`model.json.006`, `.007`).
    pub is_resource: bool,
}

/// A successfully decoded resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The type taken from `resourceType` (`model.json.001`).
    pub resource_type: String,
    /// Every diagnostic, in document order (`model.errors.003`).
    pub diagnostics: Vec<Diagnostic>,
    /// Every element of the resource, in document order.
    pub elements: Vec<DecodedElement>,
    /// The resource fully materialised and re-emitted compact in the generated
    /// element order.
    pub reencoded: Vec<u8>,
}

/// Decode `bytes` in `mode`, using the generated R4 tables.
pub fn decode(bytes: &[u8], mode: ParseMode) -> Result<Decoded, Diagnostic> {
    decode_with(bytes, mode, super::R4)
}

/// Decode with no mode stated by the caller: [`DEFAULT_PARSE_MODE`].
pub fn decode_default(bytes: &[u8]) -> Result<Decoded, Diagnostic> {
    decode(bytes, DEFAULT_PARSE_MODE)
}

/// [`decode`] against a caller-supplied table, so the codec's own tests can
/// read a fixture table rather than the generated one.
pub fn decode_with(bytes: &[u8], mode: ParseMode, tables: Tables) -> Result<Decoded, Diagnostic> {
    // Encoding-strict, up front: every later step may then treat the buffer as
    // text. This is one linear pass and the eager decode is linear anyway, so
    // it costs nothing asymptotically — and it is emphatically not on the lazy
    // path, where a scan of the whole document would be exactly the thing
    // `model.bytes.002` forbids.
    if let Err(error) = std::str::from_utf8(bytes) {
        let offset = error.valid_up_to();
        return Err(Diagnostic::new(
            ReportKind::NotUtf8,
            "",
            offset,
            format!("the input is not valid UTF-8: byte {offset} is not part of a valid sequence"),
        ));
    }

    let raw = RawParser::parse(bytes)?;
    let mut decoder = Decoder {
        bytes,
        tables,
        diagnostics: Vec::new(),
    };
    let root = decoder.shape_resource(&raw, None)?;
    let resource_type = root.type_name.to_string();

    let mut reencoded = Vec::with_capacity(bytes.len());
    decoder.emit_object(&root, &mut reencoded);

    let mut elements = Vec::new();
    decoder.flatten_object(&root, &resource_type, true, None, &mut elements);

    let diagnostics = decoder.diagnostics;
    if mode == ParseMode::Strict {
        // Strict mode raises the diagnostic lenient mode records -- same kind,
        // same path, same offset, same message -- and changes nothing else, so
        // the two modes cannot drift apart. The error is the first problem in
        // document order (`model.errors.003`).
        if let Some(first) = diagnostics.first() {
            return Err(first.clone());
        }
    }

    Ok(Decoded {
        resource_type,
        diagnostics,
        elements,
        reencoded,
    })
}

// ---------------------------------------------------------------------------
// The raw JSON tree
//
// One linear pass, keeping every scalar as the span of its own bytes so that
// re-emitting it is a copy rather than a re-rendering.
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Raw {
    /// A string (span includes the quotes), a number, `true`, `false` or
    /// `null`.
    Scalar(Span),
    Object {
        start: usize,
        /// `(key text inside its quotes, value)` in document order.
        members: Vec<(Span, Raw)>,
    },
    Array {
        start: usize,
        items: Vec<Raw>,
    },
}

impl Raw {
    fn start(&self) -> usize {
        match self {
            Raw::Scalar(span) => span.start,
            Raw::Object { start, .. } | Raw::Array { start, .. } => *start,
        }
    }
}

struct RawParser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> RawParser<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Raw, Diagnostic> {
        let mut parser = RawParser { bytes, at: 0 };
        let value = parser.value(0)?;
        parser.at = skip_ws(bytes, parser.at);
        if parser.at < bytes.len() {
            return Err(parser.malformed("trailing content after the root value"));
        }
        Ok(value)
    }

    fn malformed(&self, what: &str) -> Diagnostic {
        Diagnostic::new(
            ReportKind::IncorrectJsonType,
            "",
            self.at.min(self.bytes.len().saturating_sub(1)),
            format!(
                "the input is not well-formed JSON: {what} at byte {}",
                self.at
            ),
        )
    }

    fn value(&mut self, depth: usize) -> Result<Raw, Diagnostic> {
        if depth > MAX_DEPTH {
            return Err(Diagnostic::new(
                ReportKind::DepthLimit,
                "",
                self.at.min(self.bytes.len().saturating_sub(1)),
                format!("the document nests deeper than the {MAX_DEPTH}-level bound"),
            ));
        }
        self.at = skip_ws(self.bytes, self.at);
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Raw::Scalar(self.quoted()?)),
            Some(_) => Ok(Raw::Scalar(self.bare()?)),
            None => Err(self.malformed("unexpected end of input")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Raw, Diagnostic> {
        let start = self.at;
        self.at += 1;
        let mut members = Vec::new();
        self.at = skip_ws(self.bytes, self.at);
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Raw::Object { start, members });
        }
        loop {
            self.at = skip_ws(self.bytes, self.at);
            let quoted = self.quoted()?;
            let key = Span::new(quoted.start + 1, quoted.end.saturating_sub(1));
            self.at = skip_ws(self.bytes, self.at);
            if self.bytes.get(self.at) != Some(&b':') {
                return Err(self.malformed("expected ':' after a property name"));
            }
            self.at += 1;
            let value = self.value(depth + 1)?;
            members.push((key, value));
            self.at = skip_ws(self.bytes, self.at);
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Raw::Object { start, members });
                }
                _ => return Err(self.malformed("expected ',' or '}' after a property value")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Raw, Diagnostic> {
        let start = self.at;
        self.at += 1;
        let mut items = Vec::new();
        self.at = skip_ws(self.bytes, self.at);
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Raw::Array { start, items });
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.at = skip_ws(self.bytes, self.at);
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Raw::Array { start, items });
                }
                _ => return Err(self.malformed("expected ',' or ']' after an array item")),
            }
        }
    }

    /// Scan a quoted string, quotes included.
    fn quoted(&mut self) -> Result<Span, Diagnostic> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return Err(self.malformed("expected a JSON string"));
        }
        let start = self.at;
        let mut cursor = start + 1;
        loop {
            match self.bytes.get(cursor) {
                None => {
                    self.at = start;
                    return Err(self.malformed("unterminated string"));
                }
                Some(b'"') => {
                    self.at = cursor + 1;
                    return Ok(Span::new(start, cursor + 1));
                }
                Some(b'\\') => cursor += 2,
                Some(_) => cursor += 1,
            }
        }
    }

    /// Scan a number, `true`, `false` or `null`.
    fn bare(&mut self) -> Result<Span, Diagnostic> {
        let start = self.at;
        let mut cursor = start;
        while matches!(self.bytes.get(cursor), Some(c)
            if !matches!(c, b',' | b'}' | b']' | b':' | b' ' | b'\t' | b'\r' | b'\n'))
        {
            cursor += 1;
        }
        if cursor == start {
            return Err(self.malformed("expected a JSON value"));
        }
        self.at = cursor;
        Ok(Span::new(start, cursor))
    }
}

// ---------------------------------------------------------------------------
// The typed tree
// ---------------------------------------------------------------------------

/// One element of an object, with its two sides kept apart.
#[derive(Debug)]
struct Prop {
    def: &'static ElementDef,
    /// The choice type the property name spelled, for a `value[x]`-style
    /// element. `None` for an ordinary element.
    choice: Option<&'static str>,
    /// The concrete FHIR type the value side carries.
    concrete: &'static str,
    /// The element's own JSON value, absent when the element exists only as a
    /// `_field` sibling.
    value: Option<Shape>,
    /// The matching `_name` value, absent when there is none.
    sibling: Option<Shape>,
}

impl Prop {
    /// The JSON property name the value side takes: the base name, or the base
    /// name plus the choice type with its first letter upper-cased.
    fn property(&self) -> String {
        match self.choice {
            Some(concrete) => Tables::choice_property(self.def.name, concrete),
            None => self.def.name.to_string(),
        }
    }
}

/// The JSON shape one side of an element took: a single value or an array.
/// Keeping the distinction, and the array's length and holes, is what lets the
/// output put a `null` back where it was (`model.primext.007`).
#[derive(Debug)]
enum Shape {
    Single(Option<Value>),
    Array(Vec<Option<Value>>),
}

impl Shape {
    fn len(&self) -> usize {
        match self {
            Shape::Single(_) => 1,
            Shape::Array(items) => items.len(),
        }
    }

    fn at(&self, index: usize) -> Option<&Value> {
        match self {
            Shape::Single(value) => {
                if index == 0 {
                    value.as_ref()
                } else {
                    None
                }
            }
            Shape::Array(items) => items.get(index).and_then(|item| item.as_ref()),
        }
    }
}

#[derive(Debug)]
enum Value {
    Primitive(Primitive),
    Object(Object),
}

impl Value {
    fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(object) => Some(object),
            Value::Primitive(_) => None,
        }
    }

    fn primitive(&self) -> Option<Primitive> {
        match self {
            Value::Primitive(primitive) => Some(*primitive),
            Value::Object(_) => None,
        }
    }
}

/// A primitive value, held as **the span of the token it was read from** and
/// nothing else.
///
/// Re-emitting is therefore a copy of those bytes, which is the whole of
/// `model.json.009`, `.010` and `.017`: a decimal's trailing zeros and
/// exponent form, a `base64Binary`'s padding and an `xhtml` div's escaping
/// survive because nothing re-renders them. The lexical form is produced on
/// demand rather than stored, so a decode that nobody reads a value out of
/// allocates nothing per primitive.
#[derive(Debug, Clone, Copy)]
struct Primitive {
    raw: Span,
}

#[derive(Debug)]
struct Object {
    type_name: &'static str,
    is_resource: bool,
    props: Vec<Prop>,
}

/// Where shaping currently is, as a borrowed chain rather than a string.
///
/// Shaping needs an element path only to attach to a diagnostic, and a
/// document that decodes cleanly produces none. Formatting one per element
/// regardless was measured at **80 of the 147 microseconds** a 4 KiB `Patient`
/// took, so the path is a chain of borrows up to the root and is rendered only
/// at the moment a report is built. This is constitution rule 6.2 applied to
/// the decoder's own bookkeeping: materialise only what the caller asked for.
#[derive(Debug, Clone, Copy)]
struct Loc<'p> {
    parent: Option<&'p Loc<'p>>,
    name: &'p str,
    index: Option<usize>,
}

impl<'p> Loc<'p> {
    fn root(name: &'p str) -> Self {
        Loc {
            parent: None,
            name,
            index: None,
        }
    }

    fn child(&'p self, name: &'p str) -> Loc<'p> {
        Loc {
            parent: Some(self),
            name,
            index: None,
        }
    }

    fn at(mut self, index: Option<usize>) -> Self {
        self.index = index;
        self
    }

    fn render(&self) -> String {
        let mut out = match self.parent {
            Some(parent) => parent.render(),
            None => String::new(),
        };
        if !out.is_empty() {
            out.push('.');
        }
        out.push_str(self.name);
        if let Some(index) = self.index {
            out.push('[');
            out.push_str(&index.to_string());
            out.push(']');
        }
        out
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    tables: Tables,
    diagnostics: Vec<Diagnostic>,
}

impl Decoder<'_> {
    /// Decode a resource: `resourceType` dispatch and its three error paths —
    /// a root that is not a JSON object, a missing `resourceType`, and a
    /// `resourceType` naming no known type. All three are errors in both
    /// modes: there is no resource to return and nothing for a diagnostic to
    /// be attached to.
    fn shape_resource(&mut self, raw: &Raw, at: Option<&Loc<'_>>) -> Result<Object, Diagnostic> {
        let path = || at.map(Loc::render).unwrap_or_default();
        let Raw::Object { start, members } = raw else {
            return Err(Diagnostic::new(
                ReportKind::IncorrectJsonType,
                &path(),
                raw.start(),
                "a FHIR resource must be a JSON object".to_string(),
            ));
        };
        let Some((_, type_value)) = members
            .iter()
            .find(|(key, _)| self.key_is(*key, b"resourceType"))
        else {
            return Err(Diagnostic::new(
                ReportKind::MissingResourceType,
                &path(),
                *start,
                "resourceType is required on a FHIR resource".to_string(),
            ));
        };
        let name = match type_value {
            Raw::Scalar(span) if self.bytes.get(span.start) == Some(&b'"') => self.text(*span),
            other => {
                return Err(Diagnostic::new(
                    ReportKind::IncorrectJsonType,
                    &path(),
                    other.start(),
                    "resourceType must be a JSON string".to_string(),
                ));
            }
        };
        let Some(def) = self.tables.type_def(&name) else {
            return Err(Diagnostic::new(
                ReportKind::UnknownResourceType,
                &path(),
                type_value.start(),
                format!("unknown resourceType: {name}"),
            ));
        };
        if def.kind != TypeKind::Resource {
            return Err(Diagnostic::new(
                ReportKind::UnknownResourceType,
                &path(),
                type_value.start(),
                format!("unknown resourceType: {name} is not a resource type"),
            ));
        }
        // The root's children are pathed from the resource type name; a
        // nested resource's from where it sits (`Patient.contained[0]`).
        let own = Loc::root(def.name);
        Ok(self.shape_members(def.name, true, members, at.unwrap_or(&own)))
    }

    /// Decode a complex element or a backbone element.
    fn shape_object(&mut self, declared: &'static str, raw: &Raw, at: &Loc<'_>) -> Option<Object> {
        // A `contained` entry and a `Bundle.entry.resource` declare the
        // abstract `Resource`; the concrete type is the instance's own
        // `resourceType` (`model.json.006`, `model.json.007`).
        if matches!(
            self.tables.type_def(declared).map(|def| def.kind),
            Some(TypeKind::AnyResource)
        ) {
            return match self.shape_resource(raw, Some(at)) {
                Ok(object) => Some(object),
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    None
                }
            };
        }
        let Raw::Object { members, .. } = raw else {
            return None;
        };
        Some(self.shape_members(declared, false, members, at))
    }

    fn shape_members(
        &mut self,
        type_name: &'static str,
        is_resource: bool,
        members: &[(Span, Raw)],
        at: &Loc<'_>,
    ) -> Object {
        let mut props: Vec<Prop> = Vec::new();
        for (key, raw) in members {
            let key_bytes = key.of(self.bytes);
            if key_bytes == b"resourceType" {
                continue;
            }
            let Some((def, concrete, is_sibling)) =
                self.tables.resolve_property_bytes(type_name, key_bytes)
            else {
                // The unknown-property path is task 21's: its outcome is what
                // the two parse modes differ on. The property is dropped,
                // which is what lenient mode does with it.
                continue;
            };
            let choice = def.choice.then_some(concrete);
            // A `_field` sibling carries only the `id` and `extension` of
            // `Element`, whatever the element's own type is.
            let item_type: &'static str = if is_sibling { "Element" } else { concrete };
            let here = at.child(def.name);
            let shape = self.shape_side(def, item_type, raw, &here);
            let position = props
                .iter()
                .position(|prop| prop.def.name == def.name && prop.choice == choice);
            let index = match position {
                Some(index) => index,
                None => {
                    props.push(Prop {
                        def,
                        choice,
                        concrete,
                        value: None,
                        sibling: None,
                    });
                    props.len() - 1
                }
            };
            if is_sibling {
                props[index].sibling = Some(shape);
            } else {
                // A repeated property name silently overwrites, because HAPI
                // reads JSON through a Jackson `ObjectMapper` built without
                // `STRICT_DUPLICATE_DETECTION`. Reporting it is
                // `model.json.016`, which is task 21's.
                props[index].value = Some(shape);
            }
        }
        Object {
            type_name,
            is_resource,
            props,
        }
    }

    /// One side of an element: the JSON shape it took, with each item decoded
    /// as `item_type`.
    fn shape_side(
        &mut self,
        def: &'static ElementDef,
        item_type: &'static str,
        raw: &Raw,
        at: &Loc<'_>,
    ) -> Shape {
        match raw {
            Raw::Array { items, .. } => {
                let mut out = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    let here = at.at(def.repeating.then_some(index));
                    out.push(self.shape_item(item_type, item, &here));
                }
                Shape::Array(out)
            }
            single => {
                let here = at.at(def.repeating.then_some(0));
                Shape::Single(self.shape_item(item_type, single, &here))
            }
        }
    }

    /// One item of one side. `None` is a `null` hole, which means "nothing at
    /// this index" for this side and nothing more.
    fn shape_item(&mut self, item_type: &'static str, raw: &Raw, at: &Loc<'_>) -> Option<Value> {
        if let Raw::Scalar(span) = raw {
            if span.of(self.bytes) == b"null" {
                return None;
            }
        }
        if self.tables.is_primitive(item_type) {
            let Raw::Scalar(span) = raw else {
                // A JSON object or array where a primitive belongs is
                // `incorrectJsonType`, task 21's.
                return None;
            };
            return Some(Value::Primitive(self.shape_primitive(item_type, *span, at)));
        }
        self.shape_object(item_type, raw, at).map(Value::Object)
    }

    /// A primitive value. Its bytes are kept verbatim for re-emission; its
    /// lexical form is the string's text or the number's or boolean's own
    /// characters.
    fn shape_primitive(&mut self, type_name: &str, span: Span, at: &Loc<'_>) -> Primitive {
        let primitive = Primitive { raw: span };
        // `integer`, `positiveInt` and `unsignedInt` carry a declared range,
        // and a value outside it is reported naming the element path and the
        // declared type (`model.json.011`, `model.json.012`). An `xhtml` div
        // and a `base64Binary` are not inspected at all: the first is opaque
        // markup and the second is already its encoded text, so neither is
        // parsed, decoded or validated here or anywhere else.
        if let Some(bound) = IntegerBound::of(type_name) {
            let lexical = self.lexical(primitive);
            if let Some(message) = bound.violation(&lexical) {
                self.diagnostics.push(
                    Diagnostic::new(
                        ReportKind::ValueOutOfRange,
                        &at.render(),
                        span.start,
                        message,
                    )
                    .expecting(type_name),
                );
            }
        }
        primitive
    }

    /// A primitive's lexical form: a quoted string's text with its escapes
    /// resolved, or a number's or boolean's own characters. Built on demand,
    /// so a decode nobody reads a value out of allocates nothing per
    /// primitive.
    fn lexical(&self, primitive: Primitive) -> String {
        if self.bytes.get(primitive.raw.start) == Some(&b'"') {
            self.text(primitive.raw)
        } else {
            String::from_utf8_lossy(primitive.raw.of(self.bytes)).into_owned()
        }
    }

    fn key_is(&self, key: Span, want: &[u8]) -> bool {
        let raw = key.of(self.bytes);
        if raw.contains(&b'\\') {
            unescape(raw).map(|text| text.as_bytes() == want) == Ok(true)
        } else {
            raw == want
        }
    }

    /// The text of a quoted JSON string, escapes resolved. An unresolvable
    /// escape leaves the text as it was written rather than losing it: the
    /// invalid-value path is task 21's and this is not the place to decide it.
    fn text(&self, span: Span) -> String {
        let inner = Span::new(span.start + 1, span.end.saturating_sub(1));
        let raw = inner.of(self.bytes);
        unescape(raw).unwrap_or_else(|_| String::from_utf8_lossy(raw).into_owned())
    }

    // -----------------------------------------------------------------------
    // Re-emission, in the generated element order
    // -----------------------------------------------------------------------

    fn emit_object(&self, object: &Object, out: &mut Vec<u8>) {
        out.push(b'{');
        let mut first = true;
        if object.is_resource {
            out.extend_from_slice(br#""resourceType":""#);
            out.extend_from_slice(object.type_name.as_bytes());
            out.push(b'"');
            first = false;
        }
        for def in ordered_elements(&self.tables, object.type_name) {
            for prop in object.props.iter().filter(|prop| prop.def.name == def.name) {
                let property = prop.property();
                if let Some(shape) = &prop.value {
                    self.emit_member(&property, shape, &mut first, out);
                }
                if let Some(shape) = &prop.sibling {
                    // The `_field` sibling follows its value, which is the
                    // order `JsonParser` composes: `composeStringCore` then
                    // `composeStringExtras`.
                    self.emit_member(&format!("_{property}"), shape, &mut first, out);
                }
            }
        }
        out.push(b'}');
    }

    fn emit_member(&self, property: &str, shape: &Shape, first: &mut bool, out: &mut Vec<u8>) {
        if !*first {
            out.push(b',');
        }
        *first = false;
        out.push(b'"');
        out.extend_from_slice(property.as_bytes());
        out.extend_from_slice(br#"":"#);
        self.emit_shape(shape, out);
    }

    fn emit_shape(&self, shape: &Shape, out: &mut Vec<u8>) {
        match shape {
            Shape::Single(value) => self.emit_value(value.as_ref(), out),
            Shape::Array(items) => {
                out.push(b'[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    self.emit_value(item.as_ref(), out);
                }
                out.push(b']');
            }
        }
    }

    fn emit_value(&self, value: Option<&Value>, out: &mut Vec<u8>) {
        match value {
            None => out.extend_from_slice(b"null"),
            Some(Value::Primitive(primitive)) => {
                out.extend_from_slice(primitive.raw.of(self.bytes));
            }
            Some(Value::Object(object)) => self.emit_object(object, out),
        }
    }

    // -----------------------------------------------------------------------
    // The element list
    // -----------------------------------------------------------------------

    /// Record `object` and then every element below it, in document order.
    /// `sibling` is the `_field` object when this element is a primitive, so
    /// that the element's `id` and `extension` are read from the right side.
    fn flatten_object(
        &self,
        object: &Object,
        path: &str,
        is_resource: bool,
        sibling: Option<&Object>,
        out: &mut Vec<DecodedElement>,
    ) {
        out.push(DecodedElement {
            path: path.to_string(),
            type_name: object.type_name.to_string(),
            value: None,
            element_id: self.element_id_of(Some(object), sibling),
            extension_urls: self.extension_urls_of(Some(object), sibling),
            decimal: None,
            is_resource,
        });
        self.flatten_props(object, path, out);
    }

    fn flatten_props(&self, object: &Object, path: &str, out: &mut Vec<DecodedElement>) {
        for prop in &object.props {
            let element_path = child_path(path, prop.def.name);
            let occurrences = match (&prop.value, &prop.sibling) {
                (Some(value), Some(sibling)) => value.len().max(sibling.len()),
                (Some(value), None) => value.len(),
                (None, Some(sibling)) => sibling.len(),
                (None, None) => 0,
            };
            for index in 0..occurrences {
                let occurrence_path = if prop.def.repeating {
                    indexed(&element_path, index)
                } else {
                    element_path.clone()
                };
                let value = prop.value.as_ref().and_then(|shape| shape.at(index));
                let sibling = prop
                    .sibling
                    .as_ref()
                    .and_then(|shape| shape.at(index))
                    .and_then(Value::as_object);
                match value {
                    Some(Value::Object(child)) => {
                        let is_resource = self.tables.type_def(child.type_name).map(|def| def.kind)
                            == Some(TypeKind::Resource);
                        self.flatten_object(child, &occurrence_path, is_resource, sibling, out);
                    }
                    other => {
                        let lexical = other
                            .and_then(Value::primitive)
                            .map(|primitive| self.lexical(primitive));
                        out.push(DecodedElement {
                            path: occurrence_path.clone(),
                            type_name: prop.concrete.to_string(),
                            value: lexical.clone(),
                            element_id: self.element_id_of(None, sibling),
                            extension_urls: self.extension_urls_of(None, sibling),
                            decimal: lexical
                                .filter(|_| prop.concrete == "decimal")
                                .map(|lexical| decimal_of(&lexical)),
                            is_resource: false,
                        });
                    }
                }
                // A primitive's `id` and `extension` live on the `_field`
                // side, so they are children of the primitive element rather
                // than elements in their own right: `_birthDate` is never a
                // path (`model.primext.001`).
                if let Some(sibling) = sibling {
                    if value.and_then(Value::as_object).is_none() {
                        self.flatten_props(sibling, &occurrence_path, out);
                    }
                }
            }
        }
    }

    /// The element `id`: an object's own `id` property, or a primitive's
    /// `_field` sibling's (`model.json.020`).
    fn element_id_of(&self, object: Option<&Object>, sibling: Option<&Object>) -> Option<String> {
        object
            .and_then(|object| self.primitive_property(object, "id"))
            .or_else(|| sibling.and_then(|sibling| self.primitive_property(sibling, "id")))
    }

    /// The `url` of each extension on an element, in document order.
    fn extension_urls_of(&self, object: Option<&Object>, sibling: Option<&Object>) -> Vec<String> {
        let mut out = Vec::new();
        for source in [object, sibling].into_iter().flatten() {
            for prop in source
                .props
                .iter()
                .filter(|prop| prop.def.name == "extension")
            {
                let Some(shape) = &prop.value else { continue };
                for index in 0..shape.len() {
                    let Some(extension) = shape.at(index).and_then(Value::as_object) else {
                        continue;
                    };
                    if let Some(url) = self.primitive_property(extension, "url") {
                        out.push(url);
                    }
                }
            }
        }
        out
    }

    /// The lexical value of a singleton primitive property of an object.
    fn primitive_property(&self, object: &Object, name: &str) -> Option<String> {
        object
            .props
            .iter()
            .find(|prop| prop.def.name == name)
            .and_then(|prop| prop.value.as_ref())
            .and_then(|shape| shape.at(0))
            .and_then(Value::primitive)
            .map(|primitive| self.lexical(primitive))
    }
}

/// The path of a child element. Unlike [`Loc`], these are built eagerly,
/// because the element list is the caller's answer rather than bookkeeping: a
/// caller that asked for the elements asked for their paths.
fn child_path(parent: &str, name: &str) -> String {
    let mut out = String::with_capacity(parent.len() + name.len() + 4);
    out.push_str(parent);
    if !parent.is_empty() {
        out.push('.');
    }
    out.push_str(name);
    out
}

fn indexed(path: &str, index: usize) -> String {
    format!("{path}[{index}]")
}

// ---------------------------------------------------------------------------
// Primitive values: the integer bounds and the decimal
// ---------------------------------------------------------------------------

/// The declared range of one of R4's three integer primitives.
#[derive(Debug, Clone, Copy)]
struct IntegerBound {
    type_name: &'static str,
    low: i64,
    high: i64,
}

impl IntegerBound {
    /// R4's three bounded numeric primitives. `decimal` is deliberately not
    /// among them: it has no declared range, and giving it one would be
    /// inventing a rule — which is also why nothing here parses it into a
    /// number at all.
    fn of(type_name: &str) -> Option<Self> {
        const BOUNDS: &[IntegerBound] = &[
            IntegerBound {
                type_name: "integer",
                low: i32::MIN as i64,
                high: i32::MAX as i64,
            },
            IntegerBound {
                type_name: "positiveInt",
                low: 1,
                high: i32::MAX as i64,
            },
            IntegerBound {
                type_name: "unsignedInt",
                low: 0,
                high: i32::MAX as i64,
            },
        ];
        BOUNDS
            .iter()
            .find(|bound| bound.type_name == type_name)
            .copied()
    }

    /// The message for a value outside the range, or `None` when it is inside
    /// it. A value that is not an integer literal at all is `invalidValue`,
    /// which is task 21's, so it is not reported here.
    fn violation(&self, lexical: &str) -> Option<String> {
        if !is_integer_literal(lexical) {
            return None;
        }
        let out_of_range = match lexical.parse::<i64>() {
            Ok(value) => value < self.low || value > self.high,
            // Too many digits for an `i64` is beyond any of the three ranges,
            // all of which fit in an `i32`.
            Err(_) => true,
        };
        out_of_range.then(|| {
            format!(
                "{lexical} is out of range for {}, whose values run from {} to {}",
                self.type_name, self.low, self.high
            )
        })
    }
}

fn is_integer_literal(lexical: &str) -> bool {
    let digits = lexical.strip_prefix('-').unwrap_or(lexical);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// Read a decimal from its lexical form, with no binary float anywhere on the
/// path (`model.json.010`).
fn decimal_of(lexical: &str) -> Decimal {
    Decimal {
        lexical: lexical.to_string(),
        full_precision: full_precision(lexical),
        significant_digits: significant_digits(lexical),
    }
}

/// Split a decimal literal into its sign, its mantissa digits, the position of
/// its decimal point within them, and its exponent.
fn parts_of(lexical: &str) -> (bool, String, usize, i64) {
    let (negative, rest) = match lexical.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, lexical.strip_prefix('+').unwrap_or(lexical)),
    };
    let (mantissa, exponent) = match rest.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().unwrap_or(0)),
        None => (rest, 0),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa, ""),
    };
    let mut digits = String::with_capacity(integer.len() + fraction.len());
    digits.push_str(integer);
    digits.push_str(fraction);
    (negative, digits, integer.len(), exponent)
}

/// The exact value, rendered positionally: the input's own digits with the
/// decimal point moved, never a float. Trailing zeros are kept, because they
/// are significant digits of the value as presented.
fn full_precision(lexical: &str) -> String {
    let (negative, digits, point, exponent) = parts_of(lexical);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return lexical.to_string();
    }
    let point = point as i64 + exponent;
    let mut out = String::with_capacity(digits.len() + 8);
    if negative {
        out.push('-');
    }
    if point <= 0 {
        out.push_str("0.");
        for _ in 0..-point {
            out.push('0');
        }
        out.push_str(&digits);
    } else if point as usize >= digits.len() {
        out.push_str(&digits);
        for _ in 0..(point as usize - digits.len()) {
            out.push('0');
        }
    } else {
        let (head, tail) = digits.split_at(point as usize);
        out.push_str(head);
        out.push('.');
        out.push_str(tail);
    }
    out
}

/// Significant digits, counted as `path.literal.006` counts them: the digits
/// of the mantissa, with the leading zeros of a value below one excluded.
fn significant_digits(lexical: &str) -> u32 {
    let (_, digits, point, _) = parts_of(lexical);
    let trimmed = if point == 0 || (digits.starts_with('0') && point <= 1) {
        digits.trim_start_matches('0')
    } else {
        digits.as_str()
    };
    trimmed.len().max(1) as u32
}
