//! config.oracle: the config-roundtrip oracle itself.

mod common;

use common::{config_lines, parity};
use serde_json::json;
use std::collections::BTreeSet;

const IMAGE: &str = "hapiproject/hapi:v8.12.0-2@sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e";

// covers: config.oracle.001
#[test]
fn the_hapi_side_is_the_starters_own_classes_in_the_images_launcher() {
    let rec = common::record();
    assert_eq!(rec["image"], json!(IMAGE));
    assert_eq!(
        rec["diagnostic"],
        json!(false),
        "a diagnostic run is never the record"
    );
    let insp = &rec["inspection"];
    assert_eq!(
        insp["application_files_in_lib"],
        json!([]),
        "no WEB-INF/lib jar carries an application file"
    );
    assert_eq!(insp["problems"], json!([]));
    assert_eq!(
        rec["system_properties"]["loader.path"],
        json!("main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes")
    );
    assert_eq!(
        rec["system_properties"]["java.class.path"],
        json!("/app/main.war")
    );
    // the Dockerfile keeps the image's ENTRYPOINT (no override) and adds only /app/extra-classes
    let dockerfile = std::fs::read_to_string(common::harness().join("Dockerfile")).unwrap();
    assert!(
        !dockerfile.contains("ENTRYPOINT")
            && dockerfile.contains("COPY --from=build /out/ /app/extra-classes/")
    );
    parity("config.oracle.001");
}

// covers: config.oracle.002
#[test]
fn self_test_two_runs_are_identical() {
    let rec = common::record();
    let st = &rec["self_test"];
    assert_eq!(st["runs"], json!(2));
    assert_eq!(
        st["differing"],
        json!([]),
        "cases that differed between two runs of the corpus"
    );
    assert_eq!(
        st["cases"].as_u64().unwrap() as usize,
        common::all_case_ids().len()
    );
}

const SAMPLE: &str = concat!(
    "2026-11-22T19:42:18.070Z  INFO 1 --- [hapi-fhir-jpaserver-starter] [           main] d.frevo.harness.config.ConfigRoundtrip   : Starting ConfigRoundtrip using Java 21\n",
    "2026-11-22T19:42:19.000Z  WARN 1 --- [hapi-fhir-jpaserver-starter] [           main] s.c.a.AnnotationConfigApplicationContext : Exception encountered during context initialization - cancelling refresh attempt: x.Y: z\n",
    "2026-11-22T19:42:19.100Z  INFO 1 --- [hapi-fhir-jpaserver-starter] [           main] .s.b.a.l.ConditionEvaluationReportLogger : \n",
    "\n",
    "Error starting ApplicationContext. To display the condition evaluation report re-run your application with 'debug' enabled.\n",
    "2026-11-22T19:42:19.200Z ERROR 1 --- [hapi-fhir-jpaserver-starter] [           main] o.s.boot.SpringApplication               : Application run failed\n",
    "\n",
    "java.lang.IllegalStateException: boom\n",
    "\tat a.b.C.d(C.java:1)\n",
    "Caused by: java.lang.RuntimeException: inner\n",
    "\t... 5 more\n",
);

// covers: config.oracle.003
#[test]
fn failures_are_compared_on_the_config_related_lines() {
    // the comparator: timestamps and pid masked, WARN/INFO logger masked, frames and the
    // startup-info line stripped
    let (kept, dropped) = config_lines(SAMPLE);
    assert_eq!(
        kept,
        [
            "<ts>  WARN <pid> --- [hapi-fhir-jpaserver-starter] [           main] <logger> : Exception encountered during context initialization - cancelling refresh attempt: x.Y: z",
            "<ts>  INFO <pid> --- [hapi-fhir-jpaserver-starter] [           main] <logger> : ",
            "",
            "Error starting ApplicationContext. To display the condition evaluation report re-run your application with 'debug' enabled.",
            "<ts> ERROR <pid> --- [hapi-fhir-jpaserver-starter] [           main] o.s.boot.SpringApplication               : Application run failed",
            "",
            "java.lang.IllegalStateException: boom",
            "Caused by: java.lang.RuntimeException: inner",
        ]
    );
    assert_eq!(
        dropped.len(),
        3,
        "the startup-info line and two frame lines: {dropped:#?}"
    );
    // and Frevo against HAPI on a failing case
    parity("config.dump.006");
    parity("config.yaml.016");
}

// covers: config.oracle.004
#[test]
fn nothing_is_normalised_but_set_order() {
    let a = json!({"x": [1, 2], "y": null, "z": {"b": true, "a": "s"}});
    let b = json!({"z": {"a": "s", "b": true}, "y": null, "x": [1, 2]});
    assert_eq!(a, b, "object member order is ignored");
    assert_ne!(
        a,
        json!({"x": [2, 1], "y": null, "z": {"b": true, "a": "s"}}),
        "list order is compared"
    );
    assert_ne!(
        a,
        json!({"x": [1, 2], "z": {"b": true, "a": "s"}}),
        "a null is compared"
    );
    parity("config.source.002");
    parity("config.collection.006");
}

/// Scenarios with no configuration of their own: they are judged by inspection, by the
/// comparator, or by the bench, not by a case.
const NO_CASE: [&str; 9] = [
    "config.source.001",
    "config.key.011",
    "config.oracle.002",
    "config.oracle.003",
    "config.oracle.004",
    "config.oracle.005",
    "config.perf.001",
    "config.perf.002",
    "config.perf.003",
];

// covers: config.oracle.005
#[test]
fn the_case_corpus_covers_the_scenarios() {
    let spec = std::fs::read_to_string(
        common::repo().join("openspec/changes/add-config-application-yaml/specs/config/spec.md"),
    )
    .expect("spec delta");
    let scenarios: BTreeSet<String> = spec
        .lines()
        .filter_map(|l| l.strip_prefix("#### Scenario: ["))
        .filter_map(|l| l.split(']').next())
        .map(str::to_string)
        .collect();
    assert_eq!(scenarios.len(), 232);
    let mut tagged = BTreeSet::new();
    for id in common::all_case_ids() {
        for c in common::case(&id).covers {
            assert!(
                scenarios.contains(&c),
                "case {id} covers unknown scenario {c}"
            );
            tagged.insert(c);
        }
        // every case has a recorded HAPI result
        common::hapi(&id);
    }
    for s in &scenarios {
        if !NO_CASE.contains(&s.as_str()) {
            assert!(
                tagged.contains(s),
                "no case in harness/config-roundtrip/cases/ covers {s}"
            );
        }
    }
}
