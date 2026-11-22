//! Unit tests for the codec.
//!
//! These assert the same invariants the `model-bytes-lazy-perf` and `model-nav`
//! oracles under `crates/frevo-model/tests/` grade, from inside the crate, so
//! the codec is verified here regardless of when those oracles get their
//! binding. They carry no `covers:` tag: the scenario IDs are owned by the
//! oracle files, and claiming them twice would make the coverage accounting
//! read as broader than it is. The scenario each test corresponds to is named
//! in its doc comment instead.

use std::sync::Arc;

use frevo_path::{Model, Node};

use super::{R4, R4Model, Resource};

fn bytes_of(text: &str) -> Arc<[u8]> {
    Arc::from(text.as_bytes().to_vec().into_boxed_slice())
}

/// The same deterministic `Patient` the oracle's `patient_of_size` builds, so
/// the numbers here describe the same document.
fn patient_of_size(target: usize) -> String {
    let head = concat!(
        r##"{"resourceType":"Patient","id":"perf","##,
        r##""meta":{"versionId":"1","lastUpdated":"2026-01-01T00:00:00.000Z"},"##,
        r##""text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">Perf</div>"},"##,
        r##""active":true,"##,
        r##""name":[{"use":"official","family":"Windsor","given":["Peter","James"]}],"##,
        r##""gender":"male","birthDate":"1974-12-25","##,
        r##""address":["##
    );
    let tail = r##"]}"##;
    let mut out = String::from(head);
    let mut index = 0usize;
    loop {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r##"{{"line":["{index} Example Street","Suite {index}"],"city":"Townsville","district":"Example District","state":"NSW","postalCode":"{:04}","country":"AU"}}"##,
            2000 + index % 8000
        ));
        index += 1;
        if out.len() + tail.len() >= target {
            break;
        }
    }
    out.push_str(tail);
    out
}

const SMALL_PATIENT: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","active":true,"##,
    r##""gender":"male","birthDate":"1974-12-25"}"##
);

/// Deliberately not the generated element order.
const SMALL_PATIENT_PROPERTIES: &[(&str, &str)] = &[
    ("Patient.birthDate", "1974-12-25"),
    ("Patient.gender", "male"),
    ("Patient.active", "true"),
    ("Patient.id", "p"),
];

// -------------------------------------------------------------------------
// Byte preservation — model.bytes.001 .. 006
// -------------------------------------------------------------------------

/// `model.bytes.001`: stored bytes are returned verbatim.
#[test]
fn a_pass_through_returns_the_input_bytes() {
    let text = patient_of_size(4 * 1024);
    let input = bytes_of(&text);
    let resource = Resource::from_stored(Arc::clone(&input));
    let out = resource.to_stored_vec().expect("serialising must succeed");
    assert_eq!(out.as_slice(), &input[..]);
    assert!(!resource.is_modified());
}

/// `model.bytes.001` again, but at the level the server actually uses: the
/// buffer handed back is the *same allocation*, not an equal copy.
#[test]
fn a_pass_through_hands_back_the_same_allocation() {
    let input = bytes_of(SMALL_PATIENT);
    let resource = Resource::from_stored(Arc::clone(&input));
    let handed_back = resource
        .untouched_bytes()
        .expect("an unedited stored resource must hand its buffer back");
    assert!(
        std::ptr::eq(input.as_ptr(), handed_back.as_ptr()),
        "the buffer must be the one it was loaded with, not a copy"
    );
}

/// `model.bytes.002`: a pass-through performs no JSON parse.
#[test]
fn a_pass_through_scans_no_tokens_and_allocates_nothing_proportional() {
    let text = patient_of_size(4 * 1024);
    let resource = Resource::from_stored(bytes_of(&text));
    let mut sink = std::io::sink();
    resource
        .write_to(&mut sink)
        .expect("serialising must succeed");
    let report = resource.report();
    assert_eq!(report.tokens_scanned, 0, "no JSON token may be scanned");
    assert_eq!(
        report.size_proportional_allocations, 0,
        "and no allocation may scale with the document"
    );
    assert!(report.materialised.is_empty());
}

/// `model.bytes.003`: reading one element does not force a full parse.
#[test]
fn reading_one_element_scans_fewer_tokens_than_a_full_parse() {
    let text = patient_of_size(4 * 1024);

    let lazy = Resource::from_stored(bytes_of(&text));
    let read = lazy
        .read("Patient.birthDate")
        .expect("the read must succeed")
        .expect("birthDate is present");
    assert_eq!(read.1.as_deref(), Some("1974-12-25"));
    let lazy_tokens = lazy.report().tokens_scanned;

    let full = Resource::from_stored(bytes_of(&text));
    full.materialise_all().expect("a full parse must succeed");
    let full_tokens = full.report().tokens_scanned;

    assert!(full_tokens > 0, "a full parse must scan tokens");
    assert!(
        lazy_tokens < full_tokens,
        "a lazy read scanned {lazy_tokens} tokens and a full parse {full_tokens}"
    );
    assert!(
        lazy_tokens * 10 < full_tokens,
        "and by a wide margin: {lazy_tokens} against {full_tokens}"
    );
}

/// `model.bytes.004`: reading an element does not disturb pass-through.
#[test]
fn a_read_is_not_a_mutation() {
    let text = patient_of_size(4 * 1024);
    let input = bytes_of(&text);
    let resource = Resource::from_stored(Arc::clone(&input));
    resource.read("Patient.birthDate").expect("the read");
    assert_eq!(
        resource.to_stored_vec().expect("serialising").as_slice(),
        &input[..]
    );
    assert!(!resource.is_modified());
    assert!(
        resource.untouched_bytes().is_some(),
        "and the buffer is still handed back by Arc clone"
    );
}

/// `model.bytes.005`: a resource that was never bytes serialises from its
/// element order.
#[test]
fn a_constructed_resource_serialises_in_generated_element_order() {
    let mut resource = Resource::build("Patient");
    for (path, value) in SMALL_PATIENT_PROPERTIES {
        resource.set(path, value).expect("setting must succeed");
    }
    let out = resource.to_stored_vec().expect("serialising");
    assert_eq!(String::from_utf8_lossy(&out), SMALL_PATIENT);
    assert_eq!(
        resource.report().tokens_scanned,
        0,
        "a resource that was never bytes has no JSON to scan"
    );
}

/// `model.bytes.006`: the byte-backed and constructed forms are
/// indistinguishable to a reader.
#[test]
fn both_forms_read_alike() {
    let decoded = Resource::from_stored(bytes_of(SMALL_PATIENT));
    let mut built = Resource::build("Patient");
    for (path, value) in SMALL_PATIENT_PROPERTIES {
        built.set(path, value).expect("setting");
    }
    for path in ["Patient.birthDate", "Patient.gender", "Patient.active"] {
        let a = decoded.read(path).expect("the read").expect("present");
        let b = built.read(path).expect("the read").expect("present");
        assert_eq!(a.1, b.1, "{path}: equal values");
        assert_eq!(a.0, b.0, "{path}: the same type");
        assert!(a.1.is_some(), "{path}: and the value is really there");
    }
}

// -------------------------------------------------------------------------
// Lazy materialisation — model.lazy.001 .. 006
// -------------------------------------------------------------------------

fn bundle_of(entries: usize) -> String {
    let mut out =
        String::from(r##"{"resourceType":"Bundle","id":"b","type":"collection","entry":["##);
    for index in 0..entries {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r##"{{"fullUrl":"urn:uuid:{index}","resource":{{"resourceType":"Patient","id":"p{index}","name":[{{"family":"Family{index}"}}],"gender":"male"}}}}"##
        ));
    }
    out.push_str("]}");
    out
}

fn patient_with_names(count: usize) -> String {
    let mut out = String::from(r##"{"resourceType":"Patient","id":"p","name":["##);
    for index in 0..count {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r##"{{"family":"Family{index}","given":["Given{index}"]}}"##
        ));
    }
    out.push_str(r##"],"gender":"male"}"##);
    out
}

const PATIENT_WITH_THREE_CONTAINED: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","contained":["##,
    r##"{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"},"##,
    r##"{"resourceType":"Practitioner","id":"prac1","name":[{"family":"Smith"}]},"##,
    r##"{"resourceType":"Location","id":"loc1","name":"Ward 7"}],"##,
    r##""name":[{"family":"Chalmers"}],"gender":"male"}"##
);

fn assert_not_materialised(resource: &Resource, prefix: &str) {
    let report = resource.report();
    let hits: Vec<&String> = report
        .materialised
        .iter()
        .filter(|path| {
            *path == prefix
                || path.starts_with(&format!("{prefix}."))
                || path.starts_with(&format!("{prefix}["))
        })
        .collect();
    assert!(
        hits.is_empty(),
        "{prefix} must report as not materialised, but these were: {hits:?}"
    );
}

/// `model.lazy.001`: an unread sibling is never materialised.
#[test]
fn an_unread_bundle_entry_is_never_materialised() {
    let resource = Resource::from_stored(bytes_of(&bundle_of(100)));
    let read = resource
        .read("Bundle.entry[0].resource.id")
        .expect("the read")
        .expect("present");
    assert_eq!(read.1.as_deref(), Some("p0"));
    for index in 1..100 {
        assert_not_materialised(&resource, &format!("Bundle.entry[{index}]"));
    }
}

/// `model.lazy.002`: a repeating element can be indexed without materialising
/// the rest.
#[test]
fn indexing_a_repeating_element_materialises_only_that_item() {
    let resource = Resource::from_stored(bytes_of(&patient_with_names(10)));
    let read = resource
        .read("Patient.name[2].family")
        .expect("the read")
        .expect("present");
    assert_eq!(read.1.as_deref(), Some("Family2"));
    for index in (0..10).filter(|index| *index != 2) {
        assert_not_materialised(&resource, &format!("Patient.name[{index}]"));
    }
}

/// `model.lazy.003`: a count does not materialise the values.
#[test]
fn a_count_materialises_nothing() {
    let resource = Resource::from_stored(bytes_of(&patient_with_names(10)));
    assert_eq!(resource.count("Patient.name").expect("counting"), 10);
    let report = resource.report();
    assert!(
        report
            .materialised
            .iter()
            .all(|path| !path.starts_with("Patient.name")),
        "no HumanName may be materialised: {:?}",
        report.materialised
    );
}

/// `model.lazy.004`: materialisation is idempotent and cached.
#[test]
fn a_second_read_scans_no_further_tokens() {
    let resource = Resource::from_stored(bytes_of(&patient_of_size(4 * 1024)));
    let first = resource.read("Patient.birthDate").expect("the first read");
    let after_first = resource.report().tokens_scanned;
    let second = resource.read("Patient.birthDate").expect("the second read");
    let after_second = resource.report().tokens_scanned;
    assert_eq!(first, second, "both reads return equal values");
    assert_eq!(first.and_then(|read| read.1).as_deref(), Some("1974-12-25"));
    assert!(after_first > 0, "the first read scans something");
    assert_eq!(
        after_second, after_first,
        "the second read scans zero additional JSON tokens"
    );
}

/// `model.lazy.005`: a contained resource is materialised only when entered.
#[test]
fn contained_resources_are_not_materialised_by_an_unrelated_read() {
    let resource = Resource::from_stored(bytes_of(PATIENT_WITH_THREE_CONTAINED));
    let read = resource
        .read("Patient.name[0].family")
        .expect("the read")
        .expect("present");
    assert_eq!(read.1.as_deref(), Some("Chalmers"));
    for index in 0..3 {
        assert_not_materialised(&resource, &format!("Patient.contained[{index}]"));
    }
}

/// `model.lazy.006`: narrative is not materialised by an unrelated read.
#[test]
fn a_large_narrative_is_stepped_over_not_parsed() {
    let mut div = String::from("<div xmlns=\\\"http://www.w3.org/1999/xhtml\\\">");
    while div.len() < 50 * 1024 {
        div.push_str("<p>A paragraph of narrative text that nobody asked for.</p>");
    }
    div.push_str("</div>");
    let text = format!(
        r##"{{"resourceType":"Patient","id":"p","text":{{"status":"generated","div":"{div}"}},"gender":"male","birthDate":"1974-12-25"}}"##
    );
    assert!(text.len() > 50 * 1024);
    let resource = Resource::from_stored(bytes_of(&text));
    let read = resource
        .read("Patient.gender")
        .expect("the read")
        .expect("present");
    assert_eq!(read.1.as_deref(), Some("male"));
    assert_not_materialised(&resource, "Patient.text");
    assert!(!resource.report().xhtml_validated);
    assert!(
        resource.report().tokens_scanned < 40,
        "stepping over a 50 KiB div costs one token, not fifty thousand: {}",
        resource.report().tokens_scanned
    );
}

// -------------------------------------------------------------------------
// Writing — the splice that model.perf.004 rests on
// -------------------------------------------------------------------------

/// A metadata update keeps every untouched byte and reports as modified.
#[test]
fn a_metadata_update_splices_and_leaves_the_rest_alone() {
    let text = patient_of_size(4 * 1024);
    let mut resource = Resource::from_stored(bytes_of(&text));
    resource.set("Patient.meta.versionId", "2").expect("set");
    resource
        .set("Patient.meta.lastUpdated", "2026-11-08T12:00:00.000Z")
        .expect("set");
    assert!(resource.is_modified());
    assert!(resource.untouched_bytes().is_none());
    let out = String::from_utf8(resource.to_stored_vec().expect("serialising")).expect("utf-8");
    assert!(out.contains(r#""versionId":"2""#));
    assert!(out.contains(r#""lastUpdated":"2026-11-08T12:00:00.000Z""#));
    assert!(
        out.contains(r#""city":"Townsville""#),
        "every untouched byte survives"
    );
    assert_eq!(
        out.len(),
        text.len(),
        "each replacement is as long as what it replaced, so the document is \
         unchanged in length"
    );
}

/// Canonical serialisation sorts property names and keeps the presented
/// decimal form.
#[test]
fn canonical_serialisation_sorts_property_names() {
    let resource = Resource::from_stored(bytes_of(
        r##"{"resourceType":"Observation","id":"o","status":"final","valueQuantity":{"value":1.58700,"unit":"mmol/L"}}"##,
    ));
    let out = String::from_utf8(resource.to_canonical_vec().expect("canonicalising")).unwrap();
    assert_eq!(
        out,
        r##"{"id":"o","resourceType":"Observation","status":"final","valueQuantity":{"unit":"mmol/L","value":1.58700}}"##
    );
}

// -------------------------------------------------------------------------
// Navigation — model.nav.001 .. 011
// -------------------------------------------------------------------------

const PATIENT_TWO_NAMES: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","active":true,"##,
    r##""name":[{"family":"Alpha","given":["Ann"]},{"family":"Bravo","given":["Bob"]}],"##,
    r##""gender":"male","birthDate":"1974-12-25"}"##
);

const OBSERVATION_VALUE_QUANTITY: &str = concat!(
    r##"{"resourceType":"Observation","id":"o","status":"final","##,
    r##""code":{"text":"Glucose"},"##,
    r##""valueQuantity":{"value":6.3,"unit":"mmol/L"}}"##
);

const PATIENT_WITH_EXTENSIONS: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""extension":[{"url":"https://example.org/a","valueString":"A"}],"##,
    r##""modifierExtension":[{"url":"https://example.org/b","valueString":"B"}],"##,
    r##""active":true,"##,
    r##""name":[{"family":"Alpha"},{"family":"Bravo"}],"##,
    r##""gender":"male"}"##
);

const PATIENT_EXTENSION_ONLY_BIRTHDATE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""_birthDate":{"extension":[{"url":"https://example.org/birth-time","##,
    r##""valueString":"08:00"}]}}"##
);

fn node_at(doc: &str, path: &str) -> super::ResourceNode {
    let resource = Resource::from_stored(bytes_of(doc));
    let stored = Arc::clone(resource.stored().expect("a stored resource"));
    super::ResourceNode::at(stored, path)
        .expect("navigation must succeed")
        .unwrap_or_else(|| panic!("{path} must be present"))
}

/// `model.nav.001`: a node reports its FHIR type name.
#[test]
fn nodes_report_their_type_names() {
    assert_eq!(node_at(PATIENT_TWO_NAMES, "Patient").type_name(), "Patient");
    assert_eq!(
        node_at(PATIENT_TWO_NAMES, "Patient.name[0]").type_name(),
        "HumanName"
    );
    assert_eq!(
        node_at(PATIENT_TWO_NAMES, "Patient.name[0].family").type_name(),
        "string"
    );
}

/// `model.nav.002`: children are returned by element name in document order.
#[test]
fn children_come_back_by_name_in_document_order() {
    let patient = node_at(PATIENT_TWO_NAMES, "Patient");
    let names = patient.children("name").expect("the name children");
    assert_eq!(names.len(), 2);
    for value in &names {
        let node = value.as_node().expect("a node");
        assert_eq!(node.type_name(), "HumanName");
        assert!(node.primitive().is_none(), "a HumanName is not a primitive");
    }
    let families: Vec<Option<String>> = (0..2)
        .map(|index| {
            node_at(PATIENT_TWO_NAMES, &format!("Patient.name[{index}]"))
                .children("family")
                .expect("family")
                .first()
                .and_then(|value| value.as_node().and_then(|node| node.primitive()))
                .map(|primitive| primitive.lexical)
        })
        .collect();
    assert_eq!(
        families,
        vec![Some("Alpha".to_string()), Some("Bravo".to_string())]
    );
}

/// `model.nav.003` and `model.nav.004`: a choice element is reached by its base
/// name, and the typed spelling yields nothing.
#[test]
fn a_choice_element_answers_to_its_base_name_only() {
    let observation = node_at(OBSERVATION_VALUE_QUANTITY, "Observation");
    let by_base = observation.children("value").expect("value");
    assert_eq!(by_base.len(), 1);
    assert_eq!(
        by_base[0].as_node().expect("a node").type_name(),
        "Quantity"
    );
    let by_typed = observation
        .children("valueQuantity")
        .expect("valueQuantity must not error");
    assert!(
        by_typed.is_empty(),
        "the typed spelling yields nothing: {by_typed:?}"
    );
}

/// `model.nav.005`: an absent element yields an empty child list, not an error.
#[test]
fn an_absent_element_is_an_empty_list() {
    let patient = node_at(
        r##"{"resourceType":"Patient","id":"p","gender":"male"}"##,
        "Patient",
    );
    let children = patient
        .children("birthDate")
        .expect("an absent element must not be an error");
    assert!(children.is_empty());
}

/// `model.nav.006`: all children can be enumerated with their names.
#[test]
fn all_children_carry_their_element_names() {
    let patient = node_at(PATIENT_WITH_EXTENSIONS, "Patient");
    let children = patient.all_children().expect("every child");
    let names: Vec<&str> = children.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "id",
            "extension",
            "modifierExtension",
            "active",
            "name",
            "name",
            "gender"
        ]
    );
    assert!(!names.contains(&"resourceType"));
    assert_eq!(
        children
            .iter()
            .find(|(name, _)| name == "extension")
            .and_then(|(_, value)| value.as_node())
            .map(|node| node.type_name().to_string()),
        Some("Extension".to_string())
    );
}

/// `model.nav.007`: returned values are owned and outlive the node they came
/// from.
#[test]
fn children_outlive_the_node_they_came_from() {
    let children = {
        let parent = node_at(PATIENT_TWO_NAMES, "Patient.name[0]");
        parent.children("family").expect("family")
        // `parent` is dropped here.
    };
    assert_eq!(children.len(), 1);
    let node = children[0].as_node().expect("a node");
    assert_eq!(node.type_name(), "string");
    assert_eq!(
        node.primitive().map(|primitive| primitive.lexical),
        Some("Alpha".to_string())
    );
}

/// `model.nav.008`: a node reports whether it is a resource.
#[test]
fn nodes_report_whether_they_are_resources() {
    let doc = concat!(
        r##"{"resourceType":"Patient","id":"p","##,
        r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"}],"##,
        r##""name":[{"family":"Chalmers"}]}"##
    );
    assert!(node_at(doc, "Patient").is_resource());
    assert!(node_at(doc, "Patient.contained[0]").is_resource());
    assert!(!node_at(doc, "Patient.name[0]").is_resource());
    assert_eq!(
        node_at(doc, "Patient.contained[0]").type_name(),
        "Organization",
        "and the contained resource's concrete type comes from its resourceType"
    );
}

/// `model.nav.009`: a primitive node reports its value or reports having none.
#[test]
fn a_value_less_element_still_exists() {
    let with_value = node_at(PATIENT_TWO_NAMES, "Patient.birthDate");
    assert_eq!(
        with_value.primitive().map(|primitive| primitive.lexical),
        Some("1974-12-25".to_string())
    );
    let extension_only = node_at(PATIENT_EXTENSION_ONLY_BIRTHDATE, "Patient.birthDate");
    assert!(
        extension_only.primitive().is_none(),
        "an extension-only birthDate is an element with no value"
    );
    assert!(
        extension_only.sibling_span().is_some(),
        "and it exists because its _birthDate sibling does"
    );
}

/// `model.nav.010`: the model answers type-hierarchy questions.
#[test]
fn the_model_answers_the_hierarchy() {
    let model = R4Model::new(R4);
    assert!(model.derives_from("code", "string"));
    assert!(model.derives_from("Age", "Quantity"));
    assert_eq!(model.base_type("code"), Some("string"));
    assert_eq!(model.base_type("Age"), Some("Quantity"));
    assert!(!model.derives_from("Duration", "Period"));
}

/// `model.nav.011`: the model answers element-type questions.
#[test]
fn the_model_answers_the_choice_set() {
    let model = R4Model::new(R4);
    let mut declared = model
        .element_types("Observation", "value")
        .expect("Observation.value[x] must report its declared types");
    let mut expected: Vec<String> = [
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
    ]
    .iter()
    .map(|name| name.to_string())
    .collect();
    declared.sort();
    expected.sort();
    assert_eq!(declared, expected);
    assert!(!declared.iter().any(|name| name == "Duration"));
}

/// `model.nav.012`, the model half: the same navigation over JSON that the
/// harness's XML-backed node performs over the equivalent document.
#[test]
fn navigation_matches_the_portable_projection() {
    const DOC: &str = concat!(
        r##"{"resourceType":"Patient","id":"p","##,
        r##""name":[{"family":"Chalmers","given":["Peter","James"]},{"family":"Windsor"}],"##,
        r##""gender":"male","birthDate":"1974-12-25"}"##
    );
    let patient = node_at(DOC, "Patient");
    let portable: Vec<(String, Option<String>)> = patient
        .all_children()
        .expect("every child")
        .into_iter()
        .map(|(name, value)| {
            let primitive = value
                .as_node()
                .and_then(|node| node.primitive())
                .map(|primitive| primitive.lexical);
            (name, primitive)
        })
        .collect();
    assert_eq!(
        portable,
        vec![
            ("id".to_string(), Some("p".to_string())),
            ("name".to_string(), None),
            ("name".to_string(), None),
            ("gender".to_string(), Some("male".to_string())),
            ("birthDate".to_string(), Some("1974-12-25".to_string())),
        ]
    );
    for (path, expected) in [
        ("Patient.gender", "male"),
        ("Patient.birthDate", "1974-12-25"),
        ("Patient.name[0].given[1]", "James"),
        ("Patient.name[1].family", "Windsor"),
    ] {
        assert_eq!(
            node_at(DOC, path)
                .primitive()
                .map(|primitive| primitive.lexical),
            Some(expected.to_string()),
            "{path}"
        );
    }
}

// -------------------------------------------------------------------------
// The element-order table itself
// -------------------------------------------------------------------------

/// The order `model.bytes.005` names, asserted directly against the table so a
/// regression in it is reported as what it is.
#[test]
fn the_patient_element_order_is_the_snapshot_order() {
    let order: Vec<&str> = super::built::ordered_elements(&R4, "Patient")
        .iter()
        .map(|def| def.name)
        .collect();
    assert_eq!(
        &order[..14],
        &[
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
        ]
    );
}
