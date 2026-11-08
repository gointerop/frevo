# Capability: hfql

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/hfql/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

HFQL, the SQL-like query language over FHIR resources.

Implemented by `crates/frevo-hfql`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-hfql`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing against HAPI's HFQL endpoint; HAPI's own HFQL parser tests mined for cases.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. HFQL is HAPI-specific with no specification. Is HAPI's parser the whole definition, including its error messages?

## Scenario ID namespace

Scenario IDs for this capability are `hfql.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
