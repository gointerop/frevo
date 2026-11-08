# Capability: codegen

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/codegen/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Generates Rust models, codecs and search-parameter tables from FHIR StructureDefinitions and SearchParameter definitions.

Implemented by `crates/frevo-codegen`.

## Upstream source

`hapi-fhir/hapi-tinder-plugin` (HAPI's own generator), and the StructureDefinition packages that `org.hl7.fhir.core` consumes.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Round-trip tests over every example in the R4 specification package: parse, re-serialize, and compare byte-for-byte after canonicalisation.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Which StructureDefinition package version is authoritative for R4 generation, and how is it pinned so output is reproducible?
2. Are extensions modelled as typed structures or left as raw elements until a profile needs them?

## Scenario ID namespace

Scenario IDs for this capability are `codegen.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
