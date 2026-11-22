//! config.error: failure reporting, compared on the config-related lines (config.oracle.003).

mod common;

use common::{parity, parse_log_line};

const FAILING: [&str; 6] = [
    "config.error.002",
    "config.dump.006",
    "config.yaml.016",
    "config.args.010",
    "config.gate.005",
    "config.placeholder.011",
];

// covers: config.error.001
#[test]
fn an_invalid_configuration_stops_before_anything_else() {
    for id in FAILING {
        let r = parity(id);
        assert_eq!(r.exit, 1, "{id}: exit status 1");
    }
}

// covers: config.error.002
#[test]
fn the_failure_analysis_output() {
    let r = parity("config.error.002");
    let lines = r.failure_lines();
    let at = lines
        .iter()
        .position(|l| l.contains("o.s.b.d.LoggingFailureAnalysisReporter"))
        .expect("ERROR line");
    let error = parse_log_line(
        r.stdout
            .lines()
            .find(|l| l.contains("LoggingFailureAnalysisReporter"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(error.level, "ERROR");
    assert_eq!(error.message, "");
    let block: Vec<&str> = lines[at + 1..].iter().map(String::as_str).collect();
    assert_eq!(
        block,
        [
            "",
            "***************************",
            "APPLICATION FAILED TO START",
            "***************************",
            "",
            "Description:",
            "",
            "Failed to bind properties under 'hapi.fhir.graphql-enabled' to java.lang.Boolean:",
            "",
            "    Property: hapi.fhir.graphql-enabled",
            "    Value: \"Y\"",
            "    Origin: URL [file:config/application.yaml] - 3:22",
            "    Reason: failed to convert java.lang.String to java.lang.Boolean (caused by java.lang.IllegalArgumentException: Invalid boolean value 'Y')",
            "",
            "Action:",
            "",
            "Update your application's configuration",
        ]
    );
    // the same form for missing resources and locations and a profile rejected while binding
    for id in [
        "config.source.008",
        "config.source.028-c",
        "config.profile.015",
    ] {
        parity(id).expect_text("APPLICATION FAILED TO START");
    }
}

// covers: config.error.003
#[test]
fn the_property_is_named_canonically_whatever_was_written() {
    for id in [
        "config.error.003",
        "config.error.003-b",
        "config.error.003-c",
    ] {
        parity(id).expect_line("    Property: hapi.fhir.graphql-enabled");
    }
}

// covers: config.error.004
#[test]
fn the_origin_of_an_environment_variable_an_argument_and_spring_application_json() {
    parity("config.error.004")
        .expect_line("    Origin: System Environment Property \"HAPI_FHIR_DEFAULT_PAGE_SIZE\"");
    parity("config.error.004-b").expect_line(
        "    Origin: \"hapi.fhir.default_page_size\" from property source \"commandLineArgs\"",
    );
    parity("config.error.004-c")
        .expect_line("    Origin: System Environment Property \"SPRING_APPLICATION_JSON\"");
}

// covers: config.error.005
#[test]
fn the_origin_line_and_column_are_of_the_value() {
    parity("config.error.002").expect_line("    Origin: URL [file:config/application.yaml] - 3:22");
    parity("config.coerce.010").expect_line("    Origin: URL [file:application.yaml] - 1:30");
}

// covers: config.error.006
#[test]
fn several_invalid_values() {
    let r = parity("config.error.006");
    r.expect_failure();
    assert!(
        r.stdout.contains("hapi.fhir.default-page-size")
            || r.stdout.contains("hapi.fhir.graphql-enabled"),
        "the report names at least one of them"
    );
}

// covers: config.error.007
#[test]
fn the_run_failed_output() {
    let r = parity("config.error.007");
    let lines = r.failure_lines();
    assert_eq!(
        lines[0],
        "<ts> ERROR  --- [           main] o.s.boot.SpringApplication               : Application run failed"
    );
    assert_eq!(lines[1], "");
    assert_eq!(
        lines[2],
        "java.lang.IllegalArgumentException: Invalid argument syntax: --=x"
    );
    assert_eq!(
        r.stderr, "",
        "nothing on standard error but the JVM-option notices"
    );
    // a cause chain
    parity("config.source.027-b").expect_run_failed(&[
        "java.lang.IllegalStateException: Unable to load config data from 'conf/noext'",
        "Caused by: java.lang.IllegalStateException: File extension is not known to any PropertySourceLoader. If the location is meant to reference a directory, it must end in '/' or File.separator",
    ]);
}

// covers: config.error.008
#[test]
fn the_config_related_log_lines_around_a_failure() {
    // before logging is initialised: no process id, no application name
    let early = parity("config.error.007");
    let line = early
        .stdout
        .lines()
        .find(|l| l.ends_with("Application run failed"))
        .expect("ERROR line");
    let l = parse_log_line(line).expect("console form");
    assert_eq!((l.pid, l.app, l.thread), ("", "", "           main"));
    assert!(line.ends_with(" ERROR  --- [           main] o.s.boot.SpringApplication               : Application run failed"));

    // during context refresh: HAPI's WARN line, then the INFO line, then the ERROR line
    let bind = parity("config.error.002");
    let logged: Vec<_> = bind.stdout.lines().filter_map(parse_log_line).collect();
    assert_eq!(
        logged.len(),
        3,
        "the WARN, INFO and ERROR lines and no other log line"
    );
    assert_eq!(logged[0].level, "WARN");
    assert_eq!(logged[0].logger, "ConfigServletWebServerApplicationContext");
    assert!(logged[0].message.starts_with(
        "Exception encountered during context initialization - cancelling refresh attempt: "
    ));
    assert_eq!(logged[1].level, "INFO");
    assert_eq!(logged[1].logger, ".s.b.a.l.ConditionEvaluationReportLogger");
    assert_eq!(logged[2].level, "ERROR");
    for l in &logged {
        assert_eq!(l.app, "[hapi-fhir-jpaserver-starter] ");
        assert!(!l.pid.is_empty());
    }
    assert!(bind.stdout.contains(
        "Error starting ApplicationContext. To display the condition evaluation report re-run your application with 'debug' enabled."
    ));

    // not for the NullPointerException of config.gate.005
    let npe = parity("config.gate.005");
    assert!(!npe.stdout.contains("ConditionEvaluationReportLogger"));
    // with a profile active, too
    parity("config.error.009");
}

// covers: config.error.009
#[test]
fn stack_frames_and_non_config_log_lines_are_not_reproduced() {
    for id in [
        "config.error.009",
        "config.error.002",
        "config.yaml.016",
        "config.placeholder.011",
    ] {
        let r = parity(id);
        for line in r.stdout.lines() {
            assert!(
                !line.trim_start().starts_with("at "),
                "{id}: a Java stack frame: {line}"
            );
            assert!(
                !line.contains("Starting "),
                "{id}: a startup-info line: {line}"
            );
            assert!(!line.contains("No active profile set"), "{id}: {line}");
            assert!(!line.contains("The following "), "{id}: {line}");
            assert!(!line.contains("Tomcat"), "{id}: {line}");
        }
    }
}

// covers: config.error.010
#[test]
fn the_property_line_inside_a_map_entry_keeps_the_source_spelling_of_its_parents() {
    parity("config.error.010")
        .expect_line("    Property: hapi.fhir.implementationguides.x.package-contents");
}
