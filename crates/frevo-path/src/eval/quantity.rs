//! The engine's side of the UCUM service: how a quantity is handed to the unit
//! algebra and how its answer comes back (`path.quantity.001` to `.024`).
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`qtyToCanonicalPair` at `:2288-2299`, `qtyToCanonicalDecimal` at
//! `:2301-2312`, `pairToQty` at `:2314-2316`, `qtyToPair` at `:2319-2328`) at
//! 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the worker's UCUM service is the process's
//! loaded essence data ([`service`], ADR 0006), present in every
//! configuration (`path.quantity.015`, `.019`, `DEV-0029`); a
//! `NullPointerException` upstream is an execution error carrying the JVM's
//! text and no offset (`path.errors.002`, `path.errors.003`).

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::seam::value::{Decimal, Quantity, Value};

use super::EvalError;
use super::arith::make_decimal;
use super::operand::{Qty, UCUM_SYSTEM};
use super::ucum::{UcumEssence, UcumLoadError};
use super::ucum_algebra::UcumError;
use super::ucum_decimal::UcumDecimal;

/// The environment variable naming the `ucum-essence.xml` to load.
pub const UCUM_ESSENCE_ENV: &str = "FREVO_UCUM_ESSENCE";

/// The essence file used when [`UCUM_ESSENCE_ENV`] is not set: the fetched
/// copy in the source tree this crate was built from
/// (`oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml`, `path.quantity.016`).
const DEFAULT_ESSENCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml"
);

static SERVICE: OnceLock<Result<UcumEssence, UcumLoadError>> = OnceLock::new();

/// Install the UCUM data the engine uses, before any quantity operation needs
/// it. `false` when the engine's data is already fixed, by an earlier call or
/// by a quantity operation that loaded the default; the data in force is then
/// unchanged.
pub fn install_ucum_essence(essence: UcumEssence) -> bool {
    SERVICE.set(Ok(essence)).is_ok()
}

/// `worker.getUcumService()`: the installed data, else the file
/// [`UCUM_ESSENCE_ENV`] names, else the fetched copy. Data that cannot be
/// loaded is an execution error, never the engine's no-UCUM branch
/// (`path.quantity.015`, `.016`).
pub(super) fn service() -> Result<&'static UcumEssence, EvalError> {
    let loaded = SERVICE.get_or_init(|| {
        let path = std::env::var_os(UCUM_ESSENCE_ENV)
            .map_or_else(|| PathBuf::from(DEFAULT_ESSENCE), PathBuf::from);
        UcumEssence::load(&path)
    });
    loaded.as_ref().map_err(|e| EvalError::plain(e.to_string()))
}

/// An exception out of the service that no engine arm catches: its message,
/// with no position.
pub(super) fn uncaught(e: &UcumError) -> EvalError {
    EvalError::plain(e.message().to_owned())
}

/// `q.getValue().toPlainString()`, which dereferences a missing value.
fn plain_value(q: &Qty) -> Result<String, EvalError> {
    let Some(text) = &q.value else {
        return Err(EvalError::plain(
            "Cannot invoke \"java.math.BigDecimal.toPlainString()\" because the return value \
             of \"org.hl7.fhir.r4.model.Quantity.getValue()\" is null",
        ));
    };
    let d = Decimal::parse(text)
        .ok_or_else(|| EvalError::plain(format!("\"{text}\" is not a decimal value")))?;
    Ok(make_decimal(d.mantissa(), d.scale(), 0)?
        .lexical()
        .to_owned())
}

fn is_ucum(q: &Qty) -> bool {
    q.system.as_deref() == Some(UCUM_SYSTEM)
}

/// `qtyToCanonicalPair`: `None` outside the UCUM system or when the service
/// raises `UcumException`; a missing code is read as `1`. Any other exception
/// escapes.
pub(super) fn canonical_pair(q: &Qty) -> Result<Option<(String, String)>, EvalError> {
    if !is_ucum(q) {
        return Ok(None);
    }
    let value = plain_value(q)?;
    let code = q.code.as_deref().unwrap_or("1");
    match service()?.canonical_form(&value, code) {
        Ok(pair) => Ok(Some(pair)),
        Err(UcumError::Ucum(_)) => Ok(None),
        Err(e) => Err(uncaught(&e)),
    }
}

/// `qtyToCanonicalDecimal`: the canonical value of [`canonical_pair`].
pub(super) fn canonical_decimal(q: &Qty) -> Result<Option<String>, EvalError> {
    Ok(canonical_pair(q)?.map(|(value, _)| value))
}

/// `qtyToPair`: the plain value and the code as given (possibly `null`), or
/// `None` outside the UCUM system or when `new Decimal(...)` refuses the value.
pub(super) fn to_pair(q: &Qty) -> Result<Option<(String, Option<String>)>, EvalError> {
    if !is_ucum(q) {
        return Ok(None);
    }
    let value = plain_value(q)?;
    if UcumDecimal::new(&value).is_err() {
        return Ok(None);
    }
    Ok(Some((value, q.code.clone())))
}

/// The JVM's message for the service dereferencing a `null` pair, naming the
/// `UcumEssenceService` parameter (`path.quantity.018`).
pub(super) fn null_pair(parameter: &str) -> EvalError {
    EvalError::plain(format!(
        "Cannot invoke \"org.fhir.ucum.Pair.getValue()\" because \"{parameter}\" is null"
    ))
}

/// `pairToQty`: the value, the UCUM system, the canonical code (absent when it
/// is empty, as `Quantity.setCode` leaves it) and no `unit`.
pub(super) fn pair_to_value(
    (value, code): (String, String),
    at: usize,
) -> Result<Value, EvalError> {
    let value = Decimal::parse(&value).ok_or_else(|| super::arith::too_wide(at))?;
    Ok(Value::Quantity(Quantity {
        value,
        unit: None,
        system: Some(UCUM_SYSTEM.to_owned()),
        code: (!code.is_empty()).then_some(code),
    }))
}
