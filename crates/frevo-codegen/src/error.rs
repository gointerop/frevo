//! Why a generator run refused, and the exit code it refused with.
//!
//! Every variant of [`FailureKind`] is a condition some `codegen.*` scenario
//! requires the generator to detect and name. `NotImplemented` is deliberately
//! *not* a refusal: it says a surface has no behaviour behind it yet, and the
//! oracle treats it as an error value rather than as a verdict, so a generator
//! that does nothing can never satisfy a scenario that asks it to refuse.

use std::fmt;

/// The exit code every diagnosed refusal uses. Non-zero is all the scenarios
/// require; one value keeps the command's contract simple.
pub const REFUSAL_EXIT_CODE: i32 = 1;

/// What kind of condition a run refused on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureKind {
    /// The input package's digest is not the one the repository records
    /// (`codegen.inputs.002`).
    DigestMismatch,
    /// The input package is not present (`codegen.inputs.003`).
    PackageMissing,
    /// The input set is not the pinned R4 inventory (`codegen.inputs.006`).
    InventoryMismatch,
    /// Two elements would generate the same Rust name (`codegen.naming.003`).
    NameCollision,
    /// A decode or encode failure.
    Codec,
    /// Anything else the generator diagnoses.
    Other,
}

impl FailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureKind::DigestMismatch => "digest-mismatch",
            FailureKind::PackageMissing => "package-missing",
            FailureKind::InventoryMismatch => "inventory-mismatch",
            FailureKind::NameCollision => "name-collision",
            FailureKind::Codec => "codec",
            FailureKind::Other => "other",
        }
    }
}

/// An error from the generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    /// A diagnosed refusal: an exit code, the condition, and a message that
    /// names the facts the scenario requires.
    Failed {
        exit_code: i32,
        kind: FailureKind,
        message: String,
    },
    /// A surface that has no behaviour behind it yet.
    NotImplemented,
}

impl CodegenError {
    pub fn failed(kind: FailureKind, message: impl Into<String>) -> Self {
        CodegenError::Failed {
            exit_code: REFUSAL_EXIT_CODE,
            kind,
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        CodegenError::failed(FailureKind::Other, message)
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Failed {
                exit_code,
                kind,
                message,
            } => write!(f, "exit {exit_code} [{}] {message}", kind.as_str()),
            CodegenError::NotImplemented => f.write_str(
                "this frevo-codegen surface has no behaviour behind it yet; \
                 see openspec/changes/add-fhirpath-and-codegen-r4/tasks.md",
            ),
        }
    }
}

impl std::error::Error for CodegenError {}

pub type Result<T> = std::result::Result<T, CodegenError>;
