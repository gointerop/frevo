//! Clinical reasoning: measures, care gaps, PlanDefinition apply, SDC and CDS Hooks.
//!
//! Upstream reference: cqframework/clinical-reasoning (cqf-fhir-cr)
//!
//! Capability register: `docs/capabilities/cr.md`
//! Requirements (once archived): `openspec/specs/cr/spec.md`
//!
//! This crate is a Phase 0 placeholder: it carries no behaviour yet. Every
//! requirement arrives through an OpenSpec change on a `change/<id>` branch.
#![forbid(unsafe_code)]

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-cr";
