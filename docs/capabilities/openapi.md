# Capability: openapi

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/openapi/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

OpenAPI document generation and the Swagger UI surface.

Implemented by `crates/frevo-openapi`.

## Upstream source

`hapi-fhir/hapi-fhir-server-openapi`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Structural comparison of the generated OpenAPI document against HAPI's for the same CapabilityStatement.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Is the Swagger UI itself in scope, or only the OpenAPI document?

## Scenario ID namespace

Scenario IDs for this capability are `openapi.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
