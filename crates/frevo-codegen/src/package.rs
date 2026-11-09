//! Reading the pinned FHIR definition package.
//!
//! The package is a directory of one-resource-per-file JSON, as published by
//! the FHIR package registry. This module locates it, verifies its digest, and
//! parses the two resource types the generator takes as input:
//! `StructureDefinition` and `SearchParameter`.
//!
//! Three properties matter beyond "it parses":
//!
//! 1. **The digest covers the whole directory**, not the subset the generator
//!    happens to read today. A pin that moved every time a later task started
//!    reading `ValueSet-*.json` would pin nothing.
//! 2. **Loading is memoised per directory.** The oracle performs more than
//!    twenty full runs in one process; re-hashing 31 MB and re-parsing 26 MB of
//!    JSON on each of them would turn a test run into a benchmark of
//!    `serde_json`.
//! 3. **Nothing under `reference/` is opened** (`codegen.inputs.004`). The only
//!    directory this module reads is the one it was given.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::error::{CodegenError, FailureKind, Result};
use crate::pin::{self, FETCH_COMMAND, PACKAGE_NAME, PACKAGE_VERSION};
use crate::sha256::Sha256;

/// One snapshot element, in snapshot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotElement {
    /// `ElementDefinition.path`, `[x]` included.
    pub path: String,
    /// `ElementDefinition.min`, absent when the snapshot omits it.
    pub min: Option<u32>,
    /// `ElementDefinition.max`, as the literal `"0"`, `"1"` or `"*"`.
    pub max: Option<String>,
    /// `ElementDefinition.type.code`, in declared order.
    pub types: Vec<String>,
    /// `ElementDefinition.contentReference`.
    pub content_reference: Option<String>,
    /// `ElementDefinition.binding.strength` and `.valueSet`.
    pub binding: Option<ElementBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementBinding {
    pub strength: String,
    pub value_set: String,
}

/// One `StructureDefinition`, reduced to what the generator and the
/// cross-check need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub url: String,
    pub id: String,
    pub name: String,
    /// `primitive-type`, `complex-type`, `resource` or `logical`.
    pub kind: String,
    pub is_abstract: bool,
    /// `specialization`, `constraint`, or absent.
    pub derivation: Option<String>,
    pub base_definition: Option<String>,
    /// `StructureDefinition.type`.
    pub type_name: Option<String>,
    pub fhir_version: Option<String>,
    /// Whether the definition carries the
    /// `structuredefinition-standards-status` extension, which is what the
    /// specification stamps on every artifact it defines itself. See
    /// [`crate::inventory`] for why that is the scoping rule.
    pub standards_status: Option<String>,
    pub snapshot: Vec<SnapshotElement>,
}

impl Definition {
    /// `true` when the specification marks this definition as one of its own.
    pub fn has_standards_status(&self) -> bool {
        self.standards_status.is_some()
    }
}

/// One composite search parameter component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParamComponent {
    pub definition: String,
    pub expression: String,
}

/// One `SearchParameter`, reduced the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParameter {
    pub id: String,
    pub url: String,
    pub code: String,
    pub search_type: String,
    pub bases: Vec<String>,
    /// `None` when the resource declares no `expression` at all, which is
    /// distinct from an empty one (`codegen.searchparam.005`).
    pub expression: Option<String>,
    pub components: Vec<SearchParamComponent>,
    pub targets: Vec<String>,
    pub multiple_or: Option<bool>,
    pub multiple_and: Option<bool>,
    pub comparator: Option<Vec<String>>,
    pub modifier: Option<Vec<String>>,
    pub chain: Option<Vec<String>>,
    pub standards_status: Option<String>,
}

impl SearchParameter {
    pub fn has_standards_status(&self) -> bool {
        self.standards_status.is_some()
    }
}

/// A loaded, digest-verified package.
#[derive(Debug)]
pub struct Package {
    /// The directory the definitions were read from, after resolution.
    pub dir: PathBuf,
    /// The digest of [`Package::dir`], over the canonical manifest.
    pub digest: String,
    /// How many files the digest covers.
    pub digest_file_count: usize,
    /// How many bytes the digest covers.
    pub digest_byte_count: u64,
    /// Every path the load opened, sorted, absolute.
    pub files_read: Vec<PathBuf>,
    /// Every `StructureDefinition` in the directory, in canonical-URL order.
    pub definitions: Vec<Definition>,
    /// Every `SearchParameter` in the directory, in canonical-URL order.
    pub search_parameters: Vec<SearchParameter>,
}

/// The canonical content digest of a directory.
///
/// For every regular file below `dir`, sorted by its relative path's bytes:
///
/// ```text
/// update(relative path, with / separators, as UTF-8)
/// update(b"\n")
/// update(byte length, as 8 big-endian bytes)
/// update(contents)
/// ```
///
/// The length prefix is what stops a rename plus a concatenation from
/// colliding with the original. The path separator is normalised so that the
/// digest is the same on Windows and on Linux, which `codegen.output.002`
/// cares about for the output and this module cares about for the input.
///
/// Returns the digest, the file count and the byte count.
pub fn content_digest(dir: &Path) -> Result<(String, Vec<PathBuf>, u64)> {
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut paths = Vec::with_capacity(files.len());
    for (relative, absolute) in files {
        let contents = std::fs::read(&absolute)
            .map_err(|e| CodegenError::other(format!("reading {}: {e}", absolute.display())))?;
        hasher.update(relative.as_bytes());
        hasher.update(b"\n");
        hasher.update(&(contents.len() as u64).to_be_bytes());
        hasher.update(&contents);
        bytes += contents.len() as u64;
        paths.push(absolute);
    }
    Ok((hasher.hex(), paths, bytes))
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CodegenError::other(format!("reading {}: {e}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| CodegenError::other(format!("reading {}: {e}", dir.display())))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| CodegenError::other(format!("stat {}: {e}", path.display())))?;
        if kind.is_dir() {
            collect_files(root, &path, out)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((relative, path));
        }
    }
    Ok(())
}

/// Resolve a directory to the one that holds the definitions.
///
/// A fetched FHIR package extracts to a root containing `package/`, so both
/// the extraction root and the `package/` directory itself are accepted. Any
/// other directory, or one that does not exist, is a `PackageMissing` refusal
/// naming the package, the version and the command that fetches it
/// (`codegen.inputs.003`).
pub fn resolve_dir(dir: &Path) -> Result<PathBuf> {
    let missing = |detail: &str| {
        CodegenError::failed(
            FailureKind::PackageMissing,
            format!(
                "the input package {PACKAGE_NAME}#{PACKAGE_VERSION} is not present at {}: \
                 {detail}. Fetch it with `{FETCH_COMMAND}`.",
                dir.display()
            ),
        )
    };
    if !dir.exists() {
        return Err(missing("the directory does not exist"));
    }
    if !dir.is_dir() {
        return Err(missing("that path is not a directory"));
    }
    let nested = dir.join("package");
    if nested.join("package.json").is_file() {
        return Ok(nested);
    }
    if dir.join("package.json").is_file() {
        return Ok(dir.to_path_buf());
    }
    Err(missing(
        "neither it nor its package/ subdirectory carries a package.json",
    ))
}

/// Load the package at `dir`, memoised per resolved directory.
pub fn load(dir: &Path) -> Result<Arc<Package>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Package>>>> = OnceLock::new();
    let resolved = resolve_dir(dir)?;
    let key = std::fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().expect("package cache").get(&key).cloned() {
        return Ok(hit);
    }
    let loaded = Arc::new(read(&resolved)?);
    cache
        .lock()
        .expect("package cache")
        .insert(key, Arc::clone(&loaded));
    Ok(loaded)
}

fn read(dir: &Path) -> Result<Package> {
    let (digest, files_read, digest_byte_count) = content_digest(dir)?;
    let digest_file_count = files_read.len();

    let mut definitions = Vec::new();
    let mut search_parameters = Vec::new();
    for path in &files_read {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_definition = name.starts_with("StructureDefinition-") && name.ends_with(".json");
        let is_parameter = name.starts_with("SearchParameter-") && name.ends_with(".json");
        if !is_definition && !is_parameter {
            continue;
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| CodegenError::other(format!("reading {}: {e}", path.display())))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| CodegenError::other(format!("parsing {}: {e}", path.display())))?;
        match string_at(&value, "resourceType").unwrap_or_default() {
            "StructureDefinition" => definitions.push(definition(&value, path)?),
            "SearchParameter" => search_parameters.push(search_parameter(&value, path)?),
            other => {
                return Err(CodegenError::other(format!(
                    "{} declares resourceType {other:?}, which its file name contradicts",
                    path.display()
                )));
            }
        }
    }
    definitions.sort_by(|a, b| a.url.cmp(&b.url));
    search_parameters.sort_by(|a, b| a.url.cmp(&b.url));

    Ok(Package {
        dir: dir.to_path_buf(),
        digest,
        digest_file_count,
        digest_byte_count,
        files_read,
        definitions,
        search_parameters,
    })
}

/// The `structuredefinition-standards-status` extension url.
pub const STANDARDS_STATUS_EXTENSION: &str =
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-standards-status";

fn definition(value: &Value, path: &Path) -> Result<Definition> {
    let required = |field: &str| -> Result<String> {
        string_at(value, field).map(str::to_string).ok_or_else(|| {
            CodegenError::other(format!(
                "{} is a StructureDefinition with no {field}",
                path.display()
            ))
        })
    };
    let snapshot = value
        .get("snapshot")
        .and_then(|s| s.get("element"))
        .and_then(Value::as_array)
        .map(|elements| elements.iter().map(snapshot_element).collect())
        .unwrap_or_default();

    Ok(Definition {
        url: required("url")?,
        id: required("id")?,
        name: required("name")?,
        kind: required("kind")?,
        is_abstract: value
            .get("abstract")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        derivation: string_at(value, "derivation").map(str::to_string),
        base_definition: string_at(value, "baseDefinition").map(str::to_string),
        type_name: string_at(value, "type").map(str::to_string),
        fhir_version: string_at(value, "fhirVersion").map(str::to_string),
        standards_status: extension_value_code(value, STANDARDS_STATUS_EXTENSION),
        snapshot,
    })
}

fn snapshot_element(value: &Value) -> SnapshotElement {
    SnapshotElement {
        path: string_at(value, "path").unwrap_or_default().to_string(),
        min: value
            .get("min")
            .and_then(Value::as_u64)
            .map(|m| m.min(u32::MAX as u64) as u32),
        max: string_at(value, "max").map(str::to_string),
        types: value
            .get("type")
            .and_then(Value::as_array)
            .map(|types| {
                types
                    .iter()
                    .map(|t| string_at(t, "code").unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default(),
        content_reference: string_at(value, "contentReference").map(str::to_string),
        binding: value.get("binding").map(|b| ElementBinding {
            strength: string_at(b, "strength").unwrap_or_default().to_string(),
            value_set: string_at(b, "valueSet").unwrap_or_default().to_string(),
        }),
    }
}

fn search_parameter(value: &Value, path: &Path) -> Result<SearchParameter> {
    let required = |field: &str| -> Result<String> {
        string_at(value, field).map(str::to_string).ok_or_else(|| {
            CodegenError::other(format!(
                "{} is a SearchParameter with no {field}",
                path.display()
            ))
        })
    };
    Ok(SearchParameter {
        id: required("id")?,
        url: required("url")?,
        code: required("code")?,
        search_type: required("type")?,
        bases: string_array(value, "base"),
        expression: string_at(value, "expression").map(str::to_string),
        components: value
            .get("component")
            .and_then(Value::as_array)
            .map(|components| {
                components
                    .iter()
                    .map(|c| SearchParamComponent {
                        definition: string_at(c, "definition").unwrap_or_default().to_string(),
                        expression: string_at(c, "expression").unwrap_or_default().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        targets: string_array(value, "target"),
        multiple_or: value.get("multipleOr").and_then(Value::as_bool),
        multiple_and: value.get("multipleAnd").and_then(Value::as_bool),
        comparator: optional_string_array(value, "comparator"),
        modifier: optional_string_array(value, "modifier"),
        chain: optional_string_array(value, "chain"),
        standards_status: extension_value_code(value, STANDARDS_STATUS_EXTENSION),
    })
}

fn string_at<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn string_array(value: &Value, field: &str) -> Vec<String> {
    optional_string_array(value, field).unwrap_or_default()
}

fn optional_string_array(value: &Value, field: &str) -> Option<Vec<String>> {
    Some(
        value
            .get(field)?
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

fn extension_value_code(value: &Value, url: &str) -> Option<String> {
    value
        .get("extension")?
        .as_array()?
        .iter()
        .find(|e| string_at(e, "url") == Some(url))
        .and_then(|e| string_at(e, "valueCode"))
        .map(str::to_string)
}

/// Verify a loaded package against a digest, which is the recorded one unless
/// a run overrides it (`codegen.inputs.002`).
pub fn verify_digest(package: &Package, expected: &str) -> Result<()> {
    if package.digest == expected {
        return Ok(());
    }
    Err(CodegenError::failed(
        FailureKind::DigestMismatch,
        format!(
            "the input package at {} does not match the recorded digest: expected {expected}, \
             computed {} over {} files. Re-fetch it with `{FETCH_COMMAND}`, or correct the pin \
             in crates/frevo-codegen/packages/PINS.md if the package itself was re-published.",
            package.dir.display(),
            package.digest,
            package.digest_file_count
        ),
    ))
}

/// The package the recorded pin names, loaded and verified.
pub fn pinned() -> Result<Arc<Package>> {
    let package = load(&pin::default_package_dir())?;
    verify_digest(&package, pin::PACKAGE_CONTENT_SHA256)?;
    Ok(package)
}
