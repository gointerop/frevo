# 7. The YAML parser for frevo-config

- **Status:** accepted by owner 2026-11-22 (option A), relayed by the
  coordinator. Constitution section 5 requires an ADR and the owner's
  approval for any new runtime dependency; both now exist. The file stays a
  draft file inside the change directory, because the proposer writes nothing
  outside it, until the owner moves it to `docs/adr/0007-yaml-event-parser.md`.
- **Date:** 2026-11-22
- **Context phase:** Phase 1, `add-config-application-yaml` task 7

## Context

`frevo-config` must read the starter's `application.yaml` and every operator
YAML file exactly as HAPI FHIR JPA Server Starter 8.12.0 does. The starter runs
on Spring Boot 3.5.14, whose `OriginTrackedYamlLoader` drives SnakeYAML 2.4
with YAML 1.1 resolution minus timestamps, rejects duplicate keys, keeps the
line and column of every value for error reports, and flattens the result
with Spring's `YamlProcessor`.

YAML 1.1 resolution is observable in the starter's configuration, because
Spring converts the *resolved* value to the target type. Measured with the
starter's own classes (design.md section 2):

| Written | SnakeYAML 1.1 resolves | Bound to a `String` property | YAML 1.2 (core schema) would give |
|---|---|---|---|
| `off` | boolean `false` | `"false"` | `"off"` |
| `yes` | boolean `true` | `"true"` | `"yes"` |
| `010` | int 8 | `"8"` | int 10 → `"10"` |
| `1:20` | int 80 | `"80"` | `"1:20"` |
| `1_000` | int 1000 | `"1000"` | `"1_000"` |
| `1.10` | float 1.1 | `"1.1"` | float 1.1 → depends on renderer |
| `0b101` | int 5 | `"5"` | `"0b101"` |
| `profile_mode: OFF` | boolean `false` → enum `OFF` | — | `"OFF"` → enum `OFF` |
| `graphql_enabled: 1` | int 1 → **startup failure** | — | int 1 → failure |

So whatever parses the YAML must hand Frevo each plain scalar's **raw text**
and style, and Frevo must resolve it by SnakeYAML's rules. It must also report
**positions** (`Origin: URL [file:config/application.yaml] - 3:22`), support
anchors, aliases and `<<` merge keys, multi-document streams, and let Frevo
detect duplicate keys.

## Options

### A. `yaml-rust2` (event parser only) plus Frevo's own resolver

Pure Rust, MIT OR Apache-2.0. Its `parser::Parser` emits
`Event::Scalar(value, style, anchor, tag)` with a `Marker` (index, line,
column) for every event, before any type resolution, plus `Alias`,
`MappingStart`, `SequenceStart` and document events. Frevo builds its own
node tree from events, resolves plain scalars with a table ported from
SnakeYAML 2.4's `Resolver` (bool, int, float, null, merge; no timestamp),
expands merge keys, rejects duplicate keys, and flattens.

- Good: everything observable is under Frevo's control; no `unsafe`; the
  crate is a widely used pure-Rust YAML parser (its licence and maintenance
  are to be confirmed by `cargo deny` when task 7 adds it); the scanner is YAML 1.2 but the 1.1-vs-1.2 differences
  are almost all in resolution, which Frevo does itself.
- Bad: a dependency in the product binary; the residual scanner differences
  (YAML 1.1 allowed a few escapes and indentation forms 1.2 dropped) must be
  found by the oracle, not assumed absent.

### B. `saphyr-parser`

The maintained successor of the same code base, same event model. Younger and
with a smaller user base; otherwise equivalent to A. A reasonable switch later
if `yaml-rust2` stalls.

### C. Hand-written YAML parser

No dependency. But a YAML block-and-flow parser with anchors, merge keys,
folded and literal block scalars and correct positions is the largest single
piece of work in this change, and every bug in it is a parity bug.

### D. `serde_yaml_ng` (already listed, unused, in the workspace `Cargo.toml`)

Built on `unsafe-libyaml` (a C-to-Rust translation that is `unsafe`
throughout) and resolves scalars by YAML 1.2 rules during deserialisation,
before Frevo can see the text. Every row of the table above but the last would
be wrong, and the original text cannot be recovered. Ruled out.

## Decision (accepted by owner 2026-11-22)

**Option A.** Add `yaml-rust2` (event parser only; no use of its `Yaml` value
type, whose resolution is YAML 1.2) to `frevo-config`, and remove the unused
`serde_yaml_ng` entry from the workspace dependencies. JSON for
`SPRING_APPLICATION_JSON` uses the existing `serde_json`.

## Consequences

- Good: YAML 1.1 parity is a table Frevo owns and the oracle grades; positions
  are exact; no `unsafe`; PostgreSQL remains the only runtime *service*.
- Bad: one more crate in the product and in `cargo deny`; Frevo owns a
  resolver and a flattener that a Java user gets for free.
- Neutral: the crate's own `Yaml` loader is never used, so its resolution
  choices cannot leak in; a test asserts no `yaml_rust2::Yaml` type is
  referenced outside the event adapter.

## What would make us revisit it

- The oracle finds a scanner-level difference from SnakeYAML that cannot be
  corrected after the event stream (then C, or a patch upstream).
- `yaml-rust2` becomes unmaintained (then B; the event API is the same family).
