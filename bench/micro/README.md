# `bench/micro/` -- in-process model and FHIRPath workloads

The workloads of `design.md` section 8 in
`openspec/changes/add-fhirpath-and-codegen-r4/`, declared in
[`workloads.yaml`](workloads.yaml) and implemented in three bench targets:

| Bench | Rows | Graded by |
|---|---|---|
| [`crates/frevo-model/benches/model_micro.rs`](../../crates/frevo-model/benches/model_micro.rs) | the five `model-*` workloads (six rows) | `model.perf.001` .. `.006`, `crates/frevo-model/tests/model_bytes_lazy_perf.rs` |
| [`crates/frevo-model/benches/path_bytes.rs`](../../crates/frevo-model/benches/path_bytes.rs) | `path-eval-compiled`, `path-extract-patient` | `path.perf.002` .. `.004` and `.006`, `crates/frevo-path/tests/path_perf.rs` |
| [`crates/frevo-path/benches/path_micro.rs`](../../crates/frevo-path/benches/path_micro.rs) | `path-compile`, `path-suite-wallclock` | `path.perf.001`, `.005` and `.006`, the same file |

`model.perf.006` and `path.perf.006` are the regression policy of
[`../targets.md`](../targets.md) over the rows.

The two path workloads that evaluate over a stored resource live in
`frevo-model` because `path.perf.004` needs a byte-backed resource and
`frevo-path` may not name `frevo-model` (`path.api.008`). `path_micro.rs`
includes the committed oracle harness (`crates/frevo-path/tests/oracle/`) by
path, so `path-compile` and `path-suite-wallclock` drive the same seam the
tests do.

## Reproducing

```text
cargo bench -p frevo-model --bench model_micro                         # -> bench/micro/results.json
cargo bench -p frevo-model --bench model_micro -- --out bench/micro/baseline.json
cargo bench -p frevo-model --bench model_micro -- --no-write --diagnose  # print only, plus a meta-update breakdown
cargo bench -p frevo-model --bench model_micro -- --write-fixtures     # regenerate fixtures/ from the generator
cargo bench -p frevo-model --bench path_bytes                          # path-eval-compiled, path-extract-patient
cargo bench -p frevo-path  --bench path_micro                          # path-compile, path-suite-wallclock
```

The path benches take `--out <file>` and `--no-write` as `model_micro` does.
Every bench writes through [`record.rs`](record.rs), which replaces the rows
that bench owns and keeps every other row as it was, so the three can run in
any order. Each row names the `bench`, `commit`, `worktree_dirty` flag and
`measured_at` time it was taken with, because one file's rows can come from
different runs.

Every bench refuses to run in a debug build. Every number in `results.json` and
`baseline.json` is written by that binary; none is typed in.

## Method

`std::time::Instant` only -- no criterion or divan, so the workspace takes no
dependency for it. Windows' `Instant` resolves to about 100 ns, coarser than the
1 us budget allows to be read off one operation, so each sample times a batch
of operations sized to run for at least 500 us, and the per-operation figure is
the batch time over its length. The recorded figure is the median of 201
samples after 300 ms of discarded warm-up; p10, p90 and min are recorded
beside it. Workloads run in a fixed order in one process.

`path-suite-wallclock` is the exception: a wall-clock figure in seconds
(`median_s`), the median of five cold runs, each a fresh child process of
`path_micro` that loads the suite, its inputs and the R4 package and runs all
1,191 cases once. The run's per-status counts are recorded with it.

`path-extract-patient` also records `untouched_elements_materialised`
(`path.perf.004`): a second pass over a tracked resource logs every element an
owned value was built for, and an element counts as touched when an expression
names every element on its path or when it lies inside a value an expression
returned (measured by walking each expression's results over a fresh tracked
resource). The path fixture is `patient-example.json` from the pinned
`fhir-test-cases` tree (`oracles/PINS.md`), canonicalised before timing: it is
the JSON form of the `patient-example.xml` the tests measure over, and it is
fetched rather than committed, like the tests' own fixture.

## Files

| File | What it is |
|---|---|
| `workloads.yaml` | the declaration: name, scenario, fixture, what it stresses, budget |
| `fixtures/patient-4kib.json`, `fixtures/patient-64kib.json` | the committed documents; the bench fails if they drift from the tests' generator |
| `record.rs` | the shared writer the three benches include by path |
| `results.json` | the measurement of the commit it names |
| `baseline.json` | the measurement the regression policy compares against |

`results.json` has the shape `bench/baseline-hapi-8.12.json` set: a `workloads`
array of objects keyed by `name`, with `median_us` (and `tokens_scanned` where
the scenario counts tokens).

## The baseline

`baseline.json` is the measurement recorded for the previous commit on `main`.
The first one was recorded on `change/add-fhirpath-and-codegen-r4` when these
workloads landed (the model rows with task 27, the path rows with task 55), because `main` had no `bench/micro/` to measure; from then on
it is re-recorded from `main` when a change that moves these numbers merges,
exactly as the HAPI baselines are re-measured deliberately rather than drifting.

## Measurement environment

`bench/targets.md` names Docker Desktop on Windows with a 12-CPU Linux VM as the
reference environment. The `environment` member of each results file says where
it was actually measured; a number taken elsewhere is a leading indicator, and
CI's measurement decides.
