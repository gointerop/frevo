# Capability: jobs

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/jobs/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Batch job engine on PostgreSQL `SKIP LOCKED`, with bulk export and import, reindex, delete-expunge and the MDM jobs.

Implemented by `crates/frevo-jobs`.

## Upstream source

`hapi-fhir/hapi-fhir-storage-batch2`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing of `$export`, `$import` and `$reindex` outcomes; Inferno Bulk Data kit for export conformance.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Must job and chunk state be observable in the same shape HAPI exposes through `$job-status`?
2. How is a partially completed export resumed after a restart?

## Scenario ID namespace

Scenario IDs for this capability are `jobs.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
