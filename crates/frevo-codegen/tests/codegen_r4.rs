//! The `codegen-r4` oracle: every `codegen.*` scenario of
//! `add-fhirpath-and-codegen-r4`, driven through one seam.
//!
//! Task 5 of the change. Run it with:
//!
//! ```text
//! cargo test -p frevo-codegen --test codegen_r4
//! ```
//!
//! # Why one file and one seam
//!
//! `frevo-codegen` is a generator: its observable surface is a command that
//! verifies an input package, writes files, and reports what it found. Every
//! test below drives real inputs — the pinned package, the HAPI cross-check
//! tree, the interim example set, hand-written synthetic StructureDefinitions —
//! through [`generator_under_test`], which is bound to the real surfaces: the
//! generator through `frevo_codegen`'s `BoundGenerator`, and the codec and the
//! FHIRPath parser through `frevo-model` and `frevo-path` (see
//! [`OracleGenerator`]).
//!
//! [`CodegenError::NotImplemented`] remains an **error value, not a verdict**.
//! It can never be read as a pass, including on the scenarios that require the
//! generator to *refuse* (`codegen.inputs.002`, `codegen.inputs.003`,
//! `codegen.inputs.006`, `codegen.naming.003`): those tests assert the
//! refusal's *kind* and the content of its message, so a generator that
//! produced nothing at all fails them.
//!
//! # Provenance
//!
//! Every count and every element fact asserted below was read out of the pinned
//! HAPI copy of the R4 definitions at SHA
//! `7ab4c7c7c49281496faa1b3c2498a652457669ae`:
//!
//! - `hapi-fhir-validation-resources-r4/.../r4/model/profile/profiles-types.xml`
//!   — 63 datatype definitions, 20 `primitive-type` and 43 `complex-type`;
//! - `.../profile/profiles-resources.xml` — 149 definitions: 146
//!   `resource`/`abstract=false`/`specialization`, 2 abstract (`Resource`,
//!   `DomainResource`) and 1 `logical` (`MetadataResource`);
//! - `.../profile/profiles-others.xml` — 44 `constraint` profiles;
//! - `.../sp/search-parameters.json` — 1,375 `SearchParameter` resources.
//!
//! Parser behaviour is cited against
//! `hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/` at the same
//! SHA, and HAPI's own JUnit class is named where one covers the case.
//!
//! # The decode scenarios follow HAPI's parser
//!
//! Three conflicts this file once flagged between the delta and HAPI were
//! resolved in the spec on 2026-11-20 (`tasks/blockers.md`, "`codegen_r4.rs`
//! seam bound; six reds left"), and the tests now assert HAPI's behaviour as
//! the approved delta restates it:
//!
//! 1. `codegen.cardinality.002` and `.004`: no minimum-cardinality check on
//!    decode, in either mode, and no diagnostic.
//!    `IParserErrorHandler.missingRequiredElement`'s four call sites in
//!    `JsonParser.java` (`:1503`, `:1514`, `:1995`, `:2004`) are all about
//!    `Extension.url` or `Extension.value`.
//! 2. `codegen.choice.004`: strict mode fails at the second `value[x]`
//!    property with `HAPI-1823` naming `value`, the choice field name
//!    (`ParserState.java:660-672`); lenient mode keeps the first and drops the
//!    second with a repeated-element diagnostic.
//! 3. `codegen.binding.002`: a code outside `Patient.gender`'s enumeration is
//!    `HAPI-1821` in **both** default modes, because
//!    `LenientErrorHandler.invalidValue` hands a non-blank invalid value to the
//!    strict handler (`LenientErrorHandler.java:100-108`). HAPI's
//!    `CaseInsensitiveEnumParsingR4Test` keeps the raw code only because it
//!    installs a mock error handler.
//!
//! A decode failure that carries HAPI's message code reaches these tests as
//! `HAPI-<code>: <message>`, which is how `Msg.code` renders it, so the
//! approved delta's message texts are asserted exactly.
//!
//! Scenario coverage: all 54 `codegen.*` scenarios, the five `codegen.ancestry.*`
//! ones through their own seam, [`class_table_under_test`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// Provenance constants
// ---------------------------------------------------------------------------

/// The one input package the generator may read (`design.md` section 2).
pub const PACKAGE_NAME: &str = "hl7.fhir.r4.core";
pub const PACKAGE_VERSION: &str = "4.0.1";
/// `StructureDefinition.fhirVersion` in both the package and the HAPI copy.
pub const FHIR_VERSION: &str = "4.0.1";
/// The `--version` selector on the command line.
pub const VERSION_SELECTOR: &str = "r4";
pub const GENERATOR_NAME: &str = "frevo-codegen";

/// `profiles-types.xml`: 20 `primitive-type` plus 43 `complex-type`.
pub const EXPECTED_DATATYPE_DEFINITIONS: usize = 63;
/// `profiles-resources.xml`, every `StructureDefinition` in the bundle.
pub const EXPECTED_RESOURCE_DEFINITIONS: usize = 149;
pub const EXPECTED_CONCRETE_RESOURCES: usize = 146;
pub const EXPECTED_ABSTRACT_RESOURCES: usize = 2;
pub const EXPECTED_LOGICAL_RESOURCES: usize = 1;
/// `sp/search-parameters.json`, counted by bundle entry.
pub const EXPECTED_SEARCH_PARAMETERS: usize = 1375;
/// `profiles-types.xml`, `kind = complex-type`.
pub const EXPECTED_COMPLEX_DATATYPES: usize = 43;
/// Of those, the ones with `derivation = specialization`. The other three are
/// `Element`, which is the abstract root and declares no `derivation`, and
/// `MoneyQuantity` and `SimpleQuantity`, which are `constraint` profiles on
/// `Quantity`.
pub const EXPECTED_COMPLEX_SPECIALIZATIONS: usize = 40;

/// The HAPI cross-check tree (`codegen.inputs.005`), repo-relative.
pub const HAPI_PROFILE_DIR: &str = "reference/hapi-fhir/hapi-fhir-validation-resources-r4/src/main/resources/org/hl7/fhir/r4/model/profile";
/// The SHA the cross-check is pinned to.
pub const HAPI_PIN_SHA: &str = "7ab4c7c7c49281496faa1b3c2498a652457669ae";
/// `profiles-types.xml` 63 + `profiles-resources.xml` 149 +
/// `profiles-others.xml` 44. The 393 extension definitions the package also
/// ships live in `extension/extension-definitions.xml`, outside the delta's
/// `profile/*.xml` comparison basis, so the package side must be scoped to
/// these before the two URL sets can be identical.
pub const HAPI_PROFILE_DEFINITIONS: usize = 256;

/// The interim round-trip selector named by `tasks.md` task 5.
pub const EXAMPLE_DIR: &str = "oracles/fhir-test-cases/fetched/r4/examples";
/// `*.json` in that directory, every one of which carries a `resourceType`.
pub const EXPECTED_JSON_EXAMPLES: usize = 72;
/// `*.xml` in that directory.
pub const EXPECTED_XML_EXAMPLES: usize = 10;
/// `*.json.out` in that directory: sql-on-FHIR expected outputs, which carry no
/// `resourceType` and are not examples. `tasks.md`'s "103 examples" counts
/// them; this oracle excludes them and says so (`codegen.roundtrip.001`).
pub const EXPECTED_NON_EXAMPLE_FIXTURES: usize = 21;
/// Distinct `resourceType` values across the 72 JSON examples.
pub const EXPECTED_COVERED_RESOURCE_TYPES: usize = 69;

/// Where the generated output is checked in (`design.md` section 9).
pub const GENERATED_OUT_DIR: &str = "crates/frevo-model/src/r4";
/// The crate that must declare `#![forbid(unsafe_code)]`
/// (`codegen.output.006`).
pub const MODEL_CRATE_ROOT: &str = "crates/frevo-model/src/lib.rs";

/// The 20 R4 primitive datatypes, in the order `codegen.types.001` lists them,
/// which is `profiles-types.xml` order.
pub const R4_PRIMITIVES: [&str; 20] = [
    "base64Binary",
    "boolean",
    "canonical",
    "code",
    "date",
    "dateTime",
    "decimal",
    "id",
    "instant",
    "integer",
    "markdown",
    "oid",
    "positiveInt",
    "string",
    "time",
    "unsignedInt",
    "uri",
    "url",
    "uuid",
    "xhtml",
];

/// The complex datatypes `codegen.types.002` names explicitly.
pub const NAMED_COMPLEX_DATATYPES: [&str; 22] = [
    "Address",
    "Age",
    "Annotation",
    "Attachment",
    "CodeableConcept",
    "Coding",
    "ContactPoint",
    "Count",
    "Distance",
    "Dosage",
    "Duration",
    "HumanName",
    "Identifier",
    "Money",
    "Period",
    "Quantity",
    "Range",
    "Ratio",
    "Reference",
    "SampledData",
    "Signature",
    "Timing",
];

/// Every `complex-type` with `derivation = specialization` in
/// `profiles-types.xml`. `codegen.types.002` says a type exists for each
/// complex datatype the file declares; these are the 40 that are datatypes in
/// their own right rather than the abstract root or a profile on one.
pub const ALL_COMPLEX_SPECIALIZATIONS: [&str; 40] = [
    "Address",
    "Age",
    "Annotation",
    "Attachment",
    "BackboneElement",
    "CodeableConcept",
    "Coding",
    "ContactDetail",
    "ContactPoint",
    "Contributor",
    "Count",
    "DataRequirement",
    "Distance",
    "Dosage",
    "Duration",
    "ElementDefinition",
    "Expression",
    "Extension",
    "HumanName",
    "Identifier",
    "MarketingStatus",
    "Meta",
    "Money",
    "Narrative",
    "ParameterDefinition",
    "Period",
    "Population",
    "ProdCharacteristic",
    "ProductShelfLife",
    "Quantity",
    "Range",
    "Ratio",
    "Reference",
    "RelatedArtifact",
    "SampledData",
    "Signature",
    "SubstanceAmount",
    "Timing",
    "TriggerDefinition",
    "UsageContext",
];

/// The metadata types `codegen.types.003` names.
pub const METADATA_TYPES: [&str; 12] = [
    "ElementDefinition",
    "Extension",
    "Meta",
    "Narrative",
    "ParameterDefinition",
    "DataRequirement",
    "Expression",
    "TriggerDefinition",
    "UsageContext",
    "RelatedArtifact",
    "ContactDetail",
    "Contributor",
];

/// The `Resource` snapshot's elements, which `codegen.types.004` requires on
/// every generated resource type.
pub const RESOURCE_ELEMENTS: [&str; 4] = ["id", "meta", "implicitRules", "language"];
/// What `DomainResource` adds.
pub const DOMAIN_RESOURCE_ELEMENTS: [&str; 4] =
    ["text", "contained", "extension", "modifierExtension"];
/// The only three concrete resources whose `baseDefinition` is `Resource`
/// rather than `DomainResource`, confirmed against `profiles-resources.xml`.
pub const NON_DOMAIN_RESOURCES: [&str; 3] = ["Binary", "Bundle", "Parameters"];
/// The two abstract resource definitions.
pub const ABSTRACT_RESOURCES: [&str; 2] = ["Resource", "DomainResource"];

/// `Observation.value[x].type`, in snapshot order.
pub const OBSERVATION_VALUE_TYPES: [&str; 11] = [
    "Quantity",
    "CodeableConcept",
    "string",
    "boolean",
    "integer",
    "Range",
    "Ratio",
    "SampledData",
    "time",
    "dateTime",
    "Period",
];

/// The JSON property names `codegen.choice.001` requires to map to
/// `Observation.value`, in the same order as the types above.
pub const OBSERVATION_VALUE_PROPERTIES: [&str; 11] = [
    "valueQuantity",
    "valueCodeableConcept",
    "valueString",
    "valueBoolean",
    "valueInteger",
    "valueRange",
    "valueRatio",
    "valueSampledData",
    "valueTime",
    "valueDateTime",
    "valuePeriod",
];

/// `Extension.value[x]` declares 50 types — every open type.
pub const EXTENSION_VALUE_TYPE_COUNT: usize = 50;

/// `Patient`'s top-level element order from the snapshot, with `[x]` resolved
/// to the choice base name (`codegen.roundtrip.004`).
pub const PATIENT_ELEMENT_ORDER: [&str; 24] = [
    "id",
    "meta",
    "implicitRules",
    "language",
    "text",
    "contained",
    "extension",
    "modifierExtension",
    "identifier",
    "active",
    "name",
    "telecom",
    "gender",
    "birthDate",
    "deceased",
    "address",
    "maritalStatus",
    "multipleBirth",
    "photo",
    "contact",
    "communication",
    "generalPractitioner",
    "managingOrganization",
    "link",
];

/// `Patient.contact`'s element order from the snapshot.
pub const PATIENT_CONTACT_ELEMENT_ORDER: [&str; 10] = [
    "id",
    "extension",
    "modifierExtension",
    "relationship",
    "name",
    "telecom",
    "address",
    "gender",
    "organization",
    "period",
];

/// `administrative-gender`, the value set behind `Patient.gender`'s `required`
/// binding, in `ValueSet` order.
pub const ADMINISTRATIVE_GENDER_CODES: [&str; 4] = ["male", "female", "other", "unknown"];
pub const ADMINISTRATIVE_GENDER_VALUE_SET: &str =
    "http://hl7.org/fhir/ValueSet/administrative-gender|4.0.1";
pub const OBSERVATION_STATUS_VALUE_SET: &str =
    "http://hl7.org/fhir/ValueSet/observation-status|4.0.1";
/// `Observation.code`'s binding: `example`, and unversioned.
pub const OBSERVATION_CODES_VALUE_SET: &str = "http://hl7.org/fhir/ValueSet/observation-codes";

/// The three R4 SearchParameters that declare no `expression` at all
/// (`codegen.searchparam.005`).
pub const SEARCH_PARAMETERS_WITHOUT_EXPRESSION: [&str; 3] =
    ["DomainResource-text", "Resource-content", "Resource-query"];
/// SearchParameters that state `multipleOr`; all 46 state `false`.
pub const SEARCH_PARAMETERS_STATING_MULTIPLE_OR: usize = 46;
/// SearchParameters that state `comparator`; all 142 state the same nine codes.
pub const SEARCH_PARAMETERS_STATING_COMPARATOR: usize = 142;
/// The comparator list every one of those 142 declares.
pub const COMPARATOR_CODES: [&str; 9] = ["eq", "ne", "gt", "ge", "lt", "le", "sa", "eb", "ap"];
/// `multipleAnd`, `modifier` and `chain` are stated by **no** R4
/// SearchParameter, which is what makes absent-versus-`false` observable.
pub const SEARCH_PARAMETERS_STATING_MULTIPLE_AND: usize = 0;
pub const SEARCH_PARAMETERS_STATING_MODIFIER: usize = 0;
pub const SEARCH_PARAMETERS_STATING_CHAIN: usize = 0;

/// The six whole-system parameters `codegen.searchparam.002` requires on both
/// `Patient` and `Practitioner`, with the expression each declares. Their
/// declared `base` is the single abstract type `Resource`, so satisfying the
/// scenario means resolving a query base through the abstract element sets.
pub const WHOLE_SYSTEM_PARAMETERS: [(&str, &str); 6] = [
    ("_id", "Resource.id"),
    ("_lastUpdated", "Resource.meta.lastUpdated"),
    ("_profile", "Resource.meta.profile"),
    ("_security", "Resource.meta.security"),
    ("_source", "Resource.meta.source"),
    ("_tag", "Resource.meta.tag"),
];

/// `Observation-subject.target`, in declared order.
pub const OBSERVATION_SUBJECT_TARGETS: [&str; 4] = ["Group", "Device", "Patient", "Location"];

/// HAPI message codes, recorded for the conflicts in the module documentation.
/// `StrictErrorHandler.invalidValue`.
pub const CODE_INVALID_VALUE: u32 = 1821;
/// `StrictErrorHandler.unexpectedRepeatingElement`.
pub const CODE_UNEXPECTED_REPEATING_ELEMENT: u32 = 1823;

// ---------------------------------------------------------------------------
// The seam: what a generator run is, and what it answers with
// ---------------------------------------------------------------------------

/// The package pin the repository records: what `codegen.inputs.001` requires
/// the run to report and `codegen.inputs.002` compares against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePin {
    pub name: String,
    pub version: String,
    /// Lower-case hex, 64 characters.
    pub sha256: String,
}

/// One generator invocation, mirroring
/// `frevo-codegen generate --version r4 --package <dir> --out <dir>`.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub fhir_version: String,
    /// `None` means the directory the recorded pin names, which is what the
    /// command line does when `--package` is omitted. `Some` is how
    /// `codegen.inputs.003` points the run at an absent package.
    pub package_dir: Option<PathBuf>,
    /// The digest the run must verify against. `None` means the digest the
    /// repository records; `Some` is how `codegen.inputs.002` forces a
    /// mismatch.
    pub expected_sha256: Option<String>,
    pub out_dir: PathBuf,
    /// Where `reference/` would be. The generator must never read it
    /// (`codegen.inputs.004`); it is passed so that a run with it and a run
    /// without it are both expressible.
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

/// What `codegen.inputs.006` requires the run to state, and to fail on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inventory {
    pub datatype_definitions: usize,
    pub resource_definitions: usize,
    pub concrete_resources: usize,
    pub abstract_resources: usize,
    pub logical_resources: usize,
    pub search_parameters: usize,
}

/// An element's declared upper bound. `Zero` is not hypothetical: the snapshot
/// gives `xhtml.extension` a maximum of `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Max {
    Zero,
    One,
    Many,
}

/// What kind of thing a generated type is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeKind {
    Primitive,
    Complex,
    /// A concrete resource: one of the 146.
    Resource,
    /// A named nested type generated from a `BackboneElement` path.
    Backbone,
    /// `Resource` and `DomainResource`: an element set shared into every
    /// resource, never instantiable (`codegen.types.004`).
    AbstractElementSet,
    /// `MetadataResource`, the one `logical` definition.
    Logical,
}

impl TypeKind {
    pub const ALL: [TypeKind; 6] = [
        TypeKind::Primitive,
        TypeKind::Complex,
        TypeKind::Resource,
        TypeKind::Backbone,
        TypeKind::AbstractElementSet,
        TypeKind::Logical,
    ];
}

/// An element's value-set binding, as the snapshot declares it, plus the
/// enumeration the generator chose to emit — `None` whenever the binding is not
/// `required` or its value set is not resolvable from the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingInfo {
    pub strength: String,
    pub value_set: String,
    pub enumeration: Option<String>,
}

/// One generated element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedElement {
    /// The FHIR property name the element is addressed by: the base name for a
    /// choice, so `value` and not `value[x]`.
    pub fhir_name: String,
    pub rust_name: String,
    /// The snapshot path, `[x]` included, so the generated element can be
    /// traced back to its definition.
    pub path: String,
    pub types: Vec<String>,
    pub is_choice: bool,
    /// Every JSON property name that maps to this element. One entry for an
    /// ordinary element; one per declared type for a choice.
    pub json_property_names: Vec<String>,
    pub min: u32,
    pub max: Max,
    pub binding: Option<BindingInfo>,
    /// `Questionnaire.item.item` carries `#Questionnaire.item` here.
    pub content_reference: Option<String>,
}

/// One variant of a generated enumeration, with the code it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub code: String,
    pub rust_name: String,
}

/// A closed enumeration, generated only for a resolvable `required` binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedEnum {
    pub rust_name: String,
    pub value_set: String,
    /// Every element path bound to it.
    pub element_paths: Vec<String>,
    pub variants: Vec<EnumVariant>,
}

impl GeneratedEnum {
    pub fn codes(&self) -> Vec<&str> {
        self.variants.iter().map(|v| v.code.as_str()).collect()
    }
}

/// A `required` binding the generator could not resolve, which it must report
/// rather than silently leave out (`codegen.binding.004`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedBinding {
    pub element_path: String,
    pub value_set_url: String,
    pub strength: String,
}

/// One generated type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedType {
    pub fhir_name: String,
    pub rust_name: String,
    pub kind: TypeKind,
    pub base: Option<String>,
    /// `false` for `Resource` and `DomainResource`.
    pub instantiable: bool,
    pub elements: Vec<GeneratedElement>,
    /// The element order the snapshot declares, by FHIR property name
    /// (`codegen.roundtrip.004`).
    pub element_order: Vec<String>,
}

impl GeneratedType {
    pub fn element(&self, fhir_name: &str) -> Option<&GeneratedElement> {
        self.elements.iter().find(|e| e.fhir_name == fhir_name)
    }

    pub fn element_or_panic(&self, fhir_name: &str) -> &GeneratedElement {
        self.element(fhir_name).unwrap_or_else(|| {
            panic!(
                "{} must have an element named {fhir_name}; it has {:?}",
                self.fhir_name,
                self.element_names()
            )
        })
    }

    pub fn element_names(&self) -> Vec<&str> {
        self.elements.iter().map(|e| e.fhir_name.as_str()).collect()
    }
}

/// The resource-type registry entry: FHIR type name to Rust type and back
/// (`codegen.types.005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub fhir_name: String,
    pub rust_name: String,
}

/// A composite parameter's component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParamComponent {
    pub definition: String,
    pub expression: String,
}

/// One row of the generated search-parameter table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParamRow {
    /// The `SearchParameter.id`, which is this table's case selector.
    pub id: String,
    pub code: String,
    pub url: String,
    /// `SearchParameter.type`: `token`, `reference`, `composite` and so on.
    pub search_type: String,
    /// The base this row is for. A parameter with several bases contributes one
    /// row per base (`codegen.searchparam.002`).
    pub base: String,
    /// Every base the SearchParameter declares, so a row knows its siblings.
    pub declared_bases: Vec<String>,
    /// `None` when the SearchParameter declares no `expression` at all, which
    /// is distinct from `Some(String::new())`.
    pub expression: Option<String>,
    pub components: Vec<SearchParamComponent>,
    pub targets: Vec<String>,
    pub multiple_or: Option<bool>,
    pub multiple_and: Option<bool>,
    pub comparator: Option<Vec<String>>,
    pub modifier: Option<Vec<String>>,
    pub chain: Option<Vec<String>>,
}

/// One written output file, with the digest a cross-platform comparison needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Relative to the run's `out_dir`, with `/` separators.
    pub path: String,
    pub bytes: Vec<u8>,
    /// Lower-case hex SHA-256 of `bytes`.
    pub sha256: String,
}

impl GeneratedFile {
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes)
            .unwrap_or_else(|e| panic!("generated file {} must be UTF-8: {e}", self.path))
    }
}

/// Everything one run reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateReport {
    /// `0` on success; the refusal paths return [`CodegenError::Failed`]
    /// instead, so this field exists to make the success contract explicit.
    pub exit_code: i32,
    /// The package the run verified.
    pub pin: PackagePin,
    /// The human-readable summary the command prints.
    pub summary: String,
    pub inventory: Inventory,
    /// Every path the run opened. `codegen.inputs.004` is an assertion about
    /// this list.
    pub inputs_read: Vec<PathBuf>,
    pub types: Vec<GeneratedType>,
    pub enumerations: Vec<GeneratedEnum>,
    pub unresolved_bindings: Vec<UnresolvedBinding>,
    pub resource_registry: Vec<RegistryEntry>,
    pub search_parameters: Vec<SearchParamRow>,
    pub files: Vec<GeneratedFile>,
}

impl GenerateReport {
    pub fn ty(&self, fhir_name: &str) -> Option<&GeneratedType> {
        self.types.iter().find(|t| t.fhir_name == fhir_name)
    }

    pub fn ty_or_panic(&self, fhir_name: &str) -> &GeneratedType {
        self.ty(fhir_name).unwrap_or_else(|| {
            panic!(
                "a type must be generated for {fhir_name}; {} types were generated",
                self.types.len()
            )
        })
    }

    pub fn types_of_kind(&self, kind: TypeKind) -> Vec<&GeneratedType> {
        self.types.iter().filter(|t| t.kind == kind).collect()
    }

    pub fn enumeration_for(&self, value_set: &str) -> Option<&GeneratedEnum> {
        self.enumerations.iter().find(|e| e.value_set == value_set)
    }

    /// Every row for one `SearchParameter.id`, which is one row per declared
    /// base.
    pub fn rows_for_id(&self, id: &str) -> Vec<&SearchParamRow> {
        self.search_parameters
            .iter()
            .filter(|r| r.id == id)
            .collect()
    }

    pub fn row(&self, id: &str, base: &str) -> Option<&SearchParamRow> {
        self.search_parameters
            .iter()
            .find(|r| r.id == id && r.base == base)
    }

    /// Distinct `SearchParameter.id` values in the table.
    pub fn distinct_parameter_ids(&self) -> BTreeSet<&str> {
        self.search_parameters
            .iter()
            .map(|r| r.id.as_str())
            .collect()
    }

    pub fn file(&self, path: &str) -> Option<&GeneratedFile> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Relative path to bytes, which is what a byte-identity comparison needs.
    pub fn file_map(&self) -> BTreeMap<&str, &[u8]> {
        self.files
            .iter()
            .map(|f| (f.path.as_str(), f.bytes.as_slice()))
            .collect()
    }
}

/// Why a run refused. Every variant is a condition some scenario requires the
/// generator to detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureKind {
    /// `codegen.inputs.002`.
    DigestMismatch,
    /// `codegen.inputs.003`.
    PackageMissing,
    /// `codegen.inputs.006`.
    InventoryMismatch,
    /// `codegen.naming.003`.
    NameCollision,
    /// A decode or encode failure.
    Codec,
    /// Anything else the generator diagnoses.
    Other,
}

impl FailureKind {
    pub const ALL: [FailureKind; 6] = [
        FailureKind::DigestMismatch,
        FailureKind::PackageMissing,
        FailureKind::InventoryMismatch,
        FailureKind::NameCollision,
        FailureKind::Codec,
        FailureKind::Other,
    ];
}

/// An error from the seam. `NotImplemented` is **not** a refusal: the refusal
/// scenarios assert a [`FailureKind`] and the content of the message, so a
/// generator that does nothing fails them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Failed {
        exit_code: i32,
        kind: FailureKind,
        message: String,
    },
    NotImplemented,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Failed {
                exit_code,
                kind,
                message,
            } => write!(f, "exit {exit_code} [{kind:?}] {message}"),
            CodegenError::NotImplemented => f.write_str(
                "no generator is bound to the codegen-r4 oracle yet \
                 (crates/frevo-codegen/tests/codegen_r4.rs::generator_under_test)",
            ),
        }
    }
}

impl CodegenError {
    /// The refusal this error is, or a panic naming what was expected. An
    /// unbound seam lands here, which is why the message says so.
    pub fn refusal(&self, what: &str) -> (i32, FailureKind, &str) {
        match self {
            CodegenError::Failed {
                exit_code,
                kind,
                message,
            } => (*exit_code, *kind, message.as_str()),
            CodegenError::NotImplemented => panic!(
                "{what} must be a diagnosed refusal with an exit code, a kind and a \
                 message; instead: {self}"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The codec side of the seam
// ---------------------------------------------------------------------------

/// The two parse modes, as `FhirContext.setParserErrorHandler` selects them.
/// Lenient is the default, because that is what `FhirContext` installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeMode {
    Lenient,
    Strict,
}

/// The diagnostic conditions the `codegen.*` scenarios reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    /// A missing-required-element report. HAPI's parser checks no minimum
    /// cardinality on decode, so `codegen.cardinality.002`, `.004` and `.005`
    /// assert that this is **never** returned. The codec has no such report
    /// today, so nothing maps to it; it is kept so the absence is asserted by
    /// name.
    MissingRequiredElement,
    /// `codegen.cardinality.005`, `codegen.binding.002` (a blank code).
    InvalidValue,
    /// `codegen.choice.004`, lenient mode.
    DuplicateChoiceValue,
    /// Any other report the codec raises -- an unknown property, a JSON type
    /// mismatch, a reference-graph condition. No `codegen.*` scenario is about
    /// one, but each is carried rather than dropped, so an assertion that a
    /// decode raised nothing still sees it. The message names the codec's own
    /// condition.
    Other,
}

impl DiagnosticKind {
    pub const ALL: [DiagnosticKind; 4] = [
        DiagnosticKind::MissingRequiredElement,
        DiagnosticKind::InvalidValue,
        DiagnosticKind::DuplicateChoiceValue,
        DiagnosticKind::Other,
    ];
}

/// One diagnostic: the condition, the element path it is about, and the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub path: String,
    pub message: String,
}

/// One element occurrence in a decoded document.
///
/// The flattening contract, so assertions below are unambiguous:
///
/// - one entry per element occurrence, in document order;
/// - `path` is the FHIR path with a `[i]` index on every repeating element,
///   index `0` included, and no index on a single-valued one;
/// - `value` is the primitive lexical value, and `None` for a complex element;
/// - `chosen_type` is set only on a choice element, and names which declared
///   type the value holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementValue {
    pub path: String,
    pub chosen_type: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub resource_type: String,
    pub diagnostics: Vec<Diagnostic>,
    pub values: Vec<ElementValue>,
}

impl Decoded {
    pub fn at(&self, path: &str) -> Vec<&ElementValue> {
        self.values.iter().filter(|v| v.path == path).collect()
    }

    /// Every occurrence of a repeating element, in document order.
    pub fn repeats(&self, path: &str) -> Vec<&ElementValue> {
        let prefix = format!("{path}[");
        self.values
            .iter()
            .filter(|v| v.path.starts_with(&prefix))
            .collect()
    }

    pub fn kinds(&self) -> Vec<DiagnosticKind> {
        let mut v: Vec<DiagnosticKind> = self.diagnostics.iter().map(|d| d.kind).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    pub fn diagnostic(&self, kind: DiagnosticKind) -> Option<&Diagnostic> {
        self.diagnostics.iter().find(|d| d.kind == kind)
    }
}

// ---------------------------------------------------------------------------
// The round-trip, drift and cross-check surfaces
// ---------------------------------------------------------------------------

/// A round-trip case's outcome. `NotApplicableXml` is distinct from `Passed`,
/// which is the whole of `codegen.roundtrip.003`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoundTripStatus {
    Passed,
    Failed,
    NotApplicableXml,
}

impl RoundTripStatus {
    pub const ALL: [RoundTripStatus; 3] = [
        RoundTripStatus::Passed,
        RoundTripStatus::Failed,
        RoundTripStatus::NotApplicableXml,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RoundTripStatus::Passed => "passed",
            RoundTripStatus::Failed => "failed",
            RoundTripStatus::NotApplicableXml => "not-applicable-xml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripCase {
    /// The example's file name, so a failure identifies a case rather than a
    /// count (`codegen.roundtrip.001`).
    pub file: String,
    pub status: RoundTripStatus,
    pub detail: String,
}

/// How many examples exercised one resource type. A type with none is listed
/// with `examples: 0`, not omitted (`codegen.roundtrip.002`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCoverage {
    pub resource_type: String,
    pub examples: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripReport {
    /// Which example set this run used, named so an interim selector cannot be
    /// mistaken for the full R4 example package.
    pub selector: String,
    pub cases: Vec<RoundTripCase>,
    pub coverage: Vec<TypeCoverage>,
}

impl RoundTripReport {
    pub fn case(&self, file: &str) -> Option<&RoundTripCase> {
        self.cases.iter().find(|c| c.file == file)
    }

    pub fn with_status(&self, status: RoundTripStatus) -> Vec<&RoundTripCase> {
        self.cases.iter().filter(|c| c.status == status).collect()
    }

    pub fn coverage_of(&self, resource_type: &str) -> Option<&TypeCoverage> {
        self.coverage
            .iter()
            .find(|c| c.resource_type == resource_type)
    }
}

/// The CI drift check (`codegen.output.005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    pub in_sync: bool,
    /// Paths relative to the checked-in directory.
    pub differing_files: Vec<String>,
    /// What to do about it: the regeneration command, never an instruction to
    /// edit.
    pub remediation: String,
}

/// One element the package and the HAPI copy disagree about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDisagreement {
    pub url: String,
    pub path: String,
    pub detail: String,
}

/// The `reference/`-conditional cross-check (`codegen.inputs.005`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossCheckReport {
    /// What the package side was scoped to, so the basis of the comparison is
    /// stated rather than assumed.
    pub scope: String,
    pub package_fhir_version: String,
    pub hapi_fhir_version: String,
    pub hapi_definition_count: usize,
    pub only_in_package: Vec<String>,
    pub only_in_hapi: Vec<String>,
    pub element_disagreements: Vec<ElementDisagreement>,
}

// ---------------------------------------------------------------------------
// Synthetic definitions, for the cases the pinned package cannot reach
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefinitionKind {
    PrimitiveType,
    ComplexType,
    Resource,
    Logical,
}

impl DefinitionKind {
    pub const ALL: [DefinitionKind; 4] = [
        DefinitionKind::PrimitiveType,
        DefinitionKind::ComplexType,
        DefinitionKind::Resource,
        DefinitionKind::Logical,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingInput {
    pub strength: String,
    pub value_set: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementInput {
    pub path: String,
    pub min: u32,
    /// The snapshot's literal `max`: `"0"`, `"1"` or `"*"`.
    pub max: String,
    pub types: Vec<String>,
    pub content_reference: Option<String>,
    pub binding: Option<BindingInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionInput {
    pub url: String,
    pub name: String,
    pub kind: DefinitionKind,
    pub base: Option<String>,
    pub is_abstract: bool,
    pub derivation: Option<String>,
    pub elements: Vec<ElementInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSetInput {
    pub url: String,
    pub codes: Vec<String>,
}

/// A generator run over definitions written here rather than read from the
/// package. Used only where the pinned R4 core package contains no instance of
/// the condition a scenario describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticRequest {
    pub definitions: Vec<DefinitionInput>,
    pub value_sets: Vec<ValueSetInput>,
    pub out_dir: PathBuf,
    /// Whether the run must assert the pinned R4 inventory before generating.
    /// `true` is what `frevo-codegen generate --version r4` does, and is how
    /// `codegen.inputs.006`'s refusal is reached; the synthetic runs that
    /// exercise one naming or binding rule set it `false`.
    pub assert_pinned_inventory: bool,
}

impl SyntheticRequest {
    /// A synthetic run that does not assert the pinned inventory.
    pub fn new(
        definitions: Vec<DefinitionInput>,
        value_sets: Vec<ValueSetInput>,
        out_dir: &Path,
    ) -> Self {
        SyntheticRequest {
            definitions,
            value_sets,
            out_dir: out_dir.to_path_buf(),
            assert_pinned_inventory: false,
        }
    }
}

/// A `0..1` or `0..*` element of the given types.
pub fn element(path: &str, min: u32, max: &str, types: &[&str]) -> ElementInput {
    ElementInput {
        path: path.to_string(),
        min,
        max: max.to_string(),
        types: types.iter().map(|t| t.to_string()).collect(),
        content_reference: None,
        binding: None,
    }
}

/// The same, with a value-set binding.
pub fn bound_element(
    path: &str,
    min: u32,
    max: &str,
    types: &[&str],
    strength: &str,
    value_set: &str,
) -> ElementInput {
    let mut e = element(path, min, max, types);
    e.binding = Some(BindingInput {
        strength: strength.to_string(),
        value_set: value_set.to_string(),
    });
    e
}

/// A synthetic complex datatype named `name`, with `Element` as its base.
pub fn complex_type(name: &str, elements: Vec<ElementInput>) -> DefinitionInput {
    DefinitionInput {
        url: format!("http://frevo.test/StructureDefinition/{name}"),
        name: name.to_string(),
        kind: DefinitionKind::ComplexType,
        base: Some("http://hl7.org/fhir/StructureDefinition/Element".to_string()),
        is_abstract: false,
        derivation: Some("specialization".to_string()),
        elements,
    }
}

// ---------------------------------------------------------------------------
// The trait, and the binding under test
// ---------------------------------------------------------------------------

pub trait Codegen {
    /// The pin the repository records for the input package.
    fn recorded_pin(&self) -> Result<PackagePin, CodegenError>;

    /// One `frevo-codegen generate` run.
    fn generate(&self, request: &GenerateRequest) -> Result<GenerateReport, CodegenError>;

    /// A run over definitions written in this file.
    fn generate_synthetic(
        &self,
        request: &SyntheticRequest,
    ) -> Result<GenerateReport, CodegenError>;

    /// Compare the pinned package with the HAPI copy.
    fn cross_check(
        &self,
        package_dir: Option<&Path>,
        hapi_profile_dir: &Path,
    ) -> Result<CrossCheckReport, CodegenError>;

    /// Compare a fresh run with a checked-in output tree.
    fn drift_check(
        &self,
        request: &GenerateRequest,
        checked_in_dir: &Path,
    ) -> Result<DriftReport, CodegenError>;

    /// Decode and canonically re-encode each example.
    fn round_trip(&self, examples: &[PathBuf]) -> Result<RoundTripReport, CodegenError>;

    /// Decode a document against the generated model.
    fn decode(&self, json: &str, mode: DecodeMode) -> Result<Decoded, CodegenError>;

    /// Serialise in the generated element order — what the server stores.
    fn encode(&self, decoded: &Decoded) -> Result<String, CodegenError>;

    /// Serialise in `JsonCreatorCanonical`'s form: property names sorted
    /// lexicographically, no whitespace.
    fn encode_canonical(&self, decoded: &Decoded) -> Result<String, CodegenError>;

    /// The Rust field name for a snapshot element path. Total, so it answers
    /// for a path the package does not contain.
    fn rust_field_name(&self, element_path: &str) -> Result<String, CodegenError>;

    /// The variants the naming rule produces for a code system's codes.
    fn code_variants(&self, codes: &[&str]) -> Result<Vec<EnumVariant>, CodegenError>;

    /// The table queried by base: every row that applies to a resource type,
    /// with the abstract bases `Resource` and `DomainResource` resolved onto
    /// the concrete type (`codegen.searchparam.002`).
    fn parameters_for(&self, resource_type: &str) -> Result<Vec<SearchParamRow>, CodegenError>;

    /// Hand a search-parameter expression to `frevo-path`'s parser
    /// (`codegen.searchparam.005`). It lives on this trait so that the test
    /// does not need a dependency edge `frevo-codegen` must not have; the
    /// binding reaches the real parser through a dev-dependency, which is not
    /// an edge of the generator itself.
    fn parse_search_expression(&self, expression: &str) -> Result<(), CodegenError>;
}

/// The generator this oracle grades.
///
/// The generator half -- `generate`, the synthetic runs, the cross-check, the
/// drift check, the naming rules and the table query -- is `frevo-codegen`
/// through [`BoundGenerator`]. The codec half and the parser are the crates
/// that already implement them, reached through dev-dependencies:
/// `frevo-model`'s decoder over the checked-in generated tables, and
/// `frevo-path`'s parser. See [`OracleGenerator`].
fn generator_under_test() -> Box<dyn Codegen> {
    Box::new(OracleGenerator::default())
}

/// [`BoundGenerator`] for everything `frevo-codegen` answers, and the real
/// codec and parser for the five methods `bind_oracle!` leaves unbound.
///
/// No behaviour is invented here. Each method below is a mapping from the
/// oracle's data model onto an existing public surface:
///
/// | method | surface |
/// |---|---|
/// | `decode` | `frevo_model::codec::decode::decode_document` |
/// | `encode` | `frevo_model::Decoded::encode_stored` |
/// | `encode_canonical` | `frevo_model::Decoded::encode_canonical` |
/// | `round_trip` | the two above, over each example file |
/// | `parse_search_expression` | `frevo_path::Expression::parse` |
///
/// The oracle's [`Decoded`] is a flattened view and does not carry the codec's
/// tree, so each decode is remembered beside the view it produced and `encode`
/// looks the view up. A view this generator did not produce is an error, never
/// a guess.
#[derive(Default)]
struct OracleGenerator {
    decodes: std::cell::RefCell<Vec<(Decoded, frevo_model::Decoded)>>,
}

impl OracleGenerator {
    fn codec_error(message: String) -> CodegenError {
        CodegenError::Failed {
            exit_code: frevo_codegen::error::REFUSAL_EXIT_CODE,
            kind: FailureKind::Codec,
            message,
        }
    }

    fn mode(mode: DecodeMode) -> frevo_model::ParseMode {
        match mode {
            DecodeMode::Lenient => frevo_model::ParseMode::Lenient,
            DecodeMode::Strict => frevo_model::ParseMode::Strict,
        }
    }

    /// The codec tree an oracle view was produced from.
    fn tree(&self, decoded: &Decoded) -> Result<frevo_model::Decoded, CodegenError> {
        self.decodes
            .borrow()
            .iter()
            .rev()
            .find(|(view, _)| view == decoded)
            .map(|(_, tree)| tree.clone())
            .ok_or_else(|| CodegenError::Failed {
                exit_code: frevo_codegen::error::REFUSAL_EXIT_CODE,
                kind: FailureKind::Other,
                message: "this Decoded was not produced by this generator's decode, so there \
                          is no codec tree to encode"
                    .to_string(),
            })
    }

    /// Whether the element at `path` (indexed, choice under its base name) is
    /// a choice element, read from the generated type table: the owning type
    /// is the parent occurrence's concrete type, or the resource type at the
    /// root.
    fn is_choice(path: &str, types_by_path: &BTreeMap<String, String>, root: &str) -> bool {
        let Some((parent, last)) = path.rsplit_once('.') else {
            return false;
        };
        let name = last.split('[').next().unwrap_or(last);
        let owner = types_by_path
            .get(parent)
            .map(String::as_str)
            .unwrap_or(if parent == root { root } else { "" });
        frevo_model::codec::R4
            .element(owner, name)
            .is_some_and(|element| element.choice)
    }

    fn view(decoded: &frevo_model::Decoded) -> Decoded {
        let types_by_path: BTreeMap<String, String> = decoded
            .elements
            .iter()
            .map(|e| (e.path.clone(), e.type_name.clone()))
            .collect();
        let root = decoded.resource_type.as_str();
        let values = decoded
            .elements
            .iter()
            .map(|e| ElementValue {
                path: e.path.clone(),
                chosen_type: Self::is_choice(&e.path, &types_by_path, root)
                    .then(|| e.type_name.clone()),
                value: e.value.clone(),
            })
            .collect();
        let mut diagnostics: Vec<Diagnostic> = decoded
            .diagnostics
            .iter()
            .map(|d| {
                let kind = match d.kind {
                    frevo_model::ReportKind::InvalidValue => DiagnosticKind::InvalidValue,
                    frevo_model::ReportKind::UnexpectedRepeatingElement
                        if Self::is_choice(&d.path, &types_by_path, root) =>
                    {
                        DiagnosticKind::DuplicateChoiceValue
                    }
                    _ => DiagnosticKind::Other,
                };
                Diagnostic {
                    kind,
                    path: d.path.clone(),
                    message: format!("{:?}: {}", d.kind, d.message),
                }
            })
            .collect();
        diagnostics.extend(decoded.reference_diagnostics.iter().map(|d| Diagnostic {
            kind: DiagnosticKind::Other,
            path: d.path.clone(),
            message: format!("{:?} ({}): {}", d.issue, d.code, d.message),
        }));
        Decoded {
            resource_type: decoded.resource_type.clone(),
            diagnostics,
            values,
        }
    }

    /// A decode failure as the seam reports it.
    ///
    /// A condition HAPI has a message code for is rendered the way
    /// `Msg.code(n) + message` renders it upstream, `HAPI-<n>: <message>`, so
    /// that the decode scenarios can assert the approved delta's texts exactly
    /// (`codegen.choice.004`, `codegen.cardinality.005`, `codegen.binding.002`).
    /// A condition with no HAPI code keeps the codec's own `Display`, which
    /// names its kind, path and offset.
    fn failure_message(failure: &frevo_model::codec::decode::DecodeFailure) -> String {
        use frevo_model::codec::decode::DecodeFailure;
        match failure {
            DecodeFailure::Report(report) => match report.code {
                Some(code) => format!("HAPI-{code}: {}", report.message),
                None => failure.to_string(),
            },
            DecodeFailure::Coded(coded) => format!("HAPI-{}: {}", coded.code, coded.message),
        }
    }

    fn utf8(bytes: Vec<u8>, what: &str) -> Result<String, CodegenError> {
        String::from_utf8(bytes)
            .map_err(|e| Self::codec_error(format!("the {what} form is not UTF-8: {e}")))
    }

    /// One example through the codec: decode, canonically re-encode, and
    /// compare with the source, both canonicalised by [`canonical_json`] --
    /// which is independent of the codec, so the comparison is not the codec
    /// agreeing with itself.
    fn round_trip_one(path: &Path) -> (RoundTripStatus, String, Option<String>) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => return (RoundTripStatus::Failed, format!("unreadable: {e}"), None),
        };
        let resource_type = canonical_json::resource_type(&bytes);
        let decoded = match frevo_model::codec::decode::decode_document(
            &bytes,
            frevo_model::DEFAULT_PARSE_MODE,
        ) {
            Ok(decoded) => decoded,
            Err(e) => {
                return (
                    RoundTripStatus::Failed,
                    format!("decode failed: {e}"),
                    resource_type,
                );
            }
        };
        let encoded = match decoded.encode_canonical(frevo_model::DEFAULT_PARSE_MODE) {
            Ok(encoded) => encoded.bytes,
            Err(e) => {
                return (
                    RoundTripStatus::Failed,
                    format!("canonical encode failed: {e}"),
                    resource_type,
                );
            }
        };
        let source = match canonical_json::canonicalise(&bytes) {
            Ok(source) => source,
            Err(e) => {
                return (
                    RoundTripStatus::Failed,
                    format!("the source is not JSON: {e}"),
                    resource_type,
                );
            }
        };
        let result = match canonical_json::canonicalise(&encoded) {
            Ok(result) => result,
            Err(e) => {
                return (
                    RoundTripStatus::Failed,
                    format!("the codec's canonical output is not JSON: {e}"),
                    resource_type,
                );
            }
        };
        if source == result {
            (RoundTripStatus::Passed, String::new(), resource_type)
        } else {
            let at = source
                .bytes()
                .zip(result.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(source.len().min(result.len()));
            let window = |s: &str| {
                let start = at.saturating_sub(40);
                s.get(start..(at + 60).min(s.len()))
                    .unwrap_or_default()
                    .to_string()
            };
            (
                RoundTripStatus::Failed,
                format!(
                    "first difference at byte {at}: source …{}… result …{}…",
                    window(&source),
                    window(&result)
                ),
                resource_type,
            )
        }
    }
}

impl Codegen for OracleGenerator {
    fn recorded_pin(&self) -> Result<PackagePin, CodegenError> {
        BoundGenerator.recorded_pin()
    }

    fn generate(&self, request: &GenerateRequest) -> Result<GenerateReport, CodegenError> {
        BoundGenerator.generate(request)
    }

    fn generate_synthetic(
        &self,
        request: &SyntheticRequest,
    ) -> Result<GenerateReport, CodegenError> {
        BoundGenerator.generate_synthetic(request)
    }

    fn cross_check(
        &self,
        package_dir: Option<&Path>,
        hapi_profile_dir: &Path,
    ) -> Result<CrossCheckReport, CodegenError> {
        BoundGenerator.cross_check(package_dir, hapi_profile_dir)
    }

    fn drift_check(
        &self,
        request: &GenerateRequest,
        checked_in_dir: &Path,
    ) -> Result<DriftReport, CodegenError> {
        BoundGenerator.drift_check(request, checked_in_dir)
    }

    fn round_trip(&self, examples: &[PathBuf]) -> Result<RoundTripReport, CodegenError> {
        let interim = repo_path(EXAMPLE_DIR);
        let all_interim = !examples.is_empty()
            && examples
                .iter()
                .all(|p| p.parent().is_some_and(|dir| dir == interim));
        let selector = if all_interim {
            format!(
                "interim selector: {EXAMPLE_DIR} ({} files), not the full \
                 hl7.fhir.r4.examples#4.0.1 package (task 59)",
                examples.len()
            )
        } else {
            format!("caller-supplied selector: {} files", examples.len())
        };

        let mut counts: BTreeMap<String, usize> = frevo_model::r4::types::RESOURCE_TYPES
            .iter()
            .map(|t| (t.to_string(), 0))
            .collect();
        let mut cases = Vec::with_capacity(examples.len());
        for path in examples {
            let file = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            if file.ends_with(".xml") {
                cases.push(RoundTripCase {
                    file,
                    status: RoundTripStatus::NotApplicableXml,
                    detail: "XML input; the codec under test is the JSON one".to_string(),
                });
                continue;
            }
            let (status, detail, resource_type) = Self::round_trip_one(path);
            if let Some(count) = resource_type.and_then(|t| counts.get_mut(&t)) {
                *count += 1;
            }
            cases.push(RoundTripCase {
                file,
                status,
                detail,
            });
        }
        Ok(RoundTripReport {
            selector,
            cases,
            coverage: counts
                .into_iter()
                .map(|(resource_type, examples)| TypeCoverage {
                    resource_type,
                    examples,
                })
                .collect(),
        })
    }

    fn decode(&self, json: &str, mode: DecodeMode) -> Result<Decoded, CodegenError> {
        let tree = frevo_model::codec::decode::decode_document(json.as_bytes(), Self::mode(mode))
            .map_err(|e| Self::codec_error(Self::failure_message(&e)))?;
        let view = Self::view(&tree);
        self.decodes.borrow_mut().push((view.clone(), tree));
        Ok(view)
    }

    fn encode(&self, decoded: &Decoded) -> Result<String, CodegenError> {
        let encoded = self
            .tree(decoded)?
            .encode_stored(frevo_model::DEFAULT_PARSE_MODE)
            .map_err(|e| Self::codec_error(e.to_string()))?;
        Self::utf8(encoded.bytes, "stored")
    }

    fn encode_canonical(&self, decoded: &Decoded) -> Result<String, CodegenError> {
        let encoded = self
            .tree(decoded)?
            .encode_canonical(frevo_model::DEFAULT_PARSE_MODE)
            .map_err(|e| Self::codec_error(e.to_string()))?;
        Self::utf8(encoded.bytes, "canonical")
    }

    fn rust_field_name(&self, element_path: &str) -> Result<String, CodegenError> {
        BoundGenerator.rust_field_name(element_path)
    }

    fn code_variants(&self, codes: &[&str]) -> Result<Vec<EnumVariant>, CodegenError> {
        BoundGenerator.code_variants(codes)
    }

    fn parameters_for(&self, resource_type: &str) -> Result<Vec<SearchParamRow>, CodegenError> {
        BoundGenerator.parameters_for(resource_type)
    }

    fn parse_search_expression(&self, expression: &str) -> Result<(), CodegenError> {
        frevo_path::Expression::parse(expression)
            .map(|_| ())
            .map_err(|e| CodegenError::Failed {
                exit_code: frevo_codegen::error::REFUSAL_EXIT_CODE,
                kind: FailureKind::Other,
                message: format!("frevo-path refused {expression:?}: {e}"),
            })
    }
}

/// A canonicaliser for the round-trip comparison that shares no code with the
/// codec: object members sorted by name, no insignificant whitespace, every
/// number kept exactly as written, every string re-escaped minimally after
/// decoding its escapes. Applying it to both sides is what
/// `codegen.roundtrip.001`'s "both the source and the result are canonicalised
/// the same way" means.
mod canonical_json {
    enum Json {
        Null,
        Bool(bool),
        Number(String),
        String(String),
        Array(Vec<Json>),
        Object(Vec<(String, Json)>),
    }

    struct Parser<'a> {
        src: &'a [u8],
        at: usize,
    }

    impl Parser<'_> {
        fn fail<T>(&self, why: &str) -> Result<T, String> {
            Err(format!("{why} at byte {}", self.at))
        }

        fn ws(&mut self) {
            while self
                .src
                .get(self.at)
                .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
            {
                self.at += 1;
            }
        }

        fn value(&mut self) -> Result<Json, String> {
            self.ws();
            match self.src.get(self.at) {
                Some(b'{') => self.object(),
                Some(b'[') => self.array(),
                Some(b'"') => Ok(Json::String(self.string()?)),
                Some(b't') => self.literal("true", Json::Bool(true)),
                Some(b'f') => self.literal("false", Json::Bool(false)),
                Some(b'n') => self.literal("null", Json::Null),
                Some(c) if *c == b'-' || c.is_ascii_digit() => {
                    let start = self.at;
                    while self.src.get(self.at).is_some_and(|c| {
                        matches!(c, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                    }) {
                        self.at += 1;
                    }
                    Ok(Json::Number(
                        String::from_utf8_lossy(&self.src[start..self.at]).to_string(),
                    ))
                }
                _ => self.fail("expected a value"),
            }
        }

        fn literal(&mut self, text: &str, value: Json) -> Result<Json, String> {
            if self.src[self.at..].starts_with(text.as_bytes()) {
                self.at += text.len();
                Ok(value)
            } else {
                self.fail("bad literal")
            }
        }

        fn hex4(&mut self) -> Result<u32, String> {
            let digits = self
                .src
                .get(self.at..self.at + 4)
                .and_then(|d| std::str::from_utf8(d).ok())
                .and_then(|d| u32::from_str_radix(d, 16).ok());
            match digits {
                Some(v) => {
                    self.at += 4;
                    Ok(v)
                }
                None => self.fail("bad \\u escape"),
            }
        }

        fn string(&mut self) -> Result<String, String> {
            self.at += 1;
            let mut out: Vec<u8> = Vec::new();
            loop {
                match self.src.get(self.at) {
                    None => return self.fail("unterminated string"),
                    Some(b'"') => {
                        self.at += 1;
                        return String::from_utf8(out).map_err(|e| e.to_string());
                    }
                    Some(b'\\') => {
                        self.at += 1;
                        let c = self.src.get(self.at).copied();
                        self.at += 1;
                        let ch = match c {
                            Some(b'"') => '"',
                            Some(b'\\') => '\\',
                            Some(b'/') => '/',
                            Some(b'b') => '\u{8}',
                            Some(b'f') => '\u{c}',
                            Some(b'n') => '\n',
                            Some(b'r') => '\r',
                            Some(b't') => '\t',
                            Some(b'u') => {
                                let hi = self.hex4()?;
                                let code = if (0xD800..0xDC00).contains(&hi)
                                    && self.src[self.at..].starts_with(b"\\u")
                                {
                                    self.at += 2;
                                    let lo = self.hex4()?;
                                    0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00))
                                } else {
                                    hi
                                };
                                char::from_u32(code).unwrap_or('\u{FFFD}')
                            }
                            _ => return self.fail("bad escape"),
                        };
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    }
                    Some(c) => {
                        out.push(*c);
                        self.at += 1;
                    }
                }
            }
        }

        fn array(&mut self) -> Result<Json, String> {
            self.at += 1;
            let mut items = Vec::new();
            self.ws();
            if self.src.get(self.at) == Some(&b']') {
                self.at += 1;
                return Ok(Json::Array(items));
            }
            loop {
                items.push(self.value()?);
                self.ws();
                match self.src.get(self.at) {
                    Some(b',') => self.at += 1,
                    Some(b']') => {
                        self.at += 1;
                        return Ok(Json::Array(items));
                    }
                    _ => return self.fail("expected , or ]"),
                }
            }
        }

        fn object(&mut self) -> Result<Json, String> {
            self.at += 1;
            let mut members = Vec::new();
            self.ws();
            if self.src.get(self.at) == Some(&b'}') {
                self.at += 1;
                return Ok(Json::Object(members));
            }
            loop {
                self.ws();
                if self.src.get(self.at) != Some(&b'"') {
                    return self.fail("expected a property name");
                }
                let name = self.string()?;
                self.ws();
                if self.src.get(self.at) != Some(&b':') {
                    return self.fail("expected :");
                }
                self.at += 1;
                members.push((name, self.value()?));
                self.ws();
                match self.src.get(self.at) {
                    Some(b',') => self.at += 1,
                    Some(b'}') => {
                        self.at += 1;
                        return Ok(Json::Object(members));
                    }
                    _ => return self.fail("expected , or }"),
                }
            }
        }
    }

    fn parse(bytes: &[u8]) -> Result<Json, String> {
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        let mut parser = Parser { src: bytes, at: 0 };
        let value = parser.value()?;
        parser.ws();
        if parser.at != bytes.len() {
            return parser.fail("trailing content");
        }
        Ok(value)
    }

    fn write_string(s: &str, out: &mut String) {
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }

    fn write(value: &Json, out: &mut String) {
        match value {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(n) => out.push_str(n),
            Json::String(s) => write_string(s, out),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            Json::Object(members) => {
                let mut sorted: Vec<&(String, Json)> = members.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                out.push('{');
                for (i, (name, item)) in sorted.into_iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(name, out);
                    out.push(':');
                    write(item, out);
                }
                out.push('}');
            }
        }
    }

    /// The canonical text of one JSON document.
    pub fn canonicalise(bytes: &[u8]) -> Result<String, String> {
        let mut out = String::new();
        write(&parse(bytes)?, &mut out);
        Ok(out)
    }

    /// The top-level `resourceType`, read without the codec, so an example the
    /// codec cannot decode still counts toward its type's coverage.
    pub fn resource_type(bytes: &[u8]) -> Option<String> {
        match parse(bytes).ok()? {
            Json::Object(members) => members.into_iter().find_map(|(name, value)| match value {
                Json::String(s) if name == "resourceType" => Some(s),
                _ => None,
            }),
            _ => None,
        }
    }
}

// The `struct BoundGenerator` and `impl Codegen for BoundGenerator` this
// expands to are written in `frevo-codegen` itself, in `src/bind.rs`. They
// have to be expanded here rather than defined there because the trait and the
// data model above are test-local types that no library can name. Tasks 8 to 14
// extend the macro; nothing in this file changes when they do.
frevo_codegen::bind_oracle!(BoundGenerator);

// ---------------------------------------------------------------------------
// Harness helpers
// ---------------------------------------------------------------------------

/// The repository root, derived from the crate's manifest directory, so nothing
/// in this file names a machine or a worktree.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

pub fn repo_path(relative: &str) -> PathBuf {
    let mut p = repo_root();
    for part in relative.split('/') {
        p.push(part);
    }
    p
}

/// A fresh, empty scratch directory for one run's output.
pub fn scratch_dir(label: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "frevo-codegen-r4-{}-{label}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| {
        panic!(
            "the scratch directory {} must be creatable: {e}",
            dir.display()
        )
    });
    dir
}

/// Every file under `dir`, relative, with `/` separators, sorted.
pub fn files_under(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The interim round-trip example set: the `*.json` and `*.xml` files in
/// `oracles/fhir-test-cases/fetched/r4/examples/`, with the `*.json.out`
/// sql-on-FHIR fixtures excluded because they carry no `resourceType`.
pub struct ExampleSet {
    pub json: Vec<PathBuf>,
    pub xml: Vec<PathBuf>,
    pub excluded_out_files: usize,
}

impl ExampleSet {
    pub fn all(&self) -> Vec<PathBuf> {
        let mut v = self.json.clone();
        v.extend(self.xml.iter().cloned());
        v
    }

    pub fn file_names(&self) -> Vec<String> {
        self.all()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect()
    }
}

/// Read the interim example set off disk.
///
/// The directory is fetched, not vendored (`oracles/PINS.md`). A missing tree
/// is a loud failure naming the fetch command, never a pass.
pub fn example_set() -> ExampleSet {
    let dir = repo_path(EXAMPLE_DIR);
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "the interim round-trip example set {} is not present ({e}). It is fetched, \
             not vendored: run `bash oracles/fetch.sh`.",
            dir.display()
        )
    });
    let mut json = Vec::new();
    let mut xml = Vec::new();
    let mut excluded_out_files = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.ends_with(".json.out") {
            excluded_out_files += 1;
        } else if name.ends_with(".json") {
            json.push(path);
        } else if name.ends_with(".xml") {
            xml.push(path);
        }
    }
    json.sort();
    xml.sort();
    ExampleSet {
        json,
        xml,
        excluded_out_files,
    }
}

/// The Rust 2024 strict keywords, exactly as `codegen.naming.002` lists them
/// (38 names).
pub const RUST_STRICT_KEYWORDS: [&str; 38] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

/// The Rust 2024 reserved keywords, exactly as `codegen.naming.002` lists them
/// (14 names, `gen` included: it is reserved from the 2024 edition).
pub const RUST_RESERVED_KEYWORDS: [&str; 14] = [
    "abstract", "become", "box", "do", "final", "gen", "macro", "override", "priv", "try",
    "typeof", "unsized", "virtual", "yield",
];

/// Rust's keywords, strict and reserved: the 52 names `codegen.naming.002`
/// escapes, so "a deterministic non-keyword Rust name" is checkable rather
/// than asserted.
pub const RUST_KEYWORDS: [&str; 52] = {
    let mut all = [""; 52];
    let mut i = 0;
    while i < RUST_STRICT_KEYWORDS.len() {
        all[i] = RUST_STRICT_KEYWORDS[i];
        i += 1;
    }
    let mut j = 0;
    while j < RUST_RESERVED_KEYWORDS.len() {
        all[i + j] = RUST_RESERVED_KEYWORDS[j];
        j += 1;
    }
    all
};

/// Weak keywords: valid identifiers, which `codegen.naming.002` leaves as they
/// are.
pub const RUST_WEAK_KEYWORDS: [&str; 4] = ["union", "raw", "safe", "macro_rules"];

/// `true` when `name` is a plain, non-raw, non-keyword Rust identifier.
pub fn is_plain_rust_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with("r#")
        && !RUST_KEYWORDS.contains(&name)
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `true` for 64 lower-case hex characters.
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// The byte offset at which `needle` first occurs, or a panic naming the
/// haystack. Used to assert property order in encoded output without a JSON
/// parser.
pub fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} must appear in {haystack:?}"))
}

/// One pinned run, into a fresh scratch directory. A refusal or an unbound seam
/// is a panic naming what failed, so a test that needs a successful run cannot
/// quietly proceed without one.
pub fn pinned_run(generator: &dyn Codegen, label: &str) -> GenerateReport {
    let out = scratch_dir(label);
    let request = GenerateRequest::pinned(&out);
    generator.generate(&request).unwrap_or_else(|e| {
        panic!("`frevo-codegen generate --version r4` must succeed for {label}: {e}")
    })
}

/// Format `source` the way `cargo fmt` would, by piping it through the
/// toolchain's `rustfmt` (`codegen.output.004`).
pub fn rustfmt(source: &str, path: &str) -> String {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--quiet", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            panic!(
                "rustfmt must be on PATH to check {path} against the format gate: {e}. \
                 It ships with the pinned toolchain in rust-toolchain.toml."
            )
        });
    child
        .stdin
        .as_mut()
        .expect("rustfmt stdin")
        .write_all(source.as_bytes())
        .unwrap_or_else(|e| panic!("writing {path} to rustfmt: {e}"));
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("rustfmt on {path}: {e}"));
    assert!(
        out.status.success(),
        "rustfmt rejected {path}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap_or_else(|e| panic!("rustfmt output for {path} must be UTF-8: {e}"))
}

// ---------------------------------------------------------------------------
// Hand-written documents, one per decode condition
// ---------------------------------------------------------------------------

/// An `Observation` with the `1..1` `status` element absent
/// (`codegen.cardinality.002`).
const OBSERVATION_WITHOUT_STATUS: &str = concat!(
    r#"{"resourceType":"Observation","#,
    r#""code":{"text":"glucose"},"#,
    r#""valueString":"6.3"}"#
);

/// The same, with `status` present but blank (`codegen.cardinality.005`).
const OBSERVATION_WITH_BLANK_STATUS: &str = concat!(
    r#"{"resourceType":"Observation","#,
    r#""status":"","#,
    r#""code":{"text":"glucose"}}"#
);

/// Two values for one choice element (`codegen.choice.004`).
const OBSERVATION_WITH_TWO_CHOICE_VALUES: &str = concat!(
    r#"{"resourceType":"Observation","#,
    r#""status":"final","#,
    r#""code":{"text":"glucose"},"#,
    r#""valueString":"6.3","#,
    r#""valueQuantity":{"value":6.3,"unit":"mmol/L"}}"#
);

/// An `Observation` whose `code` carries a code no value set declares
/// (`codegen.binding.003`).
const OBSERVATION_WITH_LOCAL_CODE: &str = concat!(
    r#"{"resourceType":"Observation","status":"final","#,
    r#""code":{"coding":[{"system":"http://frevo.test/CodeSystem/local","#,
    r#""code":"not-a-loinc-code"}]}}"#
);

/// Three `Patient.name` entries, in an order that is not alphabetical, so a
/// re-serialisation that sorted them would be detectable
/// (`codegen.cardinality.003`).
const PATIENT_WITH_THREE_NAMES: &str = concat!(
    r#"{"resourceType":"Patient","name":["#,
    r#"{"family":"Smith","given":["Peter"]},"#,
    r#"{"family":"Jones"},"#,
    r#"{"family":"Brown","given":["Jim","James"]}]}"#
);

const PATIENT_WITH_ONE_BIRTH_DATE: &str = r#"{"resourceType":"Patient","birthDate":"1974-12-25"}"#;
const PATIENT_WITHOUT_BIRTH_DATE: &str = r#"{"resourceType":"Patient","active":true}"#;
/// A `0..1` element given an array: the generated type cannot hold a list.
const PATIENT_WITH_BIRTH_DATE_ARRAY: &str =
    r#"{"resourceType":"Patient","birthDate":["1974-12-25","1980-01-01"]}"#;

const PATIENT_WITH_GENDER_FEMALE: &str = r#"{"resourceType":"Patient","gender":"female"}"#;
/// `M` is not in `administrative-gender` and does not case-fold onto one
/// (`codegen.binding.002`).
const PATIENT_WITH_GENDER_M: &str = r#"{"resourceType":"Patient","gender":"M"}"#;

/// `Appointment.participant` is `1..*`; an empty array gives it nothing
/// (`codegen.cardinality.004`).
const APPOINTMENT_WITH_EMPTY_PARTICIPANT: &str =
    r#"{"resourceType":"Appointment","status":"proposed","participant":[]}"#;
const APPOINTMENT_WITH_ONE_PARTICIPANT: &str = concat!(
    r#"{"resourceType":"Appointment","status":"proposed","#,
    r#""participant":[{"status":"accepted"}]}"#
);

/// Properties written in an order that is neither the snapshot's nor
/// alphabetical, so both serialisation orders are observable
/// (`codegen.roundtrip.004`).
const PATIENT_IN_SCRAMBLED_ORDER: &str = concat!(
    r#"{"resourceType":"Patient","#,
    r#""birthDate":"1974-12-25","#,
    r#""active":true,"#,
    r#""id":"scrambled"}"#
);

/// Five levels of `Questionnaire.item`, whose `item` child is a
/// `contentReference` back to itself (`codegen.types.008`).
const QUESTIONNAIRE_NESTED_FIVE_DEEP: &str = concat!(
    r#"{"resourceType":"Questionnaire","status":"draft","item":[{"#,
    r#""linkId":"1","type":"group","item":[{"#,
    r#""linkId":"1.1","type":"group","item":[{"#,
    r#""linkId":"1.1.1","type":"group","item":[{"#,
    r#""linkId":"1.1.1.1","type":"group","item":[{"#,
    r#""linkId":"1.1.1.1.1","type":"boolean""#,
    r#"}]}]}]}]}]}"#
);

/// A minimal `Observation` carrying one choice property, so each of the eleven
/// JSON property names can be driven through the decoder.
fn observation_with_value(property: &str, value_json: &str) -> String {
    format!(
        r#"{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},"{property}":{value_json}}}"#
    )
}

/// Minimal valid content for each of `Observation.value[x]`'s eleven types,
/// paired with the JSON property name that carries it.
const OBSERVATION_VALUE_FIXTURES: [(&str, &str, &str); 11] = [
    (
        "valueQuantity",
        "Quantity",
        r#"{"value":6.3,"unit":"mmol/L"}"#,
    ),
    ("valueCodeableConcept", "CodeableConcept", r#"{"text":"x"}"#),
    ("valueString", "string", r#""6.3""#),
    ("valueBoolean", "boolean", "true"),
    ("valueInteger", "integer", "3"),
    ("valueRange", "Range", r#"{"low":{"value":1,"unit":"mg"}}"#),
    (
        "valueRatio",
        "Ratio",
        r#"{"numerator":{"value":1,"unit":"mg"}}"#,
    ),
    (
        "valueSampledData",
        "SampledData",
        r#"{"origin":{"value":0,"unit":"mg"},"period":1,"dimensions":1,"data":"1 2 3"}"#,
    ),
    ("valueTime", "time", r#""10:30:00""#),
    ("valueDateTime", "dateTime", r#""2024-01-01T10:30:00Z""#),
    ("valuePeriod", "Period", r#"{"start":"2024-01-01"}"#),
];

/// Total rows in the generated table: the sum of every SearchParameter's `base`
/// list length, counted over `sp/search-parameters.json`.
pub const EXPECTED_SEARCH_PARAMETER_ROWS: usize = 1706;
/// `clinical-patient` declares the longest `base` list in R4.
pub const CLINICAL_PATIENT_BASES: usize = 32;

// ===========================================================================
// codegen.inputs -- the pinned, verified input package
// ===========================================================================

// covers: codegen.inputs.001
#[test]
fn generating_from_the_pinned_r4_package_succeeds() {
    let generator = generator_under_test();

    // The pin is recorded in the repository, not discovered at run time.
    let pin = generator
        .recorded_pin()
        .unwrap_or_else(|e| panic!("the repository must record the R4 package pin: {e}"));
    assert_eq!(pin.name, PACKAGE_NAME);
    assert_eq!(pin.version, PACKAGE_VERSION);
    assert!(
        is_sha256_hex(&pin.sha256),
        "the recorded digest must be 64 lower-case hex characters, not {:?}",
        pin.sha256
    );

    let out = scratch_dir("inputs-001");
    let request = GenerateRequest::pinned(&out);
    let report = generator.generate(&request).unwrap_or_else(|e| {
        panic!("`frevo-codegen generate --version {VERSION_SELECTOR}` must exit 0: {e}")
    });

    assert_eq!(report.exit_code, 0);
    // The run reports the package it verified: name, version and digest.
    assert_eq!(report.pin, pin, "the run must verify the recorded pin");
    for expected in [PACKAGE_NAME, PACKAGE_VERSION, pin.sha256.as_str()] {
        assert!(
            report.summary.contains(expected),
            "the run's summary must report {expected:?}; it said {:?}",
            report.summary
        );
    }
    // And it wrote something.
    assert!(
        !report.files.is_empty(),
        "a successful run must write output files"
    );
    assert_eq!(
        files_under(&out).len(),
        report.files.len(),
        "every file the run reports must be on disk, and no others"
    );
}

// covers: codegen.inputs.002
#[test]
fn a_digest_mismatch_refuses_to_generate() {
    let generator = generator_under_test();
    let out = scratch_dir("inputs-002");

    // A digest that is well formed and wrong.
    let wrong = "0".repeat(64);
    let request = GenerateRequest {
        expected_sha256: Some(wrong.clone()),
        ..GenerateRequest::pinned(&out)
    };

    let error = generator
        .generate(&request)
        .expect_err("a digest mismatch must refuse to generate");
    let (exit_code, kind, message) = error.refusal("a digest mismatch");
    assert_ne!(exit_code, 0, "the command must exit non-zero");
    assert_eq!(kind, FailureKind::DigestMismatch);

    // The error names the expected and the actual digest.
    assert!(
        message.contains(&wrong),
        "the error must name the expected digest {wrong}; it said {message:?}"
    );
    let actual = generator
        .recorded_pin()
        .expect("the pin is recorded")
        .sha256;
    assert!(
        message.contains(&actual),
        "the error must name the actual digest {actual}; it said {message:?}"
    );

    // And nothing was written.
    assert_eq!(
        files_under(&out),
        Vec::<String>::new(),
        "a refused run must write no output file"
    );
}

// covers: codegen.inputs.003
#[test]
fn a_missing_package_refuses_to_generate() {
    let generator = generator_under_test();
    let out = scratch_dir("inputs-003");
    let absent = scratch_dir("inputs-003-absent").join("not-fetched");
    assert!(!absent.exists());

    let request = GenerateRequest {
        package_dir: Some(absent),
        ..GenerateRequest::pinned(&out)
    };

    let error = generator
        .generate(&request)
        .expect_err("an absent package must refuse to generate");
    let (exit_code, kind, message) = error.refusal("an absent input package");
    assert_ne!(exit_code, 0);
    assert_eq!(kind, FailureKind::PackageMissing);

    // It names the package, the version, and the command that fetches it.
    assert!(
        message.contains(PACKAGE_NAME) && message.contains(PACKAGE_VERSION),
        "the error must name {PACKAGE_NAME} and {PACKAGE_VERSION}; it said {message:?}"
    );
    assert!(
        message.contains(".sh") || message.contains("fetch"),
        "the error must name the command that fetches the package; it said {message:?}"
    );

    assert_eq!(
        files_under(&out),
        Vec::<String>::new(),
        "a refused run must write no output file"
    );
}

// covers: codegen.inputs.004
#[test]
fn the_generator_does_not_read_the_reference_tree() {
    let generator = generator_under_test();

    // `reference/` is cloned read-only and git-ignored, so a generator that
    // read it would make the model unreproducible. Two runs: one that is told
    // where it would be, one that is not.
    let with_out = scratch_dir("inputs-004-with");
    let with = generator
        .generate(&GenerateRequest {
            reference_dir: Some(repo_path("reference")),
            ..GenerateRequest::pinned(&with_out)
        })
        .unwrap_or_else(|e| panic!("generation must succeed with reference/ named: {e}"));

    let without_out = scratch_dir("inputs-004-without");
    let without = generator
        .generate(&GenerateRequest::pinned(&without_out))
        .unwrap_or_else(|e| panic!("generation must succeed with reference/ absent: {e}"));

    assert_eq!(
        with.file_map(),
        without.file_map(),
        "output must be identical whether or not reference/ is present"
    );

    // And the stronger statement: no input the run opened was under
    // `reference/`.
    let reference = repo_path("reference");
    for report in [&with, &without] {
        assert!(
            !report.inputs_read.is_empty(),
            "a run must report the inputs it read"
        );
        for path in &report.inputs_read {
            assert!(
                !path.starts_with(&reference),
                "the generator must not read {} -- reference/ is a cross-check, not an input",
                path.display()
            );
        }
    }
}

// covers: codegen.inputs.005
#[test]
fn the_pinned_package_and_the_hapi_copy_agree() {
    let generator = generator_under_test();
    let hapi = repo_path(HAPI_PROFILE_DIR);

    // The package side must be scoped to the definitions those three files
    // carry: the package additionally ships 393 extension definitions in
    // `extension/extension-definitions.xml`, which are outside the comparison
    // basis the scenario names.
    let report = generator.cross_check(None, &hapi).unwrap_or_else(|e| {
        panic!(
            "the cross-check against the HAPI copy at {HAPI_PIN_SHA} must run: {e}. \
             It needs reference/ -- `bash reference/clone.sh` -- and the pinned package."
        )
    });

    assert_eq!(report.package_fhir_version, FHIR_VERSION);
    assert_eq!(report.hapi_fhir_version, FHIR_VERSION);
    assert_eq!(
        report.hapi_definition_count, HAPI_PROFILE_DEFINITIONS,
        "profiles-types.xml 63 + profiles-resources.xml 149 + profiles-others.xml 44"
    );
    assert!(
        !report.scope.is_empty(),
        "the report must state what the package side was scoped to"
    );

    assert_eq!(
        report.only_in_hapi,
        Vec::<String>::new(),
        "every canonical URL the HAPI copy declares must be in the pinned package"
    );
    assert_eq!(
        report.only_in_package,
        Vec::<String>::new(),
        "and the scoped package side must declare no URL the HAPI copy does not"
    );
    assert_eq!(
        report.element_disagreements,
        Vec::<ElementDisagreement>::new(),
        "every snapshot element's name, type and cardinality must agree"
    );
}

// covers: codegen.inputs.006
#[test]
fn the_input_inventory_is_reported_and_asserted() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "inputs-006");

    assert_eq!(
        report.inventory,
        Inventory {
            datatype_definitions: EXPECTED_DATATYPE_DEFINITIONS,
            resource_definitions: EXPECTED_RESOURCE_DEFINITIONS,
            concrete_resources: EXPECTED_CONCRETE_RESOURCES,
            abstract_resources: EXPECTED_ABSTRACT_RESOURCES,
            logical_resources: EXPECTED_LOGICAL_RESOURCES,
            search_parameters: EXPECTED_SEARCH_PARAMETERS,
        }
    );
    assert_eq!(
        report.inventory.concrete_resources
            + report.inventory.abstract_resources
            + report.inventory.logical_resources,
        report.inventory.resource_definitions,
        "the three resource classes must account for every resource definition"
    );
    for n in [
        EXPECTED_DATATYPE_DEFINITIONS.to_string(),
        EXPECTED_RESOURCE_DEFINITIONS.to_string(),
        EXPECTED_SEARCH_PARAMETERS.to_string(),
    ] {
        assert!(
            report.summary.contains(&n),
            "the run must state the inventory count {n}; it said {:?}",
            report.summary
        );
    }

    // A count that differs fails the run rather than silently generating a
    // different model. One definition short of the pinned set is the cheapest
    // way to say so.
    let out = scratch_dir("inputs-006-short");
    let short = SyntheticRequest {
        assert_pinned_inventory: true,
        ..SyntheticRequest::new(
            vec![complex_type(
                "Thing",
                vec![element("Thing.label", 0, "1", &["string"])],
            )],
            Vec::new(),
            &out,
        )
    };
    let error = generator
        .generate_synthetic(&short)
        .expect_err("an input set that is not the pinned R4 inventory must fail the run");
    let (exit_code, kind, message) = error.refusal("an inventory short of the pinned counts");
    assert_ne!(exit_code, 0);
    assert_eq!(kind, FailureKind::InventoryMismatch);
    assert!(
        message.contains(&EXPECTED_DATATYPE_DEFINITIONS.to_string())
            || message.contains(&EXPECTED_RESOURCE_DEFINITIONS.to_string()),
        "the failure must name the count it expected; it said {message:?}"
    );
    assert_eq!(
        files_under(&out),
        Vec::<String>::new(),
        "a run that fails its inventory assertion must write nothing"
    );
}

// ===========================================================================
// codegen.types -- datatypes, resources, backbones, cycles
// ===========================================================================

// covers: codegen.types.001
#[test]
fn every_primitive_datatype_is_generated() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-001");

    assert_eq!(R4_PRIMITIVES.len(), 20);
    for primitive in R4_PRIMITIVES {
        let ty = report.ty_or_panic(primitive);
        assert_eq!(
            ty.kind,
            TypeKind::Primitive,
            "{primitive} is a primitive-type StructureDefinition"
        );
        // Each carries the `id` and `extension` properties `Element` declares.
        let id = ty.element_or_panic("id");
        assert_eq!(id.max, Max::One);
        assert_eq!(id.min, 0);
        let extension = ty.element_or_panic("extension");
        assert_eq!(extension.types, vec!["Extension".to_string()]);
        // And the value itself.
        let value = ty.element_or_panic("value");
        // `xhtml.value` is the only required primitive value: upstream
        // profiles-types.xml declares it min=1 max=1, and the xhtml-specific
        // assertion below says so too. Asserting 0 for all twenty contradicted
        // that, so this test could not pass against a correct generator.
        let expected_min = u32::from(primitive == "xhtml");
        assert_eq!(value.min, expected_min, "{primitive}.value");
        assert_eq!(value.max, Max::One, "{primitive}.value");
    }

    assert_eq!(
        report.types_of_kind(TypeKind::Primitive).len(),
        R4_PRIMITIVES.len(),
        "exactly the 20 primitives, so a 21st would fail rather than appear"
    );

    // The one cardinality the snapshot narrows: `xhtml.extension` is 0..0, and
    // `xhtml.value` is the only required primitive value.
    let xhtml = report.ty_or_panic("xhtml");
    assert_eq!(
        xhtml.element_or_panic("extension").max,
        Max::Zero,
        "profiles-types.xml gives xhtml.extension a maximum of 0"
    );
    assert_eq!(xhtml.element_or_panic("value").min, 1);
}

// covers: codegen.types.002
#[test]
fn every_complex_datatype_is_generated() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-002");

    for named in NAMED_COMPLEX_DATATYPES {
        let ty = report.ty_or_panic(named);
        assert_eq!(ty.kind, TypeKind::Complex, "{named}");
        assert!(
            !ty.elements.is_empty(),
            "{named} must be generated with its element set"
        );
    }

    // Every complex datatype the file declares in its own right, not only the
    // ones the scenario lists.
    assert_eq!(
        ALL_COMPLEX_SPECIALIZATIONS.len(),
        EXPECTED_COMPLEX_SPECIALIZATIONS
    );
    for name in ALL_COMPLEX_SPECIALIZATIONS {
        let ty = report.ty_or_panic(name);
        assert_eq!(ty.kind, TypeKind::Complex, "{name}");
    }
    for named in NAMED_COMPLEX_DATATYPES {
        assert!(
            ALL_COMPLEX_SPECIALIZATIONS.contains(&named),
            "{named} must be one of the 40 complex specializations"
        );
    }

    // `Element`, the abstract root of every complex datatype, is declared in
    // the same file with no `derivation` at all, and is generated too.
    assert_eq!(report.ty_or_panic("Element").kind, TypeKind::Complex);
    assert_eq!(
        report.types_of_kind(TypeKind::Complex).len(),
        EXPECTED_COMPLEX_DATATYPES,
        "all 43 complex-type definitions profiles-types.xml declares"
    );

    // `Dosage` is in the scenario's list and is based on `BackboneElement`, not
    // on `Element`; it is still a datatype and not a backbone type.
    let dosage = report.ty_or_panic("Dosage");
    assert_eq!(dosage.base.as_deref(), Some("BackboneElement"));
    assert_eq!(dosage.kind, TypeKind::Complex);
}

// covers: codegen.types.003
#[test]
fn metadata_types_are_generated() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-003");

    for name in METADATA_TYPES {
        let ty = report.ty_or_panic(name);
        assert_eq!(ty.kind, TypeKind::Complex, "{name}");
        assert!(!ty.elements.is_empty(), "{name} must carry its elements");
        assert!(
            is_plain_rust_identifier(&ty.rust_name),
            "{name} must have a usable Rust name, not {:?}",
            ty.rust_name
        );
    }

    // Two of them are the ones the codec depends on, so their shape is
    // asserted rather than assumed.
    let extension = report.ty_or_panic("Extension");
    assert_eq!(extension.element_or_panic("url").min, 1);
    assert!(extension.element_or_panic("value").is_choice);
    let meta = report.ty_or_panic("Meta");
    for element in [
        "versionId",
        "lastUpdated",
        "source",
        "profile",
        "security",
        "tag",
    ] {
        meta.element_or_panic(element);
    }
}

// covers: codegen.types.004
#[test]
fn abstract_types_are_shared_element_sets_not_instantiable_resources() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-004");

    let resources = report.types_of_kind(TypeKind::Resource);
    assert_eq!(resources.len(), EXPECTED_CONCRETE_RESOURCES);

    for resource in &resources {
        for element in RESOURCE_ELEMENTS {
            resource.element_or_panic(element);
        }
        let expects_domain = !NON_DOMAIN_RESOURCES.contains(&resource.fhir_name.as_str());
        for element in DOMAIN_RESOURCE_ELEMENTS {
            let present = resource.element(element).is_some();
            assert_eq!(
                present,
                expects_domain,
                "{}.{element}: baseDefinition is {:?}, so DomainResource's elements must be {}",
                resource.fhir_name,
                resource.base,
                if expects_domain { "present" } else { "absent" }
            );
        }
    }

    // The two abstract definitions are element sets, and nothing can hold one.
    for name in ABSTRACT_RESOURCES {
        let ty = report.ty_or_panic(name);
        assert_eq!(ty.kind, TypeKind::AbstractElementSet, "{name}");
        assert!(
            !ty.instantiable,
            "no value of type {name} may be constructible"
        );
        assert!(
            !report.resource_registry.iter().any(|e| e.fhir_name == name),
            "{name} must not appear in the resource-type registry"
        );
    }

    // And the generated source carries no constructible definition for either.
    for file in &report.files {
        for name in ABSTRACT_RESOURCES {
            for forbidden in [
                format!("pub struct {name} {{"),
                format!("pub struct {name};"),
            ] {
                assert!(
                    !file.text().contains(&forbidden),
                    "{} declares {forbidden}, which makes {name} constructible",
                    file.path
                );
            }
        }
    }
}

// covers: codegen.types.005
#[test]
fn all_146_concrete_resource_types_are_generated() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-005");

    let resources = report.types_of_kind(TypeKind::Resource);
    assert_eq!(
        resources.len(),
        EXPECTED_CONCRETE_RESOURCES,
        "profiles-resources.xml declares 146 definitions with kind=resource, \
         abstract=false and derivation=specialization"
    );
    for resource in &resources {
        assert!(resource.instantiable, "{}", resource.fhir_name);
        assert!(
            !resource.elements.is_empty(),
            "{} must carry its elements",
            resource.fhir_name
        );
    }

    // The registry maps each FHIR type name to its type and back.
    assert_eq!(report.resource_registry.len(), EXPECTED_CONCRETE_RESOURCES);
    let by_fhir: BTreeSet<&str> = report
        .resource_registry
        .iter()
        .map(|e| e.fhir_name.as_str())
        .collect();
    let by_rust: BTreeSet<&str> = report
        .resource_registry
        .iter()
        .map(|e| e.rust_name.as_str())
        .collect();
    assert_eq!(
        by_fhir.len(),
        EXPECTED_CONCRETE_RESOURCES,
        "the registry must be keyed one-to-one by FHIR type name"
    );
    assert_eq!(
        by_rust.len(),
        EXPECTED_CONCRETE_RESOURCES,
        "and one-to-one by Rust type name, so the mapping is reversible"
    );
    let generated: BTreeSet<&str> = resources.iter().map(|t| t.fhir_name.as_str()).collect();
    assert_eq!(
        by_fhir, generated,
        "the registry must name exactly the generated resource types"
    );
    for entry in &report.resource_registry {
        let ty = report.ty_or_panic(&entry.fhir_name);
        assert_eq!(
            ty.rust_name, entry.rust_name,
            "the registry's Rust name for {} must be the type's own",
            entry.fhir_name
        );
    }

    // A spot check that the set is the R4 set and not a subset of it.
    for name in [
        "Patient",
        "Observation",
        "Binary",
        "Bundle",
        "Parameters",
        "Questionnaire",
    ] {
        assert!(by_fhir.contains(name), "{name} must be in the registry");
    }
    assert!(
        !by_fhir.contains("MetadataResource"),
        "the one logical definition is not a concrete resource"
    );
}

// covers: codegen.types.006
#[test]
fn backbone_elements_become_named_nested_types() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-006");

    for path in ["Patient.contact", "Patient.communication", "Patient.link"] {
        let ty = report.ty_or_panic(path);
        assert_eq!(ty.kind, TypeKind::Backbone, "{path}");
        assert!(
            is_plain_rust_identifier(&ty.rust_name),
            "{path} must have a usable Rust name, not {:?}",
            ty.rust_name
        );
    }

    // Each nested type's element set matches the snapshot path prefix it came
    // from: `Patient.contact.*`, and nothing else.
    let contact = report.ty_or_panic("Patient.contact");
    assert_eq!(
        contact.element_names(),
        PATIENT_CONTACT_ELEMENT_ORDER.to_vec()
    );
    for element in &contact.elements {
        assert_eq!(
            element.path,
            format!("Patient.contact.{}", element.fhir_name),
            "every element of Patient.contact must come from that path prefix"
        );
    }
    let communication = report.ty_or_panic("Patient.communication");
    assert_eq!(
        communication.element_names(),
        vec![
            "id",
            "extension",
            "modifierExtension",
            "language",
            "preferred"
        ]
    );
    let link = report.ty_or_panic("Patient.link");
    assert_eq!(
        link.element_names(),
        vec!["id", "extension", "modifierExtension", "other", "type"]
    );

    // And `Patient` itself points at them.
    let patient = report.ty_or_panic("Patient");
    for name in ["contact", "communication", "link"] {
        let element = patient.element_or_panic(name);
        assert_eq!(element.max, Max::Many);
        assert_eq!(
            element.types,
            vec!["BackboneElement".to_string()],
            "Patient.{name} is declared as a BackboneElement"
        );
    }
}

// covers: codegen.types.007
#[test]
fn deeply_nested_backbone_elements_are_generated_at_every_level() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "types-007");

    for path in [
        "Questionnaire.item",
        "Questionnaire.item.enableWhen",
        "Questionnaire.item.answerOption",
        "Questionnaire.item.initial",
    ] {
        let ty = report.ty_or_panic(path);
        assert_eq!(ty.kind, TypeKind::Backbone, "{path}");
        for element in &ty.elements {
            // A choice element is addressed as `value` but keeps `[x]` in its
            // snapshot path -- which is what the `path` field documents and
            // what codegen.choice.001 and .003 assert. Three of these four
            // backbone types own a choice element, so the bare format! could
            // never match and this test could not pass against a correct
            // generator.
            let expected = if element.is_choice {
                format!("{path}.{}[x]", element.fhir_name)
            } else {
                format!("{path}.{}", element.fhir_name)
            };
            assert_eq!(
                element.path, expected,
                "{path} must take its elements from its own snapshot prefix"
            );
        }
    }

    // The second-level types are real: `enableWhen` has the three elements the
    // snapshot gives it beyond the inherited ones, and `answerOption` and
    // `initial` are choice-bearing.
    let enable_when = report.ty_or_panic("Questionnaire.item.enableWhen");
    for element in ["question", "operator", "answer"] {
        enable_when.element_or_panic(element);
    }
    assert!(enable_when.element_or_panic("answer").is_choice);
    assert!(
        report
            .ty_or_panic("Questionnaire.item.answerOption")
            .element_or_panic("value")
            .is_choice
    );
    assert!(
        report
            .ty_or_panic("Questionnaire.item.initial")
            .element_or_panic("value")
            .is_choice
    );

    // The four nested types are distinct Rust types.
    let names: BTreeSet<&str> = [
        "Questionnaire.item",
        "Questionnaire.item.enableWhen",
        "Questionnaire.item.answerOption",
        "Questionnaire.item.initial",
    ]
    .iter()
    .map(|p| report.ty_or_panic(p).rust_name.as_str())
    .collect();
    assert_eq!(
        names.len(),
        4,
        "each level needs its own Rust type: {names:?}"
    );
}

// covers: codegen.types.008
#[test]
fn recursive_backbone_elements_terminate() {
    let generator = generator_under_test();

    // Termination is the first half: the run completes at all.
    let report = pinned_run(generator.as_ref(), "types-008");
    let item = report.ty_or_panic("Questionnaire.item");
    let child = item.element_or_panic("item");
    assert_eq!(
        child.content_reference.as_deref(),
        Some("#Questionnaire.item"),
        "the snapshot declares Questionnaire.item.item by contentReference"
    );
    assert_eq!(child.max, Max::Many);
    // Exactly one nested type for the path, not one per level.
    assert_eq!(
        report
            .types
            .iter()
            .filter(|t| t.fhir_name == "Questionnaire.item")
            .count(),
        1,
        "a recursive backbone must be one type, not a tower of them"
    );
    assert!(
        !report
            .types
            .iter()
            .any(|t| t.fhir_name == "Questionnaire.item.item"),
        "the contentReference must resolve to Questionnaire.item, not generate a new type"
    );

    // The second half: the recursive child is reachable to unbounded depth at
    // run time, which a fixed expansion depth would break.
    let decoded = generator
        .decode(QUESTIONNAIRE_NESTED_FIVE_DEEP, DecodeMode::Lenient)
        .unwrap_or_else(|e| panic!("a five-deep Questionnaire.item must decode: {e}"));
    assert_eq!(decoded.resource_type, "Questionnaire");
    let deepest = "Questionnaire.item[0].item[0].item[0].item[0].item[0].linkId";
    let values = decoded.at(deepest);
    assert_eq!(
        values.len(),
        1,
        "the recursive child must be reachable at depth 5: {deepest}"
    );
    assert_eq!(values[0].value.as_deref(), Some("1.1.1.1.1"));
    assert_eq!(
        decoded.diagnostics,
        Vec::<Diagnostic>::new(),
        "a well-formed nested Questionnaire raises nothing"
    );
}

// covers: codegen.types.009
#[test]
fn type_cycles_across_datatypes_terminate() {
    let generator = generator_under_test();

    // `Reference` contains `Identifier`, which contains `Reference`. The run
    // completing is the termination; both types being complete is the rest.
    let report = pinned_run(generator.as_ref(), "types-009");

    let reference = report.ty_or_panic("Reference");
    assert_eq!(
        reference.element_names(),
        vec![
            "id",
            "extension",
            "reference",
            "type",
            "identifier",
            "display"
        ]
    );
    assert_eq!(
        reference.element_or_panic("identifier").types,
        vec!["Identifier".to_string()]
    );

    let identifier = report.ty_or_panic("Identifier");
    assert_eq!(
        identifier.element_names(),
        vec![
            "id",
            "extension",
            "use",
            "type",
            "system",
            "value",
            "period",
            "assigner"
        ]
    );
    assert_eq!(
        identifier.element_or_panic("assigner").types,
        vec!["Reference".to_string()],
        "Identifier.assigner closes the cycle"
    );

    // Neither type was truncated to break the cycle.
    for ty in [reference, identifier] {
        assert_eq!(
            ty.element_order,
            ty.element_names()
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>(),
            "{}'s element order must cover its whole element set",
            ty.fhir_name
        );
    }
}

// ===========================================================================
// codegen.choice -- value[x] under its base name
// ===========================================================================

// covers: codegen.choice.001
#[test]
fn a_choice_element_is_generated_under_its_base_name() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "choice-001");

    let observation = report.ty_or_panic("Observation");
    let value = observation.element_or_panic("value");
    assert!(value.is_choice);
    assert_eq!(
        value.path, "Observation.value[x]",
        "the element keeps its snapshot path while being addressed as `value`"
    );
    assert_eq!(value.min, 0);
    assert_eq!(value.max, Max::One);

    // The declared type set, in snapshot order.
    assert_eq!(
        value.types,
        OBSERVATION_VALUE_TYPES
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
    );

    // Every JSON property name maps to it, and only those.
    assert_eq!(
        value.json_property_names,
        OBSERVATION_VALUE_PROPERTIES
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
    );

    // The typed spelling is not itself an element: FHIRPath navigates by the
    // base name, so `Observation.valueQuantity` must not exist as one.
    for property in OBSERVATION_VALUE_PROPERTIES {
        assert!(
            observation.element(property).is_none(),
            "{property} is a JSON property name, not an element name"
        );
    }

    // And the decoded value carries which of the declared types it holds.
    for (property, expected_type, value_json) in OBSERVATION_VALUE_FIXTURES {
        let document = observation_with_value(property, value_json);
        let decoded = generator
            .decode(&document, DecodeMode::Lenient)
            .unwrap_or_else(|e| panic!("{property} must decode: {e}"));
        let values = decoded.at("Observation.value");
        assert_eq!(
            values.len(),
            1,
            "{property} must decode to exactly one Observation.value"
        );
        assert_eq!(
            values[0].chosen_type.as_deref(),
            Some(expected_type),
            "{property} holds a {expected_type}"
        );
        // And re-encoding emits the same property name it came in on.
        let encoded = generator
            .encode(&decoded)
            .unwrap_or_else(|e| panic!("{property} must re-encode: {e}"));
        assert!(
            encoded.contains(&format!("\"{property}\"")),
            "re-encoding must emit {property}; it emitted {encoded}"
        );
    }
}

// covers: codegen.choice.002
#[test]
fn every_declared_choice_type_is_representable() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "choice-002");

    let value = report.ty_or_panic("Extension").element_or_panic("value");
    assert!(value.is_choice);
    assert_eq!(
        value.types.len(),
        EXTENSION_VALUE_TYPE_COUNT,
        "Extension.value[x] permits every open type: 50 of them"
    );
    assert_eq!(
        value.json_property_names.len(),
        EXTENSION_VALUE_TYPE_COUNT,
        "one JSON property name per declared type"
    );
    let distinct: BTreeSet<&str> = value
        .json_property_names
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        distinct.len(),
        EXTENSION_VALUE_TYPE_COUNT,
        "the property names must be distinct"
    );

    // The four the scenario names, plus the pair that is easy to conflate:
    // `Age`, `Duration` and `Count` are all profiles on `Quantity`, and each
    // must be separately representable.
    for (type_name, property) in [
        ("Age", "valueAge"),
        ("Quantity", "valueQuantity"),
        ("Duration", "valueDuration"),
        ("Reference", "valueReference"),
        ("Count", "valueCount"),
    ] {
        assert!(
            value.types.iter().any(|t| t == type_name),
            "Extension.value[x] declares {type_name}"
        );
        assert!(
            value.json_property_names.iter().any(|p| p == property),
            "and {property} must map to it"
        );
    }

    // Representable at run time, not only in the type table.
    for (type_name, property, value_json) in [
        ("Age", "valueAge", r#"{"value":3,"unit":"a"}"#),
        ("Quantity", "valueQuantity", r#"{"value":3,"unit":"mg"}"#),
        ("Duration", "valueDuration", r#"{"value":3,"unit":"min"}"#),
        (
            "Reference",
            "valueReference",
            r#"{"reference":"Patient/example"}"#,
        ),
    ] {
        let document = format!(
            r#"{{"resourceType":"Patient","extension":[{{"url":"http://frevo.test/x","{property}":{value_json}}}]}}"#
        );
        let decoded = generator
            .decode(&document, DecodeMode::Lenient)
            .unwrap_or_else(|e| panic!("an extension carrying {property} must decode: {e}"));
        let values = decoded.at("Patient.extension[0].value");
        assert_eq!(values.len(), 1, "{property} must decode to one value");
        assert_eq!(
            values[0].chosen_type.as_deref(),
            Some(type_name),
            "{property} holds a {type_name}, which is not the same type as Quantity"
        );
    }
}

// covers: codegen.choice.003
#[test]
fn a_choice_element_with_a_single_permitted_type_is_still_a_choice() {
    let generator = generator_under_test();

    // No element in the pinned R4 core package has a `[x]` path with exactly
    // one `type` entry -- every core choice declares two or more -- so this
    // case is reached with a definition written here. A profile that
    // constrains `value[x]` to one type is the real-world instance.
    let out = scratch_dir("choice-003");
    let request = SyntheticRequest::new(
        vec![complex_type(
            "SingleChoice",
            vec![
                element("SingleChoice.id", 0, "1", &["string"]),
                element("SingleChoice.value[x]", 0, "1", &["string"]),
            ],
        )],
        Vec::new(),
        &out,
    );
    let report = generator
        .generate_synthetic(&request)
        .unwrap_or_else(|e| panic!("a one-type choice element must generate: {e}"));

    let value = report.ty_or_panic("SingleChoice").element_or_panic("value");
    assert!(
        value.is_choice,
        "an element whose path ends in [x] is a choice however many types it declares"
    );
    assert_eq!(value.path, "SingleChoice.value[x]");
    assert_eq!(value.types, vec!["string".to_string()]);
    assert_eq!(
        value.json_property_names,
        vec!["valueString".to_string()],
        "a one-type choice is still addressed by its typed property name"
    );
    assert!(
        report
            .ty_or_panic("SingleChoice")
            .element("valueString")
            .is_none(),
        "and not by the typed spelling as an element name"
    );
    assert!(
        report
            .ty_or_panic("SingleChoice")
            .element_order
            .contains(&"value".to_string()),
        "the element-order table records the base name"
    );
}

// covers: codegen.choice.004
#[test]
fn a_second_choice_value_fails_a_strict_decode_with_hapi_1823_naming_value() {
    let generator = generator_under_test();

    // `ParserState.enteringNewElement` records each `0..1` child it enters in
    // `myParsedNonRepeatableNames` under the choice's *field* name --
    // `RuntimeChildChoiceDefinition.getField().getName()`, `value` for every
    // `value[x]` property -- so `valueQuantity` after `valueString` is a
    // repetition (ParserState.java:660-672). `StrictErrorHandler` throws
    // `Msg.code(1823)` with no location (StrictErrorHandler.java:76-79;
    // `describeLocation(null)` is empty, ParseErrorHandler.java:24-30), so the
    // message names `value` and neither JSON property.
    let error = generator
        .decode(OBSERVATION_WITH_TWO_CHOICE_VALUES, DecodeMode::Strict)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "two values for Observation.value[x] must fail a strict decode with HAPI's \
                 repeated-element error (code {CODE_UNEXPECTED_REPEATING_ELEMENT})"
            )
        });
    let (_, kind, message) = error.refusal("two values for one choice element, strict");
    assert_eq!(kind, FailureKind::Codec);
    assert_eq!(
        message,
        "HAPI-1823: Multiple repetitions of non-repeatable element 'value' found during parse",
        "HAPI's StrictErrorHandler.unexpectedRepeatingElement text, exactly"
    );
    assert_eq!(CODE_UNEXPECTED_REPEATING_ELEMENT, 1823);

    // One value for the same element decodes cleanly in both modes, so the
    // failure is about the repetition and not about the element.
    for mode in [DecodeMode::Strict, DecodeMode::Lenient] {
        let single = generator
            .decode(&observation_with_value("valueString", r#""6.3""#), mode)
            .unwrap_or_else(|e| {
                panic!("one value for a choice element must decode in {mode:?} mode: {e}")
            });
        assert_eq!(single.at("Observation.value").len(), 1, "{mode:?}");
        assert_eq!(single.diagnostics, Vec::<Diagnostic>::new(), "{mode:?}");
    }
}

// covers: codegen.choice.004
#[test]
fn a_second_choice_value_is_dropped_with_a_diagnostic_in_a_lenient_decode() {
    let generator = generator_under_test();

    // `LenientErrorHandler.unexpectedRepeatingElement` only logs
    // (LenientErrorHandler.java:183-190), and the parser pushes
    // `SwallowChildrenWholeState` for the second property
    // (ParserState.java:670), so the first property in document order is the
    // value and the `Quantity` and everything under it are gone.
    let decoded = generator
        .decode(OBSERVATION_WITH_TWO_CHOICE_VALUES, DecodeMode::Lenient)
        .unwrap_or_else(|e| {
            panic!(
                "a lenient decode of two values for Observation.value[x] must succeed, \
                 keeping the first: {e}"
            )
        });

    let values = decoded.at("Observation.value");
    assert_eq!(
        values.len(),
        1,
        "Observation.value holds one value, the first: {values:?}"
    );
    assert_eq!(
        values[0].chosen_type.as_deref(),
        Some("string"),
        "the first property in document order, valueString, is the value"
    );
    assert_eq!(values[0].value.as_deref(), Some("6.3"));
    let swallowed: Vec<&ElementValue> = decoded
        .values
        .iter()
        .filter(|v| v.path.starts_with("Observation.value."))
        .collect();
    assert_eq!(
        swallowed,
        Vec::<&ElementValue>::new(),
        "the Quantity's children are swallowed with it"
    );

    // A repeated-element diagnostic naming `value`, where HAPI logs.
    assert_eq!(
        decoded.kinds(),
        vec![DiagnosticKind::DuplicateChoiceValue],
        "exactly one condition, the repeated choice element: {:?}",
        decoded.diagnostics
    );
    let diagnostic = decoded
        .diagnostic(DiagnosticKind::DuplicateChoiceValue)
        .expect("the repeated-element diagnostic");
    assert_eq!(diagnostic.path, "Observation.value");
    assert!(
        diagnostic.message.contains("'value'"),
        "the diagnostic names the choice field `value`: {:?}",
        diagnostic.message
    );
    assert_eq!(decoded.diagnostics.len(), 1, "{:?}", decoded.diagnostics);

    // A canonical encode of what was decoded emits only the first property.
    let canonical = generator
        .encode_canonical(&decoded)
        .unwrap_or_else(|e| panic!("canonical encoding must succeed: {e}"));
    assert!(
        canonical.contains(r#""valueString":"6.3""#),
        "the kept value is emitted: {canonical}"
    );
    assert!(
        !canonical.contains("valueQuantity"),
        "the dropped value is not part of the decoded resource: {canonical}"
    );
}

// ===========================================================================
// codegen.cardinality
// ===========================================================================

// covers: codegen.cardinality.001
#[test]
fn zero_to_one_is_optional_and_single() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "cardinality-001");

    let birth_date = report.ty_or_panic("Patient").element_or_panic("birthDate");
    assert_eq!(birth_date.min, 0);
    assert_eq!(birth_date.max, Max::One);
    assert_eq!(birth_date.types, vec!["date".to_string()]);

    // Absent.
    let absent = generator
        .decode(PATIENT_WITHOUT_BIRTH_DATE, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("a Patient with no birthDate must decode: {e}"));
    assert_eq!(absent.at("Patient.birthDate"), Vec::<&ElementValue>::new());
    assert_eq!(absent.diagnostics, Vec::<Diagnostic>::new());

    // Exactly one, with no index in the path because the element is single.
    let present = generator
        .decode(PATIENT_WITH_ONE_BIRTH_DATE, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("a Patient with one birthDate must decode: {e}"));
    let values = present.at("Patient.birthDate");
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value.as_deref(), Some("1974-12-25"));
    assert_eq!(
        present.repeats("Patient.birthDate"),
        Vec::<&ElementValue>::new(),
        "a single-valued element is not indexed"
    );

    // And the generated type cannot hold a list: a document that offers two
    // leaves at most one behind.
    let from_array = generator
        .decode(PATIENT_WITH_BIRTH_DATE_ARRAY, DecodeMode::Lenient)
        .unwrap_or_else(|e| panic!("an array for a 0..1 element must decode leniently: {e}"));
    assert!(
        from_array.at("Patient.birthDate").len() <= 1,
        "a 0..1 element must not hold a list; it held {:?}",
        from_array.at("Patient.birthDate")
    );
}

// covers: codegen.cardinality.002
#[test]
fn one_to_one_is_required_and_single_and_its_absence_is_not_checked_on_decode() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "cardinality-002");

    let status = report.ty_or_panic("Observation").element_or_panic("status");
    assert_eq!(status.max, Max::One, "the element holds at most one value");
    assert_eq!(status.min, 1, "and records that its minimum is 1");

    // HAPI's parser checks no minimum cardinality while decoding: `ParserState`
    // never consults a child's `getMin()`, and the four call sites of
    // `missingRequiredElement` in the JSON parser concern only an extension's
    // `url` and `value` (JsonParser.java:1503, :1514 on decode, :1995, :2004 on
    // encode). So neither error handler is reached, and a strict decode of
    // this document succeeds exactly as a lenient one does.
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let decoded = generator
            .decode(OBSERVATION_WITHOUT_STATUS, mode)
            .unwrap_or_else(|e| {
                panic!(
                    "a {mode:?} decode of an Observation with no status must succeed: HAPI's \
                     parser checks no minimum cardinality. It failed with: {e}"
                )
            });
        assert_eq!(
            decoded.at("Observation.status"),
            Vec::<&ElementValue>::new(),
            "{mode:?}: the element stays absent"
        );
        assert!(
            decoded
                .values
                .iter()
                .all(|v| !v.path.starts_with("Observation.status")),
            "{mode:?}: a required element must not be defaulted: {:?}",
            decoded.values
        );
        assert!(
            decoded
                .diagnostic(DiagnosticKind::MissingRequiredElement)
                .is_none(),
            "{mode:?}: no missing-required-element diagnostic"
        );
        assert_eq!(
            decoded.diagnostics,
            Vec::<Diagnostic>::new(),
            "{mode:?}: no diagnostic at all"
        );
    }
}

// covers: codegen.cardinality.003
#[test]
fn zero_to_many_is_an_ordered_list() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "cardinality-003");

    let name = report.ty_or_panic("Patient").element_or_panic("name");
    assert_eq!(name.min, 0);
    assert_eq!(name.max, Max::Many);

    let decoded = generator
        .decode(PATIENT_WITH_THREE_NAMES, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("three Patient.name entries must decode: {e}"));

    // The three values come back in document order, indexed.
    let families: Vec<&str> = (0..3)
        .map(|i| {
            let path = format!("Patient.name[{i}].family");
            let values = decoded.at(&path);
            assert_eq!(values.len(), 1, "{path} must be present");
            values[0]
                .value
                .as_deref()
                .unwrap_or_else(|| panic!("{path}"))
        })
        .collect();
    assert_eq!(
        families,
        vec!["Smith", "Jones", "Brown"],
        "document order, not sorted order"
    );

    // The repeating element reports exactly three occurrences at the element
    // itself, each indexed and each a complex node with no primitive value.
    for i in 0..3 {
        let path = format!("Patient.name[{i}]");
        let occurrences = decoded.at(&path);
        assert_eq!(
            occurrences.len(),
            1,
            "{path} must be one occurrence of the repeating element"
        );
        assert_eq!(
            occurrences[0].value, None,
            "{path} is complex and carries no primitive value"
        );
    }
    assert_eq!(
        decoded.at("Patient.name[3]"),
        Vec::<&ElementValue>::new(),
        "three entries in, three entries out"
    );

    // Nested repeats keep their own order too.
    assert_eq!(
        decoded.at("Patient.name[2].given[0]")[0].value.as_deref(),
        Some("Jim")
    );
    assert_eq!(
        decoded.at("Patient.name[2].given[1]")[0].value.as_deref(),
        Some("James")
    );

    // And re-serialising preserves that order.
    let encoded = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("re-encoding must succeed: {e}"));
    let smith = index_of(&encoded, "Smith");
    let jones = index_of(&encoded, "Jones");
    let brown = index_of(&encoded, "Brown");
    assert!(
        smith < jones && jones < brown,
        "re-serialisation must preserve document order: {encoded}"
    );
    let jim = index_of(&encoded, "Jim");
    let james = index_of(&encoded, "James");
    assert!(jim < james, "including inside a repeating primitive");
}

// covers: codegen.cardinality.004
#[test]
fn one_to_many_is_an_ordered_list_whose_minimum_is_recorded() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "cardinality-004");

    // `Appointment.participant` is 1..* in profiles-resources.xml.
    let participant = report
        .ty_or_panic("Appointment")
        .element_or_panic("participant");
    assert_eq!(participant.min, 1, "its minimum cardinality is recorded");
    assert_eq!(participant.max, Max::Many);

    // Order is preserved, as for any repeating element.
    let two = concat!(
        r#"{"resourceType":"Appointment","status":"proposed","participant":["#,
        r#"{"status":"accepted"},{"status":"declined"}]}"#
    );
    let decoded = generator
        .decode(two, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("two participants must decode: {e}"));
    assert_eq!(
        decoded.at("Appointment.participant[0].status")[0]
            .value
            .as_deref(),
        Some("accepted")
    );
    assert_eq!(
        decoded.at("Appointment.participant[1].status")[0]
            .value
            .as_deref(),
        Some("declined")
    );

    // One participant decodes cleanly too.
    let one = generator
        .decode(APPOINTMENT_WITH_ONE_PARTICIPANT, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("one participant must decode: {e}"));
    assert_eq!(one.diagnostics, Vec::<Diagnostic>::new());

    // An empty array for the `1..*` element: the JSON parser iterates the
    // array's entries and an empty array has none (JsonParser.java:1439-1457),
    // and no minimum cardinality is checked (`codegen.cardinality.002`). So
    // the decode succeeds in both modes, with the element absent and no
    // diagnostic.
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let decoded = generator
            .decode(APPOINTMENT_WITH_EMPTY_PARTICIPANT, mode)
            .unwrap_or_else(|e| {
                panic!(
                    "an empty array for a 1..* element must decode in {mode:?} mode: HAPI's \
                     parser checks no minimum cardinality. It failed with: {e}"
                )
            });
        assert_eq!(
            decoded.at("Appointment.participant"),
            Vec::<&ElementValue>::new(),
            "{mode:?}: the element is absent, not an empty list"
        );
        assert_eq!(
            decoded.repeats("Appointment.participant"),
            Vec::<&ElementValue>::new(),
            "{mode:?}"
        );
        assert!(
            decoded
                .diagnostic(DiagnosticKind::MissingRequiredElement)
                .is_none(),
            "{mode:?}: no missing-required-element diagnostic"
        );
        assert_eq!(
            decoded.diagnostics,
            Vec::<Diagnostic>::new(),
            "{mode:?}: no diagnostic at all"
        );
    }
}

// covers: codegen.cardinality.005
#[test]
fn a_required_element_present_but_blank_has_no_value_and_strict_mode_rejects_it() {
    let generator = generator_under_test();

    // `ParserState.java:1454-1457`: a `value` of `""` short-circuits before
    // `setValueAsString`, calling `invalidValue(location, "", "Attribute value
    // must not be empty (\"\")")`. `LenientErrorHandler.invalidValue` logs a
    // blank value and returns (LenientErrorHandler.java:100-108), which is why
    // a blank is the one invalid value that survives a lenient decode
    // (`model.json.023`).
    let lenient = generator
        .decode(OBSERVATION_WITH_BLANK_STATUS, DecodeMode::Lenient)
        .unwrap_or_else(|e| {
            panic!("a lenient decode of a blank required value must not fail: {e}")
        });
    let diagnostic = lenient
        .diagnostic(DiagnosticKind::InvalidValue)
        .unwrap_or_else(|| {
            panic!(
                "a blank value must return an invalid-value diagnostic where HAPI logs; it \
                 returned {:?}",
                lenient.diagnostics
            )
        });
    assert_eq!(
        diagnostic.path, "Observation.status",
        "the diagnostic must name the element"
    );
    assert!(
        diagnostic
            .message
            .contains(r#"Attribute value must not be empty ("")"#),
        "the diagnostic carries HAPI's invalid-value error: {:?}",
        diagnostic.message
    );
    assert!(
        lenient
            .diagnostic(DiagnosticKind::MissingRequiredElement)
            .is_none(),
        "no missing-required-element diagnostic (codegen.cardinality.002): {:?}",
        lenient.diagnostics
    );
    assert_eq!(
        lenient.kinds(),
        vec![DiagnosticKind::InvalidValue],
        "the invalid-value diagnostic is the only one: {:?}",
        lenient.diagnostics
    );
    assert!(
        lenient
            .at("Observation.status")
            .iter()
            .all(|v| v.value.is_none()),
        "Observation.status has no value, not an empty string: {:?}",
        lenient.at("Observation.status")
    );

    // The strict decode fails with `1821` (StrictErrorHandler.java:56-59),
    // the location rendered as `[element="<name>"] ` (ParseLocation.java:55-57).
    let error = generator
        .decode(OBSERVATION_WITH_BLANK_STATUS, DecodeMode::Strict)
        .expect_err("the strict decode of a blank required value must fail");
    let (_, kind, message) = error.refusal("a blank required value, strict");
    assert_eq!(kind, FailureKind::Codec);
    assert_eq!(
        message,
        r#"HAPI-1821: [element="status"] Invalid attribute value "": Attribute value must not be empty ("")"#,
        "HAPI's StrictErrorHandler.invalidValue text, exactly"
    );
    assert_eq!(CODE_INVALID_VALUE, 1821);
}

// ===========================================================================
// codegen.binding -- closed enumerations only where R4 closes the set
// ===========================================================================

// covers: codegen.binding.001
#[test]
fn a_required_binding_generates_a_closed_enumeration() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "binding-001");

    let gender = report.ty_or_panic("Patient").element_or_panic("gender");
    let binding = gender
        .binding
        .as_ref()
        .expect("Patient.gender is bound in the snapshot");
    assert_eq!(binding.strength, "required");
    assert_eq!(binding.value_set, ADMINISTRATIVE_GENDER_VALUE_SET);
    let enum_name = binding
        .enumeration
        .as_deref()
        .expect("a required binding with a resolvable value set generates an enumeration");

    let generated = report
        .enumeration_for(ADMINISTRATIVE_GENDER_VALUE_SET)
        .unwrap_or_else(|| {
            panic!(
                "an enumeration must exist for {ADMINISTRATIVE_GENDER_VALUE_SET}; \
                 {} were generated",
                report.enumerations.len()
            )
        });
    assert_eq!(generated.rust_name, enum_name);
    assert_eq!(
        generated.codes(),
        ADMINISTRATIVE_GENDER_CODES.to_vec(),
        "exactly the four codes administrative-gender declares, in ValueSet order"
    );
    assert!(
        generated
            .element_paths
            .iter()
            .any(|p| p == "Patient.gender"),
        "the enumeration must record which elements bind it: {:?}",
        generated.element_paths
    );
    for variant in &generated.variants {
        assert!(
            is_plain_rust_identifier(&variant.rust_name),
            "{:?} must be a usable Rust variant name",
            variant.rust_name
        );
    }

    // It round-trips to and from exactly those code strings.
    let variants = generator
        .code_variants(&ADMINISTRATIVE_GENDER_CODES)
        .unwrap_or_else(|e| panic!("the naming rule must answer for the gender codes: {e}"));
    assert_eq!(variants, generated.variants);
    let round_tripped: Vec<&str> = variants.iter().map(|v| v.code.as_str()).collect();
    assert_eq!(round_tripped, ADMINISTRATIVE_GENDER_CODES.to_vec());

    let decoded = generator
        .decode(PATIENT_WITH_GENDER_FEMALE, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("a code inside the binding must decode: {e}"));
    assert_eq!(
        decoded.at("Patient.gender")[0].value.as_deref(),
        Some("female")
    );
    assert_eq!(decoded.diagnostics, Vec::<Diagnostic>::new());
    let encoded = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("re-encoding must succeed: {e}"));
    assert!(
        encoded.contains(r#""gender":"female""#),
        "the code must re-serialise verbatim: {encoded}"
    );
}

// covers: codegen.binding.002
#[test]
fn a_code_outside_a_required_binding_is_rejected_on_decode_in_both_modes() {
    let generator = generator_under_test();

    // HAPI's R4 model holds `Patient.gender` as an `Enumeration`, whose factory
    // throws `IllegalArgumentException("Unknown AdministrativeGender code
    // 'M'")` (org.hl7.fhir.r4 Enumerations.java:256-269). `ParserState` reports
    // it through `invalidValue` with that message (ParserState.java:1482-1514),
    // and the default `LenientErrorHandler` hands a non-blank invalid value to
    // the strict handler (LenientErrorHandler.java:100-108), which throws
    // `Msg.code(1821)` (StrictErrorHandler.java:56-59). So both default modes
    // fail, and a server asked to store the resource answers 400.
    //
    // HAPI's own `CaseInsensitiveEnumParsingR4Test` keeps the raw code only
    // because it installs a mock error handler; no default configuration does.
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let error = generator
            .decode(PATIENT_WITH_GENDER_M, mode)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "\"gender\": \"M\" must fail a {mode:?} decode: M is not an \
                     AdministrativeGender code, and HAPI rejects it in both default modes \
                     (code {CODE_INVALID_VALUE})"
                )
            });
        let (_, kind, message) = error.refusal("a code outside a required binding");
        assert_eq!(kind, FailureKind::Codec, "{mode:?}");
        assert_eq!(
            message,
            r#"HAPI-1821: [element="gender"] Invalid attribute value "M": Unknown AdministrativeGender code 'M'"#,
            "{mode:?}: HAPI's text, exactly"
        );
    }

    // A code that matches a binding code ignoring case is corrected *and*
    // still reported first: `ParserState.java:1488-1503` calls `invalidValue`
    // with the case-discrepancy text, so it fails the same way in both modes.
    let male = r#"{"resourceType":"Patient","gender":"Male"}"#;
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let error = generator.decode(male, mode).err().unwrap_or_else(|| {
            panic!(
                "\"gender\": \"Male\" must fail a {mode:?} decode: HAPI reports the case \
                 discrepancy through invalidValue before correcting it"
            )
        });
        let (_, kind, message) = error.refusal("a code differing from a binding code in case");
        assert_eq!(kind, FailureKind::Codec, "{mode:?}");
        assert_eq!(
            message,
            r#"HAPI-1821: [element="gender"] Invalid attribute value "Male": Case discrepancy detected in code value 'Male' (should be 'male')"#,
            "{mode:?}: HAPI's text, exactly"
        );
    }

    // A blank code is a blank value and follows `codegen.cardinality.005`:
    // lenient keeps no value and returns a diagnostic, strict fails with 1821.
    let blank = r#"{"resourceType":"Patient","gender":""}"#;
    let lenient = generator
        .decode(blank, DecodeMode::Lenient)
        .unwrap_or_else(|e| panic!("a blank gender must survive a lenient decode: {e}"));
    let diagnostic = lenient
        .diagnostic(DiagnosticKind::InvalidValue)
        .unwrap_or_else(|| {
            panic!(
                "a blank code must return an invalid-value diagnostic; it returned {:?}",
                lenient.diagnostics
            )
        });
    assert_eq!(diagnostic.path, "Patient.gender");
    assert!(
        lenient
            .at("Patient.gender")
            .iter()
            .all(|v| v.value.is_none()),
        "a blank code leaves Patient.gender with no value: {:?}",
        lenient.at("Patient.gender")
    );
    let error = generator
        .decode(blank, DecodeMode::Strict)
        .expect_err("a blank gender must fail a strict decode");
    let (_, kind, message) = error.refusal("a blank code, strict");
    assert_eq!(kind, FailureKind::Codec);
    assert_eq!(
        message,
        r#"HAPI-1821: [element="gender"] Invalid attribute value "": Attribute value must not be empty ("")"#
    );

    // A code inside the binding decodes cleanly in both modes, so the
    // rejection is about the code and not about the element.
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let decoded = generator
            .decode(PATIENT_WITH_GENDER_FEMALE, mode)
            .unwrap_or_else(|e| panic!("a code inside the binding must decode in {mode:?}: {e}"));
        assert_eq!(decoded.diagnostics, Vec::<Diagnostic>::new(), "{mode:?}");
    }

    // The check is HAPI's `Enumeration` set, not every `required` binding: an
    // element HAPI holds as a `CodeType` accepts any code, even under a
    // required binding (`Attachment.contentType`, Attachment.java:58).
    let content_type = concat!(
        r#"{"resourceType":"Patient","#,
        r#""photo":[{"contentType":"application/x-frevo-not-registered"}]}"#
    );
    for mode in [DecodeMode::Lenient, DecodeMode::Strict] {
        let decoded = generator.decode(content_type, mode).unwrap_or_else(|e| {
            panic!("Attachment.contentType is a CodeType upstream and accepts any code: {e}")
        });
        assert_eq!(decoded.diagnostics, Vec::<Diagnostic>::new(), "{mode:?}");
    }
}

// covers: codegen.binding.003
#[test]
fn extensible_preferred_and_example_bindings_generate_no_enumeration() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "binding-003");

    // `Observation.code` is bound `example` to observation-codes.
    let code = report.ty_or_panic("Observation").element_or_panic("code");
    let binding = code
        .binding
        .as_ref()
        .expect("Observation.code is bound in the snapshot");
    assert_eq!(binding.strength, "example");
    assert_eq!(binding.value_set, OBSERVATION_CODES_VALUE_SET);
    assert_eq!(
        binding.enumeration, None,
        "no enumeration is generated for an example binding"
    );
    assert!(
        report
            .enumeration_for(OBSERVATION_CODES_VALUE_SET)
            .is_none(),
        "and none exists for its value set either"
    );

    // No enumeration anywhere comes from a binding weaker than `required`.
    let required_value_sets: BTreeSet<&str> = report
        .types
        .iter()
        .flat_map(|t| t.elements.iter())
        .filter_map(|e| e.binding.as_ref())
        .filter(|b| b.strength == "required")
        .map(|b| b.value_set.as_str())
        .collect();
    for generated in &report.enumerations {
        assert!(
            required_value_sets.contains(generated.value_set.as_str()),
            "{} came from {}, which no required binding names",
            generated.rust_name,
            generated.value_set
        );
    }
    for element in report.types.iter().flat_map(|t| t.elements.iter()) {
        if let Some(binding) = &element.binding {
            if binding.strength != "required" {
                assert_eq!(
                    binding.enumeration, None,
                    "{} is bound {} and must stay open",
                    element.path, binding.strength
                );
            }
        }
    }

    // And any code string is accepted.
    let decoded = generator
        .decode(OBSERVATION_WITH_LOCAL_CODE, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("a local code in an example-bound element must decode: {e}"));
    assert_eq!(
        decoded
            .at("Observation.code.coding[0].code")
            .first()
            .and_then(|v| v.value.as_deref()),
        Some("not-a-loinc-code")
    );
    assert_eq!(decoded.diagnostics, Vec::<Diagnostic>::new());
}

// covers: codegen.binding.004
#[test]
fn a_required_binding_whose_value_set_is_absent_generates_no_enumeration() {
    let generator = generator_under_test();

    // No element of the pinned R4 core package binds `required` to a value set
    // the package cannot resolve, so the condition is reached with a
    // definition written here.
    let unresolvable = "http://frevo.test/ValueSet/not-in-the-package";
    let out = scratch_dir("binding-004");
    let request = SyntheticRequest::new(
        vec![complex_type(
            "OpenlyBound",
            vec![
                element("OpenlyBound.id", 0, "1", &["string"]),
                bound_element(
                    "OpenlyBound.kind",
                    0,
                    "1",
                    &["code"],
                    "required",
                    unresolvable,
                ),
            ],
        )],
        Vec::new(),
        &out,
    );
    let report = generator
        .generate_synthetic(&request)
        .unwrap_or_else(|e| panic!("an unresolvable required binding must not fail the run: {e}"));

    let kind = report.ty_or_panic("OpenlyBound").element_or_panic("kind");
    let binding = kind.binding.as_ref().expect("the binding is recorded");
    assert_eq!(binding.strength, "required");
    assert_eq!(binding.value_set, unresolvable);
    assert_eq!(
        binding.enumeration, None,
        "no enumeration is generated, so the element stays open"
    );
    assert_eq!(
        report.enumerations,
        Vec::<GeneratedEnum>::new(),
        "and nothing was invented for it"
    );

    // The generator reports the element path and the unresolved URL.
    assert_eq!(
        report.unresolved_bindings,
        vec![UnresolvedBinding {
            element_path: "OpenlyBound.kind".to_string(),
            value_set_url: unresolvable.to_string(),
            strength: "required".to_string(),
        }],
        "an unresolvable required binding must be reported, not passed over"
    );

    // A resolvable one in the same shape does generate an enumeration, so the
    // absence above is about resolvability and not about synthetic input.
    let resolvable = "http://frevo.test/ValueSet/kinds";
    let out = scratch_dir("binding-004-resolvable");
    let request = SyntheticRequest::new(
        vec![complex_type(
            "ClosedlyBound",
            vec![bound_element(
                "ClosedlyBound.kind",
                0,
                "1",
                &["code"],
                "required",
                resolvable,
            )],
        )],
        vec![ValueSetInput {
            url: resolvable.to_string(),
            codes: vec!["one".to_string(), "two".to_string()],
        }],
        &out,
    );
    let report = generator
        .generate_synthetic(&request)
        .unwrap_or_else(|e| panic!("a resolvable required binding must generate: {e}"));
    let generated = report
        .enumeration_for(resolvable)
        .expect("a resolvable required binding generates an enumeration");
    assert_eq!(generated.codes(), vec!["one", "two"]);
    assert_eq!(report.unresolved_bindings, Vec::<UnresolvedBinding>::new());
}

// ===========================================================================
// codegen.naming -- total, collision-free, reversible
// ===========================================================================

/// `observation-status`, in `ObservationStatusEnumFactory` order. It carries
/// `entered-in-error`, which `codegen.naming.005` names.
pub const OBSERVATION_STATUS_CODES: [&str; 8] = [
    "registered",
    "preliminary",
    "final",
    "amended",
    "corrected",
    "cancelled",
    "entered-in-error",
    "unknown",
];

// covers: codegen.naming.001
#[test]
fn element_names_become_snake_case_fields() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "naming-001");

    // The three the scenario names, reached through the JSON property name it
    // spells them with.
    for (type_name, property, field, base_name) in [
        (
            "Patient",
            "multipleBirthBoolean",
            "multiple_birth",
            "multipleBirth",
        ),
        ("Patient", "deceasedDateTime", "deceased", "deceased"),
        ("Observation", "effectiveTiming", "effective", "effective"),
    ] {
        let ty = report.ty_or_panic(type_name);
        let element = ty
            .elements
            .iter()
            .find(|e| e.json_property_names.iter().any(|p| p == property))
            .unwrap_or_else(|| panic!("{property} must map to an element of {type_name}"));
        assert_eq!(
            element.rust_name, field,
            "{property} belongs to the choice element addressed as {base_name}"
        );
        assert_eq!(
            element.fhir_name, base_name,
            "and each element records its FHIR property name"
        );
        assert!(element.is_choice);
    }

    // The rule itself, over the snapshot paths.
    for (path, field) in [
        ("Patient.multipleBirth[x]", "multiple_birth"),
        ("Patient.deceased[x]", "deceased"),
        ("Observation.effective[x]", "effective"),
        ("Patient.birthDate", "birth_date"),
        ("Patient.generalPractitioner", "general_practitioner"),
        ("Patient.managingOrganization", "managing_organization"),
        ("Observation.referenceRange", "reference_range"),
        ("Observation.dataAbsentReason", "data_absent_reason"),
    ] {
        let name = generator
            .rust_field_name(path)
            .unwrap_or_else(|e| panic!("the naming rule must answer for {path}: {e}"));
        assert_eq!(name, field, "{path}");
    }

    // Every generated element records both names, and the Rust one is usable.
    for ty in &report.types {
        for element in &ty.elements {
            assert!(
                !element.fhir_name.is_empty(),
                "{} must record its FHIR property name",
                element.path
            );
            assert!(
                is_plain_rust_identifier(&element.rust_name) || element.rust_name.starts_with("r#"),
                "{} has an unusable Rust name {:?}",
                element.path,
                element.rust_name
            );
        }
    }
}

// covers: codegen.naming.002
#[test]
fn a_fhir_name_that_is_a_rust_keyword_is_escaped_deterministically() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "naming-002");

    // A snake_case name that is a Rust 2024 strict or reserved keyword gets one
    // `_` appended; every other name is left as it is. `class` is a keyword in
    // Java, not in Rust, so it stays `class`.
    //
    // `OperationDefinition.use` does not exist in R4; the real `use` elements
    // are on `Identifier`, `HumanName`, `Address`, `ContactPoint` and `Claim`.
    for (type_name, fhir_name, rust_name) in [
        ("ElementDefinition", "type", "type_"),
        ("Identifier", "use", "use_"),
        ("Claim", "use", "use_"),
        ("Encounter", "class", "class"),
    ] {
        let element = report.ty_or_panic(type_name).element_or_panic(fhir_name);
        assert_eq!(
            element.rust_name, rust_name,
            "{type_name}.{fhir_name} must be named {rust_name:?}"
        );
        assert!(
            is_plain_rust_identifier(&element.rust_name),
            "{type_name}.{fhir_name} must get a non-keyword, non-raw Rust name; it got {:?}",
            element.rust_name
        );

        // Deterministic: the rule answers the same way every time it is asked.
        let first = generator
            .rust_field_name(&element.path)
            .unwrap_or_else(|e| panic!("{}: {e}", element.path));
        let second = generator
            .rust_field_name(&element.path)
            .unwrap_or_else(|e| panic!("{}: {e}", element.path));
        assert_eq!(first, second, "the escape must be deterministic");
        assert_eq!(first, element.rust_name);

        // The FHIR property name is still what is read and written.
        assert_eq!(element.fhir_name, fhir_name);
        assert_eq!(
            element.json_property_names,
            vec![fhir_name.to_string()],
            "{type_name}.{fhir_name} is read and written under its FHIR name"
        );
    }

    // The keyword list is exactly the 52 names the scenario gives: 38 strict
    // and 14 reserved, all distinct.
    assert_eq!(
        RUST_STRICT_KEYWORDS.len() + RUST_RESERVED_KEYWORDS.len(),
        52
    );
    let distinct: BTreeSet<&str> = RUST_KEYWORDS.into_iter().collect();
    assert_eq!(distinct.len(), 52, "the 52 keywords are distinct");
    assert!(RUST_KEYWORDS.contains(&"gen"), "`gen` is reserved in 2024");
    assert!(!RUST_KEYWORDS.contains(&"class"));

    // Asked of every keyword, the rule escapes it with one `_`. `Self` is on
    // the list but no snake_case name can spell it, so the rule is asked
    // about the lower-case names. The rule is total (`codegen.naming.002`
    // "deterministic"), so a path the package does not contain is still
    // answered.
    let mut wrong = Vec::new();
    for keyword in RUST_KEYWORDS.iter().filter(|k| **k != "Self") {
        let path = format!("Synthetic.{keyword}");
        let name = generator
            .rust_field_name(&path)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        if name != format!("{keyword}_") {
            wrong.push(format!("{keyword} -> {name}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "every Rust 2024 strict or reserved keyword gets one `_` appended: {wrong:?}"
    );

    // And nothing else is escaped: weak keywords, Java-only keywords, and
    // ordinary FHIR names stay as they are. No raw identifier is emitted.
    for name in RUST_WEAK_KEYWORDS
        .iter()
        .chain(["class", "default", "interface", "status", "value"].iter())
    {
        let path = format!("Synthetic.{name}");
        let rust = generator
            .rust_field_name(&path)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            &rust, name,
            "{name} is not a Rust 2024 keyword and stays as it is"
        );
        assert!(!rust.starts_with("r#"), "no raw identifier is emitted");
    }

    // And the wire form is unchanged on a real document.
    let encounter = concat!(
        r#"{"resourceType":"Encounter","status":"planned","#,
        r#""class":{"code":"AMB","system":"http://terminology.hl7.org/CodeSystem/v3-ActCode"}}"#
    );
    let decoded = generator
        .decode(encounter, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("an Encounter with a class must decode: {e}"));
    assert_eq!(
        decoded
            .at("Encounter.class.code")
            .first()
            .and_then(|v| v.value.as_deref()),
        Some("AMB")
    );
    let encoded = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("re-encoding must succeed: {e}"));
    assert!(
        encoded.contains(r#""class""#),
        "the FHIR property name must be what is written: {encoded}"
    );
}

// covers: codegen.naming.003
#[test]
fn two_fhir_names_mapping_to_one_rust_name_fail_generation() {
    let generator = generator_under_test();

    // The four `kind` values the package's definitions carry, so the synthetic
    // vocabulary cannot drift from the real one.
    assert_eq!(DefinitionKind::ALL.len(), 4);
    assert_eq!(FailureKind::ALL.len(), 6);
    let kinds: BTreeSet<FailureKind> = FailureKind::ALL.into_iter().collect();
    assert_eq!(kinds.len(), 6, "each refusal kind must be distinct");

    // `someId` and `someID` both snake_case to `some_id`. R4 core contains no
    // such pair -- that is the point of the rule -- so the collision is
    // written here.
    for kind in [DefinitionKind::ComplexType, DefinitionKind::Resource] {
        let out = scratch_dir("naming-003");
        let definition = DefinitionInput {
            url: "http://frevo.test/StructureDefinition/Colliding".to_string(),
            name: "Colliding".to_string(),
            kind,
            base: Some("http://hl7.org/fhir/StructureDefinition/Element".to_string()),
            is_abstract: false,
            derivation: Some("specialization".to_string()),
            elements: vec![
                element("Colliding.someId", 0, "1", &["string"]),
                element("Colliding.someID", 0, "1", &["string"]),
            ],
        };
        let request = SyntheticRequest::new(vec![definition], Vec::new(), &out);

        let error = generator
            .generate_synthetic(&request)
            .expect_err("two elements mapping to one Rust name must fail generation");
        let (exit_code, failure, message) = error.refusal("a Rust name collision");
        assert_ne!(exit_code, 0);
        assert_eq!(failure, FailureKind::NameCollision, "{kind:?}");
        for path in ["Colliding.someId", "Colliding.someID"] {
            assert!(
                message.contains(path),
                "the error must name both FHIR paths; {path} is missing from {message:?}"
            );
        }
        assert_eq!(
            files_under(&out),
            Vec::<String>::new(),
            "a collision must emit nothing, rather than a type that loses one element"
        );
    }

    // The same shape without the collision generates both elements, so the
    // failure above is about the collision and not about synthetic input.
    let out = scratch_dir("naming-003-clean");
    let request = SyntheticRequest::new(
        vec![complex_type(
            "NotColliding",
            vec![
                element("NotColliding.someId", 0, "1", &["string"]),
                element("NotColliding.otherId", 0, "1", &["string"]),
            ],
        )],
        Vec::new(),
        &out,
    );
    let report = generator
        .generate_synthetic(&request)
        .unwrap_or_else(|e| panic!("two distinct names must generate: {e}"));
    let ty = report.ty_or_panic("NotColliding");
    assert_eq!(ty.element_or_panic("someId").rust_name, "some_id");
    assert_eq!(ty.element_or_panic("otherId").rust_name, "other_id");
}

// covers: codegen.naming.004
#[test]
fn resource_and_type_names_are_used_unchanged() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "naming-004");

    for name in [
        "ExplanationOfBenefit",
        "ChargeItemDefinition",
        "MedicinalProduct",
    ] {
        let ty = report.ty_or_panic(name);
        assert_eq!(
            ty.rust_name, name,
            "the Rust type name must be the FHIR name verbatim"
        );
        let entry = report
            .resource_registry
            .iter()
            .find(|e| e.fhir_name == name)
            .unwrap_or_else(|| panic!("{name} must be in the resource registry"));
        assert_eq!(entry.rust_name, name);
    }

    // Verbatim for every concrete resource and every datatype, not only those
    // three: no pluralisation, no case change, no prefix.
    for ty in report.types_of_kind(TypeKind::Resource) {
        assert_eq!(
            ty.rust_name, ty.fhir_name,
            "{} must keep its FHIR name",
            ty.fhir_name
        );
    }
    for ty in report.types_of_kind(TypeKind::Complex) {
        assert_eq!(ty.rust_name, ty.fhir_name, "{}", ty.fhir_name);
    }
}

// covers: codegen.naming.005
#[test]
fn enumeration_variants_are_derived_from_codes_and_the_code_is_recoverable() {
    let generator = generator_under_test();

    // The three codes the scenario names: a hyphenated one, one that starts
    // with an underscore, and one that is punctuation only.
    let codes = ["entered-in-error", "_special", ">="];
    let variants = generator
        .code_variants(&codes)
        .unwrap_or_else(|e| panic!("the naming rule must answer for {codes:?}: {e}"));

    assert_eq!(variants.len(), 3, "one variant per code");
    let names: BTreeSet<&str> = variants.iter().map(|v| v.rust_name.as_str()).collect();
    assert_eq!(names.len(), 3, "each becomes a distinct variant: {names:?}");
    for variant in &variants {
        assert!(
            is_plain_rust_identifier(&variant.rust_name),
            "{:?} -> {:?} must be a usable Rust variant name",
            variant.code,
            variant.rust_name
        );
    }
    // The original code string is what is returned on serialisation.
    let recovered: Vec<&str> = variants.iter().map(|v| v.code.as_str()).collect();
    assert_eq!(
        recovered,
        codes.to_vec(),
        "each variant must carry its original code, in order"
    );

    // And on a real R4 code system, where `entered-in-error` actually occurs.
    let report = pinned_run(generator.as_ref(), "naming-005");
    let status = report
        .enumeration_for(OBSERVATION_STATUS_VALUE_SET)
        .unwrap_or_else(|| {
            panic!("observation-status is a required binding and must generate an enumeration")
        });
    assert_eq!(status.codes(), OBSERVATION_STATUS_CODES.to_vec());
    let entered = status
        .variants
        .iter()
        .find(|v| v.code == "entered-in-error")
        .expect("entered-in-error is one of the eight codes");
    assert!(is_plain_rust_identifier(&entered.rust_name));
    assert_ne!(
        entered.rust_name, "entered-in-error",
        "a hyphenated code cannot be a Rust identifier as written"
    );

    // Round-tripping through the generated enumeration returns the code.
    let decoded = generator
        .decode(
            r#"{"resourceType":"Observation","status":"entered-in-error","code":{"text":"x"}}"#,
            DecodeMode::Strict,
        )
        .unwrap_or_else(|e| panic!("entered-in-error must decode: {e}"));
    assert_eq!(
        decoded.at("Observation.status")[0].value.as_deref(),
        Some("entered-in-error")
    );
    let encoded = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("re-encoding must succeed: {e}"));
    assert!(
        encoded.contains(r#""status":"entered-in-error""#),
        "the code string must come back unchanged: {encoded}"
    );
}

// ===========================================================================
// codegen.searchparam -- the generated table
// ===========================================================================

// covers: codegen.searchparam.001
#[test]
fn every_r4_search_parameter_is_in_the_table() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-001");

    // One parameter per SearchParameter resource in search-parameters.json.
    let ids = report.distinct_parameter_ids();
    assert_eq!(
        ids.len(),
        EXPECTED_SEARCH_PARAMETERS,
        "search-parameters.json carries 1,375 SearchParameter resources"
    );
    assert_eq!(
        report.inventory.search_parameters, EXPECTED_SEARCH_PARAMETERS,
        "and the inventory must agree with the table"
    );
    // One row per base, which is 1,706 rows over those 1,375 parameters.
    assert_eq!(
        report.search_parameters.len(),
        EXPECTED_SEARCH_PARAMETER_ROWS,
        "the sum of every SearchParameter's base list length"
    );

    // Each row carries code, canonical url, type, base list and expression.
    let mut urls: BTreeSet<&str> = BTreeSet::new();
    for row in &report.search_parameters {
        assert!(!row.id.is_empty());
        assert!(!row.code.is_empty(), "{} must carry its code", row.id);
        assert!(
            row.url.starts_with("http://hl7.org/fhir/SearchParameter/"),
            "{} must carry its canonical url; it carried {:?}",
            row.id,
            row.url
        );
        assert!(
            !row.search_type.is_empty(),
            "{} must carry its type",
            row.id
        );
        assert!(
            !row.declared_bases.is_empty(),
            "{} must carry its base list",
            row.id
        );
        assert!(
            row.declared_bases.contains(&row.base),
            "{}'s row for {} must be one of its declared bases {:?}",
            row.id,
            row.base,
            row.declared_bases
        );
        urls.insert(row.url.as_str());
    }
    assert_eq!(
        urls.len(),
        EXPECTED_SEARCH_PARAMETERS,
        "the canonical urls are one per parameter"
    );

    // Two rows chosen because their shape differs: one plain and one composite.
    let subject = report
        .row("Observation-subject", "Observation")
        .expect("Observation-subject must be in the table");
    assert_eq!(subject.code, "subject");
    assert_eq!(subject.search_type, "reference");
    assert_eq!(subject.expression.as_deref(), Some("Observation.subject"));
    let composite = report
        .row("Observation-code-value-quantity", "Observation")
        .expect("Observation-code-value-quantity must be in the table");
    assert_eq!(composite.search_type, "composite");
    assert_eq!(composite.expression.as_deref(), Some("Observation"));
}

// covers: codegen.searchparam.002
#[test]
fn a_parameter_with_several_bases_appears_once_per_base() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-002");

    // `clinical-patient` declares 32 bases, so it contributes 32 rows, one per
    // base, each carrying the whole declared list.
    let rows = report.rows_for_id("clinical-patient");
    assert_eq!(
        rows.len(),
        CLINICAL_PATIENT_BASES,
        "one row per declared base, not one row for the parameter"
    );
    let bases: BTreeSet<&str> = rows.iter().map(|r| r.base.as_str()).collect();
    assert_eq!(bases.len(), CLINICAL_PATIENT_BASES, "no base twice");
    let expressions: BTreeSet<&str> = rows
        .iter()
        .map(|r| r.expression.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(
        expressions.len(),
        1,
        "every row of one parameter carries the same declared expression"
    );
    for row in &rows {
        assert_eq!(row.declared_bases.len(), CLINICAL_PATIENT_BASES);
    }

    // The six whole-system parameters appear for both Patient and
    // Practitioner. Their declared base is the abstract `Resource`, so the
    // query must resolve through the abstract element sets rather than look
    // for a literal `Patient` base.
    for resource_type in ["Patient", "Practitioner"] {
        let applicable = generator
            .parameters_for(resource_type)
            .unwrap_or_else(|e| panic!("the table must be queryable for {resource_type}: {e}"));
        for (code, expression) in WHOLE_SYSTEM_PARAMETERS {
            let row = applicable
                .iter()
                .find(|r| r.code == code)
                .unwrap_or_else(|| {
                    panic!(
                        "{code} must apply to {resource_type}; the query returned {} rows",
                        applicable.len()
                    )
                });
            assert_eq!(
                row.expression.as_deref(),
                Some(expression),
                "{code} must carry the expression its SearchParameter declares"
            );
            assert_eq!(
                row.declared_bases,
                vec!["Resource".to_string()],
                "{code} is declared on the abstract Resource"
            );
        }
        // `_text` is declared on `DomainResource`, so it applies here too.
        assert!(
            applicable.iter().any(|r| r.code == "_text"),
            "_text is declared on DomainResource and must apply to {resource_type}"
        );
    }

    // And `_text` must not apply to the three resources that are not
    // DomainResources, which is what distinguishes the two abstract bases.
    for resource_type in NON_DOMAIN_RESOURCES {
        let applicable = generator
            .parameters_for(resource_type)
            .unwrap_or_else(|e| panic!("the table must be queryable for {resource_type}: {e}"));
        assert!(
            applicable.iter().any(|r| r.code == "_id"),
            "_id is declared on Resource and applies to {resource_type}"
        );
        assert!(
            !applicable.iter().any(|r| r.code == "_text"),
            "{resource_type} is not a DomainResource, so _text must not apply to it"
        );
    }
}

// covers: codegen.searchparam.003
#[test]
fn composite_parameters_carry_their_components() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-003");

    let row = report
        .row("Observation-code-value-quantity", "Observation")
        .expect("Observation-code-value-quantity must be in the table");
    assert_eq!(row.search_type, "composite");

    assert_eq!(
        row.components,
        vec![
            SearchParamComponent {
                definition: "http://hl7.org/fhir/SearchParameter/clinical-code".to_string(),
                expression: "code".to_string(),
            },
            SearchParamComponent {
                definition: "http://hl7.org/fhir/SearchParameter/Observation-value-quantity"
                    .to_string(),
                expression: "value.as(Quantity)".to_string(),
            },
        ],
        "each component names its parameter definition and its own expression, in order"
    );

    // Each component definition resolves to a parameter in the same table, so
    // a composite cannot name something that is not there.
    for component in &row.components {
        assert!(
            report
                .search_parameters
                .iter()
                .any(|r| r.url == component.definition),
            "{} must resolve to a parameter in the table",
            component.definition
        );
    }

    // A non-composite parameter carries no components, so the field is not
    // filled in speculatively.
    let subject = report
        .row("Observation-subject", "Observation")
        .expect("Observation-subject must be in the table");
    assert_eq!(subject.components, Vec::<SearchParamComponent>::new());

    // Every composite in the table has at least two components.
    let composites: Vec<&SearchParamRow> = report
        .search_parameters
        .iter()
        .filter(|r| r.search_type == "composite")
        .collect();
    assert!(!composites.is_empty());
    for row in composites {
        assert!(
            row.components.len() >= 2,
            "{} is composite and must name its components",
            row.id
        );
    }
}

// covers: codegen.searchparam.004
#[test]
fn reference_parameters_carry_their_targets() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-004");

    let row = report
        .row("Observation-subject", "Observation")
        .expect("Observation-subject must be in the table");
    assert_eq!(row.search_type, "reference");
    assert_eq!(
        row.targets,
        OBSERVATION_SUBJECT_TARGETS
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>(),
        "the target list is the resource types the SearchParameter declares, in order"
    );

    // A non-reference parameter carries no targets.
    let status = report
        .row("Observation-status", "Observation")
        .expect("Observation-status must be in the table");
    assert_eq!(status.search_type, "token");
    assert_eq!(status.targets, Vec::<String>::new());

    // Every target names a concrete resource type the model generated.
    let concrete: BTreeSet<&str> = report
        .types_of_kind(TypeKind::Resource)
        .iter()
        .map(|t| t.fhir_name.as_str())
        .collect();
    for row in &report.search_parameters {
        for target in &row.targets {
            assert!(
                concrete.contains(target.as_str()),
                "{} targets {target}, which is not a generated resource type",
                row.id
            );
        }
    }
}

// covers: codegen.searchparam.005
#[test]
fn every_table_expression_parses() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-005");

    // Every distinct expression goes through the parser.
    let mut expressions: BTreeSet<&str> = BTreeSet::new();
    for row in &report.search_parameters {
        if let Some(expression) = &row.expression {
            assert!(
                !expression.is_empty(),
                "{} records an empty expression, which is neither absent nor an \
                 expression",
                row.id
            );
            expressions.insert(expression.as_str());
        }
    }
    assert!(
        expressions.len() > 1000,
        "the table must carry the R4 expressions, not a sample: {} distinct",
        expressions.len()
    );
    for expression in &expressions {
        generator
            .parse_search_expression(expression)
            .unwrap_or_else(|e| panic!("a table expression must parse: {expression:?}: {e}"));
    }

    // A parameter whose expression is absent is recorded as having none,
    // distinctly from having an empty one. R4 has exactly three.
    let without: BTreeSet<&str> = report
        .search_parameters
        .iter()
        .filter(|r| r.expression.is_none())
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(
        without,
        SEARCH_PARAMETERS_WITHOUT_EXPRESSION.into_iter().collect(),
        "DomainResource-text, Resource-content and Resource-query declare no expression"
    );
}

// covers: codegen.searchparam.006
#[test]
fn parameter_modifiers_and_flags_are_carried() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "searchparam-006");

    // One row per parameter, so the counts below are over parameters.
    let mut first_row: BTreeMap<&str, &SearchParamRow> = BTreeMap::new();
    for row in &report.search_parameters {
        first_row.entry(row.id.as_str()).or_insert(row);
    }
    assert_eq!(first_row.len(), EXPECTED_SEARCH_PARAMETERS);

    let stated_or = first_row
        .values()
        .filter(|r| r.multiple_or.is_some())
        .count();
    let stated_and = first_row
        .values()
        .filter(|r| r.multiple_and.is_some())
        .count();
    let stated_comparator = first_row
        .values()
        .filter(|r| r.comparator.is_some())
        .count();
    let stated_modifier = first_row.values().filter(|r| r.modifier.is_some()).count();
    let stated_chain = first_row.values().filter(|r| r.chain.is_some()).count();

    assert_eq!(
        stated_or, SEARCH_PARAMETERS_STATING_MULTIPLE_OR,
        "46 R4 parameters state multipleOr; absent must not be read as false"
    );
    assert_eq!(stated_and, SEARCH_PARAMETERS_STATING_MULTIPLE_AND);
    assert_eq!(stated_comparator, SEARCH_PARAMETERS_STATING_COMPARATOR);
    assert_eq!(stated_modifier, SEARCH_PARAMETERS_STATING_MODIFIER);
    assert_eq!(stated_chain, SEARCH_PARAMETERS_STATING_CHAIN);

    // Every stated multipleOr in R4 is `false`, which is exactly the value
    // that must be distinguishable from absent.
    for row in first_row.values() {
        if let Some(value) = row.multiple_or {
            assert!(
                !value,
                "{} states multipleOr true, which R4 does not",
                row.id
            );
        }
    }

    // A parameter that states the flag, and one that does not.
    let composite = report
        .row("Observation-code-value-quantity", "Observation")
        .expect("in the table");
    assert_eq!(
        composite.multiple_or,
        Some(false),
        "Observation-code-value-quantity states multipleOr false"
    );
    let subject = report
        .row("Observation-subject", "Observation")
        .expect("in the table");
    assert_eq!(
        subject.multiple_or, None,
        "Observation-subject states nothing, which is not the same as false"
    );
    assert_eq!(subject.multiple_and, None);
    assert_eq!(subject.comparator, None);
    assert_eq!(subject.modifier, None);
    assert_eq!(subject.chain, None);

    // And the comparator list is carried verbatim where it is stated.
    let last_updated = report
        .row("Resource-lastUpdated", "Resource")
        .expect("Resource-lastUpdated must be in the table");
    assert_eq!(
        last_updated.comparator,
        Some(
            COMPARATOR_CODES
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
        ),
        "the nine comparator codes, in declared order"
    );
}

// ===========================================================================
// codegen.output -- determinism, markers, lints
// ===========================================================================

// covers: codegen.output.001
#[test]
fn regeneration_is_byte_identical() {
    let generator = generator_under_test();

    let first = pinned_run(generator.as_ref(), "output-001-first");
    let second = pinned_run(generator.as_ref(), "output-001-second");

    assert!(!first.files.is_empty());
    assert_eq!(
        first.file_map().keys().collect::<Vec<_>>(),
        second.file_map().keys().collect::<Vec<_>>(),
        "both runs must write the same set of files"
    );
    for (path, bytes) in first.file_map() {
        let other = second
            .file(path)
            .unwrap_or_else(|| panic!("{path} must be written by both runs"));
        assert_eq!(
            bytes,
            other.bytes.as_slice(),
            "{path} differs between two runs from the same input package"
        );
    }
    // The digests the manifest reports must agree too, so a cross-run
    // comparison can be made on digests alone.
    for file in &first.files {
        let other = second.file(&file.path).expect("same set");
        assert!(
            is_sha256_hex(&file.sha256),
            "{}'s digest must be 64 lower-case hex characters, not {:?}",
            file.path,
            file.sha256
        );
        assert_eq!(file.sha256, other.sha256, "{}", file.path);
    }
}

// covers: codegen.output.002
#[test]
fn regeneration_is_byte_identical_across_platforms() {
    let generator = generator_under_test();

    // Running the two platforms is CI's job; what is checkable in one process
    // is that nothing platform-dependent reaches the output. Three things
    // could: the line ending, an absolute path, and the output directory.
    let one = scratch_dir("output-002-one");
    let two = scratch_dir("output-002-two-with-a-different-name");
    let first = generator
        .generate(&GenerateRequest::pinned(&one))
        .unwrap_or_else(|e| panic!("generation must succeed: {e}"));
    let second = generator
        .generate(&GenerateRequest::pinned(&two))
        .unwrap_or_else(|e| panic!("generation must succeed: {e}"));

    assert_eq!(
        first.file_map(),
        second.file_map(),
        "the output must not depend on where it is written"
    );

    let root = repo_root().to_string_lossy().to_string();
    for file in &first.files {
        assert!(
            !file.bytes.contains(&b'\r'),
            "{} must use LF line endings, so the two platforms agree byte for byte",
            file.path
        );
        let text = file.text();
        for embedded in [
            one.to_string_lossy().to_string(),
            two.to_string_lossy().to_string(),
            root.clone(),
        ] {
            assert!(
                !text.contains(&embedded),
                "{} embeds the absolute path {embedded}, which differs per machine",
                file.path
            );
        }
        assert!(
            !file.path.contains('\\'),
            "the reported path {} must use / separators",
            file.path
        );
    }
}

// covers: codegen.output.003
#[test]
fn every_output_file_carries_the_generated_marker_and_its_inputs() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "output-003");

    assert!(!report.files.is_empty());
    for file in &report.files {
        let header: String = file.text().lines().take(20).collect::<Vec<_>>().join("\n");
        for required in [
            "@generated",
            GENERATOR_NAME,
            PACKAGE_NAME,
            PACKAGE_VERSION,
            report.pin.sha256.as_str(),
        ] {
            assert!(
                header.contains(required),
                "{}'s first lines must contain {required:?}; they were:\n{header}",
                file.path
            );
        }
        // The generator's own version, so a regeneration by a different
        // generator is visible.
        assert!(
            header.contains(env!("CARGO_PKG_VERSION")),
            "{}'s header must name the generator version {}; it said:\n{header}",
            file.path,
            env!("CARGO_PKG_VERSION")
        );
        // And a statement that edits are overwritten.
        assert!(
            header.to_lowercase().contains("overwritten"),
            "{}'s header must state that edits are overwritten; it said:\n{header}",
            file.path
        );
    }
}

// covers: codegen.output.004
#[test]
fn generated_code_passes_the_lint_and_format_gates() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "output-004");

    assert!(!report.files.is_empty());
    for file in &report.files {
        let text = file.text();

        // No `allow` added to accommodate the generator's output style.
        for attribute in ["#[allow(", "#![allow(", "#[expect(", "#![expect("] {
            assert!(
                !text.contains(attribute),
                "{} carries {attribute}, which accommodates the lint instead of satisfying it",
                file.path
            );
        }

        // Already formatted, so `cargo fmt --all --check` has nothing to say.
        let formatted = rustfmt(text, &file.path);
        assert_eq!(
            formatted, text,
            "{} is not rustfmt-clean, so `cargo fmt --all --check` would fail",
            file.path
        );
    }
}

// covers: codegen.output.005
#[test]
fn a_drift_check_fails_when_checked_in_output_does_not_match_a_fresh_run() {
    let generator = generator_under_test();

    // In sync: the checked-in tree matches a fresh run.
    let out = scratch_dir("output-005-fresh");
    let request = GenerateRequest::pinned(&out);
    let checked_in = repo_path(GENERATED_OUT_DIR);
    let in_sync = generator
        .drift_check(&request, &checked_in)
        .unwrap_or_else(|e| panic!("the drift check must run against {GENERATED_OUT_DIR}: {e}"));
    assert!(
        in_sync.in_sync,
        "the checked-in output must match a fresh run; these differ: {:?}",
        in_sync.differing_files
    );
    assert_eq!(in_sync.differing_files, Vec::<String>::new());

    // Out of sync: a hand edit to one checked-in file.
    let report = generator
        .generate(&request)
        .unwrap_or_else(|e| panic!("generation must succeed: {e}"));
    let mutated_dir = scratch_dir("output-005-mutated");
    for file in &report.files {
        let target = mutated_dir.join(&file.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("scratch directories are creatable");
        }
        std::fs::write(&target, &file.bytes).expect("scratch files are writable");
    }
    let victim = &report.files[0];
    let victim_path = mutated_dir.join(&victim.path);
    let mut edited = victim.text().to_string();
    edited.push_str("\n// a hand edit\n");
    std::fs::write(&victim_path, edited).expect("scratch files are writable");

    let drifted = generator
        .drift_check(&request, &mutated_dir)
        .unwrap_or_else(|e| panic!("the drift check must report drift, not fail to run: {e}"));
    assert!(!drifted.in_sync, "a hand-edited file must be detected");
    assert_eq!(
        drifted.differing_files,
        vec![victim.path.clone()],
        "the check must name the differing file, and only it"
    );

    // And the message says to regenerate, not to edit.
    let remediation = drifted.remediation.to_lowercase();
    assert!(
        remediation.contains("regenerate"),
        "the remediation must say to regenerate; it said {:?}",
        drifted.remediation
    );
    assert!(
        drifted.remediation.contains("frevo-codegen generate"),
        "and name the command that does it; it said {:?}",
        drifted.remediation
    );
    assert!(
        !remediation.contains("edit the"),
        "it must not tell anyone to edit the output: {:?}",
        drifted.remediation
    );
}

// covers: codegen.output.006
#[test]
fn generated_code_contains_no_unsafe() {
    // `frevo-model` declares the crate-level forbid, so an `unsafe` block
    // anywhere in the generated output would not compile.
    let lib = repo_path(MODEL_CRATE_ROOT);
    let source = std::fs::read_to_string(&lib)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", lib.display()));
    assert!(
        source.contains("#![forbid(unsafe_code)]"),
        "{MODEL_CRATE_ROOT} must declare #![forbid(unsafe_code)]"
    );

    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "output-006");
    assert!(!report.files.is_empty());
    for file in &report.files {
        let text = file.text();
        // Every occurrence of `unsafe` must be part of `unsafe_code`, which is
        // the forbid attribute and not a block.
        let mut rest = text;
        while let Some(at) = rest.find("unsafe") {
            let tail = &rest[at..];
            assert!(
                tail.starts_with("unsafe_code"),
                "{} contains `unsafe` that is not the forbid attribute: {:?}",
                file.path,
                &tail[..tail.len().min(40)]
            );
            rest = &tail["unsafe".len()..];
        }
    }
}

// ===========================================================================
// codegen.roundtrip -- the generated output is enough for the codec
// ===========================================================================

// covers: codegen.roundtrip.001
#[test]
fn every_r4_specification_example_round_trips_byte_identically() {
    let generator = generator_under_test();
    let examples = example_set();

    // The interim selector, stated so it cannot be mistaken for the full R4
    // example package. The 21 `*.json.out` files in the same directory are
    // sql-on-FHIR expected outputs carrying no `resourceType`, so they are not
    // examples and are excluded.
    assert_eq!(examples.json.len(), EXPECTED_JSON_EXAMPLES);
    assert_eq!(examples.xml.len(), EXPECTED_XML_EXAMPLES);
    assert_eq!(examples.excluded_out_files, EXPECTED_NON_EXAMPLE_FIXTURES);

    let report = generator
        .round_trip(&examples.all())
        .unwrap_or_else(|e| panic!("the round-trip oracle must run: {e}"));

    assert!(
        report.selector.contains("interim"),
        "the report must state that this is the interim example set; it said {:?}",
        report.selector
    );
    assert!(
        report.selector.contains("examples"),
        "and name the directory it used; it said {:?}",
        report.selector
    );

    // The report names each example by file name, so a failure identifies a
    // case rather than a count.
    assert_eq!(
        report.cases.len(),
        examples.all().len(),
        "one case per example file"
    );
    for name in examples.file_names() {
        let case = report
            .case(&name)
            .unwrap_or_else(|| panic!("{name} must appear in the report by file name"));
        assert_eq!(case.file, name);
    }

    // Every JSON example round-trips byte-identically.
    let failures: Vec<String> = report
        .with_status(RoundTripStatus::Failed)
        .iter()
        .map(|c| format!("{} ({})", c.file, c.detail))
        .collect();
    assert_eq!(
        failures,
        Vec::<String>::new(),
        "every example must round-trip byte-identically"
    );
    assert_eq!(
        report.with_status(RoundTripStatus::Passed).len(),
        EXPECTED_JSON_EXAMPLES,
        "all 72 JSON examples must pass, and only they can"
    );
}

// covers: codegen.roundtrip.002
#[test]
fn the_round_trip_covers_every_resource_type_that_has_an_example() {
    let generator = generator_under_test();
    let examples = example_set();
    let report = generator
        .round_trip(&examples.all())
        .unwrap_or_else(|e| panic!("the round-trip oracle must run: {e}"));

    // One coverage row per concrete resource type, all 146.
    assert_eq!(
        report.coverage.len(),
        EXPECTED_CONCRETE_RESOURCES,
        "the report must list every concrete resource type, covered or not"
    );
    let listed: BTreeSet<&str> = report
        .coverage
        .iter()
        .map(|c| c.resource_type.as_str())
        .collect();
    assert_eq!(listed.len(), EXPECTED_CONCRETE_RESOURCES, "no type twice");

    let generated = pinned_run(generator.as_ref(), "roundtrip-002");
    let concrete: BTreeSet<&str> = generated
        .types_of_kind(TypeKind::Resource)
        .iter()
        .map(|t| t.fhir_name.as_str())
        .collect();
    assert_eq!(
        listed, concrete,
        "the coverage list must be exactly the generated concrete resource types"
    );
    for logical in generated.types_of_kind(TypeKind::Logical) {
        assert!(
            !listed.contains(logical.fhir_name.as_str()),
            "{} is a logical definition, not a concrete resource",
            logical.fhir_name
        );
    }

    // A type with zero examples is reported as uncovered rather than omitted.
    let covered = report.coverage.iter().filter(|c| c.examples > 0).count();
    assert_eq!(
        covered, EXPECTED_COVERED_RESOURCE_TYPES,
        "the 72 JSON examples exercise 69 distinct resource types"
    );
    assert_eq!(
        report.coverage.len() - covered,
        EXPECTED_CONCRETE_RESOURCES - EXPECTED_COVERED_RESOURCE_TYPES,
        "and the remaining 77 must be listed with no examples"
    );
    for name in ["Claim", "Task", "Measure"] {
        let entry = report
            .coverage_of(name)
            .unwrap_or_else(|| panic!("{name} has no example and must still be listed"));
        assert_eq!(entry.examples, 0, "{name}");
    }
    // And the ones that do have examples report how many.
    for (name, count) in [("Patient", 1), ("Observation", 2), ("Questionnaire", 2)] {
        let entry = report
            .coverage_of(name)
            .unwrap_or_else(|| panic!("{name} must be listed"));
        assert_eq!(
            entry.examples, count,
            "{name} is exercised by {count} JSON example(s)"
        );
    }
}

// covers: codegen.roundtrip.003
#[test]
fn round_tripping_an_xml_example_through_the_json_codec_is_not_claimed() {
    let generator = generator_under_test();
    let examples = example_set();
    assert_eq!(examples.xml.len(), EXPECTED_XML_EXAMPLES);

    let report = generator
        .round_trip(&examples.all())
        .unwrap_or_else(|e| panic!("the round-trip oracle must run: {e}"));

    // The three statuses are distinct, and `not-applicable-xml` is not
    // `passed`.
    assert_eq!(RoundTripStatus::ALL.len(), 3);
    let names: BTreeSet<&str> = RoundTripStatus::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        names,
        ["failed", "not-applicable-xml", "passed"]
            .into_iter()
            .collect()
    );
    assert_ne!(RoundTripStatus::NotApplicableXml, RoundTripStatus::Passed);

    for path in &examples.xml {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let case = report
            .case(&name)
            .unwrap_or_else(|| panic!("{name} must be reported, not omitted"));
        assert_eq!(
            case.status,
            RoundTripStatus::NotApplicableXml,
            "{name} is XML and the codec under test is the JSON one; it was reported {}",
            case.status.as_str()
        );
    }
    assert_eq!(
        report.with_status(RoundTripStatus::NotApplicableXml).len(),
        EXPECTED_XML_EXAMPLES,
        "exactly the 10 XML files"
    );
    for case in report.with_status(RoundTripStatus::Passed) {
        assert!(
            !case.file.ends_with(".xml"),
            "{} is XML and cannot be claimed as passed",
            case.file
        );
    }
}

// covers: codegen.roundtrip.004
#[test]
fn the_element_order_table_is_generated() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "roundtrip-004");

    // Every generated type carries the snapshot's element order.
    for ty in &report.types {
        assert!(
            !ty.element_order.is_empty() || ty.elements.is_empty(),
            "{} must carry an element order",
            ty.fhir_name
        );
        assert_eq!(
            ty.element_order,
            ty.element_names()
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>(),
            "{}'s element order must be its element set in snapshot order",
            ty.fhir_name
        );
    }
    assert_eq!(
        report.ty_or_panic("Patient").element_order,
        PATIENT_ELEMENT_ORDER
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>(),
        "Patient's 24 top-level elements, with [x] resolved to the base name"
    );
    assert_eq!(
        report.ty_or_panic("Patient.contact").element_order,
        PATIENT_CONTACT_ELEMENT_ORDER
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
    );

    // That order is what serialisation emits, as distinct from the lexical key
    // order the canonical form uses.
    let decoded = generator
        .decode(PATIENT_IN_SCRAMBLED_ORDER, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("the scrambled Patient must decode: {e}"));

    let stored = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("encoding must succeed: {e}"));
    let id = index_of(&stored, r#""id""#);
    let active = index_of(&stored, r#""active""#);
    let birth_date = index_of(&stored, r#""birthDate""#);
    assert!(
        id < active && active < birth_date,
        "the stored form must follow the snapshot order id, active, birthDate: {stored}"
    );

    let canonical = generator
        .encode_canonical(&decoded)
        .unwrap_or_else(|e| panic!("canonical encoding must succeed: {e}"));
    let c_active = index_of(&canonical, r#""active""#);
    let c_birth_date = index_of(&canonical, r#""birthDate""#);
    let c_id = index_of(&canonical, r#""id""#);
    let c_resource_type = index_of(&canonical, r#""resourceType""#);
    assert!(
        c_active < c_birth_date && c_birth_date < c_id && c_id < c_resource_type,
        "the canonical form sorts property names lexicographically: {canonical}"
    );
    assert!(
        !canonical.contains('\n') && !canonical.contains("  "),
        "JsonCreatorCanonical refuses to pretty-print: {canonical}"
    );
}

// ===========================================================================
// The R4 model class table (`codegen.ancestry.001` to `.005`)
// ===========================================================================
//
// The class table is a second generator output with its own seam, because
// `frevo_codegen::bind_oracle!` (src/bind.rs) binds only the `Codegen` trait
// and an oracle file must not reach into the implementation. Task 14b ports
// the generator and binds [`class_table_under_test`]; until then it answers
// [`CodegenError::NotImplemented`], which every test below turns into a
// failure naming the unbound seam, never a pass.
//
// The expected rows are derived independently of the class table: from the
// pinned `generate` run (bound today), which lists every type, every nested
// component and every element of the package, and from the facts the
// approved spec states. Exact class names, ancestry sets and the 323 count
// are the spec's (`specs/codegen/spec.md`, `codegen.ancestry.002` to `.005`).

/// The reviewed exception file the generator reads (`codegen.ancestry.004`).
pub const CLASS_TABLE_EXCEPTIONS: &str = "crates/frevo-codegen/data/r4-class-table-exceptions.toml";
/// The Java package every model class lives in.
pub const MODEL_PACKAGE: &str = "org.hl7.fhir.r4.model";
/// Where the exception file's citations point (`codegen.ancestry.004`).
pub const MODEL_SOURCE_DIR: &str =
    "reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model";

/// The four interface kinds a row records (`codegen.ancestry.001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterfaceKind {
    IBase,
    IPrimitiveType,
    ICompositeType,
    IBaseReference,
}

/// What a row stands for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassRowOf {
    /// A FHIR type of kind `primitive-type`, `complex-type` or `resource`,
    /// abstract ones included, by its FHIR name.
    Type(String),
    /// A nested component, by its element path (`Patient.contact`).
    Component(String),
    /// HAPI's `Enumeration` class.
    Enumeration,
}

/// One row of the generated table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRow {
    pub of: ClassRowOf,
    /// The Java simple name, `ContactComponent`.
    pub simple_name: String,
    /// The Java fully qualified name, `org.hl7.fhir.r4.model.Patient$ContactComponent`.
    pub qualified_name: String,
    /// The FHIR type names the class is assignable to.
    pub assignable_to: BTreeSet<String>,
    pub interfaces: BTreeSet<InterfaceKind>,
}

/// The class an element's value is held in, by the qualified name of its row.
/// `through_star` differs from `by_name` only for a `runtime-class` exception
/// (`Narrative.div`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementClass {
    pub by_name: String,
    pub through_star: String,
}

/// One class-table run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassTable {
    /// The `@generated` table file, as written.
    pub file: GeneratedFile,
    /// Every path the run opened.
    pub inputs_read: Vec<PathBuf>,
    pub rows: Vec<ClassRow>,
    /// Keyed by element path; a choice element by `<owner>.<json property>`,
    /// one key per chosen type (`Observation.valueQuantity`).
    pub elements: BTreeMap<String, ElementClass>,
}

impl ClassTable {
    pub fn type_row(&self, fhir_name: &str) -> &ClassRow {
        self.row_of(&ClassRowOf::Type(fhir_name.to_string()))
    }

    pub fn component_row(&self, path: &str) -> &ClassRow {
        self.row_of(&ClassRowOf::Component(path.to_string()))
    }

    pub fn enumeration_row(&self) -> &ClassRow {
        self.row_of(&ClassRowOf::Enumeration)
    }

    fn row_of(&self, of: &ClassRowOf) -> &ClassRow {
        let found: Vec<&ClassRow> = self.rows.iter().filter(|r| &r.of == of).collect();
        assert_eq!(
            found.len(),
            1,
            "the table must have exactly one row for {of:?}"
        );
        found[0]
    }

    pub fn row_by_qualified(&self, qualified: &str) -> &ClassRow {
        self.rows
            .iter()
            .find(|r| r.qualified_name == qualified)
            .unwrap_or_else(|| panic!("the table must have a row named {qualified}"))
    }

    /// The row of the class an element's value is held in, by name.
    pub fn element_row(&self, path: &str) -> &ClassRow {
        let class = self
            .elements
            .get(path)
            .unwrap_or_else(|| panic!("the table must give {path} a value class"));
        self.row_by_qualified(&class.by_name)
    }

    /// The same, navigating with `*`.
    pub fn element_row_through_star(&self, path: &str) -> &ClassRow {
        let class = self
            .elements
            .get(path)
            .unwrap_or_else(|| panic!("the table must give {path} a value class"));
        self.row_by_qualified(&class.through_star)
    }
}

/// One class-table run's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassTableRequest {
    pub exceptions: PathBuf,
    pub out_dir: PathBuf,
    /// Where `reference/` would be; the generator must never read it.
    pub reference_dir: Option<PathBuf>,
}

impl ClassTableRequest {
    pub fn pinned(out_dir: &Path) -> Self {
        ClassTableRequest {
            exceptions: repo_path(CLASS_TABLE_EXCEPTIONS),
            out_dir: out_dir.to_path_buf(),
            reference_dir: None,
        }
    }
}

pub trait ClassTableGenerator {
    /// One run over the pinned package and an exception file. A refused
    /// exception file is [`CodegenError::Failed`].
    fn class_table(&self, request: &ClassTableRequest) -> Result<ClassTable, CodegenError>;
}

/// The class-table generator this oracle grades, bound to
/// `frevo_codegen::classtable` by task 14b.
fn class_table_under_test() -> Box<dyn ClassTableGenerator> {
    Box::new(BoundClassTable)
}

frevo_codegen::bind_class_table!(BoundClassTable);

/// One pinned class-table run, which must succeed.
fn pinned_class_table(label: &str) -> ClassTable {
    let out = scratch_dir(label);
    class_table_under_test()
        .class_table(&ClassTableRequest::pinned(&out))
        .unwrap_or_else(|e| match e {
            CodegenError::NotImplemented => panic!(
                "the class-table run must succeed for {label}, but no class-table generator \
                 is bound yet (codegen_r4.rs::class_table_under_test; task 14b)"
            ),
            e => panic!("the class-table run must succeed for {label}: {e}"),
        })
}

fn model_class(simple: &str) -> String {
    format!("{MODEL_PACKAGE}.{simple}")
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

/// The element keys a generated element contributes to the table: its path,
/// or for a choice one key per JSON property name, under its owner's path.
fn element_keys(element: &GeneratedElement) -> Vec<String> {
    if element.is_choice {
        let owner = element
            .path
            .rsplit_once('.')
            .map(|(owner, _)| owner)
            .unwrap_or(&element.path);
        element
            .json_property_names
            .iter()
            .map(|p| format!("{owner}.{p}"))
            .collect()
    } else {
        vec![element.path.clone()]
    }
}

// ---------------------------------------------------------------------------
// The exception file, read as data
// ---------------------------------------------------------------------------

/// One entry of the exception file: its `key = "value"` pairs.
///
/// The file is TOML; the oracle reads only what `codegen.ancestry.004` fixes:
/// an array of tables (`[[...]]`), each with string `kind`, `target` and
/// `source` keys and at least one further key carrying the effect. The
/// effect's key names are the generator's to choose. The crate has no TOML
/// dependency, so this is the smallest reader that can do it, and it refuses
/// any line it does not understand rather than skipping it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionEntry {
    pub pairs: BTreeMap<String, String>,
}

impl ExceptionEntry {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.get(key).map(String::as_str)
    }
}

fn read_exception_entries(text: &str) -> Vec<ExceptionEntry> {
    let mut entries = Vec::new();
    let mut current: Option<BTreeMap<String, String>> = None;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("[[") && line.ends_with("]]") {
            if let Some(pairs) = current.take() {
                entries.push(ExceptionEntry { pairs });
            }
            current = Some(BTreeMap::new());
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!(
                "{CLASS_TABLE_EXCEPTIONS}:{}: not `key = value`: {raw}",
                n + 1
            )
        });
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or_else(|| {
                panic!(
                    "{CLASS_TABLE_EXCEPTIONS}:{}: the oracle reads string values only: {raw}",
                    n + 1
                )
            });
        let pairs = current.as_mut().unwrap_or_else(|| {
            panic!(
                "{CLASS_TABLE_EXCEPTIONS}:{}: a key outside any [[...]] entry: {raw}",
                n + 1
            )
        });
        pairs.insert(key.trim().to_string(), value.to_string());
    }
    if let Some(pairs) = current.take() {
        entries.push(ExceptionEntry { pairs });
    }
    entries
}

fn exception_file_text() -> String {
    let path = repo_path(CLASS_TABLE_EXCEPTIONS);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("{CLASS_TABLE_EXCEPTIONS} must exist and be readable (codegen.ancestry.004): {e}")
    })
}

/// The 13 `class-name` entries: component path, HAPI's class name.
pub const CLASS_NAME_EXCEPTIONS: [(&str, &str); 13] = [
    (
        "CodeSystem.concept.designation",
        "ConceptDefinitionDesignationComponent",
    ),
    ("Consent.provision.actor", "provisionActorComponent"),
    ("Consent.provision.data", "provisionDataComponent"),
    (
        "InsurancePlan.coverage.benefit.limit",
        "CoverageBenefitLimitComponent",
    ),
    (
        "InsurancePlan.plan.specificCost.benefit.cost",
        "PlanBenefitCostComponent",
    ),
    (
        "MeasureReport.group.stratifier.stratum.component",
        "StratifierGroupComponentComponent",
    ),
    (
        "TestReport.setup.action.operation",
        "SetupActionOperationComponent",
    ),
    (
        "TestReport.setup.action.assert",
        "SetupActionAssertComponent",
    ),
    (
        "TestScript.setup.action.operation",
        "SetupActionOperationComponent",
    ),
    (
        "TestScript.setup.action.operation.requestHeader",
        "SetupActionOperationRequestHeaderComponent",
    ),
    (
        "TestScript.setup.action.assert",
        "SetupActionAssertComponent",
    ),
    (
        "ValueSet.compose.include.concept.designation",
        "ConceptReferenceDesignationComponent",
    ),
    (
        "ValueSet.compose.include.filter",
        "ConceptSetFilterComponent",
    ),
];

/// The `constraint` profiles on `Quantity` the pinned run reports as complex
/// types. No row: the model has no field of either class.
pub const QUANTITY_PROFILES: [&str; 2] = ["SimpleQuantity", "MoneyQuantity"];

/// The nine `superclass` entries: `id`, and the eight `BackboneType`
/// datatypes.
pub const SUPERCLASS_EXCEPTIONS: [&str; 9] = [
    "id",
    "Dosage",
    "ElementDefinition",
    "MarketingStatus",
    "Population",
    "ProdCharacteristic",
    "ProductShelfLife",
    "SubstanceAmount",
    "Timing",
];

/// The eight datatypes HAPI bases on `BackboneType`.
pub const BACKBONE_TYPE_DATATYPES: [&str; 8] = [
    "Dosage",
    "ElementDefinition",
    "MarketingStatus",
    "Population",
    "ProdCharacteristic",
    "ProductShelfLife",
    "SubstanceAmount",
    "Timing",
];

// ---------------------------------------------------------------------------
// codegen.ancestry.001
// ---------------------------------------------------------------------------

// covers: codegen.ancestry.001
#[test]
fn the_class_table_is_generated_from_the_structure_definitions_and_the_exception_file_only() {
    // The expected rows first, from the pinned generate run (bound today), so
    // this part of the fixture is checked even while the table is unbound:
    // one row per type of kind primitive-type, complex-type and resource
    // defined by specialization (abstract ones included), one per nested
    // component, and one for `Enumeration`.
    let report = pinned_run(generator_under_test().as_ref(), "ancestry-001-report");
    let expected_types: BTreeSet<String> = report
        .types
        .iter()
        .filter(|t| {
            matches!(
                t.kind,
                TypeKind::Primitive
                    | TypeKind::Complex
                    | TypeKind::Resource
                    | TypeKind::AbstractElementSet
            )
        })
        // The pinned run also reports the two `constraint` profiles on
        // `Quantity` as complex types; they are not defined by specialization,
        // and the model holds their elements as `Quantity`
        // (`codegen.ancestry.002`), so they are not rows.
        .filter(|t| !QUANTITY_PROFILES.contains(&t.fhir_name.as_str()))
        .map(|t| t.fhir_name.clone())
        .collect();
    let expected_components: BTreeSet<String> = report
        .types_of_kind(TypeKind::Backbone)
        .iter()
        .map(|t| t.fhir_name.clone())
        .collect();
    for name in ["Element", "BackboneElement", "Resource", "DomainResource"] {
        assert!(expected_types.contains(name), "{name} is a type row");
    }
    // 20 primitives, 40 complex specializations plus `Element`, 146 concrete
    // and 2 abstract resources (the inventory constants above); the logical
    // `MetadataResource` and the `SimpleQuantity`/`MoneyQuantity` profiles are
    // not rows.
    assert_eq!(
        expected_types.len(),
        R4_PRIMITIVES.len()
            + EXPECTED_COMPLEX_SPECIALIZATIONS
            + 1
            + EXPECTED_CONCRETE_RESOURCES
            + EXPECTED_ABSTRACT_RESOURCES,
        "the pinned run's type list: {:?}",
        report
            .types
            .iter()
            .filter(|t| expected_types.contains(&t.fhir_name))
            .map(|t| format!("{}:{:?}:{}", t.fhir_name, t.kind, t.instantiable))
            .collect::<Vec<_>>()
    );
    for path in ["Patient.contact", "Timing.repeat", "Questionnaire.item"] {
        assert!(
            expected_components.contains(path),
            "{path} is a component row"
        );
    }

    // The exception file is an input of the run, so it must exist.
    exception_file_text();

    let first = pinned_class_table("ancestry-001-a");
    let second = pinned_class_table("ancestry-001-b");
    assert!(
        first.file.text().contains("@generated"),
        "the class table {} must carry an @generated marker",
        first.file.path
    );
    assert_eq!(
        first.file.bytes, second.file.bytes,
        "the class table must regenerate byte-identically from the same inputs"
    );

    // It reads no `.java` file: a run with `reference/` present gets the same
    // bytes as one without, and no input it opened is Java source.
    let out = scratch_dir("ancestry-001-ref");
    let with_reference = class_table_under_test()
        .class_table(&ClassTableRequest {
            reference_dir: Some(repo_path("reference")),
            ..ClassTableRequest::pinned(&out)
        })
        .unwrap_or_else(|e| panic!("the class-table run with reference/ must succeed: {e}"));
    assert_eq!(
        with_reference.file.bytes, first.file.bytes,
        "the class table must not depend on reference/"
    );
    for run in [&first, &with_reference] {
        let java: Vec<&PathBuf> = run
            .inputs_read
            .iter()
            .filter(|p| p.extension().is_some_and(|x| x == "java"))
            .collect();
        assert!(
            java.is_empty(),
            "the generator must read no .java file: {java:?}"
        );
        assert!(
            run.inputs_read
                .iter()
                .any(|p| p.ends_with("r4-class-table-exceptions.toml")),
            "the exception file must be one of the run's inputs: {:?}",
            run.inputs_read
        );
    }

    // The table's rows against the expected sets.
    let table = first;
    let got_types: BTreeSet<String> = table
        .rows
        .iter()
        .filter_map(|r| match &r.of {
            ClassRowOf::Type(n) => Some(n.clone()),
            _ => None,
        })
        .collect();
    let got_components: BTreeSet<String> = table
        .rows
        .iter()
        .filter_map(|r| match &r.of {
            ClassRowOf::Component(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(got_types, expected_types, "one row per FHIR type");
    assert_eq!(
        got_components, expected_components,
        "one row per nested component"
    );
    assert_eq!(
        table
            .rows
            .iter()
            .filter(|r| r.of == ClassRowOf::Enumeration)
            .count(),
        1,
        "one row for Enumeration"
    );
    assert_eq!(
        table.rows.len(),
        expected_types.len() + expected_components.len() + 1,
        "and no other row"
    );

    // Each row: simple and qualified name, with `$` before a nested class's
    // name; IBase for every row.
    for row in &table.rows {
        assert!(
            row.interfaces.contains(&InterfaceKind::IBase),
            "{:?} must be IBase",
            row.of
        );
        let want = match &row.of {
            ClassRowOf::Component(path) => {
                let outer = path.split('.').next().expect("a component path has a root");
                format!(
                    "{}${}",
                    table.type_row(outer).qualified_name,
                    row.simple_name
                )
            }
            _ => model_class(&row.simple_name),
        };
        assert_eq!(row.qualified_name, want, "{:?}", row.of);
    }
    assert_eq!(table.enumeration_row().simple_name, "Enumeration");

    // Each element of a generated type carries a value class. A primitive's
    // `value` is held in a Java value, not a model class. `xhtml.id` is a gap
    // in the spec (no rule fixes it) and is deliberately not asserted.
    let mut missing = Vec::new();
    for ty in report.types.iter().filter(|t| {
        expected_types.contains(&t.fhir_name) || expected_components.contains(&t.fhir_name)
    }) {
        for element in &ty.elements {
            if ty.kind == TypeKind::Primitive && element.fhir_name == "value" {
                continue;
            }
            if element.path == "xhtml.id" {
                continue;
            }
            for key in element_keys(element) {
                match table.elements.get(&key) {
                    Some(class) => {
                        // Both classes are rows of the table.
                        table.row_by_qualified(&class.by_name);
                        table.row_by_qualified(&class.through_star);
                    }
                    None => missing.push(key),
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} elements have no value class; the first few: {:?}",
        missing.len(),
        &missing[..missing.len().min(10)]
    );
}

// ---------------------------------------------------------------------------
// codegen.ancestry.002
// ---------------------------------------------------------------------------

// covers: codegen.ancestry.002
#[test]
fn class_names_and_the_fhir_type_hierarchy_follow_fixed_rules() {
    let table = pinned_class_table("ancestry-002");

    // Class names.
    for (fhir, class) in [
        ("string", "StringType"),
        ("dateTime", "DateTimeType"),
        ("positiveInt", "PositiveIntType"),
        ("Patient", "Patient"),
        ("HumanName", "HumanName"),
        ("List", "ListResource"),
    ] {
        assert_eq!(table.type_row(fhir).simple_name, class, "{fhir}");
    }
    for (path, class) in [
        ("Patient.contact", "ContactComponent"),
        ("Consent.provision", "ProvisionComponent"),
        ("Patient.communication", "PatientCommunicationComponent"),
        ("Observation.component", "ObservationComponentComponent"),
    ] {
        assert_eq!(table.component_row(path).simple_name, class, "{path}");
    }
    // A `contentReference` element takes the class of the element it
    // references: `Questionnaire.item.item` is `#Questionnaire.item`.
    assert_eq!(
        table.element_row("Questionnaire.item.item").qualified_name,
        table.component_row("Questionnaire.item").qualified_name
    );

    // Ancestry: its own type and the baseDefinition chain.
    for (fhir, want) in [
        ("string", &["string", "Element"][..]),
        ("Patient", &["Patient", "DomainResource", "Resource"][..]),
        ("code", &["code", "string", "Element"][..]),
    ] {
        assert_eq!(table.type_row(fhir).assignable_to, set(want), "{fhir}");
    }
    assert_eq!(
        table.component_row("Patient.contact").assignable_to,
        set(&["BackboneElement", "Element"])
    );
    // A component inside a datatype is an `Element`: `Timing.repeat`.
    assert_eq!(
        table.component_row("Timing.repeat").assignable_to,
        set(&["Element"])
    );

    // The classes HAPI adds that have no FHIR type are not rows, and no FHIR
    // type name stands for them.
    for hapi_only in [
        "Base",
        "Type",
        "PrimitiveType",
        "BaseDateTimeType",
        "BackboneType",
        "MetadataResource",
        "BaseReference",
        "BaseNarrative",
        "BaseExtension",
        "BaseResource",
        "BaseBinary",
    ] {
        assert!(
            table.rows.iter().all(|r| r.simple_name != hapi_only),
            "{hapi_only} must not be a row"
        );
        assert!(
            table
                .rows
                .iter()
                .all(|r| !r.assignable_to.contains(hapi_only)),
            "{hapi_only} must not appear in any ancestry set"
        );
    }

    // Interface kinds, against the pinned run's own type kinds.
    let report = pinned_run(generator_under_test().as_ref(), "ancestry-002-report");
    for ty in &report.types {
        if QUANTITY_PROFILES.contains(&ty.fhir_name.as_str()) {
            continue;
        }
        let kinds = match ty.kind {
            TypeKind::Primitive | TypeKind::Complex | TypeKind::Resource => {
                &table.type_row(&ty.fhir_name).interfaces
            }
            TypeKind::Backbone => &table.component_row(&ty.fhir_name).interfaces,
            _ => continue,
        };
        match ty.kind {
            TypeKind::Primitive => {
                assert!(
                    kinds.contains(&InterfaceKind::IPrimitiveType),
                    "{}",
                    ty.fhir_name
                );
                assert!(
                    !kinds.contains(&InterfaceKind::ICompositeType),
                    "{}",
                    ty.fhir_name
                );
            }
            TypeKind::Complex => assert_eq!(
                kinds.contains(&InterfaceKind::ICompositeType),
                ty.instantiable,
                "{}: every concrete complex-type row is ICompositeType, and the abstract \
                 Element and BackboneElement are not",
                ty.fhir_name
            ),
            TypeKind::Backbone => assert!(
                !kinds.contains(&InterfaceKind::ICompositeType),
                "{}: a nested component is not ICompositeType",
                ty.fhir_name
            ),
            _ => {}
        }
    }
    for name in ["Element", "BackboneElement"] {
        assert!(
            !table
                .type_row(name)
                .interfaces
                .contains(&InterfaceKind::ICompositeType),
            "{name} is not ICompositeType"
        );
    }
    for name in ["Reference", "Meta", "Narrative", "Extension"] {
        assert!(
            table
                .type_row(name)
                .interfaces
                .contains(&InterfaceKind::ICompositeType),
            "{name} is ICompositeType"
        );
    }
    assert!(
        table
            .enumeration_row()
            .interfaces
            .contains(&InterfaceKind::IPrimitiveType)
    );
    assert!(
        table
            .type_row("Reference")
            .interfaces
            .contains(&InterfaceKind::IBaseReference)
    );

    // Element value classes: a `System.*` type is read through its
    // `structuredefinition-fhir-type` extension.
    assert_eq!(table.element_row("Element.id").simple_name, "StringType");
    assert_eq!(table.element_row("Extension.url").simple_name, "UriType");
    // A `Quantity` element profiled `SimpleQuantity` is still a `Quantity`.
    assert_eq!(
        table
            .element_row("Observation.referenceRange.low")
            .simple_name,
        "Quantity"
    );
    // A choice element's value class is that of the chosen type.
    assert_eq!(
        table.element_row("Observation.valueQuantity").simple_name,
        "Quantity"
    );
    assert_eq!(
        table.element_row("Observation.valueString").simple_name,
        "StringType"
    );
}

// ---------------------------------------------------------------------------
// codegen.ancestry.003
// ---------------------------------------------------------------------------

// covers: codegen.ancestry.003
#[test]
fn a_code_element_is_an_enumeration_exactly_when_its_required_binding_is_closed_and_not_a_type_list()
 {
    let table = pinned_class_table("ancestry-003");
    let enumeration = table.enumeration_row().qualified_name.clone();

    for path in ["Patient.gender", "Observation.status", "Address.use"] {
        assert_eq!(
            table.element_row(path).qualified_name,
            enumeration,
            "{path}"
        );
    }
    for path in [
        // A value set over `urn:ietf:bcp:13`, not in the package.
        "Attachment.contentType",
        // The `ResourceType` binding.
        "Questionnaire.subjectType",
        // The resource-type and data-type lists HAPI holds as `CodeType`.
        "SearchParameter.base",
        "DataRequirement.type",
    ] {
        assert_eq!(table.element_row(path).simple_name, "CodeType", "{path}");
    }

    // Inherited fields follow the same rule: a canonical resource's `status`
    // (declared on `MetadataResource`) and the `Quantity` specialisations'
    // `comparator`.
    for path in [
        "ValueSet.status",
        "CodeSystem.status",
        "Age.comparator",
        "Count.comparator",
        "Distance.comparator",
        "Duration.comparator",
    ] {
        assert_eq!(
            table.element_row(path).qualified_name,
            enumeration,
            "{path}"
        );
    }

    // The rule predicts 324; the model holds 323 of them as `Enumeration` and
    // no other element. The one exception is
    // `ImplementationGuide.definition.parameter.code` (`codegen.ancestry.004`).
    let held: Vec<&String> = table
        .elements
        .iter()
        .filter(|(_, c)| c.by_name == enumeration)
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        held.len(),
        323,
        "the table must hold exactly 323 Enumeration elements, not {}",
        held.len()
    );
    assert_eq!(
        table
            .element_row("ImplementationGuide.definition.parameter.code")
            .simple_name,
        "StringType",
        "the one element the rule predicts that the model does not hold as Enumeration"
    );
}

// ---------------------------------------------------------------------------
// codegen.ancestry.004
// ---------------------------------------------------------------------------

// covers: codegen.ancestry.004
#[test]
fn the_exception_file_lists_every_place_hapis_class_tree_differs_each_with_its_citation() {
    let entries = read_exception_entries(&exception_file_text());

    // Every entry names its kind, target and source, and carries an effect.
    for entry in &entries {
        for key in ["kind", "target", "source"] {
            assert!(
                entry.get(key).is_some(),
                "an entry must have `{key}`: {entry:?}"
            );
        }
        assert!(
            entry.pairs.len() > 3,
            "an entry must state its effect, not only kind, target and source: {entry:?}"
        );
        let kind = entry.get("kind").unwrap_or_default();
        assert!(
            ["superclass", "element-class", "runtime-class", "class-name"].contains(&kind),
            "an entry's kind must be one of the four: {entry:?}"
        );
        let source = entry.get("source").unwrap_or_default();
        assert!(
            source.contains(".java:"),
            "an entry's source must cite <file>.java:<line>: {entry:?}"
        );
        // Where `reference/` is cloned (never in CI), the cited file exists.
        if repo_path(MODEL_SOURCE_DIR).is_dir() {
            for cited in source.split(|c: char| c == ';' || c == ',' || c.is_whitespace()) {
                if let Some((file, _line)) = cited.split_once(".java:") {
                    let file = file.trim_start_matches(['(', '`']);
                    assert!(
                        repo_path(MODEL_SOURCE_DIR)
                            .join(format!("{file}.java"))
                            .is_file(),
                        "{source}: {file}.java must exist under {MODEL_SOURCE_DIR}"
                    );
                }
            }
        }
    }

    // Its entries at this change are exactly these.
    let got: BTreeSet<(String, String)> = entries
        .iter()
        .map(|e| {
            (
                e.get("kind").unwrap_or_default().to_string(),
                e.get("target").unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert_eq!(got.len(), entries.len(), "no entry is listed twice");
    let mut want: BTreeSet<(String, String)> = BTreeSet::new();
    for target in SUPERCLASS_EXCEPTIONS {
        want.insert(("superclass".to_string(), target.to_string()));
    }
    for target in [
        "Resource.id",
        "ImplementationGuide.definition.parameter.code",
    ] {
        want.insert(("element-class".to_string(), target.to_string()));
    }
    want.insert(("runtime-class".to_string(), "Narrative.div".to_string()));
    for (target, _) in CLASS_NAME_EXCEPTIONS {
        want.insert(("class-name".to_string(), target.to_string()));
    }
    assert_eq!(got, want, "the exception file's entries");
    assert_eq!(entries.len(), 9 + 2 + 1 + 13);

    // The table applies them.
    let table = pinned_class_table("ancestry-004");
    assert_eq!(
        table.type_row("id").assignable_to,
        set(&["id", "uri", "Element"]),
        "IdType extends UriType, so id is not assignable to string"
    );
    for name in BACKBONE_TYPE_DATATYPES {
        let row = table.type_row(name);
        assert_eq!(
            row.assignable_to,
            set(&[name, "Element"]),
            "{name} extends BackboneType, not BackboneElement"
        );
        assert!(
            row.interfaces.contains(&InterfaceKind::ICompositeType),
            "{name} still implements ICompositeType"
        );
    }
    assert_eq!(table.element_row("Resource.id").simple_name, "IdType");
    assert_eq!(table.element_row("Patient.id").simple_name, "IdType");
    assert_eq!(table.element_row("Element.id").simple_name, "StringType");
    let parameter_code = table.element_row("ImplementationGuide.definition.parameter.code");
    assert_eq!(parameter_code.simple_name, "StringType");
    assert!(parameter_code.assignable_to.contains("string"));
    assert!(!parameter_code.assignable_to.contains("code"));
    assert_eq!(table.element_row("Narrative.div").simple_name, "StringType");
    assert_eq!(
        table.element_row_through_star("Narrative.div").simple_name,
        "XhtmlType"
    );
    for (path, class) in CLASS_NAME_EXCEPTIONS {
        assert_eq!(table.component_row(path).simple_name, class, "{path}");
    }
    assert_eq!(
        table
            .component_row("CodeSystem.concept.designation")
            .qualified_name,
        "org.hl7.fhir.r4.model.CodeSystem$ConceptDefinitionDesignationComponent"
    );

    // The generator refuses an entry without a source, an entry whose target
    // is not in the package, and an entry that changes nothing. Each refused
    // file is the reviewed file with one entry mutated.
    let text = exception_file_text();
    let actor = entries
        .iter()
        .find(|e| e.get("target") == Some("Consent.provision.actor"))
        .expect("the Consent.provision.actor entry");
    let actor_source = format!("\"{}\"", actor.get("source").unwrap_or_default());
    let mut refused: Vec<(&str, String)> = Vec::new();
    // No source: drop the entry's `source` line.
    let no_source: String = text
        .lines()
        .filter(|l| !(l.trim_start().starts_with("source") && l.contains(&actor_source)))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(no_source, text, "the fixture must drop a source line");
    refused.push(("an entry without a source", no_source));
    // A target not in the package.
    let unknown = text.replacen(
        "\"Consent.provision.actor\"",
        "\"Consent.provision.noSuchElement\"",
        1,
    );
    assert_ne!(unknown, text);
    refused.push(("an entry whose target is not in the package", unknown));
    // No effect: the entry renames `Patient.contact` to the name the rule
    // already gives it.
    let no_effect = text
        .replacen("\"Consent.provision.actor\"", "\"Patient.contact\"", 1)
        .replacen("\"provisionActorComponent\"", "\"ContactComponent\"", 1);
    assert!(
        no_effect.contains("\"ContactComponent\""),
        "the fixture must restate the rule's own name"
    );
    refused.push(("an entry that changes nothing", no_effect));
    for (what, body) in refused {
        let dir = scratch_dir("ancestry-004-refused");
        let file = dir.join("r4-class-table-exceptions.toml");
        std::fs::write(&file, body).expect("the fixture can be written");
        let result = class_table_under_test().class_table(&ClassTableRequest {
            exceptions: file,
            ..ClassTableRequest::pinned(&dir.join("out"))
        });
        match result {
            Ok(_) => panic!("{what} must be refused, but the run succeeded"),
            Err(e) => {
                let (exit_code, _kind, message) = e.refusal(what);
                assert_ne!(exit_code, 0, "{what}: a refusal exits non-zero");
                assert!(!message.is_empty(), "{what}: the refusal says why");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// codegen.ancestry.005
// ---------------------------------------------------------------------------

// covers: codegen.ancestry.005
#[test]
fn the_generated_table_gives_path_api_005s_answers() {
    let table = pinned_class_table("ancestry-005");

    for fhir in [
        "string",
        "code",
        "markdown",
        "uri",
        "url",
        "canonical",
        "oid",
        "uuid",
        "integer",
        "positiveInt",
        "unsignedInt",
        "date",
        "dateTime",
        "instant",
        "boolean",
        "decimal",
        "base64Binary",
        "time",
        "xhtml",
        "id",
    ] {
        assert!(
            table
                .type_row(fhir)
                .interfaces
                .contains(&InterfaceKind::IPrimitiveType),
            "{fhir} is IPrimitiveType"
        );
    }
    for (fhir, class) in [
        ("string", "StringType"),
        ("code", "CodeType"),
        ("markdown", "MarkdownType"),
        ("uri", "UriType"),
        ("url", "UrlType"),
        ("canonical", "CanonicalType"),
        ("oid", "OidType"),
        ("uuid", "UuidType"),
        ("integer", "IntegerType"),
        ("positiveInt", "PositiveIntType"),
        ("unsignedInt", "UnsignedIntType"),
        ("date", "DateType"),
        ("dateTime", "DateTimeType"),
        ("instant", "InstantType"),
        ("boolean", "BooleanType"),
        ("decimal", "DecimalType"),
        ("base64Binary", "Base64BinaryType"),
        ("time", "TimeType"),
        ("xhtml", "XhtmlType"),
        ("id", "IdType"),
    ] {
        assert_eq!(table.type_row(fhir).simple_name, class, "{fhir}");
    }
    let enumeration = table.enumeration_row();
    assert!(
        enumeration
            .interfaces
            .contains(&InterfaceKind::IPrimitiveType)
    );
    assert_eq!(enumeration.assignable_to, set(&["Element"]));
    let id = table.type_row("id");
    assert!(id.assignable_to.contains("uri"));
    assert!(!id.assignable_to.contains("string"));

    for name in [
        "HumanName",
        "CodeableConcept",
        "Reference",
        "Meta",
        "Narrative",
        "Extension",
    ] {
        assert!(
            table
                .type_row(name)
                .interfaces
                .contains(&InterfaceKind::ICompositeType),
            "{name} is ICompositeType"
        );
    }
    assert!(
        table
            .type_row("Reference")
            .interfaces
            .contains(&InterfaceKind::IBaseReference)
    );
    for name in ["Age", "Count", "Distance", "Duration"] {
        assert!(
            table.type_row(name).assignable_to.contains("Quantity"),
            "{name} is assignable to Quantity"
        );
    }
    let contact = table.component_row("Patient.contact");
    assert_eq!(contact.simple_name, "ContactComponent");
    assert_eq!(
        contact.qualified_name,
        "org.hl7.fhir.r4.model.Patient$ContactComponent"
    );
    assert!(contact.assignable_to.contains("BackboneElement"));
    assert!(contact.assignable_to.contains("Element"));
    assert!(!contact.interfaces.contains(&InterfaceKind::ICompositeType));
    assert_eq!(
        table.type_row("Patient").assignable_to,
        set(&["Patient", "DomainResource", "Resource"])
    );
    assert!(
        !table
            .type_row("Dosage")
            .assignable_to
            .contains("BackboneElement")
    );

    for (path, class) in [
        ("Patient.gender", "Enumeration"),
        ("Observation.status", "Enumeration"),
        ("Patient.active", "BooleanType"),
        ("Patient.birthDate", "DateType"),
        ("Patient.name", "HumanName"),
        ("Patient.contact", "ContactComponent"),
        ("Observation.code", "CodeableConcept"),
        ("Observation.subject", "Reference"),
        ("Patient.id", "IdType"),
        ("Narrative.div", "StringType"),
    ] {
        assert_eq!(table.element_row(path).simple_name, class, "{path}");
    }
    assert_eq!(
        table.element_row_through_star("Narrative.div").simple_name,
        "XhtmlType"
    );
}
