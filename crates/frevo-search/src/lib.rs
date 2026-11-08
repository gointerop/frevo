//! Search-parameter indexing and query planning, including chaining, includes and composites.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-jpaserver-base/.../search
//!
//! Capability spec: `openspec/specs/search/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-search";
