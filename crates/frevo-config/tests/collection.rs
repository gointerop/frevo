//! config.collection and config.object: collections, maps and nested objects across sources.

mod common;

use common::{parity, parity_masked};
use serde_json::{Value, json};

// covers: config.collection.001
#[test]
fn the_highest_source_replaces_a_whole_list() {
    parity("config.collection.001").expect("hapi.fhir.logical-urls", json!(["http://x/*"]));
}

// covers: config.collection.002
#[test]
fn an_index_gap_fails_to_start() {
    // Finding O-8 (tasks/blockers.md): HAPI's description line carries a Java identity hash
    // (`Bindable@e25951c`) that differs between runs; it is masked on both sides here and
    // nowhere else until the spec delta says how config.oracle.003 treats it.
    let r = parity_masked(
        "config.collection.002",
        &[(r"Bindable@[0-9a-f]+", "Bindable@<hash>")],
    );
    r.expect_failure()
        .expect_line("    Property: hapi.fhir.supportedresourcetypes[1]")
        .expect_line("    Value: \"Patient\"")
        .expect_line(
            "    Reason: The elements [hapi.fhir.supportedresourcetypes[1]] were left unbound.",
        );
}

// covers: config.collection.003
#[test]
fn maps_merge_entries_across_sources() {
    let r = parity("config.collection.003");
    let mut keys: Vec<String> = r
        .get("hapi.fhir.tester")
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(keys, ["home", "other"]);
    r.expect(
        "hapi.fhir.tester.other.refuse-to-fetch-third-party-urls",
        json!(true),
    )
    .expect("hapi.fhir.tester.other.fhir-version", json!("R4"))
    .expect("hapi.fhir.tester.other.server-address", Value::Null);
}

// covers: config.collection.004
#[test]
fn map_keys_are_kept_verbatim() {
    let r = parity("config.collection.004");
    r.expect(
        "hapi.fhir.implementation-guides.ips_1_0_0.name",
        json!("ips"),
    )
    .expect("hapi.fhir.implementation-guides.Swiss.name", json!("swiss"))
    .expect("hapi.fhir.cr.cql.namespaces.Mixed_Key", json!("http://m"));
}

// covers: config.collection.005
#[test]
fn a_string_map_takes_dotted_keys_whole() {
    parity("config.collection.005").expect(
        "hapi.fhir.cr.cql.namespaces",
        json!({"hl7.fhir.uv.cql": "http://hl7.org/fhir/uv/cql", "hl7.fhir.us.cql": "http://hl7.org/fhir/us/cql"}),
    );
}

// covers: config.collection.006
#[test]
fn collections_with_no_setter_still_bind() {
    parity("config.collection.006")
        .expect("hapi.fhir.custom-interceptor-classes", json!(["a.B"]))
        .expect("hapi.fhir.custom-provider-classes", json!(["c.D"]))
        .expect("hapi.fhir.local-base-urls", json!(["http://l"]))
        .expect(
            "hapi.fhir.auto-version-reference-at-paths",
            json!(["Device.patient", "Observation.subject"]),
        );
}
