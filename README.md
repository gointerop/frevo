# Frevo

**Frevo — a high-performance HL7® FHIR® server in Rust, compatible with HAPI FHIR JPA Server Starter configuration.**

[![CI](https://github.com/gointerop/frevo/actions/workflows/ci.yml/badge.svg)](https://github.com/gointerop/frevo/actions/workflows/ci.yml)
[![OpenSpec Guard](https://github.com/gointerop/frevo/actions/workflows/openspec-guard.yml/badge.svg)](https://github.com/gointerop/frevo/actions/workflows/openspec-guard.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Frevo is a from-scratch Rust implementation of the feature surface exposed by
HAPI FHIR JPA Server Starter 8.12.0, on PostgreSQL, built for high throughput and
a small footprint. It reads the starter's `application.yaml` (`hapi.fhir.*` keys,
same names and semantics) so an existing deployment can switch without changing
its configuration.

> **Status: Phase 0 (bootstrap).** No FHIR capabilities are implemented yet. The
> published image serves `/fhir/metadata` with a CapabilityStatement that
> advertises no resources — deliberately, so it never claims capability it does
> not have. Follow the parity table below for progress.

## Why

A FHIR server's cost is dominated by parsing, serializing and indexing
resources. Frevo stores canonical resource JSON as bytes and serves reads without
reparsing, parses lazily when a value is actually needed, and indexes only the
search parameters that are enabled. PostgreSQL is the only runtime dependency:
full-text search, messaging, and the batch job engine are embedded rather than
delegated to Elasticsearch, a broker, or an external scheduler.

## Quick start

```bash
docker run --rm -p 8080:8080 ghcr.io/gointerop/frevo:edge
```

```bash
curl -s http://localhost:8080/fhir/metadata | jq .
```

With PostgreSQL, using the bundled compose file:

```bash
docker compose up
```

## Correctness

Specification prose is not the source of truth here. A change is correct when it
passes, in this order of authority:

1. the official HL7 test suites (FHIRPath, `fhir-test-cases`, `cql-tests`,
   Inferno kits), vendored and pinned under `oracles/`;
2. **differential testing against a live HAPI FHIR 8.12.0** — identical requests
   to both servers, responses diffed after normalization;
3. HAPI's own JUnit tests, mined for HTTP-level behaviour.

Where HAPI and the specification disagree, the case is recorded in
[`docs/deviations.md`](docs/deviations.md) with the behaviour chosen and why.
Anything that cannot be made to pass is recorded in
[`docs/known-gaps.md`](docs/known-gaps.md) rather than silently skipped.

## Parity

<!-- PARITY-TABLE:BEGIN -->
_Generated from `docs/inventory.md` by `harness/scripts/parity-table.sh`._
<!-- PARITY-TABLE:END -->

## How this repository is developed

Every change is specified before it is built, using
[OpenSpec](https://github.com/fission-ai/openspec): proposal → spec delta with
scenario IDs → design → tasks → tests → implementation → gates → archive. Tests
are written before the implementation, from the spec scenarios and the oracles.
The rules are in [`CLAUDE.md`](CLAUDE.md) (the same file is published as
[`AGENTS.md`](AGENTS.md)), and they are enforced by repository hooks and by the
`openspec-guard` CI job, not by convention.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) to work on it.

## Layout

| Path | Contents |
|---|---|
| `crates/` | the workspace: one crate per capability, plus `frevo-server` |
| `harness/` | differential runner, response normalizer, request corpora, compose files |
| `oracles/` | vendored HL7/CQL/Inferno suites, pinned |
| `bench/` | workloads and recorded baselines |
| `openspec/` | capability specs and in-flight changes |
| `docs/` | ADRs, feature inventory, deviations, known gaps, provenance |
| `reference/` | pinned upstream sources, read-only and never compiled |
| `tasks/` | the dependency-aware work queue |

## License and attribution

Apache-2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Frevo is a port of,
and derives from, HAPI FHIR (University Health Network / Smile Digital Health),
HL7 `org.hl7.fhir.core`, and the cqframework CQL and clinical-reasoning projects,
all Apache-2.0. Per-file and per-crate provenance is recorded in
[`docs/provenance.md`](docs/provenance.md).

HL7®, FHIR® and the FHIR flame design are registered trademarks of Health Level
Seven International, and their use does not constitute endorsement by HL7. Frevo
does not use the FHIR flame design. HAPI FHIR is a project of University Health
Network / Smile Digital Health; Frevo is independent of and not endorsed by them.
