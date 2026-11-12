//! Generated FHIR resource and datatype models with a lazy, byte-preserving JSON codec.
//!
//! Upstream reference: org.hl7.fhir.core/org.hl7.fhir.r4/model (regenerated, never translated)
//!
//! Capability spec: `openspec/specs/model/spec.md`
//!
//! Two things live here. [`codec`] is the hand-written lazy, byte-preserving
//! codec of `design.md` section 3 and section 9; [`r4`] is `frevo-codegen`'s
//! generated output, read from `hl7.fhir.r4.core#4.0.1`, and is what the codec
//! reads its type, element and search-parameter tables from.
//!
//! The codec has two halves, and which one a caller wants depends on what it
//! is doing. [`Resource`] is the lazy one: loading costs nothing, a read costs
//! only the tokens on the way to the element, and serialising an unedited
//! resource is the buffer it was loaded with. [`codec::decode`] is the eager
//! one: it decodes a whole document into an owned tree and can re-emit it in
//! the generated element order, which is what a round-trip comparison needs
//! and what a pass-through cannot answer.
#![forbid(unsafe_code)]

pub mod codec;

/// The generated R4 model: type, element, code and search-parameter tables.
///
/// Produced by `frevo-codegen` from `hl7.fhir.r4.core#4.0.1`; every file carries
/// an `@generated` marker. Declared here so the compiler and
/// `cargo clippy --workspace` actually see it -- until this line existed the
/// 73,465 emitted lines were in the repository but not in the build.
pub mod r4;

pub use codec::decode::{
    DEFAULT_PARSE_MODE, Decimal, Decoded, DecodedElement, Diagnostic, ParseMode, ReportKind, decode,
};
pub use codec::{DecodeError, R4, R4Model, Report, Resource, ResourceNode};

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-model";
