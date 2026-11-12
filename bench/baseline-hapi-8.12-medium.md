# Baseline: HAPI FHIR 8.12.0 (medium)

- measured: 2026-11-08T16:24:35Z
- target: `http://localhost:8090/fhir`
- dataset: `synthea-medium (1,128 patients)`

## Dataset

A latency figure without the data volume it was measured against is not
comparable to anything, so the row counts are recorded here.

| Resource | Count |
|---|---|
| Patient | 1,128 |
| Observation | 483,969 |
| Encounter | 64,025 |
| Condition | 36,302 |
| Procedure | 132,851 |
| MedicationRequest | 43,831 |
| Organization | 819 |
| Practitioner | 819 |
| Immunization | 16,704 |
| DiagnosticReport | 53,547 |
| Claim | 107,856 |

## Results

| Workload | p50 ms | p95 ms | p99 ms | rps | conc | errors | CPU peak % | RSS peak MiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `read-by-id` | 36.88 | 66.91 | 116.2 | 392.74 | 16 | 0 | 428.1 | 3330 |
| `vread` | 17.91 | 26.91 | 38.85 | 415.94 | 8 | 0 | 388.9 | 3332 |
| `search-token` | 18.99 | 26.41 | 32.16 | 404.23 | 8 | 0 | 334.4 | 3355 |
| `search-name` | 37.18 | 58.1 | 71.87 | 205.05 | 8 | 3 | 433.1 | 3358 |
| `search-date-range` | 39.77 | 55.84 | 70.9 | 193.52 | 8 | 0 | 530.2 | 3363 |
| `search-patient-observations` | 66.18 | 109.45 | 127.53 | 113.72 | 8 | 0 | 561.2 | 3368 |
| `search-sorted` | 65.44 | 106.91 | 151.37 | 114.51 | 8 | 0 | 515.6 | 3373 |
| `search-total-accurate` | 16.6 | 30.89 | 34.85 | 410.75 | 8 | 0 | 283.2 | 3373 |
| `search-chained` | 51.8 | 74.62 | 91.46 | 150.01 | 8 | 0 | 429.4 | 3374 |
| `search-include` | 66.83 | 123.53 | 178.71 | 108.86 | 8 | 0 | 649.3 | 3378 |
| `search-revinclude` | 161.84 | 725.78 | 818.48 | 33.94 | 8 | 0 | 836.7 | 3380 |
| `everything` | 140.34 | 180.11 | 201.63 | 27.73 | 4 | 0 | 395.7 | 3379 |
| `transaction-ingest` | 26.1 | 33.8 | 41.91 | 289.57 | 8 | 0 | 232.2 | 3379 |
| `create-single` | 20.66 | 27.88 | 33.34 | 366.53 | 8 | 0 | 150.3 | 3380 |

## What each workload stresses

**`read-by-id`** — The cheapest possible read. Dominated by one indexed primary-key lookup plus serialization, so it is the clearest measure of per-request overhead. Frevo's design bet -- serving stored JSON as bytes without reparsing -- should show up here first.

**`vread`** — Version-specific read; a second index and a different row path.

**`search-token`** — The most common search shape: one token parameter against the search-parameter index.

**`search-name`** — String-parameter search with a prefix match and paging.

**`search-date-range`** — Date-range predicate over an ordered index.

**`search-patient-observations`** — Reference search plus paging -- the single most common clinical query shape, and the one most sensitive to index layout.

**`search-sorted`** — Sorting a result set; keyset vs offset pagination shows here.

**`search-total-accurate`** — An exact count over the match set. HAPI pays a second query for this; it is where _total modes diverge most.

**`search-chained`** — A forward chained search. HAPI resolves this with a join that grows with the referenced set, and it is the workload most likely to expose a pathological query plan.

**`search-include`** — _include, which turns one query into two and re-serializes the included resources.

**`search-revinclude`** — _revinclude, the reverse direction, usually the more expensive one.

**`everything`** — $everything on a Patient compartment: the heaviest read in routine use, and a good proxy for compartment traversal cost.

**`transaction-ingest`** — Write path: a small transaction bundle with an internal reference. Tests per-transaction overhead and reference resolution rather than bulk throughput.

**`create-single`** — The simplest write; isolates per-create overhead.

## Method

- Warmup requests are issued and **discarded** before measurement. HAPI
  runs on the JVM, so a cold measurement would be meaningless and would
  flatter any native implementation compared against it.
- Ids are sampled from the server under test, so no workload depends on
  ids that exist only in one deployment.
- CPU and RSS are sampled from `docker stats` once a second during each
  workload, so they are the server process's usage, not the host's.
- Percentiles are computed from every measured request in the workload.

Reproduce with:

```bash
docker compose -f harness/docker-compose.yml up -d
bash harness/scripts/synthea.sh generate small
bash harness/scripts/synthea.sh load small http://localhost:8090/fhir
python bench/run.py --base http://localhost:8090/fhir --label "HAPI FHIR 8.12.0 (medium)" \
    --container frevo-harness-hapi-1 --out bench/baseline-hapi-8.12.md
```

## Compared with the small baseline

The small set holds 39,842 Observations; this one holds 483,969 -- about 12
times the data. The result is not what a 12x increase should look like.

| Workload | p50 small | p50 medium | | p95 small | p95 medium | | rps small | rps medium |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `read-by-id` | 42.36 | 36.88 | 0.87x | 68.95 | 66.91 | 0.97x | 354 | 393 |
| `vread` | 27.68 | 17.91 | 0.65x | 56.01 | 26.91 | 0.48x | 252 | 416 |
| `search-token` | 22.08 | 18.99 | 0.86x | 33.29 | 26.41 | 0.79x | 342 | 404 |
| `search-name` | 27.91 | 37.18 | 1.33x | 45.28 | 58.1 | 1.28x | 254 | 205 |
| `search-date-range` | 42.75 | 39.77 | 0.93x | 63.32 | 55.84 | 0.88x | 175 | 194 |
| `search-patient-observations` | 64.86 | 66.18 | 1.02x | 101.3 | 109.45 | 1.08x | 115 | 114 |
| `search-sorted` | 57.52 | 65.44 | 1.14x | 77.11 | 106.91 | 1.39x | 136 | 115 |
| `search-total-accurate` | 21.27 | 16.6 | 0.78x | 35.33 | 30.89 | 0.87x | 341 | 411 |
| `search-chained` | 50.29 | 51.8 | 1.03x | 79.12 | 74.62 | 0.94x | 148 | 150 |
| `search-include` | 63.21 | 66.83 | 1.06x | 81.74 | 123.53 | 1.51x | 123 | 109 |
| `search-revinclude` | 124.44 | 161.84 | 1.30x | 376.63 | 725.78 | 1.93x | 50 | 34 |
| `everything` | 121.73 | 140.34 | 1.15x | 166.14 | 180.11 | 1.08x | 31 | 28 |
| `transaction-ingest` | 32.92 | 26.1 | 0.79x | 48.65 | 33.8 | 0.69x | 221 | 290 |
| `create-single` | 26.73 | 20.66 | 0.77x | 38.23 | 27.88 | 0.73x | 271 | 367 |

**7 of 14 workloads got _faster_ on 12 times the data**: `read-by-id`, `vread`, `search-token`, `search-date-range`, `search-total-accurate`, `transaction-ingest`, `create-single`.

That cannot happen if these measurements were dominated by data volume, so
they are not. Two things follow, and both matter more than the numbers:

**1. The small baseline was under-warmed.** `vread` is 0.48x its small-set p95
-- more than twice as fast on twelve times the data. The medium run simply ran
longer, so the JVM's JIT had settled by the time it was measured. The harness
default of 50 warmup requests is too few for a JVM, and that is a defect in my
method, not a property of HAPI. The default is raised to 300 in
`bench/workloads.yaml`, with the reasoning recorded there.

Note the direction of the error: an under-warmed HAPI looks **slower**, which
makes every Frevo target **easier**. The small-set ratios were therefore
generous, not demanding, which is the worse way to be wrong.

**2. At this scale HAPI's latency is mostly fixed per-request overhead.**
`read-by-id` is 36.9 ms p50 against 484k Observations and 42.4 ms against 40k.
An indexed primary-key lookup does not care about the other 444,000 rows, so
almost all of that 37 ms is request handling: HTTP, the connection pool, JPA
session setup, entity hydration, re-serialization. That is precisely the cost
Frevo's design removes, and it is why `read-by-id` carries the largest target
ratio.

### Where data volume genuinely bites

Three workloads degraded, and they are the ones that touch many rows:

| Workload | p95 growth | What it indicates |
|---|---:|---|
| `search-revinclude` | **1.93x** (377 -> 726 ms) | the clear outlier; rps fell 50 -> 34 |
| `search-include` | 1.51x | second query plus re-serialization of included resources |
| `search-sorted` | 1.39x | sorting a larger candidate set |

`search-revinclude` was already the worst workload on the small set (p95 three
times its p50, 691% CPU). It degrades close to linearly in data volume while
everything else stays flat, which points at a query plan that widens with the
referenced set rather than a constant cost. It is the single most promising
target in the whole benchmark.

### Footprint is unchanged

RSS peaked at 3427 MiB on the small set and 3380 MiB on the medium one.
That is the configured JVM heap, not a function of the data, and it is why the
footprint target is stated as an absolute ceiling rather than a ratio.

