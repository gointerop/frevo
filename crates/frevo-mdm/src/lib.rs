//! Master data management: matching rules, golden records, links and the MDM operations.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-jpaserver-mdm
//!
//! Capability register: `docs/capabilities/mdm.md`
//! Requirements (once archived): `openspec/specs/mdm/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-mdm";
