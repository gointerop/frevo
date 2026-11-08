# Capability: model

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/model/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Generated FHIR resource and datatype models with a lazy, byte-preserving JSON codec.

Implemented by `crates/frevo-model`.

## Upstream source

`org.hl7.fhir.core/org.hl7.fhir.r4/model` -- regenerated from StructureDefinitions, never translated.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

R4 specification examples, round-tripped; the validator suite exercises the models indirectly.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. How is the original byte range of an unmodified element retained so a read can serve stored JSON without reserialising?
2. What is the representation for primitive extensions (the `_field` form) that keeps the lazy codec cheap?

## Scenario ID namespace

Scenario IDs for this capability are `model.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
