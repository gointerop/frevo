# Capability: subscription

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/subscription/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Subscriptions: REST-hook, websocket, email and message delivery, plus R5 topic-based subscriptions and the R4/R4B backport.

Implemented by `crates/frevo-subscription`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-subscription`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing of subscription matching and delivery; the topic-based subscription specification's own test cases.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Delivery moves to a PostgreSQL outbox with `LISTEN`/`NOTIFY`. What are the at-least-once guarantees, and does HAPI promise more?
2. How is cross-partition subscription matching scoped?

## Scenario ID namespace

Scenario IDs for this capability are `subscription.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
