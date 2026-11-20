//! Eager JSON decoding: structure, and primitive values.
//!
//! Derived from hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/ParserState.java
//! (`PrimitiveState.attributeValue`, `enteringNewElement`,
//! `ContainedResourcesStateHl7Org.wereBack`, `weaveContainedResources` and
//! the collection of every instantiated reference),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java
//! (`parseChildren`'s value, `null` and alternate branches, the
//! `resourceType` check, and `encodeResourceToJsonStreamWriter`'s
//! not-contained guard on `containResourcesInReferences`),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/LenientErrorHandler.java
//! (which conditions are fatal, and its message texts),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/StrictErrorHandler.java
//! (its `Msg.code`s and message texts),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/ParseLocation.java
//! (`toString`), hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/IParserErrorHandler.java
//! (the set of conditions),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/BaseParser.java
//! (`determineReferenceText`, `isValidInternalReference`),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/util/FhirTerser.java
//! (`ContainedResources.referenceMatchesAContainedResource`),
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/context/FhirContext.java
//! (the default handler) and
//! hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/util/UrlUtil.java
//! (`sanitizeUrlPart` and `isNeedsSanitization`, transcribed as
//! `sanitize_url_part`), all at 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright (C) 2014 - 2026 Smile CDR, Inc., licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: re-implemented in Rust as a recursive
//! decoder over the input bytes rather than Jackson's tree driving
//! `ParserState`'s state stack; a handler callback becomes a [`Diagnostic`]
//! or [`CodedDiagnostic`] recorded beside the resource (lenient) or returned
//! as an error (strict) instead of a logged warning or a thrown
//! `DataFormatException`; each report also carries the element path and byte
//! offset; and `sanitize_url_part` iterates Unicode scalar values rather than
//! UTF-16 code units, which keeps the same output. The R4
//! `org.hl7.fhir.r4.formats.JsonParser` named below is the behavioural
//! reference for the decoded shape only: it is per-type generated Java, and
//! nothing here is transcribed from it, since the decoder is driven by the
//! generated element tables.
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
//! Tasks 21, 21a, 22 and 26 added the diagnostic channel over that: every
//! condition `IParserErrorHandler` enumerates, each one carrying the FHIR
//! element path and the byte offset it was found at, recorded beside the
//! resource in lenient mode and raised as an error in strict mode.
//!
//! Two of the ten conditions carry a HAPI message code that a [`ReportKind`]
//! cannot name — `containedResourceWithNoId` (1819) and `unknownReference`
//! (1826) — because they are not about an element's own JSON at all but about
//! the document's internal reference graph, which HAPI resolves in a
//! post-decode pass of its own (`ParserState.weaveContainedResources`). They
//! are therefore reported through [`CodedDiagnostic`] and raised by
//! [`decode_document`]; [`decode`] is the narrower surface that raises only the
//! [`Diagnostic`]-typed conditions. The tenth, `invalidInternalReference`
//! (2724), is on the **encode** side and is task 23a's.

use super::built::ordered_elements;
use super::scan::{Span, skip_ws, unescape};
use super::tables::{ElementDef, JsonForm, Tables, TypeKind};

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
    /// `IParserErrorHandler.unexpectedRepeatingElement` (`model.json.015`,
    /// `codegen.choice.004`).
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

impl ReportKind {
    /// HAPI's `Msg.code` for this condition, where HAPI has a callback for it.
    ///
    /// Read from `ca.uhn.fhir.parser.StrictErrorHandler` at pin
    /// `7ab4c7c7c49281496faa1b3c2498a652457669ae`: `incorrectJsonType` 1820
    /// (line 52), `invalidValue` 1821 (57), `missingRequiredElement` 1822 (72),
    /// `unexpectedRepeatingElement` 1823 (77), `unknownAttribute` 1824 (83),
    /// `unknownElement` 1825 (89), `unknownReference` 1826 (96),
    /// `invalidInternalReference` 2724 (101) and
    /// `extensionContainsValueAndNestedExtensions` 1827 (109). The missing
    /// `resourceType` is not a handler callback at all: `JsonParser` throws it
    /// directly as 1843 (`JsonParser.java:1483`).
    ///
    /// `None` is the honest answer for the rest, and it is not an oversight:
    /// a duplicate JSON key, a `null` in a non-primitive array, a numeric value
    /// outside its declared range, unequal `_field` array lengths, a `_field`
    /// property on a complex element, an unknown `resourceType`, non-UTF-8
    /// input and the depth bound have no `IParserErrorHandler` callback and so
    /// no code to match. Inventing one would make a Frevo-only report look like
    /// a HAPI one in a differential diff.
    pub fn message_code(self) -> Option<u32> {
        match self {
            ReportKind::MissingResourceType => Some(1843),
            ReportKind::IncorrectJsonType => Some(1820),
            ReportKind::UnexpectedRepeatingElement => Some(1823),
            ReportKind::UnknownProperty => Some(1825),
            ReportKind::InvalidValue => Some(1821),
            ReportKind::ExtensionHasValueAndNestedExtensions => Some(1827),
            ReportKind::UnknownResourceType
            | ReportKind::DuplicateProperty
            | ReportKind::NullInNonPrimitiveArray
            | ReportKind::ValueOutOfRange
            | ReportKind::NotUtf8
            | ReportKind::PrimitiveExtensionLengthMismatch
            | ReportKind::UnexpectedUnderscoreProperty
            | ReportKind::DepthLimit => None,
        }
    }

    /// Whether this condition aborts decoding in **both** modes.
    ///
    /// `LenientErrorHandler`'s class documentation names exactly two: its
    /// `invalidValue` delegates to the strict handler unless the value is blank
    /// (`LenientErrorHandler.java:100-108`) and its
    /// `extensionContainsValueAndNestedExtensions` delegates unconditionally
    /// (`:224-229`). Everything else logs a warning and returns.
    pub fn is_fatal_in_lenient_mode(self) -> bool {
        matches!(
            self,
            ReportKind::InvalidValue | ReportKind::ExtensionHasValueAndNestedExtensions
        )
    }
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
    /// The type the element declares, where the condition has one. For the
    /// two shape conditions — a repeating element given a bare object, a
    /// `_field` given as a scalar — the declared *JSON* type is the thing that
    /// was expected, so this reads `array` or `object` there and the FHIR type
    /// name everywhere else, which is what `model.json.013` and
    /// `model.json.014` assert.
    pub expected_type: Option<String>,
    /// The JSON type actually found, where the condition has one.
    pub found_type: Option<String>,
    /// HAPI's message code, where the condition is one HAPI has a callback
    /// for. See [`ReportKind::message_code`].
    pub code: Option<u32>,
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
            code: kind.message_code(),
        }
    }

    fn expecting(mut self, type_name: &str) -> Self {
        self.expected_type = Some(type_name.to_string());
        self
    }

    fn finding(mut self, json_type: &str) -> Self {
        self.found_type = Some(json_type.to_string());
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

/// One of the three conditions about the document's internal reference graph
/// rather than about an element's own JSON: two on the decode side
/// (`model.json.024`, `model.json.025`) and the one on the encode side
/// (`model.canonical.007`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceIssue {
    /// A `contained` entry carrying no `id`.
    ///
    /// `ParserState.ContainedResourcesStateHl7Org.wereBack()` at
    /// `ParserState.java:433-454`. The comment at lines 442-443 reads "If there
    /// is no ID, we don't keep the resource because it's useless", and then the
    /// code appends it to `contained` anyway at line 452 — outside the `else`.
    /// What the `else` branch actually guards is the `put` into the
    /// id-to-resource map at lines 446-449. So the resource survives and no
    /// `#` reference can ever be woven to it, which is the opposite of what the
    /// comment claims.
    ContainedResourceWithNoId,
    /// A `#` reference, of at least one character after the `#`, whose target
    /// is not in the id-to-resource map.
    ///
    /// `ParserState.weaveContainedResources()` at `ParserState.java:1277-1293`.
    UnknownReference,
    /// **Encoding** a `#` reference whose id part is not blank, which is not
    /// exactly `#`, and which matches no resource contained in the resource
    /// being written. The only error condition on the encode side.
    ///
    /// `BaseParser.determineReferenceText` and `isValidInternalReference` at
    /// `BaseParser.java:245-268`, matched through
    /// `FhirTerser.ContainedResources.referenceMatchesAContainedResource` at
    /// `FhirTerser.java:1910-1916`. Never raised by a decode.
    InvalidInternalReference,
}

impl ReferenceIssue {
    /// HAPI's `Msg.code`: `StrictErrorHandler.java:38-40`, `:93-97` and
    /// `:99-105`.
    pub fn message_code(self) -> u32 {
        match self {
            ReferenceIssue::ContainedResourceWithNoId => 1819,
            ReferenceIssue::UnknownReference => 1826,
            ReferenceIssue::InvalidInternalReference => 2724,
        }
    }
}

/// A [`ReferenceIssue`] as reported: HAPI's code, and the element path and
/// byte offset `model.errors.001` and `model.errors.002` require of every
/// report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodedDiagnostic {
    pub issue: ReferenceIssue,
    pub code: u32,
    pub path: String,
    pub offset: usize,
    pub message: String,
}

impl CodedDiagnostic {
    fn new(issue: ReferenceIssue, path: &str, offset: usize, message: String) -> Self {
        Self {
            issue,
            code: issue.message_code(),
            path: path.to_string(),
            offset,
            message,
        }
    }
}

impl CodedDiagnostic {
    /// The encode-side condition for `reference`, worded as the handler in
    /// `mode` words it: `LenientErrorHandler.invalidInternalReference` logs
    /// one text and `StrictErrorHandler.invalidInternalReference` raises
    /// another (`LenientErrorHandler.java:214-221`,
    /// `StrictErrorHandler.java:99-105`).
    fn invalid_internal_reference(reference: &LocalReference, mode: ParseMode) -> Self {
        let text = &reference.text;
        let message = match mode {
            ParseMode::Lenient => format!(
                "Resource has a reference that begins with '#', but no resource with this ID \
                 is contained. [Reference=\"{text}\"]"
            ),
            ParseMode::Strict => format!(
                "There is a reference that begins with #, but no resource with this ID is \
                 contained. [reference={text}]"
            ),
        };
        Self::new(
            ReferenceIssue::InvalidInternalReference,
            &reference.path,
            reference.offset,
            message,
        )
    }
}

/// Hand `bytes` back through the encode-side check: in strict mode the first
/// dangling reference is raised, in lenient mode every one is returned beside
/// the bytes, which are the same bytes either way.
pub(crate) fn encoded(
    bytes: Vec<u8>,
    dangling: &[LocalReference],
    mode: ParseMode,
) -> Result<Encoded, CodedDiagnostic> {
    if mode == ParseMode::Strict {
        if let Some(first) = dangling.first() {
            return Err(CodedDiagnostic::invalid_internal_reference(first, mode));
        }
    }
    Ok(Encoded {
        bytes,
        diagnostics: dangling
            .iter()
            .map(|reference| CodedDiagnostic::invalid_internal_reference(reference, mode))
            .collect(),
    })
}

/// A resource serialised through the encode-side check: the bytes, and the
/// diagnostics lenient mode returns beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    /// Every `invalidInternalReference` condition, in document order. Empty
    /// in strict mode, which raises the first instead.
    pub diagnostics: Vec<CodedDiagnostic>,
}

impl std::fmt::Display for CodedDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} at {} (byte {})",
            self.code, self.message, self.path, self.offset
        )
    }
}

impl std::error::Error for CodedDiagnostic {}

/// What a decode can fail with. Strict mode raises whichever report came
/// first; the two fatal conditions raise in both modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeFailure {
    /// One of the conditions a [`ReportKind`] names.
    Report(Diagnostic),
    /// One of the two conditions about the internal reference graph.
    Coded(CodedDiagnostic),
}

impl std::fmt::Display for DecodeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeFailure::Report(report) => report.fmt(f),
            DecodeFailure::Coded(coded) => coded.fmt(f),
        }
    }
}

impl std::error::Error for DecodeFailure {}

impl From<Diagnostic> for DecodeFailure {
    fn from(report: Diagnostic) -> Self {
        DecodeFailure::Report(report)
    }
}

/// A local (`#`) reference the decode collected, and what the weaving pass did
/// with it.
///
/// `ParserState` collects **every** `IBaseReference` it instantiates
/// (`ParserState.java:214-222` and `:225-233`), not only the `#` ones, and
/// `weaveContainedResources` then filters. This mirrors that, so that the text
/// of a non-local reference is recorded too and the filtering rule stays in one
/// place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalReference {
    /// The element the reference sits on: `Patient.managingOrganization`.
    pub path: String,
    /// The reference's literal text, which survives in both modes — it is what
    /// `path.utility.017` later resolves against.
    pub text: String,
    /// The byte offset of the reference value in the input.
    pub offset: usize,
    /// Whether the post-decode weaving pass found a contained resource for it.
    pub resolved: bool,
}

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
    /// The same tree re-emitted with every object's property names sorted
    /// lexicographically and no whitespace, which is what
    /// `org.hl7.fhir.r4.formats.JsonCreatorCanonical` produces. The canonical
    /// form of a byte-backed resource is [`crate::Resource::write_canonical`];
    /// this is the eager tree's own, so that what survived a decode is
    /// assertable rather than described.
    pub reencoded_canonical: Vec<u8>,
    /// The `contained` entries in document order, with `None` where the entry
    /// carried no `id` (`model.json.024`).
    pub contained_ids: Vec<Option<String>>,
    /// The ids a `#` reference can be woven to: the keys of the id-to-resource
    /// map, which is deliberately **not** the same list as [`Self::contained_ids`].
    pub referenceable_contained_ids: Vec<String>,
    /// Every reference the decode collected, with the weaving pass's verdict.
    pub local_references: Vec<LocalReference>,
    /// The reference-graph diagnostics, in the order HAPI raises them: the
    /// id-less `contained` entries as they are closed, then the weaving pass.
    pub reference_diagnostics: Vec<CodedDiagnostic>,
    /// The `#` references that serialising this tree finds no contained
    /// resource for, in document order (`model.canonical.007`). Each is
    /// checked against the `contained` list of the nearest enclosing resource
    /// that is not itself contained, which is the set HAPI's encoder holds
    /// when it writes that reference. [`Decoded::encode_canonical`] and
    /// [`Decoded::encode_stored`] turn these into diagnostics or an error.
    pub dangling_internal_references: Vec<LocalReference>,
}

impl Decoded {
    /// Serialise canonically through the encode-side check. Lenient mode
    /// returns the bytes with one diagnostic per dangling `#` reference;
    /// strict mode fails on the first with HAPI's code `2724`. The reference
    /// is emitted unchanged either way (`model.canonical.007`).
    pub fn encode_canonical(&self, mode: ParseMode) -> Result<Encoded, CodedDiagnostic> {
        encoded(
            self.reencoded_canonical.clone(),
            &self.dangling_internal_references,
            mode,
        )
    }

    /// [`Self::encode_canonical`] for the stored form, generated element
    /// order. The same references are written either way, so the same
    /// condition is raised.
    pub fn encode_stored(&self, mode: ParseMode) -> Result<Encoded, CodedDiagnostic> {
        encoded(
            self.reencoded.clone(),
            &self.dangling_internal_references,
            mode,
        )
    }

    /// The stored form -- generated element order -- with indentation: two
    /// spaces per level, one member or item per line. Only whitespace is
    /// added, so it differs from [`Self::reencoded`] in nothing else and from
    /// [`Self::reencoded_canonical`] only in whitespace and key order
    /// (`model.canonical.006`).
    pub fn reencoded_pretty(&self) -> Vec<u8> {
        super::pretty::indent(&self.reencoded)
    }

    /// Whether every element of the two resources is equal: the same
    /// resource type and the same element occurrences, each with the same
    /// type, value, element id and extensions, whatever order the two
    /// documents happened to spell their properties in.
    ///
    /// Elements are compared by path rather than in document order, because a
    /// canonical and a stored document list the same elements in different
    /// orders and that difference is the one this comparison exists to ignore.
    /// Values are compared by their lexical form, so `1.50` and `1.5` are
    /// different elements, which is what `model.canonical.003` requires of the
    /// codec everywhere else.
    pub fn elements_equal(&self, other: &Decoded) -> bool {
        if self.resource_type != other.resource_type || self.elements.len() != other.elements.len()
        {
            return false;
        }
        let mut left: Vec<&DecodedElement> = self.elements.iter().collect();
        let mut right: Vec<&DecodedElement> = other.elements.iter().collect();
        left.sort_by(|a, b| a.path.cmp(&b.path));
        right.sort_by(|a, b| a.path.cmp(&b.path));
        left == right
    }
}

/// Decode `bytes` in `mode`, using the generated R4 tables.
///
/// This raises every condition a [`ReportKind`] names. The two conditions
/// about the document's internal reference graph are recorded in
/// [`Decoded::reference_diagnostics`] but **not** raised here, because a
/// [`Diagnostic`] has no `ReportKind` that could name them;
/// [`decode_document`] is the surface that raises those too, and is what a
/// caller deciding whether to accept a request body should use.
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
    let decoded = run(bytes, tables)?;
    if mode == ParseMode::Strict {
        // Strict mode raises the diagnostic lenient mode records -- same kind,
        // same path, same offset, same message -- and changes nothing else, so
        // the two modes cannot drift apart. The error is the first problem in
        // document order (`model.errors.003`).
        if let Some(first) = decoded.diagnostics.first() {
            return Err(first.clone());
        }
    }
    Ok(decoded)
}

/// Decode `bytes` in `mode`, raising **every** condition HAPI's handler
/// enumerates on the decode side, including the two that carry a message code
/// (`model.json.024`, `model.json.025`).
pub fn decode_document(bytes: &[u8], mode: ParseMode) -> Result<Decoded, DecodeFailure> {
    decode_document_with(bytes, mode, super::R4)
}

/// [`decode_document`] against a caller-supplied table.
pub fn decode_document_with(
    bytes: &[u8],
    mode: ParseMode,
    tables: Tables,
) -> Result<Decoded, DecodeFailure> {
    let decoded = run(bytes, tables)?;
    if mode == ParseMode::Strict {
        if let Some(first) = first_report(&decoded) {
            return Err(first);
        }
    }
    Ok(decoded)
}

/// The first problem in the order HAPI's handler would have seen it: the
/// element-level reports and the id-less `contained` entries interleaved by
/// byte offset, then the weaving pass, which by construction runs after the
/// whole document is decoded.
fn first_report(decoded: &Decoded) -> Option<DecodeFailure> {
    let report = decoded.diagnostics.first();
    let coded = decoded
        .reference_diagnostics
        .iter()
        .find(|coded| coded.issue == ReferenceIssue::ContainedResourceWithNoId);
    let earliest = match (report, coded) {
        (Some(report), Some(coded)) if coded.offset < report.offset => {
            Some(DecodeFailure::Coded(coded.clone()))
        }
        (Some(report), _) => Some(DecodeFailure::Report(report.clone())),
        (None, Some(coded)) => Some(DecodeFailure::Coded(coded.clone())),
        (None, None) => None,
    };
    earliest.or_else(|| {
        decoded
            .reference_diagnostics
            .iter()
            .find(|coded| coded.issue == ReferenceIssue::UnknownReference)
            .cloned()
            .map(DecodeFailure::Coded)
    })
}

/// Everything both entry points share: the decode itself, which is
/// mode-independent apart from the two conditions that abort in both modes.
fn run(bytes: &[u8], tables: Tables) -> Result<Decoded, Diagnostic> {
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
        fatal: None,
        contained_ids: Vec::new(),
        referenceable_contained_ids: Vec::new(),
        local_references: Vec::new(),
        reference_diagnostics: Vec::new(),
    };
    let root = decoder.shape_resource(&raw, None)?;
    // The two conditions `LenientErrorHandler` treats as fatal abort in both
    // modes, so they are raised before anything is handed back, and before the
    // work of re-emitting a resource the caller will not receive.
    if let Some(fatal) = decoder.fatal.take() {
        return Err(fatal);
    }
    let resource_type = root.type_name.to_string();

    let mut reencoded = Vec::with_capacity(bytes.len());
    decoder.emit_object(&root, &mut reencoded);
    let mut reencoded_canonical = Vec::with_capacity(bytes.len());
    decoder.emit_object_canonical(&root, &mut reencoded_canonical);

    // The element walk also collects the `contained` entries and the
    // references, so it has to precede the weaving pass.
    let mut elements = Vec::new();
    decoder.flatten_object(&root, &resource_type, true, None, &mut elements);

    // The weaving pass runs after the whole document is decoded, which is why
    // a reference that appears before its target still resolves
    // (`ParserState.weaveContainedResources`, called from `wereBack`).
    decoder.weave();

    // What the encoder will find when it writes this tree back out. Computed
    // here, once, because both re-encodings above write the same references.
    let mut dangling_internal_references = Vec::new();
    decoder.check_resource(&root, &resource_type, &mut dangling_internal_references);

    Ok(Decoded {
        resource_type,
        diagnostics: decoder.diagnostics,
        elements,
        reencoded,
        reencoded_canonical,
        contained_ids: decoder.contained_ids,
        referenceable_contained_ids: decoder.referenceable_contained_ids,
        local_references: decoder.local_references,
        reference_diagnostics: decoder.reference_diagnostics,
        dangling_internal_references,
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

    /// The primitive this value is, when it has a value. A valueless
    /// primitive answers `None`, as `hasValue()` would upstream.
    fn primitive(&self) -> Option<Primitive> {
        match self {
            Value::Primitive(primitive) if !primitive.valueless => Some(*primitive),
            Value::Primitive(_) | Value::Object(_) => None,
        }
    }

    /// Whether this is a primitive that holds no value.
    fn is_valueless(&self) -> bool {
        matches!(self, Value::Primitive(primitive) if primitive.valueless)
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
///
/// `valueless` is set when `ParserState` would have reported the text through
/// a non-fatal `invalidValue` and set nothing: a `""` value, or a
/// whitespace-only one its type refuses (`ParserState.java:1454-1514`,
/// `LenientErrorHandler.java:100-108`). The element is still there — it may
/// carry an `id` or extensions on its `_field` side — but it has no value, so
/// the element list reports none and re-emission writes none
/// (`codegen.cardinality.005`, `model.json.023`).
#[derive(Debug, Clone, Copy)]
struct Primitive {
    raw: Span,
    valueless: bool,
}

#[derive(Debug)]
struct Object {
    type_name: &'static str,
    is_resource: bool,
    /// The byte offset of the object's own `{`, so that a report about the
    /// object rather than about one of its properties has somewhere to point.
    start: usize,
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

/// The element a value is being shaped for: the type that owns it, its
/// definition, and the JSON property name it was read under -- which is what
/// `ParseLocation` names (`ParserState.java:1455`, `:1483`), the typed name for
/// a choice (`valueString`).
#[derive(Debug, Clone, Copy)]
struct Field<'k> {
    owner: &'static str,
    def: &'static ElementDef,
    key: &'k [u8],
}

/// `StrictErrorHandler.unexpectedRepeatingElement`'s text after its code, with
/// no location, which is how `ParserState.enteringNewElement` calls it
/// (`StrictErrorHandler.java:76-79`, `ParserState.java:669`).
fn repeated_element_message(name: &str) -> String {
    format!("Multiple repetitions of non-repeatable element '{name}' found during parse")
}

/// `ca.uhn.fhir.util.UrlUtil.sanitizeUrlPart(CharSequence)`: HTML-encode the
/// six characters it names and drop every other control character
/// (`UrlUtil.java:510-560`).
fn sanitize_url_part(text: &str) -> std::borrow::Cow<'_, str> {
    if !text
        .chars()
        .any(|c| matches!(c, '\'' | '"' | '<' | '>') || c < ' ')
    {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 10);
    for c in text.chars() {
        match c {
            '\'' => out.push_str("&apos;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c if c >= ' ' => out.push(c),
            _ => {}
        }
    }
    std::borrow::Cow::Owned(out)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    tables: Tables,
    diagnostics: Vec<Diagnostic>,
    /// The first condition that aborts in both modes, held rather than
    /// returned so that shaping unwinds through one path.
    fatal: Option<Diagnostic>,
    contained_ids: Vec<Option<String>>,
    referenceable_contained_ids: Vec<String>,
    local_references: Vec<LocalReference>,
    reference_diagnostics: Vec<CodedDiagnostic>,
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
        Ok(self.shape_members(def.name, true, *start, members, at.unwrap_or(&own)))
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
        let Raw::Object { members, start } = raw else {
            // A scalar or an array where a complex element belongs. HAPI enters
            // the element and then reports each of the object's would-be
            // properties through `attributeValue`
            // (`ParserState.java:607-623`); the element itself is dropped
            // either way, and the properties it does not have cannot be
            // enumerated, so the report is made once about the element.
            self.report(
                Diagnostic::new(
                    ReportKind::IncorrectJsonType,
                    &at.render(),
                    raw.start(),
                    format!(
                        "{declared} is a complex element, so a JSON object was expected, but a {} was found",
                        self.json_type_of(raw)
                    ),
                )
                .expecting("object")
                .finding(self.json_type_of(raw)),
            );
            return None;
        };
        let object = self.shape_members(declared, false, *start, members, at);
        self.check_extension(&object, at);
        Some(object)
    }

    fn shape_members(
        &mut self,
        type_name: &'static str,
        is_resource: bool,
        start: usize,
        members: &[(Span, Raw)],
        at: &Loc<'_>,
    ) -> Object {
        let mut props: Vec<Prop> = Vec::new();
        for (key, raw) in members {
            let key_bytes = key.of(self.bytes);
            if key_bytes == b"resourceType" {
                continue;
            }
            // The quote-inclusive start of the property name, which is where a
            // report about the property itself points.
            let key_offset = key.start.saturating_sub(1);
            let Some((def, concrete, is_sibling)) =
                self.tables.resolve_property_bytes(type_name, key_bytes)
            else {
                // `ParserState.enteringNewElement` at `:638-658`: the element
                // is swallowed whole along with its children, and the handler
                // is told. `LenientErrorHandler.unknownElement` logs and
                // returns, so the property is simply dropped
                // (`model.json.004`).
                let name = String::from_utf8_lossy(key_bytes).into_owned();
                let path = at.child(&name).render();
                self.report(Diagnostic::new(
                    ReportKind::UnknownProperty,
                    &path,
                    key_offset,
                    format!("unknown property '{name}' on {type_name}"),
                ));
                continue;
            };
            // `_field` is the primitive-extension form and nothing else.
            // `ElementDef` carries the element's declared type, so a `_name`
            // on a complex element is reported rather than quietly accepted
            // (`model.primext.009`).
            if is_sibling && !self.tables.is_primitive(concrete) {
                let name = String::from_utf8_lossy(key_bytes).into_owned();
                let path = at.child(def.name).render();
                self.report(Diagnostic::new(
                    ReportKind::UnexpectedUnderscoreProperty,
                    &path,
                    key_offset,
                    format!(
                        "'{name}' is the primitive-extension form, but {}.{} is a {concrete}",
                        type_name, def.name
                    ),
                ));
                continue;
            }
            let choice = def.choice.then_some(concrete);
            let here = at.child(def.name);
            // `ParserState.enteringNewElement` at `:660-672` records each
            // `0..1` child it enters under the choice's *field* name --
            // `RuntimeChildChoiceDefinition.getField().getName()`, `value` for
            // every `value[x]` property -- so a second property spelling a
            // different type of the same choice is a repetition. The handler is
            // told with no location, and the property is swallowed whole with
            // everything under it (`SwallowChildrenWholeState`, `:670`), so the
            // first in document order is the value (`codegen.choice.004`).
            if def.choice
                && !def.repeating
                && !is_sibling
                && props.iter().any(|prop| {
                    prop.def.name == def.name && prop.choice != choice && prop.value.is_some()
                })
            {
                self.report(Diagnostic::new(
                    ReportKind::UnexpectedRepeatingElement,
                    &here.render(),
                    key_offset,
                    repeated_element_message(def.name),
                ));
                continue;
            }
            // A `_field` sibling carries only the `id` and `extension` of
            // `Element`, whatever the element's own type is.
            let item_type: &'static str = if is_sibling { "Element" } else { concrete };
            let field = Field {
                owner: type_name,
                def,
                key: key_bytes,
            };
            let shape = self.shape_side(field, item_type, raw, &here, is_sibling);
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
            let side = if is_sibling {
                &props[index].sibling
            } else {
                &props[index].value
            };
            if side.is_some() {
                // A repeated property name silently overwrites, because HAPI
                // reads JSON through a Jackson `ObjectMapper` built without
                // `FAIL_ON_READING_DUP_TREE_KEY` and without
                // `STRICT_DUPLICATE_DETECTION`, so the last occurrence wins.
                // HAPI has no error-handler callback for this at all, so the
                // report is a Frevo-only one (`model.json.016`).
                let name = String::from_utf8_lossy(key_bytes).into_owned();
                let path = here.render();
                self.report(Diagnostic::new(
                    ReportKind::DuplicateProperty,
                    &path,
                    key_offset,
                    format!(
                        "the property '{name}' appears more than once; the last occurrence wins"
                    ),
                ));
            }
            if is_sibling {
                props[index].sibling = Some(shape);
            } else {
                props[index].value = Some(shape);
            }
            // The two sides of one element are index-aligned, so once both are
            // known their lengths are comparable. Checked here rather than
            // after the loop so that the report lands in document order.
            self.check_sibling_lengths(&props[index], &here, key_offset);
        }
        Object {
            type_name,
            is_resource,
            start,
            props,
        }
    }

    /// One side of an element: the JSON shape it took, with each item decoded
    /// as `item_type`.
    fn shape_side(
        &mut self,
        field: Field<'_>,
        item_type: &'static str,
        raw: &Raw,
        at: &Loc<'_>,
        is_sibling: bool,
    ) -> Shape {
        let def = field.def;
        match raw {
            Raw::Array { items, start } => {
                // `ParserState.enteringNewElement` at `:662-672` adds the
                // element name to `myParsedNonRepeatableNames` and reports the
                // second and later repetitions of a `0..1` element, swallowing
                // them -- so the first is kept and an array of one is not a
                // problem at all (`model.json.015`). The `_field` side is
                // governed by `parseAlternates` instead, whose rule is
                // different (`JsonParser.java:1283-1293` throws for an array
                // longer than one), so it is not reported here.
                if !def.repeating && !is_sibling && items.len() > 1 {
                    self.report(Diagnostic::new(
                        ReportKind::UnexpectedRepeatingElement,
                        &at.render(),
                        *start,
                        repeated_element_message(def.name),
                    ));
                    let first = self.shape_item(item_type, &items[0], &at.at(None), field);
                    return Shape::Array(vec![first]);
                }
                let mut out = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    let here = at.at(def.repeating.then_some(index));
                    // `null` is meaningful as a hole in a *primitive* array,
                    // where it pairs with the `_field` entry at the same index.
                    // In an array of a complex element it means nothing, and
                    // HAPI's own null branch (`JsonParser.java:1484-1488`)
                    // enters the element and ends it, which is what happens
                    // here too -- the report is Frevo's own, HAPI having no
                    // callback for it (`model.json.018`).
                    if !is_sibling && !self.tables.is_primitive(item_type) && self.is_null(item) {
                        self.report(Diagnostic::new(
                            ReportKind::NullInNonPrimitiveArray,
                            &at.render(),
                            item.start(),
                            format!(
                                "null is meaningful only as a hole in a primitive array paired \
                                 with _{}, and {} is a {item_type}",
                                def.name, def.name
                            ),
                        ));
                    }
                    out.push(self.shape_item(item_type, item, &here, field));
                }
                Shape::Array(out)
            }
            single => {
                // `JsonParser.parseChildren` at `:1460-1462`: a bare object
                // outside an array where a repeating element belongs is
                // `incorrectJsonType`, expecting an array. The check is on the
                // object case only, which is HAPI's: a bare *scalar* where a
                // repeating primitive belongs reaches the scalar branch with no
                // check at all (`model.json.014`).
                if def.repeating && !is_sibling && matches!(single, Raw::Object { .. }) {
                    self.report(
                        Diagnostic::new(
                            ReportKind::IncorrectJsonType,
                            &at.render(),
                            single.start(),
                            format!(
                                "{} may repeat, so a JSON array was expected, but a single \
                                 object was found",
                                def.name
                            ),
                        )
                        .expecting("array")
                        .finding("object"),
                    );
                }
                let here = at.at(def.repeating.then_some(0));
                Shape::Single(self.shape_item(item_type, single, &here, field))
            }
        }
    }

    fn is_null(&self, raw: &Raw) -> bool {
        matches!(raw, Raw::Scalar(span) if span.of(self.bytes) == b"null")
    }

    /// The JSON type name of a raw value, as `BaseJsonLikeValue.ValueType` and
    /// `ScalarType` together name it: a string, a number and a boolean are
    /// distinguished, because `model.json.013` asserts the found type.
    fn json_type_of(&self, raw: &Raw) -> &'static str {
        match raw {
            Raw::Object { .. } => "object",
            Raw::Array { .. } => "array",
            Raw::Scalar(span) => match span.of(self.bytes) {
                b"true" | b"false" => "boolean",
                b"null" => "null",
                bytes if bytes.first() == Some(&b'"') => "string",
                _ => "number",
            },
        }
    }

    /// One item of one side. `None` is a `null` hole, which means "nothing at
    /// this index" for this side and nothing more.
    fn shape_item(
        &mut self,
        item_type: &'static str,
        raw: &Raw,
        at: &Loc<'_>,
        field: Field<'_>,
    ) -> Option<Value> {
        if let Raw::Scalar(span) = raw {
            if span.of(self.bytes) == b"null" {
                return None;
            }
        }
        if self.tables.is_primitive(item_type) {
            let Raw::Scalar(span) = raw else {
                self.report(
                    Diagnostic::new(
                        ReportKind::IncorrectJsonType,
                        &at.render(),
                        raw.start(),
                        format!(
                            "{item_type} is a primitive, so a JSON scalar was expected, but a {} \
                             was found",
                            self.json_type_of(raw)
                        ),
                    )
                    .expecting(item_type)
                    .finding(self.json_type_of(raw)),
                );
                return None;
            };
            // A primitive's declared JSON form is part of its type. HAPI reads
            // the scalar's **string** form whatever the JSON type was --
            // `JsonParser.parseChildren` at `:1489-1492` calls
            // `attributeValue("value", theJsonVal.getAsString())` -- so the
            // value survives, which is why `"active": "true"` yields the value
            // `true` and a diagnostic rather than a dropped element
            // (`model.json.013`).
            let found = self.json_type_of(raw);
            let expected = json_form_name(self.tables.json_form(item_type));
            if found != expected {
                self.report(
                    Diagnostic::new(
                        ReportKind::IncorrectJsonType,
                        &at.render(),
                        span.start,
                        format!(
                            "{item_type} is written as a JSON {expected}, but a {found} was found"
                        ),
                    )
                    .expecting(item_type)
                    .finding(found),
                );
            }
            return Some(Value::Primitive(
                self.shape_primitive(item_type, *span, at, field),
            ));
        }
        self.shape_object(item_type, raw, at).map(Value::Object)
    }

    /// A primitive value. Its bytes are kept verbatim for re-emission; its
    /// lexical form is the string's text or the number's or boolean's own
    /// characters.
    fn shape_primitive(
        &mut self,
        type_name: &str,
        span: Span,
        at: &Loc<'_>,
        field: Field<'_>,
    ) -> Primitive {
        let has_value = self.check_value(type_name, span, at, field);
        let primitive = Primitive {
            raw: span,
            valueless: !has_value,
        };
        if !has_value {
            return primitive;
        }
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

    // -----------------------------------------------------------------------
    // The diagnostic channel
    // -----------------------------------------------------------------------

    /// Record one report. A condition `LenientErrorHandler` treats as fatal is
    /// held aside rather than recorded, so that it aborts the decode in both
    /// modes; only the first is kept, which is the one HAPI would have thrown
    /// at (`model.errors.003`).
    fn report(&mut self, diagnostic: Diagnostic) {
        if diagnostic.kind.is_fatal_in_lenient_mode() {
            if self.fatal.is_none() {
                self.fatal = Some(diagnostic);
            }
            return;
        }
        self.diagnostics.push(diagnostic);
    }

    /// Check a primitive's value the way `ParserState.PrimitiveState.attributeValue`
    /// does (`ParserState.java:1452-1514`), and answer whether the element
    /// holds a value afterwards (`model.json.023`, `codegen.cardinality.005`,
    /// `codegen.binding.002`).
    ///
    /// In HAPI's order:
    ///
    /// 1. A value of exactly `""` is reported through `invalidValue` with
    ///    `Attribute value must not be empty ("")` before anything is set
    ///    (`:1454-1457`).
    /// 2. Otherwise `setValueAsString` runs. For an element HAPI's model holds
    ///    as an `Enumeration` it is the enum factory that refuses an unknown
    ///    code, with `Unknown <Enum> code '<code>'`; a code that matches a
    ///    binding code ignoring case is reported first with the
    ///    case-discrepancy text (`:1482-1503`). The `Enumeration` set is read
    ///    from the generated required-binding enumerations,
    ///    [`crate::r4::codes`]: see `tasks/blockers.md`, "the R4 Java class
    ///    ancestry table", for why that may not yet be exactly HAPI's set.
    /// 3. For every other type the value rule of [`ValueRule`] stands in for
    ///    the Java type's parser.
    ///
    /// Which of those is fatal is `LenientErrorHandler.invalidValue`'s rule
    /// (`LenientErrorHandler.java:100-108`): a **blank** value -- `isBlank`,
    /// empty or whitespace only -- is logged and parsing continues, so it is a
    /// diagnostic, which strict mode raises as `1821`; any other invalid value
    /// is handed to the strict handler and aborts the decode in both modes. A
    /// value that was reported and not set leaves the element with no value.
    ///
    /// `xhtml` is not a `PrimitiveState` upstream (`XhtmlState` reads the div
    /// as markup), so none of this applies to it, and `base64Binary` has no
    /// rule: it is already its encoded text.
    fn check_value(&mut self, type_name: &str, span: Span, at: &Loc<'_>, field: Field<'_>) -> bool {
        if type_name == "xhtml" {
            return true;
        }
        let bytes = self.bytes;
        let raw = token_value(bytes, span);
        let quoted = bytes.get(span.start) == Some(&b'"');
        if quoted && raw.is_empty() {
            self.invalid_value(
                field,
                at,
                span.start,
                type_name,
                "",
                "Attribute value must not be empty (\"\")".to_string(),
            );
            return false;
        }
        // Resolving escapes would allocate per primitive, and none of the
        // rules below admits a backslash, so it is done only when there is one
        // to resolve -- an escape could still spell a legal character.
        let resolved = raw.contains(&b'\\').then(|| {
            self.lexical(Primitive {
                raw: span,
                valueless: false,
            })
        });
        let value = resolved.as_ref().map_or(raw, |text| text.as_bytes());

        if type_name == "code" {
            if let Some(codes) = self.enumeration_of(field) {
                let text = String::from_utf8_lossy(value);
                if codes.variant(&text).is_some() {
                    return true;
                }
                // `RuntimeChildPrimitiveEnumerationDatatypeDefinition
                // .getCodeCaseInsensitive` lower-cases with `Locale.US`.
                let lower = text.to_lowercase();
                let error = match codes
                    .codes()
                    .find(|code| code.to_lowercase() == lower)
                    .filter(|code| *code != text)
                {
                    Some(corrected) => format!(
                        "Case discrepancy detected in code value '{text}' (should be '{corrected}')"
                    ),
                    None => format!("Unknown {} code '{text}'", codes.rust_name),
                };
                let text = text.into_owned();
                return !self.invalid_value(field, at, span.start, type_name, &text, error);
            }
        }

        let Some(rule) = ValueRule::of(type_name) else {
            return true;
        };
        if rule.holds(value) {
            return true;
        }
        let text = String::from_utf8_lossy(value).into_owned();
        let error = format!("\"{text}\" is not a valid {type_name} value");
        !self.invalid_value(field, at, span.start, type_name, &text, error)
    }

    /// The required-binding enumeration of the `code` element `field` is, if
    /// HAPI holds it as an `Enumeration`. The snapshot's binding is consulted
    /// first, a binary search, so that the common unbound `code` (every
    /// `Coding.code`) costs no scan of the enumeration table.
    fn enumeration_of(&self, field: Field<'_>) -> Option<&'static crate::r4::codes::CodeEnum> {
        let meta = crate::r4::elements::element_meta(field.owner, field.def.name)?;
        if meta.binding?.strength != "required" {
            return None;
        }
        crate::r4::codes::code_enum_of_element(&format!("{}.{}", field.owner, field.def.name))
    }

    /// Report `invalidValue` for `value` on `field`, worded as
    /// `StrictErrorHandler.invalidValue` words it after its code
    /// (`StrictErrorHandler.java:56-59`): the location as
    /// `[element="<name>"] ` (`ParseLocation.java:55-57`), where the name is
    /// the JSON property the value was read under, then
    /// `Invalid attribute value "<value>": <error>` with the value passed
    /// through `UrlUtil.sanitizeUrlPart`.
    ///
    /// Returns `true` when the report is the non-fatal kind -- the value is
    /// blank -- which is when the element is left with no value; a fatal
    /// report aborts the decode, so what the element holds no longer matters.
    fn invalid_value(
        &mut self,
        field: Field<'_>,
        at: &Loc<'_>,
        offset: usize,
        type_name: &str,
        value: &str,
        error: String,
    ) -> bool {
        let element = String::from_utf8_lossy(field.key);
        let message = format!(
            "[element=\"{element}\"] Invalid attribute value \"{}\": {error}",
            sanitize_url_part(value)
        );
        let diagnostic = Diagnostic::new(ReportKind::InvalidValue, &at.render(), offset, message)
            .expecting(type_name);
        if is_blank(value.as_bytes()) {
            // Logged, not raised, by `LenientErrorHandler`; raised as `1821`
            // by strict mode like every other diagnostic.
            self.diagnostics.push(diagnostic);
            true
        } else {
            self.report(diagnostic);
            false
        }
    }

    /// The two sides of one element are index-aligned, so their array lengths
    /// must agree (`model.primext.006`).
    ///
    /// CONFLICT WITH THE REFERENCE, implemented as the scenario specifies it.
    /// HAPI reports nothing: `JsonParser.parseChildren` at `:1451-1458`
    /// iterates the **value** array and takes an alternate only
    /// `if (nextAlternateArray.size() >= (i + 1))`, so the entries of an
    /// over-long `_field` array are never visited and an imbalance in either
    /// direction is silent. This is therefore a Frevo-only report of the same
    /// kind as `model.json.016` and `model.json.018`.
    fn check_sibling_lengths(&mut self, prop: &Prop, at: &Loc<'_>, offset: usize) {
        let (Some(Shape::Array(value)), Some(Shape::Array(sibling))) = (&prop.value, &prop.sibling)
        else {
            return;
        };
        if value.len() == sibling.len() {
            return;
        }
        let (value_len, sibling_len) = (value.len(), sibling.len());
        let path = at.render();
        let name = prop.def.name;
        self.report(Diagnostic::new(
            ReportKind::PrimitiveExtensionLengthMismatch,
            &path,
            offset,
            format!(
                "{name} has {value_len} entries and _{name} has {sibling_len}; the two sides of \
                 an element are aligned by index, so the lengths must agree"
            ),
        ));
    }

    /// An `Extension` carrying both a `value[x]` and nested `extension`
    /// children (`model.json.022`).
    ///
    /// CONFLICT WITH THE REFERENCE as to **when**, implemented as the scenario
    /// specifies it. HAPI raises this only while **composing**:
    /// `JsonParser.writeUndeclaredExtension` at `:2008-2012` and
    /// `XmlParser.encodeCompositeElementToStreamWriter` at `:516-519` are its
    /// only two call sites in `hapi-fhir-base`, and `ParserState` has none. So
    /// HAPI parses such a body and throws when it serialises it again, while
    /// Frevo refuses it on the way in. The observable outcome for a request is
    /// the same `DataFormatException`; the stage differs.
    fn check_extension(&mut self, object: &Object, at: &Loc<'_>) {
        if object.type_name != "Extension" {
            return;
        }
        let has_value = object
            .props
            .iter()
            .filter(|prop| prop.def.name == "value")
            .any(|prop| Self::side_has_an_item(prop.value.as_ref()));
        let has_nested = object
            .props
            .iter()
            .filter(|prop| prop.def.name == "extension")
            .any(|prop| Self::side_has_an_item(prop.value.as_ref()));
        if !has_value || !has_nested {
            return;
        }
        let url = self
            .primitive_property(object, "url")
            .unwrap_or_else(|| "an extension with no url".to_string());
        let path = at.render();
        self.report(Diagnostic::new(
            ReportKind::ExtensionHasValueAndNestedExtensions,
            &path,
            object.start,
            format!("the extension {url} carries both a value and nested extensions"),
        ));
    }

    fn side_has_an_item(shape: Option<&Shape>) -> bool {
        shape.is_some_and(|shape| {
            (0..shape.len()).any(|index| shape.at(index).is_some_and(|value| !value.is_valueless()))
        })
    }

    /// Record a `Reference` for the weaving pass.
    ///
    /// Collected from the finished tree rather than during shaping, so that a
    /// `Reference` inside a property that a later duplicate overwrote is not
    /// woven: the tree holds the occurrence that won.
    fn record_reference(&mut self, object: &Object, path: &str) {
        if object.type_name != "Reference" {
            return;
        }
        let Some((text, offset)) = self.primitive_property_at(object, "reference") else {
            return;
        };
        self.local_references.push(LocalReference {
            path: path.to_string(),
            text,
            offset,
            resolved: false,
        });
    }

    /// Record one `contained` entry, in the list and — only if it has an `id` —
    /// in the id-to-resource map (`model.json.024`).
    ///
    /// Also collected from the finished tree, which is what makes a nested
    /// resource's own `contained` entries additive rather than a replacement
    /// for its parent's: `ParserState` keeps one map per resource state, and
    /// walking the tree in document order is how one flat list can carry both.
    fn record_contained(&mut self, object: &Object, path: &str) {
        let id = self
            .primitive_property(object, "id")
            .filter(|id| !is_blank(id.as_bytes()));
        match &id {
            // `IdDt`/`IIdElement` reduce a contained resource's id to its id
            // part before it is mapped (`ParserState.java:446-449`), so `#org1`
            // and `org1` map alike.
            Some(id) => self
                .referenceable_contained_ids
                .push(id.trim_start_matches('#').to_string()),
            None => self.reference_diagnostics.push(CodedDiagnostic::new(
                ReferenceIssue::ContainedResourceWithNoId,
                path,
                object.start,
                "Resource has contained child resource with no ID".to_string(),
            )),
        }
        self.contained_ids.push(id);
    }

    /// `ParserState.weaveContainedResources()` at `:1277-1293`. A reference is
    /// looked up only when it is non-blank and is `#` followed by at least one
    /// character, so a reference of exactly `#` -- a reference to the
    /// containing resource -- and a blank one are never looked up at all
    /// (`model.json.025`).
    fn weave(&mut self) {
        for reference in &mut self.local_references {
            let text = reference.text.as_str();
            if is_blank(text.as_bytes()) {
                continue;
            }
            let Some(id) = text.strip_prefix('#') else {
                continue;
            };
            if id.is_empty() {
                continue;
            }
            if self
                .referenceable_contained_ids
                .iter()
                .any(|contained| contained == id)
            {
                reference.resolved = true;
                continue;
            }
            self.reference_diagnostics.push(CodedDiagnostic::new(
                ReferenceIssue::UnknownReference,
                &reference.path,
                reference.offset,
                format!("Resource has invalid reference: {text}"),
            ));
        }
    }

    // -----------------------------------------------------------------------
    // The encode-side check (`model.canonical.007`)
    // -----------------------------------------------------------------------

    /// Check every reference under `resource`, a resource that is **not**
    /// itself contained, against that resource's own `contained` entries.
    ///
    /// `JsonParser.encodeResourceToJsonStreamWriter` calls
    /// `containResourcesInReferences` only for a resource that is not
    /// contained (`JsonParser.java:924-926`), so a reference inside a
    /// contained resource is checked against its container's list. A resource
    /// that is not contained but is not the root either -- a
    /// `Bundle.entry.resource`, a `Parameters.parameter.resource` -- starts a
    /// list of its own.
    fn check_resource(&self, resource: &Object, path: &str, out: &mut Vec<LocalReference>) {
        let mut contained = Vec::new();
        for prop in resource
            .props
            .iter()
            .filter(|prop| prop.def.name == "contained")
        {
            let Some(shape) = &prop.value else { continue };
            for index in 0..shape.len() {
                let Some(entry) = shape.at(index).and_then(Value::as_object) else {
                    continue;
                };
                // `FhirTerser.containResources` strips a leading `#` from a
                // contained resource's id before matching.
                if let Some(id) = self.primitive_property(entry, "id") {
                    contained.push(id.trim_start_matches('#').to_string());
                }
            }
        }
        self.check_props(resource, path, &contained, out);
    }

    fn check_props(
        &self,
        object: &Object,
        path: &str,
        contained: &[String],
        out: &mut Vec<LocalReference>,
    ) {
        for prop in &object.props {
            let element_path = child_path(path, prop.def.name);
            // A primitive's `_field` side carries extensions, and an extension
            // can carry a `valueReference`, so both sides are walked.
            for shape in [&prop.value, &prop.sibling].into_iter().flatten() {
                for index in 0..shape.len() {
                    let Some(child) = shape.at(index).and_then(Value::as_object) else {
                        continue;
                    };
                    let occurrence = if prop.def.repeating {
                        indexed(&element_path, index)
                    } else {
                        element_path.clone()
                    };
                    let is_contained = object.is_resource && prop.def.name == "contained";
                    if child.is_resource && !is_contained {
                        self.check_resource(child, &occurrence, out);
                        continue;
                    }
                    self.check_reference(child, &occurrence, contained, out);
                    self.check_props(child, &occurrence, contained, out);
                }
            }
        }
    }

    /// `BaseParser.determineReferenceText`: a reference with a blank id part
    /// takes the contained-resource-naming branch and is not checked; then
    /// `isValidInternalReference` accepts a non-local reference, a reference
    /// of exactly `#` -- the containing resource -- and one whose text after
    /// the `#` is the id of a contained resource.
    fn check_reference(
        &self,
        object: &Object,
        path: &str,
        contained: &[String],
        out: &mut Vec<LocalReference>,
    ) {
        if object.type_name != "Reference" {
            return;
        }
        let Some((text, offset)) = self.primitive_property_at(object, "reference") else {
            return;
        };
        if is_blank(text.as_bytes()) {
            return;
        }
        let Some(id) = text.strip_prefix('#') else {
            return;
        };
        if id.is_empty() || contained.iter().any(|candidate| candidate == id) {
            return;
        }
        out.push(LocalReference {
            path: path.to_string(),
            text,
            offset,
            resolved: false,
        });
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
                if let Some(shape) = prop
                    .value
                    .as_ref()
                    .filter(|shape| !Self::is_valueless(shape))
                {
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

    /// Whether a value side is a single primitive with no value, which
    /// writes nothing at all: a primitive that holds no value is not
    /// composed upstream, and its `_field` side, if any, still is.
    fn is_valueless(shape: &Shape) -> bool {
        matches!(shape, Shape::Single(Some(value)) if value.is_valueless())
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
            Some(Value::Primitive(primitive)) if primitive.valueless => {
                out.extend_from_slice(b"null");
            }
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
    ///
    /// This walk is also where the reference graph is collected, because it is
    /// the one pass that visits the finished tree in document order with a path
    /// for every element in hand — so the `contained` entries and the
    /// references cost no second traversal.
    fn flatten_object(
        &mut self,
        object: &Object,
        path: &str,
        is_resource: bool,
        sibling: Option<&Object>,
        out: &mut Vec<DecodedElement>,
    ) {
        self.record_reference(object, path);
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

    fn flatten_props(&mut self, object: &Object, path: &str, out: &mut Vec<DecodedElement>) {
        // `contained` is the only element whose entries enter the
        // id-to-resource map, and only on a resource: a
        // `Bundle.entry.resource` is a resource in its own right and is not
        // contained in anything.
        let is_container = object.is_resource;
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
                        if is_container && prop.def.name == "contained" {
                            self.record_contained(child, &occurrence_path);
                        }
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
        self.primitive_property_at(object, name)
            .map(|(lexical, _)| lexical)
    }

    /// [`Self::primitive_property`] with the byte offset of the value, for a
    /// report that has to point at it.
    fn primitive_property_at(&self, object: &Object, name: &str) -> Option<(String, usize)> {
        object
            .props
            .iter()
            .find(|prop| prop.def.name == name)
            .and_then(|prop| prop.value.as_ref())
            .and_then(|shape| shape.at(0))
            .and_then(Value::primitive)
            .map(|primitive| (self.lexical(primitive), primitive.raw.start))
    }

    // -----------------------------------------------------------------------
    // The canonical form
    // -----------------------------------------------------------------------

    /// Re-emit with every object's property names sorted lexicographically and
    /// no whitespace, which is what
    /// `org.hl7.fhir.r4.formats.JsonCreatorCanonical` produces: it collects
    /// each object's property names, sorts them, and refuses to pretty-print.
    fn emit_object_canonical(&self, object: &Object, out: &mut Vec<u8>) {
        // `None` is `resourceType`, whose value is the type name rather than a
        // shape. It takes its sorted position like any other property.
        let mut members: Vec<(String, Option<&Shape>)> = Vec::new();
        if object.is_resource {
            members.push(("resourceType".to_string(), None));
        }
        for prop in &object.props {
            let property = prop.property();
            if let Some(shape) = prop
                .value
                .as_ref()
                .filter(|shape| !Self::is_valueless(shape))
            {
                members.push((property.clone(), Some(shape)));
            }
            if let Some(shape) = &prop.sibling {
                members.push((format!("_{property}"), Some(shape)));
            }
        }
        members.sort_by(|left, right| left.0.cmp(&right.0));
        out.push(b'{');
        for (index, (property, shape)) in members.iter().enumerate() {
            if index > 0 {
                out.push(b',');
            }
            out.push(b'"');
            out.extend_from_slice(property.as_bytes());
            out.extend_from_slice(br#"":"#);
            match shape {
                None => {
                    out.push(b'"');
                    out.extend_from_slice(object.type_name.as_bytes());
                    out.push(b'"');
                }
                Some(shape) => self.emit_shape_canonical(shape, out),
            }
        }
        out.push(b'}');
    }

    fn emit_shape_canonical(&self, shape: &Shape, out: &mut Vec<u8>) {
        match shape {
            Shape::Single(value) => self.emit_value_canonical(value.as_ref(), out),
            Shape::Array(items) => {
                out.push(b'[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    self.emit_value_canonical(item.as_ref(), out);
                }
                out.push(b']');
            }
        }
    }

    fn emit_value_canonical(&self, value: Option<&Value>, out: &mut Vec<u8>) {
        match value {
            None => out.extend_from_slice(b"null"),
            Some(Value::Primitive(primitive)) if primitive.valueless => {
                out.extend_from_slice(b"null");
            }
            Some(Value::Primitive(primitive)) => {
                out.extend_from_slice(primitive.raw.of(self.bytes));
            }
            Some(Value::Object(object)) => self.emit_object_canonical(object, out),
        }
    }
}

/// The bytes a value rule is checked against: a quoted string's own text, or a
/// number's or boolean's token.
fn token_value(bytes: &[u8], span: Span) -> &[u8] {
    if bytes.get(span.start) == Some(&b'"') && span.end > span.start + 1 {
        &bytes[span.start + 1..span.end - 1]
    } else {
        span.of(bytes)
    }
}

/// `org.apache.commons.lang3.StringUtils.isBlank`, which is what
/// `LenientErrorHandler.invalidValue` tests: empty, or nothing but whitespace.
fn is_blank(value: &[u8]) -> bool {
    value.iter().all(|byte| byte.is_ascii_whitespace())
}

/// The JSON type name a primitive's declared form takes.
fn json_form_name(form: JsonForm) -> &'static str {
    match form {
        JsonForm::Quoted => "string",
        JsonForm::Boolean => "boolean",
        JsonForm::Number => "number",
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

// ---------------------------------------------------------------------------
// The primitive value rules
//
// `invalidValue` is raised when `IPrimitiveType.setValueAsString` refuses the
// text (`ParserState.java:1481-1514`), so the rule is the one the R4
// StructureDefinition states for the type. Only the types whose rule is a
// syntax are checked: a `string` or a `markdown` admits any non-blank text, and
// an `xhtml` and a `base64Binary` are never inspected at all, so none of them
// has a rule here. A type with no rule is accepted, which is the safe default:
// a decoder that invented a rule would reject a document HAPI accepts.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum ValueRule {
    /// `true|false`.
    Boolean,
    /// An integer literal. The declared *range* is [`IntegerBound`]'s, and a
    /// value inside the syntax but outside the range is `ValueOutOfRange`
    /// rather than `InvalidValue`, which is what `model.json.011` and
    /// `model.json.012` require -- so this rule is deliberately looser than
    /// R4's `integer` regex, which forbids a leading zero and a `+`.
    IntegerLiteral,
    /// A decimal literal. HAPI strips a leading `+` and leading zeros before
    /// parsing (`ParserState.java:1459-1478`), so both are accepted.
    Decimal,
    /// `yyyy`, `yyyy-mm` or `yyyy-mm-dd`.
    Date,
    /// A [`ValueRule::Date`], or a full date and time, which then needs a
    /// timezone.
    DateTime,
    /// A full date and time with a timezone, always.
    Instant,
    /// `hh:mm:ss` with optional fractional seconds.
    Time,
    /// `[^\s]+( [^\s]+)*`: no leading, trailing or repeated whitespace.
    Code,
    /// `[A-Za-z0-9\-\.]{1,64}`.
    Id,
    /// `urn:oid:` and then a dotted decimal.
    Oid,
    /// `urn:uuid:` and then a hex-and-dash string.
    Uuid,
    /// `\S*`: a URI may not contain whitespace.
    Uri,
}

impl ValueRule {
    fn of(type_name: &str) -> Option<Self> {
        Some(match type_name {
            "boolean" => ValueRule::Boolean,
            "integer" | "positiveInt" | "unsignedInt" => ValueRule::IntegerLiteral,
            "decimal" => ValueRule::Decimal,
            "date" => ValueRule::Date,
            "dateTime" => ValueRule::DateTime,
            "instant" => ValueRule::Instant,
            "time" => ValueRule::Time,
            "code" => ValueRule::Code,
            "id" => ValueRule::Id,
            "oid" => ValueRule::Oid,
            "uuid" => ValueRule::Uuid,
            "uri" | "url" | "canonical" => ValueRule::Uri,
            _ => return None,
        })
    }

    fn holds(self, value: &[u8]) -> bool {
        match self {
            ValueRule::Boolean => value == b"true" || value == b"false",
            ValueRule::IntegerLiteral => is_integer_literal_bytes(value),
            ValueRule::Decimal => decimal_holds(value),
            ValueRule::Date => date_holds(value),
            ValueRule::DateTime => date_holds(value) || date_time_holds(value),
            ValueRule::Instant => date_time_holds(value),
            ValueRule::Time => time_holds(value),
            ValueRule::Code => code_holds(value),
            ValueRule::Id => {
                (1..=64).contains(&value.len())
                    && value
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'.')
            }
            ValueRule::Oid => {
                value.starts_with(b"urn:oid:")
                    && value[8..].split(|byte| *byte == b'.').all(is_digits)
            }
            ValueRule::Uuid => {
                value.starts_with(b"urn:uuid:")
                    && value[9..]
                        .iter()
                        .all(|byte| byte.is_ascii_hexdigit() || *byte == b'-')
            }
            ValueRule::Uri => !value.iter().any(|byte| byte.is_ascii_whitespace()),
        }
    }
}

fn is_digits(value: &[u8]) -> bool {
    !value.is_empty() && value.iter().all(|byte| byte.is_ascii_digit())
}

fn is_integer_literal_bytes(value: &[u8]) -> bool {
    let digits = match value.first() {
        Some(b'-' | b'+') => &value[1..],
        _ => value,
    };
    is_digits(digits)
}

fn decimal_holds(value: &[u8]) -> bool {
    let mut at = usize::from(matches!(value.first(), Some(b'-' | b'+')));
    let digits = |value: &[u8], at: &mut usize| {
        let start = *at;
        while matches!(value.get(*at), Some(byte) if byte.is_ascii_digit()) {
            *at += 1;
        }
        *at > start
    };
    if !digits(value, &mut at) {
        return false;
    }
    if value.get(at) == Some(&b'.') {
        at += 1;
        if !digits(value, &mut at) {
            return false;
        }
    }
    if matches!(value.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(value.get(at), Some(b'-' | b'+')) {
            at += 1;
        }
        if !digits(value, &mut at) {
            return false;
        }
    }
    at == value.len()
}

fn date_holds(value: &[u8]) -> bool {
    match value.len() {
        4 => is_digits(value),
        7 => is_digits(&value[..4]) && value[4] == b'-' && is_digits(&value[5..]),
        10 => {
            is_digits(&value[..4])
                && value[4] == b'-'
                && is_digits(&value[5..7])
                && value[7] == b'-'
                && is_digits(&value[8..])
        }
        _ => false,
    }
}

fn time_holds(value: &[u8]) -> bool {
    if value.len() < 8
        || !is_digits(&value[..2])
        || value[2] != b':'
        || !is_digits(&value[3..5])
        || value[5] != b':'
        || !is_digits(&value[6..8])
    {
        return false;
    }
    value.len() == 8 || (value[8] == b'.' && is_digits(&value[9..]))
}

/// A full date and time with a timezone, which is `instant` exactly and
/// `dateTime`'s longest form.
fn date_time_holds(value: &[u8]) -> bool {
    if value.len() < 11 || value[10] != b'T' || !date_holds(&value[..10]) {
        return false;
    }
    let rest = &value[11..];
    let (time, zone): (&[u8], &[u8]) = if rest.last() == Some(&b'Z') {
        (&rest[..rest.len() - 1], b"Z")
    } else if rest.len() >= 6 && matches!(rest[rest.len() - 6], b'+' | b'-') {
        rest.split_at(rest.len() - 6)
    } else {
        return false;
    };
    time_holds(time) && zone_holds(zone)
}

fn zone_holds(zone: &[u8]) -> bool {
    zone == b"Z"
        || (zone.len() == 6
            && matches!(zone[0], b'+' | b'-')
            && is_digits(&zone[1..3])
            && zone[3] == b':'
            && is_digits(&zone[4..]))
}

fn code_holds(value: &[u8]) -> bool {
    if value.is_empty() || value[0] == b' ' || value[value.len() - 1] == b' ' {
        return false;
    }
    let mut previous_was_space = false;
    for byte in value {
        if *byte == b' ' {
            if previous_was_space {
                return false;
            }
            previous_was_space = true;
        } else if byte.is_ascii_whitespace() || *byte < 0x20 {
            return false;
        } else {
            previous_was_space = false;
        }
    }
    true
}

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
