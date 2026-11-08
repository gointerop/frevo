//! OpenAPI document generation and the Swagger UI surface.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-server-openapi
//!
//! Capability register: `docs/capabilities/openapi.md`
//! Requirements (once archived): `openspec/specs/openapi/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-openapi";
