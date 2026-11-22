//! config.relaxed: relaxed property names against the starter's bean properties.

mod common;

use common::{parity, run};
use serde_json::json;

// covers: config.relaxed.001
#[test]
fn kebab_snake_camel_and_upper_case_all_bind() {
    for (id, v) in [
        ("config.relaxed.001", 5),
        ("config.relaxed.001-b", 6),
        ("config.relaxed.001-c", 7),
        ("config.relaxed.001-d", 9),
    ] {
        parity(id).expect("hapi.fhir.default-page-size", json!(v));
    }
}

// covers: config.relaxed.002
#[test]
fn a_dot_is_an_element_separator_not_a_word_separator() {
    parity("config.relaxed.002").expect("hapi.fhir.default-page-size", json!(20));
}

// covers: config.relaxed.003
#[test]
fn upstream_camel_case_names_accept_every_spelling() {
    parity("config.relaxed.003")
        .expect("hapi.fhir.uplifted-refchains-enabled", json!(true))
        .expect(
            "hapi.fhir.user-request-retry-version-conflicts-interceptor-enabled",
            json!(true),
        )
        .expect("hapi.fhir.cdshooks.client-id-header-name", json!("x"))
        .expect("hapi.fhir.cors.allow-credentials", json!(true))
        .expect(
            "hapi.fhir.cr.terminology-server-client-settings.max-retry-count",
            json!(5),
        )
        .expect(
            "hapi.fhir.implementation-guides.a.install-mode",
            json!("STORE_ONLY"),
        )
        .expect("hapi.fhir.subscription.email.start-tls-enable", json!(true));
}

// covers: config.relaxed.004
#[test]
fn names_follow_the_bean_property_not_the_field() {
    // DEBT-0025: the field name is ignored, as in HAPI.
    parity("config.relaxed.004").expect(
        "hapi.fhir.cr.cql.compiler.error-severity-level",
        json!("Info"),
    );
    parity("config.relaxed.004-b").expect(
        "hapi.fhir.cr.cql.compiler.error-severity-level",
        json!("Error"),
    );
}

// covers: config.relaxed.005
#[test]
fn a_higher_source_overrides_a_lower_sources_other_spelling() {
    parity("config.relaxed.005").expect("hapi.fhir.fhir-version", json!("R5"));
}

// covers: config.relaxed.006
#[test]
fn two_spellings_in_one_source_kebab_case_wins() {
    parity("config.relaxed.006").expect("hapi.fhir.default-page-size", json!(4));
}

// covers: config.relaxed.007
#[test]
fn two_non_kebab_spellings_in_one_source_fail_to_start() {
    // DEV-0032: HAPI starts and picks one spelling by HashSet order (its record shows 3 for
    // both cases); Frevo fails, naming both spellings and their origins.
    for (id, a, b) in [
        ("config.relaxed.007", "defaultPageSize", "default_page_size"),
        ("config.relaxed.007-b", "max_page_size", "maxPageSize"),
    ] {
        let r = run(id);
        r.expect_failure();
        assert!(
            r.stdout.contains(a) && r.stdout.contains(b),
            "{id}: the report names both spellings:\n{}",
            r.stdout
        );
        assert!(
            r.stdout.contains("application.yaml"),
            "{id}: the report names their origins"
        );
        assert_eq!(
            common::hapi(id)["exit"],
            json!(0),
            "{id}: HAPI starts (DEV-0032 evidence)"
        );
    }
}

// covers: config.relaxed.008
#[test]
fn map_keys_are_matched_case_insensitively_but_kept_verbatim() {
    let r = parity("config.relaxed.008");
    let tester = r.get("hapi.fhir.tester");
    let mut keys: Vec<&String> = tester.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["Home", "home"]);
    for k in ["Home", "home"] {
        r.expect(&format!("hapi.fhir.tester.{k}.name"), json!("Upper"))
            .expect(
                &format!("hapi.fhir.tester.{k}.server-address"),
                json!("http://localhost:8080/fhir"),
            );
    }
}
