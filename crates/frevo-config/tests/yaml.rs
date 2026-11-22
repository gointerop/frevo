//! config.yaml: YAML documents, SnakeYAML 2.4 resolution and Spring's flattening.

mod common;

use common::parity;
use serde_json::{Value, json};

fn srt(id: &str) -> Value {
    parity(id).get("hapi.fhir.supported-resource-types")
}

// covers: config.yaml.001
#[test]
fn nested_mappings_and_dotted_keys_are_equivalent() {
    for id in ["config.yaml.001", "config.yaml.001-b", "config.yaml.001-c"] {
        parity(id).expect("hapi.fhir.default-page-size", json!(3));
    }
}

// covers: config.yaml.002
#[test]
fn multiple_documents_in_one_file_apply_in_order() {
    parity("config.yaml.002").expect("hapi.fhir.default-page-size", json!(2));
}

// covers: config.yaml.003
#[test]
fn a_duplicate_key_in_one_mapping_fails_to_start() {
    // Finding O-5 (tasks/blockers.md): HAPI's exception line is the SnakeYAML message
    // without the `org.yaml.snakeyaml.constructor.DuplicateKeyException: ` prefix the
    // scenario quotes; the test follows the record.
    parity("config.yaml.003").expect_run_failed(&[
        "while constructing a mapping",
        " in 'reader', line 3, column 5:",
        "        default_page_size: 3",
        "found duplicate key default_page_size",
        " in 'reader', line 4, column 5:",
        "        default_page_size: 4",
    ]);
}

// covers: config.yaml.004
#[test]
fn yaml_1_1_booleans() {
    let expected: Vec<&str> =
        "true True TRUE false False FALSE yes Yes YES no No NO on On ON off Off OFF"
            .split(' ')
            .map(|b| {
                if [
                    "true", "True", "TRUE", "yes", "Yes", "YES", "on", "On", "ON",
                ]
                .contains(&b)
                {
                    "true"
                } else {
                    "false"
                }
            })
            .collect();
    assert_eq!(srt("config.yaml.004"), json!(expected));
    assert_eq!(srt("config.yaml.004-b"), json!(["y", "Y", "n", "tRUE"]));
}

// covers: config.yaml.005
#[test]
fn yaml_1_1_integers() {
    assert_eq!(
        srt("config.yaml.005"),
        json!(["31", "8", "511", "5", "1000", "685230", "09", "0o17"])
    );
}

// covers: config.yaml.006
#[test]
fn yaml_1_1_floats_render_as_java_doubles() {
    assert_eq!(
        srt("config.yaml.006"),
        json!([
            "1.1",
            "1.0",
            "0.5",
            "0.5",
            "10.5",
            "1000.0",
            "1000.0",
            "-Infinity",
            "NaN",
            "1.0E7",
            "1.0E-4"
        ])
    );
}

// covers: config.yaml.007
#[test]
fn timestamps_quoted_scalars_and_str_tags_stay_text() {
    parity("config.yaml.007")
        .expect("hapi.fhir.server-address", json!("2026-01-02"))
        .expect(
            "hapi.fhir.custom-content-path",
            json!("2026-01-02T10:00:00Z"),
        )
        .expect("hapi.fhir.mdm-rules-json-location", json!("010"))
        .expect("hapi.fhir.app-content-path", json!("yes"))
        .expect("hapi.fhir.logger.name", json!("010"));
}

// covers: config.yaml.008
#[test]
fn null_tilde_and_an_absent_value_are_the_empty_string() {
    let r = parity("config.yaml.008");
    for p in [
        "hapi.fhir.server-address",
        "hapi.fhir.custom-content-path",
        "hapi.fhir.mdm-rules-json-location",
        "hapi.fhir.binary-storage-filesystem-base-directory",
        "hapi.fhir.logger.name",
    ] {
        r.expect(p, json!(""));
    }
}

// covers: config.yaml.009
#[test]
fn anchors_aliases_and_merge_keys() {
    parity("config.yaml.009")
        .expect("hapi.fhir.default-page-size", json!(5))
        .expect("hapi.fhir.max-page-size", json!(9));
}

// covers: config.yaml.010
#[test]
fn an_empty_sequence_is_a_value_an_empty_mapping_is_nothing() {
    parity("config.yaml.010")
        .expect("hapi.fhir.cors.allowed-origin", json!([]))
        .expect("hapi.fhir.partitioning", Value::Null);
}

// covers: config.yaml.011
#[test]
fn a_sequence_flattens_to_indexed_elements() {
    parity("config.yaml.011").expect(
        "hapi.fhir.supported-resource-types",
        json!(["Patient", "Observation"]),
    );
}

// covers: config.yaml.012
#[test]
fn non_string_mapping_keys_take_their_resolved_text() {
    let r = parity("config.yaml.012");
    let igs = r.get("hapi.fhir.implementation-guides");
    assert_eq!(
        igs.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["false"]
    );
    r.expect("hapi.fhir.implementation-guides.false.name", json!("x"));
}

// covers: config.yaml.013
#[test]
fn bracketed_keys_keep_their_dots() {
    parity("config.yaml.013").expect("hapi.fhir.implementation-guides[a.b].name", json!("e"));
}

// covers: config.yaml.014
#[test]
fn block_scalars_fold_as_yaml_specifies() {
    parity("config.yaml.014").expect(
        "hapi.fhir.logger.format",
        json!("Path[${servletPath}] Source[${requestHeader.x-forwarded-for}] Operation[${operationType} ${operationName} ${idOrResourceName}] UA[${requestHeader.user-agent}] Params[${requestParameters}] ResponseEncoding[${responseEncodingNoDefault}]"),
    );
}

// covers: config.yaml.015
#[test]
fn commented_examples_in_the_packaged_file_have_no_effect() {
    let r = parity("config.source.002");
    r.expect("hapi.fhir.max-page-size", json!(2147483647))
        .expect("hapi.fhir.server-address", Value::Null);
}

// covers: config.yaml.016
#[test]
fn a_yaml_syntax_error_fails_to_start_with_its_position() {
    // Finding O-5: no exception class prefix on the first line in HAPI's output.
    parity("config.yaml.016").expect_run_failed(&[
        "while parsing a flow sequence",
        " in 'reader', line 5, column 31:",
        "        supported_resource_types: [${nope3}]",
        "expected ',' or ']', but got {",
        " in 'reader', line 5, column 33:",
    ]);
}
