//! config.unknown: unknown, unbound and setter-less keys.

mod common;

use common::parity;
use serde_json::{Value, json};

// covers: config.unknown.001
#[test]
fn unknown_keys_are_ignored() {
    let r = parity("config.unknown.001");
    r.expect("hapi.fhir.default-page-size", json!(3));
    let mut doc = r.doc().clone();
    let mut base = parity("config.source.002").doc().clone();
    doc["properties"]["hapi.fhir"]["default-page-size"] = Value::Null;
    base["properties"]["hapi.fhir"]["default-page-size"] = Value::Null;
    assert_eq!(doc, base, "nothing else differs from config.source.002");
    // HAPI_FHIR_ALLOW_PLACEHOLDER_REFERENCES of config.env.013 names no property
    let text = serde_json::to_string(parity("config.env.013").doc()).unwrap();
    assert!(!text.contains("allow-placeholder-references"));
}

// covers: config.unknown.002
#[test]
fn the_packaged_search_coord_keys_bind_to_nothing() {
    let packaged = parity("config.source.002");
    let text = serde_json::to_string(packaged.doc()).unwrap();
    assert!(
        !text.contains("search-coord"),
        "no property carries the search-coord keys"
    );
    let r = parity("config.unknown.002");
    assert_eq!(
        r.doc(),
        packaged.doc(),
        "setting them to abc changes nothing"
    );
}

// covers: config.unknown.003
#[test]
fn keys_documented_at_the_wrong_level_bind_to_nothing() {
    parity("config.unknown.003")
        .expect("hapi.fhir.partitioning", Value::Null)
        .expect(
            "hapi.fhir.install-additional-resources-from-ig-folders",
            json!([]),
        );
    parity("config.unknown.003-b")
        .expect(
            "hapi.fhir.partitioning.database-partition-mode-enabled",
            json!(true),
        )
        .expect(
            "hapi.fhir.partitioning.patient-id-partitioning-mode",
            json!(true),
        )
        .expect(
            "hapi.fhir.partitioning.request-tenant-partitioning-mode",
            json!(false),
        );
}

// covers: config.unknown.004
#[test]
fn keys_outside_hapi_fhir_are_kept_for_reads() {
    let r = parity("config.unknown.004");
    let props = r.doc()["properties"].as_object().unwrap();
    assert!(
        props.keys().all(|k| k.starts_with("hapi.fhir")),
        "spring.*, server.*, management.* are not properties"
    );
    r.expect_read("spring.ai.mcp.server.enabled", json!("false"))
        .expect_gate("mcp", false);
    // spring.elasticsearch.* is the one exception: bound and type-checked (config.gate.024)
    parity("config.gate.024-g").expect_failure();
}

// covers: config.unknown.005
#[test]
fn a_setter_less_property_fails_when_set_to_a_new_value() {
    parity("config.unknown.005").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.cr.cql.runtime' to ca.uhn.fhir.jpa.starter.cr.CqlRuntimeProperties:",
        &[
            "    Property: hapi.fhir.cr.cql.runtime.enable-validation",
            "    Value: \"true\"",
            "    Reason: java.lang.IllegalStateException: No setter found for property: enable-validation",
        ],
    );
    // DEBT-0026: its current value starts
    parity("config.unknown.005-b")
        .expect("hapi.fhir.cr.cql.runtime.enable-validation", json!(false));
}

// covers: config.unknown.006
#[test]
fn the_getter_only_remote_terminology_map() {
    parity("config.unknown.006").expect_analysis(
        "Failed to bind properties under 'hapi.fhir' to ca.uhn.fhir.jpa.starter.AppProperties:",
        &["    Reason: java.lang.IllegalStateException: No setter found for property: remote-terminology-services-map"],
    );
    parity("config.unknown.006-b").expect(
        "hapi.fhir.remote-terminology-service.a.url",
        json!("http://x"),
    );
}

// covers: config.unknown.007
#[test]
fn a_set_key_this_build_does_not_act_on_is_accepted_silently() {
    // No HAPI counterpart for "silently" beyond the record's empty standard error; the
    // scenario's text is asserted directly.
    let r = parity("config.unknown.007");
    r.expect("hapi.fhir.bulk-export-enabled", json!(true));
    let printed: serde_json::Value =
        serde_json::from_str(&r.stdout).expect("standard output is the document only");
    assert_eq!(&printed, r.doc());
    assert_eq!(r.stderr, "", "nothing about the key on standard error");
    assert!(
        !r.stdout.to_lowercase().contains("not implemented")
            && !r.stdout.to_lowercase().contains("ignored")
    );
}

// covers: config.unknown.008
#[test]
fn the_setter_less_derived_properties_fail_when_set() {
    parity("config.unknown.008").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.cr.cql.data' to ca.uhn.fhir.jpa.starter.cr.CqlData:",
        &["    Reason: java.lang.IllegalStateException: No setter found for property: retrieve-settings"],
    );
    parity("config.unknown.008-b").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.cr.cql' to ca.uhn.fhir.jpa.starter.cr.CqlProperties:",
        &["    Reason: java.lang.IllegalStateException: No setter found for property: retrieve-settings"],
    );
    parity("config.unknown.008-c").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.cr.cql.terminology' to ca.uhn.fhir.jpa.starter.cr.CqlTerminologyProperties:",
        &["    Reason: java.lang.IllegalStateException: No setter found for property: terminology-settings"],
    );
}
