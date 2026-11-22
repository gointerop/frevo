//! config.coerce: conversion of configured values to the starter's property types.

mod common;

use common::parity;
use serde_json::{Value, json};

fn bind_reason(id: &str, property: &str, target: &str, value: &str, reason: &str) {
    parity(id).expect_analysis(
        &format!("Failed to bind properties under '{property}' to {target}:"),
        &[
            &format!("    Property: {property}"),
            &format!("    Value: \"{value}\""),
            &format!("    Reason: {reason}"),
        ],
    );
}

// covers: config.coerce.001
#[test]
fn yaml_booleans_bind_to_boolean() {
    for id in [
        "config.coerce.001",
        "config.coerce.001-b",
        "config.coerce.001-c",
        "config.coerce.001-d",
    ] {
        parity(id).expect("hapi.fhir.graphql-enabled", json!(true));
    }
}

// covers: config.coerce.002
#[test]
fn boolean_text() {
    for s in ["", "-b", "-c", "-d", "-e"] {
        parity(&format!("config.coerce.002{s}")).expect("hapi.fhir.graphql-enabled", json!(true));
    }
    for s in ["-f", "-g", "-h", "-i"] {
        parity(&format!("config.coerce.002{s}")).expect("hapi.fhir.graphql-enabled", json!(false));
    }
}

// covers: config.coerce.003
#[test]
fn other_boolean_text_fails() {
    for (id, v) in [("config.coerce.003", "Y"), ("config.coerce.003-b", "2")] {
        bind_reason(
            id,
            "hapi.fhir.graphql-enabled",
            "java.lang.Boolean",
            v,
            &format!(
                "failed to convert java.lang.String to java.lang.Boolean (caused by java.lang.IllegalArgumentException: Invalid boolean value '{v}')"
            ),
        );
    }
}

// covers: config.coerce.004
#[test]
fn a_yaml_integer_does_not_convert_to_boolean() {
    bind_reason(
        "config.coerce.004",
        "hapi.fhir.graphql-enabled",
        "java.lang.Boolean",
        "1",
        "org.springframework.core.convert.ConverterNotFoundException: No converter found capable of converting from type [java.lang.Integer] to type [java.lang.Boolean]",
    );
}

// covers: config.coerce.005
#[test]
fn an_empty_boolean_keeps_the_existing_value() {
    for id in [
        "config.coerce.005",
        "config.coerce.005-b",
        "config.coerce.005-c",
    ] {
        parity(id).expect("hapi.fhir.etag-support-enabled", json!(true));
    }
}

// covers: config.coerce.006
#[test]
fn integer_text_sign_and_whitespace() {
    for (id, v) in [
        ("config.coerce.006", 5),
        ("config.coerce.006-b", 5),
        ("config.coerce.006-c", 50),
    ] {
        parity(id).expect("hapi.fhir.default-page-size", json!(v));
    }
}

// covers: config.coerce.007
#[test]
fn hexadecimal_integer_text() {
    for (id, v) in [
        ("config.coerce.007", 16),
        ("config.coerce.007-b", 16),
        ("config.coerce.007-c", -16),
        ("config.coerce.007-d", 10),
    ] {
        parity(id).expect("hapi.fhir.default-page-size", json!(v));
    }
    // the YAML scalar 010 is octal (config.yaml.005, config.coerce.013)
    parity("config.coerce.013-f").expect("hapi.fhir.mdm-rules-json-location", json!("8"));
}

// covers: config.coerce.008
#[test]
fn underscores_and_decimals_in_integer_text_fail() {
    for (id, v) in [
        ("config.coerce.008", "1_000"),
        ("config.coerce.008-b", "5.0"),
    ] {
        bind_reason(
            id,
            "hapi.fhir.default-page-size",
            "java.lang.Integer",
            v,
            &format!(
                "failed to convert java.lang.String to java.lang.Integer (caused by java.lang.NumberFormatException: For input string: \"{v}\")"
            ),
        );
    }
}

// covers: config.coerce.009
#[test]
fn yaml_numbers_convert_to_integer_by_truncation() {
    for (id, v) in [
        ("config.coerce.009", 1000),
        ("config.coerce.009-b", 80),
        ("config.coerce.009-c", 5),
        ("config.coerce.009-d", 1000),
        ("config.coerce.009-e", 16),
    ] {
        parity(id).expect("hapi.fhir.default-page-size", json!(v));
    }
}

// covers: config.coerce.010
#[test]
fn out_of_range_numbers_fail() {
    bind_reason(
        "config.coerce.010",
        "hapi.fhir.default-page-size",
        "java.lang.Integer",
        "99999999999",
        "failed to convert java.lang.Long to java.lang.Integer (caused by java.lang.IllegalArgumentException: Could not convert number [99999999999] of type [java.lang.Long] to target class [java.lang.Integer]: overflow)",
    );
    bind_reason(
        "config.coerce.010-b",
        "hapi.fhir.default-page-size",
        "java.lang.Integer",
        "Infinity",
        "failed to convert java.lang.Double to java.lang.Integer (caused by java.lang.IllegalArgumentException: Could not convert number [Infinity] of type [java.lang.Double] to target class [java.lang.Integer]: overflow)",
    );
    parity("config.coerce.010-c").expect(
        "hapi.fhir.retain-cached-searches-mins",
        json!(99999999999_i64),
    );
}

// covers: config.coerce.011
#[test]
fn an_empty_number_keeps_the_existing_value() {
    for id in ["config.coerce.011", "config.coerce.011-b"] {
        parity(id)
            .expect("hapi.fhir.default-page-size", json!(20))
            .expect("hapi.fhir.max-binary-size", Value::Null);
    }
}

// covers: config.coerce.012
#[test]
fn an_empty_value_in_an_object_property_is_skipped() {
    parity("config.coerce.012")
        .expect("hapi.fhir.subscription.email.port", json!(25))
        .expect("hapi.fhir.subscription.email.username", json!(""));
}

// covers: config.coerce.013
#[test]
fn yaml_booleans_and_numbers_bound_to_text() {
    let want = [
        "false", "true", "1.1", "1.0", "0.8.0", "8", "31", "80", "0.5", "1000.0",
    ];
    for (i, w) in want.iter().enumerate() {
        let id = if i == 0 {
            "config.coerce.013".to_string()
        } else {
            format!("config.coerce.013-{}", (b'a' + i as u8) as char)
        };
        parity(&id).expect("hapi.fhir.mdm-rules-json-location", json!(w));
    }
}

// covers: config.coerce.014
#[test]
fn empty_text_is_the_empty_string_not_absent() {
    for id in [
        "config.coerce.014",
        "config.coerce.014-b",
        "config.coerce.014-c",
    ] {
        parity(id).expect("hapi.fhir.server-address", json!(""));
    }
}

// covers: config.coerce.015
#[test]
fn text_is_not_trimmed() {
    parity("config.coerce.015").expect("hapi.fhir.server-address", json!(" http://x "));
}

// covers: config.coerce.016
#[test]
fn enums_by_exact_name() {
    parity("config.coerce.016").expect("hapi.fhir.fhir-version", json!("R4B"));
    parity("config.coerce.016-b").expect("hapi.fhir.fhir-version", json!("DSTU2_HL7ORG"));
}

// covers: config.coerce.017
#[test]
fn enums_leniently_case_separators_and_whitespace() {
    for (id, v) in [
        ("config.coerce.017", "R4"),
        ("config.coerce.017-b", "R4"),
        ("config.coerce.017-c", "DSTU3"),
        ("config.coerce.017-d", "R5"),
    ] {
        parity(id).expect("hapi.fhir.fhir-version", json!(v));
    }
    parity("config.coerce.017-e").expect(
        "hapi.fhir.implementation-guides.x.install-mode",
        json!("STORE_AND_INSTALL"),
    );
    parity("config.coerce.017-f").expect("hapi.fhir.cr.cql.data.profile-mode", json!("ENFORCED"));
    parity("config.coerce.017-g").expect("hapi.fhir.cr.cql.data.profile-mode", json!("DECLARED"));
}

// covers: config.coerce.018
#[test]
fn an_unknown_enum_value_fails() {
    for (id, v) in [
        ("config.coerce.018", "4.0.1"),
        ("config.coerce.018-b", "STU3"),
    ] {
        bind_reason(
            id,
            "hapi.fhir.fhir-version",
            "ca.uhn.fhir.context.FhirVersionEnum",
            v,
            &format!(
                "failed to convert java.lang.String to ca.uhn.fhir.context.FhirVersionEnum (caused by java.lang.IllegalArgumentException: No enum constant ca.uhn.fhir.context.FhirVersionEnum.{v})"
            ),
        );
    }
}

// covers: config.coerce.019
#[test]
fn a_yaml_boolean_binds_to_an_enums_on_or_off() {
    for id in [
        "config.coerce.019",
        "config.coerce.019-b",
        "config.coerce.019-c",
    ] {
        parity(id).expect("hapi.fhir.cr.cql.data.profile-mode", json!("OFF"));
    }
}

// covers: config.coerce.020
#[test]
fn an_empty_enum_keeps_the_existing_value() {
    parity("config.coerce.020").expect("hapi.fhir.fhir-version", json!("R4"));
}

// covers: config.coerce.021
#[test]
fn comma_separated_text_becomes_a_list_trimmed_empties_kept() {
    parity("config.coerce.021").expect(
        "hapi.fhir.supported-resource-types",
        json!(["Patient", "Observation", "", "Encounter"]),
    );
}

// covers: config.coerce.022
#[test]
fn list_elements_convert_individually() {
    parity("config.coerce.022")
        .expect("hapi.fhir.search-prefetch-thresholds", json!([13, 503, -1]))
        .expect(
            "hapi.fhir.allowed-bundle-types",
            json!(["COLLECTION", "TRANSACTIONRESPONSE"]),
        );
}

// covers: config.coerce.023
#[test]
fn yaml_sequence_elements_convert_individually() {
    parity("config.coerce.023").expect(
        "hapi.fhir.supported-resource-types",
        json!(["Patient", "1.1", "true", ""]),
    );
}

// covers: config.coerce.024
#[test]
fn sets_drop_duplicates() {
    parity("config.coerce.024").expect(
        "hapi.fhir.implementation-guides.x.additional-resource-folders",
        json!(["ex1"]),
    );
}

// covers: config.coerce.025
#[test]
fn an_empty_collection_value_is_an_empty_collection() {
    parity("config.coerce.025")
        .expect("hapi.fhir.cors.allowed-origin", json!([]))
        .expect("hapi.fhir.supported-resource-types", json!([]));
    parity("config.coerce.025-b").expect("hapi.fhir.cors.allowed-origin", json!([]));
    parity("config.coerce.025-c")
        .expect("hapi.fhir.cors.allowed-headers", json!([]))
        .expect("hapi.fhir.cors.exposed-headers", json!([]))
        .expect("hapi.fhir.cors.allowed-methods", json!([]));
}

// covers: config.coerce.026
#[test]
fn a_yaml_integer_binds_to_an_enum_by_ordinal() {
    let r = parity("config.coerce.026");
    r.expect("hapi.fhir.fhir-version", json!("R5"))
        .expect("hapi.fhir.binary-storage-mode", json!("FILESYSTEM"))
        .expect_read("hapi.fhir.fhir_version", json!("6"))
        .expect_read("hapi.fhir.binary_storage_mode", json!("1"));
    for g in [
        "fhir-dstu2",
        "fhir-dstu3",
        "fhir-r4",
        "fhir-r4b",
        "fhir-r5",
        "fhir-any",
        "binary-storage-filesystem",
        "binary-storage-database",
    ] {
        r.expect_gate(g, false);
    }
    let b = parity("config.coerce.026-b");
    b.expect_failure().expect_text(
        "No enum constant ca.uhn.fhir.jpa.api.config.JpaStorageSettings.ClientIdStrategyEnum.1",
    );
}

// covers: config.coerce.027
#[test]
fn an_out_of_range_ordinal_fails() {
    bind_reason(
        "config.coerce.027",
        "hapi.fhir.binary-storage-mode",
        "ca.uhn.fhir.jpa.starter.AppProperties$BinaryStorageMode",
        "7",
        "failed to convert java.lang.Integer to ca.uhn.fhir.jpa.starter.AppProperties$BinaryStorageMode (caused by java.lang.ArrayIndexOutOfBoundsException: Index 7 out of bounds for length 2)",
    );
}
