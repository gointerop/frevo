# Capability: search

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/search/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Search-parameter indexing and query planning: all parameter types, modifiers, prefixes, chaining, `_has`, includes, composites and sorting.

Implemented by `crates/frevo-search`.

## Upstream source

`hapi-fhir/hapi-fhir-jpaserver-base/.../search`, and the `HFJ_SPIDX_*` tables.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

Differential testing against live HAPI; combo-unique and uplifted refchain behaviour needs cases derived from HAPI's own tests.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Which index layout serves chained search and `_has` without the join explosion HAPI suffers?
2. Is `_total=accurate` required by default, and what does it cost against keyset pagination?
3. Fulltext `_content` and `_text` move to tantivy; how is index consistency maintained across a crash?

## Scenario ID namespace

Scenario IDs for this capability are `search.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
