# Capability: server

## Purpose

HTTP surface, interceptor pipeline and capability wiring.

Implemented by `crates/frevo-server`.

Unlike the other entries in this register, this capability already has
requirements: see [`openspec/specs/server/spec.md`](../../openspec/specs/server/spec.md).
It is the one spec written by hand, during Phase 0 bootstrap, before enforcement
existed -- it records the single behaviour the bootstrap build actually has
(`GET /fhir/metadata`), and it validates strictly. Every later requirement in
this or any other capability arrives through `openspec archive`.

## Upstream source

`hapi-fhir/hapi-fhir-server` (the provider and interceptor framework), and the
starter's Spring Boot application wiring. Interceptor hook parity is specified
against HAPI's `Pointcut` enumeration.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md).

## Oracles

Differential testing against live HAPI 8.12.0 for every endpoint; the
CapabilityStatement is compared structurally for the same configuration.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Interceptor hooks must reach parity with HAPI's `Pointcut` set. Which
   pointcuts are reachable from the starter's configuration, and which are
   only reachable from custom Java that has no equivalent here?
2. Does the `/fhir` base path need to be configurable in the same way, including
   the starter's servlet-path handling?

## Scenario ID namespace

Scenario IDs for this capability are `server.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
