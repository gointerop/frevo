//! Generated FHIR resource and datatype models with a lazy, byte-preserving JSON codec.
//!
//! Upstream reference: org.hl7.fhir.core/org.hl7.fhir.r4/model (regenerated, never translated)
//!
//! Capability spec: `openspec/specs/model/spec.md`
//!
//! What is here today is the hand-written lazy codec of `design.md` section 3
//! and section 9: [`codec`]. The generated model — `src/r4/` — is
//! `frevo-codegen`'s output and arrives with tasks 7 to 14 of
//! `add-fhirpath-and-codegen-r4`.
#![forbid(unsafe_code)]

pub mod codec;

pub use codec::{DecodeError, R4, R4Model, Report, Resource, ResourceNode};

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-model";
