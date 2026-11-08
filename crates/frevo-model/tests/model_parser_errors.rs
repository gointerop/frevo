//! The `model-parser-errors` in-process oracle.
//!
//! Task 5a of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo nextest run -p frevo-model --test model_parser_errors
//! ```
//!
//! Why an oracle of its own. Three of HAPI's ten parser error conditions cannot
//! be reached by the `r4-example-roundtrip` oracle, because no example in
//! `hl7.fhir.r4.examples#4.0.1` carries a contained resource without an `id` or
//! a dangling `#` reference, and the third fires only on the **encode** side.
//! So they are graded here, over hand-built documents — one per condition,
//! asserted in both parse modes.
//!
//! | Condition | Side | HAPI code | Scenario |
//! |---|---|---:|---|
//! | a contained resource with no `id` | decode | 1819 | `model.json.024` |
//! | a `#` reference whose target is not contained | decode | 1826 | `model.json.025` |
//! | encoding a `#` reference that matches no contained resource | encode | 2724 | `model.canonical.007` |
//!
//! Provenance, all at HAPI pin `7ab4c7c7c49281496faa1b3c2498a652457669ae`:
//!
//! - `ca.uhn.fhir.parser.StrictErrorHandler` lines 38-40 (1819), 93-97 (1826)
//!   and 99-105 (2724) are where the codes come from;
//! - `ca.uhn.fhir.parser.LenientErrorHandler` logs all three and aborts for
//!   neither, which is why lenient mode succeeds;
//! - `ca.uhn.fhir.parser.ParserState.ContainedResourcesStateHl7Org.wereBack()`
//!   is where 1819 is raised. Its own comment says "we don't keep the resource
//!   because it's useless", and then the code appends it to `contained`
//!   anyway. What it actually skips is the id-to-resource map, so the resource
//!   survives and no `#` reference can ever be woven to it;
//! - `ParserState.weaveContainedResources()` is the post-decode pass that
//!   raises 1826. It tests `isNotBlank(ref)` and
//!   `ref.startsWith("#") && ref.length() > 1`, so a reference of exactly `#`
//!   or a blank one is never looked up at all;
//! - `BaseParser.determineReferenceText` and `BaseParser.isValidInternalReference`
//!   raise 2724 on encode. `isValidInternalReference` returns true for a
//!   non-local reference, for a reference whose value is exactly `#`, and for
//!   one matching a contained resource — the match being against
//!   `FhirTerser.ContainedResources.referenceMatchesAContainedResource`, which
//!   compares the text after `#` with each contained resource's id part. A
//!   reference with a blank id part never reaches the check: it takes the
//!   contained-resource-naming branch at the head of `determineReferenceText`.
//!
//! The oracle runs against a trait object, so it compiles and runs before the
//! codec exists. The unbound binding returns [`ParserError::NotImplemented`]
//! from both surfaces, which is an error value and not a panicking
//! placeholder.
//!
//! Scenario coverage: `model.json.024`, `model.json.025`, `model.canonical.007`.

// ---------------------------------------------------------------------------
// HAPI's message codes for the three conditions
// ---------------------------------------------------------------------------

/// `StrictErrorHandler.containedResourceWithNoId`.
pub const CODE_CONTAINED_RESOURCE_WITH_NO_ID: u32 = 1819;
/// `StrictErrorHandler.unknownReference`.
pub const CODE_UNKNOWN_REFERENCE: u32 = 1826;
/// `StrictErrorHandler.invalidInternalReference` — the only error condition on
/// the encode side.
pub const CODE_INVALID_INTERNAL_REFERENCE: u32 = 2724;

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

/// The two parse modes, as `FhirContext.setParserErrorHandler` selects them.
/// Lenient is the default, because that is what `FhirContext` installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Lenient,
    Strict,
}

/// One diagnostic: HAPI's message code, the message, and the element path and
/// byte offset `model.errors.001` and `model.errors.002` require.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: u32,
    pub message: String,
    pub path: String,
    pub offset: usize,
}

/// A local (`#`) reference the decode saw, and what the weaving pass did with
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalReference {
    /// The element the reference sits on, e.g. `Patient.managingOrganization`.
    pub path: String,
    /// The reference's literal text, which survives in both modes — it is what
    /// `path.utility.017` later resolves against.
    pub text: String,
    /// Whether the post-decode weaving pass found a contained resource for it.
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeOutcome {
    pub diagnostics: Vec<Diagnostic>,
    /// The `contained` entries in document order, with `None` where the entry
    /// carried no `id`. Length is the number of resources that survived.
    pub contained_ids: Vec<Option<String>>,
    /// The ids a `#` reference can be woven to — the id-to-resource map's
    /// keys, which is **not** the same list as `contained_ids`.
    pub referenceable_contained_ids: Vec<String>,
    pub local_references: Vec<LocalReference>,
    /// The surviving resource, canonically re-encoded, so what survived is
    /// assertable rather than described.
    pub canonical_json: String,
}

impl DecodeOutcome {
    pub fn codes(&self) -> Vec<u32> {
        self.diagnostics.iter().map(|d| d.code).collect()
    }

    pub fn reference_at(&self, path: &str) -> Option<&LocalReference> {
        self.local_references.iter().find(|r| r.path == path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeOutcome {
    pub diagnostics: Vec<Diagnostic>,
    pub canonical_json: String,
}

impl EncodeOutcome {
    pub fn codes(&self) -> Vec<u32> {
        self.diagnostics.iter().map(|d| d.code).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    /// The operation failed. In strict mode this is the diagnostic lenient mode
    /// would have returned, raised instead of recorded.
    Fatal(Diagnostic),
    /// No codec is bound to this oracle yet.
    NotImplemented,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::Fatal(d) => write!(
                f,
                "[{}] {} at {} (byte {})",
                d.code, d.message, d.path, d.offset
            ),
            ParserError::NotImplemented => f.write_str(
                "no codec is bound to the model-parser-errors oracle yet \
                 (crates/frevo-model/tests/model_parser_errors.rs::oracle_under_test)",
            ),
        }
    }
}

pub trait ParserOracle {
    /// Decode a document in the given mode.
    fn decode(&self, json: &str, mode: ParseMode) -> Result<DecodeOutcome, ParserError>;

    /// Decode the document **leniently** and then canonically encode it in the
    /// given mode.
    ///
    /// The decode is always lenient so that a fixture carrying a decode-side
    /// diagnostic is still expressible on the encode side: the point of the
    /// 2724 case is that a document HAPI accepts on the way in is diagnosed
    /// again on the way out.
    fn encode(&self, json: &str, mode: ParseMode) -> Result<EncodeOutcome, ParserError>;
}

/// The codec this oracle grades.
///
/// Tasks 21a and 23a replace this body with the real one.
fn oracle_under_test() -> Box<dyn ParserOracle> {
    Box::new(UnboundCodec)
}

struct UnboundCodec;

impl ParserOracle for UnboundCodec {
    fn decode(&self, _json: &str, _mode: ParseMode) -> Result<DecodeOutcome, ParserError> {
        Err(ParserError::NotImplemented)
    }

    fn encode(&self, _json: &str, _mode: ParseMode) -> Result<EncodeOutcome, ParserError> {
        Err(ParserError::NotImplemented)
    }
}

// ---------------------------------------------------------------------------
// The hand-built documents, one per condition
// ---------------------------------------------------------------------------

/// A contained `Organization` with no `id`, and nothing referring to it.
const CONTAINED_WITHOUT_ID: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""contained":[{"resourceType":"Organization","name":"Acme Healthcare"}]}"##
);

/// The same, plus a `#` reference that could only ever have pointed at it.
/// Both conditions fire, which is the point: a contained resource with no `id`
/// is never entered into the id-to-resource map, so no `#` reference can be
/// woven to it.
const CONTAINED_WITHOUT_ID_AND_REFERENCE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""contained":[{"resourceType":"Organization","name":"Acme Healthcare"}],"##,
    r##""managingOrganization":{"reference":"#org1"}}"##
);

/// A `#` reference whose target is not among `contained`.
const DANGLING_LOCAL_REFERENCE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""contained":[{"resourceType":"Organization","id":"org2","name":"Other"}],"##,
    r##""managingOrganization":{"reference":"#org1"}}"##
);

/// The reference appears **before** its target. The weaving pass runs after the
/// whole document is decoded, so it still resolves.
const REFERENCE_BEFORE_ITS_TARGET: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""managingOrganization":{"reference":"#org1"},"##,
    r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme"}]}"##
);

/// A reference of exactly `#` — a reference to the containing resource.
const REFERENCE_TO_THE_CONTAINER: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""managingOrganization":{"reference":"#"}}"##
);

/// A reference whose id part is blank.
const REFERENCE_WITH_BLANK_ID_PART: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""managingOrganization":{"reference":""}}"##
);

/// A non-local reference, with nothing contained.
const NON_LOCAL_REFERENCE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""managingOrganization":{"reference":"Organization/org1"}}"##
);

/// A `#` reference that does match a contained resource.
const RESOLVED_LOCAL_REFERENCE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme"}],"##,
    r##""managingOrganization":{"reference":"#org1"}}"##
);

fn expect_strict_failure(err: ParserError, code: u32, what: &str) -> Diagnostic {
    match err {
        ParserError::Fatal(d) => {
            assert_eq!(
                d.code, code,
                "{what} must fail strict mode with HAPI message code {code}, not {}",
                d.code
            );
            assert!(
                !d.path.is_empty(),
                "{what}: the strict error must carry an element path"
            );
            d
        }
        ParserError::NotImplemented => {
            panic!("{what}: {}", ParserError::NotImplemented)
        }
    }
}

// ---------------------------------------------------------------------------
// model.json.024 — a contained resource with no id
// ---------------------------------------------------------------------------

// covers: model.json.024
#[test]
fn a_contained_resource_with_no_id_survives_lenient_decode_but_cannot_be_referenced() {
    let oracle = oracle_under_test();

    let out = oracle
        .decode(CONTAINED_WITHOUT_ID, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("a lenient decode must succeed: {e}"));

    assert_eq!(
        out.codes(),
        [CODE_CONTAINED_RESOURCE_WITH_NO_ID],
        "exactly one diagnostic, HAPI's code 1819"
    );

    // The resource is still appended to `contained`. HAPI's comment at the
    // call site says the opposite of what the code does: it skips the
    // id-to-resource map, not the list.
    assert_eq!(
        out.contained_ids,
        vec![None],
        "the id-less contained resource must survive in the contained list"
    );
    assert!(
        out.canonical_json.contains("Acme Healthcare"),
        "the surviving resource must still carry its content: {}",
        out.canonical_json
    );

    // But it is not referenceable, so no `#` reference can ever be woven to
    // it.
    assert!(
        out.referenceable_contained_ids.is_empty(),
        "an id-less contained resource must not enter the id-to-resource map: {:?}",
        out.referenceable_contained_ids
    );

    // With a `#` reference present, both conditions fire and the reference
    // stays unresolved.
    let both = oracle
        .decode(CONTAINED_WITHOUT_ID_AND_REFERENCE, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("a lenient decode must succeed: {e}"));
    assert!(both.codes().contains(&CODE_CONTAINED_RESOURCE_WITH_NO_ID));
    assert!(
        both.codes().contains(&CODE_UNKNOWN_REFERENCE),
        "the reference cannot be woven to an id-less contained resource: {:?}",
        both.codes()
    );
    let r = both
        .reference_at("Patient.managingOrganization")
        .expect("the local reference is recorded");
    assert_eq!(r.text, "#org1");
    assert!(!r.resolved);

    // Strict mode fails with the same code.
    let err = oracle
        .decode(CONTAINED_WITHOUT_ID, ParseMode::Strict)
        .expect_err("strict mode must reject a contained resource with no id");
    let strict = expect_strict_failure(
        err,
        CODE_CONTAINED_RESOURCE_WITH_NO_ID,
        "a contained resource with no id",
    );
    assert_eq!(
        strict.code, out.diagnostics[0].code,
        "strict mode raises the diagnostic lenient mode records; it invents nothing"
    );
}

// ---------------------------------------------------------------------------
// model.json.025 — a dangling # reference
// ---------------------------------------------------------------------------

// covers: model.json.025
#[test]
fn a_local_reference_with_no_matching_contained_resource_is_diagnosed_not_resolved() {
    let oracle = oracle_under_test();

    let out = oracle
        .decode(DANGLING_LOCAL_REFERENCE, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("a lenient decode must succeed: {e}"));

    assert_eq!(out.codes(), [CODE_UNKNOWN_REFERENCE]);
    assert!(
        out.diagnostics[0].message.contains("#org1"),
        "the diagnostic must name the reference text: {}",
        out.diagnostics[0].message
    );

    // The reference's literal text survives, which is what path.utility.017
    // later resolves against.
    let r = out
        .reference_at("Patient.managingOrganization")
        .expect("the local reference is recorded");
    assert_eq!(r.text, "#org1");
    assert!(!r.resolved, "it must not resolve to the unrelated org2");
    assert!(
        out.canonical_json.contains("#org1"),
        "the reference text must survive the decode: {}",
        out.canonical_json
    );
    // The contained resource that *is* properly identified is unaffected.
    assert_eq!(out.contained_ids, vec![Some("org2".to_string())]);
    assert_eq!(out.referenceable_contained_ids, vec!["org2".to_string()]);

    // Strict mode fails with code 1826.
    let err = oracle
        .decode(DANGLING_LOCAL_REFERENCE, ParseMode::Strict)
        .expect_err("strict mode must reject a dangling local reference");
    let strict = expect_strict_failure(err, CODE_UNKNOWN_REFERENCE, "a dangling # reference");
    assert!(strict.message.contains("#org1"));

    // The pass runs after the whole document is decoded, so a reference that
    // appears before its target still resolves — in both modes.
    for mode in [ParseMode::Lenient, ParseMode::Strict] {
        let ok = oracle
            .decode(REFERENCE_BEFORE_ITS_TARGET, mode)
            .unwrap_or_else(|e| panic!("a forward reference must resolve in {mode:?}: {e}"));
        assert!(
            ok.diagnostics.is_empty(),
            "a forward reference is not a diagnostic: {:?}",
            ok.codes()
        );
        assert!(
            ok.reference_at("Patient.managingOrganization")
                .expect("recorded")
                .resolved
        );
    }

    // A reference that does match is not diagnosed either.
    let resolved = oracle
        .decode(RESOLVED_LOCAL_REFERENCE, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(resolved.diagnostics.is_empty());
    assert!(
        resolved
            .reference_at("Patient.managingOrganization")
            .expect("recorded")
            .resolved
    );
}

// covers: model.json.025
#[test]
fn a_bare_hash_or_a_blank_reference_is_never_looked_up_on_decode() {
    let oracle = oracle_under_test();

    // `weaveContainedResources` tests `isNotBlank(ref)` and
    // `ref.startsWith("#") && ref.length() > 1`, so neither of these is looked
    // up at all — in either mode.
    for (fixture, what) in [
        (REFERENCE_TO_THE_CONTAINER, "a reference of exactly #"),
        (
            REFERENCE_WITH_BLANK_ID_PART,
            "a reference with a blank id part",
        ),
        (NON_LOCAL_REFERENCE, "a non-local reference"),
    ] {
        for mode in [ParseMode::Lenient, ParseMode::Strict] {
            let out = oracle
                .decode(fixture, mode)
                .unwrap_or_else(|e| panic!("{what} must decode cleanly in {mode:?}: {e}"));
            assert!(
                !out.codes().contains(&CODE_UNKNOWN_REFERENCE),
                "{what} must not be diagnosed in {mode:?}: {:?}",
                out.codes()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// model.canonical.007 — the encode side
// ---------------------------------------------------------------------------

// covers: model.canonical.007
#[test]
fn encoding_a_local_reference_with_no_matching_contained_resource_is_diagnosed() {
    let oracle = oracle_under_test();

    let out = oracle
        .encode(DANGLING_LOCAL_REFERENCE, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("a lenient encode must succeed: {e}"));

    assert_eq!(
        out.codes(),
        [CODE_INVALID_INTERNAL_REFERENCE],
        "exactly one diagnostic on the encode side, HAPI's code 2724"
    );
    let d = &out.diagnostics[0];
    assert!(
        d.message.contains("#org1"),
        "the diagnostic must name the reference: {}",
        d.message
    );
    assert!(
        !d.path.is_empty(),
        "the diagnostic must name the element it was encoding"
    );

    // The reference is emitted unchanged either way.
    assert!(
        out.canonical_json.contains("#org1"),
        "the reference must be emitted unchanged: {}",
        out.canonical_json
    );

    // Strict mode raises the same code.
    let err = oracle
        .encode(DANGLING_LOCAL_REFERENCE, ParseMode::Strict)
        .expect_err("strict mode must reject the dangling reference on encode");
    let strict = expect_strict_failure(
        err,
        CODE_INVALID_INTERNAL_REFERENCE,
        "encoding a dangling # reference",
    );
    assert!(strict.message.contains("#org1"));

    // This is the only error condition on the encode side: nothing else in
    // these fixtures is diagnosed on the way out.
    let resolved = oracle
        .encode(RESOLVED_LOCAL_REFERENCE, ParseMode::Strict)
        .unwrap_or_else(|e| panic!("a matching reference must encode cleanly: {e}"));
    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.canonical_json.contains("#org1"));
}

// covers: model.canonical.007
#[test]
fn the_encode_side_exemptions_are_not_diagnosed() {
    let oracle = oracle_under_test();

    // `isValidInternalReference` returns true for a reference of exactly `#`
    // (a reference to the containing resource) and for a non-local reference,
    // which is not checked on this path at all. A reference with a blank id
    // part never reaches the check: it takes the contained-resource-naming
    // branch at the head of `determineReferenceText`.
    for (fixture, what) in [
        (REFERENCE_TO_THE_CONTAINER, "a reference of exactly #"),
        (
            REFERENCE_WITH_BLANK_ID_PART,
            "a reference with a blank id part",
        ),
        (NON_LOCAL_REFERENCE, "a non-local reference"),
    ] {
        for mode in [ParseMode::Lenient, ParseMode::Strict] {
            let out = oracle
                .encode(fixture, mode)
                .unwrap_or_else(|e| panic!("{what} must encode cleanly in {mode:?}: {e}"));
            assert!(
                !out.codes().contains(&CODE_INVALID_INTERNAL_REFERENCE),
                "{what} must not be diagnosed in {mode:?}: {:?}",
                out.codes()
            );
        }
    }

    // And a document whose only problem is on the decode side is not reported
    // twice: encoding a resource with an id-less contained resource raises
    // 1819 on the way in, not 2724 on the way out.
    let out = oracle
        .encode(CONTAINED_WITHOUT_ID, ParseMode::Lenient)
        .unwrap_or_else(|e| panic!("a lenient encode must succeed: {e}"));
    assert!(
        !out.codes().contains(&CODE_INVALID_INTERNAL_REFERENCE),
        "nothing refers to the id-less resource, so there is nothing to diagnose on \
         encode: {:?}",
        out.codes()
    );
}

// covers: model.json.024
// covers: model.json.025
// covers: model.canonical.007
#[test]
fn the_three_message_codes_are_the_ones_hapi_raises() {
    // Recorded from ca.uhn.fhir.parser.StrictErrorHandler at HAPI pin
    // 7ab4c7c7c49281496faa1b3c2498a652457669ae: lines 38-40, 93-97 and 99-105.
    assert_eq!(CODE_CONTAINED_RESOURCE_WITH_NO_ID, 1819);
    assert_eq!(CODE_UNKNOWN_REFERENCE, 1826);
    assert_eq!(CODE_INVALID_INTERNAL_REFERENCE, 2724);

    // And they are what the oracle actually reports, rather than constants
    // nothing checks.
    let oracle = oracle_under_test();
    let decode_codes: Vec<u32> = [CONTAINED_WITHOUT_ID, DANGLING_LOCAL_REFERENCE]
        .iter()
        .map(|f| match oracle.decode(f, ParseMode::Strict) {
            Err(ParserError::Fatal(d)) => d.code,
            Err(e) => panic!("{e}"),
            Ok(_) => panic!("strict mode must reject {f}"),
        })
        .collect();
    assert_eq!(
        decode_codes,
        [CODE_CONTAINED_RESOURCE_WITH_NO_ID, CODE_UNKNOWN_REFERENCE]
    );

    let encode_code = match oracle.encode(DANGLING_LOCAL_REFERENCE, ParseMode::Strict) {
        Err(ParserError::Fatal(d)) => d.code,
        Err(e) => panic!("{e}"),
        Ok(_) => panic!("strict mode must reject the dangling reference on encode"),
    };
    assert_eq!(encode_code, CODE_INVALID_INTERNAL_REFERENCE);
}
