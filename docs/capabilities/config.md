# Capability: config

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/config/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

`application.yaml`-compatible configuration with the starter's `hapi.fhir.*` key names and semantics.

Implemented by `crates/frevo-config`.

## Upstream source

`hapi-fhir-jpaserver-starter/.../AppProperties.java`, plus the starter's `application.yaml`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Every key in `docs/inventory.md` round-trips: parsed from the starter's own configuration files and observably affecting behaviour.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Spring relaxed binding accepts several spellings of each key (kebab-case, snake_case, camelCase, environment variables). Which forms must be supported?
2. Which keys are accepted but ignored by the starter, and should those be accepted-and-ignored here too rather than rejected?

## Scenario ID namespace

Scenario IDs for this capability are `config.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
