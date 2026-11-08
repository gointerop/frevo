//! Loading the suite's `inputfile` resources.
//!
//! `org.hl7.fhir.r4.test.FHIRPathTests` resolves an `inputfile` through
//! `TestingUtilities.loadTestResource("r4", input)`, which in the pinned
//! `fhir-test-cases` tree is the `r4/` directory. Here that is
//! `oracles/fhir-test-cases/fetched/r4/`, which is **fetched, not vendored**
//! (286 MB, `oracles/PINS.md`), so a missing tree is a `did-not-run` with a
//! message naming the fetch command — never a pass (`path.oracle.003`).
//!
//! Both file forms are loaded through this one reader (`path.oracle.004`): the
//! suite references 13 XML and 2 JSON inputs, and the engine under test is
//! handed the parsed document either way.

use std::path::{Path, PathBuf};

use super::json;
use super::suite::INPUT_DIR;
use super::xml;

/// The repository root, derived from the crate's manifest directory.
///
/// The literal in this source is repo-relative; the absolute prefix comes from
/// Cargo at compile time, so nothing here names a machine or a worktree.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

/// Resolve a repo-relative path such as `oracles/fhirpath/tests-fhir-r4.xml`.
pub fn repo_path(relative: &str) -> PathBuf {
    let mut p = repo_root();
    for part in relative.split('/') {
        p.push(part);
    }
    p
}

/// A loaded input file, in whichever of the two forms it was written.
#[derive(Debug, Clone)]
pub enum Document {
    Xml(xml::Document),
    Json(json::Value),
}

/// A loaded input resource, as the engine under test receives it.
#[derive(Debug, Clone)]
pub struct SuiteInput {
    /// The `inputfile` value, e.g. `patient-example.xml`.
    pub file: String,
    /// The file's bytes, unmodified.
    pub bytes: Vec<u8>,
    pub document: Document,
}

impl SuiteInput {
    /// `true` for an XML input, `false` for a JSON one.
    pub fn is_xml(&self) -> bool {
        matches!(self.document, Document::Xml(_))
    }

    /// The resource type: the root element name for XML, `resourceType` for
    /// JSON. This is what `FHIRPathTests` passes to `check` as both the root
    /// resource type and the resource type.
    pub fn resource_type(&self) -> Option<&str> {
        match &self.document {
            Document::Xml(d) => Some(d.root.name.as_str()),
            Document::Json(v) => v.get("resourceType").and_then(|v| v.as_str()),
        }
    }
}

/// Why an input could not be loaded. Every variant produces `did-not-run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// The fetched tree is not present at all.
    DirectoryMissing(String),
    /// The tree is present but the named file is not.
    FileMissing(String),
    NotUtf8(String),
    Malformed(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::DirectoryMissing(d) => write!(
                f,
                "the suite's input directory {d} is not present. It is fetched, not vendored: \
                 run `bash oracles/fetch.sh`. Until then every case with an inputfile reports \
                 did-not-run, which is not a pass."
            ),
            InputError::FileMissing(p) => write!(f, "input file {p} is missing"),
            InputError::NotUtf8(p) => write!(f, "input file {p} is not valid UTF-8"),
            InputError::Malformed(m) => write!(f, "malformed input: {m}"),
        }
    }
}

/// Load one `inputfile` value.
pub fn load(file: &str) -> Result<SuiteInput, InputError> {
    let dir = repo_path(INPUT_DIR);
    if !dir.is_dir() {
        return Err(InputError::DirectoryMissing(INPUT_DIR.to_string()));
    }
    let path = dir.join(file);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Err(InputError::FileMissing(format!("{INPUT_DIR}/{file}"))),
    };
    let text = String::from_utf8(bytes.clone())
        .map_err(|_| InputError::NotUtf8(format!("{INPUT_DIR}/{file}")))?;

    // `FHIRPathTests` switches on the `.json` suffix and treats everything
    // else as XML; the harness does the same rather than sniffing content, so
    // a mis-named file is a visible failure.
    let document = if file.ends_with(".json") {
        Document::Json(
            json::parse(&text).map_err(|e| InputError::Malformed(format!("{file}: {e}")))?,
        )
    } else {
        Document::Xml(xml::parse(&text).map_err(|e| InputError::Malformed(format!("{file}: {e}")))?)
    };

    Ok(SuiteInput {
        file: file.to_string(),
        bytes,
        document,
    })
}
