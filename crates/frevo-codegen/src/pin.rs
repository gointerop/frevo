//! The input package pin: what the repository records, and where the fetch
//! script puts it.
//!
//! `design.md` section 2 states the rule these constants implement: the
//! authoritative input is the published FHIR package `hl7.fhir.r4.core#4.0.1`,
//! fetched once into a git-ignored directory and verified against a SHA-256
//! recorded in the repository. `reference/` is the cross-check, never the
//! input (`codegen.inputs.004`).
//!
//! Two digests are recorded, because they verify different things:
//!
//! - [`PACKAGE_ARCHIVE_SHA256`] is the digest of the published `.tgz` as the
//!   registry serves it. `packages/fetch.sh` checks it, so a substituted
//!   download fails before anything is extracted.
//! - [`PACKAGE_CONTENT_SHA256`] is the digest of the extracted definition
//!   directory, computed by [`crate::package::content_digest`]. The generator
//!   checks that one, because it is the digest of what the generator actually
//!   reads: an archive digest proves nothing about a tree somebody edited
//!   after extracting it.
//!
//! Both are recorded in `crates/frevo-codegen/packages/PINS.md` as well, in
//! prose a reviewer can check against `sha256sum`.

use std::path::{Path, PathBuf};

/// `package.json`'s `name`.
pub const PACKAGE_NAME: &str = "hl7.fhir.r4.core";
/// `package.json`'s `version`, and `StructureDefinition.fhirVersion`.
pub const PACKAGE_VERSION: &str = "4.0.1";
/// The `--version` selector that chooses this package.
pub const VERSION_SELECTOR: &str = "r4";
/// The generator's own name, as it appears in every generated header.
pub const GENERATOR_NAME: &str = "frevo-codegen";

/// SHA-256 of the published archive, from
/// `https://packages.fhir.org/hl7.fhir.r4.core/4.0.1` and, byte for byte, from
/// `https://packages.simplifier.net/hl7.fhir.r4.core/4.0.1`.
pub const PACKAGE_ARCHIVE_SHA256: &str =
    "ebd7731df7d36b5b7d39d5fb6c9d77b44bb7fe5742f1a2e87f164738c3289d44";

/// SHA-256 of the extracted definition directory, over the canonical manifest
/// [`crate::package::content_digest`] builds. 4,583 files, 31,260,763 bytes.
pub const PACKAGE_CONTENT_SHA256: &str =
    "8af9fe30008c2a02c351728c198759141b05482f91dee63475777aa23f06354c";

/// Where `packages/fetch.sh` extracts the package, relative to the repository
/// root. Git-ignored: see `crates/frevo-codegen/packages/.gitignore`.
pub const PACKAGE_DIR: &str = "crates/frevo-codegen/packages/fetched/hl7.fhir.r4.core-4.0.1";

/// The command that fetches it, named by every `PackageMissing` refusal
/// (`codegen.inputs.003`).
pub const FETCH_COMMAND: &str = "bash crates/frevo-codegen/packages/fetch.sh";

/// The environment variable that overrides the recorded location, for a
/// checkout whose package lives elsewhere.
pub const PACKAGE_DIR_ENV: &str = "FREVO_R4_PACKAGE_DIR";

/// The pin the repository records for the input package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePin {
    pub name: String,
    pub version: String,
    /// Lower-case hex, 64 characters: [`PACKAGE_CONTENT_SHA256`].
    pub sha256: String,
}

/// The recorded pin. Read from the repository, never discovered at run time.
pub fn recorded_pin() -> PackagePin {
    PackagePin {
        name: PACKAGE_NAME.to_string(),
        version: PACKAGE_VERSION.to_string(),
        sha256: PACKAGE_CONTENT_SHA256.to_string(),
    }
}

/// The repository root, derived from this crate's manifest directory so that
/// no machine or worktree is named in the source.
///
/// Deliberately **not** canonicalised, even though that would tidy the `..`
/// components out of the paths a refusal message prints. `codegen.inputs.004`
/// asserts that no path a run opened is under `reference/`, and it builds the
/// `reference/` prefix the same way this does. Canonicalising one side and not
/// the other — on Windows, into a `\\?\` form — would make that assertion
/// compare two paths that can never share a prefix, and so pass whatever the
/// generator read.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

/// A repository-relative path, with `/` separators in the input.
pub fn repo_path(relative: &str) -> PathBuf {
    let mut p = repo_root();
    for part in relative.split('/') {
        p.push(part);
    }
    p
}

/// Where the package is, when a run does not say.
///
/// [`PACKAGE_DIR_ENV`] wins when it is set, so a checkout that fetched the
/// package somewhere else does not have to pass `--package` on every run.
pub fn default_package_dir() -> PathBuf {
    match std::env::var_os(PACKAGE_DIR_ENV) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => repo_path(PACKAGE_DIR),
    }
}
