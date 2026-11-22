//! config.gate: the starter's conditions, evaluated with the starter's predicates.

mod common;

use common::{GATES, parity};
use serde_json::json;

fn on(id: &str) -> Vec<String> {
    let mut g = parity(id).gates_on();
    g.sort();
    g
}

// covers: config.gate.001
#[test]
fn the_gates_reported() {
    let r = parity("config.gate.001");
    let gates = r.doc()["gates"].as_object().expect("gates");
    let mut got: Vec<&str> = gates.keys().map(String::as_str).collect();
    got.sort();
    let mut want = GATES.to_vec();
    want.sort();
    assert_eq!(got, want);
    assert!(gates.values().all(|v| v.is_boolean()));
    let mut expected = vec![
        "binary-storage-database",
        "cors",
        "fhir-any",
        "fhir-r4",
        "mcp",
        "tester",
    ];
    expected.sort();
    assert_eq!(on("config.gate.001"), expected);
}

// covers: config.gate.002
#[test]
fn the_version_gates_follow_for_version_string() {
    for (id, gate) in [
        ("config.gate.002", "fhir-r4"),
        ("config.gate.002-b", "fhir-r4"),
        ("config.gate.002-c", "fhir-r4b"),
        ("config.gate.002-d", "fhir-r5"),
        ("config.gate.002-e", "fhir-dstu2"),
        ("config.gate.002-f", "fhir-dstu3"),
        ("config.gate.002-g", "fhir-dstu3"),
    ] {
        let r = parity(id);
        for g in ["fhir-dstu2", "fhir-dstu3", "fhir-r4", "fhir-r4b", "fhir-r5"] {
            r.expect_gate(g, g == gate);
        }
        r.expect_gate("fhir-any", true);
    }
    // 4.0 and 4.0.1 do not bind to hapi.fhir.fhir-version (config.coerce.018), so the
    // start fails before any gate is reported (finding O-10: the gate clause is not
    // observable through `effective`).
    parity("config.gate.002-h").expect_failure();
    parity("config.gate.002-i").expect_failure();
}

// covers: config.gate.003
#[test]
fn an_empty_version_turns_on_dstu2() {
    // DEBT-0023
    parity("config.coerce.020")
        .expect("hapi.fhir.fhir-version", json!("R4"))
        .expect_gate("fhir-dstu2", true)
        .expect_gate("fhir-r4", false);
}

// covers: config.gate.004
#[test]
fn a_version_the_gates_do_not_know_turns_every_version_gate_off() {
    for id in ["config.gate.004", "config.gate.004-b"] {
        let r = parity(id);
        for g in [
            "fhir-dstu2",
            "fhir-dstu3",
            "fhir-r4",
            "fhir-r4b",
            "fhir-r5",
            "fhir-any",
        ] {
            r.expect_gate(g, false);
        }
    }
    parity("config.gate.004").expect("hapi.fhir.fhir-version", json!("R4"));
}

// covers: config.gate.005
#[test]
fn no_fhir_version_anywhere_fails_to_start_as_hapi_does() {
    let r = parity("config.gate.005");
    r.expect_run_failed(&[
        "java.lang.NullPointerException: Cannot invoke \"String.toUpperCase()\" because the return value of \"org.springframework.core.env.Environment.getProperty(String)\" is null",
    ]);
}

// covers: config.gate.006
#[test]
fn cr_and_cdshooks_use_boolean_parse_boolean() {
    for (id, gate) in [
        ("config.gate.006", true),
        ("config.gate.006-b", true),
        ("config.gate.006-c", false),
        ("config.gate.006-d", false),
        ("config.gate.006-e", false),
    ] {
        parity(id).expect_gate("cr", gate);
    }
    // DEBT-0022: the quoted "yes" binds true while the read is "yes" and the gate is off
    for id in [
        "config.gate.006-c",
        "config.gate.006-f",
        "config.gate.006-g",
    ] {
        parity(id)
            .expect("hapi.fhir.cr.enabled", json!(true))
            .expect_read("hapi.fhir.cr.enabled", json!("yes"))
            .expect_gate("cr", false);
    }
    parity("config.gate.006-h").expect_gate("cdshooks", true);
    parity("config.gate.006-i").expect_gate("cdshooks", false);
}

// covers: config.gate.007
#[test]
fn mdm_is_on_for_any_present_value_but_false() {
    for (id, gate) in [
        ("config.gate.007", true),
        ("config.gate.007-b", true),
        ("config.gate.007-c", true),
        ("config.gate.007-d", true),
        ("config.gate.007-e", false),
        ("config.gate.007-f", false),
    ] {
        parity(id).expect_gate("mdm", gate);
    }
    // DEBT-0022
    parity("config.gate.007-b").expect("hapi.fhir.mdm-enabled", json!(false));
}

// covers: config.gate.008
#[test]
fn having_value_gates_compare_ignoring_case() {
    for (id, gate) in [("config.gate.008", true), ("config.gate.008-b", false)] {
        let r = parity(id);
        for g in [
            "ips",
            "ig-runtime-upload",
            "repository-validating-interceptor",
        ] {
            r.expect_gate(g, gate);
        }
    }
}

// covers: config.gate.009
#[test]
fn binary_storage_gates() {
    for (id, db, fs) in [
        ("config.gate.009", true, false),
        ("config.gate.009-b", true, false),
        ("config.gate.009-c", false, true),
        ("config.gate.009-d", false, false),
    ] {
        parity(id)
            .expect_gate("binary-storage-database", db)
            .expect_gate("binary-storage-filesystem", fs);
    }
    parity("config.gate.009-d").expect("hapi.fhir.binary-storage-mode", json!("FILESYSTEM"));
}

// covers: config.gate.010
#[test]
fn custom_and_app_content_gates() {
    for (id, gate) in [
        ("config.gate.010", true),
        ("config.gate.010-b", true),
        ("config.gate.010-c", false),
        ("config.gate.010-d", false),
    ] {
        parity(id)
            .expect_gate("custom-content", gate)
            .expect_gate("app-content", gate);
    }
}

// covers: config.gate.011
#[test]
fn mcp_gates() {
    parity("config.gate.001")
        .expect_gate("mcp", true)
        .expect_gate("mcp-cdshooks", false);
    parity("config.gate.011").expect_gate("mcp", false);
    parity("config.gate.011-b").expect_gate("mcp-cdshooks", true);
}

// covers: config.gate.012
#[test]
fn the_cors_gate_follows_the_bound_object() {
    let r = parity("config.gate.012");
    assert!(r.get("hapi.fhir.cors").is_object());
    r.expect_gate("cors", true);
    parity("config.gate.012-b").expect_gate("cors", false);
}

// covers: config.gate.013
#[test]
fn the_implementation_guides_gate() {
    parity("config.gate.001").expect_gate("implementation-guides", false);
    parity("config.gate.013").expect_gate("implementation-guides", false);
    parity("config.gate.013-b").expect_gate("implementation-guides", true);
}

// covers: config.gate.014
#[test]
fn the_partitioning_gate() {
    parity("config.object.001").expect_gate("partitioning", false);
    parity("config.object.001-b").expect_gate("partitioning", false);
    parity("config.gate.014").expect_gate("partitioning", true);
}

// covers: config.gate.015
#[test]
fn the_remote_terminology_gate() {
    parity("config.gate.001").expect_gate("remote-terminology", false);
    parity("config.gate.015").expect_gate("remote-terminology", false);
    parity("config.unknown.006-b").expect_gate("remote-terminology", true);
}

// covers: config.gate.016
#[test]
fn the_tester_gate_is_a_raw_name_prefix_test() {
    parity("config.gate.016").expect_gate("tester", true);
    parity("config.gate.016-b")
        .expect_gate("tester", false)
        .expect("hapi.fhir.tester.home.name", json!("x"));
}

// covers: config.gate.017
#[test]
fn cds_hooks_need_both_gates() {
    let r = parity("config.gate.017");
    r.expect_gate("cdshooks", true).expect_gate("cr", false);
    assert_eq!(
        r.doc()["gates"].as_object().unwrap().len(),
        25,
        "no separate member for the conjunction"
    );
}

// covers: config.gate.018
#[test]
fn gates_are_computed_after_every_source_is_applied() {
    for id in [
        "config.gate.018",
        "config.gate.018-b",
        "config.gate.018-c",
        "config.gate.018-d",
    ] {
        parity(id).expect_gate("mdm", true);
    }
}

// covers: config.gate.019
#[test]
fn a_failure_while_computing_a_gate_fails_to_start() {
    parity("config.unknown.006")
        .expect_failure()
        .expect_text("Failed to bind properties under 'hapi.fhir'");
}

// covers: config.gate.020
#[test]
fn the_elasticsearch_gate_binds_elasticsearch_properties() {
    for id in [
        "config.gate.020",
        "config.gate.020-b",
        "config.gate.020-c",
        "config.gate.020-d",
        "config.profile.002",
    ] {
        parity(id).expect_gate("elasticsearch", true);
    }
    parity("config.gate.020-e").expect_gate("elasticsearch", false);
    parity("config.gate.001").expect_gate("elasticsearch", false);
}

// covers: config.gate.021
#[test]
fn the_scheduling_disabled_gate_is_case_sensitive() {
    parity("config.read.008").expect_gate("scheduling-disabled", true);
    parity("config.read.008-d").expect_gate("scheduling-disabled", false);
    parity("config.read.008-c").expect_gate("scheduling-disabled", true); // YAML yes reads "true"
    parity("config.gate.001").expect_gate("scheduling-disabled", false);
    parity("config.read.008-b").expect_gate("scheduling-disabled", true);
    parity("config.gate.021").expect_gate("scheduling-disabled", false);
}

// covers: config.gate.022
#[test]
fn system_property_names_count_for_the_tester_gate() {
    parity("config.gate.022").expect_gate("tester", true);
}

// covers: config.gate.023
#[test]
fn an_unquoted_yaml_yes_turns_the_cr_gate_on() {
    parity("config.gate.023")
        .expect("hapi.fhir.cr.enabled", json!(true))
        .expect_read("hapi.fhir.cr.enabled", json!("true"))
        .expect_gate("cr", true);
}

// covers: config.gate.024
#[test]
fn elasticsearch_properties_is_bound_and_type_checked() {
    for id in [
        "config.gate.024",
        "config.gate.024-b",
        "config.gate.024-c",
        "config.gate.024-d",
    ] {
        parity(id).expect_gate("elasticsearch", true);
    }
    let duration = |v: &str| {
        format!(
            "    Reason: failed to convert java.lang.String to java.time.Duration (caused by java.lang.IllegalArgumentException: '{v}' is not a valid duration)"
        )
    };
    for (id, v) in [
        ("config.gate.024-e", "5 s"),
        ("config.gate.024-f", "1.5s"),
        ("config.gate.024-g", "abc"),
        ("config.gate.024-j", "abc"),
    ] {
        parity(id).expect_analysis(
            "Failed to bind properties under 'spring.elasticsearch.connection-timeout' to java.time.Duration:",
            &[&duration(v)],
        );
    }
    parity("config.gate.024-h").expect_analysis(
        "Failed to bind properties under 'spring.elasticsearch.socket-keep-alive' to boolean:",
        &["    Reason: failed to convert java.lang.String to boolean (caused by java.lang.IllegalArgumentException: Invalid boolean value 'maybe')"],
    );
    parity("config.gate.024-i").expect_analysis(
        "Failed to bind properties under 'spring.elasticsearch' to org.springframework.boot.autoconfigure.elasticsearch.ElasticsearchProperties:",
        &["    Reason: org.springframework.core.convert.ConverterNotFoundException: No converter found capable of converting from type [java.lang.String] to type [org.springframework.boot.autoconfigure.elasticsearch.ElasticsearchProperties]"],
    );
}
