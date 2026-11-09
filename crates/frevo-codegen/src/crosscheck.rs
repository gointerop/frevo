//! The `reference/`-conditional cross-check (`codegen.inputs.005`).
//!
//! The pinned package is the generator's input; HAPI's copy of the same
//! definitions is the cross-check. This module compares them, so that a drift
//! between what the registry publishes and what the reference implementation
//! actually validates against is a test failure rather than a silent parity
//! break.
//!
//! Three things are compared, and the first of them is the reason this is not
//! a one-line set difference. The package ships 658 `StructureDefinition`s;
//! HAPI's `profile/*.xml` carry 256. The missing 402 are the R4 extension
//! registry, which HAPI keeps in `extension/extension-definitions.xml`, plus
//! four derivation-less logical models. So the package side is scoped first,
//! by the rule [`crate::inventory`] documents, and the report states the scope
//! it used rather than leaving a reader to assume one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{CodegenError, Result};
use crate::inventory::{self, CORE_SCOPE_DESCRIPTION};
use crate::package::{Definition, Package, SnapshotElement};
use crate::xml;

/// The three files the scenario names, in the order it names them.
pub const HAPI_PROFILE_FILES: [&str; 3] = [
    "profiles-types.xml",
    "profiles-resources.xml",
    "profiles-others.xml",
];

/// The SHA the cross-check is pinned to, recorded in `reference/PINS.md`.
pub const HAPI_PIN_SHA: &str = "7ab4c7c7c49281496faa1b3c2498a652457669ae";

/// One element the two sides disagree about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDisagreement {
    pub url: String,
    pub path: String,
    pub detail: String,
}

/// What the cross-check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossCheckReport {
    /// What the package side was scoped to.
    pub scope: String,
    pub package_fhir_version: String,
    pub hapi_fhir_version: String,
    pub hapi_definition_count: usize,
    pub only_in_package: Vec<String>,
    pub only_in_hapi: Vec<String>,
    pub element_disagreements: Vec<ElementDisagreement>,
    /// Every file the cross-check opened, so the comparison's inputs are
    /// stated rather than implied.
    pub files_read: Vec<PathBuf>,
}

/// One definition as HAPI's XML declares it.
#[derive(Debug, Clone, Default)]
struct XmlDefinition {
    url: String,
    fhir_version: Option<String>,
    snapshot: Vec<SnapshotElement>,
}

/// Compare the pinned package with HAPI's copy.
///
/// `hapi_profile_dir` is the directory holding the three `profiles-*.xml`
/// files. A missing directory or file is an error naming what to clone, never
/// a silent pass.
pub fn run(package: &Package, hapi_profile_dir: &Path) -> Result<CrossCheckReport> {
    let mut hapi: BTreeMap<String, XmlDefinition> = BTreeMap::new();
    let mut files_read = Vec::new();
    let mut hapi_versions: Vec<String> = Vec::new();

    for file in HAPI_PROFILE_FILES {
        let path = hapi_profile_dir.join(file);
        let bytes = std::fs::read(&path).map_err(|e| {
            CodegenError::other(format!(
                "the cross-check needs {} ({e}). It is cloned read-only at {HAPI_PIN_SHA}: \
                 run `bash reference/clone.sh`.",
                path.display()
            ))
        })?;
        files_read.push(path.clone());
        for definition in read_bundle(&bytes, &path)? {
            if let Some(version) = &definition.fhir_version {
                hapi_versions.push(version.clone());
            }
            if let Some(previous) = hapi.insert(definition.url.clone(), definition) {
                return Err(CodegenError::other(format!(
                    "{} declares {} twice",
                    path.display(),
                    previous.url
                )));
            }
        }
    }

    let scoped: BTreeMap<&str, &Definition> = inventory::core_definitions(package)
        .into_iter()
        .map(|d| (d.url.as_str(), d))
        .collect();

    let package_versions: Vec<String> = scoped
        .values()
        .filter_map(|d| d.fhir_version.clone())
        .collect();

    let only_in_package: Vec<String> = scoped
        .keys()
        .filter(|url| !hapi.contains_key(**url))
        .map(|url| (*url).to_string())
        .collect();
    let only_in_hapi: Vec<String> = hapi
        .keys()
        .filter(|url| !scoped.contains_key(url.as_str()))
        .cloned()
        .collect();

    let mut element_disagreements = Vec::new();
    for (url, hapi_definition) in &hapi {
        let Some(package_definition) = scoped.get(url.as_str()) else {
            continue;
        };
        compare_snapshots(
            url,
            &package_definition.snapshot,
            &hapi_definition.snapshot,
            &mut element_disagreements,
        );
    }

    Ok(CrossCheckReport {
        scope: format!(
            "{CORE_SCOPE_DESCRIPTION} Scoped that way the package contributes {} of its {} \
             StructureDefinitions.",
            scoped.len(),
            package.definitions.len()
        ),
        package_fhir_version: single_version(&package_versions),
        hapi_fhir_version: single_version(&hapi_versions),
        hapi_definition_count: hapi.len(),
        only_in_package,
        only_in_hapi,
        element_disagreements,
        files_read,
    })
}

/// The one `fhirVersion` a side declares, or a description of the several it
/// declares — which then fails the scenario's equality assertion rather than
/// hiding behind a first-wins pick.
fn single_version(versions: &[String]) -> String {
    let mut distinct: Vec<&str> = versions.iter().map(String::as_str).collect();
    distinct.sort_unstable();
    distinct.dedup();
    match distinct.as_slice() {
        [] => "none declared".to_string(),
        [one] => (*one).to_string(),
        several => format!("several: {}", several.join(", ")),
    }
}

/// Compare the snapshots' element names, types and cardinalities, in order.
fn compare_snapshots(
    url: &str,
    package: &[SnapshotElement],
    hapi: &[SnapshotElement],
    out: &mut Vec<ElementDisagreement>,
) {
    for (index, (left, right)) in package.iter().zip(hapi).enumerate() {
        if left.path != right.path {
            out.push(ElementDisagreement {
                url: url.to_string(),
                path: left.path.clone(),
                detail: format!(
                    "snapshot element {index} is {:?} in the package and {:?} in the HAPI copy",
                    left.path, right.path
                ),
            });
            continue;
        }
        if left.min != right.min || left.max != right.max {
            out.push(ElementDisagreement {
                url: url.to_string(),
                path: left.path.clone(),
                detail: format!(
                    "cardinality {:?}..{:?} in the package, {:?}..{:?} in the HAPI copy",
                    left.min, left.max, right.min, right.max
                ),
            });
        }
        if left.types != right.types {
            out.push(ElementDisagreement {
                url: url.to_string(),
                path: left.path.clone(),
                detail: format!(
                    "types {:?} in the package, {:?} in the HAPI copy",
                    left.types, right.types
                ),
            });
        }
    }
    if package.len() != hapi.len() {
        let longer = if package.len() > hapi.len() {
            &package[hapi.len()..]
        } else {
            &hapi[package.len()..]
        };
        out.push(ElementDisagreement {
            url: url.to_string(),
            path: longer
                .first()
                .map(|e| e.path.clone())
                .unwrap_or_else(|| url.to_string()),
            detail: format!(
                "the snapshot has {} elements in the package and {} in the HAPI copy",
                package.len(),
                hapi.len()
            ),
        });
    }
}

/// Every `StructureDefinition` in one FHIR XML bundle.
fn read_bundle(bytes: &[u8], path: &Path) -> Result<Vec<XmlDefinition>> {
    let mut reader = xml::Reader::new(bytes);
    let mut stack: Vec<String> = Vec::new();
    let mut definition_depth: Option<usize> = None;
    let mut current = XmlDefinition::default();
    let mut out = Vec::new();

    loop {
        let event = reader
            .next_event()
            .map_err(|e| CodegenError::other(format!("reading {}: {e}", path.display())))?;
        let Some(event) = event else { break };
        match event {
            xml::Event::Start { ref name, .. } => {
                stack.push(name.clone());
                if definition_depth.is_none() {
                    if name == "StructureDefinition" {
                        definition_depth = Some(stack.len() - 1);
                        current = XmlDefinition::default();
                    }
                    continue;
                }
                let depth = definition_depth.expect("checked above");
                absorb(&mut current, &stack[depth + 1..], &event);
            }
            xml::Event::End { .. } => {
                if definition_depth == Some(stack.len().saturating_sub(1)) {
                    definition_depth = None;
                    out.push(std::mem::take(&mut current));
                }
                stack.pop();
            }
        }
    }
    Ok(out)
}

/// Fold one start event into the definition being read, addressed by its path
/// below the `StructureDefinition` element.
///
/// Addressing by relative path rather than by element name is what keeps
/// `snapshot.element.slicing.discriminator.path` from being mistaken for
/// `snapshot.element.path`, and keeps `differential` out of the comparison
/// altogether.
fn absorb(definition: &mut XmlDefinition, relative: &[String], event: &xml::Event) {
    let at = |index: usize| -> &str { relative.get(index).map(String::as_str).unwrap_or_default() };
    let value = || event.attribute("value").unwrap_or_default().to_string();

    match (relative.len(), at(0), at(1), at(2)) {
        (1, "url", _, _) => definition.url = value(),
        (1, "fhirVersion", _, _) => definition.fhir_version = Some(value()),
        (2, "snapshot", "element", _) => definition.snapshot.push(SnapshotElement {
            path: String::new(),
            min: None,
            max: None,
            types: Vec::new(),
            content_reference: None,
            binding: None,
        }),
        (3, "snapshot", "element", field) => {
            let Some(element) = definition.snapshot.last_mut() else {
                return;
            };
            match field {
                "path" => element.path = value(),
                "min" => element.min = value().parse().ok(),
                "max" => element.max = Some(value()),
                "contentReference" => element.content_reference = Some(value()),
                "type" => element.types.push(String::new()),
                _ => {}
            }
        }
        (4, "snapshot", "element", "type") if at(3) == "code" => {
            if let Some(code) = definition
                .snapshot
                .last_mut()
                .and_then(|e| e.types.last_mut())
            {
                *code = value();
            }
        }
        _ => {}
    }
}
