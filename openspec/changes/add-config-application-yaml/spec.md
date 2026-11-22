# Spec delta — `add-config-application-yaml`

This is the index to the spec delta. The **normative text** — every
`#### Scenario:` with its `WHEN`/`THEN` body — lives in one capability file,
because that is the layout `openspec validate` parses and `openspec archive`
promotes into `openspec/specs/config/spec.md`:

| Capability | Delta file | Scenarios |
|---|---|---:|
| `config` | [`specs/config/spec.md`](specs/config/spec.md) | 200 |
| **Total** | | **200** |

[`keys.md`](keys.md) is normative for the four scenarios that reference its
tables (`config.key.001`, `.003`, `.004`, `.011`) and is otherwise the key
register: every inventory row, its disposition, its canonical path, its
scenarios and the change that wires it to behaviour.

This file carries no normative text of its own. It carries the ID registry,
which is the thing a later change must not collide with.

## ID allocation

Format is `<capability>.<area>.<nnn>`, written in a delta as
`#### Scenario: [<id>] <description>`. IDs are allocated once and never
renumbered or reused, because tests reference them by ID forever.

No `config.*` ID was allocated before this change: `openspec/specs/` holds
`codegen`, `model`, `path` and `server`, and the one archived change
(`2026-11-22-add-fhirpath-and-codegen-r4`) declares only `codegen.*`,
`model.*` and `path.*`. Every area below therefore starts at `001` and is
contiguous.

| Area | What it covers | IDs | Count |
|---|---|---|---:|
| `config.dump` | the effective-configuration document and `frevo-config effective` | `001`-`006` | 6 |
| `config.source` | packaged files, locations, imports, `optional:`, precedence across source kinds | `001`-`022` | 22 |
| `config.yaml` | YAML structure, YAML 1.1 scalar resolution, flattening, YAML errors | `001`-`016` | 16 |
| `config.properties` | `.properties` syntax | `001`-`007` | 7 |
| `config.profile` | profiles, profile files and documents, the packaged `cds` and `elastic` profiles | `001`-`013` | 13 |
| `config.env` | environment-variable name mapping | `001`-`013` | 13 |
| `config.args` | command-line arguments, `SPRING_APPLICATION_JSON` | `001`-`007` | 7 |
| `config.placeholder` | `${...}` resolution, lenient and strict, `random.*` | `001`-`011` | 11 |
| `config.relaxed` | relaxed property-name matching | `001`-`008` | 8 |
| `config.coerce` | value conversion and empty-value handling | `001`-`027` | 27 |
| `config.collection` | lists, sets and maps across sources | `001`-`006` | 6 |
| `config.object` | when nested objects exist | `001`-`005` | 5 |
| `config.unknown` | unknown, unbound, setter-less and not-yet-wired keys | `001`-`007` | 7 |
| `config.error` | failure reports and early exit | `001`-`006` | 6 |
| `config.read` | keys the starter reads by name | `001`-`007` | 7 |
| `config.gate` | the starter's bean conditions | `001`-`020` | 20 |
| `config.key` | every key's defaults and round-trips; the inventory | `001`-`011` | 11 |
| `config.oracle` | the `config-roundtrip` harness | `001`-`005` | 5 |
| `config.perf` | load and access budgets | `001`-`003` | 3 |
| **Total** | | | **200** |

## Scenarios that depend on an open question

They state the recommended option of [`proposal.md`](proposal.md) and say
"pending Open Question N". If the owner chooses otherwise, the scenario is
rewritten in place under the same ID.

| Open question | Scenarios |
|---|---|
| 1 — the observable surface | `config.dump.001`-`.006` |
| 2 — `classpath:` | `config.source.017` |
| 3 — reproducing hazards | `config.read.005`, `config.gate.003`, `config.placeholder.009` |
| 4 — not-yet-wired keys | `config.unknown.007` |
| 6 — Java system properties | `config.source.019` (the gap in the order) |
| 7 — `configtree:`, `.xml` | `config.source.020` |
| 8 — environment case on Linux | `config.env.004` |
| 9 — report text | `config.error.002` and every quoted `Reason:` |
| 10 — two non-kebab spellings | `config.relaxed.007` |
