//! config.key: every starter key and its defaults (keys.md is normative for .001, .003,
//! .004 and .011).

mod common;

use common::parity;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn keys_md() -> String {
    std::fs::read_to_string(
        common::repo().join("openspec/changes/add-config-application-yaml/keys.md"),
    )
    .expect("keys.md")
}

/// `| `path` | `json` |` rows of one keys.md section.
fn two_column_rows(section: &str) -> Vec<(String, Value)> {
    let text = keys_md();
    let start = text
        .find(&format!("## {section}"))
        .unwrap_or_else(|| panic!("keys.md section {section}"));
    let rest = &text[start + 3..];
    let body = &rest[..rest.find("\n## ").unwrap_or(rest.len())];
    let mut out = Vec::new();
    for line in body.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() == 4 && cells[1].starts_with('`') && cells[2].starts_with('`') {
            let path = cells[1].trim_matches('`').to_string();
            let value = serde_json::from_str(cells[2].trim_matches('`'))
                .unwrap_or_else(|e| panic!("keys.md {path}: {e}"));
            out.push((path, value));
        }
    }
    assert!(!out.is_empty(), "keys.md section {section} has rows");
    out
}

/// Flattens the `properties` member to `prefix.path` leaves (map keys joined with `.`).
fn flatten(doc: &Value) -> BTreeMap<String, Value> {
    fn walk(v: &Value, path: String, out: &mut BTreeMap<String, Value>) {
        match v {
            Value::Object(m) if !m.is_empty() => {
                for (k, x) in m {
                    walk(x, format!("{path}.{k}"), out);
                }
            }
            _ => {
                out.insert(path, v.clone());
            }
        }
    }
    let mut out = BTreeMap::new();
    for (prefix, v) in doc["properties"].as_object().unwrap() {
        walk(v, prefix.clone(), &mut out);
    }
    out
}

// covers: config.key.001
#[test]
fn app_properties_class_defaults() {
    // Finding O-6: exactly as the scenario states it (no file at all), HAPI fails with
    // config.gate.005's NullPointerException; the version is set on the command line.
    parity("config.key.001").expect_failure();
    let r = parity("config.key.001-v");
    for (path, default) in two_column_rows("1.") {
        r.expect(&path, default);
    }
    r.expect("hapi.fhir.openapi-enabled", json!(false))
        .expect("hapi.fhir.narrative-enabled", json!(true))
        .expect("hapi.fhir.client-id-strategy", json!("ALPHANUMERIC"))
        .expect("hapi.fhir.server-id-strategy", Value::Null)
        .expect("hapi.fhir.reuse-cached-search-results-millis", json!(60000));
}

// covers: config.key.002
#[test]
fn nested_class_defaults() {
    let r = parity("config.key.002");
    r.expect("hapi.fhir.cors.allow-credentials", json!(false))
        .expect("hapi.fhir.cors.allowed-origin", json!(["*"]))
        .expect(
            "hapi.fhir.cors.allowed-headers",
            json!([
                "Origin",
                "Accept",
                "Content-Type",
                "Authorization",
                "Cache-Control",
                "If-Match",
                "If-None-Match",
                "x-fhir-starter",
                "X-Requested-With",
                "Prefer"
            ]),
        )
        .expect(
            "hapi.fhir.cors.exposed-headers",
            json!([
                "Location",
                "Content-Location",
                "ETag",
                "Date",
                "Retry-After",
                "X-Correlation-Id",
                "X-Progress",
                "X-Request-Id"
            ]),
        )
        .expect(
            "hapi.fhir.cors.allowed-methods",
            json!(["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH", "HEAD"]),
        )
        .expect("hapi.fhir.logger.name", json!("fhirtest.access"))
        .expect(
            "hapi.fhir.logger.error-format",
            json!("ERROR - ${requestVerb} ${requestUrl}"),
        )
        .expect("hapi.fhir.logger.log-exceptions", json!(true))
        .expect(
            "hapi.fhir.tester.t.refuse-to-fetch-third-party-urls",
            json!(true),
        )
        .expect("hapi.fhir.tester.t.fhir-version", json!("R4"))
        .expect("hapi.fhir.validation.requests-enabled", json!(false))
        .expect("hapi.fhir.validation.responses-enabled", json!(false))
        .expect(
            "hapi.fhir.partitioning.request-tenant-partitioning-mode",
            json!(true),
        )
        .expect("hapi.fhir.partitioning.default-partition-id", json!(0))
        .expect(
            "hapi.fhir.partitioning.patient-id-partitioning-mode",
            json!(false),
        )
        .expect("hapi.fhir.subscription.polling-interval-ms", Value::Null)
        .expect("hapi.fhir.subscription.resthook-enabled", json!(false))
        .expect("hapi.fhir.elasticsearch.index-prefix", json!(""))
        .expect("hapi.fhir.client.socket-timeout", json!(10000))
        .expect("hapi.fhir.client.connect-timeout", json!(10000))
        .expect("hapi.fhir.client.connection-request-timeout", json!(10000))
        .expect("hapi.fhir.client.connection-ttl", json!(5000))
        .expect("hapi.fhir.client.pool-max-total", json!(20))
        .expect("hapi.fhir.client.pool-max-per-route", json!(20))
        .expect("hapi.fhir.remote-terminology-service.r.system", Value::Null)
        .expect("hapi.fhir.remote-terminology-service.r.url", json!("u"));
    let format = r.get("hapi.fhir.logger.format");
    assert_eq!(
        format.as_str().map(str::len),
        Some(407),
        "the 407-character default of AppProperties.java:970-971"
    );
}

// covers: config.key.003
#[test]
fn clinical_reasoning_and_cds_hooks_class_defaults() {
    let r = parity("config.key.001-v");
    for (path, default) in two_column_rows("2.") {
        r.expect(&path, default);
    }
    r.expect(
        "hapi.fhir.cr.terminology-server-client-settings",
        json!({"max-retry-count": 3, "retry-interval-millis": 1000, "timeout-seconds": 30, "socket-timeout": 60,
               "crmi-version": "1.0.0", "expansions-per-page": 1000, "max-expansion-pages": 1000,
               "tx-resource-mode": "AUTO", "max-tx-resource-code-system-concepts": 1000}),
    );
}

// covers: config.key.004
#[test]
fn the_packaged_application_yaml_differences_from_the_class_defaults() {
    let defaults = flatten(parity("config.key.001-v").doc());
    let packaged = flatten(parity("config.key.004").doc());
    let section3: Vec<String> = {
        let text = keys_md();
        let s = &text[text.find("## 3.").unwrap()..text.find("## 4.").unwrap()];
        s.lines()
            .filter_map(|l| l.strip_prefix("| `"))
            .filter_map(|l| l.split('`').next())
            .map(str::to_string)
            .collect()
    };
    let mut paths: Vec<&String> = defaults.keys().chain(packaged.keys()).collect();
    paths.sort();
    paths.dedup();
    for p in paths {
        if defaults.get(p) != packaged.get(p) {
            assert!(
                section3
                    .iter()
                    .any(|s| p == s || p.starts_with(&format!("{s}."))),
                "{p} differs from the class default but keys.md section 3 does not list it"
            );
        }
    }
    let r = parity("config.key.004");
    r.expect("hapi.fhir.openapi-enabled", json!(true))
        .expect("hapi.fhir.narrative-enabled", json!(false))
        .expect("hapi.fhir.cr.cql.data.profile-mode", json!("DECLARED"))
        .expect(
            "hapi.fhir.cr.cql.terminology.code-lookup-mode",
            json!("USE_VALIDATE_CODE_OPERATION"),
        );
}

// covers: config.key.005
#[test]
fn every_bound_key_round_trips_from_yaml() {
    let r = parity("all-keys.yaml");
    r.expect("hapi.fhir.subscription.email.password", json!("******"));
}

// covers: config.key.006
#[test]
fn every_bound_key_round_trips_from_the_environment() {
    parity("all-keys.env");
}

// covers: config.key.007
#[test]
fn every_bound_key_accepts_its_kebab_case_spelling() {
    let kebab = parity("all-keys-kebab.yaml");
    let yaml = parity("all-keys.yaml");
    // Finding O-13 (tasks/blockers.md): the properties are equal, as the scenario says; the
    // reads and gates are not, in HAPI too, because the exact-name reads (`fhir_version`,
    // `mdm_enabled`, ...) do not see a kebab-case spelling (config.read.002, DEBT-0022).
    assert_eq!(
        kebab.doc()["properties"],
        yaml.doc()["properties"],
        "the kebab-case properties equal config.key.005's"
    );
    assert_eq!(kebab.doc()["profiles"], yaml.doc()["profiles"]);
    kebab.expect_read("hapi.fhir.fhir_version", json!("R4"));
    yaml.expect_read("hapi.fhir.fhir_version", json!("R5"));
}

// covers: config.key.008
#[test]
fn zero_cache_reuse_milliseconds_means_absent() {
    parity("config.key.008").expect("hapi.fhir.reuse-cached-search-results-millis", Value::Null);
    parity("config.key.008-b").expect("hapi.fhir.reuse-cached-search-results-millis", json!(1));
}

// covers: config.key.009
#[test]
fn implementation_guide_entries() {
    parity("config.key.009").expect(
        "hapi.fhir.implementation-guides.x",
        json!({"name": "x", "package-url": null, "version": null, "install-mode": null, "install-resource-types": [],
               "fetch-dependencies": false, "reload-existing": true, "dependency-excludes": [],
               "additional-resource-folders": [], "version-policy": "MULTI_VERSION", "dry-run": false,
               "overwrite-content-not-present-code-systems": false, "package-contents": null}),
    );
    parity("config.key.009-b")
        .expect(
            "hapi.fhir.implementation-guides.x.install-resource-types",
            json!(["CodeSystem", "ValueSet"]),
        )
        .expect(
            "hapi.fhir.implementation-guides.x.fetch-dependencies",
            json!(true),
        )
        .expect(
            "hapi.fhir.implementation-guides.x.reload-existing",
            json!(false),
        );
}

// covers: config.key.010
#[test]
fn bound_keys_the_starter_never_reads() {
    parity("config.key.010")
        .expect("hapi.fhir.cr-enabled", json!(true))
        .expect(
            "hapi.fhir.install-additional-resources-from-ig-folders",
            json!(["f"]),
        )
        .expect("hapi.fhir.subscription.email.auth", json!(true))
        .expect("hapi.fhir.subscription.email.quit-wait", json!(true))
        .expect(
            "hapi.fhir.subscription.email.start-tls-required",
            json!(true),
        )
        .expect("hapi.fhir.cr.cql.compiler.translator-format", json!("XML"))
        .expect("hapi.fhir.cr.cql.use-embedded-libraries", json!(false));
    let text = keys_md();
    for k in [
        "cr_enabled",
        "install_additional_resources_from_ig_folders",
        "subscription.email.auth",
        "translator_format",
        "use_embedded_libraries",
    ] {
        let row = text.lines().find(|l| {
            l.contains(&format!("`hapi.fhir.{k}`"))
                || l.contains(&format!("`hapi.fhir.cr.cql.compiler.{k}`"))
                || l.contains(&format!("`hapi.fhir.cr.cql.{k}`"))
        });
        let row = row.unwrap_or_else(|| panic!("keys.md has no row for {k}"));
        assert!(
            row.contains("bound, never read"),
            "keys.md records that HAPI reads none of them: {row}"
        );
    }
}

// covers: config.key.011
#[test]
fn every_inventory_row_is_accounted_for() {
    // Finding O-11 (tasks/blockers.md): the scenario counts 209 rows; docs/inventory.md was
    // corrected on main by #39 (task 6) and now has 220, the 209 of keys.md section 5 less
    // its three `not a key` rows plus the fourteen keys of section 6. The test checks that
    // every current row is accounted for in keys.md, and that section 5 still holds the 209
    // rows the scenario names, each with a disposition, scenarios and a wiring change.
    let inventory = std::fs::read_to_string(common::repo().join("docs/inventory.md"))
        .expect("docs/inventory.md");
    let section = &inventory[inventory.find("## Configuration keys").unwrap()
        ..inventory.find("## Operations").unwrap()];
    let rows: Vec<String> = section
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .map(str::to_string)
        .collect();
    let keys = keys_md();
    let s5 = &keys[keys.find("## 5.").unwrap()..keys.find("## 6.").unwrap()];
    let s6 = &keys[keys.find("## 6.").unwrap()..keys.find("## 7.").unwrap()];
    let s5_rows: Vec<&str> = s5
        .lines()
        .filter(|l| {
            l.split('|')
                .nth(1)
                .is_some_and(|c| c.trim().parse::<u32>().is_ok())
        })
        .collect();
    assert_eq!(
        s5_rows.len(),
        209,
        "keys.md section 5 lists the 209 inventory rows the scenario names"
    );
    let dispositions = [
        "bound",
        "bound, never read",
        "bound, setter-less",
        "map entry",
        "read only",
        "not bound",
        "not a key",
    ];
    for row in &s5_rows {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert!(
            dispositions.contains(&cells[3]),
            "{}: disposition {:?}",
            cells[2],
            cells[3]
        );
        assert!(
            !cells[5].is_empty(),
            "{}: the scenarios that cover it",
            cells[2]
        );
        assert!(
            !cells[6].is_empty(),
            "{}: the change that wires it, or none",
            cells[2]
        );
    }
    for key in rows {
        let quoted = format!("`{key}`");
        let in_s5 = s5_rows
            .iter()
            .any(|l| l.split('|').nth(2).map(str::trim) == Some(quoted.as_str()));
        let in_s6 = s6.contains(&quoted)
            || key
                .strip_prefix("hapi.fhir.implementationguides.<id>.")
                .is_some_and(|field| {
                    s6.lines().any(|l| {
                        l.contains("`hapi.fhir.implementationguides.<id>.")
                            && l.contains(&format!("`.{field}`"))
                    }) || s6.contains(&format!("`hapi.fhir.implementationguides.<id>.{field}`"))
                });
        assert!(
            in_s5 || in_s6,
            "inventory key {key} is in neither keys.md section 5 nor section 6"
        );
    }
}

// covers: config.key.012
#[test]
fn package_contents_binds_bytes() {
    parity("config.key.012")
        .expect(
            "hapi.fhir.implementation-guides.x.package-contents",
            json!([65, 66]),
        )
        .expect_gate("implementation-guides", true);
    for (id, v) in [("config.error.010", "abc"), ("config.key.012-b", "QUI=")] {
        parity(id).expect_line(&format!(
            "    Reason: failed to convert java.lang.String to byte (caused by java.lang.NumberFormatException: For input string: \"{v}\")"
        ));
    }
}
