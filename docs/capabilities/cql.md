# Capability: cql

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/cql/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

CQL to ELM translation and the ELM evaluation engine.

Implemented by `crates/frevo-cql`.

## Upstream source

`cqframework/clinical_quality_language` (approximately 53k lines).

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

`cqframework/cql-tests`, vendored under `oracles/cql-tests`. The gate is parity with the reference engine, which does not pass every case itself.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Is a full CQL-to-ELM translator needed, or can measures be evaluated from pre-translated ELM with translation as a separate tool?
2. How is the ELM JSON representation versioned against the translator?

## Scenario ID namespace

Scenario IDs for this capability are `cql.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
