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

use std::error::Error;
use std::fmt;

use crate::seam::node::NodeError;

/// An evaluation did not produce a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// A feature the engine does not implement. `feature` names it, as
    /// `the function where()` or `the operator =`; `offset` is the byte
    /// offset in the expression of the sub-expression that needs it.
    Unsupported { feature: String, offset: usize },
    /// An execution error: the expression is valid and the engine implements
    /// it, and evaluating it over this input fails.
    Execution { message: String, offset: usize },
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

    pub(crate) fn execution(message: impl Into<String>, offset: usize) -> Self {
        EvalError::Execution {
            message: message.into(),
            offset,
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
            EvalError::Unsupported { offset, .. } | EvalError::Execution { offset, .. } => {
                Some(*offset)
            }
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
            EvalError::Execution { message, offset } => {
                write!(f, "{message} (at offset {offset})")
            }
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
