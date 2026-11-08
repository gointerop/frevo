# Capability: path

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/path/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

FHIRPath parser and evaluation engine.

Implemented by `crates/frevo-path`.

## Upstream source

`org.hl7.fhir.core/.../utils/FHIRPathEngine.java`, and `hapi-fhir/hapi-fhir-base/.../fhirpath`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

The official FHIRPath test suite, vendored under `oracles/fhirpath`. The gate is parity with HAPI 8.12.0's pass rate on the same cases.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Which FHIRPath version does HAPI 8.12.0 implement, and which non-standard functions does it add?
2. Is evaluation needed over the lazy model without full materialisation, and at what cost?

## Scenario ID namespace

Scenario IDs for this capability are `path.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
