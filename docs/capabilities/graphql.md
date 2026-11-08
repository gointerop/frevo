# Capability: graphql

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/graphql/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

GraphQL endpoint over the FHIR repository.

Implemented by `crates/frevo-graphql`.

## Upstream source

`hapi-fhir/hapi-fhir-server/.../graphql`, and `org.hl7.fhir.core/.../graphql`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing of GraphQL queries against HAPI's endpoint.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Is the schema generated from StructureDefinitions, and must it match HAPI's generated schema exactly?

## Scenario ID namespace

Scenario IDs for this capability are `graphql.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
