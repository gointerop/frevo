//! `ServerHostServices` -- the host service Frevo's own server-side callers
//! install: it owns no name, so it answers empty for every one.
//!
//! Derived from hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java
//! (`setEvaluationContext`'s anonymous `IHostApplicationServices`) at
//! 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright Smile CDR, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the delegate that answers `resolveConstant`
//! and `resolveReference` upstream is not part of this crate, so it is the
//! "nothing owned" delegate; `executeFunction`, which answers `null` upstream
//! and so fails later with a `NullPointerException`, is a typed failure.
//!
//! Why this exists, and why it must answer empty: evaluation asks the host
//! service about a **bare name** before navigating when an application context
//! is passed (`ConstantMode::ImplicitBefore`), and a non-empty answer replaces
//! the element (`DEV-0010`). A host service that answered anything for a name
//! it does not own would make a resource's own elements unreachable on the
//! search-parameter extraction path.

use super::host::{ConstantMode, FunctionDetails, HostError, HostServices};
use super::model::TypeSet;
use super::value::Value;

/// A host service that owns no constant and resolves nothing. Every
/// `conformsTo` answer is `false`, as the upstream anonymous class's is.
#[derive(Debug, Clone, Copy, Default)]
pub struct ServerHostServices;

impl HostServices for ServerHostServices {
    fn resolve_constant(&self, _name: &str, _mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        Ok(Vec::new())
    }

    fn resolve_constant_type(
        &self,
        _name: &str,
        _mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        Ok(None)
    }

    fn resolve_reference(
        &self,
        _url: &str,
        _identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        Ok(None)
    }

    fn conforms_to_profile(&self, _item: &Value, _url: &str) -> Result<bool, HostError> {
        Ok(false)
    }

    fn resolve_value_set(&self, _url: &str) -> Result<Option<Value>, HostError> {
        Ok(None)
    }

    fn resolve_function(&self, _name: &str) -> Option<FunctionDetails> {
        None
    }

    fn check_function(
        &self,
        name: &str,
        _focus: &TypeSet,
        _params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        Err(HostError::new(format!("no such function: {name}")))
    }

    fn execute_function(
        &self,
        name: &str,
        _focus: &[Value],
        _params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        Err(HostError::new(format!("no such function: {name}")))
    }

    fn param_is_type(&self, _name: &str, _index: usize) -> bool {
        false
    }

    fn log(&self, _name: &str, _focus: &[Value]) -> bool {
        false
    }
}
