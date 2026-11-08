//! The `model-mutate` in-process oracle: copy-on-write mutation with byte
//! splicing.
//!
//! Task 5 of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo test -p frevo-model --test model_mutate
//! ```
//!
//! Scenario coverage: `model.mutate.001` .. `model.mutate.007`.
//!
//! ## What is actually being asserted
//!
//! "A change to one element does not cost a re-serialisation of the whole
//! document, and does not change the bytes of any subtree that was not
//! touched." A test that only compared the output against an expected document
//! would pass for a codec that re-encoded everything and happened to produce
//! the same bytes -- which is the implementation the requirement exists to
//! rule out. So the seam reports the **provenance** of every byte of the
//! output: which runs came verbatim from the input buffer, at which offset, and
//! which ranges were newly encoded.
//!
//! [`assert_spliced_verbatim`] is where that is used: it locates a subtree in
//! the input, finds the splice that covers it, and checks the output bytes at
//! the corresponding offset are the same bytes. A codec that re-encoded the
//! subtree to an identical spelling would have no splice covering it, and would
//! fail.
//!
//! `design.md` section 3, "Mutation and splicing": `Stored` starts
//! `dirty: false`; a write replaces the written subtree with an owned value and
//! records the span it replaced; serialisation walks the original bytes and, at
//! each recorded span, emits the new encoding instead. Everything else is
//! `copy_from_slice`.
//!
//! ## The seam
//!
//! The unbound binding returns [`CodecError::NotImplemented`] from every
//! surface, and every helper turns that into an explicit "nothing is bound yet"
//! failure rather than letting it read as a pass.

use std::fmt;
use std::ops::Range;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

/// One mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Set one element's primitive value to the given lexical form.
    Set(&'static str, &'static str),
    /// Append one repetition to a repeating element, with the given
    /// sub-element values. Paths are relative to the new repetition.
    Append(&'static str, &'static [(&'static str, &'static str)]),
    /// Remove one repetition, by its indexed path.
    Remove(&'static str),
}

/// A run of input bytes copied verbatim into the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    /// The range in the **input** buffer.
    pub input: Range<usize>,
    /// Where that run begins in the **output**.
    pub output_at: usize,
}

impl Splice {
    pub fn output(&self) -> Range<usize> {
        self.output_at..self.output_at + self.input.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutateResult {
    pub output: Vec<u8>,
    /// Whether the resource reports itself as modified.
    pub modified: bool,
    /// Every verbatim run, in output order. Together with `newly_encoded` this
    /// must account for the whole output.
    pub spliced: Vec<Splice>,
    /// The ranges of the output that were newly encoded rather than spliced.
    pub newly_encoded: Vec<Range<usize>>,
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

const NOT_BOUND: &str = "no codec is bound to the model-mutate oracle yet \
     (crates/frevo-model/tests/model_mutate.rs::codec_under_test)";

pub trait MutateOracle {
    /// Load from stored bytes, apply the mutations, serialise, and report where
    /// every byte of the output came from.
    fn mutate(&self, bytes: &[u8], ops: &[Op]) -> Result<MutateResult, CodecError>;

    /// Decode both inputs and report whether every element of the two
    /// resources is equal.
    fn decodes_equal(&self, a: &[u8], b: &[u8]) -> Result<bool, CodecError>;
}

/// The codec this oracle grades. Phase C replaces this body with the real one.
fn codec_under_test() -> Box<dyn MutateOracle> {
    Box::new(UnboundCodec)
}

struct UnboundCodec;

impl MutateOracle for UnboundCodec {
    fn mutate(&self, _bytes: &[u8], _ops: &[Op]) -> Result<MutateResult, CodecError> {
        Err(CodecError::NotImplemented)
    }

    fn decodes_equal(&self, _a: &[u8], _b: &[u8]) -> Result<bool, CodecError> {
        Err(CodecError::NotImplemented)
    }
}

fn must_mutate(bytes: &[u8], ops: &[Op], what: &str) -> MutateResult {
    match codec_under_test().mutate(bytes, ops) {
        Ok(r) => r,
        Err(e) => panic!("{what}: the mutation must succeed, but: {e}"),
    }
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The byte range of `needle` in `haystack`, which must occur exactly once.
fn sole_range(haystack: &[u8], needle: &str) -> Range<usize> {
    let hay = std::str::from_utf8(haystack).expect("the fixture is UTF-8");
    let first = hay
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} does not occur in:\n{hay}"));
    assert!(
        !hay[first + needle.len()..].contains(needle),
        "{needle:?} occurs more than once; the assertion would be ambiguous"
    );
    first..first + needle.len()
}

/// Assert that the input bytes encoding `needle` were spliced into the output
/// verbatim -- not re-encoded to the same spelling.
fn assert_spliced_verbatim(input: &[u8], r: &MutateResult, needle: &str, what: &str) {
    let range = sole_range(input, needle);
    let splice = r
        .spliced
        .iter()
        .find(|s| s.input.start <= range.start && s.input.end >= range.end)
        .unwrap_or_else(|| {
            panic!(
                "{what}: the input byte range {range:?} encoding {needle:?} must \
                 appear unchanged in the output, but no verbatim splice covers \
                 it. The splices were {:?}",
                r.spliced
            )
        });
    let offset = splice.output_at + (range.start - splice.input.start);
    let out = r
        .output
        .get(offset..offset + range.len())
        .unwrap_or_else(|| {
            panic!(
                "{what}: the splice claims output offset {offset} for {needle:?}, \
                 but the output is only {} bytes",
                r.output.len()
            )
        });
    assert_eq!(
        as_text(out),
        needle,
        "{what}: the spliced bytes at output offset {offset} must be the input \
         bytes for {needle:?}"
    );
}

/// Assert that `needle` appears in the output inside a newly encoded range.
fn assert_newly_encoded(r: &MutateResult, needle: &str, what: &str) {
    let range = sole_range(&r.output, needle);
    assert!(
        r.newly_encoded
            .iter()
            .any(|n| n.start <= range.start && n.end >= range.end),
        "{what}: {needle:?} must be newly encoded, but the newly encoded ranges \
         are {:?} and it sits at {range:?}",
        r.newly_encoded
    );
}

/// The ranges of the input that no splice copied into the output.
fn unspliced_input_ranges(r: &MutateResult, input_len: usize) -> Vec<Range<usize>> {
    let mut covered: Vec<Range<usize>> = r.spliced.iter().map(|s| s.input.clone()).collect();
    covered.sort_by_key(|c| c.start);
    let mut gaps = Vec::new();
    let mut at = 0usize;
    for c in covered {
        if c.start > at {
            gaps.push(at..c.start);
        }
        at = at.max(c.end);
    }
    if at < input_len {
        gaps.push(at..input_len);
    }
    gaps
}

/// `bench/micro/` must declare the named workload. `model.mutate.007` states
/// that the metadata update "is measured by the `model-meta-update` workload in
/// `bench/micro/`", so the existence of that workload is part of the scenario.
fn assert_workload_declared(workload: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/micro");
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "{} does not exist ({e}), so the {workload} workload that measures \
             this operation is not declared anywhere",
            dir.display()
        )
    });
    let mut declared_in = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path)
            && text.contains(workload)
        {
            declared_in.push(path);
        }
    }
    assert!(
        !declared_in.is_empty(),
        "no file under {} names the {workload} workload",
        dir.display()
    );
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A `Patient` in generated element order with a narrative, two contained
/// resources and three names, so each of `model.mutate.001`'s three subtrees is
/// a distinct, locatable byte range.
const PATIENT: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">Patient</div>"},"##,
    r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"},"##,
    r##"{"resourceType":"Location","id":"loc1","name":"Ward 7"}],"##,
    r##""active":true,"##,
    r##""name":[{"family":"Alpha"},{"family":"Bravo"},{"family":"Charlie"}],"##,
    r##""gender":"male"}"##
);

const TEXT_SUBTREE: &str = concat!(
    r##""text":{"status":"generated","##,
    r##""div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">Patient</div>"}"##
);

const CONTAINED_SUBTREE: &str = concat!(
    r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"},"##,
    r##"{"resourceType":"Location","id":"loc1","name":"Ward 7"}]"##
);

const NAME_SUBTREE: &str =
    r##""name":[{"family":"Alpha"},{"family":"Bravo"},{"family":"Charlie"}]"##;

const CONTAINED_ZERO: &str =
    r##"{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"}"##;

const CONTAINED_ONE: &str = r##"{"resourceType":"Location","id":"loc1","name":"Ward 7"}"##;

const NAME_ZERO: &str = r##"{"family":"Alpha"}"##;
const NAME_ONE: &str = r##"{"family":"Bravo"}"##;
const NAME_TWO: &str = r##"{"family":"Charlie"}"##;

const APPENDED_NAME: &[(&str, &str)] = &[("family", "Delta")];

/// The `meta` subtree of [`patient_of_size`], which is the only part
/// `model.mutate.007` permits to be re-encoded.
const META_SUBTREE: &str = r##""meta":{"versionId":"1","lastUpdated":"2026-01-01T00:00:00.000Z"}"##;

/// A deterministic `Patient` of at least `target` bytes, in generated element
/// order and compact, with `meta` first among the `Resource` elements after
/// `id`. The same shape as the fixture in `model_bytes_lazy_perf.rs`, so the
/// splicing claim and the timing claim describe the same document.
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

fn patient_64kib() -> Vec<u8> {
    let b = patient_of_size(64 * 1024);
    assert!(
        (64 * 1024..66 * 1024).contains(&b.len()),
        "the 64 KiB fixture must be about 64 KiB, not {} bytes",
        b.len()
    );
    b
}

// ---------------------------------------------------------------------------
// model.mutate.001 .. model.mutate.007
// ---------------------------------------------------------------------------

// covers: model.mutate.001
#[test]
fn changing_one_element_preserves_the_bytes_of_untouched_subtrees() {
    let input = PATIENT.as_bytes();
    let what = "setting Patient.active to false";
    let r = must_mutate(input, &[Op::Set("Patient.active", "false")], what);

    assert!(
        as_text(&r.output).contains(r##""active":false"##),
        "the change must actually be in the output: {}",
        as_text(&r.output)
    );

    // The three subtrees the scenario names, each asserted as *the input bytes*
    // rather than as equal text, so a full re-encode that happened to agree
    // would not pass.
    for (label, subtree) in [
        ("Patient.name", NAME_SUBTREE),
        ("Patient.contained", CONTAINED_SUBTREE),
        ("Patient.text", TEXT_SUBTREE),
    ] {
        assert_spliced_verbatim(input, &r, subtree, &format!("{what}: {label}"));
    }

    // And only `active` was re-encoded, so the splices account for everything
    // else.
    let gaps = unspliced_input_ranges(&r, input.len());
    let active = sole_range(input, r##""active":true"##);
    for gap in &gaps {
        assert!(
            gap.start >= active.start && gap.end <= active.end,
            "{what}: the only input bytes that may go unspliced are the ones \
             encoding Patient.active ({active:?}), but {gap:?} did too: {:?}",
            as_text(&input[gap.clone()])
        );
    }
}

// covers: model.mutate.002
#[test]
fn adding_a_repeating_element_keeps_the_existing_entries_bytes() {
    let input = PATIENT.as_bytes();
    let what = "appending a fourth Patient.name";
    let r = must_mutate(input, &[Op::Append("Patient.name", APPENDED_NAME)], what);

    for name in [NAME_ZERO, NAME_ONE, NAME_TWO] {
        assert_spliced_verbatim(input, &r, name, what);
    }
    assert_newly_encoded(&r, r##"{"family":"Delta"}"##, what);

    // The fourth entry is last, and there are four of them.
    let out = as_text(&r.output);
    assert!(
        out.contains(
            r##""name":[{"family":"Alpha"},{"family":"Bravo"},{"family":"Charlie"},{"family":"Delta"}]"##
        ),
        "the appended entry must be the fourth: {out}"
    );
    assert!(r.modified);
}

// covers: model.mutate.003
#[test]
fn removing_an_element_removes_only_its_bytes() {
    let input = PATIENT.as_bytes();
    let what = "removing Patient.name[1]";
    let r = must_mutate(input, &[Op::Remove("Patient.name[1]")], what);

    assert_spliced_verbatim(input, &r, NAME_ZERO, what);
    assert_spliced_verbatim(input, &r, NAME_TWO, what);

    let out = as_text(&r.output);
    assert!(
        !out.contains(NAME_ONE),
        "the removed entry must be gone: {out}"
    );
    let alpha = out.find(NAME_ZERO).expect("name[0] is in the output");
    let charlie = out
        .find(NAME_TWO)
        .expect("the former name[2] is in the output");
    assert!(
        alpha < charlie,
        "name[0] and the former name[2] must appear in that order: {out}"
    );
    assert!(
        out.contains(r##""name":[{"family":"Alpha"},{"family":"Charlie"}]"##),
        "and nothing else about the array may change: {out}"
    );
}

// covers: model.mutate.004
#[test]
fn a_mutated_resource_no_longer_claims_byte_identity() {
    let input = PATIENT.as_bytes();
    for (what, op) in [
        ("a changed primitive", Op::Set("Patient.gender", "female")),
        (
            "an appended repetition",
            Op::Append("Patient.name", APPENDED_NAME),
        ),
        ("a removed repetition", Op::Remove("Patient.name[1]")),
    ] {
        let r = must_mutate(input, &[op], what);
        assert!(
            r.modified,
            "{what}: the resource must report itself as modified"
        );
        assert_ne!(
            as_text(&r.output),
            as_text(input),
            "{what}: serialising must not return the original bytes"
        );
    }
}

// covers: model.mutate.005
#[test]
fn setting_an_element_to_the_value_it_already_has_still_marks_the_resource_modified() {
    let input = PATIENT.as_bytes();
    let what = "setting Patient.gender to male, which it already is";
    let r = must_mutate(input, &[Op::Set("Patient.gender", "male")], what);

    assert!(
        r.modified,
        "{what}: the resource must report as modified -- the codec records that \
         a write happened, it does not compare values"
    );

    match codec_under_test().decodes_equal(&r.output, input) {
        Ok(true) => {}
        Ok(false) => panic!(
            "{what}: serialising must produce a document that decodes equal to \
             the original\noriginal: {}\noutput:   {}",
            as_text(input),
            as_text(&r.output)
        ),
        Err(e) => panic!("{what}: comparing the two decodes must succeed, but: {e}"),
    }
}

// covers: model.mutate.006
#[test]
fn a_mutation_inside_a_contained_resource_splices_at_that_resources_boundary() {
    let input = PATIENT.as_bytes();
    let what = "changing an element of Patient.contained[0]";
    let r = must_mutate(
        input,
        &[Op::Set("Patient.contained[0].name", "Globex Healthcare")],
        what,
    );

    // contained[0] is re-encoded.
    assert_newly_encoded(&r, "Globex Healthcare", what);
    assert!(
        !as_text(&r.output).contains("Acme Healthcare"),
        "the old value must be gone: {}",
        as_text(&r.output)
    );
    assert!(
        !r.spliced
            .iter()
            .any(|s| s.input.start <= sole_range(input, CONTAINED_ZERO).start
                && s.input.end >= sole_range(input, CONTAINED_ZERO).end),
        "{what}: contained[0] must not be spliced verbatim, since it is the \
         subtree that changed"
    );

    // contained[1..] and the enclosing Patient are spliced from the original.
    assert_spliced_verbatim(input, &r, CONTAINED_ONE, what);
    assert_spliced_verbatim(input, &r, TEXT_SUBTREE, what);
    assert_spliced_verbatim(input, &r, NAME_SUBTREE, what);

    // Nothing outside contained[0] goes unspliced.
    let boundary = sole_range(input, CONTAINED_ZERO);
    for gap in unspliced_input_ranges(&r, input.len()) {
        assert!(
            gap.start >= boundary.start && gap.end <= boundary.end,
            "{what}: the splice boundary is contained[0] ({boundary:?}), but \
             {gap:?} was re-encoded too: {:?}",
            as_text(&input[gap.clone()])
        );
    }
}

// covers: model.mutate.007
#[test]
fn meta_updates_are_the_common_case_and_are_cheap() {
    let input = patient_64kib();
    let what = "setting meta.versionId and meta.lastUpdated on a 64 KiB resource";
    let r = must_mutate(
        &input,
        &[
            Op::Set("Patient.meta.versionId", "2"),
            Op::Set("Patient.meta.lastUpdated", "2026-11-08T12:00:00.000Z"),
        ],
        what,
    );

    assert!(r.modified);
    let out = as_text(&r.output);
    assert!(
        out.contains(r##""versionId":"2""##) && out.contains("2026-11-08T12:00:00.000Z"),
        "the update must be in the output"
    );

    // Every byte outside `meta` is spliced from the original.
    let meta = sole_range(&input, META_SUBTREE);
    for gap in unspliced_input_ranges(&r, input.len()) {
        assert!(
            gap.start >= meta.start && gap.end <= meta.end,
            "{what}: every byte outside meta ({meta:?}) must be spliced from \
             the original, but {gap:?} was not -- {} bytes of {}",
            gap.len(),
            input.len()
        );
    }

    // Stated positively as well, so a codec that reported no splices at all
    // could not satisfy the loop above by vacuity.
    let spliced_bytes: usize = r.spliced.iter().map(|s| s.input.len()).sum();
    assert!(
        spliced_bytes >= input.len() - META_SUBTREE.len(),
        "{what}: at least {} of the {} input bytes must be spliced, but only {} \
         were",
        input.len() - META_SUBTREE.len(),
        input.len(),
        spliced_bytes
    );

    // And the operation is measured, not merely asserted.
    assert_workload_declared("model-meta-update");
}
