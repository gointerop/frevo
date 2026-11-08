# Capability: terminology

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/terminology/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Terminology store, code-system loaders, and the terminology operations.

Implemented by `crates/frevo-terminology`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-base/.../term`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Specification-provided ValueSet expansion fixtures, plus differential testing of `$expand`, `$validate-code`, `$lookup`, `$subsumes` and `$translate` against HAPI.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. SNOMED CT, LOINC and ICD-10-CM content cannot be redistributed. Which cases can run on redistributable samples, and which become environment-conditional?
2. Does pre-expansion need to match HAPI's row layout, or only its observable expansion output?

## Scenario ID namespace

Scenario IDs for this capability are `terminology.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
