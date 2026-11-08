//! CQL to ELM translation and the ELM evaluation engine.
//!
//! Upstream reference: cqframework/clinical_quality_language
//!
//! Capability register: `docs/capabilities/cql.md`
//! Requirements (once archived): `openspec/specs/cql/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-cql";
