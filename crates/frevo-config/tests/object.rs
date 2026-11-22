//! config.object: when nested properties objects exist.

mod common;

use common::parity;
use serde_json::{Value, json};

// covers: config.object.001
#[test]
fn an_object_with_nothing_bound_beneath_it_stays_null() {
    parity("config.object.001")
        .expect("hapi.fhir.partitioning", Value::Null)
        .expect("hapi.fhir.subscription.email", Value::Null);
    parity("config.object.001-b").expect("hapi.fhir.partitioning", Value::Null);
}

// covers: config.object.002
#[test]
fn any_bound_value_beneath_creates_the_object_with_its_defaults() {
    parity("config.object.002").expect(
        "hapi.fhir.subscription.email",
        json!({"username": "", "port": 25, "auth": false, "start-tls-enable": false, "start-tls-required": false,
               "quit-wait": false, "from": null, "host": null, "password": null}),
    );
}

// covers: config.object.003
#[test]
fn a_scalar_where_an_object_is_expected_fails_when_nothing_is_beneath() {
    parity("config.object.003").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.partitioning' to ca.uhn.fhir.jpa.starter.AppProperties$Partitioning:",
        &["    Reason: org.springframework.core.convert.ConverterNotFoundException: No converter found capable of converting from type [java.lang.String] to type [ca.uhn.fhir.jpa.starter.AppProperties$Partitioning]"],
    );
}

// covers: config.object.004
#[test]
fn a_scalar_where_an_object_is_expected_is_ignored_when_keys_exist_beneath() {
    parity("config.object.004").expect("hapi.fhir.cors.allowed-origin", json!(["*"]));
}

// covers: config.object.005
#[test]
fn which_objects_exist_by_default() {
    let r = parity("config.object.005");
    for p in [
        "hapi.fhir.validation",
        "hapi.fhir.logger",
        "hapi.fhir.subscription",
        "hapi.fhir.client",
    ] {
        assert!(
            r.get(p).is_object(),
            "{p} is an object with its class defaults"
        );
    }
    for p in [
        "hapi.fhir.cors",
        "hapi.fhir.partitioning",
        "hapi.fhir.elasticsearch",
        "hapi.fhir.subscription.email",
        "hapi.fhir.tester",
        "hapi.fhir.implementation-guides",
        "hapi.fhir.remote-terminology-service",
        "hapi.fhir.allowed-bundle-types",
    ] {
        r.expect(p, Value::Null);
    }
    let packaged = parity("config.source.002");
    assert!(
        packaged.get("hapi.fhir.cors").is_object() && packaged.get("hapi.fhir.tester").is_object()
    );
}
