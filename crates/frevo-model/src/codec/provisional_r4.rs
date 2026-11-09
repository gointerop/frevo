//! A provisional subset of the R4 type and element-order tables.
//!
//! **This is not `@generated` output, and it is not the final table.** Tasks 7
//! to 14 of `add-fhirpath-and-codegen-r4` generate
//! `crates/frevo-model/src/r4/` from `hl7.fhir.r4.core#4.0.1`, and that output
//! replaces this module wholesale. Nothing outside `codec` may depend on it:
//! the codec reaches it only through [`super::tables::Tables`], so the swap is
//! one `const` rebinding in `codec/mod.rs`.
//!
//! Why it exists at all. Task 17 is the task that decides whether constitution
//! rule 6.1 -- store and serve canonical resource JSON as bytes, and never
//! reparse a read that returns a resource unchanged -- is achievable, and
//! `design.md` section 10 names a missed `model.perf.001` as an owner
//! checkpoint. That question cannot wait for Phase B to finish, so the codec is
//! built and measured against the element orders and types of the handful of
//! types the Phase C oracles exercise.
//!
//! Provenance of the data below: the R4 StructureDefinition snapshots of
//! `hl7.fhir.r4.core#4.0.1`, element order as the snapshot declares it. The
//! package is not vendored in this repository (task 6 fetches it into a
//! git-ignored directory), so these rows are **unverified against the package**
//! and are recorded in `tasks/blockers.md` as such. The generator's output is
//! the authority; a disagreement between the two is a defect in this file, not
//! in the generator.
//!
//! Elements whose subtree no Phase C oracle enters are declared with their type
//! and cardinality, but that type carries no elements, so navigating into one
//! reports an unknown type rather than silently answering wrongly.

use super::tables::{ElementDef, JsonForm, Tables, TypeDef, TypeKind};

const fn one(name: &'static str, types: &'static [&'static str]) -> ElementDef {
    ElementDef {
        name,
        choice: false,
        repeating: false,
        types,
    }
}

const fn many(name: &'static str, types: &'static [&'static str]) -> ElementDef {
    ElementDef {
        name,
        choice: false,
        repeating: true,
        types,
    }
}

const fn pick(name: &'static str, types: &'static [&'static str]) -> ElementDef {
    ElementDef {
        name,
        choice: true,
        repeating: false,
        types,
    }
}

const fn primitive(name: &'static str, base: Option<&'static str>, form: JsonForm) -> TypeDef {
    TypeDef {
        name,
        base,
        kind: TypeKind::Primitive,
        json_form: form,
        elements: &[],
    }
}

const fn complex(
    name: &'static str,
    base: Option<&'static str>,
    elements: &'static [ElementDef],
) -> TypeDef {
    TypeDef {
        name,
        base,
        kind: TypeKind::Complex,
        json_form: JsonForm::Quoted,
        elements,
    }
}

const fn resource(name: &'static str, elements: &'static [ElementDef]) -> TypeDef {
    TypeDef {
        name,
        base: Some("DomainResource"),
        kind: TypeKind::Resource,
        json_form: JsonForm::Quoted,
        elements,
    }
}

const ELEMENTS_ELEMENT: &[ElementDef] =
    &[one("id", &["string"]), many("extension", &["Extension"])];

const ELEMENTS_BACKBONEELEMENT: &[ElementDef] = &[many("modifierExtension", &["Extension"])];

const ELEMENTS_EXTENSION: &[ElementDef] = &[
    one("url", &["uri"]),
    pick(
        "value",
        &[
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
            "Address",
            "Age",
            "Annotation",
            "Attachment",
            "CodeableConcept",
            "Coding",
            "ContactPoint",
            "Count",
            "Distance",
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
        ],
    ),
];

const ELEMENTS_NARRATIVE: &[ElementDef] = &[one("status", &["code"]), one("div", &["xhtml"])];

const ELEMENTS_META: &[ElementDef] = &[
    one("versionId", &["id"]),
    one("lastUpdated", &["instant"]),
    one("source", &["uri"]),
    many("profile", &["canonical"]),
    many("security", &["Coding"]),
    many("tag", &["Coding"]),
];

const ELEMENTS_CODING: &[ElementDef] = &[
    one("system", &["uri"]),
    one("version", &["string"]),
    one("code", &["code"]),
    one("display", &["string"]),
    one("userSelected", &["boolean"]),
];

const ELEMENTS_CODEABLECONCEPT: &[ElementDef] =
    &[many("coding", &["Coding"]), one("text", &["string"])];

const ELEMENTS_PERIOD: &[ElementDef] = &[one("start", &["dateTime"]), one("end", &["dateTime"])];

const ELEMENTS_QUANTITY: &[ElementDef] = &[
    one("value", &["decimal"]),
    one("comparator", &["code"]),
    one("unit", &["string"]),
    one("system", &["uri"]),
    one("code", &["code"]),
];

const ELEMENTS_AGE: &[ElementDef] = &[];

const ELEMENTS_COUNT: &[ElementDef] = &[];

const ELEMENTS_DISTANCE: &[ElementDef] = &[];

const ELEMENTS_DURATION: &[ElementDef] = &[];

const ELEMENTS_MONEYQUANTITY: &[ElementDef] = &[];

const ELEMENTS_SIMPLEQUANTITY: &[ElementDef] = &[];

const ELEMENTS_MONEY: &[ElementDef] = &[one("value", &["decimal"]), one("currency", &["code"])];

const ELEMENTS_RANGE: &[ElementDef] = &[
    one("low", &["SimpleQuantity"]),
    one("high", &["SimpleQuantity"]),
];

const ELEMENTS_RATIO: &[ElementDef] = &[
    one("numerator", &["Quantity"]),
    one("denominator", &["Quantity"]),
];

const ELEMENTS_SAMPLEDDATA: &[ElementDef] = &[
    one("origin", &["SimpleQuantity"]),
    one("period", &["decimal"]),
    one("factor", &["decimal"]),
    one("lowerLimit", &["decimal"]),
    one("upperLimit", &["decimal"]),
    one("dimensions", &["positiveInt"]),
    one("data", &["string"]),
];

const ELEMENTS_IDENTIFIER: &[ElementDef] = &[
    one("use", &["code"]),
    one("type", &["CodeableConcept"]),
    one("system", &["uri"]),
    one("value", &["string"]),
    one("period", &["Period"]),
    one("assigner", &["Reference"]),
];

const ELEMENTS_REFERENCE: &[ElementDef] = &[
    one("reference", &["string"]),
    one("type", &["uri"]),
    one("identifier", &["Identifier"]),
    one("display", &["string"]),
];

const ELEMENTS_HUMANNAME: &[ElementDef] = &[
    one("use", &["code"]),
    one("text", &["string"]),
    one("family", &["string"]),
    many("given", &["string"]),
    many("prefix", &["string"]),
    many("suffix", &["string"]),
    one("period", &["Period"]),
];

const ELEMENTS_ADDRESS: &[ElementDef] = &[
    one("use", &["code"]),
    one("type", &["code"]),
    one("text", &["string"]),
    many("line", &["string"]),
    one("city", &["string"]),
    one("district", &["string"]),
    one("state", &["string"]),
    one("postalCode", &["string"]),
    one("country", &["string"]),
    one("period", &["Period"]),
];

const ELEMENTS_CONTACTPOINT: &[ElementDef] = &[
    one("system", &["code"]),
    one("value", &["string"]),
    one("use", &["code"]),
    one("rank", &["positiveInt"]),
    one("period", &["Period"]),
];

const ELEMENTS_ATTACHMENT: &[ElementDef] = &[
    one("contentType", &["code"]),
    one("language", &["code"]),
    one("data", &["base64Binary"]),
    one("url", &["url"]),
    one("size", &["unsignedInt"]),
    one("hash", &["base64Binary"]),
    one("title", &["string"]),
    one("creation", &["dateTime"]),
];

const ELEMENTS_ANNOTATION: &[ElementDef] = &[
    pick("author", &["Reference", "string"]),
    one("time", &["dateTime"]),
    one("text", &["markdown"]),
];

const ELEMENTS_SIGNATURE: &[ElementDef] = &[
    many("type", &["Coding"]),
    one("when", &["instant"]),
    one("who", &["Reference"]),
    one("onBehalfOf", &["Reference"]),
    one("targetFormat", &["code"]),
    one("sigFormat", &["code"]),
    one("data", &["base64Binary"]),
];

const ELEMENTS_TIMING: &[ElementDef] = &[
    many("event", &["dateTime"]),
    one("repeat", &["BackboneElement"]),
    one("code", &["CodeableConcept"]),
];

const ELEMENTS_RESOURCE: &[ElementDef] = &[
    one("id", &["id"]),
    one("meta", &["Meta"]),
    one("implicitRules", &["uri"]),
    one("language", &["code"]),
];

const ELEMENTS_DOMAINRESOURCE: &[ElementDef] = &[
    one("text", &["Narrative"]),
    many("contained", &["AnyResource"]),
    many("extension", &["Extension"]),
    many("modifierExtension", &["Extension"]),
];

const ELEMENTS_PATIENT_CONTACT: &[ElementDef] = &[];

const ELEMENTS_PATIENT_COMMUNICATION: &[ElementDef] = &[];

const ELEMENTS_PATIENT_LINK: &[ElementDef] = &[];

const ELEMENTS_OBSERVATION_REFERENCERANGE: &[ElementDef] = &[];

const ELEMENTS_OBSERVATION_COMPONENT: &[ElementDef] = &[];

const ELEMENTS_BUNDLE_LINK: &[ElementDef] = &[];

const ELEMENTS_BUNDLE_ENTRY_SEARCH: &[ElementDef] = &[];

const ELEMENTS_BUNDLE_ENTRY_REQUEST: &[ElementDef] = &[];

const ELEMENTS_BUNDLE_ENTRY_RESPONSE: &[ElementDef] = &[];

const ELEMENTS_ORGANIZATION_CONTACT: &[ElementDef] = &[];

const ELEMENTS_PRACTITIONER_QUALIFICATION: &[ElementDef] = &[];

const ELEMENTS_LOCATION_POSITION: &[ElementDef] = &[];

const ELEMENTS_LOCATION_HOURSOFOPERATION: &[ElementDef] = &[];

const ELEMENTS_PATIENT: &[ElementDef] = &[
    many("identifier", &["Identifier"]),
    one("active", &["boolean"]),
    many("name", &["HumanName"]),
    many("telecom", &["ContactPoint"]),
    one("gender", &["code"]),
    one("birthDate", &["date"]),
    pick("deceased", &["boolean", "dateTime"]),
    many("address", &["Address"]),
    one("maritalStatus", &["CodeableConcept"]),
    pick("multipleBirth", &["boolean", "integer"]),
    many("photo", &["Attachment"]),
    many("contact", &["Patient.contact"]),
    many("communication", &["Patient.communication"]),
    many("generalPractitioner", &["Reference"]),
    one("managingOrganization", &["Reference"]),
    many("link", &["Patient.link"]),
];

const ELEMENTS_OBSERVATION: &[ElementDef] = &[
    many("identifier", &["Identifier"]),
    many("basedOn", &["Reference"]),
    many("partOf", &["Reference"]),
    one("status", &["code"]),
    many("category", &["CodeableConcept"]),
    one("code", &["CodeableConcept"]),
    one("subject", &["Reference"]),
    many("focus", &["Reference"]),
    one("encounter", &["Reference"]),
    pick("effective", &["dateTime", "Period", "Timing", "instant"]),
    one("issued", &["instant"]),
    many("performer", &["Reference"]),
    pick(
        "value",
        &[
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
        ],
    ),
    one("dataAbsentReason", &["CodeableConcept"]),
    many("interpretation", &["CodeableConcept"]),
    many("note", &["Annotation"]),
    one("bodySite", &["CodeableConcept"]),
    one("method", &["CodeableConcept"]),
    one("specimen", &["Reference"]),
    one("device", &["Reference"]),
    many("referenceRange", &["Observation.referenceRange"]),
    many("hasMember", &["Reference"]),
    many("derivedFrom", &["Reference"]),
    many("component", &["Observation.component"]),
];

const ELEMENTS_ORGANIZATION: &[ElementDef] = &[
    many("identifier", &["Identifier"]),
    one("active", &["boolean"]),
    many("type", &["CodeableConcept"]),
    one("name", &["string"]),
    many("alias", &["string"]),
    many("telecom", &["ContactPoint"]),
    many("address", &["Address"]),
    one("partOf", &["Reference"]),
    many("contact", &["Organization.contact"]),
    many("endpoint", &["Reference"]),
];

const ELEMENTS_PRACTITIONER: &[ElementDef] = &[
    many("identifier", &["Identifier"]),
    one("active", &["boolean"]),
    many("name", &["HumanName"]),
    many("telecom", &["ContactPoint"]),
    many("address", &["Address"]),
    one("gender", &["code"]),
    one("birthDate", &["date"]),
    many("photo", &["Attachment"]),
    many("qualification", &["Practitioner.qualification"]),
    many("communication", &["CodeableConcept"]),
];

const ELEMENTS_LOCATION: &[ElementDef] = &[
    many("identifier", &["Identifier"]),
    one("status", &["code"]),
    one("operationalStatus", &["Coding"]),
    one("name", &["string"]),
    many("alias", &["string"]),
    one("description", &["string"]),
    one("mode", &["code"]),
    many("type", &["CodeableConcept"]),
    many("telecom", &["ContactPoint"]),
    one("address", &["Address"]),
    one("physicalType", &["CodeableConcept"]),
    one("position", &["Location.position"]),
    one("managingOrganization", &["Reference"]),
    one("partOf", &["Reference"]),
    many("hoursOfOperation", &["Location.hoursOfOperation"]),
    one("availabilityExceptions", &["string"]),
    many("endpoint", &["Reference"]),
];

const ELEMENTS_BUNDLE: &[ElementDef] = &[
    one("identifier", &["Identifier"]),
    one("type", &["code"]),
    one("timestamp", &["instant"]),
    one("total", &["unsignedInt"]),
    many("link", &["Bundle.link"]),
    many("entry", &["Bundle.entry"]),
    one("signature", &["Signature"]),
];

const ELEMENTS_BUNDLE_ENTRY: &[ElementDef] = &[
    many("link", &["Bundle.link"]),
    one("fullUrl", &["uri"]),
    one("resource", &["AnyResource"]),
    one("search", &["Bundle.entry.search"]),
    one("request", &["Bundle.entry.request"]),
    one("response", &["Bundle.entry.response"]),
];

/// Every type the provisional table knows, in no significant order.
const TYPES: &[TypeDef] = &[
    primitive("base64Binary", None, JsonForm::Quoted),
    primitive("boolean", None, JsonForm::Boolean),
    primitive("canonical", Some("uri"), JsonForm::Quoted),
    primitive("code", Some("string"), JsonForm::Quoted),
    primitive("date", None, JsonForm::Quoted),
    primitive("dateTime", None, JsonForm::Quoted),
    primitive("decimal", None, JsonForm::Number),
    primitive("id", Some("string"), JsonForm::Quoted),
    primitive("instant", None, JsonForm::Quoted),
    primitive("integer", None, JsonForm::Number),
    primitive("markdown", Some("string"), JsonForm::Quoted),
    primitive("oid", Some("uri"), JsonForm::Quoted),
    primitive("positiveInt", Some("integer"), JsonForm::Number),
    primitive("string", None, JsonForm::Quoted),
    primitive("time", None, JsonForm::Quoted),
    primitive("unsignedInt", Some("integer"), JsonForm::Number),
    primitive("uri", None, JsonForm::Quoted),
    primitive("url", Some("uri"), JsonForm::Quoted),
    primitive("uuid", Some("uri"), JsonForm::Quoted),
    primitive("xhtml", None, JsonForm::Quoted),
    complex("Element", None, ELEMENTS_ELEMENT),
    complex("BackboneElement", Some("Element"), ELEMENTS_BACKBONEELEMENT),
    complex("Extension", Some("Element"), ELEMENTS_EXTENSION),
    complex("Narrative", Some("Element"), ELEMENTS_NARRATIVE),
    complex("Meta", Some("Element"), ELEMENTS_META),
    complex("Coding", Some("Element"), ELEMENTS_CODING),
    complex("CodeableConcept", Some("Element"), ELEMENTS_CODEABLECONCEPT),
    complex("Period", Some("Element"), ELEMENTS_PERIOD),
    complex("Quantity", Some("Element"), ELEMENTS_QUANTITY),
    complex("Age", Some("Quantity"), ELEMENTS_AGE),
    complex("Count", Some("Quantity"), ELEMENTS_COUNT),
    complex("Distance", Some("Quantity"), ELEMENTS_DISTANCE),
    complex("Duration", Some("Quantity"), ELEMENTS_DURATION),
    complex("MoneyQuantity", Some("Quantity"), ELEMENTS_MONEYQUANTITY),
    complex("SimpleQuantity", Some("Quantity"), ELEMENTS_SIMPLEQUANTITY),
    complex("Money", Some("Element"), ELEMENTS_MONEY),
    complex("Range", Some("Element"), ELEMENTS_RANGE),
    complex("Ratio", Some("Element"), ELEMENTS_RATIO),
    complex("SampledData", Some("Element"), ELEMENTS_SAMPLEDDATA),
    complex("Identifier", Some("Element"), ELEMENTS_IDENTIFIER),
    complex("Reference", Some("Element"), ELEMENTS_REFERENCE),
    complex("HumanName", Some("Element"), ELEMENTS_HUMANNAME),
    complex("Address", Some("Element"), ELEMENTS_ADDRESS),
    complex("ContactPoint", Some("Element"), ELEMENTS_CONTACTPOINT),
    complex("Attachment", Some("Element"), ELEMENTS_ATTACHMENT),
    complex("Annotation", Some("Element"), ELEMENTS_ANNOTATION),
    complex("Signature", Some("Element"), ELEMENTS_SIGNATURE),
    complex("Timing", Some("BackboneElement"), ELEMENTS_TIMING),
    complex("Resource", None, ELEMENTS_RESOURCE),
    complex("DomainResource", Some("Resource"), ELEMENTS_DOMAINRESOURCE),
    complex(
        "Patient.contact",
        Some("BackboneElement"),
        ELEMENTS_PATIENT_CONTACT,
    ),
    complex(
        "Patient.communication",
        Some("BackboneElement"),
        ELEMENTS_PATIENT_COMMUNICATION,
    ),
    complex(
        "Patient.link",
        Some("BackboneElement"),
        ELEMENTS_PATIENT_LINK,
    ),
    complex(
        "Observation.referenceRange",
        Some("BackboneElement"),
        ELEMENTS_OBSERVATION_REFERENCERANGE,
    ),
    complex(
        "Observation.component",
        Some("BackboneElement"),
        ELEMENTS_OBSERVATION_COMPONENT,
    ),
    complex("Bundle.link", Some("BackboneElement"), ELEMENTS_BUNDLE_LINK),
    complex(
        "Bundle.entry.search",
        Some("BackboneElement"),
        ELEMENTS_BUNDLE_ENTRY_SEARCH,
    ),
    complex(
        "Bundle.entry.request",
        Some("BackboneElement"),
        ELEMENTS_BUNDLE_ENTRY_REQUEST,
    ),
    complex(
        "Bundle.entry.response",
        Some("BackboneElement"),
        ELEMENTS_BUNDLE_ENTRY_RESPONSE,
    ),
    complex(
        "Organization.contact",
        Some("BackboneElement"),
        ELEMENTS_ORGANIZATION_CONTACT,
    ),
    complex(
        "Practitioner.qualification",
        Some("BackboneElement"),
        ELEMENTS_PRACTITIONER_QUALIFICATION,
    ),
    complex(
        "Location.position",
        Some("BackboneElement"),
        ELEMENTS_LOCATION_POSITION,
    ),
    complex(
        "Location.hoursOfOperation",
        Some("BackboneElement"),
        ELEMENTS_LOCATION_HOURSOFOPERATION,
    ),
    TypeDef {
        name: "AnyResource",
        base: None,
        kind: TypeKind::AnyResource,
        json_form: JsonForm::Quoted,
        elements: &[],
    },
    resource("Patient", ELEMENTS_PATIENT),
    resource("Observation", ELEMENTS_OBSERVATION),
    resource("Organization", ELEMENTS_ORGANIZATION),
    resource("Practitioner", ELEMENTS_PRACTITIONER),
    resource("Location", ELEMENTS_LOCATION),
    TypeDef {
        name: "Bundle",
        base: Some("Resource"),
        kind: TypeKind::Resource,
        json_form: JsonForm::Quoted,
        elements: ELEMENTS_BUNDLE,
    },
    complex(
        "Bundle.entry",
        Some("BackboneElement"),
        ELEMENTS_BUNDLE_ENTRY,
    ),
];

/// The tables the codec reads until `src/r4/` is generated.
pub const TABLES: Tables = Tables { types: TYPES };
