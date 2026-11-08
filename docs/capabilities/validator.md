# Capability: validator

> **Phase 0 stub.** This records what the capability is for, where its behaviour
> is defined upstream, and how it will be graded. It carries **no requirements**
> on purpose.
>
> Requirements live in `openspec/specs/validator/spec.md`, and they get there in
> exactly one way: written in an OpenSpec change under
> `openspec/changes/<id>/`, reviewed, implemented, gated, and then promoted by
> `openspec archive`. That is why this register is under `docs/` -- a
> hand-written file in `openspec/specs/` would both fail
> `openspec validate --strict` (no requirements) and contradict the rule that
> settled specs change only through archiving.

## Purpose

Full profile validation: snapshot generation, slicing, invariants, bindings and profile resolution.

Implemented by `crates/frevo-validator`.

## Upstream source

`org.hl7.fhir.core/org.hl7.fhir.validation` (approximately 270k lines with utilities), and `hapi-fhir/hapi-fhir-validation`.

Pinned commits are in [`reference/PINS.md`](../../reference/PINS.md). Any Rust
file derived from upstream source carries a provenance header and a row in
[`docs/provenance.md`](../provenance.md).

## Oracles

`HL7/fhir-test-cases` validator suite, vendored under `oracles/fhir-test-cases`. The gate is parity with HAPI's pass rate, including issue severities.

HAPI FHIR 8.12.0 is the reference behaviour. A difference from HAPI is a bug
unless it is recorded in [`docs/deviations.md`](../deviations.md).

## Open questions

1. Must OperationOutcome issue text match HAPI's exactly, or only the severity and location? Message text parity is far more expensive.
2. Is Schematron validation reachable in the starter's configuration, and is it worth implementing or a known gap?

## Scenario ID namespace

Scenario IDs for this capability are `validator.<area>.<nnn>`. IDs are allocated
once and never renumbered or reused, because tests reference them by ID.
