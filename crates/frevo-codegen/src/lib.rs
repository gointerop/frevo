//! Code generator: FHIR StructureDefinitions to Rust models, codecs and search-parameter tables.
//!
//! Upstream reference: hapi-fhir/hapi-tinder-plugin, org.hl7.fhir.core/org.hl7.fhir.r4/model
//!
//! Capability spec: `openspec/specs/codegen/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-codegen";
