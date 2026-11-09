//! FHIRPath parser and evaluation engine.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-base/.../fhirpath, org.hl7.fhir.core/.../utils/FHIRPathEngine.java
//!
//! Capability spec: `openspec/specs/path/spec.md`
//!
//! The lexer, parser, check pass and evaluator arrive through
//! `add-fhirpath-and-codegen-r4`. What is here today is the **evaluation
//! seam**: the `Value`, `Node`, `Model` and `HostServices` definitions of
//! `design.md` section 4, with no implementation behind them.
//!
//! The seam lives here rather than in `frevo-model` because the dependency
//! edge runs the other way: `frevo-model` implements `Node` and `Model` over
//! its span-based codec, and `path.api.008` makes it a test that this crate's
//! manifest and sources never name `frevo-model`.
#![forbid(unsafe_code)]

mod seam;

pub use seam::host::{ConstantMode, FunctionDetails, HostError, HostServices};
pub use seam::model::{Model, TypeSet};
pub use seam::node::{Node, NodeError};
pub use seam::value::{
    Decimal, PartialDate, PartialDateTime, PartialTime, Primitive, Quantity, TemporalPrecision,
    Value,
};

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-path";
