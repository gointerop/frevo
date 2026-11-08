# Capability: package

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/package/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

FHIR NPM package and implementation-guide installation, including transitive dependencies.

Implemented by `crates/frevo-package`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-base/.../packages`, and `org.hl7.fhir.core/.../npm`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Install a known IG set at startup and compare the resulting StructureDefinition and ValueSet inventory against HAPI's.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Where are packages cached, and is the cache layout required to be compatible with HAPI's?
2. How are version conflicts between transitive IG dependencies resolved, and does HAPI's resolution order need matching exactly?

## Scenario ID namespace

Scenario IDs for this capability are `package.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
