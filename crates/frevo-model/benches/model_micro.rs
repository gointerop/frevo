//! The `bench/micro/` model workloads of `design.md` section 8.
//!
//! ```text
//! cargo bench -p frevo-model --bench model_micro                 # writes bench/micro/results.json
//! cargo bench -p frevo-model --bench model_micro -- --out bench/micro/baseline.json
//! cargo bench -p frevo-model --bench model_micro -- --diagnose   # also prints a cost breakdown
//! ```
//!
//! Five workloads, six recorded rows (`model-meta-update` is measured at two
//! sizes): `model-passthrough`, `model-lazy-read`, `model-full-parse`,
//! `model-meta-update-4kib`, `model-meta-update-64kib` and
//! `model-canonical-encode`. Their budgets are `model.perf.001` .. `.005`; the
//! regression policy over them is `model.perf.006`. The tests in
//! `crates/frevo-model/tests/model_bytes_lazy_perf.rs` grade the medians this
//! writes, so a number in `bench/micro/results.json` is only ever produced by
//! running this binary -- never typed in.
//!
//! The harness is `std::time::Instant` and nothing else, so the workspace takes
//! no dependency for it. Windows' `Instant` resolves to about 100 ns, which is
//! coarser than the 1 us budget allows to be read off a single operation, so
//! every sample times a **batch** of operations sized to take at least
//! [`MIN_BATCH`], and the per-operation figure is the batch time divided by its
//! length. The recorded figure is the median of [`SAMPLES`] such samples, after
//! [`WARMUP`] of discarded runs.
//!
//! The fixtures are committed beside the results (`bench/micro/fixtures/`) and
//! checked here against the generator the tests use, so the recorded medians
//! and the tests' in-process ratios describe the same documents.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use frevo_model::Resource;

/// Discarded running time before any workload is sampled.
const WARMUP: Duration = Duration::from_millis(300);
/// A batch runs for at least this long, so timer resolution is under 0.1% of it.
const MIN_BATCH: Duration = Duration::from_micros(500);
/// Samples per workload. Odd, so the median is one sample.
const SAMPLES: usize = 201;

const FIXTURE_4KIB: &str = "patient-4kib.json";
const FIXTURE_64KIB: &str = "patient-64kib.json";

/// The metadata update of `model.perf.004` and `model.mutate.007`.
const META_UPDATE: [(&str, &str); 2] = [
    ("Patient.meta.versionId", "2"),
    ("Patient.meta.lastUpdated", "2026-11-08T12:00:00.000Z"),
];

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The generator `model_bytes_lazy_perf.rs` and `model_mutate.rs` build their
/// fixtures with, copied verbatim. The committed fixture files must equal its
/// output; [`check_fixtures`] fails the run when they drift.
fn patient_of_size(target: usize) -> Vec<u8> {
    let head = concat!(
        r##"{"resourceType":"Patient","id":"perf","##,
        r##""meta":{"versionId":"1","lastUpdated":"2026-01-01T00:00:00.000Z"},"##,
        r##""text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">Perf</div>"},"##,
        r##""active":true,"##,
        r##""name":[{"use":"official","family":"Windsor","given":["Peter","James"]}],"##,
        r##""gender":"male","birthDate":"1974-12-25","##,
        r##""address":["##
    );
    let tail = r##"]}"##;
    let mut s = String::from(head);
    let mut i = 0usize;
    loop {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r##"{{"line":["{i} Example Street","Suite {i}"],"city":"Townsville","district":"Example District","state":"NSW","postalCode":"{:04}","country":"AU"}}"##,
            2000 + i % 8000
        ));
        i += 1;
        if s.len() + tail.len() >= target {
            break;
        }
    }
    s.push_str(tail);
    s.into_bytes()
}

/// Read the committed fixtures, failing if either has drifted from the
/// generator.
fn load_fixtures(dir: &Path) -> (Arc<[u8]>, Arc<[u8]>) {
    let mut loaded = Vec::new();
    for (name, target) in [(FIXTURE_4KIB, 4 * 1024), (FIXTURE_64KIB, 64 * 1024)] {
        let path = dir.join("fixtures").join(name);
        let committed = std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "{} cannot be read ({e}); create it with --write-fixtures",
                path.display()
            )
        });
        let generated = patient_of_size(target);
        assert!(
            committed == generated.as_slice(),
            "bench/micro/fixtures/{name} no longer equals the tests' generator \
             output ({} bytes committed, {} generated); regenerate it with \
             --write-fixtures",
            committed.len(),
            generated.len()
        );
        loaded.push(Arc::<[u8]>::from(committed));
    }
    let large = loaded.pop().expect("two fixtures");
    let small = loaded.pop().expect("two fixtures");
    (small, large)
}

fn write_fixtures(dir: &Path) {
    let fixtures = dir.join("fixtures");
    std::fs::create_dir_all(&fixtures).expect("create bench/micro/fixtures");
    for (name, target) in [(FIXTURE_4KIB, 4 * 1024), (FIXTURE_64KIB, 64 * 1024)] {
        std::fs::write(fixtures.join(name), patient_of_size(target)).expect("write a fixture");
    }
}

/// The `model-canonical-encode` input: the same properties the test supplies
/// (`id`, `active`, `gender`, `birthDate`, then `address` repetitions of
/// `city`, `district`, `state`, `country` and `postalCode`), with as many
/// repetitions as it takes for the canonical output to reach 4 KiB, since the
/// scenario names a constructed **4 KiB** `Patient`.
fn constructed_properties() -> Vec<(String, String)> {
    let header = [
        ("Patient.id", "perf"),
        ("Patient.active", "true"),
        ("Patient.gender", "male"),
        ("Patient.birthDate", "1974-12-25"),
    ];
    let mut addresses = 1usize;
    loop {
        let mut properties: Vec<(String, String)> = header
            .iter()
            .map(|(p, v)| ((*p).to_string(), (*v).to_string()))
            .collect();
        for i in 0..addresses {
            for (element, value) in [
                ("city", "Townsville".to_string()),
                ("district", "Example District".to_string()),
                ("state", "NSW".to_string()),
                ("country", "AU".to_string()),
                ("postalCode", format!("{:04}", 2000 + i % 8000)),
            ] {
                properties.push((format!("Patient.address[{i}].{element}"), value));
            }
        }
        if construct(&properties)
            .to_canonical_vec()
            .expect("canonical encode")
            .len()
            >= 4 * 1024
        {
            return properties;
        }
        addresses += 1;
    }
}

fn construct(properties: &[(String, String)]) -> Resource {
    let mut resource = Resource::build("Patient");
    for (path, value) in properties {
        resource.set(path, value).expect("construct the Patient");
    }
    resource
}

// ---------------------------------------------------------------------------
// The harness
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

/// Size a batch so it runs for at least [`MIN_BATCH`].
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
// The workloads
// ---------------------------------------------------------------------------

struct Row {
    name: &'static str,
    scenario: &'static str,
    budget: &'static str,
    fixture_bytes: usize,
    stats: Stats,
    tokens_scanned: Option<u64>,
}

/// Load from stored bytes and serialise unchanged. The buffer arrives owned
/// (from the database driver, in the server), so loading is an `Arc` clone;
/// serialising is one copy of it into the output.
fn passthrough(bytes: &Arc<[u8]>) -> Vec<u8> {
    let resource = Resource::from_stored_untracked(Arc::clone(bytes));
    resource.to_stored_vec().expect("serialise unchanged")
}

fn lazy_read(bytes: &Arc<[u8]>) -> Option<(String, Option<String>)> {
    let resource = Resource::from_stored_untracked(Arc::clone(bytes));
    resource.read("Patient.birthDate").expect("read birthDate")
}

fn full_parse(bytes: &Arc<[u8]>) -> Resource {
    let resource = Resource::from_stored_untracked(Arc::clone(bytes));
    resource.materialise_all().expect("decode completely");
    resource
}

fn meta_update(bytes: &Arc<[u8]>) -> Vec<u8> {
    let mut resource = Resource::from_stored_untracked(Arc::clone(bytes));
    for (path, value) in META_UPDATE {
        resource.set(path, value).expect("set meta");
    }
    resource.to_stored_vec().expect("serialise the update")
}

fn run_workloads(samples: usize, small: &Arc<[u8]>, large: &Arc<[u8]>) -> Vec<Row> {
    let mut rows = Vec::new();

    // model.perf.001
    let probe = Resource::from_stored_untracked(Arc::clone(small));
    let out = probe.to_stored_vec().expect("serialise unchanged");
    assert_eq!(out.as_slice(), &small[..], "a pass-through is the input");
    let tokens = probe.report().tokens_scanned;
    rows.push(Row {
        name: "model-passthrough",
        scenario: "model.perf.001",
        budget: "median <= 1 us, zero tokens scanned",
        fixture_bytes: small.len(),
        stats: measure(samples, || {
            black_box(passthrough(black_box(small)));
        }),
        tokens_scanned: Some(tokens),
    });

    // model.perf.002
    let probe = lazy_read(small);
    assert_eq!(
        probe.as_ref().and_then(|(_, v)| v.as_deref()),
        Some("1974-12-25"),
        "the lazy read finds birthDate"
    );
    let probe = Resource::from_stored_untracked(Arc::clone(small));
    probe.read("Patient.birthDate").expect("read birthDate");
    rows.push(Row {
        name: "model-lazy-read",
        scenario: "model.perf.002",
        budget: "median <= 2 us and <= 10% of model-full-parse",
        fixture_bytes: small.len(),
        stats: measure(samples, || {
            black_box(lazy_read(black_box(small)));
        }),
        tokens_scanned: Some(probe.report().tokens_scanned),
    });

    // model.perf.003
    let probe = full_parse(small);
    rows.push(Row {
        name: "model-full-parse",
        scenario: "model.perf.003",
        budget: "median <= 20 us",
        fixture_bytes: small.len(),
        stats: measure(samples, || {
            black_box(full_parse(black_box(small)));
        }),
        tokens_scanned: Some(probe.report().tokens_scanned),
    });

    // model.perf.004, at both sizes.
    for (name, bytes) in [
        ("model-meta-update-4kib", small),
        ("model-meta-update-64kib", large),
    ] {
        let out = meta_update(bytes);
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains(r#""versionId":"2""#) && text.contains("2026-11-08T12:00:00.000Z"),
            "{name}: the update is in the output"
        );
        let mut probe = Resource::from_stored_untracked(Arc::clone(bytes));
        for (path, value) in META_UPDATE {
            probe.set(path, value).expect("set meta");
        }
        rows.push(Row {
            name,
            scenario: "model.perf.004",
            budget: "the 4 KiB and 64 KiB medians within 25%",
            fixture_bytes: bytes.len(),
            stats: measure(samples, || {
                black_box(meta_update(black_box(bytes)));
            }),
            tokens_scanned: Some(probe.report().tokens_scanned),
        });
    }

    // model.perf.005: construction is set-up, serialisation is measured.
    let properties = constructed_properties();
    let constructed = construct(&properties);
    let encoded = constructed.to_canonical_vec().expect("canonical encode");
    rows.push(Row {
        name: "model-canonical-encode",
        scenario: "model.perf.005",
        budget: "median <= 25 us",
        fixture_bytes: encoded.len(),
        stats: measure(samples, || {
            black_box(
                black_box(&constructed)
                    .to_canonical_vec()
                    .expect("canonical encode"),
            );
        }),
        tokens_scanned: None,
    });

    rows
}

/// Where the 64 KiB metadata update's time goes, step by step, against a bare
/// copy of the same number of bytes. Printed, not recorded: it explains
/// `model.perf.004`, it is not graded.
fn diagnose(samples: usize, small: &Arc<[u8]>, large: &Arc<[u8]>) {
    println!("\nmodel-meta-update breakdown (median us per operation)");
    println!(
        "{:<44} {:>10} {:>10}",
        "step (cumulative)", "4 KiB", "64 KiB"
    );
    type Step = fn(&Arc<[u8]>) -> usize;
    let steps: [(&str, Step); 8] = [
        ("load (Arc clone)", |b| {
            Resource::from_stored_untracked(Arc::clone(b))
                .stored_len()
                .unwrap_or(0)
        }),
        ("+ set versionId", |b| {
            let mut r = Resource::from_stored_untracked(Arc::clone(b));
            r.set(META_UPDATE[0].0, META_UPDATE[0].1).expect("set");
            0
        }),
        ("+ set lastUpdated", |b| {
            let mut r = Resource::from_stored_untracked(Arc::clone(b));
            for (p, v) in META_UPDATE {
                r.set(p, v).expect("set");
            }
            0
        }),
        ("+ to_stored_vec (the workload)", |b| meta_update(b).len()),
        ("  same, write_to a reused buffer", |b| {
            thread_local! {
                static BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
            }
            let mut r = Resource::from_stored_untracked(Arc::clone(b));
            for (p, v) in META_UPDATE {
                r.set(p, v).expect("set");
            }
            BUF.with(|buf| {
                let mut buf = buf.borrow_mut();
                buf.clear();
                r.write_to(&mut *buf).expect("write");
                buf.len()
            })
        }),
        ("bare: Vec::from(slice) of the document", |b| {
            black_box(b.to_vec()).len()
        }),
        ("bare: test adapter's owned() copy", |b| {
            let owned: Arc<[u8]> = Arc::from(b.to_vec().into_boxed_slice());
            owned.len()
        }),
        ("test adapter: owned() + workload", |b| {
            let owned: Arc<[u8]> = Arc::from(b.to_vec().into_boxed_slice());
            meta_update(&owned).len()
        }),
    ];
    for (label, step) in steps {
        let s = measure(samples, || {
            black_box(step(black_box(small)));
        });
        let l = measure(samples, || {
            black_box(step(black_box(large)));
        });
        println!("{label:<44} {:>10.3} {:>10.3}", s.median_us, l.median_us);
    }
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

fn git(args: &[&str], repo: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the system clock, without a date dependency.
fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Howard Hinnant's civil_from_days.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn render(rows: &[Row], repo: &Path) -> String {
    let commit = git(&["rev-parse", "HEAD"], repo).unwrap_or_else(|| "unknown".to_string());
    let dirty = git(
        &[
            "status",
            "--porcelain",
            "--",
            "crates",
            "bench/micro",
            ":(exclude)bench/micro/results.json",
            ":(exclude)bench/micro/baseline.json",
        ],
        repo,
    )
    .map(|s| !s.is_empty())
    .unwrap_or(true);
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    let mut s = String::new();
    s.push_str("{\n");
    let _ = writeln!(
        s,
        "  \"label\": \"frevo-model bench/micro (crates/frevo-model/benches/model_micro.rs)\","
    );
    let _ = writeln!(s, "  \"profile\": \"release\",");
    let _ = writeln!(s, "  \"commit\": {},", json_string(&commit));
    let _ = writeln!(s, "  \"worktree_dirty\": {dirty},");
    let _ = writeln!(s, "  \"generated_at\": {},", json_string(&utc_now()));
    let _ = writeln!(
        s,
        "  \"environment\": {{ \"os\": {}, \"arch\": {}, \"cpus\": {cpus} }},",
        json_string(std::env::consts::OS),
        json_string(std::env::consts::ARCH)
    );
    let _ = writeln!(
        s,
        "  \"method\": \"median of {SAMPLES} samples, each a batch of at least {} us, after {} ms warmup; per-operation figures in microseconds\",",
        MIN_BATCH.as_micros(),
        WARMUP.as_millis()
    );
    s.push_str("  \"workloads\": [\n");
    for (i, row) in rows.iter().enumerate() {
        let st = &row.stats;
        let _ = write!(
            s,
            "    {{ \"name\": {}, \"scenario\": {}, \"budget\": {}, \"fixture_bytes\": {}, \
             \"median_us\": {:.3}, \"p10_us\": {:.3}, \"p90_us\": {:.3}, \"min_us\": {:.3}, \
             \"batch\": {}, \"samples\": {}",
            json_string(row.name),
            json_string(row.scenario),
            json_string(row.budget),
            row.fixture_bytes,
            st.median_us,
            st.p10_us,
            st.p90_us,
            st.min_us,
            st.batch,
            st.samples
        );
        if let Some(tokens) = row.tokens_scanned {
            let _ = write!(s, ", \"tokens_scanned\": {tokens}");
        }
        s.push_str(" }");
        s.push_str(if i + 1 < rows.len() { ",\n" } else { "\n" });
    }
    s.push_str("  ]\n}\n");
    s
}

fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let micro = repo.join("bench/micro");
    let mut out = micro.join("results.json");
    let mut samples = SAMPLES;
    let mut write = true;
    let mut diagnose_too = false;
    let mut diagnose_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // cargo bench passes this to every harness = false target.
            "--bench" => {}
            "--out" => out = repo.join(args.next().expect("--out takes a path")),
            "--samples" => {
                samples = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .expect("--samples takes a count")
            }
            "--no-write" => write = false,
            "--diagnose" => diagnose_too = true,
            "--diagnose-only" => diagnose_only = true,
            "--write-fixtures" => {
                write_fixtures(&micro);
                println!("wrote {}", micro.join("fixtures").display());
                return;
            }
            other => panic!("unknown argument {other:?}"),
        }
    }
    if cfg!(debug_assertions) {
        panic!("run the bench/micro workloads in release: cargo bench -p frevo-model");
    }
    let (small, large) = load_fixtures(&micro);
    if diagnose_only {
        diagnose(samples.min(51), &small, &large);
        return;
    }

    let rows = run_workloads(samples, &small, &large);
    println!(
        "{:<26} {:>8} {:>10} {:>10} {:>10} {:>8}  budget",
        "workload", "bytes", "median us", "p10 us", "p90 us", "tokens"
    );
    for row in &rows {
        println!(
            "{:<26} {:>8} {:>10.3} {:>10.3} {:>10.3} {:>8}  {} ({})",
            row.name,
            row.fixture_bytes,
            row.stats.median_us,
            row.stats.p10_us,
            row.stats.p90_us,
            row.tokens_scanned
                .map(|t| t.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.budget,
            row.scenario
        );
    }
    if write {
        std::fs::write(&out, render(&rows, &repo)).expect("write the results");
        println!("\nwrote {}", out.display());
    }
    if diagnose_too {
        diagnose(samples.min(51), &small, &large);
    }
}
