//! Code generator: FHIR StructureDefinitions to Rust models, codecs and search-parameter tables.
//!
//! Upstream reference: hapi-fhir/hapi-tinder-plugin, org.hl7.fhir.core/org.hl7.fhir.r4/model
//!
//! Capability spec: `openspec/specs/codegen/spec.md`
//!
//! # What this crate is
//!
//! A build-time generator, not a runtime dependency of anything. It reads one
//! pinned FHIR definition package and writes the `frevo-model` sources, which
//! are **checked in** rather than produced by a `build.rs` — `design.md`
//! section 1 records why.
//!
//! The input is the published package `hl7.fhir.r4.core#4.0.1`, fetched by
//! `packages/fetch.sh` into a git-ignored directory and verified against the
//! digest [`pin`] records. `reference/` is the cross-check, never the input: a
//! generator that read it would make the model unreproducible for anyone who
//! had not cloned it (`codegen.inputs.004`).
//!
//! # Where to look
//!
//! | Concern | Module |
//! |---|---|
//! | the recorded package pin and where the package lives | [`pin`] |
//! | locating, digesting and parsing the package | [`package`] |
//! | classifying definitions and asserting the input inventory | [`inventory`] |
//! | comparing the package with HAPI's copy | [`crosscheck`] |
//! | one `generate` run, the drift check, the refusal paths | [`run`] |
//! | every generated type, its elements and their names | [`model`] |
//! | the header and the written files | [`emit`] |
//! | the type and element tables | [`emit_tables`] |
//! | emitting source `rustfmt` already agrees with | [`rustsrc`] |
//! | definitions written by a caller rather than read | [`synthetic`] |
//! | binding the `codegen-r4` oracle to all of it | [`bind_oracle!`] |
#![forbid(unsafe_code)]

pub mod bind;
pub mod crosscheck;
pub mod emit;
pub mod emit_tables;
pub mod error;
pub mod inventory;
pub mod model;
pub mod package;
pub mod pin;
pub mod run;
pub mod rustsrc;
pub mod sha256;
pub mod synthetic;
pub mod xml;

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-codegen";

pub use error::{CodegenError, FailureKind};
pub use inventory::Inventory;
pub use pin::{PACKAGE_NAME, PACKAGE_VERSION, PackagePin};
pub use run::{GenerateReport, GenerateRequest, generate};
