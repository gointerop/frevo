//! config.properties: `.properties` files with OriginTrackedPropertiesLoader syntax.

mod common;

use common::parity;
use serde_json::json;

// covers: config.properties.001
#[test]
fn the_three_separators() {
    parity("config.properties.001")
        .expect("hapi.fhir.server-address", json!("http://a"))
        .expect("hapi.fhir.mdm-rules-json-location", json!("x"))
        .expect("hapi.fhir.custom-content-path", json!("abc"));
}

// covers: config.properties.002
#[test]
fn comments_contribute_nothing() {
    parity("config.properties.002")
        .expect("hapi.fhir.default-page-size", json!(20))
        .expect("hapi.fhir.max-page-size", json!(2147483647))
        .expect("hapi.fhir.server-address", serde_json::Value::Null)
        .expect("hapi.fhir.expunge-thread-count", json!(3));
}

// covers: config.properties.003
#[test]
fn trailing_whitespace_is_kept_leading_whitespace_is_not() {
    parity("config.properties.003").expect("hapi.fhir.server-address", json!("http://a   "));
}

// covers: config.properties.004
#[test]
fn unicode_escapes_character_escapes_and_line_continuation() {
    parity("config.properties.004")
        .expect("hapi.fhir.custom-content-path", json!("Abc"))
        .expect("hapi.fhir.mdm-rules-json-location", json!("ab"))
        .expect("hapi.fhir.server-address", json!("x\ty"));
}

// covers: config.properties.005
#[test]
fn hash_dash_dash_dash_separates_documents() {
    parity("config.properties.005").expect("hapi.fhir.default-page-size", json!(5));
    parity("config.properties.005-b").expect("hapi.fhir.default-page-size", json!(6));
}

// covers: config.properties.006
#[test]
fn every_value_is_text() {
    parity("config.properties.006").expect("hapi.fhir.graphql-enabled", json!(true));
    // unlike the YAML integer 1 of config.coerce.004
    parity("config.coerce.004").expect_failure();
}

// covers: config.properties.007
#[test]
fn an_empty_value_is_the_empty_string() {
    parity("config.properties.007").expect("hapi.fhir.app-content-path", json!(""));
}
