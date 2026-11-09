//! Classifying the package's `StructureDefinition`s, and asserting the input
//! inventory before anything is generated (`codegen.inputs.006`).
//!
//! # Why a scoping rule is needed at all
//!
//! The counts the specification states — 63 datatype definitions, 149 resource
//! definitions, 1,375 SearchParameters — were read from HAPI's copy of the R4
//! definitions, which is split across
//! `profile/{profiles-types,profiles-resources,profiles-others}.xml` and
//! `sp/search-parameters.json`. The published package is not split that way: it
//! ships 658 `StructureDefinition`s and 1,400 `SearchParameter`s in one flat
//! directory, because it also carries the R4 extension registry, four logical
//! models that are documentation rather than definitions, and a handful of
//! example parameters.
//!
//! So the package side has to be scoped before any count can be compared. Two
//! facts about the pinned package, both verified against the HAPI copy at
//! `7ab4c7c7c49281496faa1b3c2498a652457669ae`, make that exact rather than
//! approximate:
//!
//! 1. **The specification stamps its own artifacts.** Every definition HAPI
//!    carries in `profile/*.xml` carries the
//!    `structuredefinition-standards-status` extension; no extension-registry
//!    definition does. Of the 658, exactly 260 carry it, and the four that
//!    carry it without being in the HAPI set are the logical models
//!    `Definition`, `Event`, `FiveWs` and `Request` — the only four core
//!    definitions that declare neither a `derivation` nor `abstract = true`.
//!    Requiring a `derivation` or `abstract` therefore leaves exactly the 256
//!    URLs HAPI has, and `codegen.inputs.005` asserts that set equality.
//!
//! 2. **A type definition names itself.** Within those 256, `id == name` holds
//!    for the 212 definitions that *are* a type — the 63 datatypes and the 149
//!    resource definitions — and fails for all 44 that are a profile on one
//!    (`elementdefinition-de` is named "DataElement constraint on
//!    ElementDefinition data type"; `shareablevalueset` is named "Shareable
//!    ValueSet"). Those 44 are precisely `profiles-others.xml`.
//!
//! Neither rule is invented here: both were measured across the whole package
//! and the whole HAPI copy, and both split it with no remainder. The three
//! derivation shapes the 212 take are recorded on
//! [`Classification::of`], so a future package whose shape differs fails the
//! inventory assertion rather than silently generating a different model.

use crate::error::{CodegenError, FailureKind, Result};
use crate::package::{Definition, Package};

/// `profiles-types.xml`: 20 `primitive-type` plus 43 `complex-type`.
pub const EXPECTED_DATATYPE_DEFINITIONS: usize = 63;
pub const EXPECTED_PRIMITIVE_DATATYPES: usize = 20;
pub const EXPECTED_COMPLEX_DATATYPES: usize = 43;
/// `profiles-resources.xml`, every definition in the bundle.
pub const EXPECTED_RESOURCE_DEFINITIONS: usize = 149;
pub const EXPECTED_CONCRETE_RESOURCES: usize = 146;
pub const EXPECTED_ABSTRACT_RESOURCES: usize = 2;
pub const EXPECTED_LOGICAL_RESOURCES: usize = 1;
/// `profiles-others.xml`: the constraint profiles on a core type.
pub const EXPECTED_PROFILE_DEFINITIONS: usize = 44;
/// `sp/search-parameters.json`, counted by bundle entry.
pub const EXPECTED_SEARCH_PARAMETERS: usize = 1375;
/// The three profile files together, which is the cross-check's basis.
pub const EXPECTED_CORE_DEFINITIONS: usize = 256;

/// What the run states, and what it asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inventory {
    pub datatype_definitions: usize,
    pub resource_definitions: usize,
    pub concrete_resources: usize,
    pub abstract_resources: usize,
    pub logical_resources: usize,
    pub search_parameters: usize,
}

impl Inventory {
    /// The pinned R4 inventory: the counts `codegen.inputs.006` names.
    pub const PINNED_R4: Inventory = Inventory {
        datatype_definitions: EXPECTED_DATATYPE_DEFINITIONS,
        resource_definitions: EXPECTED_RESOURCE_DEFINITIONS,
        concrete_resources: EXPECTED_CONCRETE_RESOURCES,
        abstract_resources: EXPECTED_ABSTRACT_RESOURCES,
        logical_resources: EXPECTED_LOGICAL_RESOURCES,
        search_parameters: EXPECTED_SEARCH_PARAMETERS,
    };
}

/// What one `StructureDefinition` in the package is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Classification {
    /// One of the 20 primitive datatypes.
    PrimitiveDatatype,
    /// One of the 43 complex datatypes.
    ComplexDatatype,
    /// One of the 146 concrete resources.
    ConcreteResource,
    /// `Resource` or `DomainResource`.
    AbstractResource,
    /// `MetadataResource`, the one logical definition.
    LogicalResource,
    /// One of the 44 constraint profiles: `profiles-others.xml`.
    Profile,
    /// An extension definition, a logical model, or anything else the package
    /// ships that the specification's own profile bundles do not.
    OutOfScope,
}

impl Classification {
    /// Classify one definition.
    ///
    /// The derivation shapes this accepts, measured over the pinned package:
    ///
    /// | kind | abstract | derivation | count | classification |
    /// |---|---|---|---|---|
    /// | `primitive-type` | no | `specialization` | 20 | primitive datatype |
    /// | `complex-type` | no | `specialization` | 39 | complex datatype |
    /// | `complex-type` | yes | `specialization` | 1 (`BackboneElement`) | complex datatype |
    /// | `complex-type` | yes | — | 1 (`Element`) | complex datatype |
    /// | `complex-type` | no | `constraint` | 2 (`MoneyQuantity`, `SimpleQuantity`) | complex datatype |
    /// | `resource` | no | `specialization` | 146 | concrete resource |
    /// | `resource` | yes | — | 1 (`Resource`) | abstract resource |
    /// | `resource` | yes | `specialization` | 1 (`DomainResource`) | abstract resource |
    /// | `logical` | no | `specialization` | 1 (`MetadataResource`) | logical |
    pub fn of(definition: &Definition) -> Classification {
        if !in_core_scope(definition) {
            return Classification::OutOfScope;
        }
        if !names_itself(definition) {
            return Classification::Profile;
        }
        match definition.kind.as_str() {
            "primitive-type" => Classification::PrimitiveDatatype,
            "complex-type" => Classification::ComplexDatatype,
            "resource" if definition.is_abstract => Classification::AbstractResource,
            "resource" => Classification::ConcreteResource,
            "logical" => Classification::LogicalResource,
            _ => Classification::OutOfScope,
        }
    }

    pub fn is_datatype(self) -> bool {
        matches!(
            self,
            Classification::PrimitiveDatatype | Classification::ComplexDatatype
        )
    }

    pub fn is_resource_definition(self) -> bool {
        matches!(
            self,
            Classification::ConcreteResource
                | Classification::AbstractResource
                | Classification::LogicalResource
        )
    }

    /// Whether the definition is in the basis `codegen.inputs.005` compares:
    /// the three `profile/*.xml` bundles.
    pub fn is_core(self) -> bool {
        self != Classification::OutOfScope
    }
}

/// Rule 1: the specification stamps its own artifacts, and every one of them
/// states a derivation or is abstract.
pub fn in_core_scope(definition: &Definition) -> bool {
    definition.has_standards_status() && (definition.derivation.is_some() || definition.is_abstract)
}

/// Rule 2: a type definition names itself; a profile on a type does not.
pub fn names_itself(definition: &Definition) -> bool {
    definition.id == definition.name
}

/// The prose a cross-check report states for the package side, so the basis of
/// the comparison is never assumed (`codegen.inputs.005`).
pub const CORE_SCOPE_DESCRIPTION: &str = concat!(
    "the StructureDefinitions of the pinned package that carry the ",
    "structuredefinition-standards-status extension and declare either a derivation or ",
    "abstract=true -- the basis of ",
    "profile/{profiles-types,profiles-resources,profiles-others}.xml. The package's ",
    "extension-registry definitions, which HAPI keeps in ",
    "extension/extension-definitions.xml, and its four derivation-less logical models ",
    "(Definition, Event, FiveWs, Request) are outside that basis and excluded."
);

/// Which search parameters are the specification's own.
///
/// The same stamp as rule 1 above: all 1,375 parameters in HAPI's
/// `sp/search-parameters.json` carry `structuredefinition-standards-status`,
/// and none of the 25 the package additionally ships — five extension-registry
/// packs and the four `example`/`filter` parameters — carries it.
pub const SEARCH_PARAMETER_SCOPE_DESCRIPTION: &str = concat!(
    "the SearchParameters of the pinned package that carry the ",
    "structuredefinition-standards-status extension -- the basis of ",
    "sp/search-parameters.json. The 25 extension-registry and example parameters the ",
    "package additionally ships are outside that basis and excluded."
);

/// Count the package's inputs.
pub fn of(package: &Package) -> Inventory {
    let mut inventory = Inventory {
        datatype_definitions: 0,
        resource_definitions: 0,
        concrete_resources: 0,
        abstract_resources: 0,
        logical_resources: 0,
        search_parameters: 0,
    };
    for definition in &package.definitions {
        match Classification::of(definition) {
            Classification::PrimitiveDatatype | Classification::ComplexDatatype => {
                inventory.datatype_definitions += 1;
            }
            Classification::ConcreteResource => {
                inventory.concrete_resources += 1;
                inventory.resource_definitions += 1;
            }
            Classification::AbstractResource => {
                inventory.abstract_resources += 1;
                inventory.resource_definitions += 1;
            }
            Classification::LogicalResource => {
                inventory.logical_resources += 1;
                inventory.resource_definitions += 1;
            }
            Classification::Profile | Classification::OutOfScope => {}
        }
    }
    inventory.search_parameters = package
        .search_parameters
        .iter()
        .filter(|p| p.has_standards_status())
        .count();
    inventory
}

/// Every definition in the cross-check's basis, in canonical-URL order.
pub fn core_definitions(package: &Package) -> Vec<&Definition> {
    package
        .definitions
        .iter()
        .filter(|d| in_core_scope(d))
        .collect()
}

/// Fail the run when the input set is not the pinned R4 inventory, rather than
/// silently generating a different model (`codegen.inputs.006`).
pub fn assert_pinned(found: Inventory) -> Result<()> {
    let expected = Inventory::PINNED_R4;
    if found == expected {
        return Ok(());
    }
    let mut differences = Vec::new();
    let mut note = |label: &str, got: usize, want: usize| {
        if got != want {
            differences.push(format!("{label}: expected {want}, found {got}"));
        }
    };
    note(
        "datatype StructureDefinitions",
        found.datatype_definitions,
        expected.datatype_definitions,
    );
    note(
        "resource StructureDefinitions",
        found.resource_definitions,
        expected.resource_definitions,
    );
    note(
        "concrete resources",
        found.concrete_resources,
        expected.concrete_resources,
    );
    note(
        "abstract resources",
        found.abstract_resources,
        expected.abstract_resources,
    );
    note(
        "logical resources",
        found.logical_resources,
        expected.logical_resources,
    );
    note(
        "SearchParameter resources",
        found.search_parameters,
        expected.search_parameters,
    );
    Err(CodegenError::failed(
        FailureKind::InventoryMismatch,
        format!(
            "the input set is not the pinned R4 inventory, so no model was generated. \
             The pinned inventory is {} datatype StructureDefinitions, {} resource \
             StructureDefinitions ({} concrete, {} abstract, {} logical) and {} \
             SearchParameter resources. Differences: {}.",
            expected.datatype_definitions,
            expected.resource_definitions,
            expected.concrete_resources,
            expected.abstract_resources,
            expected.logical_resources,
            expected.search_parameters,
            differences.join("; ")
        ),
    ))
}
