# Capability: mcp

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/mcp/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Model Context Protocol server bridging FHIR and CDS Hooks.

Implemented by `crates/frevo-mcp`.

## Upstream source

The starter's MCP module, which depends on Spring AI.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

MCP protocol conformance against the specification; tool behaviour compared with the starter's.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Which MCP specification revision does the starter implement?
2. Is the Spring AI dependency's tool surface part of the parity requirement, or is the protocol enough?

## Scenario ID namespace

Scenario IDs for this capability are `mcp.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
