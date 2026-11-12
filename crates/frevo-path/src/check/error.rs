//! What a check reports: a failure, a feature this pass cannot check yet, and
//! the warnings that never fail a check.
//!
//! The first two are kept apart for the same reason `EvalError` keeps them
//! apart. [`CheckError::Failed`] is a FHIRPath verdict — the expression is
//! semantically invalid, as `FHIRPathEngine.check` would say by raising.
//! [`CheckError::NotImplemented`] is not a verdict at all: the expression
//! needs something this pass, or the model it was given, cannot type yet. A
//! grading caller reports it as not having run. Answering "passes" instead
//! would turn every unchecked feature into a check that accepted it.

use std::error::Error;
use std::fmt;

/// A check did not produce a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The check failed: upstream's `PathEngineException` from the check
    /// pass. `id` is upstream's message constant (`FHIRPATH_UNKNOWN_NAME`),
    /// or `HOST_SERVICE` when the host service raised; `offset` is the byte
    /// offset of the sub-expression it failed on.
    Failed {
        message: String,
        id: &'static str,
        offset: Option<usize>,
    },
    /// Something this pass cannot check yet — a host-supplied function, a
    /// model that cannot describe its elements, a path on which the pinned
    /// engine itself crashes. `feature` names it.
    NotImplemented { feature: String, offset: usize },
}

impl CheckError {
    pub(crate) fn failed(id: &'static str, message: impl Into<String>, offset: usize) -> Self {
        CheckError::Failed {
            message: message.into(),
            id,
            offset: Some(offset),
        }
    }

    pub(crate) fn not_implemented(feature: impl Into<String>, offset: usize) -> Self {
        CheckError::NotImplemented {
            feature: feature.into(),
            offset,
        }
    }

    /// Whether this is the not-implemented kind rather than a verdict.
    pub fn is_not_implemented(&self) -> bool {
        matches!(self, CheckError::NotImplemented { .. })
    }

    /// The diagnostic text of a failure, or the feature of a not-implemented
    /// answer.
    pub fn message(&self) -> &str {
        match self {
            CheckError::Failed { message, .. } => message,
            CheckError::NotImplemented { feature, .. } => feature,
        }
    }

    /// The byte offset in the expression the answer is attributed to.
    pub fn offset(&self) -> Option<usize> {
        match self {
            CheckError::Failed { offset, .. } => *offset,
            CheckError::NotImplemented { offset, .. } => Some(*offset),
        }
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Failed { message, .. } => f.write_str(message),
            CheckError::NotImplemented { feature, offset } => write!(
                f,
                "checking {feature} is not implemented by this engine yet (at offset {offset})"
            ),
        }
    }
}

impl Error for CheckError {}

/// The nine warnings the check pass can emit: the nine `typeWarnings.add`
/// sites in `FHIRPathEngine.java`. A warning never fails a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WarningId {
    ChoiceNoTypeSpecifier,
    ChoiceSpuriousTypeSpecifier,
    CollectionStatusOperationLeft,
    CollectionStatusOperationRight,
    CollectionStatusParameter,
    CollectionStatusContext,
    NotACollection,
    UnknownExtension,
    OfTypeImpossible,
}

impl WarningId {
    /// The stable message id.
    pub fn as_str(self) -> &'static str {
        match self {
            WarningId::ChoiceNoTypeSpecifier => "choice-no-type-specifier",
            WarningId::ChoiceSpuriousTypeSpecifier => "choice-spurious-type-specifier",
            WarningId::CollectionStatusOperationLeft => "collection-status-operation-left",
            WarningId::CollectionStatusOperationRight => "collection-status-operation-right",
            WarningId::CollectionStatusParameter => "collection-status-parameter",
            WarningId::CollectionStatusContext => "collection-status-context",
            WarningId::NotACollection => "not-a-collection",
            WarningId::UnknownExtension => "unknown-extension",
            WarningId::OfTypeImpossible => "of-type-impossible",
        }
    }

    /// Upstream's `I18nConstants` name.
    pub fn upstream_constant(self) -> &'static str {
        match self {
            WarningId::ChoiceNoTypeSpecifier => "FHIRPATH_CHOICE_NO_TYPE_SPECIFIER",
            WarningId::ChoiceSpuriousTypeSpecifier => "FHIRPATH_CHOICE_SPURIOUS_TYPE_SPECIFIER",
            WarningId::CollectionStatusOperationLeft => "FHIRPATH_COLLECTION_STATUS_OPERATION_LEFT",
            WarningId::CollectionStatusOperationRight => {
                "FHIRPATH_COLLECTION_STATUS_OPERATION_RIGHT"
            }
            WarningId::CollectionStatusParameter => "FHIRPATH_COLLECTION_STATUS_PARAMETER",
            WarningId::CollectionStatusContext => "FHIRPATH_COLLECTION_STATUS_CONTEXT",
            WarningId::NotACollection => "FHIRPATH_NOT_A_COLLECTION",
            WarningId::UnknownExtension => "FHIRPATH_UNKNOWN_EXTENSION",
            WarningId::OfTypeImpossible => "FHIRPATH_OFTYPE_IMPOSSIBLE",
        }
    }
}

/// One warning: the stable id and a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckWarning {
    pub id: WarningId,
    pub message: String,
}
