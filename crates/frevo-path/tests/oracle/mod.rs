//! The FHIRPath oracle harness.
//!
//! Task 1 of `add-fhirpath-and-codegen-r4`. It reads the pinned suite, loads
//! the suite's input files, runs every case through a trait object in three
//! stages, compares the outcome on whichever of the two comparison branches the
//! case selects, and reports a per-case status that never confuses a case that
//! did not run with one that passed.
//!
//! Provenance of every grading rule here:
//! `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java`
//! at `73c432b5f43ddbce979978a760ec4aa81e929094`, which is the harness HAPI
//! 8.12.0's own pass rate is measured with.
//!
//! Scenarios: `path.oracle.001`-`010`, `015`, `017` and `path.datetime.004`.
//! The gate itself — Frevo's pass count against HAPI's, and the
//! `hapi-cannot-pass` status — is task 3 and is not wired here.
//!
//! `dead_code` is allowed for this module and nothing else in the crate. It is
//! a harness surface, shared by several test binaries and by the tests tasks 3,
//! 4 and 5 add: an accessor no *current* test binary happens to call is not
//! dead, and silencing the warning here is cheaper than the alternative, which
//! is deleting parts of the oracle's reporting surface to keep the linter quiet.

pub mod compare;
pub mod engine;
pub mod input;
pub mod json;
pub mod run;
pub mod suite;
pub mod value;
pub mod xml;

/// Read the pinned suite from the repository, panicking with a message that
/// names the file when it cannot be read.
///
/// A panic is right here and nowhere else in the harness: if the vendored
/// suite is unreadable there are no cases to grade at all, and reporting 1,191
/// `did-not-run`s would hide the fact that the oracle content itself is gone.
pub fn load_pinned_suite() -> suite::Suite {
    let path = input::repo_path(suite::SUITE_PATH);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the vendored FHIRPath suite {}: {e}. It is committed, \
             so this means the working tree is damaged; `bash oracles/fetch.sh --verify` \
             checks it against its pin.",
            suite::SUITE_PATH
        )
    });
    suite::load(&text).unwrap_or_else(|e| panic!("cannot parse {}: {e}", suite::SUITE_PATH))
}
