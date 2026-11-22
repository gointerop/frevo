# Spec delta — `add-config-application-yaml`

This is the index to the spec delta. The **normative text** — every
`#### Scenario:` with its `WHEN`/`THEN` body — lives in one capability file,
because that is the layout `openspec validate` parses and `openspec archive`
promotes into `openspec/specs/config/spec.md`:

| Capability | Delta file | Scenarios |
|---|---|---:|
| `config` | [`specs/config/spec.md`](specs/config/spec.md) | 204 |
| **Total** | | **204** |

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
| `config.source` | packaged files, locations, imports, `optional:`, precedence across source kinds, JVM option variables | `001`-`024` | 24 |
| `config.yaml` | YAML structure, YAML 1.1 scalar resolution, flattening, YAML errors | `001`-`016` | 16 |
| `config.properties` | `.properties` syntax | `001`-`007` | 7 |
| `config.profile` | profiles, profile files and documents, the packaged `cds` and `elastic` profiles, `on-cloud-platform` | `001`-`014` | 14 |
| `config.env` | environment-variable name mapping | `001`-`014` | 14 |
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
| **Total** | | | **204** |

## Revision after the owner's rulings of 2026-11-22

Every open question of the first draft was decided (proposal.md,
"Decisions"). **4 IDs were added** at the end of their areas:
`config.source.023` and `.024` (Java system properties from the `-D` tokens of
`JAVA_TOOL_OPTIONS` and `JDK_JAVA_OPTIONS`), `config.profile.014`
(`on-cloud-platform` never activates) and `config.env.014` (an environment
variable named `--spring.config.location`). **1 scenario was rewritten in
place**: `config.unknown.007` now states that a key this build does not act
on yet is accepted silently, as in HAPI, instead of warned about. The other
scenarios that said "pending" keep their behaviour and now cite their register
entry or ruling. None was removed, renumbered, reused or repurposed.

## Scenarios that carry a register entry

| Entry | Scenarios |
|---|---|
| `DEV-0031` — unsupported location kinds, `on-cloud-platform` | `config.source.017`, `config.source.020`, `config.profile.014` |
| `DEV-0032` — two non-kebab spellings in one source | `config.relaxed.007` |
| `DEBT-0022` — named reads split from binding | `config.read.005`, `config.read.002`, `config.gate.006`, `config.gate.007` |
| `DEBT-0023` — empty `fhir_version` selects DSTU2 | `config.gate.003` |
| `DEBT-0024` — environment variables expand into the request-log templates | `config.placeholder.009` |
| `DEBT-0025` — `cr.cql.compiler.error_level` accepted and ignored | `config.relaxed.004` |
| `DEBT-0026` — `cr.cql.runtime.enable_validation` cannot be enabled | `config.unknown.005` |

The entries' text is in [`register-entries.md`](register-entries.md).
