//! Full profile validation: snapshot generation, slicing, invariants, bindings.
//!
//! Upstream reference: org.hl7.fhir.core/org.hl7.fhir.validation, hapi-fhir/hapi-fhir-validation
//!
//! Capability register: `docs/capabilities/validator.md`
//! Requirements (once archived): `openspec/specs/validator/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-validator";
