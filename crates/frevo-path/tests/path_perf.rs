//! FHIRPath performance budgets: `path.perf.001` through `path.perf.006`.
//!
//! ```text
//! cargo test -p frevo-path --test path_perf
//! ```
//!
//! Task 4 of `add-fhirpath-and-codegen-r4`.
//!
//! ## How these are made to fail for the right reason
//!
//! A budget stated as "the median is at most 20 microseconds" cannot be graded
//! by an in-process `cargo test` alone: the test profile is unoptimised, so
//! asserting a release-profile number here would be a false red once the engine
//! lands, and skipping the assertion in a debug build would be a false green.
//!
//! So every test here has two halves, and both must hold:
//!
//! 1. **The recorded half.** The absolute budget is graded against the median
//!    the `bench/micro/` workload recorded, read from [`MICRO_RESULTS`] (and,
//!    for `path.perf.006`, [`MICRO_BASELINE`]). This is the gate condition as
//!    `design.md` section 8 states it. It fails today because neither the
//!    workload nor the recorded result exists -- which is exactly what "the
//!    capability is missing" means for a measured budget. The file's shape is
//!    documented on [`MICRO_RESULTS`] and mirrors
//!    `bench/baseline-hapi-8.12.json`, which is the convention already in the
//!    repository.
//! 2. **The in-process half.** Everything in the scenario that is a *ratio* or
//!    an *invariant* rather than an absolute time is measured here and asserted
//!    unconditionally, because a ratio survives a change of build profile: one
//!    parse for 1,000 evaluations (`path.perf.002`), one compile per distinct
//!    expression across two extractions (`path.perf.003`), an evaluation cost
//!    that does not scale with document size (`path.perf.004`), and a suite run
//!    in which every case was actually graded (`path.perf.005`).
//!
//! Every in-process measurement runs through the committed harness seam and
//! **requires the operation to succeed**, so a `NotImplemented` answer is a
//! panic naming the stage rather than a free pass: a timing taken over an
//! engine that refuses to do anything would otherwise come in at zero and
//! satisfy any budget.
//!
//! The workloads and budgets are `design.md` section 8:
//!
//! | Workload | Budget | Scenario |
//! |---|---|---|
//! | `path-compile` | median <= 20 us | `path.perf.001` |
//! | `path-eval-compiled` | median <= 5 us per evaluation, no reparse | `path.perf.002` |
//! | `path-extract-patient` | median total <= 300 us | `path.perf.003`, `path.perf.004` |
//! | `path-suite-wallclock` | <= 10 s | `path.perf.005` |

// `dead_code` is allowed for the shared harness module and nothing else; see
// `tests/oracle/mod.rs:18-23`.
#[allow(dead_code)]
mod oracle;

use std::hint::black_box;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use oracle::engine::{
    CaseContext, EngineConfig, ParsedHandle, StrictnessFlags, SuiteEngine, engine_under_test,
};
use oracle::input::{self, Document, SuiteInput};
use oracle::json::{self, Value};
use oracle::run::{self, CaseStatus, RunReport};
use oracle::suite::Suite;
use oracle::value::ResultItem;

// ---------------------------------------------------------------------------
// The recorded half
// ---------------------------------------------------------------------------

/// Where the `bench/micro/` workloads record what they measured.
///
/// ```json
/// {
///   "profile": "release",
///   "commit": "<the commit measured>",
///   "workloads": [
///     { "name": "path-compile", "median_us": 14.2 },
///     { "name": "path-eval-compiled", "median_us": 3.1, "parses": 1 },
///     { "name": "path-extract-patient", "median_us": 240.0,
///       "untouched_elements_materialised": 0 },
///     { "name": "path-suite-wallclock", "median_s": 6.4 }
///   ]
/// }
/// ```
const MICRO_RESULTS: &str = "bench/micro/results.json";

/// The same shape, recorded for the previous commit on `main`.
/// `path.perf.006` compares the two.
const MICRO_BASELINE: &str = "bench/micro/baseline.json";

/// Every path workload `design.md` section 8 names.
const PATH_WORKLOADS: [&str; 4] = [
    "path-compile",
    "path-eval-compiled",
    "path-extract-patient",
    "path-suite-wallclock",
];

/// A number recorded for a workload, or the reason it could not be read.
fn recorded(file: &str, workload: &str, key: &str) -> Result<f64, String> {
    let path = input::repo_path(file);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{file} does not exist ({e}). The {workload} workload under \
             bench/micro/ must record its median there; until it does, this \
             budget is unmeasured."
        )
    })?;
    let doc = json::parse(&text).map_err(|e| format!("{file} is not JSON: {e}"))?;
    let entries = match doc.get("workloads") {
        Some(Value::Array(items)) => items,
        Some(other) => return Err(format!("{file}: \"workloads\" must be an array: {other:?}")),
        None => return Err(format!("{file} has no \"workloads\" member")),
    };
    let entry = entries
        .iter()
        .find(|e| e.get("name").and_then(Value::as_str) == Some(workload))
        .ok_or_else(|| format!("{file} records no workload named {workload:?}"))?;
    match entry.get(key) {
        Some(Value::Number(n)) => n
            .parse::<f64>()
            .map_err(|_| format!("{file}: {workload}.{key} is not a number: {n}")),
        Some(other) => Err(format!(
            "{file}: {workload}.{key} is not a number: {other:?}"
        )),
        None => Err(format!("{file}: {workload} records no {key:?}")),
    }
}

/// Require a recorded number, naming the scenario that needs it.
fn require_recorded(file: &str, workload: &str, key: &str, scenario: &str) -> f64 {
    recorded(file, workload, key)
        .unwrap_or_else(|why| panic!("{scenario} is graded on a measured median: {why}"))
}

// ---------------------------------------------------------------------------
// The in-process half: a counting engine over the committed seam
// ---------------------------------------------------------------------------

/// Wraps the engine under test and counts the calls, which is what makes
/// "parsed once" and "compiled once per expression" assertable rather than
/// assumed.
struct Counting {
    inner: Box<dyn SuiteEngine>,
    parses: usize,
    checks: usize,
    evaluations: usize,
}

impl Counting {
    fn new() -> Self {
        let mut inner = engine_under_test();
        inner
            .configure(&EngineConfig::default())
            .unwrap_or_else(|e| {
                panic!("the engine must configure before anything is measured: {e}")
            });
        Counting {
            inner,
            parses: 0,
            checks: 0,
            evaluations: 0,
        }
    }

    fn parse(&mut self, expr: &str) -> ParsedHandle {
        self.parses += 1;
        self.inner
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr:?} must parse before it can be measured: {e}"))
    }

    fn check(&mut self, parsed: ParsedHandle, context: &CaseContext<'_>) {
        self.checks += 1;
        self.inner
            .check(parsed, context, StrictnessFlags::CHECK)
            .unwrap_or_else(|e| panic!("the check must succeed before it can be measured: {e}"));
    }

    fn evaluate(
        &mut self,
        parsed: ParsedHandle,
        inp: Option<&SuiteInput>,
        context: &CaseContext<'_>,
    ) -> Vec<ResultItem> {
        self.evaluations += 1;
        self.inner
            .evaluate(parsed, inp, context, StrictnessFlags::EVALUATE)
            .unwrap_or_else(|e| panic!("evaluation must succeed before it can be measured: {e}"))
    }
}

fn context_for<'a>(inp: Option<&'a SuiteInput>) -> CaseContext<'a> {
    let resource_type = inp.and_then(|i| i.resource_type());
    CaseContext {
        root_resource_type: resource_type,
        resource_type,
        ..CaseContext::default()
    }
}

fn micros(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000_000.0
}

/// The median of `samples`, which is what every budget is stated against.
fn median(mut samples: Vec<f64>) -> f64 {
    assert!(!samples.is_empty(), "a median needs samples");
    samples.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a timing"));
    let mid = samples.len() / 2;
    if samples.len() % 2 == 0 {
        (samples[mid - 1] + samples[mid]) / 2.0
    } else {
        samples[mid]
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn load_input(file: &str) -> SuiteInput {
    input::load(file).unwrap_or_else(|e| {
        panic!("the input {file} must load before it can be measured over: {e}")
    })
}

/// The 4 KiB `Patient` every workload is stated against. `patient-example.xml`
/// is 4,029 bytes, which is what makes it the fixture rather than a new file.
fn patient_4kib() -> SuiteInput {
    let inp = load_input("patient-example.xml");
    assert!(
        (3_500..4_500).contains(&inp.bytes.len()),
        "the fixture must be about 4 KiB, not {} bytes",
        inp.bytes.len()
    );
    inp
}

/// A `Patient` of about 64 KiB, built here rather than committed: the point of
/// `path.perf.004` is a **ratio** between two document sizes, so the large one
/// only has to be large and well-formed.
fn patient_64kib() -> SuiteInput {
    let mut body = String::from(
        "{\"resourceType\":\"Patient\",\"id\":\"big\",\"birthDate\":\"1974-12-25\",\"name\":[",
    );
    // Repetitions are appended until the document passes 64 KiB, so the
    // fixture's size is a property of this loop and not of an estimate that can
    // drift.
    let mut i = 0usize;
    while body.len() < 64 * 1024 {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            "{{\"family\":\"Family{i}\",\"given\":[\"Given{i}A\",\"Given{i}B\"]}}"
        ));
        i += 1;
    }
    body.push_str("]}");
    assert!(
        body.len() >= 64 * 1024,
        "the large fixture must be at least 64 KiB, not {} bytes",
        body.len()
    );

    let document = Document::Json(
        json::parse(&body).unwrap_or_else(|e| panic!("the generated fixture must be JSON: {e}")),
    );
    SuiteInput {
        file: "generated-patient-64kib.json".to_string(),
        bytes: body.into_bytes(),
        document,
    }
}

/// A typical search-parameter expression, which is what `path-compile`
/// measures. This is R4's `Patient-name` expression.
const TYPICAL_SEARCH_PARAMETER: &str = "Patient.name";

/// Every enabled `Patient` search-parameter expression, as R4 defines them.
/// `path-extract-patient` evaluates all of them over one resource, and the
/// 300 us budget is the per-resource share of the 5,000 resources/s bulk-ingest
/// target in `bench/targets.md`.
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

// ---------------------------------------------------------------------------
// path.perf.001 -- compiling an expression stays within budget
// ---------------------------------------------------------------------------

// covers: path.perf.001
#[test]
fn compiling_an_expression_stays_within_budget() {
    // The in-process half runs **first**, so that today's failure names the
    // missing engine rather than the missing measurement: a timing taken over
    // an engine that refuses to compile would otherwise be meaningless.
    //
    // Parse and check 200 times, requiring every call to succeed. The invariant
    // is profile-independent: compilation must not get slower as it is
    // repeated, which is what a per-compile leak or a quadratic symbol table
    // would do.
    let patient = patient_4kib();
    let context = context_for(Some(&patient));
    let mut engine = Counting::new();

    let mut first = Vec::new();
    let mut last = Vec::new();
    for i in 0..200 {
        let start = Instant::now();
        let parsed = engine.parse(TYPICAL_SEARCH_PARAMETER);
        engine.check(parsed, &context);
        let elapsed = micros(start.elapsed());
        black_box(parsed);
        if i < 50 {
            first.push(elapsed);
        } else if i >= 150 {
            last.push(elapsed);
        }
    }
    assert_eq!(engine.parses, 200, "every iteration must have compiled");
    assert_eq!(engine.checks, 200);

    let early = median(first);
    let late = median(last);
    assert!(
        late <= early * 1.25 + 1.0,
        "compiling must not get slower as it is repeated: the first 50 compiles \
         median {early:.2} us, the last 50 median {late:.2} us"
    );

    // A compound expression costs more than a trivial one but stays within an
    // order of magnitude, so the budget describes a class of expression rather
    // than one string.
    let compound = median(
        (0..200)
            .map(|_| {
                let start = Instant::now();
                let parsed =
                    engine.parse("Patient.telecom.where(system='phone' or system='email')");
                engine.check(parsed, &context);
                black_box(parsed);
                micros(start.elapsed())
            })
            .collect(),
    );
    assert!(
        compound <= early * 10.0 + 5.0,
        "a compound search-parameter expression must stay within an order of \
         magnitude of a simple one: {early:.2} us against {compound:.2} us"
    );

    // The recorded half: the gate condition, 20 us, measured by the
    // `path-compile` workload in the release profile.
    let recorded_median =
        require_recorded(MICRO_RESULTS, "path-compile", "median_us", "path.perf.001");
    assert!(
        recorded_median <= 20.0,
        "path-compile must record a median of at most 20 us; {MICRO_RESULTS} \
         records {recorded_median} us"
    );
}

// ---------------------------------------------------------------------------
// path.perf.002 -- evaluating a compiled expression does not re-parse it
// ---------------------------------------------------------------------------

// covers: path.perf.002
#[test]
fn evaluating_a_compiled_expression_does_not_reparse_it() {
    // The in-process half: parse once, evaluate 1,000 times, and count.
    let patient = patient_4kib();
    let context = context_for(Some(&patient));
    let mut engine = Counting::new();
    let parsed = engine.parse(TYPICAL_SEARCH_PARAMETER);
    assert_eq!(engine.parses, 1);

    let mut samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let start = Instant::now();
        let values = engine.evaluate(parsed, Some(&patient), &context);
        samples.push(micros(start.elapsed()));
        black_box(&values);
        assert_eq!(values.len(), 3, "the expression must keep answering");
    }

    // No parse after the first, which is the assertion the scenario leads with.
    assert_eq!(
        engine.parses, 1,
        "1,000 evaluations of a parsed expression must perform no further parse"
    );
    assert_eq!(engine.evaluations, 1000);

    // And the per-evaluation cost is stable, so the parsed form is reused
    // rather than rebuilt: the last hundred must not be slower than the first.
    let early = median(samples[..100].to_vec());
    let late = median(samples[900..].to_vec());
    assert!(
        late <= early * 1.25 + 1.0,
        "the per-evaluation cost must not grow: first hundred {early:.2} us, \
         last hundred {late:.2} us"
    );

    // The recorded half: 5 us per evaluation, and the workload records how many
    // parses its 1,000 evaluations cost.
    let per_eval = require_recorded(
        MICRO_RESULTS,
        "path-eval-compiled",
        "median_us",
        "path.perf.002",
    );
    assert!(
        per_eval <= 5.0,
        "path-eval-compiled must record a median of at most 5 us per \
         evaluation; {MICRO_RESULTS} records {per_eval} us"
    );
    let parses = require_recorded(
        MICRO_RESULTS,
        "path-eval-compiled",
        "parses",
        "path.perf.002",
    );
    assert_eq!(
        parses, 1.0,
        "the workload must record exactly one parse for its 1,000 evaluations, \
         not {parses}"
    );
}

// ---------------------------------------------------------------------------
// path.perf.003 -- extracting a resource's whole search-parameter set
// ---------------------------------------------------------------------------

// covers: path.perf.003
#[test]
fn extracting_a_resources_whole_search_parameter_set_stays_within_budget() {
    // The in-process half: every expression must compile and evaluate over the
    // 4 KiB Patient, and extraction must compile each expression **once** -- a
    // second extraction over a second resource performs no further parse, which
    // is what makes 300 us a per-resource figure rather than a
    // per-resource-plus-compilation one.
    let first_patient = patient_4kib();
    let second_patient = patient_64kib();
    let mut engine = Counting::new();

    let compiled: Vec<ParsedHandle> = PATIENT_SEARCH_PARAMETERS
        .iter()
        .map(|expr| engine.parse(expr))
        .collect();
    assert_eq!(engine.parses, PATIENT_SEARCH_PARAMETERS.len());

    let context = context_for(Some(&first_patient));
    for parsed in &compiled {
        engine.check(*parsed, &context);
        let values = engine.evaluate(*parsed, Some(&first_patient), &context);
        black_box(&values);
    }
    let after_first = engine.parses;

    let context = context_for(Some(&second_patient));
    for parsed in &compiled {
        let values = engine.evaluate(*parsed, Some(&second_patient), &context);
        black_box(&values);
    }
    assert_eq!(
        engine.parses, after_first,
        "extracting over a second resource must not recompile anything"
    );
    assert_eq!(engine.evaluations, PATIENT_SEARCH_PARAMETERS.len() * 2);

    // The whole set really is evaluable: the fixture carries data for at least
    // ten of the search parameters, so a set that silently returned empty
    // everywhere could not satisfy this.
    let context = context_for(Some(&first_patient));
    let answered = compiled
        .iter()
        .filter(|parsed| {
            !engine
                .evaluate(**parsed, Some(&first_patient), &context)
                .is_empty()
        })
        .count();
    assert!(
        answered >= 10,
        "the 4 KiB Patient carries data for at least ten of the search \
         parameters, but only {answered} answered"
    );

    // The recorded half: 300 us total, which `design.md` derives as 12.5% of
    // the 2.4 ms of core time 5,000 resources/s leaves per resource.
    let total = require_recorded(
        MICRO_RESULTS,
        "path-extract-patient",
        "median_us",
        "path.perf.003",
    );
    assert!(
        total <= 300.0,
        "path-extract-patient must record a median total of at most 300 us; \
         {MICRO_RESULTS} records {total} us"
    );

    // About 25 expressions, so about 12 us each -- stated so the total and the
    // per-expression figure cannot drift apart.
    let per_expression = total / PATIENT_SEARCH_PARAMETERS.len() as f64;
    assert!(
        per_expression <= 13.0,
        "{per_expression:.1} us per expression over {} expressions exceeds the \
         derivation in design.md section 8",
        PATIENT_SEARCH_PARAMETERS.len()
    );
}

// ---------------------------------------------------------------------------
// path.perf.004 -- evaluation does not force full materialisation
// ---------------------------------------------------------------------------

// covers: path.perf.004
#[test]
fn evaluation_over_the_lazy_model_does_not_force_full_materialisation() {
    // The in-process half is a ratio, because the materialisation counters
    // belong to `frevo-model` and are graded by `model.lazy.*`: what is
    // observable at *this* seam is that the cost of reaching one element does
    // not scale with the size of the document around it.
    let small = patient_4kib();
    let large = patient_64kib();
    assert!(
        large.bytes.len() > small.bytes.len() * 10,
        "the two fixtures must differ by more than an order of magnitude: {} \
         against {} bytes",
        small.bytes.len(),
        large.bytes.len()
    );

    let mut engine = Counting::new();
    let parsed = engine.parse("Patient.birthDate");

    let time_over = |engine: &mut Counting, inp: &SuiteInput| -> f64 {
        let context = context_for(Some(inp));
        engine.check(parsed, &context);
        median(
            (0..200)
                .map(|_| {
                    let start = Instant::now();
                    let values = engine.evaluate(parsed, Some(inp), &context);
                    let t = micros(start.elapsed());
                    black_box(&values);
                    assert_eq!(values.len(), 1, "both fixtures carry a birthDate");
                    t
                })
                .collect(),
        )
    };

    let small_us = time_over(&mut engine, &small);
    let large_us = time_over(&mut engine, &large);
    assert!(
        large_us <= small_us * 1.5 + 1.0,
        "reaching one element must not cost more in a 16x larger document: \
         {small_us:.2} us against {large_us:.2} us -- a cost that scales with \
         the document is full materialisation"
    );

    // And an expression that genuinely does touch everything costs
    // substantially more over the large document, so the ratio above is a
    // property of the expression's reach and not of the measurement being too
    // coarse to see anything.
    let all = engine.parse("Patient.descendants().count()");
    let context = context_for(Some(&large));
    engine.check(all, &context);
    let all_us = median(
        (0..50)
            .map(|_| {
                let start = Instant::now();
                let values = engine.evaluate(all, Some(&large), &context);
                let t = micros(start.elapsed());
                black_box(&values);
                t
            })
            .collect(),
    );
    assert!(
        all_us >= large_us * 4.0,
        "walking the whole 64 KiB document must cost materially more than \
         reading one element of it: {large_us:.2} us against {all_us:.2} us"
    );

    // The recorded half: the workload runs over a byte-backed resource and
    // reports how many elements no expression touched were nonetheless
    // materialised. Zero is the condition.
    let untouched = require_recorded(
        MICRO_RESULTS,
        "path-extract-patient",
        "untouched_elements_materialised",
        "path.perf.004",
    );
    assert_eq!(
        untouched, 0.0,
        "path-extract-patient must materialise no element that no expression \
         touches; {MICRO_RESULTS} records {untouched}"
    );
}

// ---------------------------------------------------------------------------
// path.perf.005 -- the whole suite runs fast enough to be a gate
// ---------------------------------------------------------------------------

fn pinned() -> &'static (Suite, RunReport) {
    static ONCE: OnceLock<(Suite, RunReport)> = OnceLock::new();
    ONCE.get_or_init(run::run_pinned_suite)
}

// covers: path.perf.005
#[test]
fn the_whole_suite_runs_fast_enough_to_be_a_per_pull_request_gate() {
    // The in-process half: the run must actually have **graded** all 1,191
    // cases. A run that answered `did-not-run` for every case finishes
    // instantly and would satisfy any wall-clock budget, so the budget is only
    // meaningful beside this.
    let start = Instant::now();
    let (suite, report) = pinned();
    let elapsed = start.elapsed();

    assert_eq!(suite.case_count(), 1191);
    assert_eq!(report.cases.len(), 1191);
    assert_eq!(
        report.count(CaseStatus::DidNotRun),
        0,
        "every case must have been graded for a wall-clock figure to mean \
         anything; {} did not run",
        report.count(CaseStatus::DidNotRun)
    );

    // Every case carries one of the four statuses, and the two that are neither
    // a pass nor a failure are accounted for individually.
    let graded = report.count(CaseStatus::Passed)
        + report.count(CaseStatus::Failed)
        + report.count(CaseStatus::SkippedForLicence);
    assert_eq!(graded, 1191, "the four statuses must partition the run");

    // Reported, not asserted: `pinned()` may have been initialised by an
    // earlier test in this binary, so this figure is a floor, and a debug build
    // is not the reference environment either way.
    println!(
        "the in-process suite run took {:.3} s (debug profile)",
        elapsed.as_secs_f64()
    );

    // The recorded half: 10 s on the reference environment, measured by the
    // `path-suite-wallclock` workload.
    let seconds = require_recorded(
        MICRO_RESULTS,
        "path-suite-wallclock",
        "median_s",
        "path.perf.005",
    );
    assert!(
        seconds <= 10.0,
        "the full 1,191-case run must record at most 10 s; {MICRO_RESULTS} \
         records {seconds} s"
    );
}

// ---------------------------------------------------------------------------
// path.perf.006 -- no regression against the recorded baseline
// ---------------------------------------------------------------------------

// covers: path.perf.006
#[test]
fn no_regression_against_the_recorded_baseline() {
    // This scenario is recorded-only by construction: a regression is a
    // comparison between two measured runs, and neither can be taken in a debug
    // `cargo test`. Its in-process half is therefore a statement about the two
    // files rather than about a timing -- both must exist, describe the same
    // workloads, and say which profile they measured.
    //
    // It fails today because `bench/micro/` does not exist at all.
    for file in [MICRO_RESULTS, MICRO_BASELINE] {
        let text = std::fs::read_to_string(input::repo_path(file)).unwrap_or_else(|e| {
            panic!(
                "path.perf.006 compares two measured runs, and {file} does not \
                 exist ({e}). Until bench/micro/ records them, there is no \
                 regression measurement -- which is not the same as no \
                 regression."
            )
        });
        let doc = json::parse(&text).unwrap_or_else(|e| panic!("{file} is not JSON: {e}"));
        assert_eq!(
            doc.get("profile").and_then(Value::as_str),
            Some("release"),
            "{file} must record which profile it measured: a debug figure \
             compared with a release baseline is not a regression measurement"
        );
        // Both files must describe the same set of workloads, so a regression
        // cannot be hidden by dropping a workload from the current run.
        for workload in PATH_WORKLOADS {
            assert!(
                recorded(file, workload, median_key(workload)).is_ok(),
                "{file} must record {workload}, or a regression could be hidden \
                 by omitting it"
            );
        }
    }

    // `bench/targets.md`'s regression policy, applied per workload: more than
    // 10% slower than the baseline recorded for the previous commit on `main`
    // fails the performance gate.
    let mut regressions = Vec::new();
    for workload in PATH_WORKLOADS {
        let key = median_key(workload);
        let now = require_recorded(MICRO_RESULTS, workload, key, "path.perf.006");
        let baseline = require_recorded(MICRO_BASELINE, workload, key, "path.perf.006");
        assert!(
            baseline > 0.0,
            "{workload}: a baseline of {baseline} cannot be compared against"
        );
        let ratio = now / baseline;
        if ratio > 1.10 {
            regressions.push(format!(
                "{workload}: {now} against a baseline of {baseline} ({:.1}% slower)",
                (ratio - 1.0) * 100.0
            ));
        }
    }
    assert!(
        regressions.is_empty(),
        "{} workload(s) regressed by more than 10%:\n  {}",
        regressions.len(),
        regressions.join("\n  ")
    );
}

/// `path-suite-wallclock` is recorded in seconds; every other path workload in
/// microseconds.
fn median_key(workload: &str) -> &'static str {
    if workload == "path-suite-wallclock" {
        "median_s"
    } else {
        "median_us"
    }
}
