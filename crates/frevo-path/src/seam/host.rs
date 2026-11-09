//! `HostServices` — what the engine cannot know.
//!
//! `design.md` section 4.4 of `add-fhirpath-and-codegen-r4`. Definitions only.

use std::error::Error;
use std::fmt;

use super::model::TypeSet;
use super::value::Value;

/// The four modes `FHIRPathConstantEvaluationMode` carries.
/// `FhirPathR4.setEvaluationContext` maps them one-for-one onto
/// `IFhirPathEvaluationContext.ConstantEvaluationMode`, so a host service
/// written against HAPI's wrapper sees the same value Frevo passes. The sites
/// are not interchangeable:
///
/// | Mode | Site |
/// |---|---|
/// | [`ConstantMode::Explicit`] | a `%name` the expression wrote, at both the evaluation and the check constant node |
/// | [`ConstantMode::NoValue`] | one site: the check pass's name node, at the entry of the expression, when there is no focus type at all |
/// | [`ConstantMode::ImplicitBefore`] | the entry of evaluation, before navigation |
/// | [`ConstantMode::ImplicitAfter`] | after navigation yielded empty |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstantMode {
    Explicit,
    NoValue,
    ImplicitBefore,
    ImplicitAfter,
}

/// A host-supplied function's name and arity. The parser takes a custom
/// function's arity from here, which is why `Function::Custom` is reachable at
/// all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDetails {
    pub name: String,
    pub min_params: usize,
    pub max_params: usize,
}

/// A host service refused or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    pub message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for HostError {}

/// The `IHostApplicationServices` surface, optional and replaceable
/// (`path.api.007`).
///
/// "No host service" is not the same state as "a host service that says no",
/// and the engine distinguishes them: with no host service installed,
/// `conformsTo` raises a no-host-services error (`path.utility.022`).
pub trait HostServices: Send + Sync {
    // --- constants -------------------------------------------------------
    /// Resolve `%name`. Returns a `Vec` because
    /// `FHIRPathTests.resolveConstantReturnsList` asserts that a host service
    /// may return two values for one `%name` (`path.env.012`).
    fn resolve_constant(&self, name: &str, mode: ConstantMode) -> Result<Vec<Value>, HostError>;

    /// The check-pass counterpart of [`HostServices::resolve_constant`].
    fn resolve_constant_type(
        &self,
        name: &str,
        mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError>;

    // --- resolution ------------------------------------------------------
    /// Resolve a reference. `funcResolve` reads both `Reference.reference` and
    /// `Reference.identifier` and passes both on, so a host service that can
    /// resolve by business identifier is able to (`path.utility.017`).
    fn resolve_reference(
        &self,
        url: &str,
        identifier: Option<&Value>,
        context: Option<&Value>,
    ) -> Result<Option<Value>, HostError>;

    fn conforms_to_profile(&self, item: &Value, url: &str) -> Result<bool, HostError>;

    fn resolve_value_set(&self, url: &str) -> Result<Option<Value>, HostError>;

    // --- host-supplied functions -----------------------------------------
    fn resolve_function(&self, name: &str) -> Option<FunctionDetails>;

    fn check_function(
        &self,
        name: &str,
        focus: &TypeSet,
        params: &[TypeSet],
    ) -> Result<TypeSet, HostError>;

    fn execute_function(
        &self,
        name: &str,
        focus: &[Value],
        params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError>;

    /// Whether parameter `index` of `name` is a type name to be passed
    /// unevaluated rather than evaluated.
    fn param_is_type(&self, name: &str, index: usize) -> bool;

    // --- diagnostics -----------------------------------------------------
    /// `true` suppresses the engine's own trace buffer for this entry
    /// (`path.utility.028`). A `()` return would make that unexpressible.
    fn log(&self, name: &str, focus: &[Value]) -> bool;
}
