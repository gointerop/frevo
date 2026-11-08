//! Terminology store, code-system loaders and the terminology operations.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-jpaserver-base/.../term
//!
//! Capability spec: `openspec/specs/terminology/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-terminology";
