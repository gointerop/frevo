# Spec delta — `add-config-application-yaml`

This is the index to the spec delta. The **normative text** — every
`#### Scenario:` with its `WHEN`/`THEN` body — lives in one capability file,
because that is the layout `openspec validate` parses and `openspec archive`
promotes into `openspec/specs/config/spec.md`:

| Capability | Delta file | Scenarios |
|---|---|---:|
| `config` | [`specs/config/spec.md`](specs/config/spec.md) | 232 |
| **Total** | | **232** |

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
| `config.source` | packaged files, locations, imports, `optional:`, precedence across source kinds, JVM option variables, `/app/extra-classes/`, location syntax | `001`-`030` | 30 |
| `config.yaml` | YAML structure, YAML 1.1 scalar resolution, flattening, YAML errors | `001`-`016` | 16 |
| `config.properties` | `.properties` syntax | `001`-`007` | 7 |
| `config.profile` | profiles, profile files and documents, the packaged `cds` and `elastic` profiles, `on-cloud-platform`, profile-name validation | `001`-`018` | 18 |
| `config.env` | environment-variable name mapping | `001`-`014` | 14 |
| `config.args` | command-line arguments, `spring.application.json` | `001`-`012` | 12 |
| `config.placeholder` | `${...}` resolution, lenient and strict, `random.*`, JVM system properties | `001`-`013` | 13 |
| `config.relaxed` | relaxed property-name matching | `001`-`008` | 8 |
| `config.coerce` | value conversion and empty-value handling | `001`-`027` | 27 |
| `config.collection` | lists, sets and maps across sources | `001`-`006` | 6 |
| `config.object` | when nested objects exist | `001`-`005` | 5 |
| `config.unknown` | unknown, unbound, setter-less and not-yet-wired keys | `001`-`008` | 8 |
| `config.error` | failure output and early exit | `001`-`010` | 10 |
| `config.read` | keys the starter reads by name | `001`-`008` | 8 |
| `config.gate` | the starter's bean conditions and HAPI's `scheduling_disabled` | `001`-`024` | 24 |
| `config.key` | every key's defaults and round-trips; the inventory | `001`-`012` | 12 |
| `config.oracle` | the `config-roundtrip` harness | `001`-`005` | 5 |
| `config.perf` | load and access budgets | `001`-`003` | 3 |
| **Total** | | | **232** |

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

## Revision after the review of 2026-11-22 (`review.md`, 13 findings)

**28 IDs were added** at the end of their areas: `config.source.025`-`.030`,
`config.profile.015`-`.018`, `config.args.008`-`.012`,
`config.placeholder.012`-`.013`, `config.unknown.008`,
`config.error.007`-`.010`, `config.read.008`, `config.gate.021`-`.024` and
`config.key.012`. **21 scenarios were rewritten in place**, keeping their
subject: `config.dump.006`, `config.source.017`, `.020`, `.023`, `.024`,
`config.yaml.003`, `.016`, `config.properties.004`, `config.profile.012`,
`config.args.007`, `config.placeholder.011`, `config.unknown.004`,
`config.error.002`, `config.read.001`, `config.gate.001`, `.005`, `.006`,
`.020`, `config.key.009`, `config.oracle.001` and `.003`. The rewrites state
what HAPI does as measured with the starter's own conditions and
`logback.xml` (`probe/`): failure output on standard output in two forms,
`/app/extra-classes/` as part of `classpath:`, `scheduling_disabled`, the
JVM's system properties and quoting, `ElasticsearchProperties`, and the
corrected `cr.enabled: yes`. None was removed, renumbered, reused or
repurposed. The delta changed after review, so the spec-reviewer must
re-review it.

## Revision after the second review of 2026-11-22

No ID was added. **4 scenarios were rewritten in place**, keeping their
subject: `config.error.002`, `.008` and `.009` and `config.oracle.003`.
Under the owner's ruling of 2026-11-22, the failure output is reproduced
verbatim only for the failure-analysis block or exception chain and the
config-related `WARN`/`INFO` lines (with HAPI's servlet-context logger
name); stack frames and every other log line (startup info, Tomcat, servlet
context, components) are not written, and the oracle compares only the
config-related lines. `DEV-0033` is approved and its scope extended
accordingly.

## Revision after the oracle findings of 2026-11-22 (O-1 to O-13)

No ID was added or removed; the count stays 232. The `config-roundtrip`
oracle's first full run inside the pinned image (Spring Framework 6.2.19,
ruled the reference) contradicted or sharpened thirteen points, and the owner
ruled that HAPI's recorded behaviour wins (proposal.md, "Oracle findings of
2026-11-22 and the owner's rulings"). **28 scenarios were rewritten in
place**, keeping their subject:

- O-1, logback status lines: `config.dump.006`, `config.error.007`, `.009`,
  `config.oracle.003`;
- O-2, O-3, environment names: `config.env.004`, `.014`;
- O-4, O-5, recorded exception lines: `config.args.007`, `.009`,
  `config.yaml.003`, `.016`;
- O-6, no version after `spring.config.location`: `config.source.007`,
  `.009`, `.010`, `.011`, `.013`, `config.key.001`, `.003`;
- O-7, gate site names: `config.placeholder.011`;
- O-8, `Bindable@<hex>`: `config.collection.002`, `config.oracle.002`;
- O-9, the bean-creation wrapper: `config.unknown.005`, `.008`;
- O-10: `config.gate.002`; O-11: `config.key.011`; O-12:
  `config.relaxed.007`; O-13: `config.key.007`;
- the jar versions and the `javac` build of the HAPI side:
  `config.oracle.001`;
- the enum action text the record shows: `config.error.002`.

The notation comment at the top of the delta now says that failure output is
on standard output, and how a scenario that replaces the default locations
without a version is to be read.

## Revision after the fourth review (`831c5ba`) and the re-record from HAPI's real `Application`

No ID was added or removed; the count stays 232. Every failing case of the
`config-roundtrip` record now comes from the image's own
`ca.uhn.fhir.jpa.starter.Application` (runner `application`; 80 of 419
cases), and the delta was diffed against the whole record. **20 scenarios
were rewritten in place**, keeping their subject (proposal.md, "Fourth review
(`831c5ba`) and the re-record from HAPI's real `Application`"):

- refresh-time log lines, per failure kind: `config.error.002`, `.008`
  (a per-kind table of the `WARN` exception and whether the `INFO` line
  appears), `config.dump.006`, `config.collection.002`, `config.gate.024`;
- the missing version: `config.gate.005` and the "fails as config.gate.005
  does" definition in the notation comment, `config.env.014`;
- strict and lenient placeholders: `config.placeholder.007`, `.011`,
  `config.read.007`;
- the CQL beans: `config.unknown.005`, `.008`;
- Jackson's location line and tab-led frames: `config.args.007`, `.009`,
  `config.error.007`, `.009`, `config.oracle.003`;
- the two runners: `config.oracle.001`;
- other clauses the record contradicted or sharpened: `config.source.016`
  (the action line), `config.source.020` (the `x.txt` cause differs from the
  `configserver:` one);
- the notation comment's reading rule (which cases set the version in
  their arguments, which in the named file), touching the text that
  `config.source.007`, `.009`, `.010`, `.011`, `.013` and `config.key.001`
  rely on without changing those scenarios' clauses.

config.oracle.003 no longer exempts the `WARN`/`INFO` logger field or
shortens the bean-creation wrapper: both rules existed only because the
record was the probe's. The `Bindable@<hex>` mask stays.

## Scenarios that carry a register entry

| Entry | Scenarios |
|---|---|
| `DEV-0031` — `configtree:`, `.xml`, `on-cloud-platform` (narrowed) | `config.source.020`, `config.profile.014` |
| `DEV-0032` — two non-kebab spellings in one source | `config.relaxed.007` |
| `DEV-0033` — failure output keeps only the config-related lines (no stack frames, no other log lines, no logback status lines); owner approved 2026-11-22; replacement text restated after the fourth review, still to be applied to `docs/deviations.md` | `config.dump.006`, `config.error.007`, `config.error.008`, `config.error.009`, `config.oracle.003` |
| `DEBT-0022` — named reads split from binding | `config.read.002`, `config.read.005`, `config.gate.006`, `config.gate.007` |
| `DEBT-0023` — empty `fhir_version` selects DSTU2 | `config.gate.003` |
| `DEBT-0024` — environment variables expand into the request-log templates | `config.placeholder.009` |
| `DEBT-0025` — `cr.cql.compiler.error_level` accepted and ignored | `config.relaxed.004` |
| `DEBT-0026` — `cr.cql.runtime.enable_validation` cannot be enabled | `config.unknown.005` |

The entries' text is in [`register-entries.md`](register-entries.md).
