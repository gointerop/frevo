//! GraphQL endpoint over the FHIR repository.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-server/.../graphql, org.hl7.fhir.core/.../graphql
//!
//! Capability register: `docs/capabilities/graphql.md`
//! Requirements (once archived): `openspec/specs/graphql/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-graphql";
