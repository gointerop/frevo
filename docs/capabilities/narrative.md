# Capability: narrative

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/narrative/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Narrative generation from resource content.

Implemented by `crates/frevo-narrative`.

## Upstream source

`hapi-fhir/hapi-fhir-base/.../narrative`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential comparison of generated narrative, normalised for whitespace.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. HAPI uses Thymeleaf templates; Frevo uses minijinja. Byte-identical narrative is likely unachievable. Is this a deviation or a configuration-gated difference?

## Scenario ID namespace

Scenario IDs for this capability are `narrative.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
