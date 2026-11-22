//! config.profile: profile activation, profile files and on-profile documents.

mod common;

use common::{parity, run};
use serde_json::{Value, json};

// covers: config.profile.001
#[test]
fn the_packaged_cds_profile() {
    let r = parity("config.profile.001");
    assert_eq!(r.active(), ["cds"]);
    r.expect("hapi.fhir.cr.enabled", json!(true))
        .expect("hapi.fhir.cdshooks.enabled", json!(true))
        .expect("hapi.fhir.allow-external-references", json!(true))
        .expect("hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode", json!("REQUIRE"))
        .expect("hapi.fhir.cr.cql.terminology.code-lookup-mode", json!("USE_CODESYSTEM_URL"))
        .expect(
            "hapi.fhir.cr.cql.namespaces",
            json!({"hl7.fhir.uv.cql": "http://hl7.org/fhir/uv/cql", "hl7.fhir.us.cql": "http://hl7.org/fhir/us/cql"}),
        )
        .expect("hapi.fhir.implementation-guides.cql.name", json!("hl7.fhir.uv.cql"))
        .expect("hapi.fhir.implementation-guides.cql.version", json!("2.0.0"))
        .expect("hapi.fhir.implementation-guides.uscql.name", json!("hl7.fhir.us.cql"))
        .expect("hapi.fhir.implementation-guides.uscql.version", json!("1.0.0"))
        .expect("hapi.fhir.tester.home.name", json!("Local Tester"));
    let types = json!([
        "CodeSystem",
        "ValueSet",
        "ConceptMap",
        "ActivityDefinition",
        "PlanDefinition",
        "Library",
        "Measure",
        "StructureDefinition"
    ]);
    for ig in ["cql", "uscql"] {
        r.expect(
            &format!("hapi.fhir.implementation-guides.{ig}.install-mode"),
            json!("STORE_AND_INSTALL"),
        )
        .expect(
            &format!("hapi.fhir.implementation-guides.{ig}.install-resource-types"),
            types.clone(),
        );
    }
}

// covers: config.profile.002
#[test]
fn the_packaged_elastic_profile() {
    parity("config.profile.002")
        .expect("hapi.fhir.advanced-lucene-indexing", json!(true))
        .expect("hapi.fhir.search-index-full-text-enabled", json!(true))
        .expect_gate("elasticsearch", true);
}

// covers: config.profile.003
#[test]
fn external_base_files_beat_packaged_profile_files() {
    parity("config.profile.003").expect("hapi.fhir.cdshooks.enabled", json!(false));
}

// covers: config.profile.004
#[test]
fn the_last_active_profile_wins() {
    parity("config.profile.004").expect("hapi.fhir.default-page-size", json!(12));
    parity("config.profile.004-b").expect("hapi.fhir.default-page-size", json!(11));
}

// covers: config.profile.005
#[test]
fn a_profile_specific_file_beats_a_profile_document_in_a_base_file() {
    parity("config.profile.005").expect("hapi.fhir.default-page-size", json!(10));
    parity("config.profile.005-b").expect("hapi.fhir.default-page-size", json!(10));
}

// covers: config.profile.006
#[test]
fn on_profile_documents_negation_and_expressions() {
    parity("config.profile.006")
        .expect("hapi.fhir.default-page-size", json!(1))
        .expect("hapi.fhir.max-page-size", json!(3))
        .expect("hapi.fhir.server-address", Value::Null);
    parity("config.profile.006-b")
        .expect("hapi.fhir.default-page-size", json!(2))
        .expect("hapi.fhir.max-page-size", json!(2))
        .expect("hapi.fhir.server-address", Value::Null);
    parity("config.profile.006-c")
        .expect("hapi.fhir.max-page-size", json!(2))
        .expect("hapi.fhir.server-address", json!("a-and-b"));
}

// covers: config.profile.007
#[test]
fn the_default_profile() {
    let r = parity("config.profile.007");
    r.expect("hapi.fhir.server-address", json!("default-profile"));
    assert!(r.active().is_empty());
    parity("config.profile.007-b").expect("hapi.fhir.server-address", Value::Null);
}

// covers: config.profile.008
#[test]
fn spring_profiles_default_renames_the_default_profile() {
    let r = parity("config.profile.008");
    r.expect("hapi.fhir.server-address", json!("dflt"));
    assert_eq!(r.default_profiles(), ["dd"]);
}

// covers: config.profile.009
#[test]
fn include_adds_profiles_ahead_of_the_active_ones() {
    let r = parity("config.profile.009");
    assert_eq!(r.active(), ["b", "cds"]);
    r.expect("hapi.fhir.default-page-size", json!(15));
}

// covers: config.profile.010
#[test]
fn spring_profiles_active_set_in_a_file_activates_profiles() {
    let r = parity("config.profile.010");
    assert_eq!(r.active(), ["cds"]);
    r.expect("hapi.fhir.cr.enabled", json!(true));
}

// covers: config.profile.011
#[test]
fn profile_groups_expand_in_place() {
    let r = parity("config.profile.011");
    assert_eq!(r.active(), ["g", "b", "a"]);
    r.expect("hapi.fhir.default-page-size", json!(11));
}

// covers: config.profile.012
#[test]
fn spring_profiles_active_in_a_profile_specific_file_fails_to_start() {
    parity("config.profile.012").expect_run_failed(&[
        "org.springframework.boot.context.config.InvalidConfigDataPropertyException: Property 'spring.profiles.active' imported from location 'file [application-q.yaml]' is invalid in a profile specific resource [origin: URL [file:application-q.yaml] - 1:25]",
    ]);
}

// covers: config.profile.013
#[test]
fn spring_profiles_active_in_an_on_profile_document_is_ignored() {
    assert_eq!(parity("config.profile.013").active(), ["q"]);
}

// covers: config.profile.014
#[test]
fn on_cloud_platform_documents_never_activate() {
    // DEV-0031: HAPI applies the document when it detects Kubernetes (its record shows 7).
    let r = run("config.profile.014");
    r.expect("hapi.fhir.default-page-size", json!(20))
        .expect("hapi.fhir.max-page-size", json!(1));
    assert_eq!(
        common::hapi("config.profile.014")["document"]["properties"]["hapi.fhir"]["default-page-size"],
        json!(7)
    );
}

// covers: config.profile.015
#[test]
fn profile_names_are_validated() {
    parity("config.profile.015").expect_analysis(
        "Failed to bind properties under 'spring.profiles.active' to java.util.Set<java.lang.String>:",
        &[
            "    Property: spring.profiles.active",
            "    Value: \"a b\"",
            "    Origin: System Environment Property \"SPRING_PROFILES_ACTIVE\"",
            "    Reason: java.lang.IllegalStateException: Profile 'a b' must contain a letter, digit or allowed char ('-', '_', '.', '+', '@')",
        ],
    );
    let head = "java.lang.IllegalStateException: Invalid profile property value found in Environment under 'spring.profiles.active'";
    for (id, cause) in [
        (
            "config.profile.015-b",
            "Caused by: java.lang.IllegalStateException: Profile '-x' must start and end with a letter or digit",
        ),
        (
            "config.profile.015-c",
            "Caused by: java.lang.IllegalStateException: Profile 'a/b' must contain a letter, digit or allowed char ('-', '_', '.', '+', '@')",
        ),
        (
            "config.profile.015-d",
            "Caused by: java.lang.IllegalStateException: Profile 'x!' must contain a letter, digit or allowed char ('-', '_', '.', '+', '@')",
        ),
        (
            "config.profile.015-e",
            "Caused by: java.lang.IllegalStateException: Invalid empty profile",
        ),
    ] {
        parity(id).expect_run_failed(&[head, cause]);
    }
    assert_eq!(parity("config.profile.015-f").active(), ["a.b+c@d_e"]);
    assert_eq!(parity("config.profile.015-g").active(), ["a", "b"]);
}

// covers: config.profile.016
#[test]
fn validation_can_be_turned_off() {
    assert_eq!(parity("config.profile.016").active(), ["-x"]);
}

// covers: config.profile.017
#[test]
fn the_legacy_spring_profiles_key_fails_to_start() {
    parity("config.profile.017").expect_run_failed(&[
        "org.springframework.boot.context.config.InvalidConfigDataPropertyException: Property 'spring.profiles' imported from location 'file [application.yaml]' is invalid and should be replaced with 'spring.config.activate.on-profile' [origin: URL [file:application.yaml] - 1:18]",
    ]);
}

// covers: config.profile.018
#[test]
fn include_and_default_are_invalid_in_a_profile_specific_file_group_is_not() {
    for (id, key) in [
        ("config.profile.018", "include"),
        ("config.profile.018-b", "default"),
    ] {
        parity(id).expect_run_failed(&[&format!(
            "org.springframework.boot.context.config.InvalidConfigDataPropertyException: Property 'spring.profiles.{key}' imported from location 'file [application-q.yaml]' is invalid in a profile specific resource [origin: URL [file:application-q.yaml] - 1:26]"
        )]);
    }
    assert_eq!(parity("config.profile.018-c").active(), ["q"]);
}
