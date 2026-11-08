# Capability: store

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/store/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Resource persistence on PostgreSQL: CRUD, vread, history, transactions, conditional operations and referential integrity.

Implemented by `crates/frevo-store`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-base/.../dao`, and the `HFJ_*` schema.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing against live HAPI 8.12.0 over a request corpus; HAPI's own DAO JUnit tests mined for HTTP-level behaviour.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Does the schema need to match `HFJ_*` for the migration tool, or can it be native with a converter? This is the single biggest design decision in the project.
2. How are placeholder references and conditional-create identifiers resolved inside a transaction bundle, in HAPI's exact order?

## Scenario ID namespace

Scenario IDs for this capability are `store.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
