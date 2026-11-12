# `bench/micro/` -- in-process model workloads

The five `frevo-model` workloads of `design.md` section 8 in
`openspec/changes/add-fhirpath-and-codegen-r4/`, declared in
[`workloads.yaml`](workloads.yaml) and implemented in
[`crates/frevo-model/benches/model_micro.rs`](../../crates/frevo-model/benches/model_micro.rs).
Their budgets are the scenarios `model.perf.001` .. `model.perf.005`, graded by
`crates/frevo-model/tests/model_bytes_lazy_perf.rs` against the medians
recorded here; `model.perf.006` is the regression policy of
[`../targets.md`](../targets.md) over them.

## Reproducing

```text
cargo bench -p frevo-model --bench model_micro                         # -> bench/micro/results.json
cargo bench -p frevo-model --bench model_micro -- --out bench/micro/baseline.json
cargo bench -p frevo-model --bench model_micro -- --no-write --diagnose  # print only, plus a meta-update breakdown
cargo bench -p frevo-model --bench model_micro -- --write-fixtures     # regenerate fixtures/ from the generator
```

The bench refuses to run in a debug build. Every number in `results.json` and
`baseline.json` is written by that binary; none is typed in.

## Method

`std::time::Instant` only -- no criterion or divan, so the workspace takes no
dependency for it. Windows' `Instant` resolves to about 100 ns, coarser than the
1 us budget allows to be read off one operation, so each sample times a batch
of operations sized to run for at least 500 us, and the per-operation figure is
the batch time over its length. The recorded figure is the median of 201
samples after 300 ms of discarded warm-up; p10, p90 and min are recorded
beside it. Workloads run in a fixed order in one process.

## Files

| File | What it is |
|---|---|
| `workloads.yaml` | the declaration: name, scenario, fixture, what it stresses, budget |
| `fixtures/patient-4kib.json`, `fixtures/patient-64kib.json` | the committed documents; the bench fails if they drift from the tests' generator |
| `results.json` | the measurement of the commit it names |
| `baseline.json` | the measurement the regression policy compares against |

`results.json` has the shape `bench/baseline-hapi-8.12.json` set: a `workloads`
array of objects keyed by `name`, with `median_us` (and `tokens_scanned` where
the scenario counts tokens).

## The baseline

`baseline.json` is the measurement recorded for the previous commit on `main`.
The first one was recorded on `change/add-fhirpath-and-codegen-r4` when these
workloads landed, because `main` had no `bench/micro/` to measure; from then on
it is re-recorded from `main` when a change that moves these numbers merges,
exactly as the HAPI baselines are re-measured deliberately rather than drifting.

## Measurement environment

`bench/targets.md` names Docker Desktop on Windows with a 12-CPU Linux VM as the
reference environment. The `environment` member of each results file says where
it was actually measured; a number taken elsewhere is a leading indicator, and
CI's measurement decides.
