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
//! verifies an input package, writes files, and reports what it found. None of
//! that exists yet. Every test below drives real inputs — the pinned package,
//! the HAPI cross-check tree, the interim example set, hand-written synthetic
//! StructureDefinitions — through [`generator_under_test`], which today returns
//! a binding whose every method answers [`CodegenError::NotImplemented`].
//!
//! `NotImplemented` is an **error value, not a verdict**. It can never be read
//! as a pass, including on the scenarios that require the generator to *refuse*
//! (`codegen.inputs.002`, `codegen.inputs.003`, `codegen.inputs.006`,
//! `codegen.naming.003`): those tests assert the refusal's *kind* and the
//! content of its message, so a generator that produced nothing at all fails
//! them. One edit to [`generator_under_test`] points the whole file at the real
//! generator.
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
//! # Three conflicts between this delta and HAPI, flagged and not coded around
//!
//! Each is written to the scenario **as the delta states it**, with the
//! upstream evidence recorded at the test, and reported to the owner rather
//! than silently reinterpreted.
//!
//! 1. `codegen.cardinality.002` and `codegen.cardinality.004` require a
//!    missing-required-element diagnostic for `Observation.status` and for a
//!    `1..*` element given an empty array, and a strict-mode failure.
//!    `IParserErrorHandler.missingRequiredElement` has exactly four call sites
//!    in HAPI's JSON parser — `JsonParser.java:1503`, `:1514`, `:1995` and
//!    `:2004` — and all four are about `Extension.url` or `Extension.value`.
//!    `ParserState.java` contains no minimum-cardinality check at all. HAPI
//!    therefore reports **nothing** for a missing `Observation.status`, in
//!    either mode, and a strict parse of that document succeeds.
//! 2. `codegen.choice.004` requires a decode of a document carrying both
//!    `valueString` and `valueQuantity` to **fail** with an error naming the
//!    element path and both properties. HAPI routes the second one to
//!    `unexpectedRepeatingElement` keyed on the choice **field** name — the
//!    `RuntimeChildChoiceDefinition.getField().getName()` at
//!    `ParserState.java:663-671`, which is `value`, neither a path nor both
//!    property names — and `LenientErrorHandler.unexpectedRepeatingElement`
//!    only logs, so the default decode succeeds with the first value kept.
//!    Only `StrictErrorHandler` throws (`Msg.code(1823)`).
//! 3. `codegen.binding.002` requires a decode of `"gender": "M"` to survive and
//!    re-serialise unchanged. The surviving-raw-string half is confirmed by
//!    HAPI's own `CaseInsensitiveEnumParsingR4Test` — but that test installs a
//!    mock error handler. With the default `LenientErrorHandler`, an unknown
//!    enum code reaches `invalidValue` (`ParserState.java:1510`) with a
//!    non-blank value, and `LenientErrorHandler.invalidValue` delegates a
//!    non-blank invalid value to the strict handler, which throws. So the delta
//!    needs to say which decode configuration preserves the code.
//!
//! Scenario coverage: all 49 `codegen.*` scenarios.

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
    /// `codegen.cardinality.002`, `codegen.cardinality.004`.
    MissingRequiredElement,
    /// `codegen.cardinality.005`.
    InvalidValue,
    /// `codegen.choice.004`.
    DuplicateChoiceValue,
    /// `codegen.binding.002`.
    CodeNotInRequiredBinding,
}

impl DiagnosticKind {
    pub const ALL: [DiagnosticKind; 4] = [
        DiagnosticKind::MissingRequiredElement,
        DiagnosticKind::InvalidValue,
        DiagnosticKind::DuplicateChoiceValue,
        DiagnosticKind::CodeNotInRequiredBinding,
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
    /// does not need a dependency edge `frevo-codegen` must not have; task 13
    /// binds it to the real parser.
    fn parse_search_expression(&self, expression: &str) -> Result<(), CodegenError>;
}

/// The generator this oracle grades.
///
/// Tasks 6 to 14 replace this body with the real one. Nothing else in this file
/// changes when they do.
fn generator_under_test() -> Box<dyn Codegen> {
    Box::new(BoundGenerator)
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

/// Rust's keywords, so "a deterministic non-keyword Rust name" is checkable
/// rather than asserted (`codegen.naming.002`).
pub const RUST_KEYWORDS: [&str; 51] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

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
fn two_values_for_the_same_choice_element_are_rejected_on_decode() {
    let generator = generator_under_test();

    // CONFLICT WITH HAPI, written to the scenario as the delta states it.
    //
    // The delta requires this decode to fail with an error naming the element
    // path and both properties. HAPI does not do that. `ParserState`'s
    // `enteringNewElement` keys the non-repeatable check on the choice field
    // name -- `RuntimeChildChoiceDefinition.getField().getName()`, which is
    // `value` -- and calls `unexpectedRepeatingElement(null, "value")`
    // (ParserState.java:663-671). `LenientErrorHandler` only logs that, and
    // the parser pushes `SwallowChildrenWholeState`, so the default decode
    // succeeds with `valueString` kept and `valueQuantity` dropped. Only
    // `StrictErrorHandler` throws, with `Msg.code(1823)` and a message naming
    // `value` alone.
    //
    // Both modes are asserted here so the two halves of the conflict are
    // separately visible when this goes red.
    for mode in [DecodeMode::Strict, DecodeMode::Lenient] {
        let error = generator
            .decode(OBSERVATION_WITH_TWO_CHOICE_VALUES, mode)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "two values for Observation.value[x] must fail to decode in {mode:?} mode. \
                     In lenient mode this is the conflict with HAPI recorded above: HAPI keeps \
                     the first value and logs unexpectedRepeatingElement (code \
                     {CODE_UNEXPECTED_REPEATING_ELEMENT})."
                )
            });
        let (_, kind, message) = error.refusal("two values for one choice element");
        assert_eq!(kind, FailureKind::Codec);
        assert!(
            message.contains("Observation.value"),
            "the error must name the element path; it said {message:?}"
        );
        for property in ["valueString", "valueQuantity"] {
            assert!(
                message.contains(property),
                "the error must name {property} as well; it said {message:?}"
            );
        }
    }

    // One value for the same element decodes cleanly, so the rejection is
    // about the duplication and not about the element.
    let single = generator
        .decode(
            &observation_with_value("valueString", r#""6.3""#),
            DecodeMode::Strict,
        )
        .unwrap_or_else(|e| panic!("one value for a choice element must decode: {e}"));
    assert_eq!(single.at("Observation.value").len(), 1);
    assert_eq!(single.diagnostics, Vec::<Diagnostic>::new());
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
fn one_to_one_is_required_and_single_and_its_absence_is_a_diagnostic() {
    let generator = generator_under_test();
    let report = pinned_run(generator.as_ref(), "cardinality-002");

    let status = report.ty_or_panic("Observation").element_or_panic("status");
    assert_eq!(status.max, Max::One, "the element holds at most one value");
    assert_eq!(status.min, 1, "and records that its minimum is 1");

    // CONFLICT WITH HAPI, written to the scenario as the delta states it.
    //
    // `missingRequiredElement` has exactly four call sites in HAPI's JSON
    // parser -- JsonParser.java:1503, :1514, :1995 and :2004 -- and all four
    // are about `Extension.url` or `Extension.value`. `ParserState.java`
    // carries no minimum-cardinality check at all, so HAPI reports *nothing*
    // for a missing `Observation.status` and a strict parse of this document
    // succeeds. The delta's "because LenientErrorHandler.missingRequiredElement
    // logs a warning and returns" is true of the handler and false of the
    // parser, which never calls it here.
    let lenient = generator
        .decode(OBSERVATION_WITHOUT_STATUS, DecodeMode::Lenient)
        .unwrap_or_else(|e| {
            panic!("a lenient decode of an Observation with no status must succeed: {e}")
        });
    assert_eq!(
        lenient.at("Observation.status"),
        Vec::<&ElementValue>::new(),
        "the element stays absent"
    );
    let diagnostic = lenient
        .diagnostic(DiagnosticKind::MissingRequiredElement)
        .unwrap_or_else(|| {
            panic!(
                "the lenient decode must return a missing-required-element diagnostic. \
                 This is the conflict with HAPI recorded above: HAPI's JSON parser never \
                 calls missingRequiredElement for a resource element. It returned {:?}",
                lenient.kinds()
            )
        });
    assert_eq!(
        diagnostic.path, "Observation.status",
        "the diagnostic must name the element"
    );

    // Strict mode fails, naming the same element.
    let error = generator
        .decode(OBSERVATION_WITHOUT_STATUS, DecodeMode::Strict)
        .expect_err("a strict decode of an Observation with no status must fail");
    let (_, kind, message) = error.refusal("a missing required element in strict mode");
    assert_eq!(kind, FailureKind::Codec);
    assert!(
        message.contains("Observation.status"),
        "the strict failure must name Observation.status; it said {message:?}"
    );

    // And in neither mode is the element given a default value.
    assert!(
        lenient
            .values
            .iter()
            .all(|v| v.path != "Observation.status"),
        "a required element must not be defaulted: {:?}",
        lenient.at("Observation.status")
    );
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

    // One participant is enough, so the diagnostic below is about emptiness.
    let one = generator
        .decode(APPOINTMENT_WITH_ONE_PARTICIPANT, DecodeMode::Strict)
        .unwrap_or_else(|e| panic!("one participant must decode: {e}"));
    assert_eq!(one.diagnostics, Vec::<Diagnostic>::new());

    // CONFLICT WITH HAPI, as recorded at `codegen.cardinality.002`: HAPI's
    // JSON parser calls `missingRequiredElement` only for `Extension.url` and
    // `Extension.value`, so an empty array for a `1..*` element is accepted
    // with no diagnostic in either mode.
    let lenient = generator
        .decode(APPOINTMENT_WITH_EMPTY_PARTICIPANT, DecodeMode::Lenient)
        .unwrap_or_else(|e| panic!("an empty array must decode leniently: {e}"));
    assert_eq!(
        lenient.at("Appointment.participant"),
        Vec::<&ElementValue>::new(),
        "the element is absent, not an empty list"
    );
    assert_eq!(
        lenient.repeats("Appointment.participant"),
        Vec::<&ElementValue>::new()
    );
    let diagnostic = lenient
        .diagnostic(DiagnosticKind::MissingRequiredElement)
        .unwrap_or_else(|| {
            panic!(
                "an empty array for a 1..* element must return a missing-required-element \
                 diagnostic. This is the conflict with HAPI recorded at \
                 codegen.cardinality.002. It returned {:?}",
                lenient.kinds()
            )
        });
    assert_eq!(diagnostic.path, "Appointment.participant");

    let error = generator
        .decode(APPOINTMENT_WITH_EMPTY_PARTICIPANT, DecodeMode::Strict)
        .expect_err("the strict decode must fail");
    let (_, kind, message) = error.refusal("an empty array for a 1..* element");
    assert_eq!(kind, FailureKind::Codec);
    assert!(
        message.contains("Appointment.participant"),
        "the failure must name the element; it said {message:?}"
    );
}

// covers: codegen.cardinality.005
#[test]
fn a_required_element_present_but_empty_is_treated_as_absent() {
    let generator = generator_under_test();

    // `ParserState.java:1454-1456`: a `value` of `""` short-circuits before
    // `setValueAsString`, calling `invalidValue(location, "", "Attribute value
    // must not be empty (\"\")")`. `LenientErrorHandler.invalidValue` logs a
    // blank value and raises every other invalid one, which is why a blank is
    // the one invalid value that survives a lenient decode (`model.json.023`).
    let lenient = generator
        .decode(OBSERVATION_WITH_BLANK_STATUS, DecodeMode::Lenient)
        .unwrap_or_else(|e| {
            panic!("a lenient decode of a blank required value must not fail: {e}")
        });
    let diagnostic = lenient
        .diagnostic(DiagnosticKind::InvalidValue)
        .unwrap_or_else(|| {
            panic!(
                "a blank value must return an invalid-value diagnostic, not a \
                 missing-required-element one; it returned {:?}",
                lenient.kinds()
            )
        });
    assert!(
        diagnostic.path.contains("status"),
        "the diagnostic must name the element; it named {:?}",
        diagnostic.path
    );
    assert_eq!(
        lenient.at("Observation.status"),
        Vec::<&ElementValue>::new(),
        "a blank value is treated as absent, not stored as an empty string"
    );

    // The strict decode fails, naming the element.
    let error = generator
        .decode(OBSERVATION_WITH_BLANK_STATUS, DecodeMode::Strict)
        .expect_err("the strict decode of a blank required value must fail");
    let (_, kind, message) = error.refusal("a blank required value");
    assert_eq!(kind, FailureKind::Codec);
    assert!(
        message.contains("status"),
        "the failure must name the element; it said {message:?}"
    );
    // The invalid-value condition, not the missing-element one: HAPI's codes
    // for the two differ, and conflating them would hide which one fired.
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
fn a_code_outside_a_required_binding_is_preserved_not_dropped() {
    let generator = generator_under_test();

    // CONFLICT WITH HAPI, written to the scenario as the delta states it.
    //
    // The preserved-raw-string half is HAPI's own documented behaviour:
    // `CaseInsensitiveEnumParsingR4Test.testEnumValueNotCorrectable_single`
    // asserts `getGenderElement().getValue()` is null while
    // `getValueAsString()` is the original `FooBar`, because
    // `PrimitiveType.setValueAsString` assigns the lexical form before `parse`
    // throws. But that test installs a mock error handler. With the default
    // `LenientErrorHandler`, the unknown code reaches `invalidValue`
    // (ParserState.java:1510) with a non-blank value, and
    // `LenientErrorHandler.invalidValue` delegates a non-blank invalid value to
    // `StrictErrorHandler`, which throws `Msg.code(1821)`. So the delta needs
    // to say which decode configuration this scenario is about.
    let decoded = generator
        .decode(PATIENT_WITH_GENDER_M, DecodeMode::Lenient)
        .unwrap_or_else(|e| {
            panic!(
                "a code outside a required binding must survive the decode so that it can \
                 be re-serialised: {e}. This is the conflict with HAPI recorded above."
            )
        });

    let diagnostic = decoded
        .diagnostic(DiagnosticKind::CodeNotInRequiredBinding)
        .unwrap_or_else(|| {
            panic!(
                "the decode must report the code as not in the required binding; it \
                 reported {:?}",
                decoded.kinds()
            )
        });
    assert_eq!(diagnostic.path, "Patient.gender");
    assert!(
        diagnostic.message.contains('M'),
        "the diagnostic must name the code; it said {:?}",
        diagnostic.message
    );

    // The code is preserved, not dropped and not coerced.
    let values = decoded.at("Patient.gender");
    assert_eq!(values.len(), 1, "the element must survive");
    assert_eq!(
        values[0].value.as_deref(),
        Some("M"),
        "the lexical value is kept exactly"
    );

    // And re-serialising the resource unchanged still emits it.
    let encoded = generator
        .encode(&decoded)
        .unwrap_or_else(|e| panic!("re-encoding must succeed: {e}"));
    assert!(
        encoded.contains(r#""gender":"M""#),
        "re-serialisation must emit the original code: {encoded}"
    );
    let canonical = generator
        .encode_canonical(&decoded)
        .unwrap_or_else(|e| panic!("canonical re-encoding must succeed: {e}"));
    assert!(
        canonical.contains(r#""gender":"M""#),
        "including in canonical form: {canonical}"
    );
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

    // `OperationDefinition.use` does not exist in R4 -- the scenario says
    // "`OperationDefinition.use`-style", and the real `use` elements are on
    // `Identifier`, `HumanName`, `Address`, `ContactPoint` and `Claim`.
    for (type_name, fhir_name) in [
        ("Encounter", "class"),
        ("ElementDefinition", "type"),
        ("Identifier", "use"),
        ("Claim", "use"),
    ] {
        let element = report.ty_or_panic(type_name).element_or_panic(fhir_name);

        // A deterministic non-keyword Rust name.
        assert!(
            is_plain_rust_identifier(&element.rust_name),
            "{type_name}.{fhir_name} must get a non-keyword, non-raw Rust name; it got {:?}",
            element.rust_name
        );
        assert_ne!(element.rust_name, fhir_name);

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

    // Two different keywords must not collapse onto one escape.
    let class_name = report
        .ty_or_panic("Encounter")
        .element_or_panic("class")
        .rust_name
        .clone();
    let type_name = report
        .ty_or_panic("ElementDefinition")
        .element_or_panic("type")
        .rust_name
        .clone();
    let use_name = report
        .ty_or_panic("Identifier")
        .element_or_panic("use")
        .rust_name
        .clone();
    let distinct: BTreeSet<&str> = [class_name.as_str(), type_name.as_str(), use_name.as_str()]
        .into_iter()
        .collect();
    assert_eq!(
        distinct.len(),
        3,
        "class, type and use must escape to three different names: {distinct:?}"
    );

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
