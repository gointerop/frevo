# Register entries — `add-config-application-yaml`

Text for the coordinator, following the entry formats of `docs/deviations.md`
and `docs/tech-debt.md`. The proposer writes nothing outside this directory.

Status on this branch after `920a00c` (which appended the first version of
every entry below) and the review of 2026-11-22:

| Entry | Action |
|---|---|
| `DEV-0031` | **replace** the entry in `docs/deviations.md` with the text below (narrowed: `classpath:` now matches HAPI, finding 8) |
| `DEV-0032` | unchanged; nothing to do |
| `DEV-0033` | registered by the coordinator in `f81cfd5`, owner approved 2026-11-22, and already carrying the scope extended the same day to every non-config log line (second review, finding 1); **replace** it again with the text below, which extends it to logback's own status lines (owner ruling of 2026-11-22 on oracle finding O-1) and restates it against the record of HAPI's real `Application` (fourth review, findings 1 to 3). The changed fields are the title, **Scenario IDs**, **HAPI 8.12.0 behaviour** (tab-led frames, library and shutdown lines, item 3 added with the measured counts), **Chosen behaviour** (Jackson's line, the `WARN` exception text, the kinds with `INFO`, last sentence), **Normalizer impact** (tab-led frames, logger field compared, `Bindable` mask, status lines) and **Reviewed**. Still owed in `docs/deviations.md` (review finding 3): `docs/deviations.md:1385` has the pre-extension title and scope |
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

### DEV-0033 — startup-failure output keeps only the config-related lines: no Java stack frames, no other log lines, no logback status lines

- **Area:** config
- **Scenario IDs:** config.dump.006, config.error.007, config.error.008,
  config.error.009, config.oracle.003
- **HAPI 8.12.0 behaviour:** when startup fails, HAPI's standard output holds,
  besides the failure itself, (1) Java stack frames after the exception
  chain of the `Application run failed` form, each led by a tab
  (`\tat <class>.<method>(<file>:<line>)`, `\t... N more` and `\t... N
  common frames omitted`; with no `hapi.fhir.fhir_version`, for example, 35
  frame lines from `ca.uhn.fhir.jpa.starter.annotations.On*Condition.matches`
  down to `ca.uhn.fhir.jpa.starter.Application.main`), and (2) log lines that
  are not about configuration: the startup-info lines `Starting Application
  using Java … with PID …` and `No active profile set, falling back to 1
  default profile: "default"` (or the active-profiles variant), embedded
  Tomcat's startup lines, the servlet context's initialisation lines,
  whatever HAPI components, beans and libraries (Hibernate, Hikari, Quartz)
  log before the failing bean is created, and their shutdown lines after the
  `WARN … cancelling refresh attempt` line, in an order set by HAPI's bean
  graph. (3) logback prints its own status lines (`HH:MM:SS,mmm |-<LEVEL> in
  …`, among them `|-WARN in …ConfigurationModelHandlerFull - Missing
  watchable .xml or .properties files.` and `Watching .xml files requires
  …`), because the packaged `logback.xml` sets `scan="true"` while it is read
  from inside a jar: on standard output on every run, failing or not (57
  lines in each of the oracle's 419 runs), and on standard error on every run
  except those that fail before the application context exists (2 lines in
  396 runs, none in the other 23).
- **Specification requirement:** none.
- **Chosen behaviour:** Frevo. It reproduces verbatim the failure-analysis
  block or the `Application run failed` exception chain (each exception class
  and message, SnakeYAML's position and caret lines, Jackson's space-led
  ` at [Source: …]` location line, each `Caused by:`), the `ERROR` line that
  introduces it, and the config-related `WARN` (`Exception encountered during
  context initialization - cancelling refresh attempt: <exception>`, with
  HAPI's `ConfigServletWebServerApplicationContext` logger name and HAPI's
  exception text, including the `BeanDefinitionStoreException` that names a
  class file under `jar:file:/app/main.war!/WEB-INF/classes!/`) and `INFO`
  (`.s.b.a.l.ConditionEvaluationReportLogger`, `Error starting
  ApplicationContext …`) lines, for exactly the failure kinds in which HAPI's
  `Application` prints them (config.error.008), all on standard output in the
  starter's logback console format. It writes no stack-frame line and none of
  the other log lines: no startup-info line, no Tomcat line, no
  servlet-context, component, library or shutdown line, and no logback status
  line on either stream, whether startup fails or succeeds.
- **Configuration flag:** none.
- **Debt:** none
- **Rationale:** the frames describe the JVM call stack of HAPI's own code
  and the other lines describe a Java servlet container, a Spring bean graph
  and a logback configuration scanner that a Rust process does not have;
  reproducing them would be
  fabricated output that no operator can act on and that changes with every
  HAPI build. The lines an operator reads and alerts on — the failure itself
  and its config-related context — are kept identical.
- **Normalizer impact:** the `config-roundtrip` failure comparison
  (config.oracle.003) compares only the config-related lines of the record,
  which for every failing case is the image's own `Application`
  (config.oracle.001): the `ERROR` line and every line after it with
  stack-frame lines (`^\t+at `, `^\t+\.\.\. [0-9]+ more$`,
  `^\t+\.\.\. [0-9]+ common frames omitted$`; tab-led only, so Jackson's
  space-led location line is kept) removed, and the `WARN` and `INFO` lines
  above when HAPI has them, logger field included. In each compared log line
  the timestamp and process id are masked, and in every compared line the
  number of `Bindable@<hex>`. Every other line of
  HAPI's output is stripped before comparing, and Frevo's output must contain
  no line outside the compared set. Logback status lines, matching
  `^\d\d:\d\d:\d\d,\d\d\d \|-(INFO|WARN|ERROR) in `, are removed from
  both of HAPI's streams, for every case, before anything is compared, and
  counted in the record (`logback_status_lines`).
- **Reviewed:** owner approved 2026-11-22; scope extended same day; extended
  again the same day to logback's status lines (oracle finding O-1); text
  restated the same day against the record of HAPI's real `Application`
  (fourth review, findings 1 to 3), with no change of scope.
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
