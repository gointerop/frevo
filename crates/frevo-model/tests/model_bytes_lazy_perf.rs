//! The `model-bytes-lazy-perf` in-process oracle: byte pass-through, lazy
//! materialisation, and the model performance budgets.
//!
//! Task 5 of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo test -p frevo-model --test model_bytes_lazy_perf
//! ```
//!
//! Scenario coverage, 18 of the 80 uncovered `model.*` scenarios:
//!
//! - `model.bytes.001` .. `model.bytes.006` (6);
//! - `model.lazy.001` .. `model.lazy.006` (6);
//! - `model.perf.001` .. `model.perf.006` (6).
//!
//! ## Why these three areas share a seam
//!
//! All eighteen are about the same thing: constitution rules 6.1 and 6.2, the
//! reason `frevo-model` exists rather than a `serde` derive. They are stated as
//! observable behaviour and **measured**, not asserted in a comment, so the
//! seam is an instrumented one: a script of operations driven over a document,
//! and a result that reports the byte output, the number of JSON tokens
//! scanned, the allocations that scaled with the document, and which element
//! paths were materialised.
//!
//! The counters are not debug conveniences: `model.bytes.002` and
//! `model.perf.001` are stated in terms of a token counter the codec exposes,
//! so the counter is part of the specified surface.
//!
//! ## How `model.perf.*` is made to fail for the right reason
//!
//! A budget stated as "the median time is at most 1 microsecond" cannot be
//! graded by an in-process `cargo test` alone: the test profile is unoptimised,
//! and asserting a release-profile number against a debug build would be a
//! false red once the codec lands, while skipping the assertion in debug would
//! be a false green. So every `model.perf.*` test here has two halves, and both
//! must hold:
//!
//! 1. **The recorded half.** The absolute budget is graded against the median
//!    the `bench/micro/` workload recorded, read from `bench/micro/results.json`
//!    (and, for `model.perf.006`, `bench/micro/baseline.json`). This is the
//!    gate condition as `design.md` section 8 states it. It fails today because
//!    neither the workload nor the recorded result exists -- which is exactly
//!    what "the capability is missing" means for a measured budget.
//! 2. **The in-process half.** Everything in the scenario that is a *ratio* or
//!    an *invariant* rather than an absolute time is measured here and asserted
//!    unconditionally, because a ratio survives the change of build profile:
//!    zero tokens scanned for a pass-through (`model.perf.001`), a lazy read at
//!    most a tenth of a full parse (`model.perf.002`), the 4 KiB and 64 KiB
//!    metadata updates within 25% of each other (`model.perf.004`).
//!
//! The file format the recorded half expects is documented on
//! [`MICRO_RESULTS`]. It mirrors `bench/baseline-hapi-8.12.json`, which is the
//! convention already in the repository: a `workloads` array of objects keyed
//! by `name`.
//!
//! ## The seam
//!
//! The unbound binding returns [`CodecError::NotImplemented`] from every
//! surface, and every helper turns that into an explicit "nothing is bound yet"
//! failure rather than letting it read as a pass.

use std::fmt;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

/// One operation in a script driven over a loaded resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Read one element's primitive value, by FHIR element path.
    Read(&'static str),
    /// Count the repetitions of a repeating element, without reading them.
    Count(&'static str),
    /// Set one element's primitive value to the given lexical form.
    Set(&'static str, &'static str),
    /// Materialise the whole document -- the `model-full-parse` workload.
    MaterialiseAll,
}

/// What one [`Op::Read`] returned, with the token counter at the moment it
/// returned so that `model.lazy.004` can assert "zero additional tokens".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadResult {
    pub path: String,
    pub value: Option<String>,
    /// The FHIR type name of the element that was read.
    pub type_name: Option<String>,
    /// The codec's cumulative token counter when this read returned.
    pub tokens_scanned_so_far: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    /// One entry per [`Op::Read`], in script order.
    pub reads: Vec<ReadResult>,
    /// One entry per [`Op::Count`], in script order.
    pub counts: Vec<usize>,
    /// JSON tokens scanned over the whole run. Zero for a pass-through.
    pub tokens_scanned: u64,
    /// Allocations whose size scaled with the document. Zero for a
    /// pass-through: an `Arc` clone is not proportional to the document.
    pub size_proportional_allocations: u64,
    /// Every element path the run materialised, in the order it happened.
    pub materialised: Vec<String>,
    /// Whether the narrative's XHTML was parsed or validated during the run.
    pub xhtml_validated: bool,
    /// The bytes produced by serialising at the end of the script.
    pub output: Vec<u8>,
    /// Whether the resource reports itself as modified.
    pub modified: bool,
}

impl RunResult {
    pub fn read(&self, path: &str) -> &ReadResult {
        self.reads
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| {
                panic!(
                    "no read of {path:?} in the run; reads were {:?}",
                    self.reads.iter().map(|r| &r.path).collect::<Vec<_>>()
                )
            })
    }

    /// Every materialised path at or below `prefix`.
    pub fn materialised_under(&self, prefix: &str) -> Vec<&str> {
        self.materialised
            .iter()
            .map(|p| p.as_str())
            .filter(|p| {
                *p == prefix
                    || p.starts_with(&format!("{prefix}."))
                    || p.starts_with(&format!("{prefix}["))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    Failed(String),
    /// No codec is bound to this oracle yet.
    NotImplemented,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Failed(m) => write!(f, "the codec failed: {m}"),
            CodecError::NotImplemented => f.write_str(NOT_BOUND),
        }
    }
}

const NOT_BOUND: &str = "no codec is bound to the model-bytes-lazy-perf oracle \
     yet (crates/frevo-model/tests/model_bytes_lazy_perf.rs::codec_under_test)";

pub trait LazyOracle {
    /// Load a resource from stored canonical JSON bytes, run the script, then
    /// serialise. An empty script is the pass-through case: nothing read and
    /// nothing changed.
    fn run(&self, bytes: &[u8], ops: &[Op]) -> Result<RunResult, CodecError>;

    /// Construct a resource in memory rather than decoding it -- the oracle
    /// supplies element paths and lexical values, in the order given, which is
    /// deliberately **not** the generated element order -- then run the script
    /// and serialise in the stored form.
    fn build(
        &self,
        resource_type: &str,
        properties: &[(&str, &str)],
        ops: &[Op],
    ) -> Result<RunResult, CodecError>;

    /// Construct as [`LazyOracle::build`] does, then serialise canonically.
    /// This is the `model-canonical-encode` workload.
    fn build_then_canonicalise(
        &self,
        resource_type: &str,
        properties: &[(&str, &str)],
    ) -> Result<Vec<u8>, CodecError>;
}

/// The codec this oracle grades. Phase C replaces this body with the real one.
fn codec_under_test() -> Box<dyn LazyOracle> {
    Box::new(FrevoCodec)
}

struct FrevoCodec;

impl FrevoCodec {
    fn owned(bytes: &[u8]) -> std::sync::Arc<[u8]> {
        // The adapter's copy, not the codec's: the server gets an owned buffer
        // from the database driver and moves it into the Arc.
        std::sync::Arc::from(bytes.to_vec().into_boxed_slice())
    }

    fn drive(
        resource: &mut frevo_model::Resource,
        ops: &[Op],
    ) -> Result<(Vec<ReadResult>, Vec<usize>), CodecError> {
        let mut reads = Vec::new();
        let mut counts = Vec::new();
        for op in ops {
            match op {
                Op::Read(path) => {
                    let found = resource
                        .read(path)
                        .map_err(|e| CodecError::Failed(e.to_string()))?;
                    let (type_name, value) = match found {
                        Some((type_name, value)) => (Some(type_name), value),
                        None => (None, None),
                    };
                    reads.push(ReadResult {
                        path: (*path).to_string(),
                        value,
                        type_name,
                        tokens_scanned_so_far: resource.report().tokens_scanned,
                    });
                }
                Op::Count(path) => counts.push(
                    resource
                        .count(path)
                        .map_err(|e| CodecError::Failed(e.to_string()))?,
                ),
                Op::Set(path, value) => resource
                    .set(path, value)
                    .map_err(|e| CodecError::Failed(e.to_string()))?,
                Op::MaterialiseAll => resource
                    .materialise_all()
                    .map_err(|e| CodecError::Failed(e.to_string()))?,
            }
        }
        Ok((reads, counts))
    }

    fn finish(
        resource: &frevo_model::Resource,
        reads: Vec<ReadResult>,
        counts: Vec<usize>,
    ) -> Result<RunResult, CodecError> {
        let output = resource
            .to_stored_vec()
            .map_err(|e| CodecError::Failed(e.to_string()))?;
        let report = resource.report();
        Ok(RunResult {
            reads,
            counts,
            tokens_scanned: report.tokens_scanned,
            size_proportional_allocations: report.size_proportional_allocations,
            materialised: report.materialised,
            xhtml_validated: report.xhtml_validated,
            output,
            modified: resource.is_modified(),
        })
    }
}

impl LazyOracle for FrevoCodec {
    fn run(&self, bytes: &[u8], ops: &[Op]) -> Result<RunResult, CodecError> {
        let mut resource = frevo_model::Resource::from_stored(Self::owned(bytes));
        let (reads, counts) = Self::drive(&mut resource, ops)?;
        Self::finish(&resource, reads, counts)
    }

    fn build(
        &self,
        resource_type: &str,
        properties: &[(&str, &str)],
        ops: &[Op],
    ) -> Result<RunResult, CodecError> {
        let mut resource = frevo_model::Resource::build(resource_type);
        for (path, value) in properties {
            resource
                .set(path, value)
                .map_err(|e| CodecError::Failed(e.to_string()))?;
        }
        let (reads, counts) = Self::drive(&mut resource, ops)?;
        Self::finish(&resource, reads, counts)
    }

    fn build_then_canonicalise(
        &self,
        resource_type: &str,
        properties: &[(&str, &str)],
    ) -> Result<Vec<u8>, CodecError> {
        let mut resource = frevo_model::Resource::build(resource_type);
        for (path, value) in properties {
            resource
                .set(path, value)
                .map_err(|e| CodecError::Failed(e.to_string()))?;
        }
        resource
            .to_canonical_vec()
            .map_err(|e| CodecError::Failed(e.to_string()))
    }
}

fn must_run(bytes: &[u8], ops: &[Op], what: &str) -> RunResult {
    match codec_under_test().run(bytes, ops) {
        Ok(r) => r,
        Err(e) => panic!("{what}: the run must succeed, but: {e}"),
    }
}

fn must_build(
    resource_type: &str,
    properties: &[(&str, &str)],
    ops: &[Op],
    what: &str,
) -> RunResult {
    match codec_under_test().build(resource_type, properties, ops) {
        Ok(r) => r,
        Err(e) => panic!("{what}: construction must succeed, but: {e}"),
    }
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Assert that nothing at or below `prefix` was materialised.
fn assert_not_materialised(r: &RunResult, prefix: &str, what: &str) {
    let hits = r.materialised_under(prefix);
    assert!(
        hits.is_empty(),
        "{what}: {prefix} must report as not materialised, but these were: {hits:?}"
    );
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A deterministic `Patient` of at least `target` bytes, in generated element
/// order and compact, with `meta` present so a metadata update has something to
/// splice. The same shape must be the fixture committed with the `bench/micro/`
/// workloads, so that the recorded numbers and the in-process ratios describe
/// the same document.
fn patient_of_size(target: usize) -> Vec<u8> {
    let head = concat!(
        r##"{"resourceType":"Patient","id":"perf","##,
        r##""meta":{"versionId":"1","lastUpdated":"2026-01-01T00:00:00.000Z"},"##,
        r##""text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">Perf</div>"},"##,
        r##""active":true,"##,
        r##""name":[{"use":"official","family":"Windsor","given":["Peter","James"]}],"##,
        r##""gender":"male","birthDate":"1974-12-25","##,
        r##""address":["##
    );
    let tail = r##"]}"##;
    let mut s = String::from(head);
    let mut i = 0usize;
    loop {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r##"{{"line":["{i} Example Street","Suite {i}"],"city":"Townsville","district":"Example District","state":"NSW","postalCode":"{:04}","country":"AU"}}"##,
            2000 + i % 8000
        ));
        i += 1;
        if s.len() + tail.len() >= target {
            break;
        }
    }
    s.push_str(tail);
    s.into_bytes()
}

/// The 4 KiB `Patient` of `model.perf.001`, `.002`, `.003` and `.005`, and the
/// document `model.bytes.003` reads one element from.
fn patient_4kib() -> Vec<u8> {
    let b = patient_of_size(4 * 1024);
    assert!(
        (4 * 1024..5 * 1024).contains(&b.len()),
        "the 4 KiB fixture must be about 4 KiB, not {} bytes",
        b.len()
    );
    b
}

/// The 64 KiB `Patient` of `model.perf.004` and `model.mutate.007`.
fn patient_64kib() -> Vec<u8> {
    let b = patient_of_size(64 * 1024);
    assert!(
        (64 * 1024..66 * 1024).contains(&b.len()),
        "the 64 KiB fixture must be about 64 KiB, not {} bytes",
        b.len()
    );
    b
}

/// A `Bundle` of `entries` entries, each holding a distinct `Patient`.
fn bundle_of(entries: usize) -> Vec<u8> {
    let mut s =
        String::from(r##"{"resourceType":"Bundle","id":"b","type":"collection","entry":["##);
    for i in 0..entries {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r##"{{"fullUrl":"urn:uuid:{i}","resource":{{"resourceType":"Patient","id":"p{i}","name":[{{"family":"Family{i}"}}],"gender":"male"}}}}"##
        ));
    }
    s.push_str("]}");
    s.into_bytes()
}

/// A `Patient` with `n` `name` entries, each with a distinct `family`.
fn patient_with_names(n: usize) -> Vec<u8> {
    let mut s = String::from(r##"{"resourceType":"Patient","id":"p","name":["##);
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r##"{{"family":"Family{i}","given":["Given{i}"]}}"##
        ));
    }
    s.push_str(r##"],"gender":"male"}"##);
    s.into_bytes()
}

/// A `Patient` whose `contained` array holds three resources, plus a `name` to
/// read.
const PATIENT_WITH_THREE_CONTAINED: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","contained":["##,
    r##"{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"},"##,
    r##"{"resourceType":"Practitioner","id":"prac1","name":[{"family":"Smith"}]},"##,
    r##"{"resourceType":"Location","id":"loc1","name":"Ward 7"}],"##,
    r##""name":[{"family":"Chalmers"}],"gender":"male"}"##
);

/// A `Patient` with a `text.div` of at least 50 KiB and a `gender` to read.
fn patient_with_large_narrative() -> Vec<u8> {
    let mut div = String::from("<div xmlns=\\\"http://www.w3.org/1999/xhtml\\\">");
    while div.len() < 50 * 1024 {
        div.push_str("<p>A paragraph of narrative text that nobody asked for.</p>");
    }
    div.push_str("</div>");
    format!(
        r##"{{"resourceType":"Patient","id":"p","text":{{"status":"generated","div":"{div}"}},"gender":"male","birthDate":"1974-12-25"}}"##
    )
    .into_bytes()
}

/// A singleton primitive with a `_field` sibling, for `model.bytes.006`: the
/// constructed and the byte-backed form must read the same.
const SMALL_PATIENT: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","active":true,"##,
    r##""gender":"male","birthDate":"1974-12-25"}"##
);

/// The properties of [`SMALL_PATIENT`], deliberately **not** in the generated
/// element order: `id` precedes `active`, which precedes `gender`, which
/// precedes `birthDate`.
const SMALL_PATIENT_PROPERTIES: &[(&str, &str)] = &[
    ("Patient.birthDate", "1974-12-25"),
    ("Patient.gender", "male"),
    ("Patient.active", "true"),
    ("Patient.id", "p"),
];

// ---------------------------------------------------------------------------
// model.bytes.001 .. model.bytes.006
// ---------------------------------------------------------------------------

// covers: model.bytes.001
#[test]
fn stored_bytes_are_returned_verbatim() {
    let input = patient_4kib();
    let r = must_run(&input, &[], "a pass-through with nothing read or changed");
    assert_eq!(
        as_text(&r.output),
        as_text(&input),
        "the output byte sequence must be identical to the input byte sequence"
    );
    assert!(
        !r.modified,
        "and the resource must not report itself as modified"
    );
}

// covers: model.bytes.002
#[test]
fn a_pass_through_performs_no_json_parse() {
    let input = patient_4kib();
    let r = must_run(&input, &[], "a pass-through");
    assert_eq!(
        r.tokens_scanned, 0,
        "the number of JSON tokens scanned must be zero, as reported by the \
         codec's instrumentation counter"
    );
    assert_eq!(
        r.size_proportional_allocations, 0,
        "and the number of heap allocations proportional to the document size \
         must be zero -- an Arc clone is not proportional to the document"
    );
    assert!(
        r.materialised.is_empty(),
        "nothing is materialised either: {:?}",
        r.materialised
    );
}

// covers: model.bytes.003
#[test]
fn reading_one_element_does_not_force_a_full_parse() {
    let input = patient_4kib();

    let lazy = must_run(
        &input,
        &[Op::Read("Patient.birthDate")],
        "reading Patient.birthDate from a 4 KiB Patient",
    );
    assert_eq!(
        lazy.read("Patient.birthDate").value.as_deref(),
        Some("1974-12-25"),
        "the value is returned"
    );

    let full = must_run(&input, &[Op::MaterialiseAll], "a full parse");
    assert!(
        full.tokens_scanned > 0,
        "a full parse must scan tokens, or the comparison is vacuous"
    );
    assert!(
        lazy.tokens_scanned < full.tokens_scanned,
        "the token counter must report fewer tokens scanned ({}) than a full \
         parse of the same document reports ({})",
        lazy.tokens_scanned,
        full.tokens_scanned
    );
}

// covers: model.bytes.004
#[test]
fn reading_an_element_does_not_disturb_pass_through() {
    let input = patient_4kib();
    let r = must_run(
        &input,
        &[Op::Read("Patient.birthDate")],
        "reading Patient.birthDate and then serialising with nothing changed",
    );
    assert_eq!(
        r.read("Patient.birthDate").value.as_deref(),
        Some("1974-12-25")
    );
    assert_eq!(
        as_text(&r.output),
        as_text(&input),
        "the output byte sequence must still be identical to the input byte \
         sequence: a read is not a mutation"
    );
    assert!(!r.modified, "and the resource is not modified by a read");
}

// covers: model.bytes.005
#[test]
fn a_resource_that_was_never_bytes_serialises_from_its_element_order() {
    // Properties are supplied birthDate, gender, active, id -- the reverse of
    // the generated order -- so only consulting the element-order table of the
    // StructureDefinition snapshot produces the expectation. This is the same
    // table `codegen.roundtrip.004` grades.
    let r = must_build(
        "Patient",
        SMALL_PATIENT_PROPERTIES,
        &[],
        "a Patient constructed in memory",
    );
    assert_eq!(
        as_text(&r.output),
        SMALL_PATIENT,
        "serialising a constructed resource must emit properties in the \
         generated element order: resourceType, id, meta, implicitRules, \
         language, text, contained, extension, modifierExtension, identifier, \
         active, name, telecom, gender, birthDate, ..."
    );
    assert_eq!(
        r.tokens_scanned, 0,
        "a resource that was never bytes has no JSON to scan"
    );
}

// covers: model.bytes.006
#[test]
fn the_byte_backed_and_constructed_forms_are_indistinguishable_to_a_reader() {
    let reads = [
        Op::Read("Patient.birthDate"),
        Op::Read("Patient.gender"),
        Op::Read("Patient.active"),
    ];

    let decoded = must_run(
        SMALL_PATIENT.as_bytes(),
        &reads,
        "the same resource obtained by decoding bytes",
    );
    let built = must_build(
        "Patient",
        SMALL_PATIENT_PROPERTIES,
        &reads,
        "the same resource obtained by construction",
    );

    for path in ["Patient.birthDate", "Patient.gender", "Patient.active"] {
        let a = decoded.read(path);
        let b = built.read(path);
        assert_eq!(
            a.value, b.value,
            "{path}: both reads must return equal values"
        );
        assert_eq!(
            a.type_name, b.type_name,
            "{path}: with the same type -- {:?} against {:?}",
            a.type_name, b.type_name
        );
        assert!(
            a.value.is_some(),
            "{path}: and the value must actually be there, or the comparison is \
             vacuous"
        );
    }
}

// ---------------------------------------------------------------------------
// model.lazy.001 .. model.lazy.006
// ---------------------------------------------------------------------------

// covers: model.lazy.001
#[test]
fn an_unread_sibling_is_never_materialised() {
    let input = bundle_of(100);
    let r = must_run(
        &input,
        &[Op::Read("Bundle.entry[0].resource.id")],
        "reading Bundle.entry[0].resource.id from a 100-entry Bundle",
    );
    assert_eq!(
        r.read("Bundle.entry[0].resource.id").value.as_deref(),
        Some("p0"),
        "the value is returned"
    );
    for i in 1..100 {
        assert_not_materialised(&r, &format!("Bundle.entry[{i}]"), "a 100-entry Bundle");
    }
}

// covers: model.lazy.002
#[test]
fn a_repeating_element_can_be_indexed_without_materialising_the_rest() {
    let input = patient_with_names(10);
    let r = must_run(
        &input,
        &[Op::Read("Patient.name[2].family")],
        "reading Patient.name[2].family from a document with ten names",
    );
    assert_eq!(
        r.read("Patient.name[2].family").value.as_deref(),
        Some("Family2"),
        "the value is returned"
    );
    for i in (0..10).filter(|i| *i != 2) {
        assert_not_materialised(&r, &format!("Patient.name[{i}]"), "ten name entries");
    }
}

// covers: model.lazy.003
#[test]
fn a_count_does_not_materialise_the_values() {
    let input = patient_with_names(10);
    let r = must_run(
        &input,
        &[Op::Count("Patient.name")],
        "counting Patient.name",
    );
    assert_eq!(r.counts, [10], "the count is correct");
    for i in 0..10 {
        assert_not_materialised(
            &r,
            &format!("Patient.name[{i}]"),
            "counting without materialising",
        );
    }
    assert!(
        r.materialised
            .iter()
            .all(|p| !p.starts_with("Patient.name")),
        "no HumanName is materialised: {:?}",
        r.materialised
    );
}

// covers: model.lazy.004
#[test]
fn materialisation_is_idempotent_and_cached() {
    let input = patient_4kib();
    let r = must_run(
        &input,
        &[Op::Read("Patient.birthDate"), Op::Read("Patient.birthDate")],
        "reading Patient.birthDate twice",
    );
    assert_eq!(r.reads.len(), 2, "two reads were asked for");
    assert_eq!(
        r.reads[0].value, r.reads[1].value,
        "both reads must return equal values"
    );
    assert_eq!(
        r.reads[0].value.as_deref(),
        Some("1974-12-25"),
        "and the value must be the real one"
    );
    assert!(
        r.reads[0].tokens_scanned_so_far > 0,
        "the first read scans something, or there is nothing to cache"
    );
    assert_eq!(
        r.reads[1].tokens_scanned_so_far, r.reads[0].tokens_scanned_so_far,
        "the second read must scan zero additional JSON tokens"
    );
}

// covers: model.lazy.005
#[test]
fn a_contained_resource_is_materialised_only_when_entered() {
    let r = must_run(
        PATIENT_WITH_THREE_CONTAINED.as_bytes(),
        &[Op::Read("Patient.name[0].family")],
        "reading Patient.name.family with three contained resources present",
    );
    assert_eq!(
        r.read("Patient.name[0].family").value.as_deref(),
        Some("Chalmers")
    );
    for i in 0..3 {
        assert_not_materialised(
            &r,
            &format!("Patient.contained[{i}]"),
            "three contained resources",
        );
    }
}

// covers: model.lazy.006
#[test]
fn narrative_is_not_materialised_by_an_unrelated_read() {
    let input = patient_with_large_narrative();
    assert!(
        input.len() > 50 * 1024,
        "the fixture must carry a text.div of at least 50 KiB, not {} bytes in \
         total",
        input.len()
    );
    let r = must_run(
        &input,
        &[Op::Read("Patient.gender")],
        "reading Patient.gender with a 50 KiB text.div present",
    );
    assert_eq!(r.read("Patient.gender").value.as_deref(), Some("male"));
    assert_not_materialised(&r, "Patient.text", "a 50 KiB narrative");
    assert!(
        !r.xhtml_validated,
        "and the XHTML must not be validated by an unrelated read"
    );
}

// ---------------------------------------------------------------------------
// The recorded half of model.perf.*
// ---------------------------------------------------------------------------

/// Where the `bench/micro/` workloads record what they measured. The shape
/// mirrors `bench/baseline-hapi-8.12.json`:
///
/// ```json
/// {
///   "profile": "release",
///   "commit": "<the commit measured>",
///   "workloads": [
///     { "name": "model-passthrough", "median_us": 0.84, "tokens_scanned": 0 },
///     { "name": "model-lazy-read", "median_us": 1.6 },
///     { "name": "model-full-parse", "median_us": 17.2 },
///     { "name": "model-meta-update-4kib", "median_us": 2.1 },
///     { "name": "model-meta-update-64kib", "median_us": 2.4 },
///     { "name": "model-canonical-encode", "median_us": 21.0 }
///   ]
/// }
/// ```
const MICRO_RESULTS: &str = "bench/micro/results.json";

/// The same shape, recorded for the previous commit on `main`.
/// `model.perf.006` compares the two.
const MICRO_BASELINE: &str = "bench/micro/baseline.json";

/// Every model workload named in `design.md` section 8, with the budget the
/// scenario states for it.
const MODEL_WORKLOADS: &[&str] = &[
    "model-passthrough",
    "model-lazy-read",
    "model-full-parse",
    "model-meta-update-4kib",
    "model-meta-update-64kib",
    "model-canonical-encode",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A number recorded for a workload, or the reason it could not be read. The
/// reason is returned rather than panicked so the caller can say which scenario
/// is missing its measurement.
fn recorded(file: &str, workload: &str, key: &str) -> Result<f64, String> {
    let path = repo_root().join(file);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{file} does not exist ({e}). The {workload} workload under \
             bench/micro/ must record its median there; until it does, this \
             budget is unmeasured."
        )
    })?;
    let doc = json_parse(&text).map_err(|e| format!("{file} is not JSON: {e}"))?;
    let workloads =
        json_get(&doc, "workloads").ok_or_else(|| format!("{file} has no \"workloads\" member"))?;
    let entries = match workloads {
        Json::Array(items) => items,
        other => {
            return Err(format!(
                "{file}: \"workloads\" must be an array, got {other:?}"
            ));
        }
    };
    let entry = entries
        .iter()
        .find(|e| matches!(json_get(e, "name"), Some(Json::Str(n)) if n == workload))
        .ok_or_else(|| {
            format!(
                "{file} records no workload named {workload:?}; it has {:?}",
                entries
                    .iter()
                    .filter_map(|e| match json_get(e, "name") {
                        Some(Json::Str(n)) => Some(n.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            )
        })?;
    match json_get(entry, key) {
        Some(Json::Number(n)) => n
            .parse::<f64>()
            .map_err(|_| format!("{file}: {workload}.{key} is not a number: {n}")),
        Some(other) => Err(format!(
            "{file}: {workload}.{key} is not a number: {other:?}"
        )),
        None => Err(format!("{file}: {workload} records no {key:?}")),
    }
}

/// Require a recorded number, naming the scenario that needs it.
fn require_recorded(file: &str, workload: &str, key: &str, scenario: &str) -> f64 {
    recorded(file, workload, key)
        .unwrap_or_else(|why| panic!("{scenario} is graded on a measured median: {why}"))
}

// ---------------------------------------------------------------------------
// A minimal JSON reader for the recorded results
//
// frevo-model has no dependencies and this change adds none, so the results
// file is read with the smallest parser that can do it. Numbers are kept as
// their lexical text.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

fn json_get<'a>(v: &'a Json, key: &str) -> Option<&'a Json> {
    match v {
        Json::Object(props) => props.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn json_parse(text: &str) -> Result<Json, String> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let v = json_value(bytes, &mut at)?;
    json_ws(bytes, &mut at);
    if at != bytes.len() {
        return Err(format!("trailing bytes at offset {at}"));
    }
    Ok(v)
}

fn json_ws(b: &[u8], at: &mut usize) {
    while matches!(b.get(*at), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        *at += 1;
    }
}

fn json_value(b: &[u8], at: &mut usize) -> Result<Json, String> {
    json_ws(b, at);
    match b.get(*at) {
        Some(b'{') => {
            *at += 1;
            let mut props = Vec::new();
            json_ws(b, at);
            if b.get(*at) == Some(&b'}') {
                *at += 1;
                return Ok(Json::Object(props));
            }
            loop {
                json_ws(b, at);
                let key = json_string(b, at)?;
                json_ws(b, at);
                if b.get(*at) != Some(&b':') {
                    return Err(format!("expected : at offset {at}"));
                }
                *at += 1;
                props.push((key, json_value(b, at)?));
                json_ws(b, at);
                match b.get(*at) {
                    Some(b',') => *at += 1,
                    Some(b'}') => {
                        *at += 1;
                        return Ok(Json::Object(props));
                    }
                    _ => return Err(format!("expected , or }} at offset {at}")),
                }
            }
        }
        Some(b'[') => {
            *at += 1;
            let mut items = Vec::new();
            json_ws(b, at);
            if b.get(*at) == Some(&b']') {
                *at += 1;
                return Ok(Json::Array(items));
            }
            loop {
                items.push(json_value(b, at)?);
                json_ws(b, at);
                match b.get(*at) {
                    Some(b',') => *at += 1,
                    Some(b']') => {
                        *at += 1;
                        return Ok(Json::Array(items));
                    }
                    _ => return Err(format!("expected , or ] at offset {at}")),
                }
            }
        }
        Some(b'"') => json_string(b, at).map(Json::Str),
        Some(b't') => json_literal(b, at, "true").map(|()| Json::Bool(true)),
        Some(b'f') => json_literal(b, at, "false").map(|()| Json::Bool(false)),
        Some(b'n') => json_literal(b, at, "null").map(|()| Json::Null),
        Some(_) => {
            let start = *at;
            while matches!(b.get(*at), Some(c)
                if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E'))
            {
                *at += 1;
            }
            if *at == start {
                return Err(format!("expected a value at offset {at}"));
            }
            Ok(Json::Number(
                String::from_utf8_lossy(&b[start..*at]).into_owned(),
            ))
        }
        None => Err("unexpected end of input".to_string()),
    }
}

fn json_literal(b: &[u8], at: &mut usize, text: &str) -> Result<(), String> {
    if b[*at..].starts_with(text.as_bytes()) {
        *at += text.len();
        Ok(())
    } else {
        Err(format!("expected {text} at offset {at}"))
    }
}

fn json_string(b: &[u8], at: &mut usize) -> Result<String, String> {
    if b.get(*at) != Some(&b'"') {
        return Err(format!("expected a string at offset {at}"));
    }
    *at += 1;
    let mut out = String::new();
    loop {
        match b.get(*at) {
            None => return Err("unterminated string".to_string()),
            Some(b'"') => {
                *at += 1;
                return Ok(out);
            }
            Some(b'\\') => {
                *at += 1;
                let esc = *b.get(*at).ok_or("bad escape")?;
                *at += 1;
                out.push(match esc {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    other => other as char,
                });
            }
            Some(_) => {
                out.push(b[*at] as char);
                *at += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The in-process half of model.perf.*
// ---------------------------------------------------------------------------

const WARMUP: usize = 20;
const ITERATIONS: usize = 200;

/// The median wall-clock of `ITERATIONS` runs of `f`, after `WARMUP` discarded
/// runs. The result of `f` goes through `black_box` so the work cannot be
/// elided.
fn median_of<T>(mut f: impl FnMut() -> T) -> Duration {
    for _ in 0..WARMUP {
        black_box(f());
    }
    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        let out = f();
        let elapsed = started.elapsed();
        black_box(out);
        samples.push(elapsed);
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn micros(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}

// covers: model.perf.001
#[test]
fn pass_through_is_a_byte_copy() {
    let input = patient_4kib();

    // The in-process half: no JSON token is scanned, and the output is the
    // input. Build-profile independent, so asserted unconditionally.
    let once = must_run(&input, &[], "the model-passthrough workload");
    assert_eq!(
        once.tokens_scanned, 0,
        "no JSON token is scanned by a pass-through"
    );
    assert_eq!(as_text(&once.output), as_text(&input));
    let median = median_of(|| must_run(&input, &[], "the model-passthrough workload"));
    assert!(
        micros(median) > 0.0,
        "the measurement must actually measure something"
    );

    // The recorded half: the budget as design.md section 8 states it, graded
    // against what the bench/micro/ workload measured on the reference
    // environment.
    let recorded_median = require_recorded(
        MICRO_RESULTS,
        "model-passthrough",
        "median_us",
        "model.perf.001",
    );
    assert!(
        recorded_median <= 1.0,
        "model-passthrough: the median time must be at most 1 us, but \
         {MICRO_RESULTS} records {recorded_median} us"
    );
    let recorded_tokens = require_recorded(
        MICRO_RESULTS,
        "model-passthrough",
        "tokens_scanned",
        "model.perf.001",
    );
    assert_eq!(
        recorded_tokens, 0.0,
        "and the recorded run must report zero tokens scanned"
    );
}

// covers: model.perf.002
#[test]
fn a_single_element_read_is_far_cheaper_than_a_full_parse() {
    let input = patient_4kib();

    // The in-process half: the ratio, which survives the change of build
    // profile that the absolute budget does not.
    let lazy = median_of(|| {
        must_run(
            &input,
            &[Op::Read("Patient.birthDate")],
            "the model-lazy-read workload",
        )
    });
    let full = median_of(|| {
        must_run(
            &input,
            &[Op::MaterialiseAll],
            "the model-full-parse workload",
        )
    });
    assert!(
        micros(full) > 0.0,
        "the full parse must take measurable time, or the ratio is meaningless"
    );
    assert!(
        micros(lazy) <= micros(full) / 10.0,
        "model-lazy-read must be at most one tenth of the median time of \
         model-full-parse: {:.3} us against {:.3} us",
        micros(lazy),
        micros(full)
    );

    // The recorded half: the absolute budget, and the same ratio over the
    // recorded medians.
    let lazy_us = require_recorded(
        MICRO_RESULTS,
        "model-lazy-read",
        "median_us",
        "model.perf.002",
    );
    let full_us = require_recorded(
        MICRO_RESULTS,
        "model-full-parse",
        "median_us",
        "model.perf.002",
    );
    assert!(
        lazy_us <= 2.0,
        "model-lazy-read: the median must be at most 2 us, but \
         {MICRO_RESULTS} records {lazy_us} us"
    );
    assert!(
        lazy_us <= full_us / 10.0,
        "and at most one tenth of model-full-parse: {lazy_us} us against \
         {full_us} us"
    );
}

// covers: model.perf.003
#[test]
fn full_materialisation_stays_within_budget() {
    let input_4k = patient_4kib();
    let input_64k = patient_64kib();

    // The in-process half: a full parse of the 4 KiB document materialises it
    // completely, and the token count scales with the document, so
    // "model-full-parse decodes a 4 KiB Patient completely" is a claim about
    // work actually done and not a no-op being timed.
    let small = must_run(
        &input_4k,
        &[Op::MaterialiseAll],
        "the model-full-parse workload",
    );
    let large = must_run(
        &input_64k,
        &[Op::MaterialiseAll],
        "a full parse of the 64 KiB document",
    );
    assert!(
        small.tokens_scanned > 0,
        "a full parse of 4 KiB must scan tokens"
    );
    assert!(
        large.tokens_scanned > small.tokens_scanned,
        "and a full parse of 64 KiB must scan more: {} against {}",
        large.tokens_scanned,
        small.tokens_scanned
    );
    assert!(
        small
            .materialised
            .iter()
            .any(|p| p.starts_with("Patient.address")),
        "a complete decode materialises the repeating elements too: {:?}",
        small.materialised
    );

    // The recorded half.
    let median = require_recorded(
        MICRO_RESULTS,
        "model-full-parse",
        "median_us",
        "model.perf.003",
    );
    assert!(
        median <= 20.0,
        "model-full-parse: the median time must be at most 20 us, but \
         {MICRO_RESULTS} records {median} us"
    );
}

// covers: model.perf.004
#[test]
fn a_metadata_update_is_proportional_to_meta_not_to_the_document() {
    let meta_update = [
        Op::Set("Patient.meta.versionId", "2"),
        Op::Set("Patient.meta.lastUpdated", "2026-11-08T12:00:00.000Z"),
    ];
    let input_4k = patient_4kib();
    let input_64k = patient_64kib();

    // The in-process half: the 4 KiB and 64 KiB medians within 25% of each
    // other. This is a ratio between two measurements taken the same way, so it
    // holds whatever the build profile.
    let small = median_of(|| {
        must_run(
            &input_4k,
            &meta_update,
            "the model-meta-update workload at 4 KiB",
        )
    });
    let large = median_of(|| {
        must_run(
            &input_64k,
            &meta_update,
            "the model-meta-update workload at 64 KiB",
        )
    });
    assert!(
        micros(small) > 0.0 && micros(large) > 0.0,
        "both cases must take measurable time"
    );
    let spread = (micros(large) - micros(small)).abs() / micros(small).min(micros(large));
    assert!(
        spread <= 0.25,
        "the median times must differ by at most 25%, but 4 KiB took {:.3} us \
         and 64 KiB took {:.3} us -- a spread of {:.1}%. A metadata update is \
         proportional to meta, not to the document.",
        micros(small),
        micros(large),
        spread * 100.0
    );

    // The recorded half, over the two recorded medians.
    let small_us = require_recorded(
        MICRO_RESULTS,
        "model-meta-update-4kib",
        "median_us",
        "model.perf.004",
    );
    let large_us = require_recorded(
        MICRO_RESULTS,
        "model-meta-update-64kib",
        "median_us",
        "model.perf.004",
    );
    let recorded_spread = (large_us - small_us).abs() / small_us.min(large_us);
    assert!(
        recorded_spread <= 0.25,
        "the recorded medians must differ by at most 25%: {small_us} us at \
         4 KiB against {large_us} us at 64 KiB"
    );
}

// covers: model.perf.005
#[test]
fn canonical_serialisation_of_a_constructed_resource_stays_within_budget() {
    // A constructed 4 KiB Patient: the same address repetitions the decoded
    // fixture carries, supplied as properties so nothing is ever bytes.
    let mut properties: Vec<(String, String)> = vec![
        ("Patient.id".to_string(), "perf".to_string()),
        ("Patient.active".to_string(), "true".to_string()),
        ("Patient.gender".to_string(), "male".to_string()),
        ("Patient.birthDate".to_string(), "1974-12-25".to_string()),
    ];
    let mut i = 0usize;
    let mut approximate = 100usize;
    while approximate < 4 * 1024 {
        for (element, value) in [
            ("city", "Townsville".to_string()),
            ("district", "Example District".to_string()),
            ("state", "NSW".to_string()),
            ("country", "AU".to_string()),
            ("postalCode", format!("{:04}", 2000 + i % 8000)),
        ] {
            let path = format!("Patient.address[{i}].{element}");
            approximate += path.len() + value.len() + 8;
            properties.push((path, value));
        }
        i += 1;
    }
    let borrowed: Vec<(&str, &str)> = properties
        .iter()
        .map(|(p, v)| (p.as_str(), v.as_str()))
        .collect();

    // The in-process half: the construction really does produce a document of
    // about 4 KiB, canonically, so the recorded median describes the workload
    // the scenario names.
    let codec = codec_under_test();
    let out = match codec.build_then_canonicalise("Patient", &borrowed) {
        Ok(v) => v,
        Err(e) => panic!("the model-canonical-encode workload must succeed, but: {e}"),
    };
    assert!(
        out.len() >= 4 * 1024,
        "the constructed Patient must canonicalise to at least 4 KiB, not {} \
         bytes",
        out.len()
    );
    let median = median_of(|| {
        codec
            .build_then_canonicalise("Patient", &borrowed)
            .unwrap_or_else(|e| panic!("the model-canonical-encode workload must succeed: {e}"))
    });
    assert!(
        micros(median) > 0.0,
        "the measurement must actually measure something"
    );

    // The recorded half.
    let recorded_median = require_recorded(
        MICRO_RESULTS,
        "model-canonical-encode",
        "median_us",
        "model.perf.005",
    );
    assert!(
        recorded_median <= 25.0,
        "model-canonical-encode: the median time must be at most 25 us, but \
         {MICRO_RESULTS} records {recorded_median} us"
    );
}

// covers: model.perf.006
#[test]
fn no_regression_against_the_recorded_baseline() {
    // The regression policy of bench/targets.md, applied unchanged: a
    // regression of more than 10% on any workload fails the performance gate.
    // The baseline is the one recorded for the previous commit on main.
    let mut regressions = Vec::new();
    for workload in MODEL_WORKLOADS {
        let now = require_recorded(MICRO_RESULTS, workload, "median_us", "model.perf.006");
        let baseline = require_recorded(MICRO_BASELINE, workload, "median_us", "model.perf.006");
        assert!(
            baseline > 0.0,
            "{workload}: a baseline median of {baseline} us cannot be compared \
             against"
        );
        let ratio = now / baseline;
        if ratio > 1.10 {
            regressions.push(format!(
                "{workload}: {now} us against a baseline of {baseline} us, \
                 {:.1}% slower",
                (ratio - 1.0) * 100.0
            ));
        }
    }
    assert!(
        regressions.is_empty(),
        "no bench/micro/ model workload may be more than 10% slower than the \
         baseline recorded for the previous commit on main, per the regression \
         policy in bench/targets.md: {regressions:?}"
    );
}
