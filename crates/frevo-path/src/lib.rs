//! FHIRPath parser and evaluation engine.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-base/.../fhirpath, org.hl7.fhir.core/.../utils/FHIRPathEngine.java
//!
//! Capability register: `docs/capabilities/path.md`
//! Requirements (once archived): `openspec/specs/path/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-path";
