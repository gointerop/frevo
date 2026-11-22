//! What an evaluation failure carries.
//!
//! Two kinds of failure are kept apart on purpose, and conflating them is the
//! failure mode this type exists to prevent:
//!
//! - [`EvalError::Unsupported`] — the expression uses a function, an operator
//!   or a literal form this engine **does not implement yet**. It is not a
//!   FHIRPath answer at all. A caller that grades results (the oracle harness)
//!   must report it as not having run, and a caller that serves results must
//!   refuse rather than answer. Returning an empty collection here instead
//!   would make every unimplemented feature look like an expression that
//!   matched nothing, which FHIRPath semantics would then propagate silently.
//! - [`EvalError::Execution`] and [`EvalError::Node`] — real failures, the
//!   ones `FHIRPathEngine` raises as `PathEngineException` or that the model
//!   raised while navigating (`path.errors.005`).
//!
//! [`EvalError::execution`] is derived from
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/exceptions/PathEngineException.java
//! (the location-carrying constructors, `:81-93`) at
//! 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the location suffix and the byte offset are
//! attached to an error value rather than to a thrown exception.

use std::error::Error;
use std::fmt;

use crate::seam::node::NodeError;
pub(crate) use crate::syntax::ast::At;

/// An evaluation did not produce a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// A feature the engine does not implement. `feature` names it, as
    /// `the function where()` or `the operator =`; `offset` is the byte
    /// offset in the expression of the sub-expression that needs it.
    Unsupported { feature: String, offset: usize },
    /// An execution error: the expression is valid and the engine implements
    /// it, and evaluating it over this input fails.
    ///
    /// `offset` is the zero-based byte offset of the first token of the node
    /// HAPI attributes the failure to. It is present exactly when HAPI's
    /// message carries a position suffix -- ` (@char C)` or
    /// ` (@line L char C)`, already part of `message` -- and absent where
    /// HAPI's failing node has no start, where it raises with no holder, and
    /// where the failure is not a `PathEngineException` built with a location
    /// at all (`path.errors.002`).
    Execution {
        message: String,
        offset: Option<usize>,
    },
    /// The model failed to navigate. Propagated, never turned into empty.
    Node(NodeError),
}

impl EvalError {
    pub(crate) fn unsupported(feature: impl Into<String>, offset: usize) -> Self {
        EvalError::Unsupported {
            feature: feature.into(),
            offset,
        }
    }

    /// `makeException(holder, ...)` and `makeExceptionPlural` (and the direct
    /// `new PathEngineException(message, id, location, ...)` constructions):
    /// `message` is HAPI's formatted text, and `at` the holder's start, or the
    /// operator start the construction passes. With a start, the message gets
    /// `rep(location, ...)`'s suffix and the error the start's byte offset;
    /// without one it is the bare message with no offset
    /// (`PathEngineException.java:81-93`, `path.errors.002`).
    pub(crate) fn execution(message: impl Into<String>, at: At) -> Self {
        let mut message = message.into();
        let offset = at.position.map(|position| {
            message.push_str(&position.location.suffix());
            position.offset
        });
        EvalError::Execution { message, offset }
    }

    /// A failure HAPI raises with no location: a `PathEngineException` built
    /// without one, or a Java runtime exception the engine does not wrap. The
    /// message is exactly as given and there is no offset (`path.errors.002`).
    pub(crate) fn plain(message: impl Into<String>) -> Self {
        EvalError::Execution {
            message: message.into(),
            offset: None,
        }
    }

    /// Whether this is the not-implemented kind rather than a real failure.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, EvalError::Unsupported { .. })
    }

    /// The byte offset in the expression the failure is attributed to, when
    /// there is one. A model failure carries a document offset instead, in its
    /// [`NodeError`].
    pub fn offset(&self) -> Option<usize> {
        match self {
            EvalError::Unsupported { offset, .. } => Some(*offset),
            EvalError::Execution { offset, .. } => *offset,
            EvalError::Node(_) => None,
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Unsupported { feature, offset } => write!(
                f,
                "{feature} is not implemented by this engine yet (at offset {offset})"
            ),
            EvalError::Execution {
                message,
                offset: Some(offset),
            } => write!(f, "{message} (at offset {offset})"),
            EvalError::Execution {
                message,
                offset: None,
            } => f.write_str(message),
            EvalError::Node(e) => write!(f, "navigation failed: {e}"),
        }
    }
}

impl Error for EvalError {}

impl From<NodeError> for EvalError {
    fn from(e: NodeError) -> Self {
        EvalError::Node(e)
    }
}
