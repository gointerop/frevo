//! config.env: environment variables through Spring Boot's two name mappers.

mod common;

use common::parity;
use serde_json::{Value, json};

// covers: config.env.001
#[test]
fn the_upper_case_underscore_form() {
    parity("config.env.001").expect("hapi.fhir.default-page-size", json!(11));
}

// covers: config.env.002
#[test]
fn the_upper_case_form_with_the_separators_removed() {
    parity("config.env.002").expect("hapi.fhir.default-page-size", json!(12));
}

// covers: config.env.003
#[test]
fn dotted_variable_names_bind_with_relaxed_matching() {
    parity("config.env.003").expect("hapi.fhir.default-page-size", json!(13));
    parity("config.env.003-b").expect("hapi.fhir.default-page-size", json!(14));
    parity("config.env.003-c").expect("hapi.fhir.default-page-size", json!(17));
}

// covers: config.env.004
#[test]
fn forms_that_do_not_bind() {
    parity("config.env.004").expect("hapi.fhir.default-page-size", json!(20));
    // Finding O-2 (tasks/blockers.md): on Linux HAPI *does* bind the lower-case
    // hapi_fhir_default_page_size (the record shows 15), contradicting the scenario's Linux
    // clause; the test follows the record.
    parity("config.env.004-b").expect("hapi.fhir.default-page-size", json!(15));
    // The mixed-case form does not bind on Linux. On Windows variable names are
    // case-insensitive, so the Linux record cannot judge this variant there.
    #[cfg(unix)]
    parity("config.env.004-c").expect("hapi.fhir.default-page-size", json!(20));
}

// covers: config.env.005
#[test]
fn nested_properties_and_camel_case_upstream_names() {
    parity("config.env.005").expect("hapi.fhir.cors.allow-credentials", json!(true));
}

// covers: config.env.006
#[test]
fn list_elements_by_index() {
    parity("config.env.006").expect(
        "hapi.fhir.supported-resource-types",
        json!(["Patient", "Obs"]),
    );
    parity("config.env.006-b").expect("hapi.fhir.supported-resource-types", json!(["Patient"]));
    parity("config.env.006-c").expect("hapi.fhir.supported-resource-types", json!([]));
}

// covers: config.env.007
#[test]
fn an_indexed_variable_replaces_the_whole_collection() {
    parity("config.env.007").expect("hapi.fhir.logical-urls", json!(["x"]));
}

// covers: config.env.008
#[test]
fn comma_separated_values() {
    parity("config.env.008").expect("hapi.fhir.search-prefetch-thresholds", json!([1, 2]));
}

// covers: config.env.009
#[test]
fn map_entries_through_the_upper_case_form_are_lower_cased() {
    parity("config.env.009")
        .expect(
            "hapi.fhir.implementation-guides.swiss.name",
            json!("swiss.x"),
        )
        .expect("hapi.fhir.cr.cql.namespaces.foo", json!("http://foo"));
    parity("config.env.009-b").expect(
        "hapi.fhir.implementation-guides.someIg.name",
        json!("com.org.something"),
    );
}

// covers: config.env.010
#[test]
fn map_entry_fields_merge_with_lower_sources() {
    parity("config.env.010")
        .expect("hapi.fhir.tester.home.name", json!("EnvTester"))
        .expect(
            "hapi.fhir.tester.home.server-address",
            json!("http://localhost:8080/fhir"),
        );
}

// covers: config.env.011
#[test]
fn a_map_key_with_digits_or_underscores_cannot_be_addressed_in_upper_case_form() {
    parity("config.env.011").expect("hapi.fhir.implementation-guides", Value::Null);
}

// covers: config.env.012
#[test]
fn variable_values_are_text() {
    parity("config.env.012").expect("hapi.fhir.graphql-enabled", json!(true));
}

// covers: config.env.013
#[test]
fn the_oracles_own_environment_binds() {
    let r = parity("config.env.013");
    r.expect(
        "hapi.fhir.server-address",
        json!("http://localhost:8090/fhir"),
    )
    .expect("hapi.fhir.allow-multiple-delete", json!(true))
    .expect("hapi.fhir.allow-cascading-deletes", json!(true))
    .expect("hapi.fhir.allow-external-references", json!(true))
    .expect(
        "hapi.fhir.auto-create-placeholder-reference-targets",
        json!(true),
    )
    .expect("hapi.fhir.delete-expunge-enabled", json!(true))
    .expect("hapi.fhir.graphql-enabled", json!(true))
    .expect("hapi.fhir.bulk-export-enabled", json!(true))
    .expect("hapi.fhir.bulk-import-enabled", json!(true))
    .expect("hapi.fhir.narrative-enabled", json!(false))
    .expect("hapi.fhir.dao-scheduling-enabled", json!(false))
    .expect("hapi.fhir.cr.enabled", json!(false))
    .expect("hapi.fhir.mdm-enabled", json!(false));
    assert_eq!(
        r.stderr,
        "Picked up JAVA_TOOL_OPTIONS: -Xmx3g -XX:+UseG1GC\n"
    );
}

// covers: config.env.014
#[test]
fn a_variable_named_like_an_argument() {
    // The scenario defers to the recorded HAPI run. Finding O-3: HAPI applies the variable
    // `--spring.config.location` as the location (the packaged files are not loaded, no
    // version is set, and it fails with config.gate.005's NullPointerException).
    parity("config.env.014").expect_failure();
}
