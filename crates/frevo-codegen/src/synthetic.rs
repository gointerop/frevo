//! Definitions written by a caller rather than read from the package.
//!
//! Used only where the pinned R4 core package contains no instance of a
//! condition a scenario describes — a name collision, a `0` upper bound on an
//! element that also has a type, an unresolvable required binding. Keeping
//! them in their own input shape means the generator never has to accept a
//! half-built `StructureDefinition`, and means a synthetic run cannot be
//! mistaken for a run over the real package: the inventory assertion is opt-in
//! per request, and a synthetic set that opts in and is not the pinned R4
//! inventory fails (`codegen.inputs.006`).

use std::path::{Path, PathBuf};

use crate::inventory::Inventory;

/// What kind of definition a synthetic input declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    PrimitiveType,
    ComplexType,
    Resource,
    Logical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub strength: String,
    pub value_set: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub path: String,
    pub min: u32,
    /// The snapshot's literal `max`: `"0"`, `"1"` or `"*"`.
    pub max: String,
    pub types: Vec<String>,
    pub content_reference: Option<String>,
    pub binding: Option<Binding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub url: String,
    pub name: String,
    pub kind: Kind,
    pub base: Option<String>,
    pub is_abstract: bool,
    pub derivation: Option<String>,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSet {
    pub url: String,
    pub codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub definitions: Vec<Definition>,
    pub value_sets: Vec<ValueSet>,
    pub out_dir: PathBuf,
    /// Whether the run must assert the pinned R4 inventory before generating.
    pub assert_pinned_inventory: bool,
}

impl Request {
    /// A synthetic run that does not assert the pinned inventory.
    pub fn new(definitions: Vec<Definition>, value_sets: Vec<ValueSet>, out_dir: &Path) -> Self {
        Request {
            definitions,
            value_sets,
            out_dir: out_dir.to_path_buf(),
            assert_pinned_inventory: false,
        }
    }
}

/// Count a synthetic definition set the way [`crate::inventory::of`] counts the
/// package.
///
/// There is no `structuredefinition-standards-status` stamp to scope by here:
/// a synthetic set is, by construction, exactly the definitions the caller
/// wrote. So the classification reduces to `kind` and `abstract`, and a
/// synthetic set declares no SearchParameters at all — which is why opting in
/// to the inventory assertion with anything short of the real package fails on
/// several counts at once, and why the refusal names each of them.
pub fn inventory_of(definitions: &[Definition]) -> Inventory {
    let mut inventory = Inventory {
        datatype_definitions: 0,
        resource_definitions: 0,
        concrete_resources: 0,
        abstract_resources: 0,
        logical_resources: 0,
        search_parameters: 0,
    };
    for definition in definitions {
        match definition.kind {
            Kind::PrimitiveType | Kind::ComplexType => inventory.datatype_definitions += 1,
            Kind::Resource if definition.is_abstract => {
                inventory.abstract_resources += 1;
                inventory.resource_definitions += 1;
            }
            Kind::Resource => {
                inventory.concrete_resources += 1;
                inventory.resource_definitions += 1;
            }
            Kind::Logical => {
                inventory.logical_resources += 1;
                inventory.resource_definitions += 1;
            }
        }
    }
    inventory
}
