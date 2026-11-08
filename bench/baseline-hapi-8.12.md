# Baseline: HAPI FHIR 8.12.0

- measured: 2026-11-08T14:10:14Z
- target: `http://localhost:8090/fhir`
- dataset: `synthea-small (112 patients)`

## Dataset

A latency figure without the data volume it was measured against is not
comparable to anything, so the row counts are recorded here.

| Resource | Count |
|---|---|
| Patient | 112 |
| Observation | 39,842 |
| Encounter | 5,355 |
| Condition | 3,502 |
| Procedure | 12,283 |
| MedicationRequest | 3,965 |
| Organization | 282 |
| Practitioner | 282 |
| Immunization | 1,743 |
| DiagnosticReport | 4,570 |
| Claim | 9,320 |

## Results

| Workload | p50 ms | p95 ms | p99 ms | rps | conc | errors | CPU peak % | RSS peak MiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `read-by-id` | 42.36 | 68.95 | 82.79 | 354.28 | 16 | 0 | 297.7 | 3405 |
| `vread` | 27.68 | 56.01 | 75.76 | 252.42 | 8 | 0 | 169.0 | 3414 |
| `search-token` | 22.08 | 33.29 | 38.79 | 341.7 | 8 | 0 | 197.7 | 3417 |
| `search-name` | 27.91 | 45.28 | 58.46 | 254.08 | 8 | 6 | 278.7 | 3417 |
| `search-date-range` | 42.75 | 63.32 | 93.45 | 175.19 | 8 | 0 | 381.4 | 3420 |
| `search-patient-observations` | 64.86 | 101.3 | 122.05 | 115.05 | 8 | 0 | 428.5 | 3421 |
| `search-sorted` | 57.52 | 77.11 | 85.46 | 136.13 | 8 | 0 | 378.6 | 3424 |
| `search-total-accurate` | 21.27 | 35.33 | 44.6 | 340.74 | 8 | 0 | 132.5 | 3424 |
| `search-chained` | 50.29 | 79.12 | 97.72 | 147.52 | 8 | 2 | 438.7 | 3424 |
| `search-include` | 63.21 | 81.74 | 95.92 | 123.48 | 8 | 0 | 391.7 | 3426 |
| `search-revinclude` | 124.44 | 376.63 | 465.83 | 49.59 | 8 | 0 | 690.9 | 3427 |
| `everything` | 121.73 | 166.14 | 175.7 | 31.38 | 4 | 0 | 313.0 | 3427 |
| `transaction-ingest` | 32.92 | 48.65 | 111.75 | 220.97 | 8 | 0 | 168.3 | 3427 |
| `create-single` | 26.73 | 38.23 | 134.68 | 271.13 | 8 | 0 | 71.3 | 3427 |

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
python bench/run.py --base http://localhost:8090/fhir --label "HAPI FHIR 8.12.0" \
    --container frevo-harness-hapi-1 --out bench/baseline-hapi-8.12.md
```
