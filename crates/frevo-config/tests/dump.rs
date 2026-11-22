//! config.dump: the effective-configuration document of `frevo-config effective`.
//!
//! config.dump.002 and .004 have a HAPI counterpart (the document is compared with the
//! recorded HAPI document); the shape assertions below state the spec text directly.

mod common;

use common::{PREFIXES, parity};
use serde_json::{Value, json};

// covers: config.dump.001
#[test]
fn a_valid_configuration_prints_one_document_and_exits_0() {
    let r = parity("config.dump.001");
    assert_eq!(r.exit, 0);
    let doc = r.doc().as_object().expect("one JSON object");
    let mut members: Vec<&str> = doc.keys().map(String::as_str).collect();
    members.sort();
    assert_eq!(members, ["gates", "profiles", "properties", "reads"]);
    let profiles = doc["profiles"].as_object().expect("profiles is an object");
    let mut pm: Vec<&str> = profiles.keys().map(String::as_str).collect();
    pm.sort();
    assert_eq!(pm, ["active", "default"]);
    assert!(profiles["active"].is_array() && profiles["default"].is_array());
    assert!(
        profiles["active"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
}

// covers: config.dump.002
#[test]
fn properties_are_reported_under_the_nine_bean_prefixes() {
    let r = parity("config.dump.001");
    let props = r.doc()["properties"].as_object().expect("properties");
    let mut got: Vec<&str> = props.keys().map(String::as_str).collect();
    got.sort();
    let mut want = PREFIXES.to_vec();
    want.sort();
    assert_eq!(got, want);

    // Where two classes bind the same key, every class reports the same value; checked on
    // the all-keys case, where every such key has a non-default value.
    let all = parity("all-keys.yaml");
    let d = all.doc();
    let p = |prefix: &str| d["properties"][prefix].clone();
    assert_eq!(p("hapi.fhir.cr")["cql"], p("hapi.fhir.cr.cql"));
    assert_eq!(
        p("hapi.fhir.cr")["cql"]["compiler"],
        p("hapi.fhir.cr.cql.compiler")
    );
    assert_eq!(
        p("hapi.fhir.cr.cql")["compiler"],
        p("hapi.fhir.cr.cql.compiler")
    );
    assert_eq!(
        p("hapi.fhir.cr.cql")["runtime"],
        p("hapi.fhir.cr.cql.runtime")
    );
    assert_eq!(
        p("hapi.fhir.cr.cql")["terminology"],
        p("hapi.fhir.cr.cql.terminology")
    );
    assert_eq!(p("hapi.fhir.cr.cql")["data"], p("hapi.fhir.cr.cql.data"));
    assert_eq!(p("hapi.fhir.cr")["care-gaps"], p("hapi.fhir.cr.caregaps"));
    // the scenario's example: validate-units, bound by CrProperties, CqlProperties and
    // CqlCompilerProperties, set to its non-default false by all-keys.yaml
    all.expect("hapi.fhir.cr.cql.compiler.validate-units", json!(false));
    assert_eq!(
        p("hapi.fhir.cr")["cql"]["compiler"]["validate-units"],
        json!(false)
    );
    assert_eq!(
        p("hapi.fhir.cr.cql")["compiler"]["validate-units"],
        json!(false)
    );
}

// covers: config.dump.003
#[test]
fn property_paths_are_canonical_whatever_spelling_was_written() {
    let r = parity("config.dump.003");
    r.expect("hapi.fhir.default-page-size", json!(6));
    let text = serde_json::to_string(r.doc()).unwrap();
    assert!(
        !text.contains("defaultPageSize"),
        "a non-canonical path appears"
    );
    assert!(
        !text.contains("default_page_size"),
        "a non-canonical path appears"
    );
}

// covers: config.dump.004
#[test]
fn values_are_typed_as_the_starters_property_types() {
    let r = parity("config.dump.001");
    r.expect("hapi.fhir.fhir-version", json!("R4"))
        .expect("hapi.fhir.max-page-size", json!(2147483647))
        .expect("hapi.fhir.retain-cached-searches-mins", json!(60))
        .expect("hapi.fhir.default-page-size", json!(20))
        .expect("hapi.fhir.partitioning", Value::Null)
        .expect("hapi.fhir.server-address", Value::Null);
    assert!(r.get("hapi.fhir.openapi-enabled").is_boolean());
    assert!(r.get("hapi.fhir.mdm-rules-json-location").is_string());
    assert!(r.get("hapi.fhir.tester").is_object(), "a Map is an object");
    assert!(
        r.get("hapi.fhir.search-prefetch-thresholds").is_array(),
        "a List is an array"
    );
    // A Set is an array sorted by code point.
    let urls = common::strings(&r.get("hapi.fhir.logical-urls"));
    let mut sorted = urls.clone();
    sorted.sort();
    assert_eq!(urls, sorted);
    assert_eq!(urls.len(), 8);
}

// covers: config.dump.005
#[test]
fn the_email_password_is_redacted() {
    parity("config.dump.005").expect("hapi.fhir.subscription.email.password", json!("******"));
    parity("config.dump.005-b").expect("hapi.fhir.subscription.email.password", json!(""));
}

// covers: config.dump.006
#[test]
fn a_failure_prints_no_document() {
    let r = parity("config.dump.006");
    r.expect_failure();
    r.expect_analysis(
        "Failed to bind properties under 'hapi.fhir.default-page-size' to java.lang.Integer:",
        &[
            "    Property: hapi.fhir.default-page-size",
            "    Value: \"abc\"",
        ],
    );
    assert_eq!(r.stderr, "", "nothing is written to standard error");
}
