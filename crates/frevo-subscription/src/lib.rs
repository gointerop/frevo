//! Subscriptions: REST-hook, websocket, email and message delivery, plus topic-based subscriptions.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-jpaserver-subscription
//!
//! Capability register: `docs/capabilities/subscription.md`
//! Requirements (once archived): `openspec/specs/subscription/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-subscription";
