//! application.yaml-compatible configuration with the starter's hapi.fhir.* key semantics.
//!
//! Upstream reference: hapi-fhir-jpaserver-starter/.../AppProperties.java
//!
//! Capability register: `docs/capabilities/config.md`
//! Requirements (once archived): `openspec/specs/config/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-config";
