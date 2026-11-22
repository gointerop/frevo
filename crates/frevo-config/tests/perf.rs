//! config.perf: the load and read budgets of design.md section 9. These scenarios have no
//! HAPI counterpart; they assert the spec text against the bench records.

mod common;

use serde_json::Value;

fn workloads_yaml() -> String {
    std::fs::read_to_string(common::repo().join("bench/micro/workloads.yaml"))
        .expect("bench/micro/workloads.yaml")
}

fn row(file: &str, name: &str) -> Value {
    let p = common::repo().join("bench/micro").join(file);
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(&p).expect("bench record")).expect("JSON");
    v["workloads"]
        .as_array()
        .expect("workloads")
        .iter()
        .find(|w| w["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("bench/micro/{file} has no `{name}` row (task 22 not done)"))
}

/// The workload block of `name` in workloads.yaml, from `- name: <name>` to the next `- name:`.
fn workload_block(name: &str) -> String {
    let y = workloads_yaml();
    let start = y.find(&format!("- name: {name}\n")).unwrap_or_else(|| {
        panic!("bench/micro/workloads.yaml has no `{name}` workload (task 22 not done)")
    });
    let rest = &y[start + 1..];
    let end = rest
        .find("\n  - name: ")
        .map(|e| e + 1)
        .unwrap_or(rest.len());
    y[start..start + 1 + end].to_string()
}

// covers: config.perf.001
#[test]
fn loading_the_packaged_configuration_is_within_5_ms() {
    let block = workload_block("config-load-packaged");
    assert!(block.contains("scenario: config.perf.001"), "{block}");
    assert!(block.contains("fixtures/config-all-keys.yaml"), "{block}");
    let fixture = common::repo().join("bench/micro/fixtures/config-all-keys.yaml");
    let text =
        std::fs::read_to_string(&fixture).expect("bench/micro/fixtures/config-all-keys.yaml");
    assert_eq!(
        text.lines().count(),
        120,
        "the fixture is the 120-line all-keys file of design.md section 9"
    );
    let r = row("results.json", "config-load-packaged");
    let median = r["median_us"].as_f64().expect("median_us");
    assert!(
        median <= 5000.0,
        "config-load-packaged median {median} us exceeds 5 ms"
    );
}

// covers: config.perf.002
#[test]
fn reading_a_bound_value_is_within_5_ns_and_allocates_nothing() {
    let block = workload_block("config-get");
    assert!(block.contains("scenario: config.perf.002"), "{block}");
    let r = row("results.json", "config-get");
    let median = r["median_us"].as_f64().expect("median_us");
    assert!(
        median <= 0.005,
        "config-get median {median} us exceeds 5 ns"
    );
    assert_eq!(
        r["allocations"],
        Value::from(0),
        "config-get must perform no allocation"
    );
}

// covers: config.perf.003
#[test]
fn neither_workload_regresses_more_than_10_percent_against_the_baseline() {
    for name in ["config-load-packaged", "config-get"] {
        let now = row("results.json", name)["median_us"]
            .as_f64()
            .expect("median");
        let base = row("baseline.json", name)["median_us"]
            .as_f64()
            .expect("baseline median");
        assert!(
            now <= base * 1.10,
            "{name}: median {now} us is more than 10% over the baseline {base} us"
        );
    }
}
