# Capability: cr

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/cr/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Clinical reasoning: measures, care gaps, data requirements, PlanDefinition and ActivityDefinition `$apply`, SDC and CDS Hooks.

Implemented by `crates/frevo-cr`.

## Upstream source

`cqframework/clinical-reasoning` (`cqf-fhir-cr`, approximately 65k lines), as wired by the starter's `cqf-fhir-cr-hapi`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Measure evaluation fixtures from the clinical-reasoning repository, plus differential comparison of MeasureReport output.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. MeasureReport output includes computed scores. Which fields are stable enough to diff, and which need normalisation?
2. Do CDS Hooks discovery and feedback endpoints need to match the starter's shape exactly?

## Scenario ID namespace

Scenario IDs for this capability are `cr.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
