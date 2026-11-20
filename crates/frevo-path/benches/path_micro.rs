//! The `bench/micro/` FHIRPath workloads of `design.md` section 8 that need no
//! stored resource: `path-compile` and `path-suite-wallclock`.
//!
//! ```text
//! cargo bench -p frevo-path --bench path_micro                 # merges into bench/micro/results.json
//! cargo bench -p frevo-path --bench path_micro -- --out bench/micro/baseline.json
//! ```
//!
//! Their budgets are `path.perf.001` and `path.perf.005`, graded by
//! `crates/frevo-path/tests/path_perf.rs` against the figures this writes; the
//! regression policy over them is `path.perf.006`. The two workloads that
//! evaluate over stored bytes, `path-eval-compiled` and
//! `path-extract-patient`, are `crates/frevo-model/benches/path_bytes.rs`,
//! because this crate may not name `frevo-model` (`path.api.008`).
//!
//! Both workloads go through the **committed oracle harness**
//! (`tests/oracle/`), included here by path so the bench and the tests drive
//! the same seam: `path-compile` is the harness's parse stage
//! (`Expression::parse`) followed by its check stage
//! (`oracle::engine::check_types`, over the pinned R4 package's
//! StructureDefinitions), in the context `path_perf.rs` checks in; and
//! `path-suite-wallclock` is `oracle::run::run_pinned_suite`, the run the
//! FHIRPath gate grades.
//!
//! `path-suite-wallclock` is measured **cold**: each sample is a fresh child
//! process of this binary that loads the suite, the input files and the type
//! package and runs all 1,191 cases once, which is what a per-pull-request gate
//! pays. The recorded figure is the median of [`SUITE_RUNS`] such runs, timed
//! inside the child from its start to the end of the run.

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../tests/oracle/mod.rs"]
mod oracle;

#[path = "../../../bench/micro/record.rs"]
mod record;

use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use oracle::engine::{CaseContext, EngineConfig, check_types};
use oracle::run::CaseStatus;

use record::Row;

const WARMUP: Duration = Duration::from_millis(300);
const MIN_BATCH: Duration = Duration::from_micros(500);
const SAMPLES: usize = 201;
/// Cold suite runs per `path-suite-wallclock` measurement. Odd, so the median
/// is one run.
const SUITE_RUNS: usize = 5;

/// R4's `Patient-name` expression, as `path_perf.rs` states it.
const TYPICAL_SEARCH_PARAMETER: &str = "Patient.name";
/// The compound expression `path_perf.rs` holds within an order of magnitude
/// of the simple one; recorded beside it, not graded.
const COMPOUND_SEARCH_PARAMETER: &str = "Patient.telecom.where(system='phone' or system='email')";

const OWNED: [&str; 2] = ["path-compile", "path-suite-wallclock"];

/// The flag the parent passes to a child that performs one cold suite run.
const CHILD_FLAG: &str = "--one-suite-run";

// ---------------------------------------------------------------------------
// The harness (crates/frevo-model/benches/model_micro.rs's, unchanged)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Stats {
    median_us: f64,
    p10_us: f64,
    p90_us: f64,
    min_us: f64,
    batch: usize,
    samples: usize,
}

fn calibrate(op: &mut dyn FnMut()) -> usize {
    let mut batch = 1usize;
    loop {
        let started = Instant::now();
        for _ in 0..batch {
            op();
        }
        if started.elapsed() >= MIN_BATCH {
            return batch;
        }
        batch *= 2;
    }
}

fn measure(samples: usize, mut op: impl FnMut()) -> Stats {
    let warm_until = Instant::now() + WARMUP;
    while Instant::now() < warm_until {
        op();
    }
    let batch = calibrate(&mut op);
    let mut per_op: Vec<f64> = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        for _ in 0..batch {
            op();
        }
        per_op.push(started.elapsed().as_secs_f64() * 1e6 / batch as f64);
    }
    per_op.sort_by(f64::total_cmp);
    let at = |q: f64| per_op[((per_op.len() - 1) as f64 * q).round() as usize];
    Stats {
        median_us: at(0.5),
        p10_us: at(0.1),
        p90_us: at(0.9),
        min_us: per_op[0],
        batch,
        samples,
    }
}

// ---------------------------------------------------------------------------
// path-compile
// ---------------------------------------------------------------------------

/// The harness's parse stage then its check stage, requiring both to succeed:
/// a compile the engine refused would time at nothing and pass any budget.
fn compile(text: &str, context: &CaseContext<'_>, config: &EngineConfig) {
    let expr = frevo_path::Expression::parse(text)
        .unwrap_or_else(|e| panic!("{text:?} must parse before it is measured: {}", e.message()));
    check_types(&expr, context, config)
        .unwrap_or_else(|e| panic!("{text:?} must check before it is measured: {e}"));
    black_box(expr);
}

fn path_compile(samples: usize) -> Row {
    // `path_perf.rs`'s context: the root and resource type of the 4 KiB
    // `Patient` fixture.
    let context = CaseContext {
        root_resource_type: Some("Patient"),
        resource_type: Some("Patient"),
        ..CaseContext::default()
    };
    let config = EngineConfig::default();
    compile(TYPICAL_SEARCH_PARAMETER, &context, &config);
    let simple = measure(samples, || {
        compile(black_box(TYPICAL_SEARCH_PARAMETER), &context, &config)
    });
    let compound = measure(samples, || {
        compile(black_box(COMPOUND_SEARCH_PARAMETER), &context, &config)
    });
    Row::new("path-compile")
        .str("scenario", "path.perf.001")
        .str("budget", "median <= 20 us")
        .str("expression", TYPICAL_SEARCH_PARAMETER)
        .num("median_us", simple.median_us, 3)
        .num("p10_us", simple.p10_us, 3)
        .num("p90_us", simple.p90_us, 3)
        .num("min_us", simple.min_us, 3)
        .int("batch", simple.batch as u64)
        .int("samples", simple.samples as u64)
        .num("compound_median_us", compound.median_us, 3)
        .str("method", &format!(
            "parse and check per operation; median of {} samples, each a batch of at least {} us, after {} ms warmup; microseconds",
            simple.samples,
            MIN_BATCH.as_micros(),
            WARMUP.as_millis()
        ))
}

// ---------------------------------------------------------------------------
// path-suite-wallclock
// ---------------------------------------------------------------------------

/// The child: one cold run of the pinned suite. Prints the elapsed seconds and
/// the per-status counts on one line for the parent to read.
fn one_suite_run(started: Instant) {
    let (suite, report) = oracle::run::run_pinned_suite();
    let seconds = started.elapsed().as_secs_f64();
    println!(
        "suite-run seconds={seconds:.6} cases={} passed={} failed={} did_not_run={} skipped_for_licence={}",
        suite.case_count(),
        report.count(CaseStatus::Passed),
        report.count(CaseStatus::Failed),
        report.count(CaseStatus::DidNotRun),
        report.count(CaseStatus::SkippedForLicence),
    );
}

fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|kv| kv.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
        .unwrap_or_else(|| panic!("the child's report has no {key}: {line:?}"))
}

fn path_suite_wallclock() -> Row {
    let exe = std::env::current_exe().expect("this binary's path");
    let mut seconds = Vec::with_capacity(SUITE_RUNS);
    let mut last = String::new();
    for _ in 0..SUITE_RUNS {
        let out = std::process::Command::new(&exe)
            .arg(CHILD_FLAG)
            .output()
            .expect("run one cold suite run");
        assert!(
            out.status.success(),
            "the cold suite run failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let line = stdout
            .lines()
            .find(|l| l.starts_with("suite-run "))
            .unwrap_or_else(|| panic!("the child printed no report:\n{stdout}"))
            .to_string();
        seconds.push(
            field(&line, "seconds")
                .parse::<f64>()
                .expect("seconds is a number"),
        );
        last = line;
    }
    seconds.sort_by(f64::total_cmp);
    let count = |key: &str| -> u64 {
        field(&last, key)
            .parse()
            .unwrap_or_else(|_| panic!("{key} is a count"))
    };
    Row::new("path-suite-wallclock")
        .str("scenario", "path.perf.005")
        .str("budget", "<= 10 s on the reference environment")
        .num("median_s", seconds[seconds.len() / 2], 3)
        .num("min_s", seconds[0], 3)
        .num("max_s", seconds[seconds.len() - 1], 3)
        .int("runs", SUITE_RUNS as u64)
        .int("cases", count("cases"))
        .int("passed", count("passed"))
        .int("failed", count("failed"))
        .int("did_not_run", count("did_not_run"))
        .int("skipped_for_licence", count("skipped_for_licence"))
        .str("method", &format!(
            "median of {SUITE_RUNS} cold runs, each a fresh process loading the suite, its inputs and the R4 package and running every case once; seconds, timed in the process"
        ))
}

fn main() {
    let started = Instant::now();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = repo.join("bench/micro/results.json");
    let mut samples = SAMPLES;
    let mut write = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            CHILD_FLAG => {
                one_suite_run(started);
                return;
            }
            "--bench" => {}
            "--out" => out = repo.join(args.next().expect("--out takes a path")),
            "--samples" => {
                samples = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .expect("--samples takes a count")
            }
            "--no-write" => write = false,
            other => panic!("unknown argument {other:?}"),
        }
    }
    if cfg!(debug_assertions) {
        panic!(
            "run the bench/micro workloads in release: cargo bench -p frevo-path --bench path_micro"
        );
    }

    let rows = vec![path_compile(samples), path_suite_wallclock()];
    for r in &rows {
        let members: Vec<String> = r
            .members
            .iter()
            .filter(|(k, _)| !matches!(*k, "method" | "budget"))
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        println!("{:<22} {}", r.name, members.join(" "));
    }
    if write {
        record::write_merged(
            &out,
            &repo,
            "path_micro",
            &|name| OWNED.contains(&name),
            &rows,
        );
        println!("\nwrote {}", out.display());
    }
}
