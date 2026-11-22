//! config.placeholder: `${...}` resolution, lenient when binding, strict for named reads.

mod common;

use common::parity;
use serde_json::{Value, json};

// covers: config.placeholder.001
#[test]
fn a_placeholder_with_a_default() {
    parity("config.placeholder.001")
        .expect("hapi.fhir.server-address", json!("http://fallback/fhir"));
    parity("config.placeholder.001-b").expect("hapi.fhir.server-address", json!("http://env/fhir"));
}

// covers: config.placeholder.002
#[test]
fn nested_defaults() {
    parity("config.placeholder.002").expect("hapi.fhir.mdm-rules-json-location", json!("nested"));
    parity("config.placeholder.002-b").expect("hapi.fhir.mdm-rules-json-location", json!("y"));
}

// covers: config.placeholder.003
#[test]
fn a_placeholder_names_another_property_by_any_spelling() {
    parity("config.placeholder.003").expect("hapi.fhir.custom-content-path", json!("nested"));
}

// covers: config.placeholder.004
#[test]
fn an_unresolvable_placeholder_in_a_bound_string_stays_literal() {
    parity("config.placeholder.004")
        .expect("hapi.fhir.supported-resource-types", json!(["${nope3}"]));
}

// covers: config.placeholder.005
#[test]
fn an_unresolvable_placeholder_in_a_bound_number_fails_conversion() {
    parity("config.placeholder.005").expect_analysis(
        "Failed to bind properties under 'hapi.fhir.default-page-size' to java.lang.Integer:",
        &[
            "    Property: hapi.fhir.default-page-size",
            "    Value: \"${NOPE}\"",
            "    Reason: failed to convert java.lang.String to java.lang.Integer (caused by java.lang.NumberFormatException: For input string: \"${NOPE}\")",
        ],
    );
}

// covers: config.placeholder.006
#[test]
fn an_escaped_placeholder_is_literal() {
    parity("config.placeholder.006").expect("hapi.fhir.logger.name", json!("${escaped}"));
}

// covers: config.placeholder.007
#[test]
fn a_circular_reference_fails_to_start() {
    let r = parity("config.placeholder.007");
    r.expect_failure();
    r.expect_text("Circular placeholder reference 'hapi.fhir.server_address' in value \"${hapi.fhir.server_address}\"");
}

// covers: config.placeholder.008
#[test]
fn placeholders_resolve_before_comma_splitting() {
    parity("config.placeholder.008").expect("hapi.fhir.cors.allowed-origin", json!(["a", "b"]));
    parity("config.placeholder.008-b").expect("hapi.fhir.cors.allowed-origin", json!(["x", "y"]));
}

// covers: config.placeholder.009
#[test]
fn the_request_logger_templates_are_placeholders_too() {
    // DEBT-0024: HAPI's behaviour, reproduced.
    parity("config.placeholder.009").expect(
        "hapi.fhir.logger.error-format",
        json!("ERROR - GET ${requestUrl}"),
    );
    parity("config.placeholder.009-b").expect(
        "hapi.fhir.logger.error-format",
        json!("ERROR - ${requestVerb} ${requestUrl}"),
    );
}

fn is_digits(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

// covers: config.placeholder.010
#[test]
fn random_values() {
    let r = parity("config.placeholder.010");
    let v = r.get("hapi.fhir.server-address");
    let n: i64 = v
        .as_str()
        .expect("text")
        .parse()
        .expect("a decimal integer");
    assert!((0..10).contains(&n), "random.int(10) gave {n}");
    r.expect("hapi.fhir.default-page-size", json!(5));
    let b = parity("config.placeholder.010-b");
    let uuid = b.get("hapi.fhir.server-address");
    let uuid = uuid.as_str().unwrap();
    assert_eq!(uuid.len(), 36);
    assert!(
        uuid.chars()
            .enumerate()
            .all(|(i, c)| if [8, 13, 18, 23].contains(&i) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            })
    );
    let long = b.get("hapi.fhir.custom-content-path");
    assert!(is_digits(long.as_str().unwrap()) && long.as_str().unwrap().parse::<i64>().is_ok());
    let int = b.get("hapi.fhir.mdm-rules-json-location");
    assert!(int.as_str().unwrap().parse::<i32>().is_ok());
    let value = b.get("hapi.fhir.app-content-path");
    let value = value.as_str().unwrap();
    assert_eq!(value.len(), 32);
    assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
}

// covers: config.placeholder.011
#[test]
fn a_key_read_by_name_resolves_strictly() {
    let cause = "Caused by: org.springframework.util.PlaceholderResolutionException: Could not resolve placeholder 'nope' in value \"${nope}\"";
    for (id, class) in [
        (
            "config.placeholder.011",
            "ca.uhn.fhir.jpa.starter.web.WebAppFilesConfigurer",
        ),
        (
            "config.placeholder.011-b",
            "ca.uhn.fhir.jpa.starter.web.CustomContentFilesConfigurer",
        ),
        (
            "config.placeholder.011-c",
            "ca.uhn.fhir.jpa.starter.mdm.MdmConfig",
        ),
        (
            "config.placeholder.011-d",
            "ca.uhn.fhir.jpa.starter.common.FhirServerConfigCommon.filesystemBinaryStorageSvc",
        ),
        (
            "config.placeholder.011-e",
            "ca.uhn.fhir.jpa.starter.common.StarterJpaConfig.repositoryValidatingInterceptor",
        ),
        (
            "config.placeholder.011-f",
            "ca.uhn.fhir.jpa.starter.ig.ImplementationGuideR4OperationProvider",
        ),
        (
            "config.placeholder.011-g",
            "ca.uhn.fhir.jpa.starter.ips.StarterIpsConfig",
        ),
        (
            "config.placeholder.011-h",
            "ca.uhn.fhir.jpa.starter.mcp.McpServerConfig",
        ),
    ] {
        parity(id).expect_run_failed(&[
            &format!("java.lang.IllegalStateException: Error processing condition on {class}"),
            cause,
        ]);
    }
}

/// The name of the user this test runs as, which is the user of the child process.
fn process_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .expect("USER or USERNAME")
}

// covers: config.placeholder.012
#[test]
fn the_jvms_system_properties_resolve_in_placeholders() {
    let r = parity("config.placeholder.012");
    let hapi_value = common::hapi("config.placeholder.012")["document"]["properties"]["hapi.fhir"]
        ["server-address"]
        .clone();
    assert_eq!(
        hapi_value,
        json!(
            "/tmp|/app|1|Linux|/|main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes|nonroot"
        ),
        "the recorded HAPI value"
    );
    // user.dir, PID and user.name take the running process's values, every other name the
    // image's (design.md section 3.6).
    let expected = format!(
        "/tmp|{}|{}|Linux|/|main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes|{}",
        r.workdir.display(),
        r.pid,
        process_user()
    );
    r.expect("hapi.fhir.server-address", Value::String(expected));
    // The recorded set of system-property names is the set Frevo provides.
    let names = common::record()["system_properties"]
        .as_object()
        .expect("system_properties recorded");
    for n in [
        "java.io.tmpdir",
        "user.dir",
        "PID",
        "os.name",
        "file.separator",
        "loader.path",
        "user.name",
        "user.home",
    ] {
        assert!(names.contains_key(n), "system_properties lacks {n}");
    }
}

// covers: config.placeholder.013
#[test]
fn jvm_identity_properties_resolve_to_the_images_values() {
    let r = parity("config.placeholder.013");
    let sp = &common::record()["system_properties"];
    let expected = format!(
        "{}|{}|{}|{}",
        sp["java.version"].as_str().unwrap(),
        sp["java.vendor"].as_str().unwrap(),
        sp["java.home"].as_str().unwrap(),
        sp["java.class.path"].as_str().unwrap()
    );
    assert_eq!(sp["java.class.path"], json!("/app/main.war"));
    r.expect("hapi.fhir.server-address", Value::String(expected));
}
