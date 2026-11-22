//! config.read: the keys the starter reads by name from the Spring Environment.

mod common;

use common::{READS, parity};
use serde_json::{Value, json};

// covers: config.read.001
#[test]
fn the_reads_reported() {
    let r = parity("config.read.001");
    let reads = r.doc()["reads"].as_object().expect("reads");
    let mut got: Vec<&str> = reads.keys().map(String::as_str).collect();
    got.sort();
    let mut want = READS.to_vec();
    want.sort();
    assert_eq!(got, want);
    assert!(reads.values().all(|v| v.is_string() || v.is_null()));
    r.expect_read("hapi.fhir.fhir_version", json!("R4"))
        .expect_read("hapi.fhir.cr.enabled", json!("false"))
        .expect_read("hapi.fhir.mdm_enabled", json!("false"))
        .expect_read("hapi.fhir.ips_enabled", Value::Null)
        .expect_read("spring.ai.mcp.server.enabled", json!("true"))
        .expect_read("scheduling_disabled", Value::Null);
}

// covers: config.read.002
#[test]
fn an_underscore_name_is_read_exactly() {
    // DEBT-0022
    parity("config.read.002")
        .expect("hapi.fhir.mdm-enabled", json!(true))
        .expect("hapi.fhir.ips-enabled", json!(true))
        .expect_read("hapi.fhir.mdm_enabled", json!("false"))
        .expect_read("hapi.fhir.ips_enabled", Value::Null);
}

// covers: config.read.003
#[test]
fn a_canonical_name_is_read_relaxed() {
    parity("config.read.003")
        .expect_read("hapi.fhir.cr.enabled", json!("true"))
        .expect_read("hapi.fhir.cdshooks.enabled", json!("true"));
    parity("config.read.003-b").expect_read("hapi.fhir.custom-bean-packages", json!("a.b"));
}

// covers: config.read.004
#[test]
fn environment_variable_variants_for_an_exact_read() {
    parity("config.read.004").expect_read("hapi.fhir.mdm_enabled", json!("true"));
    parity("config.read.004-b")
        .expect_read("hapi.fhir.ips_enabled", Value::Null)
        .expect("hapi.fhir.ips-enabled", json!(true));
    // the variants: n with . and - replaced by _, upper-cased (HAPI_FHIR_IPS_ENABLED) ...
    parity("config.read.004-c").expect_read("hapi.fhir.ips_enabled", json!("true"));
    // ... and, recorded on Linux, the lower-case form with . replaced by _ (finding O-2)
    parity("config.read.004-d").expect_read("hapi.fhir.ips_enabled", json!("true"));
}

// covers: config.read.005
#[test]
fn a_dashed_spelling_splits_the_read_from_the_binding() {
    // DEBT-0022
    parity("config.read.005")
        .expect("hapi.fhir.fhir-version", json!("R5"))
        .expect_read("hapi.fhir.fhir_version", json!("R4"));
}

// covers: config.read.006
#[test]
fn reads_see_yaml_values_as_their_text() {
    parity("config.read.006").expect_read("hapi.fhir.mdm_enabled", json!("true"));
    parity("config.read.006-b").expect_read("hapi.fhir.mdm_enabled", json!(""));
}

// covers: config.read.007
#[test]
fn reads_resolve_placeholders_strictly() {
    parity("config.placeholder.011").expect_failure();
}

// covers: config.read.008
#[test]
fn scheduling_disabled_is_read_by_exact_name() {
    parity("config.read.008").expect_read("scheduling_disabled", json!("true"));
    parity("config.read.008-b").expect_read("scheduling_disabled", json!("true"));
    parity("config.read.008-c").expect_read("scheduling_disabled", json!("true"));
    parity("config.read.008-d").expect_read("scheduling_disabled", json!("TRUE"));
    parity("config.read.008-e").expect_read("scheduling_disabled", Value::Null);
}
