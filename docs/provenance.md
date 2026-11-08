# Provenance

Every crate that contains logic derived from an upstream Apache-2.0 project is
listed here with the upstream module and the pinned commit it was derived from.
Pins are defined in [`reference/PINS.md`](../reference/PINS.md).

Apache-2.0 section 4 obligations this file helps discharge:

- derived files carry a notice of modification,
- upstream copyright and attribution notices are retained (see [`NOTICE`](../NOTICE)),
- the origin of each derived work is identifiable.

## Required header on derived files

Any Rust file whose logic was derived from upstream Java starts with:

```rust
// Derived from <upstream repo>/<path> at <commit SHA>.
// Original work Copyright <upstream copyright holder>, licensed under the
// Apache License, Version 2.0.
// Modified by the Frevo authors: <one line on what changed>.
```

Generated code instead carries an `@generated` marker and names the generator
and the input StructureDefinition package; generated model code is **not** a
translation of upstream Java and is not listed as derived.

## Crate provenance

Status values: `generated` (produced by `frevo-codegen` from specification
artifacts), `derived` (logic ported from upstream source), `independent` (written
against the specification and the oracles, not from upstream source), `empty`
(Phase 0 placeholder).

| Crate | Status | Upstream repo | Upstream module(s) | Pinned SHA |
|---|---|---|---|---|
| `frevo-codegen` | empty | — | — | — |
| `frevo-model` | empty | — | (will be `generated` from R4 StructureDefinitions) | — |
| `frevo-path` | empty | — | — | — |
| `frevo-convert` | empty | — | — | — |
| `frevo-package` | empty | — | — | — |
| `frevo-validator` | empty | — | — | — |
| `frevo-terminology` | empty | — | — | — |
| `frevo-store` | empty | — | — | — |
| `frevo-search` | empty | — | — | — |
| `frevo-jobs` | empty | — | — | — |
| `frevo-subscription` | empty | — | — | — |
| `frevo-mdm` | empty | — | — | — |
| `frevo-cql` | empty | — | — | — |
| `frevo-cr` | empty | — | — | — |
| `frevo-ips` | empty | — | — | — |
| `frevo-graphql` | empty | — | — | — |
| `frevo-hfql` | empty | — | — | — |
| `frevo-narrative` | empty | — | — | — |
| `frevo-openapi` | empty | — | — | — |
| `frevo-mcp` | empty | — | — | — |
| `frevo-config` | empty | — | — | — |
| `frevo-server` | independent | — | — | — |

No crate contains derived upstream logic yet. Each row is updated by the change
that first ports into that crate; `openspec-guard` checks that a crate whose
sources gained a provenance header also gained a row here.
