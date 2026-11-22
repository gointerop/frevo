//! The in-memory code validation `memberOf` needs: whether a code is in a
//! `ValueSet`, read over the [`Value`] node a host service or the model hands
//! back.
//!
//! The pinned engine calls `worker.validateCode(options, coding, valueSet)`
//! (`FHIRPathEngine.java:2608`-`:2620`, `:4799`-`:4805`) and keeps only
//! `isOk()`. What answers that depends on the worker a caller installs; this
//! module is the part of it that can be decided from the value set and the
//! code systems the model carries, without a terminology server:
//!
//! - `compose.include` is a union and `compose.exclude` is subtracted from it;
//! - an include's `concept` list is matched by code, and its `system`, when
//!   both it and the code's system are present, must be the code's;
//! - an include of a whole code system is matched against that code system's
//!   concepts, hierarchy included, when the code system's `content` is
//!   `complete`;
//! - a value set with no `compose` is matched against `expansion.contains`.
//!
//! Everything else -- an include `filter`, an include that imports another
//! value set, an incomplete or unknown code system, a `Coding` with no system
//! and no system guessing -- is [`EvalError::Unsupported`], never a guess at
//! `false` or `true`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/terminologies/ValueSetCheckerSimple.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 -- `codeInValueSet` at `:327`,
//! `inComponent` at `:344`, `findCodeInConcept` at `:275` and
//! `checkExpansion` at `:150`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: an include whose `concept` list is matched
//! without a `system` on either side is accepted at face value (upstream
//! accepts it only when it is the sole include and the code has no system);
//! a value set with a `compose` is not also read for an `expansion`; the
//! display check is not made, because `memberOf` reads only `isOk()`; and
//! every branch upstream would raise on, or answer from a terminology server,
//! is [`EvalError::Unsupported`].

use crate::seam::model::Model;
use crate::seam::value::Value;

use super::EvalError;
use super::equality::primitive_text;
use super::navigate::children_by_name;

/// One code to validate: `castToCoding`'s system and code, and whether the
/// system may be guessed (`terminologyServiceOptions.withGuessSystem()`).
#[derive(Debug, Clone)]
pub(super) struct CodeToCheck {
    pub(super) system: Option<String>,
    pub(super) code: Option<String>,
    pub(super) guess_system: bool,
}

/// The children of `item` named `name`.
fn children(item: &Value, name: &str) -> Result<Vec<Value>, EvalError> {
    let mut out = Vec::new();
    children_by_name(item, name, &mut out)?;
    Ok(out)
}

/// The primitive text of the first child of `item` named `name`.
fn text(model: &dyn Model, item: &Value, name: &str) -> Result<Option<String>, EvalError> {
    Ok(children(item, name)?
        .first()
        .and_then(|v| primitive_text(model, v)))
}

/// Whether `worker.validateCode(...)` would be ok for `code` against `vs`.
pub(super) fn validate_code(
    model: &dyn Model,
    code: &CodeToCheck,
    vs: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    let Some(value) = code.code.as_deref() else {
        // A Coding with no code is never valid.
        return Ok(false);
    };
    if code.system.is_none() && !code.guess_system {
        return Err(EvalError::unsupported(
            "memberOf() of a Coding with no system",
            offset,
        ));
    }
    let system = code.system.as_deref();
    let compose = children(vs, "compose")?;
    if let Some(compose) = compose.first() {
        let mut ok = false;
        for include in children(compose, "include")? {
            if in_component(model, &include, system, value, offset)? {
                ok = true;
                break;
            }
        }
        if ok {
            for exclude in children(compose, "exclude")? {
                if in_component(model, &exclude, system, value, offset)? {
                    return Ok(false);
                }
            }
        }
        return Ok(ok);
    }
    let mut contains = Vec::new();
    for expansion in children(vs, "expansion")? {
        contains.extend(children(&expansion, "contains")?);
    }
    in_expansion(model, &contains, system, value)
}

/// One `compose.include` or `compose.exclude` entry.
fn in_component(
    model: &dyn Model,
    component: &Value,
    system: Option<&str>,
    code: &str,
    offset: usize,
) -> Result<bool, EvalError> {
    if !children(component, "filter")?.is_empty() {
        return Err(EvalError::unsupported(
            "memberOf() against a value set whose compose uses a filter",
            offset,
        ));
    }
    if !children(component, "valueSet")?.is_empty() {
        return Err(EvalError::unsupported(
            "memberOf() against a value set whose compose imports another value set",
            offset,
        ));
    }
    let component_system = text(model, component, "system")?;
    if let (Some(wanted), Some(declared)) = (system, component_system.as_deref()) {
        if wanted != declared {
            return Ok(false);
        }
    }
    let concepts = children(component, "concept")?;
    if !concepts.is_empty() {
        for concept in &concepts {
            if text(model, concept, "code")?.as_deref() == Some(code) {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    let Some(declared) = component_system else {
        return Ok(false);
    };
    let cs = match model.code_system(&declared) {
        Some(Some(cs)) => cs,
        Some(None) | None => {
            return Err(EvalError::unsupported(
                format!(
                    "memberOf() needs the code system {declared}, which the model does not carry"
                ),
                offset,
            ));
        }
    };
    if text(model, &cs, "content")?.as_deref() != Some("complete") {
        return Err(EvalError::unsupported(
            format!("memberOf() needs the code system {declared}, which is not complete"),
            offset,
        ));
    }
    in_concepts(model, &children(&cs, "concept")?, code)
}

/// `findCodeInConcept`: the concept tree, depth first.
fn in_concepts(model: &dyn Model, concepts: &[Value], code: &str) -> Result<bool, EvalError> {
    for concept in concepts {
        if text(model, concept, "code")?.as_deref() == Some(code) {
            return Ok(true);
        }
        if in_concepts(model, &children(concept, "concept")?, code)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `checkExpansion`: the `contains` tree, depth first.
fn in_expansion(
    model: &dyn Model,
    contains: &[Value],
    system: Option<&str>,
    code: &str,
) -> Result<bool, EvalError> {
    for c in contains {
        let system_ok = match system {
            Some(s) => text(model, c, "system")?.as_deref() == Some(s),
            None => true,
        };
        if system_ok && text(model, c, "code")?.as_deref() == Some(code) {
            return Ok(true);
        }
        if in_expansion(model, &children(c, "contains")?, system, code)? {
            return Ok(true);
        }
    }
    Ok(false)
}
