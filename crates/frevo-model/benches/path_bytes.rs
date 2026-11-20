//! The `bench/micro/` FHIRPath workloads of `design.md` section 8 that evaluate
//! over a stored resource: `path-eval-compiled` and `path-extract-patient`.
//!
//! ```text
//! cargo bench -p frevo-model --bench path_bytes                 # merges into bench/micro/results.json
//! cargo bench -p frevo-model --bench path_bytes -- --out bench/micro/baseline.json
//! ```
//!
//! Their budgets are `path.perf.002` .. `.004`, graded by
//! `crates/frevo-path/tests/path_perf.rs` against the medians this writes; the
//! regression policy over them is `path.perf.006`. The other two path
//! workloads, `path-compile` and `path-suite-wallclock`, need no stored
//! resource and are `crates/frevo-path/benches/path_micro.rs`.
//!
//! ## Why a `frevo-path` workload is measured from this crate
//!
//! `path.perf.004` states the extraction workload "runs over a byte-backed
//! resource", and the only byte-backed resource is this crate's
//! [`frevo_model::ResourceNode`]. `frevo-path` may not name `frevo-model`
//! (`path.api.008`), so a bench that evaluates over stored bytes can only live
//! on this side of the edge. Both workloads run over the production pieces the
//! write path will use -- the generated R4 type tables
//! ([`frevo_model::R4Model`]), the span-based node, and
//! [`frevo_path::ServerHostServices`] with an application context -- rather
//! than over the oracle harness's test-only tree.
//!
//! ## The fixture
//!
//! `patient-example.json` from the pinned `fhir-test-cases` tree
//! (`oracles/PINS.md`), which is the JSON form of the `patient-example.xml`
//! the tests' in-process halves measure over. It is canonicalised once with
//! [`frevo_model::Resource::to_canonical_vec`] before anything is timed,
//! because canonical compact JSON is what the server stores and what an
//! extraction reads.
//!
//! The harness is the one `model_micro.rs` uses: `std::time::Instant`, batches
//! of at least [`MIN_BATCH`], the median of [`SAMPLES`] after [`WARMUP`].

#![forbid(unsafe_code)]

#[path = "../../../bench/micro/record.rs"]
mod record;

use std::collections::BTreeSet;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use frevo_model::{R4, R4Model, Resource};
use frevo_path::{Evaluator, Expression, Model, ParseOptions, ServerHostServices, Value};

use record::Row;

const WARMUP: Duration = Duration::from_millis(300);
const MIN_BATCH: Duration = Duration::from_micros(500);
const SAMPLES: usize = 201;

/// The pinned input, repo-relative.
const FIXTURE: &str = "oracles/fhir-test-cases/fetched/r4/examples/patient-example.json";

/// `path-eval-compiled`'s expression: R4's `Patient-name`, as in
/// `path_perf.rs`.
const TYPICAL_SEARCH_PARAMETER: &str = "Patient.name";

/// `path-eval-compiled` runs this many evaluations of one parsed expression
/// per sample batch unit, as the scenario states.
const EVALUATIONS: usize = 1000;

/// Every enabled `Patient` search-parameter expression, the list
/// `crates/frevo-path/tests/path_perf.rs` states, in the same order.
const PATIENT_SEARCH_PARAMETERS: [&str; 25] = [
    "Patient.active",
    "Patient.address",
    "Patient.address.city",
    "Patient.address.country",
    "Patient.address.postalCode",
    "Patient.address.state",
    "Patient.address.use",
    "Patient.birthDate",
    "Patient.deceased.exists() and Patient.deceased != false",
    "Patient.deceased.as(dateTime)",
    "Patient.telecom.where(system='email')",
    "Patient.name.family",
    "Patient.gender",
    "Patient.generalPractitioner",
    "Patient.identifier",
    "Patient.link.other",
    "Patient.name",
    "Patient.managingOrganization",
    "Patient.telecom.where(system='phone')",
    "Patient.telecom.where(system='phone' or system='email')",
    "Patient.name | Patient.contact.name",
    "Patient.telecom.value",
    "Patient.address.line",
    "Patient.communication.language",
    "Patient.maritalStatus",
];

const OWNED: [&str; 2] = ["path-eval-compiled", "path-extract-patient"];

// ---------------------------------------------------------------------------
// The harness (model_micro.rs's, unchanged)
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

fn row(name: &str, scenario: &str, budget: &str, fixture_bytes: usize, st: &Stats) -> Row {
    Row::new(name)
        .str("scenario", scenario)
        .str("budget", budget)
        .int("fixture_bytes", fixture_bytes as u64)
        .num("median_us", st.median_us, 3)
        .num("p10_us", st.p10_us, 3)
        .num("p90_us", st.p90_us, 3)
        .num("min_us", st.min_us, 3)
        .int("batch", st.batch as u64)
        .int("samples", st.samples as u64)
        .str(
            "method",
            &format!(
                "median of {} samples, each a batch of at least {} us, after {} ms warmup; per-operation figures in microseconds",
                st.samples,
                MIN_BATCH.as_micros(),
                WARMUP.as_millis()
            ),
        )
}

// ---------------------------------------------------------------------------
// The pieces every workload shares
// ---------------------------------------------------------------------------

static HOST: ServerHostServices = ServerHostServices;

fn compile(text: &str) -> Expression {
    Expression::parse_with(text, ParseOptions::default(), Some(&HOST))
        .unwrap_or_else(|e| panic!("{text:?} must parse: {}", e.message()))
}

/// The evaluator the write path uses: the R4 settings, the server host
/// service and an application context, so the implicit-before host call
/// fires once per expression (design.md section 8).
fn extraction_evaluator(model: &dyn Model) -> Evaluator<'_> {
    Evaluator::for_version(model, "4.0.1")
        .expect("4.0.1 is a version")
        .with_host_services(&HOST)
        .with_application_context(true)
}

fn root_of(resource: &Resource) -> Value {
    Value::Node(Arc::new(
        resource
            .as_node()
            .expect("a byte-backed resource hands out a node"),
    ))
}

fn evaluate(evaluator: &Evaluator<'_>, expr: &Expression, root: &Value) -> Vec<Value> {
    evaluator
        .evaluate(expr, Some(root))
        .unwrap_or_else(|e| panic!("evaluation must succeed before it is measured: {e:?}"))
}

/// One extraction: load the stored bytes and evaluate every expression.
/// Loading is an `Arc` clone and is part of the per-resource cost.
fn extract(evaluator: &Evaluator<'_>, compiled: &[Expression], bytes: &Arc<[u8]>) -> usize {
    let resource = Resource::from_stored_untracked(Arc::clone(bytes));
    let root = root_of(&resource);
    let mut n = 0;
    for expr in compiled {
        n += black_box(evaluate(evaluator, expr, &root)).len();
    }
    n
}

// ---------------------------------------------------------------------------
// path.perf.004: what the extraction materialised that it did not touch
// ---------------------------------------------------------------------------

/// The identifiers an expression names, outside string literals.
fn identifiers(expr: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_string = false;
    let mut word = String::new();
    for c in expr.chars() {
        if in_string {
            if c == '\'' {
                in_string = false;
            }
            continue;
        }
        if c == '\'' {
            in_string = true;
        }
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else if !word.is_empty() {
            out.insert(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        out.insert(word);
    }
    out
}

/// An element path's names below the root, indices dropped:
/// `Patient.telecom[1].system` is `["telecom", "system"]`.
fn segments(path: &str) -> Vec<&str> {
    path.split('.')
        .skip(1)
        .map(|s| s.split('[').next().unwrap_or(s))
        .collect()
}

/// Materialise every element inside `value`: its own primitive, then each
/// child's, depth first.
fn walk(value: &Value) {
    if let Value::Node(node) = value {
        black_box(node.primitive());
        for (_, child) in node.all_children().expect("a returned node's children") {
            walk(&child);
        }
    }
}

/// The elements the expression set **touches**, as paths with indices dropped:
/// every element an expression names on a path from the root
/// (`Patient.telecom.system` for `Patient.telecom.where(system='phone')`), and
/// every element inside a value an expression returns, which is the caller's
/// to read -- an extractor indexes `Patient.name`'s `family` and `given`.
///
/// The second set is measured rather than declared: each expression is
/// evaluated over a fresh tracked resource and everything it returned is
/// walked, so the meter of that resource lists exactly the returned subtrees.
fn touched(
    evaluator: &Evaluator<'_>,
    compiled: &[Expression],
    bytes: &Arc<[u8]>,
) -> (Vec<BTreeSet<String>>, BTreeSet<String>) {
    let named = PATIENT_SEARCH_PARAMETERS
        .iter()
        .map(|e| identifiers(e))
        .collect();
    let mut returned = BTreeSet::new();
    for expr in compiled {
        let resource = Resource::from_stored(Arc::clone(bytes));
        let root = root_of(&resource);
        let values = evaluate(evaluator, expr, &root);
        // The log entries after this point are the walk's alone; an element
        // the evaluation already materialised is logged again when walked.
        let before = resource.report().materialised.len();
        for value in &values {
            walk(value);
        }
        returned.extend(
            resource
                .report()
                .materialised
                .into_iter()
                .skip(before)
                .map(|p| segments(&p).join(".")),
        );
    }
    (named, returned)
}

/// The materialised elements the expression set does not touch. Anything here
/// was built without any expression reaching for it, which is what
/// `path.perf.004` requires never to happen.
fn untouched(
    materialised: &[String],
    named: &[BTreeSet<String>],
    returned: &BTreeSet<String>,
) -> Vec<String> {
    materialised
        .iter()
        .filter(|path| {
            let segs = segments(path);
            let by_name = named
                .iter()
                .any(|ids| !segs.is_empty() && segs.iter().all(|s| ids.contains(*s)));
            !by_name && !returned.contains(&segs.join("."))
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// The workloads
// ---------------------------------------------------------------------------

fn run_workloads(samples: usize, bytes: &Arc<[u8]>) -> (Vec<Row>, Vec<String>) {
    let model = R4Model::new(R4);
    let evaluator = extraction_evaluator(&model);
    let mut rows = Vec::new();
    let mut notes = Vec::new();

    // path.perf.002: one parse, then evaluations of the parsed form. The
    // figure is per evaluation; `parses` counts the parses the workload
    // performed, which is the one above and nothing inside the loop.
    let mut parses = 0u64;
    let expr = {
        parses += 1;
        compile(TYPICAL_SEARCH_PARAMETER)
    };
    let resource = Resource::from_stored_untracked(Arc::clone(bytes));
    let root = root_of(&resource);
    let answered = evaluate(&evaluator, &expr, &root).len();
    assert_eq!(answered, 3, "patient-example carries three names");
    let st = measure(samples, || {
        black_box(evaluate(&evaluator, black_box(&expr), &root));
    });
    let mut eval_row = row(
        "path-eval-compiled",
        "path.perf.002",
        "median <= 5 us per evaluation, no reparse",
        bytes.len(),
        &st,
    )
    .int("parses", parses)
    .int("evaluations_per_parse", EVALUATIONS as u64);
    // The 1,000-evaluation run itself, timed whole, so the "1,000 times"
    // of the scenario is a measured figure and not only a batch size.
    let started = Instant::now();
    for _ in 0..EVALUATIONS {
        black_box(evaluate(&evaluator, &expr, &root));
    }
    eval_row = eval_row.num(
        "thousand_evaluations_us",
        started.elapsed().as_secs_f64() * 1e6,
        3,
    );
    rows.push(eval_row);

    // path.perf.003: the whole set over one stored resource, compiled once.
    let compiled: Vec<Expression> = PATIENT_SEARCH_PARAMETERS
        .iter()
        .map(|e| compile(e))
        .collect();
    let answering = {
        let resource = Resource::from_stored_untracked(Arc::clone(bytes));
        let root = root_of(&resource);
        compiled
            .iter()
            .filter(|e| !evaluate(&evaluator, e, &root).is_empty())
            .count()
    };
    let st = measure(samples, || {
        black_box(extract(&evaluator, &compiled, black_box(bytes)));
    });

    // path.perf.004: the same extraction once more over a **tracked**
    // resource, whose meter logs the path of every element an owned value was
    // built for.
    let tracked = Resource::from_stored(Arc::clone(bytes));
    let root = root_of(&tracked);
    for expr in &compiled {
        black_box(evaluate(&evaluator, expr, &root));
    }
    let materialised = tracked.report().materialised;
    let (named, returned) = touched(&evaluator, &compiled, bytes);
    let missed = untouched(&materialised, &named, &returned);
    if !missed.is_empty() {
        notes.push(format!(
            "path-extract-patient materialised {} element(s) no expression touches: {missed:?}",
            missed.len()
        ));
    }
    rows.push(
        row(
            "path-extract-patient",
            "path.perf.003, path.perf.004",
            "median total <= 300 us; no untouched element materialised",
            bytes.len(),
            &st,
        )
        .int("expressions", compiled.len() as u64)
        .int("expressions_answering", answering as u64)
        .int("elements_materialised", materialised.len() as u64)
        .int("untouched_elements_materialised", missed.len() as u64),
    );

    (rows, notes)
}

fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = repo.join("bench/micro/results.json");
    let mut samples = SAMPLES;
    let mut write = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
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
            "run the bench/micro workloads in release: cargo bench -p frevo-model --bench path_bytes"
        );
    }

    let source = std::fs::read(repo.join(FIXTURE)).unwrap_or_else(|e| {
        panic!("{FIXTURE} cannot be read ({e}); it is fetched, not vendored: run `bash oracles/fetch.sh`")
    });
    let canonical = Resource::from_stored(Arc::from(source))
        .to_canonical_vec()
        .expect("patient-example canonicalises");
    let bytes: Arc<[u8]> = Arc::from(canonical);

    let (rows, notes) = run_workloads(samples, &bytes);
    println!(
        "{:<22} {:>8} {:>10} {:>10} {:>10}  budget",
        "workload", "bytes", "median us", "p10 us", "p90 us"
    );
    for r in &rows {
        let get = |k: &str| {
            r.members
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.as_str())
                .unwrap_or("-")
        };
        println!(
            "{:<22} {:>8} {:>10} {:>10} {:>10}  {} ({})",
            r.name,
            get("fixture_bytes"),
            get("median_us"),
            get("p10_us"),
            get("p90_us"),
            get("budget").trim_matches('"'),
            get("scenario").trim_matches('"'),
        );
        for (key, value) in &r.members {
            if matches!(
                *key,
                "parses"
                    | "thousand_evaluations_us"
                    | "expressions_answering"
                    | "elements_materialised"
                    | "untouched_elements_materialised"
            ) {
                println!("{:<22} {key} = {value}", "");
            }
        }
    }
    for note in &notes {
        println!("note: {note}");
    }
    if write {
        record::write_merged(
            &out,
            &repo,
            "path_bytes",
            &|name| OWNED.contains(&name),
            &rows,
        );
        println!("\nwrote {}", out.display());
    }
}
