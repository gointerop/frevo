//! The `model-decode` in-process oracle: FHIR JSON decoding, the `_field`
//! primitive-extension form, and decode error reporting.
//!
//! Task 5 of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo test -p frevo-model --test model_decode
//! ```
//!
//! Scenario coverage, 37 of the 80 uncovered `model.*` scenarios:
//!
//! - `model.json.001` .. `model.json.023` (23 -- `.024` and `.025` are graded
//!   by the committed `model_parser_errors.rs` oracle and are not repeated);
//! - `model.primext.001` .. `model.primext.009` (9);
//! - `model.errors.001` .. `model.errors.005` (5).
//!
//! ## Why the fixtures are hand-built and compact
//!
//! Every fixture below is written in the **generated element order** of its
//! resource, compact and with no insignificant whitespace, so that a full
//! materialise-and-re-emit is expected to reproduce the input byte for byte.
//! That turns the round-trip scenarios (`model.json.008`, `.017`, `.018`,
//! `.020`, `model.primext.007`) into assertions a byte-splicing pass-through
//! cannot satisfy by accident: [`Decoded::reencoded_in_element_order`] is
//! defined as the output of a *full* materialisation, so returning the input
//! buffer is not an answer to it. `model.bytes.001` is where pass-through byte
//! identity is graded, in `model_bytes_lazy_perf.rs`.
//!
//! The R4 example package is git-ignored (`oracles/**/fetched/`), so no test
//! here depends on it. Where a scenario names an example file, that file's
//! pinned content is embedded and its provenance recorded on the constant.
//!
//! ## Choice elements are addressed by their base name
//!
//! Paths in this oracle use the FHIR element name with no `[x]` suffix and no
//! typed spelling: `Observation.value`, never `Observation.valueQuantity`.
//! That is `model.nav.003`/`model.nav.004` and `design.md` section 4.2, and it
//! is why the fixtures that need a *path* assertion prefer a non-choice element
//! of the same type where one exists.
//!
//! ## The seam
//!
//! The oracle runs against a trait object, so it compiles and runs before the
//! codec of `design.md` section 3 exists. The unbound binding returns
//! [`DecodeError::NotImplemented`] from every surface, which is an error value
//! and not a panicking placeholder; every helper in this file turns it into an
//! explicit "nothing is bound yet" failure. That matters most for the scenarios
//! that expect a *rejection*: without it, a codec that refused to parse
//! anything would score a point on every one of them.
//!
//! Provenance for the behaviour asserted here, all at HAPI pin
//! `7ab4c7c7c49281496faa1b3c2498a652457669ae`:
//!
//! - `ca.uhn.fhir.parser.LenientErrorHandler` class documentation: "logs issues
//!   but does not abort parsing, with only two exceptions", those being
//!   `invalidValue` and `extensionContainsValueAndNestedExtensions`;
//! - `LenientErrorHandler.invalidValue` lines 100-107: a **blank** value is
//!   logged rather than raised, which is `model.json.023`'s second clause;
//! - `ca.uhn.fhir.parser.JsonParser.parseChildren` lines 1482-1486: the
//!   explicit `theJsonVal.isNull()` branch that enters the element, parses the
//!   `_name` sibling and ends it -- `model.json.018`;
//! - `JsonParser.parseChildren` lines 1451-1458: the repeating branch iterates
//!   the **value** array and takes the `_name` entry only when the alternate
//!   array reaches that index -- `model.primext.002`, `model.primext.003`;
//! - `JsonParser.parseChildren` lines 1378-1408: a `_name` with no `name`
//!   sibling still enters the element -- `model.primext.005`;
//! - `JsonParser.parseChildren` lines 1485-1492: a scalar reaches
//!   `attributeValue("value", asString)` as its **string** form whatever the
//!   JSON type was -- `model.json.013`.

use std::fmt;
use std::ops::Range;

use frevo_model::codec::decode as codec;

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

/// The two parse modes, as `FhirContext.setParserErrorHandler` selects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Lenient,
    Strict,
}

/// What a decode report is about. One variant per condition the spec delta
/// names, so a test can assert the condition rather than matching message
/// prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// `resourceType` absent -- `model.json.002`.
    MissingResourceType,
    /// `resourceType` names no known type -- `model.json.003`.
    UnknownResourceType,
    /// `IParserErrorHandler.unknownElement` / `unknownAttribute` --
    /// `model.json.004`.
    UnknownProperty,
    /// `IParserErrorHandler.incorrectJsonType` -- `model.json.013`,
    /// `model.json.014`.
    IncorrectJsonType,
    /// `IParserErrorHandler.unexpectedRepeatingElement` -- `model.json.015`.
    UnexpectedRepeatingElement,
    /// No HAPI callback exists; Frevo's strict mode is a superset here, which
    /// `docs/deviations.md` records -- `model.json.016`.
    DuplicateProperty,
    /// No HAPI callback exists; same note -- `model.json.018`.
    NullInNonPrimitiveArray,
    /// A numeric value outside its declared type's range -- `model.json.011`,
    /// `model.json.012`.
    ValueOutOfRange,
    /// `IParserErrorHandler.invalidValue` -- `model.json.023`.
    InvalidValue,
    /// `IParserErrorHandler.extensionContainsValueAndNestedExtensions` --
    /// `model.json.022`.
    ExtensionHasValueAndNestedExtensions,
    /// Input bytes are not valid UTF-8 -- `model.json.019`.
    NotUtf8,
    /// `_field` and its value array disagree in length -- `model.primext.006`.
    PrimitiveExtensionLengthMismatch,
    /// A `_field` property on a non-primitive element -- `model.primext.009`.
    UnexpectedUnderscoreProperty,
    /// Nesting beyond the codec's bound -- `model.errors.005`.
    DepthLimit,
}

/// One report. In lenient mode it is a diagnostic returned beside the resource;
/// in strict mode the same thing is raised as an error instead
/// (`model.json.021`). The `path` and `offset` fields are `model.errors.001`
/// and `model.errors.002`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
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

/// A decimal read back from the model: its lexical form as it appeared, and the
/// arbitrary-precision value rendered at full precision. The two are kept apart
/// so `model.json.010` can assert that the value did not pass through a binary
/// float -- an `f64` cannot hold 40 significant digits, so a `full_precision`
/// that still carries them proves the conversion never happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal {
    pub lexical: String,
    pub full_precision: String,
    pub significant_digits: u32,
}

/// One element of a decoded resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// FHIR element path with `[i]` for each repetition, choice elements under
    /// their base name.
    pub path: String,
    /// FHIR type name: `Patient`, `HumanName`, `string`, `Quantity`.
    pub type_name: String,
    /// The primitive value's lexical form, or `None` when the element has no
    /// value -- which is exactly what `hasValue()` reports
    /// (`model.primext.004`).
    pub value: Option<String>,
    /// The element's own `id`: from the `_field` sibling for a primitive, from
    /// the object's `id` property otherwise (`model.json.020`).
    pub element_id: Option<String>,
    /// The `url` of each extension on this element, in document order.
    pub extension_urls: Vec<String>,
    /// Set when the element's declared type is `decimal`.
    pub decimal: Option<Decimal>,
    /// Whether this element is a resource (`model.json.006`, `.007`).
    pub is_resource: bool,
}

/// The result of a successful decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The type taken from `resourceType` (`model.json.001`).
    pub resource_type: String,
    /// Every diagnostic, in document order (`model.errors.003`).
    pub diagnostics: Vec<Report>,
    /// Every element of the resource, in document order.
    pub elements: Vec<Element>,
    /// The resource **fully materialised** and re-emitted compact in the
    /// generated element order. Defined as a full re-encode precisely so that
    /// it cannot be answered by handing back the input bytes.
    pub reencoded_in_element_order: Vec<u8>,
}

impl Decoded {
    pub fn kinds(&self) -> Vec<ReportKind> {
        self.diagnostics.iter().map(|d| d.kind).collect()
    }

    pub fn at(&self, path: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.path == path)
    }

    pub fn value(&self, path: &str) -> Option<&str> {
        self.at(path).and_then(|e| e.value.as_deref())
    }

    /// Every element whose path is strictly below `prefix`.
    pub fn descendants_of(&self, prefix: &str) -> Vec<&Element> {
        let dotted = format!("{prefix}.");
        let indexed = format!("{prefix}[");
        self.elements
            .iter()
            .filter(|e| e.path.starts_with(&dotted) || e.path.starts_with(&indexed))
            .collect()
    }

    /// The repetitions of a repeating element, in index order.
    pub fn repetitions_of(&self, name: &str) -> Vec<&Element> {
        let mut out = Vec::new();
        for i in 0usize.. {
            match self.at(&format!("{name}[{i}]")) {
                Some(e) => out.push(e),
                None => break,
            }
        }
        out
    }

    pub fn paths(&self) -> Vec<&str> {
        self.elements.iter().map(|e| e.path.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The decode failed. In strict mode this is the report lenient mode would
    /// have returned as a diagnostic, raised instead of recorded.
    Fatal(Report),
    /// No codec is bound to this oracle yet.
    NotImplemented,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Fatal(r) => write!(
                f,
                "{:?}: {} at {} (byte {})",
                r.kind, r.message, r.path, r.offset
            ),
            DecodeError::NotImplemented => f.write_str(NOT_BOUND),
        }
    }
}

const NOT_BOUND: &str = "no codec is bound to the model-decode oracle yet \
     (crates/frevo-model/tests/model_decode.rs::codec_under_test)";

pub trait DecodeOracle {
    /// Decode in an explicitly chosen mode.
    fn decode(&self, bytes: &[u8], mode: ParseMode) -> Result<Decoded, DecodeError>;

    /// Decode with no mode stated by the caller. `model.json.021` requires this
    /// to behave exactly as [`ParseMode::Lenient`], because that is the handler
    /// `FhirContext` installs.
    fn decode_with_default_mode(&self, bytes: &[u8]) -> Result<Decoded, DecodeError>;

    /// The mode a decoder created with no explicit mode is in.
    fn default_mode(&self) -> Result<ParseMode, DecodeError>;
}

/// The codec this oracle grades. Phase C replaces this body with the real one.
fn codec_under_test() -> Box<dyn DecodeOracle> {
    Box::new(FrevoCodec)
}

struct FrevoCodec;

impl FrevoCodec {
    fn mode(mode: ParseMode) -> codec::ParseMode {
        match mode {
            ParseMode::Lenient => codec::ParseMode::Lenient,
            ParseMode::Strict => codec::ParseMode::Strict,
        }
    }

    fn kind(kind: codec::ReportKind) -> ReportKind {
        match kind {
            codec::ReportKind::MissingResourceType => ReportKind::MissingResourceType,
            codec::ReportKind::UnknownResourceType => ReportKind::UnknownResourceType,
            codec::ReportKind::UnknownProperty => ReportKind::UnknownProperty,
            codec::ReportKind::IncorrectJsonType => ReportKind::IncorrectJsonType,
            codec::ReportKind::UnexpectedRepeatingElement => ReportKind::UnexpectedRepeatingElement,
            codec::ReportKind::DuplicateProperty => ReportKind::DuplicateProperty,
            codec::ReportKind::NullInNonPrimitiveArray => ReportKind::NullInNonPrimitiveArray,
            codec::ReportKind::ValueOutOfRange => ReportKind::ValueOutOfRange,
            codec::ReportKind::InvalidValue => ReportKind::InvalidValue,
            codec::ReportKind::ExtensionHasValueAndNestedExtensions => {
                ReportKind::ExtensionHasValueAndNestedExtensions
            }
            codec::ReportKind::NotUtf8 => ReportKind::NotUtf8,
            codec::ReportKind::PrimitiveExtensionLengthMismatch => {
                ReportKind::PrimitiveExtensionLengthMismatch
            }
            codec::ReportKind::UnexpectedUnderscoreProperty => {
                ReportKind::UnexpectedUnderscoreProperty
            }
            codec::ReportKind::DepthLimit => ReportKind::DepthLimit,
        }
    }

    fn report(diagnostic: &codec::Diagnostic) -> Report {
        Report {
            kind: Self::kind(diagnostic.kind),
            path: diagnostic.path.clone(),
            offset: diagnostic.offset,
            message: diagnostic.message.clone(),
            expected_type: diagnostic.expected_type.clone(),
            found_type: diagnostic.found_type.clone(),
        }
    }

    fn decimal(decimal: &codec::Decimal) -> Decimal {
        Decimal {
            lexical: decimal.lexical.clone(),
            full_precision: decimal.full_precision.clone(),
            significant_digits: decimal.significant_digits,
        }
    }

    fn decoded(decoded: codec::Decoded) -> Decoded {
        Decoded {
            resource_type: decoded.resource_type,
            diagnostics: decoded.diagnostics.iter().map(Self::report).collect(),
            elements: decoded
                .elements
                .iter()
                .map(|element| Element {
                    path: element.path.clone(),
                    type_name: element.type_name.clone(),
                    value: element.value.clone(),
                    element_id: element.element_id.clone(),
                    extension_urls: element.extension_urls.clone(),
                    decimal: element.decimal.as_ref().map(Self::decimal),
                    is_resource: element.is_resource,
                })
                .collect(),
            reencoded_in_element_order: decoded.reencoded,
        }
    }

    fn run(result: Result<codec::Decoded, codec::Diagnostic>) -> Result<Decoded, DecodeError> {
        match result {
            Ok(decoded) => Ok(Self::decoded(decoded)),
            Err(diagnostic) => Err(DecodeError::Fatal(Self::report(&diagnostic))),
        }
    }
}

impl DecodeOracle for FrevoCodec {
    fn decode(&self, bytes: &[u8], mode: ParseMode) -> Result<Decoded, DecodeError> {
        Self::run(codec::decode(bytes, Self::mode(mode)))
    }

    fn decode_with_default_mode(&self, bytes: &[u8]) -> Result<Decoded, DecodeError> {
        Self::run(codec::decode_default(bytes))
    }

    fn default_mode(&self) -> Result<ParseMode, DecodeError> {
        Ok(match codec::DEFAULT_PARSE_MODE {
            codec::ParseMode::Lenient => ParseMode::Lenient,
            codec::ParseMode::Strict => ParseMode::Strict,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers. Every one turns NotImplemented into an explicit failure, so
// "nothing is bound yet" can never read as a pass.
// ---------------------------------------------------------------------------

/// Require a successful decode.
fn must_decode(bytes: &[u8], mode: ParseMode, what: &str) -> Decoded {
    match codec_under_test().decode(bytes, mode) {
        Ok(d) => d,
        Err(e) => panic!("{what}: decoding in {mode:?} must succeed, but: {e}"),
    }
}

/// Require a failed decode and return the report it failed with. A
/// `NotImplemented` is not a rejection, and is refused here explicitly.
fn must_reject(bytes: &[u8], mode: ParseMode, what: &str) -> Report {
    match codec_under_test().decode(bytes, mode) {
        Err(DecodeError::Fatal(r)) => r,
        Err(DecodeError::NotImplemented) => {
            panic!("{what}: {NOT_BOUND} -- an unbound codec is not a rejection")
        }
        Ok(d) => panic!(
            "{what}: decoding in {mode:?} must fail, but it succeeded with diagnostics {:?}",
            d.kinds()
        ),
    }
}

/// A condition the spec states as "decoding reports ..." without saying which
/// side of the fatal line it falls on. Returns the report however it is
/// delivered -- as a lenient diagnostic or as a fatal error -- and fails on an
/// unbound codec, on a silent success, and on more than one report.
fn report_for(bytes: &[u8], what: &str) -> Report {
    match codec_under_test().decode(bytes, ParseMode::Lenient) {
        Ok(d) => {
            assert_eq!(
                d.diagnostics.len(),
                1,
                "{what}: decoding must report the problem exactly once, got {:?}",
                d.kinds()
            );
            d.diagnostics[0].clone()
        }
        Err(DecodeError::Fatal(r)) => r,
        Err(DecodeError::NotImplemented) => panic!("{what}: {NOT_BOUND}"),
    }
}

/// Require the given report to be about `path`, carry a byte offset inside the
/// input, and mention `needle`.
fn assert_located(r: &Report, input_len: usize, path: &str, needle: &str, what: &str) {
    assert_eq!(
        r.path, path,
        "{what}: the report must name the element path"
    );
    assert!(
        r.offset < input_len,
        "{what}: the byte offset {} must point inside the {input_len}-byte input",
        r.offset
    );
    assert!(
        r.message.contains(needle),
        "{what}: the message must mention {needle:?}, but it reads {:?}",
        r.message
    );
}

/// The byte range of `needle` in `haystack`, which must occur exactly once.
fn sole_range(haystack: &[u8], needle: &str) -> Range<usize> {
    let hay = std::str::from_utf8(haystack).expect("the fixture is UTF-8");
    let first = hay
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} does not occur in {hay}"));
    assert!(
        !hay[first + needle.len()..].contains(needle),
        "{needle:?} occurs more than once; the assertion would be ambiguous"
    );
    first..first + needle.len()
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The first 40 characters at `offset`, for a failure message.
fn peek(bytes: &[u8], offset: usize) -> String {
    let tail = bytes.get(offset..).unwrap_or_default();
    String::from_utf8_lossy(tail).chars().take(40).collect()
}

// ---------------------------------------------------------------------------
// Fixtures
//
// `Patient` property order from the R4 snapshot: `resourceType`, then
// `Resource` (`id`, `meta`, `implicitRules`, `language`), then `DomainResource`
// (`text`, `contained`, `extension`, `modifierExtension`), then `identifier`,
// `active`, `name`, `telecom`, `gender`, `birthDate`, `deceased[x]`,
// `address`, `maritalStatus`, `multipleBirth[x]`, `photo`, ... -- and a
// primitive's `_field` sibling directly after its value, which is the order
// `org.hl7.fhir.r4.formats.JsonParser` composes (`composeStringCore` then
// `composeStringExtras`).
// ---------------------------------------------------------------------------

const OBSERVATION_WITH_VALUE_QUANTITY: &str = concat!(
    r##"{"resourceType":"Observation","id":"o","status":"final","##,
    r##""code":{"text":"Glucose"},"##,
    r##""valueQuantity":{"value":6.3,"unit":"mmol/L"}}"##
);

const PATIENT_WITHOUT_RESOURCE_TYPE: &str = r##"{"id":"p","gender":"male"}"##;

const MISSPELLED_RESOURCE_TYPE: &str = r##"{"resourceType":"Patinet","id":"p"}"##;

/// `nmae` for `name`: the property is dropped and a diagnostic names
/// `Patient.nmae`.
const UNKNOWN_PROPERTY: &str = r##"{"resourceType":"Patient","id":"p","nmae":[],"gender":"male"}"##;

/// Pinned content of `patient-container-example.json` from
/// `oracles/fhir-test-cases/fetched/r4/`, re-emitted compact in element order.
const CONTAINER_EXAMPLE: &str = concat!(
    r##"{"resourceType":"Patient","id":"example-container","##,
    r##""contained":[{"resourceType":"Organization","id":"1"}],"##,
    r##""name":[{"text":"some-name"}],"##,
    r##""managingOrganization":{"reference":"1","display":"Gastroenterology"}}"##
);

/// A `Bundle` whose entries are of three different types. `Bundle` order:
/// `resourceType`, `id`, ..., `identifier`, `type`, `timestamp`, `total`,
/// `link`, `entry`, `signature`; `Bundle.entry` order: `id`, `extension`,
/// `modifierExtension`, `link`, `fullUrl`, `resource`, `search`, `request`,
/// `response`.
const BUNDLE_OF_MIXED_TYPES: &str = concat!(
    r##"{"resourceType":"Bundle","id":"b","type":"collection","entry":["##,
    r##"{"resource":{"resourceType":"Patient","id":"p1"}},"##,
    r##"{"resource":{"resourceType":"Observation","id":"o1","status":"final","##,
    r##""code":{"text":"Glucose"}}},"##,
    r##"{"resource":{"resourceType":"Organization","id":"org1"}}]}"##
);

/// Nested markup and entity references inside `text.div`. `Narrative` order is
/// `id`, `extension`, `status`, `div`.
const PATIENT_WITH_NARRATIVE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","text":{"status":"generated","##,
    r##""div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">"##,
    r##"<p><b>Chalmers</b> &amp; Sons &lt;est. 1896&gt;</p>"##,
    r##"<table><tr><td>a&#160;b</td></tr></table></div>"},"##,
    r##""gender":"male"}"##
);

/// The JSON equivalent of `parameters-example-decimals.xml`, carrying the three
/// forms `model.json.009` names. `Parameters` order: `resourceType`, `id`,
/// `meta`, `implicitRules`, `language`, `parameter`; `Parameters.parameter`
/// order: `id`, `extension`, `modifierExtension`, `name`, `value[x]`,
/// `resource`, `part`.
const PARAMETERS_WITH_DECIMALS: &str = concat!(
    r##"{"resourceType":"Parameters","id":"example-decimals","parameter":["##,
    r##"{"name":"trailing-zeros","valueDecimal":1.58700},"##,
    r##"{"name":"exponent","valueDecimal":1.00e3},"##,
    r##"{"name":"small","valueDecimal":0.0014}]}"##
);

/// `Media.duration` is a non-choice `decimal`, so `model.json.010` can assert a
/// path without raising the choice-naming question. `Media` order:
/// `resourceType`, `id`, ..., `identifier`, `basedOn`, `partOf`, `status`,
/// `type`, `modality`, `view`, `subject`, `encounter`, `created[x]`, `issued`,
/// `operator`, `reasonCode`, `bodySite`, `deviceName`, `device`, `height`,
/// `width`, `frames`, `duration`, `content`.
fn media_with_duration(literal: &str) -> String {
    format!(r##"{{"resourceType":"Media","id":"m","status":"completed","duration":{literal}}}"##)
}

/// 40 significant digits -- more than twice what an `f64` can round-trip, so a
/// value that survives it did not pass through a binary float.
const FORTY_DIGIT_DECIMAL: &str = "1.234567890123456789012345678901234567890";

/// `MolecularSequence.coordinateSystem` is a non-choice `integer`.
const INTEGER_OUT_OF_RANGE: &str =
    r##"{"resourceType":"MolecularSequence","id":"s","coordinateSystem":2147483648}"##;

/// `Media.height` is `positiveInt`, so `0` is out of range.
const POSITIVE_INT_ZERO: &str =
    r##"{"resourceType":"Media","id":"m","status":"completed","height":0}"##;

/// `Bundle.total` is `unsignedInt`, so `-1` is out of range.
const UNSIGNED_INT_NEGATIVE: &str =
    r##"{"resourceType":"Bundle","id":"b","type":"searchset","total":-1}"##;

/// `Patient.active` is `boolean`; the JSON gives a string.
const BOOLEAN_AS_STRING: &str = r##"{"resourceType":"Patient","id":"p","active":"true"}"##;

/// `Patient.name` is `0..*`; the JSON gives a bare object.
const REPEATING_GIVEN_ONE_OBJECT: &str =
    r##"{"resourceType":"Patient","id":"p","name":{"family":"Chalmers"}}"##;

/// `Patient.birthDate` is `0..1`; the JSON gives an array.
const SINGLETON_GIVEN_AN_ARRAY: &str =
    r##"{"resourceType":"Patient","id":"p","birthDate":["1974-12-25","1980-01-01"]}"##;

/// The same property name twice.
const DUPLICATE_PROPERTY: &str =
    r##"{"resourceType":"Patient","id":"p","gender":"male","gender":"female"}"##;

/// `Attachment` order: `id`, `extension`, `contentType`, `language`, `data`,
/// `url`, `size`, `hash`, `title`, `creation`. The base64 text carries its own
/// padding, which must survive untouched.
const PATIENT_WITH_ATTACHMENT: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""photo":[{"contentType":"text/plain","data":"VGhpcyBpcyBzb21lIGJhc2U2NCB0ZXh0Lg=="}]}"##
);

/// The base64 text of [`PATIENT_WITH_ATTACHMENT`], with its padding.
const ATTACHMENT_BASE64: &str = "VGhpcyBpcyBzb21lIGJhc2U2NCB0ZXh0Lg==";

/// `null` inside a non-primitive array.
const NULL_IN_COMPLEX_ARRAY: &str = r##"{"resourceType":"Patient","id":"p","name":[null]}"##;

/// A `HumanName` carrying an element `id` and an `extension` array.
const NAME_WITH_ID_AND_EXTENSION: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","name":[{"id":"n1","##,
    r##""extension":[{"url":"https://example.org/name-qualifier","valueString":"preferred"}],"##,
    r##""family":"Chalmers"}]}"##
);

/// An `Extension` with both a `value[x]` and nested `extension` children --
/// one of the only two conditions `LenientErrorHandler` treats as fatal.
const EXTENSION_WITH_VALUE_AND_NESTED: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","extension":[{"##,
    r##""extension":[{"url":"https://example.org/inner","valueString":"inner"}],"##,
    r##""url":"https://example.org/outer","valueString":"outer"}]}"##
);

/// A primitive whose value does not match its type's value rule -- the other
/// fatal condition.
const INVALID_PRIMITIVE_VALUE: &str =
    r##"{"resourceType":"Patient","id":"p","birthDate":"not-a-date"}"##;

/// A **blank** primitive value, which `LenientErrorHandler.invalidValue` logs
/// rather than raises.
const BLANK_PRIMITIVE_VALUE: &str = r##"{"resourceType":"Patient","id":"p","birthDate":""}"##;

/// Three independent problems, in document order: an unknown property, a JSON
/// type mismatch, and an array where a `0..1` element belongs.
const THREE_INDEPENDENT_PROBLEMS: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","nmae":[],"active":"true","##,
    r##""birthDate":["1974-12-25","1980-01-01"]}"##
);

/// A singleton primitive with a `_field` sibling.
const BIRTHDATE_WITH_SIBLING: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","birthDate":"1974-12-25","##,
    r##""_birthDate":{"extension":[{"url":"https://example.org/birth-time","##,
    r##""valueString":"08:00"}]}}"##
);

/// Pinned content of `patient-name-extensions.json` from
/// `oracles/fhir-test-cases/fetched/r4/`, re-emitted compact in element order.
/// `HumanName` order: `id`, `extension`, `use`, `text`, `family`, `given`,
/// `prefix`, `suffix`, `period`.
///
/// Note that `given` has two entries and `_given` one: HAPI's repeating branch
/// iterates the value array and takes an alternate only when the alternate
/// array reaches that index, so index 0 pairs and index 1 does not. This is the
/// document behind oracle case `miscEngineTests/testPrimitiveExtensions`.
const NAME_EXTENSIONS_EXAMPLE: &str = concat!(
    r##"{"resourceType":"Patient","id":"example","active":true,"name":[{"##,
    r##""use":"maiden","family":"Windsor","given":[null,"James"],"##,
    r##""_given":[{"extension":[{"url":"https://example.org/syllable-count","##,
    r##""valueString":"five"}]}],"##,
    r##""period":{"end":"2002"}}]}"##
);

/// `model.primext.003` exactly as the scenario states it: a hole in the value
/// array at index 0 and a hole in the `_field` array at index 1.
const GIVEN_WITH_HOLES: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","name":[{"##,
    r##""given":[null,"Peter"],"##,
    r##""_given":[{"extension":[{"url":"https://example.org/syllable-count","##,
    r##""valueString":"two"}]},null]}]}"##
);

/// A `_field` sibling with no value sibling at all.
const SIBLING_WITHOUT_VALUE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""_birthDate":{"extension":[{"url":"https://example.org/birth-time","##,
    r##""valueString":"08:00"}]}}"##
);

/// `given` has three entries, `_given` two.
const UNEQUAL_SIBLING_ARRAYS: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","name":[{"##,
    r##""given":["Peter","James","Duncan"],"##,
    r##""_given":[{"extension":[{"url":"https://example.org/n1","valueString":"1"}]},"##,
    r##"{"extension":[{"url":"https://example.org/n2","valueString":"2"}]}]}]}"##
);

/// A `_field` property on `Patient.name`, which is a complex type.
const UNDERSCORE_ON_COMPLEX_ELEMENT: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","name":[{"family":"Chalmers"}],"##,
    r##""_name":{"extension":[{"url":"https://example.org/x","valueString":"y"}]}}"##
);

/// A `_field` sibling whose extension subtree is large, so that
/// `model.primext.008` has something worth not materialising.
fn birthdate_with_large_sibling() -> String {
    let mut s = String::from(r##"{"resourceType":"Patient","id":"p","birthDate":"1974-12-25","##);
    s.push_str(r##""_birthDate":{"extension":["##);
    for i in 0..200 {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r##"{{"url":"https://example.org/e{i}","valueString":"value-{i}"}}"##
        ));
    }
    s.push_str("]}}");
    s
}

/// A valid document with one byte that is not valid UTF-8, at a known offset.
fn not_utf8() -> (Vec<u8>, usize) {
    let head = r##"{"resourceType":"Patient","id":"p","gender":""##;
    let tail = r##"ale"}"##;
    let mut bytes = head.as_bytes().to_vec();
    let offset = bytes.len();
    bytes.push(0xFF);
    bytes.extend_from_slice(tail.as_bytes());
    (bytes, offset)
}

/// Arrays nested `depth` deep under `Patient.name`. Pure array nesting, so the
/// only thing a depth bound can be reacting to is the depth.
fn deeply_nested(depth: usize) -> Vec<u8> {
    let mut s = String::from(r##"{"resourceType":"Patient","id":"p","name":"##);
    s.push_str(&"[".repeat(depth));
    s.push_str(&"]".repeat(depth));
    s.push('}');
    s.into_bytes()
}

/// A fixture that must produce exactly one report, with the condition, the
/// element path, a fragment of the message, and the tokens the reported byte
/// offset may legitimately point at. This is the table `model.errors.001`,
/// `model.errors.002` and `model.json.021` are graded over, so that adding a
/// condition cannot quietly skip them.
struct Fixture {
    what: &'static str,
    json: &'static str,
    kind: ReportKind,
    path: &'static str,
    message_needle: &'static str,
    at_offset: &'static [&'static str],
}

const SINGLE_PROBLEM_FIXTURES: &[Fixture] = &[
    Fixture {
        what: "an unknown property",
        json: UNKNOWN_PROPERTY,
        kind: ReportKind::UnknownProperty,
        path: "Patient.nmae",
        message_needle: "nmae",
        at_offset: &[r##""nmae""##],
    },
    Fixture {
        what: "a boolean given as a string",
        json: BOOLEAN_AS_STRING,
        kind: ReportKind::IncorrectJsonType,
        path: "Patient.active",
        message_needle: "boolean",
        at_offset: &[r##""active""##, r##""true""##],
    },
    Fixture {
        what: "a repeating element given a bare object",
        json: REPEATING_GIVEN_ONE_OBJECT,
        kind: ReportKind::IncorrectJsonType,
        path: "Patient.name",
        message_needle: "name",
        at_offset: &[r##""name""##, r##"{"family""##],
    },
    Fixture {
        what: "a 0..1 element given an array",
        json: SINGLETON_GIVEN_AN_ARRAY,
        kind: ReportKind::UnexpectedRepeatingElement,
        path: "Patient.birthDate",
        message_needle: "birthDate",
        at_offset: &[r##""birthDate""##, r##"["1974-12-25""##],
    },
    Fixture {
        what: "a duplicate property",
        json: DUPLICATE_PROPERTY,
        kind: ReportKind::DuplicateProperty,
        path: "Patient.gender",
        message_needle: "gender",
        at_offset: &[r##""gender":"female""##],
    },
    Fixture {
        what: "a null in a non-primitive array",
        json: NULL_IN_COMPLEX_ARRAY,
        kind: ReportKind::NullInNonPrimitiveArray,
        path: "Patient.name",
        message_needle: "name",
        at_offset: &[r##""name""##, "null"],
    },
    Fixture {
        what: "an integer above the integer range",
        json: INTEGER_OUT_OF_RANGE,
        kind: ReportKind::ValueOutOfRange,
        path: "MolecularSequence.coordinateSystem",
        message_needle: "2147483648",
        at_offset: &[r##""coordinateSystem""##, "2147483648"],
    },
    Fixture {
        what: "a positiveInt of zero",
        json: POSITIVE_INT_ZERO,
        kind: ReportKind::ValueOutOfRange,
        path: "Media.height",
        message_needle: "positiveInt",
        at_offset: &[r##""height""##, "0}"],
    },
    Fixture {
        what: "an unsignedInt below zero",
        json: UNSIGNED_INT_NEGATIVE,
        kind: ReportKind::ValueOutOfRange,
        path: "Bundle.total",
        message_needle: "unsignedInt",
        at_offset: &[r##""total""##, "-1}"],
    },
    Fixture {
        what: "an invalid primitive value",
        json: INVALID_PRIMITIVE_VALUE,
        kind: ReportKind::InvalidValue,
        path: "Patient.birthDate",
        message_needle: "not-a-date",
        at_offset: &[r##""birthDate""##, r##""not-a-date""##],
    },
    Fixture {
        what: "unequal _field array lengths",
        json: UNEQUAL_SIBLING_ARRAYS,
        kind: ReportKind::PrimitiveExtensionLengthMismatch,
        path: "Patient.name[0].given",
        message_needle: "given",
        at_offset: &[r##""given""##, r##""_given""##],
    },
    Fixture {
        what: "a _field property on a complex element",
        json: UNDERSCORE_ON_COMPLEX_ELEMENT,
        kind: ReportKind::UnexpectedUnderscoreProperty,
        path: "Patient.name",
        message_needle: "_name",
        at_offset: &[r##""name""##, r##""_name""##],
    },
];

// ---------------------------------------------------------------------------
// model.json.001 .. model.json.023
// ---------------------------------------------------------------------------

// covers: model.json.001
#[test]
fn the_resource_type_comes_from_the_resource_type_property() {
    let d = must_decode(
        OBSERVATION_WITH_VALUE_QUANTITY.as_bytes(),
        ParseMode::Lenient,
        "an Observation whose first property is resourceType",
    );
    assert_eq!(
        d.resource_type, "Observation",
        "the type is taken from resourceType, with no caller-stated type"
    );
    assert_eq!(
        d.at("Observation").map(|e| e.type_name.as_str()),
        Some("Observation")
    );
    assert!(d.diagnostics.is_empty(), "{:?}", d.kinds());
}

// covers: model.json.002
#[test]
fn a_missing_resource_type_is_an_error() {
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(
            PATIENT_WITHOUT_RESOURCE_TYPE.as_bytes(),
            mode,
            "a document with no resourceType",
        );
        assert_eq!(r.kind, ReportKind::MissingResourceType);
        assert!(
            r.message.contains("resourceType"),
            "the error must say resourceType is required, but reads {:?}",
            r.message
        );
        assert!(
            r.offset < PATIENT_WITHOUT_RESOURCE_TYPE.len(),
            "and carry a byte offset"
        );
    }
}

// covers: model.json.003
#[test]
fn an_unknown_resource_type_is_an_error_naming_the_value() {
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(
            MISSPELLED_RESOURCE_TYPE.as_bytes(),
            mode,
            "a document whose resourceType is Patinet",
        );
        assert_eq!(r.kind, ReportKind::UnknownResourceType);
        assert!(
            r.message.contains("Patinet"),
            "the error must contain the offending value, but reads {:?}",
            r.message
        );
    }
}

// covers: model.json.004
#[test]
fn an_unknown_property_is_a_lenient_diagnostic_and_a_strict_error() {
    let input = UNKNOWN_PROPERTY.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "a Patient carrying nmae");
    assert_eq!(
        d.kinds(),
        [ReportKind::UnknownProperty],
        "LenientErrorHandler.unknownElement logs a warning and returns"
    );
    assert_located(
        &d.diagnostics[0],
        input.len(),
        "Patient.nmae",
        "nmae",
        "the unrecognised-property diagnostic",
    );

    // The property is dropped from the resource.
    assert!(
        d.at("Patient.nmae").is_none(),
        "the unrecognised property must not survive as an element: {:?}",
        d.paths()
    );
    assert!(
        !as_text(&d.reencoded_in_element_order).contains("nmae"),
        "nor be re-emitted: {}",
        as_text(&d.reencoded_in_element_order)
    );
    // The rest of the resource decodes normally, which is why a server storing
    // this body answers 2xx.
    assert_eq!(d.value("Patient.gender"), Some("male"));

    let r = must_reject(input, ParseMode::Strict, "a Patient carrying nmae");
    assert_eq!(r.kind, ReportKind::UnknownProperty);
    assert_eq!(r.path, "Patient.nmae");
}

// covers: model.json.005
#[test]
fn a_choice_element_decodes_from_its_typed_property_name() {
    let d = must_decode(
        OBSERVATION_WITH_VALUE_QUANTITY.as_bytes(),
        ParseMode::Lenient,
        "an Observation carrying valueQuantity",
    );
    let value = d
        .at("Observation.value")
        .expect("the choice element is reachable under its base name");
    assert_eq!(
        value.type_name, "Quantity",
        "Observation.value holds a Quantity, and its FHIR type name is Quantity"
    );
    assert_eq!(d.value("Observation.value.value"), Some("6.3"));
    assert_eq!(d.value("Observation.value.unit"), Some("mmol/L"));
}

// covers: model.json.006
#[test]
fn contained_resources_decode_as_resources() {
    let d = must_decode(
        CONTAINER_EXAMPLE.as_bytes(),
        ParseMode::Lenient,
        "patient-container-example.json",
    );
    let contained = d.repetitions_of("Patient.contained");
    assert_eq!(contained.len(), 1, "Patient.contained holds one resource");
    assert_eq!(contained[0].type_name, "Organization");
    assert!(
        contained[0].is_resource,
        "a contained entry is a resource, not a complex element"
    );
    assert_eq!(
        d.value("Patient.contained[0].id"),
        Some("1"),
        "whose id is 1"
    );
}

// covers: model.json.007
#[test]
fn a_bundles_entry_resources_decode_as_their_own_types() {
    let d = must_decode(
        BUNDLE_OF_MIXED_TYPES.as_bytes(),
        ParseMode::Lenient,
        "a Bundle with entries of differing resource types",
    );
    let entries: Vec<&Element> = (0..3)
        .map(|i| {
            d.at(&format!("Bundle.entry[{i}].resource"))
                .unwrap_or_else(|| {
                    panic!(
                        "Bundle.entry[{i}].resource must decode; elements were {:?}",
                        d.paths()
                    )
                })
        })
        .collect();
    assert_eq!(
        entries
            .iter()
            .map(|e| e.type_name.as_str())
            .collect::<Vec<_>>(),
        ["Patient", "Observation", "Organization"],
        "each entry decodes as the type its own resourceType names"
    );
    assert!(
        entries.iter().all(|e| e.is_resource),
        "and each reports as a resource"
    );
}

// covers: model.json.008
#[test]
fn narrative_div_is_carried_as_an_opaque_xhtml_string() {
    let input = PATIENT_WITH_NARRATIVE.as_bytes();
    let d = must_decode(input, ParseMode::Lenient, "a Patient with a text.div");

    // The div arrives as one opaque string with its escaping and entity
    // references intact. The JSON source escapes the quotes of the xmlns
    // attribute; the decoded value carries the real quotes.
    let div = d
        .value("Patient.text.div")
        .expect("text.div decodes as a string");
    assert!(
        div.starts_with("<div xmlns=\"http://www.w3.org/1999/xhtml\">"),
        "the div is the markup itself, not a parsed tree: {div}"
    );
    assert!(
        div.contains("&amp; Sons &lt;est. 1896&gt;") && div.contains("a&#160;b"),
        "and entity references are carried through unexpanded: {div}"
    );

    // Re-encoding reproduces the input exactly, which it can only do by
    // emitting the same escape sequence for the same character.
    assert_eq!(
        as_text(&d.reencoded_in_element_order),
        as_text(input),
        "the div string must be byte-identical to the input after a full re-encode"
    );
}

// covers: model.json.009
#[test]
fn a_decimals_lexical_form_is_preserved() {
    let input = PARAMETERS_WITH_DECIMALS.as_bytes();
    let d = must_decode(
        input,
        ParseMode::Lenient,
        "the JSON equivalent of parameters-example-decimals",
    );
    let out = as_text(&d.reencoded_in_element_order);

    for literal in ["1.58700", "1.00e3", "0.0014"] {
        assert!(
            out.contains(literal),
            "{literal} must be emitted with the same digits, the same trailing \
             zeros and the same exponent form; the re-encode reads {out}"
        );
    }
    assert!(
        !out.contains("1.587,") && !out.contains("1.587}"),
        "1.58700 must not be shortened to 1.587: {out}"
    );
    assert!(
        !out.contains("1000") && !out.contains("1.0E+3") && !out.contains("1.0e3"),
        "1.00e3 must keep its digits and its exponent form: {out}"
    );
    assert_eq!(
        out,
        as_text(input),
        "and a full re-encode of a document already in element order must be \
         byte-identical"
    );
}

// covers: model.json.010
#[test]
fn a_decimal_is_never_converted_through_a_binary_float() {
    for literal in ["0.1", "1e308", FORTY_DIGIT_DECIMAL] {
        let fixture = media_with_duration(literal);
        let what = format!("a Media.duration of {literal}");
        let d = must_decode(fixture.as_bytes(), ParseMode::Lenient, &what);

        assert_eq!(
            as_text(&d.reencoded_in_element_order),
            fixture,
            "{literal} must be emitted unchanged"
        );

        let element = d
            .at("Media.duration")
            .unwrap_or_else(|| panic!("{what}: Media.duration must decode"));
        let decimal = element
            .decimal
            .clone()
            .unwrap_or_else(|| panic!("{what}: Media.duration is a decimal and must read as one"));
        assert_eq!(decimal.lexical, literal, "the lexical form is retained");
        assert_eq!(
            decimal.significant_digits,
            significant_digits(literal),
            "{literal} carries {} significant digits",
            significant_digits(literal)
        );
    }

    // The 40-digit case is the one an f64 cannot hold: 17 significant digits is
    // the most a binary double round-trips, so a full-precision rendering that
    // still carries all 40 proves the value never went through one.
    let fixture = media_with_duration(FORTY_DIGIT_DECIMAL);
    let d = must_decode(
        fixture.as_bytes(),
        ParseMode::Lenient,
        "a 40-digit Media.duration",
    );
    let decimal = d
        .at("Media.duration")
        .expect("Media.duration decodes")
        .decimal
        .clone()
        .expect("and reads as a decimal");
    let binary = FORTY_DIGIT_DECIMAL
        .parse::<f64>()
        .expect("the literal parses as f64");
    assert_eq!(
        decimal.full_precision, FORTY_DIGIT_DECIMAL,
        "the arbitrary-precision value must render all 40 digits, not the \
         rounded {binary}"
    );
    assert_ne!(
        decimal.full_precision,
        format!("{binary}"),
        "and must not equal the binary approximation"
    );
}

/// Significant digits of a decimal literal, as `path.literal.006` counts them:
/// the digits of the mantissa, with the leading zeros of a value below one
/// excluded.
fn significant_digits(literal: &str) -> u32 {
    let mantissa = literal
        .split(['e', 'E'])
        .next()
        .unwrap_or(literal)
        .trim_start_matches('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let trimmed = if mantissa.starts_with("0.") || mantissa.starts_with('.') {
        digits.trim_start_matches('0')
    } else {
        digits.as_str()
    };
    trimmed.len().max(1) as u32
}

// covers: model.json.011
#[test]
fn integer_bounds_are_enforced() {
    let report = report_for(
        INTEGER_OUT_OF_RANGE.as_bytes(),
        "an integer element carrying 2147483648",
    );
    assert_eq!(report.kind, ReportKind::ValueOutOfRange);
    assert_eq!(
        report.path, "MolecularSequence.coordinateSystem",
        "the report names the element path"
    );
    assert!(
        report.message.contains("2147483648"),
        "and the offending value: {:?}",
        report.message
    );
    assert_eq!(
        report.expected_type.as_deref(),
        Some("integer"),
        "and the type the value is out of range for"
    );
}

// covers: model.json.012
#[test]
fn positive_int_and_unsigned_int_bounds_are_enforced() {
    for (json, path, declared, value) in [
        (POSITIVE_INT_ZERO, "Media.height", "positiveInt", "0"),
        (UNSIGNED_INT_NEGATIVE, "Bundle.total", "unsignedInt", "-1"),
    ] {
        let report = report_for(json.as_bytes(), &format!("a {declared} carrying {value}"));
        assert_eq!(report.kind, ReportKind::ValueOutOfRange);
        assert_eq!(report.path, path, "the report names the element path");
        assert_eq!(
            report.expected_type.as_deref(),
            Some(declared),
            "and the declared type"
        );
        assert!(
            report.message.contains(value),
            "and the offending value: {:?}",
            report.message
        );
    }
}

// covers: model.json.013
#[test]
fn a_json_type_mismatch_is_a_lenient_diagnostic_and_a_strict_error() {
    let input = BOOLEAN_AS_STRING.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "active given as a string");
    assert_eq!(
        d.kinds(),
        [ReportKind::IncorrectJsonType],
        "LenientErrorHandler.incorrectJsonType logs and returns"
    );
    let diag = d.diagnostics[0].clone();
    assert_eq!(diag.path, "Patient.active");
    assert_eq!(
        diag.expected_type.as_deref(),
        Some("boolean"),
        "the diagnostic names the expected type"
    );
    assert_eq!(
        diag.found_type.as_deref(),
        Some("string"),
        "and the JSON type found"
    );

    // HAPI passes the scalar string form to attributeValue whatever the JSON
    // type was, so the element value is that string form.
    assert_eq!(
        d.value("Patient.active"),
        Some("true"),
        "the element value is the scalar string form"
    );

    let r = must_reject(input, ParseMode::Strict, "active given as a string");
    assert_eq!(
        r, diag,
        "strict mode fails with that same information as an error"
    );
}

// covers: model.json.014
#[test]
fn a_repeating_element_given_a_single_object_is_accepted_leniently() {
    let input = REPEATING_GIVEN_ONE_OBJECT.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "name given as a bare object");
    assert_eq!(d.kinds(), [ReportKind::IncorrectJsonType]);
    assert_eq!(d.diagnostics[0].path, "Patient.name");
    assert_eq!(
        d.repetitions_of("Patient.name").len(),
        1,
        "the bare object becomes one name entry"
    );
    assert_eq!(d.value("Patient.name[0].family"), Some("Chalmers"));

    let r = must_reject(input, ParseMode::Strict, "name given as a bare object");
    assert_eq!(r.path, "Patient.name");
    assert_eq!(
        r.expected_type.as_deref(),
        Some("array"),
        "strict mode says an array was expected"
    );
}

// covers: model.json.015
#[test]
fn a_single_element_given_an_array_keeps_the_first_value_leniently() {
    let input = SINGLETON_GIVEN_AN_ARRAY.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "birthDate given as an array");
    assert_eq!(
        d.kinds(),
        [ReportKind::UnexpectedRepeatingElement],
        "LenientErrorHandler.unexpectedRepeatingElement logs and returns"
    );
    assert_eq!(d.diagnostics[0].path, "Patient.birthDate");
    assert_eq!(
        d.value("Patient.birthDate"),
        Some("1974-12-25"),
        "and the parser keeps the first repetition"
    );
    assert!(
        d.at("Patient.birthDate[1]").is_none(),
        "the second repetition is gone: {:?}",
        d.paths()
    );

    let r = must_reject(input, ParseMode::Strict, "birthDate given as an array");
    assert_eq!(r.path, "Patient.birthDate");
}

// covers: model.json.016
#[test]
fn a_duplicate_json_key_takes_the_last_occurrence_leniently() {
    let input = DUPLICATE_PROPERTY.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "gender twice");
    assert_eq!(
        d.value("Patient.gender"),
        Some("female"),
        "HAPI reads JSON through a Jackson ObjectMapper configured without \
         FAIL_ON_READING_DUP_TREE_KEY and without STRICT_DUPLICATE_DETECTION, \
         so a repeated field name silently overwrites"
    );
    assert_eq!(
        d.kinds(),
        [ReportKind::DuplicateProperty],
        "Frevo reports it, which is more than HAPI reports; HAPI has no \
         error-handler callback for this case at all, so there is nothing to \
         match and reporting it costs nothing observable"
    );
    assert!(
        d.diagnostics[0].message.contains("gender"),
        "the diagnostic names the duplicated property: {:?}",
        d.diagnostics[0].message
    );

    let r = must_reject(input, ParseMode::Strict, "gender twice");
    assert_eq!(r.kind, ReportKind::DuplicateProperty);
    assert!(r.message.contains("gender"));
}

// covers: model.json.017
#[test]
fn base64_binary_is_carried_as_its_encoded_text() {
    let input = PATIENT_WITH_ATTACHMENT.as_bytes();
    let d = must_decode(input, ParseMode::Lenient, "an Attachment.data value");

    assert_eq!(
        d.value("Patient.photo[0].data"),
        Some(ATTACHMENT_BASE64),
        "the value is the encoded text, padding and all -- not decoded bytes"
    );

    let out = d.reencoded_in_element_order.clone();
    let before = sole_range(input, ATTACHMENT_BASE64);
    let after = sole_range(&out, ATTACHMENT_BASE64);
    assert_eq!(
        &input[before], &out[after],
        "the base64 text must be byte-identical, with the padding of the input \
         and without re-encoding"
    );
    assert_eq!(as_text(&out), as_text(input));
}

// covers: model.json.018
#[test]
fn a_null_in_a_non_primitive_array_yields_a_value_less_element_leniently() {
    let input = NULL_IN_COMPLEX_ARRAY.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "name given as [null]");
    let names = d.repetitions_of("Patient.name");
    assert_eq!(
        names.len(),
        1,
        "the isNull() branch of JsonParser.parseChildren enters the element, \
         parses any _name sibling at the same index, and ends it -- it does not \
         reject"
    );
    assert_eq!(names[0].value, None, "the entry has no value");
    assert!(
        d.descendants_of("Patient.name[0]").is_empty(),
        "and no children: {:?}",
        d.descendants_of("Patient.name[0]")
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        d.kinds(),
        [ReportKind::NullInNonPrimitiveArray],
        "Frevo reports it; HAPI has no callback, so strict mode is a superset \
         here and docs/deviations.md records that"
    );

    assert!(
        as_text(&d.reencoded_in_element_order).contains(r##""name":[null]"##),
        "re-serialising the resource unchanged emits the hole again: {}",
        as_text(&d.reencoded_in_element_order)
    );

    let r = must_reject(input, ParseMode::Strict, "name given as [null]");
    assert_eq!(r.path, "Patient.name");
    assert!(
        r.message.contains("null"),
        "strict mode says null is meaningful only as a hole in a primitive \
         array paired with _name: {:?}",
        r.message
    );
}

// covers: model.json.019
#[test]
fn decoding_is_encoding_strict() {
    let (bytes, offset) = not_utf8();
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(&bytes, mode, "input that is not valid UTF-8");
        assert_eq!(r.kind, ReportKind::NotUtf8);
        assert_eq!(
            r.offset, offset,
            "the error must name the byte offset of the invalid byte"
        );
        assert!(
            r.message.contains(&offset.to_string()),
            "and say so in the message: {:?}",
            r.message
        );
    }
}

// covers: model.json.020
#[test]
fn element_id_and_extension_decode_on_any_element() {
    let input = NAME_WITH_ID_AND_EXTENSION.as_bytes();
    let d = must_decode(
        input,
        ParseMode::Lenient,
        "a HumanName with an id and an extension",
    );

    let name = d.at("Patient.name[0]").expect("the HumanName decodes");
    assert_eq!(
        name.element_id.as_deref(),
        Some("n1"),
        "the element id is readable on that element"
    );
    assert_eq!(
        name.extension_urls,
        ["https://example.org/name-qualifier"],
        "and so is its extension"
    );
    assert_eq!(
        d.value("Patient.name[0].extension[0].value"),
        Some("preferred"),
        "the extension value[x] is reachable under its base name"
    );
    assert!(
        d.diagnostics.is_empty(),
        "neither id nor extension is an unknown property: {:?}",
        d.kinds()
    );
    assert_eq!(
        as_text(&d.reencoded_in_element_order),
        as_text(input),
        "and both round-trip unchanged"
    );
}

// covers: model.json.021
#[test]
fn the_parse_mode_is_selectable_and_lenient_is_the_default() {
    let codec = codec_under_test();

    // Lenient is the default, matching the default error handler of
    // FhirContext.
    match codec.default_mode() {
        Ok(m) => assert_eq!(
            m,
            ParseMode::Lenient,
            "a decoder created with no explicit mode must be lenient"
        ),
        Err(e) => panic!("the default parse mode must be reportable: {e}"),
    }

    // So a request body HAPI 8.12.0 accepts is accepted.
    let input = UNKNOWN_PROPERTY.as_bytes();
    let by_default = match codec.decode_with_default_mode(input) {
        Ok(d) => d,
        Err(e) => panic!("the default mode must accept a body HAPI accepts: {e}"),
    };
    let explicitly_lenient = must_decode(input, ParseMode::Lenient, "an unknown property");
    assert_eq!(
        by_default, explicitly_lenient,
        "stating Lenient explicitly must change nothing"
    );

    // Every diagnostic lenient mode returns becomes an error instead, with the
    // same kind, path, offset and message -- so the two modes cannot drift
    // apart -- and nothing else about decoding changes.
    for f in SINGLE_PROBLEM_FIXTURES {
        let lenient = must_decode(f.json.as_bytes(), ParseMode::Lenient, f.what);
        assert_eq!(
            lenient.diagnostics.len(),
            1,
            "{}: this is a single-problem fixture, got {:?}",
            f.what,
            lenient.kinds()
        );
        let strict = must_reject(f.json.as_bytes(), ParseMode::Strict, f.what);
        assert_eq!(
            strict, lenient.diagnostics[0],
            "{}: strict mode must raise exactly the diagnostic lenient mode \
             records -- same kind, path, offset and message",
            f.what
        );
        assert!(
            !lenient.elements.is_empty(),
            "{}: a lenient decode still returns the resource",
            f.what
        );
    }
}

// covers: model.json.022
#[test]
fn an_extension_with_a_value_and_nested_extensions_is_fatal_in_both_modes() {
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(
            EXTENSION_WITH_VALUE_AND_NESTED.as_bytes(),
            mode,
            "an Extension with both a value[x] and nested extensions",
        );
        assert_eq!(
            r.kind,
            ReportKind::ExtensionHasValueAndNestedExtensions,
            "this is one of only two conditions LenientErrorHandler treats as \
             fatal by default, so lenient mode must fail too -- a server asked \
             to store this answers 400 rather than storing a resource that \
             cannot later be read, updated or deleted"
        );
        assert_eq!(
            r.path, "Patient.extension[0]",
            "the error names the element path of the extension"
        );
        assert!(
            r.message.contains("https://example.org/outer"),
            "and its URL: {:?}",
            r.message
        );
    }
}

// covers: model.json.023
#[test]
fn an_invalid_primitive_value_is_fatal_in_both_modes_but_a_blank_one_is_not() {
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(
            INVALID_PRIMITIVE_VALUE.as_bytes(),
            mode,
            "birthDate given as not-a-date",
        );
        assert_eq!(
            r.kind,
            ReportKind::InvalidValue,
            "an invalid value is the other condition LenientErrorHandler treats \
             as fatal, on the stated grounds that ignoring it loses data \
             silently -- a server asked to store it answers 400"
        );
        assert_eq!(r.path, "Patient.birthDate");
        assert!(
            r.message.contains("not-a-date"),
            "and the error names the offending value: {:?}",
            r.message
        );
    }

    // A blank value is NOT fatal: LenientErrorHandler.invalidValue logs it and
    // returns, so parsing continues.
    let d = must_decode(
        BLANK_PRIMITIVE_VALUE.as_bytes(),
        ParseMode::Lenient,
        "birthDate given as a blank string",
    );
    assert!(
        d.at("Patient.birthDate").is_some(),
        "a blank value is logged and parsing continues, so the element is still \
         there: {:?}",
        d.paths()
    );
    assert_eq!(
        d.resource_type, "Patient",
        "and the resource survives the blank value"
    );
}

// ---------------------------------------------------------------------------
// model.primext.001 .. model.primext.009
// ---------------------------------------------------------------------------

// covers: model.primext.001
#[test]
fn a_singleton_primitives_field_sibling_is_readable() {
    let d = must_decode(
        BIRTHDATE_WITH_SIBLING.as_bytes(),
        ParseMode::Lenient,
        "a birthDate with a _birthDate sibling",
    );
    let birth_date = d
        .at("Patient.birthDate")
        .expect("the primitive and its sibling are one element");
    assert_eq!(
        birth_date.value.as_deref(),
        Some("1974-12-25"),
        "the value of Patient.birthDate is 1974-12-25"
    );
    assert_eq!(
        birth_date.extension_urls,
        ["https://example.org/birth-time"],
        "and its extensions are the ones in _birthDate"
    );
    assert_eq!(
        d.value("Patient.birthDate.extension[0].value"),
        Some("08:00"),
        "which are navigable as ordinary extensions"
    );
    assert!(
        d.at("Patient._birthDate").is_none(),
        "the sibling is not an element in its own right: {:?}",
        d.paths()
    );
}

// covers: model.primext.002
#[test]
fn a_repeating_primitives_field_array_aligns_by_index() {
    let d = must_decode(
        NAME_EXTENSIONS_EXAMPLE.as_bytes(),
        ParseMode::Lenient,
        "patient-name-extensions.json",
    );
    let given = d.repetitions_of("Patient.name[0].given");
    assert_eq!(given.len(), 2, "two given entries");

    // Index 0 of `given` is the null hole; index 0 of `_given` is the extension
    // object. They describe the same element, so the extension lands on the
    // value-less entry and not on James. An implementation that compacted the
    // null away would pair them the other way round, which is precisely the
    // failure this scenario exists to catch.
    assert_eq!(given[0].value, None);
    assert_eq!(
        given[0].extension_urls,
        ["https://example.org/syllable-count"],
        "index 0 of given and index 0 of _given describe the same element"
    );
    assert_eq!(given[1].value.as_deref(), Some("James"));
    assert!(
        given[1].extension_urls.is_empty(),
        "index 1 of _given is absent, so the second element has no extensions: {:?}",
        given[1].extension_urls
    );
}

// covers: model.primext.003
#[test]
fn a_null_hole_in_the_value_array_means_extensions_but_no_value() {
    let d = must_decode(
        GIVEN_WITH_HOLES.as_bytes(),
        ParseMode::Lenient,
        "given [null, Peter] with _given [{...}, null]",
    );
    let given = d.repetitions_of("Patient.name[0].given");
    assert_eq!(given.len(), 2, "name.given yields two elements");

    assert_eq!(given[0].value, None, "the first has no value");
    assert_eq!(
        given[0].extension_urls,
        ["https://example.org/syllable-count"],
        "but it does have extensions"
    );

    assert_eq!(
        given[1].value.as_deref(),
        Some("Peter"),
        "the second has a value"
    );
    assert!(
        given[1].extension_urls.is_empty(),
        "and no extensions: {:?}",
        given[1].extension_urls
    );
}

// covers: model.primext.004
#[test]
fn has_value_is_false_for_an_extension_only_primitive() {
    // The model-side invariant that makes
    // Patient.name.given.select($this.hasValue()) return false, true over
    // patient-name-extensions.json -- oracle case
    // miscEngineTests/testPrimitiveExtensions, graded end to end by the
    // FHIRPath suite in frevo-path. hasValue() is value.is_some() and nothing
    // more (design.md section 3), so this is where the model side is pinned.
    let d = must_decode(
        NAME_EXTENSIONS_EXAMPLE.as_bytes(),
        ParseMode::Lenient,
        "patient-name-extensions.json",
    );
    let has_value: Vec<bool> = d
        .repetitions_of("Patient.name[0].given")
        .iter()
        .map(|e| e.value.is_some())
        .collect();
    assert_eq!(
        has_value,
        [false, true],
        "the result is the two booleans false, true in that order"
    );
}

// covers: model.primext.005
#[test]
fn a_field_sibling_with_no_value_sibling_is_still_an_element() {
    let d = must_decode(
        SIBLING_WITHOUT_VALUE.as_bytes(),
        ParseMode::Lenient,
        "a _birthDate with no birthDate",
    );
    let birth_date = d.at("Patient.birthDate").unwrap_or_else(|| {
        panic!(
            "the allUnderscoreNames > handledUnderscoreNames pass of JsonParser \
             enters the element even with no value sibling, so \
             Patient.birthDate must yield one element; elements were {:?}",
            d.paths()
        )
    });
    assert_eq!(birth_date.value, None, "one element with no value");
    assert_eq!(
        birth_date.extension_urls,
        ["https://example.org/birth-time"],
        "and the extensions from _birthDate"
    );
    assert!(
        d.repetitions_of("Patient.birthDate").is_empty(),
        "birthDate is 0..1, so it is not indexed: {:?}",
        d.paths()
    );
}

// covers: model.primext.006
#[test]
fn field_arrays_of_unequal_length_are_reported() {
    // `given` has three entries, `_given` two. The scenario says decoding
    // "reports the mismatch" without saying which side of the fatal line that
    // falls on, so report_for accepts either delivery and requires exactly one
    // report.
    //
    // CONFLICT WITH THE REFERENCE, written to the scenario as specified. HAPI
    // reports nothing here:
    // reference/hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java:1451-1458
    // iterates the *value* array and takes an alternate only
    // `if (nextAlternateArray.size() >= (i + 1))`, with no error-handler call
    // on either side of an imbalance; the extra entries of an over-long
    // `_field` array are never visited at all. So this is a Frevo-only report,
    // like model.json.016 and model.json.018 -- but unlike those two it is
    // neither admitted in the scenario text nor listed in the "Lenient request
    // parsing" note of docs/deviations.md. Raised with the caller rather than
    // coded around.
    let report = report_for(
        UNEQUAL_SIBLING_ARRAYS.as_bytes(),
        "given of three entries with _given of two",
    );
    assert_eq!(report.kind, ReportKind::PrimitiveExtensionLengthMismatch);
    assert_eq!(
        report.path, "Patient.name[0].given",
        "the mismatch is reported at path Patient.name.given"
    );
}

// covers: model.primext.007
#[test]
fn a_field_sibling_round_trips_in_place() {
    for (what, fixture) in [
        ("a singleton _field sibling", BIRTHDATE_WITH_SIBLING),
        ("an index-aligned _field array", NAME_EXTENSIONS_EXAMPLE),
        ("holes on both sides", GIVEN_WITH_HOLES),
        ("a _field with no value sibling", SIBLING_WITHOUT_VALUE),
    ] {
        let input = fixture.as_bytes();
        let d = must_decode(input, ParseMode::Lenient, what);
        assert_eq!(
            as_text(&d.reencoded_in_element_order),
            as_text(input),
            "{what}: a full re-encode must be byte-identical"
        );
    }

    // And specifically: the null holes stay where they were, rather than being
    // compacted out of either array.
    let d = must_decode(
        GIVEN_WITH_HOLES.as_bytes(),
        ParseMode::Lenient,
        "holes on both sides",
    );
    let out = as_text(&d.reencoded_in_element_order);
    assert!(
        out.contains(r##""given":[null,"Peter"]"##),
        "the hole in the value array keeps its position: {out}"
    );
    assert!(
        out.contains(r##",null]"##),
        "and so does the hole in the _given array: {out}"
    );
}

// covers: model.primext.008
#[test]
fn reading_a_primitives_value_does_not_materialise_its_extensions() {
    // The value and the _field sibling are two independent spans (design.md
    // section 3), so reading one must not force the other. This oracle grades
    // the observable half -- the value is returned, and the extension tree is
    // still whole and in order when it is asked for -- while the token-counter
    // half is graded by model.lazy.* in model_bytes_lazy_perf.rs, where the
    // instrumentation lives.
    let fixture = birthdate_with_large_sibling();
    let d = must_decode(
        fixture.as_bytes(),
        ParseMode::Lenient,
        "a birthDate whose _birthDate carries 200 extensions",
    );

    assert_eq!(
        d.value("Patient.birthDate"),
        Some("1974-12-25"),
        "the value is returned"
    );

    let birth_date = d.at("Patient.birthDate").expect("the element decodes");
    assert_eq!(
        birth_date.extension_urls.len(),
        200,
        "the whole extension tree is reachable on demand"
    );
    assert_eq!(
        birth_date.extension_urls[0], "https://example.org/e0",
        "in document order"
    );
    assert_eq!(
        as_text(&d.reencoded_in_element_order),
        fixture,
        "and the two spans re-encode in place"
    );
}

// covers: model.primext.009
#[test]
fn a_field_property_is_rejected_on_a_non_primitive_element() {
    // CONFLICT WITH THE REFERENCE, written to the scenario as specified. HAPI
    // accepts this quietly:
    // reference/hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java:1369
    // passes the `_name` value through for any element, and parseAlternates
    // (JsonParser.java:1272-1332) handles `extension`, `modifierExtension`,
    // `id` and `fhir_comments` without ever consulting whether the element is
    // primitive. Section 3 of design.md states the Frevo behaviour
    // deliberately -- "ElementDef carries whether the element is primitive, so
    // a _name property on a complex element is rejected rather than quietly
    // accepted" -- but the "Lenient request parsing" note of
    // docs/deviations.md does not list model.primext.009 among the strict-mode
    // supersets. Raised with the caller rather than coded around.
    let report = report_for(
        UNDERSCORE_ON_COMPLEX_ELEMENT.as_bytes(),
        "a _name property on Patient.name",
    );
    assert_eq!(report.kind, ReportKind::UnexpectedUnderscoreProperty);
    assert_eq!(
        report.path, "Patient.name",
        "decoding reports an unexpected underscore property at Patient.name"
    );
    assert!(
        report.message.contains("_name"),
        "naming the property: {:?}",
        report.message
    );
}

// ---------------------------------------------------------------------------
// model.errors.001 .. model.errors.005
// ---------------------------------------------------------------------------

// covers: model.errors.001
#[test]
fn every_decode_error_carries_an_element_path() {
    for f in SINGLE_PROBLEM_FIXTURES {
        let r = must_reject(f.json.as_bytes(), ParseMode::Strict, f.what);
        assert_eq!(r.kind, f.kind, "{}: the condition reported", f.what);
        assert_eq!(
            r.path, f.path,
            "{}: the error must name the FHIR element path",
            f.what
        );
        assert!(
            !r.path.contains('/'),
            "{}: {:?} reads as a JSON pointer, not an element path",
            f.what,
            r.path
        );
        assert!(
            r.message.contains(f.message_needle),
            "{}: the message must mention {:?}: {:?}",
            f.what,
            f.message_needle,
            r.message
        );
    }

    // The two conditions that are fatal in both modes name a path too.
    for (what, json) in [
        (
            "an extension with a value and nested extensions",
            EXTENSION_WITH_VALUE_AND_NESTED,
        ),
        ("an invalid primitive value", INVALID_PRIMITIVE_VALUE),
    ] {
        let r = must_reject(json.as_bytes(), ParseMode::Lenient, what);
        assert!(
            !r.path.is_empty() && !r.path.contains('/'),
            "{what}: the error must name an element path, got {:?}",
            r.path
        );
    }
}

// covers: model.errors.002
#[test]
fn every_decode_error_carries_a_byte_offset() {
    for f in SINGLE_PROBLEM_FIXTURES {
        let bytes = f.json.as_bytes();
        let r = must_reject(bytes, ParseMode::Strict, f.what);
        assert!(
            r.offset < bytes.len(),
            "{}: the offset {} must point inside the {}-byte input",
            f.what,
            r.offset,
            bytes.len()
        );
        let tail = &f.json[r.offset..];
        assert!(
            f.at_offset.iter().any(|t| tail.starts_with(t)),
            "{}: the offset must name where the problem was found. Byte {} \
             starts {:?}; one of {:?} was expected",
            f.what,
            r.offset,
            peek(bytes, r.offset),
            f.at_offset
        );
    }
}

// covers: model.errors.003
#[test]
fn strict_reports_the_first_error_and_lenient_reports_every_diagnostic() {
    let input = THREE_INDEPENDENT_PROBLEMS.as_bytes();

    let d = must_decode(input, ParseMode::Lenient, "three independent problems");
    assert_eq!(
        d.kinds(),
        [
            ReportKind::UnknownProperty,
            ReportKind::IncorrectJsonType,
            ReportKind::UnexpectedRepeatingElement,
        ],
        "a lenient decode succeeds and returns every diagnostic, because HAPI \
         logs each one and continues"
    );
    assert_eq!(
        d.diagnostics
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        ["Patient.nmae", "Patient.active", "Patient.birthDate"],
        "in document order"
    );
    let offsets: Vec<usize> = d.diagnostics.iter().map(|r| r.offset).collect();
    assert!(
        offsets.windows(2).all(|w| w[0] < w[1]),
        "and document order means ascending byte offsets, got {offsets:?}"
    );

    let r = must_reject(input, ParseMode::Strict, "three independent problems");
    assert_eq!(
        r, d.diagnostics[0],
        "a strict decode returns an error for the first problem in document \
         order and does not panic"
    );
}

// covers: model.errors.004
#[test]
fn truncated_input_is_an_error_not_a_panic() {
    let input = CONTAINER_EXAMPLE.as_bytes();
    let codec = codec_under_test();

    // The untruncated document must decode, so that a codec which rejected
    // every input could not pass this by rejecting everything.
    match codec.decode(input, ParseMode::Lenient) {
        Ok(d) => assert_eq!(d.resource_type, "Patient"),
        Err(e) => panic!("the untruncated document must decode: {e}"),
    }

    for cut in 0..input.len() {
        let prefix = &input[..cut];
        match codec.decode(prefix, ParseMode::Lenient) {
            Ok(_) => {}
            Err(DecodeError::Fatal(r)) => assert!(
                r.offset <= prefix.len(),
                "the offset {} must point inside the {cut}-byte prefix",
                r.offset
            ),
            Err(DecodeError::NotImplemented) => {
                panic!("the {cut}-byte prefix: {NOT_BOUND}")
            }
        }
    }
}

// covers: model.errors.005
#[test]
fn adversarially_nested_input_is_bounded() {
    let bytes = deeply_nested(10_000);
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let r = must_reject(&bytes, mode, "a document nested 10,000 deep");
        assert_eq!(
            r.kind,
            ReportKind::DepthLimit,
            "decoding must return a depth-limit error rather than overflowing \
             the stack"
        );
        assert!(
            r.offset < bytes.len(),
            "and name where the limit was reached"
        );
    }

    // A document within the bound still decodes, so the limit is a bound and
    // not a blanket refusal.
    match codec_under_test().decode(CONTAINER_EXAMPLE.as_bytes(), ParseMode::Lenient) {
        Ok(d) => assert_eq!(d.resource_type, "Patient"),
        Err(e) => panic!("an ordinarily nested document must decode: {e}"),
    }
}
