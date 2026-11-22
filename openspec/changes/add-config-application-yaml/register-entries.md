# Register entries — `add-config-application-yaml`

Append-ready text for the coordinator, following the entry formats of
`docs/deviations.md` and `docs/tech-debt.md`. The proposer writes nothing
outside this directory, so these are not yet in the registers. Numbers are the
next free ones on `main` at `7f1c8e6` (`DEV-0030` and `DEBT-0021` are the last
used); renumber in place, and in the spec delta, if another branch takes them
first.

The two `DEV` entries carry no `DEBT` entry, because the owner ruled that the
deferred support is owed only when a deployment needs it; each entry's
rationale states that condition. The five `DEBT` entries carry no `DEV`
entry, because each one is HAPI's behaviour, matched (standing ruling: no
`DEV` entry for matching HAPI).

---

## Append to `docs/deviations.md`

```markdown
### DEV-0031 — configuration locations Frevo does not load fail loudly; `on-cloud-platform` never matches

- **Area:** config
- **Scenario IDs:** config.source.017, config.source.020, config.profile.014
- **HAPI 8.12.0 behaviour:** Spring Boot 3.5.14 in the starter loads four
  things Frevo does not. (1) Any `classpath:` resource on the war's class
  path, which in the distroless image includes `/app/extra-classes/`
  (`Dockerfile`, `-Dloader.path=main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes`),
  so `--spring.config.location=classpath:/another.application.yaml` loads an
  operator-mounted `/app/extra-classes/another.application.yaml`.
  (2) `spring.config.import=configtree:<dir>/`, one property per file under
  the directory. (3) `.xml` files in Java `Properties.loadFromXML` format at
  any location. (4) Documents with
  `spring.config.activate.on-cloud-platform: kubernetes` (or another
  platform), activated when Spring Boot detects that platform, for Kubernetes
  from `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT`.
- **Specification requirement:** none; this is configuration loading, which
  the FHIR specification does not address. The reference is HAPI's behaviour
  alone.
- **Chosen behaviour:** Frevo. A `classpath:` location other than the three
  packaged files (`application.yaml`, `application-cds.yaml`,
  `application-elastic.yaml`) fails to start with Spring's `Config data
  resource '…' via location '…' does not exist` report; a `configtree:` or
  `.xml` location fails to start with a report naming the location and the
  supported kinds (`file:`, `classpath:` for the packaged files, `.properties`,
  `.yml`, `.yaml`); `optional:` makes each of them a no-op instead, as it does
  for a missing file. An `on-cloud-platform` document never activates, on any
  platform.
- **Configuration flag:** none.
- **Debt:** none. Owner ruling of 2026-11-22: support is owed when, and only
  when, a deployment needs one of the four; the failure is loud precisely so
  that such a deployment finds out at startup rather than running with
  configuration silently missing.
- **Rationale:** Frevo is a single binary with no class path, so (1) can only
  be imitated by guessing a directory layout that is specific to one image.
  (2) to (4) are rare in starter deployments; the starter's own Helm chart
  uses `SPRING_CONFIG_IMPORT` with a plain file path, which Frevo supports.
  Failing loudly turns every case where the two servers would differ into a
  startup error instead of a silent difference in effective configuration.
  `on-cloud-platform` cannot fail loudly without failing every Kubernetes
  deployment that never uses it, so it is the one silent case, and it is
  listed here for that reason.
- **Normalizer impact:** none. The `config-roundtrip` cases for these four
  record HAPI's result and assert Frevo's documented failure (or, for
  `on-cloud-platform`, its non-activation) instead of equality; no
  normalisation rule is added.
- **Reviewed:** 2026-11-22, owner: chosen as written (relayed by the
  coordinator).

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
```

---

## Append to `docs/tech-debt.md`

```markdown
### DEBT-0022 — a spelling the starter's named reads do not see splits a feature's gate from its bound value

- **Area:** config
- **Origin:** `add-config-application-yaml` (config.read.002, config.read.005,
  config.gate.006, config.gate.007), matching HAPI 8.12.0
- **Owed:** a startup check, behind no flag, that reports when a gate and the
  property it guards disagree — for example `mdm-enabled: true` binding
  `true` while the `mdm` gate reads the packaged `false`; `cr.enabled: yes`
  binding `true` while `Boolean.parseBoolean` turns the `cr` gate off;
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
