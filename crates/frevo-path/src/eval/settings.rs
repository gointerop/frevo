//! The engine-wide switches `FHIRPathEngine` exposes, as one value.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (the fields at `:173-184`, `initFlags()` at `:211-216`) and
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/VersionUtilities.java
//! (`isR5Plus` at `:287`, `isR5Ver` at `:296`, `isR6Ver` at `:279`,
//! `removeLabels` at `:1197`, `fixForSpecialValue` at `:1297`), at
//! 73c432b5f43ddbce979978a760ec4aa81e929094, and
//! hapi-fhir/hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java
//! (the constructor) at the pin in `reference/PINS.md`.
//! Original work Copyright HL7, Inc. and contributors, and Copyright University
//! Health Network, licensed under the Apache License, Version 2.0.
//! Modified by the Frevo authors: the seven fields are one `Copy` value in
//! positive polarity (`enforce_*` rather than `doNotEnforce*`), and the
//! version test is a direct reading of the strings `isR5Plus` accepts rather
//! than a call into a semantic-version parser.
//!
//! Task 52 of `add-fhirpath-and-codegen-r4` (`path.strictness.001` to
//! `path.strictness.010`).

use std::error::Error;
use std::fmt;

use crate::check::CheckOptions;

/// The seven switches of `FHIRPathEngine`, in this crate's polarity: `true`
/// means the rule **is** enforced or the feature **is** on.
///
/// | Field | Upstream | Read by |
/// |---|---|---|
/// | `enforce_as_singleton_rule` | `!doNotEnforceAsSingletonRule` (`:180`) | `opAs`, `funcAs` |
/// | `enforce_as_case_sensitive` | `!doNotEnforceAsCaseSensitive` (`:181`) | `compareTypeNames` |
/// | `allow_polymorphic_names` | `allowPolymorphicNames` (`:177`) | `getChildrenByName`, the check pass |
/// | `implicit_string_conversion` | `doImplicitStringConversion` (`:178`) | the string functions |
/// | `liquid_mode` | `liquidMode` (`:179`) | the parser |
/// | `legacy_mode` | `legacyMode` (`:173`) | `opNotEquals` |
/// | `emit_sql_on_fhir_warning` | `emitSQLonFHIRWarning` (`:184`) | the check pass |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineSettings {
    /// `as` raises for a focus of more than one item.
    pub enforce_as_singleton_rule: bool,
    /// `as` compares type names exactly rather than ignoring case.
    pub enforce_as_case_sensitive: bool,
    /// A typed choice name (`valueQuantity`) navigates the choice element
    /// (`value`), keeping the items of that type.
    pub allow_polymorphic_names: bool,
    /// The string functions convert a non-string focus to its string value
    /// rather than answering empty.
    pub implicit_string_conversion: bool,
    /// `||` terminates the expression and hands the parser back to a host
    /// template engine. Not implemented: this change owns only the off state
    /// (`path.strictness.010`), and a parse under it is
    /// [`EvalError::Unsupported`](crate::EvalError::Unsupported).
    pub liquid_mode: bool,
    /// `!=` with an empty operand answers `true` rather than empty, as DSTU2
    /// and STU3 constraints assume.
    pub legacy_mode: bool,
    /// The check pass warns about a choice element navigated without
    /// `ofType()`.
    pub emit_sql_on_fhir_warning: bool,
}

impl EngineSettings {
    /// The fields as `FHIRPathEngine` declares them, before `initFlags()`
    /// runs: every Java field `false`, so both `as` rules **enforced** and
    /// everything else off. This is what an engine over an R5 or later worker
    /// keeps, and what [`Evaluator::new`](crate::Evaluator::new) starts from.
    pub const DECLARED: EngineSettings = EngineSettings {
        enforce_as_singleton_rule: true,
        enforce_as_case_sensitive: true,
        allow_polymorphic_names: false,
        implicit_string_conversion: false,
        liquid_mode: false,
        legacy_mode: false,
        emit_sql_on_fhir_warning: false,
    };

    /// What `initFlags()` leaves an engine at for a worker below R5: both `as`
    /// rules **not** enforced, everything else off (`path.strictness.001`).
    pub const PRE_R5: EngineSettings = EngineSettings {
        enforce_as_singleton_rule: false,
        enforce_as_case_sensitive: false,
        ..EngineSettings::DECLARED
    };

    /// The settings of an engine created over a worker context reporting
    /// `version`, with no explicit configuration: `initFlags()`'s
    /// `if (!VersionUtilities.isR5Plus(worker.getVersion()))`.
    ///
    /// An empty version is not R5 or later, as upstream (`Utilities.noString`
    /// makes `isR5Ver` and `isR6Ver` false). A version `isR5Plus` would reject
    /// as malformed -- upstream throws `FHIRException("Invalid version")` from
    /// the engine's constructor -- is an [`InvalidVersion`].
    pub fn for_version(version: &str) -> Result<EngineSettings, InvalidVersion> {
        if is_r5_plus(version)? {
            Ok(EngineSettings::DECLARED)
        } else {
            Ok(EngineSettings::PRE_R5)
        }
    }

    /// The engine `FhirPathR4`'s constructor builds: one over the R4 worker,
    /// whose version is `4.0.1`, with `setDoNotEnforceAsCaseSensitive(true)`
    /// and `setDoNotEnforceAsSingletonRule(true)` re-asserting what
    /// `initFlags()` already set. Nothing else is touched, so this equals
    /// [`EngineSettings::PRE_R5`] (`path.strictness.001`, `.007` to `.010`).
    pub const fn server_facing() -> EngineSettings {
        EngineSettings {
            enforce_as_singleton_rule: false,
            enforce_as_case_sensitive: false,
            ..EngineSettings::PRE_R5
        }
    }

    /// The two settings the check pass reads.
    pub fn check_options(&self) -> CheckOptions {
        CheckOptions {
            allow_polymorphic_names: self.allow_polymorphic_names,
            emit_sql_on_fhir_warning: self.emit_sql_on_fhir_warning,
        }
    }
}

/// A version string `VersionUtilities.isR5Plus` rejects as malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidVersion {
    /// The version as given.
    pub version: String,
}

impl fmt::Display for InvalidVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `VersionUtilities.checkVersionValidWildcards` (`:1261-1268`).
        write!(f, "Invalid version: '{}\"", self.version)
    }
}

impl Error for InvalidVersion {}

/// `VersionUtilities.isR5Plus`: `isR5Ver || isR6Ver`, each of which is
/// `removeLabels(checkVersionValidWildcards(fixForSpecialValue(v)))` followed
/// by a prefix test.
fn is_r5_plus(version: &str) -> Result<bool, InvalidVersion> {
    let Some(fixed) = fix_for_special_value(version) else {
        return Ok(false);
    };
    if !is_sem_ver_with_wildcards(&fixed) {
        return Err(InvalidVersion {
            version: fixed.to_string(),
        });
    }
    let bare = remove_labels(&fixed);
    Ok(bare.starts_with("4.5") || bare.starts_with("5.0") || bare.starts_with("6.0"))
}

/// `fixForSpecialValue` (`:1297`): `None` for an empty version; a spec URL is
/// cut to its version segment; the release names map to their versions.
fn fix_for_special_value(version: &str) -> Option<String> {
    if version.is_empty() {
        return None;
    }
    let mut v = version;
    if let Some(rest) = v.strip_prefix("http://hl7.org/fhir/") {
        v = rest.split('/').next().unwrap_or(rest);
    }
    let mapped = match v.to_ascii_uppercase().as_str() {
        "R2" | "DSTU2" => "1.0.2",
        "R2B" => "1.4.0",
        "R3" | "STU3" => "3.0.2",
        "R4" => "4.0.1",
        "R4B" => "4.3.0",
        "R5" => "5.0.0",
        "R6" => "6.0.0-cibuild",
        _ => v,
    };
    Some(mapped.to_string())
}

/// `removeLabels` (`:1197`): everything from the first `+`, then from the
/// first `-`.
fn remove_labels(version: &str) -> &str {
    let v = version.split('+').next().unwrap_or(version);
    v.split('-').next().unwrap_or(v)
}

/// `isSemVerWithWildcards` (`:472`) for the forms a worker context reports:
/// `major.minor[.patch]` with an optional `-label` and `+build`, the major an
/// integer and the minor and patch an integer or a wildcard (`x`, `X`, `*`).
fn is_sem_ver_with_wildcards(version: &str) -> bool {
    let core = remove_labels(version);
    let parts: Vec<&str> = core.split('.').collect();
    let is_int = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    let is_int_or_x = |p: &str| matches!(p, "x" | "X" | "*") || is_int(p);
    match parts.as_slice() {
        [major, minor] => is_int(major) && is_int_or_x(minor),
        [major, minor, patch] => is_int(major) && is_int_or_x(minor) && is_int_or_x(patch),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: path.strictness.001
    #[test]
    fn every_pre_r5_version_is_non_strict_and_r5_is_strict() {
        for v in [
            "1.0.2",
            "3.0.2",
            "4.0.1",
            "4.3.0",
            "R4",
            "4.0",
            "4.0.1-ballot",
            "",
        ] {
            assert_eq!(
                EngineSettings::for_version(v),
                Ok(EngineSettings::PRE_R5),
                "{v}"
            );
        }
        for v in [
            "5.0.0",
            "4.5.0",
            "6.0.0-cibuild",
            "R5",
            "http://hl7.org/fhir/5.0",
        ] {
            assert_eq!(
                EngineSettings::for_version(v),
                Ok(EngineSettings::DECLARED),
                "{v}"
            );
        }
        assert!(EngineSettings::for_version("not a version").is_err());
        assert_eq!(EngineSettings::server_facing(), EngineSettings::PRE_R5);
    }

    // covers: path.strictness.007, path.strictness.008, path.strictness.009, path.strictness.010
    #[test]
    fn the_other_five_switches_are_off_for_r4() {
        let s = EngineSettings::for_version("4.0.1").expect("valid");
        assert!(!s.allow_polymorphic_names);
        assert!(!s.implicit_string_conversion);
        assert!(!s.liquid_mode);
        assert!(!s.legacy_mode);
        assert!(!s.emit_sql_on_fhir_warning);
        assert_eq!(s.check_options(), CheckOptions::default());
    }
}
