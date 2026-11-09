//! One `frevo-codegen generate` run, and the two checks that hang off it.
//!
//! The order of the steps is the contract, not an implementation detail. A run
//! locates the package, verifies its digest, asserts the input inventory, and
//! only then writes anything — so every refusal path leaves the output
//! directory untouched, which `codegen.inputs.002`, `codegen.inputs.003` and
//! `codegen.inputs.006` each assert separately.

use std::path::{Path, PathBuf};

use crate::crosscheck::{self, CrossCheckReport};
use crate::emit::{self, GeneratedFile};
use crate::error::{CodegenError, Result};
use crate::inventory::{self, Inventory};
use crate::package::{self, Package};
use crate::pin::{self, PACKAGE_NAME, PACKAGE_VERSION, PackagePin, VERSION_SELECTOR};
use crate::synthetic;

/// One generator invocation.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    /// The `--version` selector.
    pub fhir_version: String,
    /// `--package`, or `None` for the directory the recorded pin names.
    pub package_dir: Option<PathBuf>,
    /// The digest to verify against, or `None` for the recorded one.
    pub expected_sha256: Option<String>,
    /// `--out`.
    pub out_dir: PathBuf,
    /// Where `reference/` would be. Accepted so that a run with it and a run
    /// without it are both expressible, and ignored because the generator must
    /// never read it (`codegen.inputs.004`).
    pub reference_dir: Option<PathBuf>,
}

impl GenerateRequest {
    /// The ordinary run: the pinned package, the recorded digest, no
    /// `reference/`.
    pub fn pinned(out_dir: &Path) -> Self {
        GenerateRequest {
            fhir_version: VERSION_SELECTOR.to_string(),
            package_dir: None,
            expected_sha256: None,
            out_dir: out_dir.to_path_buf(),
            reference_dir: None,
        }
    }
}

/// Everything one run reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateReport {
    /// `0` on success; every refusal is a [`CodegenError::Failed`] instead.
    pub exit_code: i32,
    /// The pin the run verified.
    pub pin: PackagePin,
    /// The summary the command prints.
    pub summary: String,
    pub inventory: Inventory,
    /// Every path the run opened.
    pub inputs_read: Vec<PathBuf>,
    pub files: Vec<GeneratedFile>,
}

/// The CI drift check (`codegen.output.005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    pub in_sync: bool,
    /// Paths relative to the checked-in directory, with `/` separators.
    pub differing_files: Vec<String>,
    /// What to do about it: the regeneration command, never an instruction to
    /// edit the output.
    pub remediation: String,
}

/// What a run produces, before anything is written.
struct Plan {
    pin: PackagePin,
    summary: String,
    inventory: Inventory,
    inputs_read: Vec<PathBuf>,
    files: Vec<GeneratedFile>,
}

/// Locate, verify and read the input package.
fn input(request: &GenerateRequest) -> Result<std::sync::Arc<Package>> {
    if request.fhir_version != VERSION_SELECTOR {
        return Err(CodegenError::other(format!(
            "--version {:?} is not supported; this generator emits {VERSION_SELECTOR} only",
            request.fhir_version
        )));
    }
    let dir = request
        .package_dir
        .clone()
        .unwrap_or_else(pin::default_package_dir);
    let loaded = package::load(&dir)?;
    let expected = request
        .expected_sha256
        .clone()
        .unwrap_or_else(|| pin::PACKAGE_CONTENT_SHA256.to_string());
    package::verify_digest(&loaded, &expected)?;
    Ok(loaded)
}

fn plan(request: &GenerateRequest) -> Result<Plan> {
    let loaded = input(request)?;
    let found = inventory::of(&loaded);
    inventory::assert_pinned(found)?;

    let pin = pin::recorded_pin();
    let files = vec![emit::module_root(&pin, &found)];
    let summary = summary(&pin, &loaded, &found, &files);
    Ok(Plan {
        pin,
        summary,
        inventory: found,
        inputs_read: loaded.files_read.clone(),
        files,
    })
}

fn summary(
    pin: &PackagePin,
    loaded: &Package,
    found: &Inventory,
    files: &[GeneratedFile],
) -> String {
    format!(
        "{PACKAGE_NAME} {PACKAGE_VERSION} verified: SHA-256 {digest} over {hashed} files \
         ({bytes} bytes). Read {datatypes} datatype StructureDefinitions, {resources} resource \
         StructureDefinitions ({concrete} concrete, {abstract_count} abstract, {logical} \
         logical) and {parameters} SearchParameter resources. Wrote {written} file(s).",
        digest = pin.sha256,
        hashed = loaded.digest_file_count,
        bytes = loaded.digest_byte_count,
        datatypes = found.datatype_definitions,
        resources = found.resource_definitions,
        concrete = found.concrete_resources,
        abstract_count = found.abstract_resources,
        logical = found.logical_resources,
        parameters = found.search_parameters,
        written = files.len(),
    )
}

/// One `frevo-codegen generate` run.
pub fn generate(request: &GenerateRequest) -> Result<GenerateReport> {
    let planned = plan(request)?;
    emit::write_all(&request.out_dir, &planned.files)?;
    Ok(GenerateReport {
        exit_code: 0,
        pin: planned.pin,
        summary: planned.summary,
        inventory: planned.inventory,
        inputs_read: planned.inputs_read,
        files: planned.files,
    })
}

/// A run over definitions written by a caller rather than read from the
/// package.
///
/// The one behaviour tasks 6 and 7 owe this surface is the inventory
/// assertion: a synthetic input set that claims to be the pinned R4 package
/// and is not fails the run (`codegen.inputs.006`). Generating a model from
/// synthetic definitions is tasks 8 to 13, and until one of them lands this
/// answers [`CodegenError::NotImplemented`] rather than an empty success,
/// so a scenario that needs a generated type cannot be read as passing.
pub fn generate_synthetic(request: &synthetic::Request) -> Result<GenerateReport> {
    if request.assert_pinned_inventory {
        inventory::assert_pinned(synthetic::inventory_of(&request.definitions))?;
    }
    Err(CodegenError::NotImplemented)
}

/// Compare the pinned package with HAPI's copy (`codegen.inputs.005`).
pub fn cross_check(
    package_dir: Option<&Path>,
    hapi_profile_dir: &Path,
) -> Result<CrossCheckReport> {
    let dir = package_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(pin::default_package_dir);
    let loaded = package::load(&dir)?;
    package::verify_digest(&loaded, pin::PACKAGE_CONTENT_SHA256)?;
    crosscheck::run(&loaded, hapi_profile_dir)
}

/// Compare a fresh run with a checked-in output tree (`codegen.output.005`).
pub fn drift_check(request: &GenerateRequest, checked_in_dir: &Path) -> Result<DriftReport> {
    let planned = plan(request)?;

    let mut differing = Vec::new();
    for file in &planned.files {
        let mut target = checked_in_dir.to_path_buf();
        for part in file.path.split('/') {
            target.push(part);
        }
        match std::fs::read(&target) {
            Ok(bytes) if bytes == file.bytes => {}
            _ => differing.push(file.path.clone()),
        }
    }
    let generated: Vec<&str> = planned.files.iter().map(|f| f.path.as_str()).collect();
    for existing in checked_in_files(checked_in_dir) {
        if !generated.contains(&existing.as_str()) {
            differing.push(existing);
        }
    }
    differing.sort();
    differing.dedup();

    Ok(DriftReport {
        in_sync: differing.is_empty(),
        differing_files: differing,
        remediation: format!(
            "regenerate the checked-in output: `{} generate --version {VERSION_SELECTOR} \
             --out {}`, then commit the result. A generated file is never corrected by hand.",
            pin::GENERATOR_NAME,
            checked_in_dir.display()
        ),
    })
}

/// Every file under `dir`, relative, with `/` separators.
fn checked_in_files(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}
