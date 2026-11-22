# Register entries — `add-config-application-yaml`

Text for the coordinator, following the entry formats of `docs/deviations.md`
and `docs/tech-debt.md`. The proposer writes nothing outside this directory.

Status on this branch after `920a00c` (which appended the first version of
every entry below) and the review of 2026-11-22:

| Entry | Action |
|---|---|
| `DEV-0031` | **replace** the entry in `docs/deviations.md` with the text below (narrowed: `classpath:` now matches HAPI, finding 8) |
| `DEV-0032` | unchanged; nothing to do |
| `DEV-0033` | **new, proposed — needs the owner's approval before it is appended** (Java stack frames, finding 12) |
| `DEBT-0022` | **replace** the entry in `docs/tech-debt.md` with the text below (the `cr.enabled: yes` example corrected, finding 11) |
| `DEBT-0023` to `DEBT-0026` | unchanged; nothing to do |

The `DEV` entries carry no `DEBT` entry, because the owner ruled that the
deferred support is owed only when a deployment needs it (`DEV-0031`), or
because nothing is owed (`DEV-0032`, `DEV-0033`). The five `DEBT` entries carry
no `DEV` entry, because each one is HAPI's behaviour, matched (standing
ruling: no `DEV` entry for matching HAPI).

---

## `docs/deviations.md`

```markdown
### DEV-0031 — `configtree:` and `.xml` configuration locations fail loudly; `on-cloud-platform` never matches

- **Area:** config
- **Scenario IDs:** config.source.020, config.profile.014
- **HAPI 8.12.0 behaviour:** Spring Boot 3.5.14 in the starter loads three
  things Frevo does not. (1) `spring.config.import=configtree:<dir>/`, one
  property per file under the directory. (2) `.xml` files in Java
  `Properties.loadFromXML` format at any location. (3) Documents with
  `spring.config.activate.on-cloud-platform: kubernetes` (or another
  platform), activated when Spring Boot detects that platform, for Kubernetes
  from `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT`.
  `classpath:` locations are **not** part of this entry: Frevo resolves them
  as the image does, to the packaged files and then the fixed directory
  `/app/extra-classes/` (the last entry of the ENTRYPOINT's `loader.path`),
  for explicitly named and default locations alike (config.source.017, .030).
- **Specification requirement:** none; this is configuration loading, which
  the FHIR specification does not address. The reference is HAPI's behaviour
  alone.
- **Chosen behaviour:** Frevo. A `configtree:` or `.xml` location fails to
  start with a report naming the location and the supported kinds (`file:`,
  `classpath:`, `.properties`, `.yml`, `.yaml`); `optional:` makes it a no-op
  instead, as it does for a missing file. An `on-cloud-platform` document
  never activates, on any platform.
- **Configuration flag:** none.
- **Debt:** none
- **Rationale:** Owner ruling of 2026-11-22: support is owed when, and only
  when, a deployment needs one of the three; the failure is loud precisely so
  that such a deployment finds out at startup rather than running with
  configuration silently missing. (1) and (2) are rare in starter deployments; the starter's
  own Helm chart uses `SPRING_CONFIG_IMPORT` with a plain file path, which
  Frevo supports. Failing loudly turns both into a startup error instead of a
  silent difference in effective configuration. `on-cloud-platform` cannot
  fail loudly without failing every Kubernetes deployment that never uses it,
  so it is the one silent case, and it is listed here for that reason.
- **Normalizer impact:** none. The `config-roundtrip` cases for these three
  record HAPI's result and assert Frevo's documented failure (or, for
  `on-cloud-platform`, its non-activation) instead of equality; no
  normalisation rule is added.
- **Reviewed:** 2026-11-22, owner: chosen as written (relayed by the
  coordinator); narrowed the same day after review finding 8, by the
  coordinator's direction that `classpath:` match HAPI.

### DEV-0032 — two non-kebab spellings of one property in one source fail to start

- **Area:** config
- **Scenario IDs:** config.relaxed.007
- **HAPI 8.12.0 behaviour:** when one configuration source spells the same
  property two ways, neither of them the canonical kebab-case form — for
  example one YAML mapping holding both `defaultPageSize: 4` and
  `default_page_size: 3` — HAPI starts and binds one of them, chosen by the
  iteration order of a `java.util.HashSet` of the source's names inside
  Spring Boot's `SpringIterableConfigurationPropertySource`. Measured on
  Spring Boot 3.5.14: it took `default_page_size` (3) over `defaultPageSize`
  (4), and `max_page_size` (5) over `maxPageSize` (6), whatever their order in
  the file. When one of the two spellings is the kebab-case form, that one
  wins deterministically (`config.relaxed.006`), and Frevo matches that.
- **Specification requirement:** none; configuration binding is outside the
  FHIR specification.
- **Chosen behaviour:** Frevo. Frevo fails to start with a report naming both
  spellings and their origins (file and `line:column`).
- **Configuration flag:** none.
- **Debt:** none.
- **Rationale:** the configuration is ambiguous, and HAPI's answer is an
  accident of `String.hashCode` and hash-table capacity, not a rule anyone
  could rely on or document. Reproducing it would mean porting Java's
  `HashSet` iteration order into the binder. Refusing to start costs a
  deployment one edit, made once, with the report saying exactly which two
  lines collide.
- **Normalizer impact:** none. The `config-roundtrip` case records HAPI's
  pick and asserts Frevo's failure instead of equality.
- **Reviewed:** 2026-11-22, owner: chosen as written (relayed by the
  coordinator).

### DEV-0033 — startup-failure output omits Java stack frames

- **Area:** config
- **Scenario IDs:** config.error.009, config.oracle.003
- **HAPI 8.12.0 behaviour:** when startup fails with the `Application run
  failed` form (config.error.007), HAPI's log line is followed by the
  exception chain with every Java stack frame (`\tat <class>.<method>(<file>:<line>)`
  lines and `\t... N more`), and the `WARN` line of config.error.008 carries a
  frame list too. Example: with no `hapi.fhir.fhir_version`, the
  `NullPointerException` is followed by frames from
  `ca.uhn.fhir.jpa.starter.annotations.On*Condition.matches` down to
  `ca.uhn.fhir.jpa.starter.Application.main`.
- **Specification requirement:** none.
- **Chosen behaviour:** Frevo. Every other line is reproduced verbatim —
  stream, log header lines, `Application run failed`, each exception class
  and message, each `Caused by:` — and the stack-frame lines are omitted.
- **Configuration flag:** none.
- **Debt:** none.
- **Rationale:** the frames describe the JVM call stack of HAPI's own code;
  a Rust process has no such stack, and fabricating one from a table would be
  output no operator can act on and that changes with every HAPI build. The
  lines an operator reads and alerts on (the exception classes and messages)
  are kept identical.
- **Normalizer impact:** the `config-roundtrip` failure comparison removes
  lines matching `^\tat ` and `^\t\.\.\. [0-9]+ more$` from HAPI's output
  before comparing, and masks each log line's timestamp and process id.
- **Reviewed:** proposed 2026-11-22 by the proposer after review finding 12;
  **needs the owner's approval**.
```

---

## `docs/tech-debt.md`

```markdown
### DEBT-0022 — a spelling the starter's named reads do not see splits a feature's gate from its bound value

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.read.002, config.read.005,
  config.gate.006, config.gate.007), matching HAPI 8.12.0
- **Owed:** a startup check, behind no flag, that reports when a gate and the
  property it guards disagree — for example `mdm-enabled: true` binding
  `true` while the `mdm` gate reads the packaged `false`; the quoted
  `cr.enabled: "yes"` (or `HAPI_FHIR_CR_ENABLED=yes`) binding `true` while the
  read is the text `yes` and `Boolean.parseBoolean` turns the `cr` gate off
  (the unquoted YAML `yes` is a boolean, reads `"true"` and turns the gate
  on, config.gate.023);
  `mdm_enabled: "no"` binding `false` while the `mdm` gate is on;
  `hapi.fhir.fhir-version=R5` binding R5 while the version gates read R4.
- **Why deferred:** HAPI starts in every one of these cases, with the feature
  wired by the gate and configured by the bound value; constitution section 1
  requires Frevo to match.
- **Cost if unpaid:** an operator who writes a relaxed spelling the starter's
  `Environment.getProperty` calls do not see gets a feature half on: its
  beans present or absent by one reading, its settings taken from the other.
  The symptom appears far from the cause.
- **Trigger:** an operator report of a feature on or off against its
  configuration, or a consumer change that cannot define behaviour for the
  split state.
- **Size:** small — the gates and the bound values are both in the effective
  configuration; the check compares them.
- **Status:** open
- **Reviewed:** 2026-11-22, owner (standing ruling: reproduce HAPI's hazards
  exactly)

### DEBT-0023 — an empty `hapi.fhir.fhir_version` selects the DSTU2 configuration

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.gate.003), matching HAPI
  8.12.0 (`FhirVersionEnum.forVersionString`, `hapi-fhir-base`
  `FhirVersionEnum.java:283-315`)
- **Owed:** fail to start when the version read is empty, instead of turning
  on `fhir-dstu2` while `hapi.fhir.fhir-version` stays bound to `R4`.
- **Why deferred:** HAPI does exactly this: the empty string is a prefix of
  every version string, so the first constant, DSTU2, matches; constitution
  section 1.
- **Cost if unpaid:** `fhir_version:` left empty in an override file serves a
  DSTU2-wired server whose bound version says R4.
- **Trigger:** any report of it, or the first change that wires DSTU2
  (`add-dstu3-dstu2`), whichever comes first.
- **Size:** trivial.
- **Status:** open
- **Reviewed:** 2026-11-22, owner (standing ruling)

### DEBT-0024 — environment variables and other properties expand inside the request-log templates

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.placeholder.009),
  matching HAPI 8.12.0
- **Owed:** keep `${requestVerb}`-style tokens of `hapi.fhir.logger.format`
  and `hapi.fhir.logger.error_format` out of configuration placeholder
  resolution, so that they reach the request logger intact.
- **Why deferred:** Spring Boot resolves `${...}` in every bound value; with
  `requestVerb=GET` in the environment, HAPI's
  `"ERROR - ${requestVerb} ${requestUrl}"` binds as
  `"ERROR - GET ${requestUrl}"`. Constitution section 1.
- **Cost if unpaid:** an unrelated environment variable or property whose name
  matches a template token (`requestVerb`, `servletPath`,
  `operationType`, …) silently replaces that field of every access-log line
  with a constant.
- **Trigger:** an operator report, or `add-server-skeleton` wiring the request
  logger and needing the tokens intact.
- **Size:** small.
- **Status:** open
- **Reviewed:** 2026-11-22, owner (standing ruling)

### DEBT-0025 — `hapi.fhir.cr.cql.compiler.error_level` is accepted and ignored

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.relaxed.004), matching
  HAPI 8.12.0 (`CqlCompilerProperties` names the property
  `errorSeverityLevel`; its field is `error_level`)
- **Owed:** report, at startup, a key that matches a starter field name but no
  bean property, starting with this one, which the packaged
  `application.yaml` itself sets (`error_level: Info`).
- **Why deferred:** HAPI binds with `ignoreUnknownFields=true`, so
  `error_level: Error` starts and stays `Info`; the bindable key is
  `error_severity_level`. Constitution section 1.
- **Cost if unpaid:** an operator who copies the packaged file's spelling and
  changes the value gets no effect and no message.
- **Trigger:** an operator report, or `add-cql-translator` needing the
  severity setting to work as documented.
- **Size:** trivial.
- **Status:** open
- **Reviewed:** 2026-11-22, owner (standing ruling)

### DEBT-0026 — `hapi.fhir.cr.cql.runtime.enable_validation` cannot be enabled

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.unknown.005), matching
  HAPI 8.12.0 (`CqlRuntimeProperties` declares `EnableValidation(boolean)`,
  which is not a JavaBean setter)
- **Owed:** a working way to enable CQL runtime validation, once a consumer
  needs it.
- **Why deferred:** in HAPI, `enable_validation: true` fails to start with
  `No setter found for property: enable-validation`, and `false` (its
  current value) is accepted. Constitution section 1.
- **Cost if unpaid:** CQL runtime validation stays off in every deployment,
  and an operator who tries to turn it on gets a startup failure that names a
  setter, not the cause.
- **Trigger:** `add-cql-engine` or a later change needing runtime validation
  on, or an operator report.
- **Size:** trivial in configuration; the feature itself belongs to
  `add-cql-engine`.
- **Status:** open
- **Reviewed:** 2026-11-22, owner (standing ruling)
```
