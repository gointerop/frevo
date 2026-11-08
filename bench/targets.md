# Performance targets

**Status: accepted by the owner, 2026-11-08.** These are gate conditions: a
change on a read, search or ingest path must meet the budget for its workload
before it can be archived.

They remain open to revision on evidence. The ratios were derived from a
baseline on the small dataset, and the first real implementation
(`add-fhirpath-and-codegen-r4`, then `add-store-crud`) is what will show
whether they are too soft or too hard. Revising one is a normal decision with
the measurement attached, not an admission of failure -- what is not
acceptable is quietly missing one.

Derived from [`baseline-hapi-8.12.md`](baseline-hapi-8.12.md), measured on the
same machine, the same dataset and the same database tuning.

## Measurement conditions, stated first

Everything below is relative to a baseline measured on:

- Docker Desktop on Windows, Linux VM with **12 CPUs and 8 GiB RAM**
- `synthea-small`: 112 Patients, 39,842 Observations, 81k resources in total
- PostgreSQL 17.8 with identical tuning for both servers, `fsync` **on**
- warmup issued and discarded (the JVM demands it)

Two caveats that limit how far these numbers can be pushed:

1. **The baseline is the small dataset.** A 1,000-patient medium set is being
   measured separately; index selectivity and buffer-cache hit rates change
   materially once the working set stops fitting in cache, so the ratios below
   will need re-checking against it. Targets are expressed as ratios partly for
   that reason.

   The medium set was originally specified as 10,000 patients. Generating that
   produced 31 GB of bundles before any of it reached a database, and loading
   it into HAPI's PostgreSQL needs several times that again -- more than the
   measurement machine has. A baseline nobody can reproduce is not a baseline,
   so **medium is 1,000 patients** (about 400k resources) and 10,000 is kept as
   `large` for a machine or CI runner with the disk to host it.
2. **`search-name` recorded 6 transport errors in 500 requests (1.2%)** and
   `search-chained` 2 in 200. Those are client-side connection failures under
   concurrency, not HTTP errors. Their latency figures are usable; their
   throughput figures are slightly pessimistic for HAPI.

## What HAPI 8.12.0 does today

| Workload | p50 ms | p95 ms | p99 ms | rps | CPU peak % | RSS MiB |
|---|---:|---:|---:|---:|---:|---:|
| read-by-id | 42.4 | 69.0 | 82.8 | 354 | 298 | 3405 |
| vread | 27.7 | 56.0 | 75.8 | 252 | 169 | 3414 |
| search-token | 22.1 | 33.3 | 38.8 | 342 | 198 | 3417 |
| search-name | 27.9 | 45.3 | 58.5 | 254 | 279 | 3417 |
| search-date-range | 42.7 | 63.3 | 93.4 | 175 | 381 | 3420 |
| search-patient-observations | 64.9 | 101.3 | 122.0 | 115 | 429 | 3421 |
| search-sorted | 57.5 | 77.1 | 85.5 | 136 | 379 | 3424 |
| search-total-accurate | 21.3 | 35.3 | 44.6 | 341 | 133 | 3424 |
| search-chained | 50.3 | 79.1 | 97.7 | 148 | 439 | 3424 |
| search-include | 63.2 | 81.7 | 95.9 | 124 | 392 | 3426 |
| **search-revinclude** | **124.4** | **376.6** | **465.8** | **50** | **691** | 3427 |
| $everything | 121.7 | 166.1 | 175.7 | 31 | 313 | 3427 |
| transaction-ingest | 32.9 | 48.7 | 111.7 | 221 | 168 | 3427 |
| create-single | 26.7 | 38.2 | 134.7 | 271 | 71 | 3427 |

Bulk ingest: 112 Synthea transaction bundles (≈81k resources) in **86.9 s** —
1.3 bundles/s, ≈930 resources/s, at concurrency 6.

## Proposed targets

Each target is a **gate condition**: the named workload must meet it on the
same dataset and hardware before the change that implements the capability can
be archived.

### Footprint — the easiest and most valuable win

| Metric | HAPI 8.12.0 | Frevo target | Why |
|---|---:|---:|---|
| RSS, steady state under load | 3,427 MiB | **≤ 256 MiB** | No JVM heap, no ORM session cache, no second-level cache. 13× reduction. |
| RSS, idle | ~3,400 MiB | **≤ 64 MiB** | |
| Container image | ~700 MiB | **≤ 32 MiB** | Already 8.86 MiB with a placeholder binary; the budget allows for real capability. |
| Cold start to first served request | ~90 s | **≤ 1 s** | Measured by the harness's readiness wait. |

Footprint is where the architectural choice is least arguable, so these are the
strictest targets and the ones I would least want relaxed.

### Latency and throughput

Stated as p99 ceilings (what a deployment can promise) plus a throughput ratio
(what the design should deliver).

| Workload | p99 target | Throughput target | Ratio |
|---|---:|---:|---:|
| read-by-id | **≤ 5 ms** | ≥ 5,000 rps | 14× |
| vread | **≤ 6 ms** | ≥ 4,000 rps | 16× |
| search-token | **≤ 8 ms** | ≥ 3,000 rps | 9× |
| search-name | **≤ 10 ms** | ≥ 2,000 rps | 8× |
| search-date-range | **≤ 15 ms** | ≥ 1,200 rps | 7× |
| search-patient-observations | **≤ 25 ms** | ≥ 700 rps | 6× |
| search-sorted | **≤ 25 ms** | ≥ 800 rps | 6× |
| search-total-accurate | **≤ 10 ms** | ≥ 2,000 rps | 6× |
| search-chained | **≤ 30 ms** | ≥ 800 rps | 5× |
| search-include | **≤ 30 ms** | ≥ 700 rps | 6× |
| search-revinclude | **≤ 60 ms** | ≥ 400 rps | 8× |
| $everything | **≤ 80 ms** | ≥ 200 rps | 6× |
| transaction-ingest | **≤ 40 ms** | ≥ 1,000 rps | 5× |
| create-single | **≤ 20 ms** | ≥ 1,500 rps | 6× |
| bulk ingest | — | **≥ 5,000 resources/s** | 5× |

### Where the ratios come from

These are not round numbers chosen for ambition. Each follows from a specific
design decision in the constitution:

- **read-by-id at 14×.** HAPI parses stored JSON into Hibernate entities and
  re-serializes on every read. Frevo stores canonical JSON as bytes and returns
  it without parsing, so a read is one indexed lookup plus a socket write. The
  remaining cost is almost entirely syscalls, which is why this is the largest
  ratio. If this target is missed, the central performance premise is wrong and
  that is worth knowing early.
- **Searches at 5–9×.** These are dominated by the database, which both servers
  share, so the ceiling is lower. The gain comes from extracting only enabled
  search parameters, prepared statements, keyset pagination instead of `OFFSET`,
  and not materialising result resources before serialising them.
- **`search-revinclude` at 8×.** HAPI's worst workload by a wide margin — p95
  of 377 ms and 691% CPU, which indicates a query plan that degrades under
  concurrency rather than a constant-factor cost. A better index layout should
  beat it by more than the general search ratio.
- **Ingest at 5×.** Writes are durability-bound and `fsync` stays on, so this
  is the most constrained figure. The gain is `COPY` for bulk paths and no ORM
  flush cycle.

### Regression policy, once targets are approved

- Any change touching a read, search or ingest path reports measured before and
  after figures in its pull request.
- A regression of **more than 10%** on any workload fails the performance gate.
- A regression that cannot be resolved is a checkpoint for the owner, not
  something to absorb silently.
- The baseline is re-measured whenever the HAPI image digest, the dataset, or
  the database tuning changes — all three are pinned precisely so this stays a
  deliberate act.

## Open questions

1. **Are these the right ratios?** Accepted as written, with the footprint
   targets the strongest and the throughput ratios still informed estimates
   rather than measurements of anything that exists. Re-check them against the
   medium baseline.
2. **Should the gate use the small or the medium dataset?** The medium set is
   more representative but makes every gate run far slower. Current plan: the
   small set per pull request, the medium set nightly and at phase boundaries.
3. **Is 8 GiB of VM RAM the reference environment?** HAPI was given `-Xmx3g` on
   a 12-CPU, 8 GiB VM. A larger reference machine would favour HAPI
   (more heap, more cache) and compress these ratios.
