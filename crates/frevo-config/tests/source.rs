//! config.source: configuration locations, imports and source precedence.
//! (Run under the oracle self-check with the replay stand-in: harness/config-roundtrip/README.md.)
//!
//! Every config case is compared with the recorded HAPI result (`common::parity`); the
//! assertions after it state the scenario's THEN clauses. Where the recorded HAPI result
//! contradicts a THEN clause, the test follows HAPI and names the finding in
//! tasks/blockers.md (add-config-application-yaml, oracle findings).

mod common;

use common::{parity, run};
use serde_json::{Value, json};

// covers: config.source.001
#[test]
fn the_starters_packaged_files_are_built_in() {
    let harness = common::harness().join("packaged");
    let ours = common::repo().join("crates/frevo-config/packaged");
    for f in [
        "application.yaml",
        "application-cds.yaml",
        "application-elastic.yaml",
    ] {
        let war = std::fs::read(harness.join(f))
            .expect("harness/config-roundtrip/packaged holds the war's copy");
        let embedded = std::fs::read(ours.join(f)).unwrap_or_else(|e| {
            panic!("crates/frevo-config/packaged/{f}: the copy Frevo embeds is missing ({e})")
        });
        assert!(
            embedded == war,
            "crates/frevo-config/packaged/{f} is not byte-identical to main.war!/WEB-INF/classes/{f}"
        );
    }
    let app = std::fs::read_to_string(ours.join("application.yaml")).unwrap();
    assert!(
        app.contains("    name: \"hapi-fhir-jpaserver-starter\"\n"),
        "Maven resource filtering applied"
    );
    assert!(!app.contains("@project.artifactId@"));
}

// covers: config.source.002
#[test]
fn with_no_external_file_the_packaged_application_yaml_applies() {
    let r = parity("config.source.002");
    r.expect("hapi.fhir.openapi-enabled", json!(true))
        .expect("hapi.fhir.narrative-enabled", json!(false))
        .expect("hapi.fhir.fhir-version", json!("R4"))
        .expect("hapi.fhir.ig-runtime-upload-enabled", json!(false))
        .expect("hapi.fhir.mdm-enabled", json!(false))
        .expect("hapi.fhir.mdm-rules-json-location", json!("mdm-rules.json"))
        .expect(
            "hapi.fhir.bulk-export-file-retention-period-hours",
            json!(2),
        )
        .expect("hapi.fhir.binary-storage-minimum-binary-size", json!(4000))
        .expect(
            "hapi.fhir.search-prefetch-thresholds",
            json!([13, 503, 2003, -1]),
        )
        .expect("hapi.fhir.cors.allowed-origin", json!(["*"]))
        .expect(
            "hapi.fhir.logical-urls",
            json!([
                "http://loinc.org/*",
                "http://snomed.info/*",
                "http://terminology.hl7.org/*",
                "http://unitsofmeasure.org/*",
                "https://loinc.org/*",
                "https://snomed.info/*",
                "https://terminology.hl7.org/*",
                "https://unitsofmeasure.org/*"
            ]),
        )
        .expect(
            "hapi.fhir.tester",
            json!({"home": {"name": "Local Tester", "server-address": "http://localhost:8080/fhir",
                            "refuse-to-fetch-third-party-urls": false, "fhir-version": "R4"}}),
        )
        .expect("hapi.fhir.cr.enabled", json!(false))
        .expect("hapi.fhir.cdshooks.enabled", json!(false))
        .expect(
            "hapi.fhir.cdshooks.client-id-header-name",
            json!("client_id"),
        );
}

// covers: config.source.003
#[test]
fn default_locations_are_searched_in_spring_boot_order() {
    parity("config.source.003")
        .expect("hapi.fhir.default-page-size", json!(3))
        .expect("hapi.fhir.max-page-size", json!(2));
}

// covers: config.source.004
#[test]
fn config_subdirectories_apply_in_name_order() {
    parity("config.source.004").expect("hapi.fhir.max-page-size", json!(22));
}

// covers: config.source.005
#[test]
fn within_one_location_properties_beats_yml_beats_yaml() {
    parity("config.source.005").expect("hapi.fhir.default-page-size", json!(4));
    parity("config.source.005-b").expect("hapi.fhir.default-page-size", json!(6));
}

// covers: config.source.006
#[test]
fn absent_default_locations_are_not_an_error() {
    let r = parity("config.source.006");
    let packaged = parity("config.source.002");
    assert_eq!(r.doc(), packaged.doc());
}

// covers: config.source.007
#[test]
fn spring_config_location_replaces_the_default_locations_entirely() {
    // Finding O-6 (tasks/blockers.md): exactly as the scenario states it, no source sets
    // hapi.fhir.fhir_version and HAPI fails with config.gate.005's NullPointerException.
    let exact = parity("config.source.007");
    exact.expect_failure();
    // With the version set on the command line (the class default R4):
    let r = parity("config.source.007-v");
    r.expect("hapi.fhir.default-page-size", json!(5))
        .expect("hapi.fhir.openapi-enabled", json!(false))
        .expect("hapi.fhir.tester", Value::Null)
        .expect("hapi.fhir.cors", Value::Null)
        .expect("hapi.fhir.narrative-enabled", json!(true))
        // ./application.yaml (max_page_size: 1) is not loaded either
        .expect("hapi.fhir.max-page-size", json!(2147483647));
}

// covers: config.source.008
#[test]
fn a_missing_mandatory_location_fails_to_start() {
    let r = parity("config.source.008");
    r.expect_analysis(
        "Config data resource 'file [nope.yaml]' via location 'file:./nope.yaml' does not exist",
        &["Check that the value 'file:./nope.yaml' is correct, or prefix it with 'optional:'"],
    );
}

// covers: config.source.009
#[test]
fn an_optional_location_may_be_missing() {
    // Finding O-6: without a version HAPI fails (config.gate.005).
    parity("config.source.009").expect_failure();
    let r = parity("config.source.009-v");
    let defaults = parity("config.key.001-v");
    assert_eq!(
        r.doc()["properties"],
        defaults.doc()["properties"],
        "every property has its class default"
    );
}

// covers: config.source.010
#[test]
fn a_location_ending_in_slash_is_a_directory() {
    parity("config.source.010").expect_failure(); // finding O-6
    parity("config.source.010-v").expect("hapi.fhir.default-page-size", json!(33));
    let p = parity("config.source.010-p");
    p.expect("hapi.fhir.default-page-size", json!(33))
        .expect("hapi.fhir.max-page-size", json!(34));
    assert_eq!(p.active(), ["p"]);
}

// covers: config.source.011
#[test]
fn spring_config_name_changes_the_file_base_name() {
    parity("config.source.011")
        .expect("hapi.fhir.max-page-size", json!(44))
        .expect("hapi.fhir.default-page-size", json!(20));
}

// covers: config.source.012
#[test]
fn additional_location_layers_over_the_defaults() {
    parity("config.source.012")
        .expect("hapi.fhir.default-page-size", json!(5))
        .expect("hapi.fhir.max-page-size", json!(2))
        .expect("hapi.fhir.openapi-enabled", json!(true));
}

// covers: config.source.013
#[test]
fn in_a_location_list_later_entries_win() {
    parity("config.source.013").expect("hapi.fhir.default-page-size", json!(32));
}

// covers: config.source.014
#[test]
fn spring_config_import_imports_a_plain_path_over_the_defaults() {
    parity("config.source.014").expect("hapi.fhir.default-page-size", json!(5));
    parity("config.source.014-b").expect("hapi.fhir.default-page-size", json!(5));
}

// covers: config.source.015
#[test]
fn an_import_in_a_file_resolves_relative_to_that_file_and_overrides_it() {
    parity("config.source.015")
        .expect("hapi.fhir.default-page-size", json!(2))
        .expect("hapi.fhir.max-page-size", json!(1));
}

// covers: config.source.016
#[test]
fn a_missing_mandatory_import_fails_an_optional_one_does_not() {
    let r = parity("config.source.016");
    r.expect_analysis(
        "Config data resource 'file [config/missing.yaml]' via location 'missing.yaml' does not exist",
        &[],
    );
    r.expect_no_text("nope.yaml");
}

// covers: config.source.017
#[test]
fn classpath_resolves_to_the_packaged_files_then_app_extra_classes() {
    parity("config.source.017").expect("hapi.fhir.cr.enabled", json!(true));
    parity("config.source.017-v").expect("hapi.fhir.cr.enabled", json!(true));
    parity("config.source.017-b").expect("hapi.fhir.default-page-size", json!(9));
    parity("config.source.017-c").expect_analysis(
        "Config data resource 'class path resource [another.application.yaml]' via location 'classpath:/another.application.yaml' does not exist",
        &[],
    );
    // the packaged file shadows the same name in /app/extra-classes/
    parity("config.source.017-d")
        .expect("hapi.fhir.cr.enabled", json!(true))
        .expect("hapi.fhir.default-page-size", json!(20));
}

// covers: config.source.018
#[test]
fn file_urls_with_an_authority_less_absolute_path_are_accepted() {
    parity("config.source.018").expect("hapi.fhir.default-page-size", json!(9));
}

// covers: config.source.019
#[test]
fn precedence_across_source_kinds() {
    parity("config.source.019")
        .expect("hapi.fhir.default-page-size", json!(4))
        .expect("hapi.fhir.max-page-size", json!(3))
        .expect("hapi.fhir.expunge-thread-count", json!(2))
        .expect("hapi.fhir.reindex-thread-count", json!(4));
}

// covers: config.source.020
#[test]
fn unsupported_location_kinds_fail_loudly() {
    // DEV-0031: HAPI loads configtree: and .xml locations (its record shows them applied);
    // Frevo fails to start with the run-failed output naming the location.
    for (id, location) in [
        ("config.source.020", "configtree:./tree/"),
        ("config.source.020-b", "x.xml"),
    ] {
        let r = run(id);
        r.expect_failure();
        let lines = r.failure_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.ends_with(": Application run failed")),
            "{id}: the run-failed form of config.error.007: {lines:#?}"
        );
        assert!(
            r.stdout.contains(location),
            "{id}: the report names the location {location}"
        );
        assert!(
            r.stdout.contains("not supported"),
            "{id}: the report states the location kind is not supported"
        );
        assert_eq!(
            common::hapi(id)["exit"],
            json!(0),
            "{id}: HAPI loads it (DEV-0031 evidence)"
        );
    }
    parity("config.source.020-c").expect_run_failed(&[
        "java.lang.IllegalStateException: Unable to load config data from 'configserver:'",
    ]);
    parity("config.source.020-d");
    parity("config.source.020-e").expect_run_failed(&[
        "java.lang.IllegalStateException: Unable to load config data from 'x.txt'",
    ]);
    parity("config.source.020-f");
}

// covers: config.source.021
#[test]
fn an_empty_or_comment_only_file_contributes_nothing() {
    let packaged = parity("config.source.002");
    assert_eq!(parity("config.source.021").doc(), packaged.doc());
    assert_eq!(parity("config.source.021-b").doc(), packaged.doc());
}

// covers: config.source.022
#[test]
fn the_active_sources_are_reported() {
    let r = parity("config.source.022");
    assert!(r.active().is_empty());
    assert_eq!(r.default_profiles(), ["default"]);
}

// covers: config.source.023
#[test]
fn d_tokens_in_the_jvm_option_variables_are_system_properties() {
    let r = parity("config.source.023");
    r.expect("hapi.fhir.default-page-size", json!(6));
    assert_eq!(
        r.stderr,
        "Picked up JAVA_TOOL_OPTIONS: -Xmx3g -Dhapi.fhir.default_page_size=6 -XX:+UseG1GC\n"
    );
    let b = parity("config.source.023-b");
    b.expect("hapi.fhir.default-page-size", json!(9));
    assert_eq!(
        b.stderr,
        "NOTE: Picked up JDK_JAVA_OPTIONS: -Dhapi.fhir.default_page_size=9\nPicked up JAVA_TOOL_OPTIONS: -Dhapi.fhir.default_page_size=6\n"
    );
    parity("config.source.023-c").expect("hapi.fhir.default-page-size", json!(7));
    parity("config.source.023-d").expect("hapi.fhir.default-page-size", json!(6));
    parity("config.source.023-e").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.default-page-size' to java.lang.Integer:",
        &["    Origin: \"hapi.fhir.default_page_size\" from property source \"systemProperties\""],
    );
    // below SPRING_APPLICATION_JSON, above the environment
    parity("config.source.023-f").expect("hapi.fhir.default-page-size", json!(5));
}

// covers: config.source.024
#[test]
fn only_d_tokens_are_read_with_the_jvms_quoting() {
    parity("config.source.024").expect("hapi.fhir.server-address", json!(""));
    parity("config.source.024-b")
        .expect("hapi.fhir.server-address", json!("a b"))
        .expect("hapi.fhir.custom-content-path", json!("c d"));
    parity("config.source.024-c").expect("hapi.fhir.server-address", json!("ab cd"));
}

// covers: config.source.025
#[test]
fn jdk_java_options_has_the_same_quoting() {
    let r = parity("config.source.025");
    r.expect("hapi.fhir.server-address", json!("x y"));
    assert_eq!(
        r.stderr,
        "NOTE: Picked up JDK_JAVA_OPTIONS: -Dhapi.fhir.server_address=\"x y\"\n"
    );
}

// covers: config.source.026
#[test]
fn a_named_file_location_or_import_also_loads_its_profile_variants() {
    for id in [
        "config.source.026",
        "config.source.026-b",
        "config.source.026-c",
        "config.source.026-d",
    ] {
        parity(id).expect("hapi.fhir.default-page-size", json!(6));
    }
}

// covers: config.source.027
#[test]
fn an_extension_hint() {
    parity("config.source.027").expect("hapi.fhir.max-page-size", json!(7));
    parity("config.source.027-b").expect_run_failed(&[
        "java.lang.IllegalStateException: Unable to load config data from 'conf/noext'",
        "Caused by: java.lang.IllegalStateException: File extension is not known to any PropertySourceLoader. If the location is meant to reference a directory, it must end in '/' or File.separator",
    ]);
}

// covers: config.source.028
#[test]
fn a_directory_import() {
    parity("config.source.028")
        .expect("hapi.fhir.max-page-size", json!(8))
        .expect("hapi.fhir.default-page-size", json!(9));
    let only_x = parity("config.source.028-b");
    assert_eq!(
        only_x.doc()["properties"],
        parity("config.source.002").doc()["properties"]
    );
    parity("config.source.028-c")
        .expect_analysis("Config data location 'conf/' does not exist", &[]);
}

// covers: config.source.029
#[test]
fn one_wildcard_in_a_user_location() {
    parity("config.source.029").expect("hapi.fhir.max-page-size", json!(12));
    parity("config.source.029-b").expect("hapi.fhir.max-page-size", json!(14));
    parity("config.source.029-c").expect_run_failed(&[
        "java.lang.IllegalStateException: Location 'file:./conf/*/*/application.yaml' cannot contain multiple wildcards",
    ]);
}

// covers: config.source.030
#[test]
fn the_default_classpath_locations_include_app_extra_classes() {
    let r = parity("config.source.030");
    r.expect("hapi.fhir.default-page-size", json!(41))
        .expect("hapi.fhir.max-page-size", json!(42))
        .expect("hapi.fhir.server-address", json!("foo"))
        .expect("hapi.fhir.openapi-enabled", json!(true));
    // all of these rank below file:./ and file:./config/
    parity("config.source.030-b")
        .expect("hapi.fhir.default-page-size", json!(51))
        .expect("hapi.fhir.max-page-size", json!(52));
}
