//! config.args: command-line arguments and SPRING_APPLICATION_JSON.

mod common;

use common::parity;
use serde_json::json;

// covers: config.args.001
#[test]
fn arguments_override_every_other_source() {
    parity("config.args.001").expect("hapi.fhir.default-page-size", json!(4));
}

// covers: config.args.002
#[test]
fn argument_names_are_relaxed() {
    parity("config.args.002").expect("hapi.fhir.reindex-thread-count", json!(4));
}

// covers: config.args.003
#[test]
fn other_arguments_are_ignored() {
    let r = parity("config.args.003");
    assert_eq!(r.doc(), parity("config.source.002").doc());
}

// covers: config.args.004
#[test]
fn an_argument_with_no_value_is_the_empty_string() {
    parity("config.args.004").expect("hapi.fhir.server-address", json!(""));
}

// covers: config.args.005
#[test]
fn a_repeated_argument_joins_its_values_with_commas() {
    parity("config.args.005").expect("hapi.fhir.supported-resource-types", json!(["A", "B"]));
}

// covers: config.args.006
#[test]
fn spring_application_json_is_flattened_like_yaml() {
    parity("config.args.006").expect("hapi.fhir.server-address", json!("json"));
    parity("config.args.006-b")
        .expect("hapi.fhir.default-page-size", json!(7))
        .expect("hapi.fhir.graphql-enabled", json!(true))
        .expect("hapi.fhir.mdm-rules-json-location", json!("1.1"))
        .expect("hapi.fhir.server-address", json!("true"));
}

// covers: config.args.007
#[test]
fn invalid_spring_application_json_fails_to_start() {
    // Finding O-4 (tasks/blockers.md): in the image Spring's JsonParserFactory picks
    // Jackson, so the cause is Jackson's, not the NullPointerException the scenario quotes.
    parity("config.args.007").expect_run_failed(&[
        "org.springframework.boot.json.JsonParseException: Cannot parse JSON",
        "Caused by: com.fasterxml.jackson.core.JsonParseException: Unexpected character ('b' (code 98)): was expecting double-quote to start field name",
    ]);
}

// covers: config.args.008
#[test]
fn spring_application_json_as_an_argument() {
    parity("config.args.008").expect("hapi.fhir.default-page-size", json!(7));
    parity("config.args.008-b").expect("hapi.fhir.default-page-size", json!(7));
    parity("config.args.008-c").expect("hapi.fhir.default-page-size", json!(4));
    parity("config.args.008-d").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.default-page-size' to java.lang.Integer:",
        &["    Origin: \"spring.application.json\" from property source \"commandLineArgs\""],
    );
}

// covers: config.args.009
#[test]
fn spring_application_json_as_a_system_property() {
    parity("config.args.009").expect("hapi.fhir.default-page-size", json!(8));
    parity("config.args.009-b").expect("hapi.fhir.default-page-size", json!(8));
    parity("config.args.009-c").expect("hapi.fhir.default-page-size", json!(7));
    parity("config.args.009-d").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.default-page-size' to java.lang.Integer:",
        &["    Origin: \"spring.application.json\" from property source \"systemProperties\""],
    );
    // Without the single quotes the JVM strips the double quotes. Finding O-4: the cause
    // is Jackson's message.
    parity("config.args.009-e").expect_run_failed(&[
        "org.springframework.boot.json.JsonParseException: Cannot parse JSON",
        "Caused by: com.fasterxml.jackson.core.JsonParseException: Unexpected character ('h' (code 104)): was expecting double-quote to start field name",
    ]);
}

// covers: config.args.010
#[test]
fn an_empty_argument_name_is_invalid() {
    parity("config.args.010")
        .expect_run_failed(&["java.lang.IllegalArgumentException: Invalid argument syntax: --=x"]);
}

// covers: config.args.011
#[test]
fn double_dash_ends_option_parsing() {
    parity("config.args.011").expect("hapi.fhir.default-page-size", json!(20));
    parity("config.args.011-b")
        .expect("hapi.fhir.default-page-size", json!(3))
        .expect("hapi.fhir.max-page-size", json!(2147483647));
}

// covers: config.args.012
#[test]
fn the_first_equals_sign_splits_name_from_value() {
    parity("config.args.012").expect("hapi.fhir.server-address", json!("a=b"));
}
