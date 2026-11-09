//! Element paths: `Patient`, `Patient.meta.versionId`,
//! `Bundle.entry[0].resource.id`.
//!
//! This is the addressing the Phase C oracles drive the codec through. It is
//! not FHIRPath: there is no filtering, no function call and no choice-type
//! spelling, and the first step is always the resource type.

use super::scan::DecodeError;

/// One step of an element path: a name and, for a repeating element, a
/// repetition index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStep {
    pub name: String,
    pub index: Option<usize>,
}

/// Split an element path into its steps.
pub fn parse_path(path: &str) -> Result<Vec<PathStep>, DecodeError> {
    if path.is_empty() {
        return Err(DecodeError::new(path, 0, "an element path cannot be empty"));
    }
    let mut out = Vec::new();
    for raw in path.split('.') {
        let step = match raw.split_once('[') {
            None => PathStep {
                name: raw.to_string(),
                index: None,
            },
            Some((name, rest)) => {
                let digits = rest.strip_suffix(']').ok_or_else(|| {
                    DecodeError::new(path, 0, format!("{raw:?} is missing its closing ']'"))
                })?;
                let index = digits.parse::<usize>().map_err(|_| {
                    DecodeError::new(path, 0, format!("{raw:?} has no repetition index"))
                })?;
                PathStep {
                    name: name.to_string(),
                    index: Some(index),
                }
            }
        };
        if step.name.is_empty() {
            return Err(DecodeError::new(
                path,
                0,
                "an element path step cannot be empty",
            ));
        }
        out.push(step);
    }
    Ok(out)
}

/// Append one step to a path, with its index when the element repeats.
pub fn join(prefix: &str, name: &str, index: Option<usize>) -> String {
    match index {
        Some(index) => format!("{prefix}.{name}[{index}]"),
        None => format!("{prefix}.{name}"),
    }
}
