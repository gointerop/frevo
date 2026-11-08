//! Cross-version resource conversion (DSTU2/DSTU3/R4/R4B/R5).
//!
//! Upstream reference: org.hl7.fhir.core/org.hl7.fhir.convertors
//!
//! Capability spec: `openspec/specs/convert/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-convert";
