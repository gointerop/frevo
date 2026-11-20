//! FHIRPath parser and evaluation engine.
//!
//! Upstream reference: hapi-fhir/hapi-fhir-base/.../fhirpath, org.hl7.fhir.core/.../utils/FHIRPathEngine.java
//!
//! Capability spec: `openspec/specs/path/spec.md`
//!
//! Four things live here today:
//!
//! - the **evaluation seam** — the `Value`, `Node`, `Model` and `HostServices`
//!   definitions of `design.md` section 4;
//! - the **front end** — the lexer, the literal forms and the parser, which
//!   turn an expression into a reusable [`Expression`] and reject what
//!   `FHIRPathEngine` rejects at parse time;
//! - the **static check pass** — [`Checker`], which types a parsed expression
//!   against a focus type without evaluating it, and fails it where
//!   `FHIRPathEngine.check` would;
//! - the **evaluator core** — [`Evaluator`], which walks a parsed expression
//!   over a [`Node`]: navigation, the leading type name, the indexer,
//!   `$this`/`$index`/`$total`, `children()` and `descendants()`.
//!
//! Every function and operator the evaluator does not implement yet answers
//! [`EvalError::Unsupported`], never an empty collection; everything the
//! check pass cannot type answers [`CheckError::NotImplemented`], never a
//! pass.
//!
//! The seam lives here rather than in `frevo-model` because the dependency
//! edge runs the other way: `frevo-model` implements `Node` and `Model` over
//! its span-based codec, and `path.api.008` makes it a test that this crate's
//! manifest and sources never name `frevo-model`.
#![forbid(unsafe_code)]

mod check;
mod eval;
mod seam;
mod syntax;

pub use check::{
    CheckError, CheckFocus, CheckOptions, CheckOutcome, CheckRequest, CheckWarning, Checker,
    CollectionStatus, WarningId,
};
pub use eval::{
    EngineSettings, EvalError, Evaluator, InvalidVersion, PathError, UCUM_ESSENCE_NAMESPACE,
    UcumBaseUnit, UcumEssence, UcumFunction, UcumLoadError, UcumPrefix, UcumUnit,
};

pub use seam::host::{ConstantMode, FunctionDetails, HostError, HostServices};
pub use seam::model::{ElementInfo, Model, TypeSet};
pub use seam::node::{Node, NodeError};
pub use seam::server::ServerHostServices;
pub use seam::value::{
    Decimal, PartialDate, PartialDateTime, PartialTime, Primitive, Quantity, TemporalPrecision,
    Value,
};
pub use syntax::ast::{
    Arity, Constant, Expression, ExpressionNode, Function, Kind, NodeId, Operation,
};
pub use syntax::error::ParseError;
pub use syntax::literal::{UCUM_SYSTEM, process_date_constant};
pub use syntax::parser::{MAX_PARSE_DEPTH, ParseOptions};

/// Crate identity, used by the server to report which capabilities are linked in.
pub const CRATE_NAME: &str = "frevo-path";
