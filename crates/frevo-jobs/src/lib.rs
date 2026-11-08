//! Batch job engine on PostgreSQL SKIP LOCKED, with the bulk export/import and maintenance jobs.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-storage-batch2
//!
//! Capability register: `docs/capabilities/jobs.md`
//! Requirements (once archived): `openspec/specs/jobs/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-jobs";
