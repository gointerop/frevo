# add-config-application-yaml

## Why

The constitution's first promise to an operator is that an existing HAPI FHIR
JPA Server Starter deployment can switch to Frevo **without changing its
configuration** (`openspec/config.yaml`, "Configuration compatibility is a hard
requirement"). That promise is not about key names alone. The starter's
configuration is whatever Spring Boot 3.5.14 makes of `application.yaml`,
profile files, imports, environment variables and arguments, bound into nine
`@ConfigurationProperties` classes and read a second time, by exact name, by
roughly twenty bean conditions. An operator who wrote
`HAPI_FHIR_CORS_ALLOW_CREDENTIALS=true`, `mdm-enabled: true`,
`allowed_bundle_types: COLLECTION,DOCUMENT`, `profile_mode: OFF` or
`SPRING_CONFIG_IMPORT=/app/config/application-extra.yaml` (the starter's own
Helm chart) gets a specific, measurable result from HAPI, and has to get the
same one from Frevo.

`frevo-config` is also the first thing every later change reads. Every
`hapi.fhir.*` scenario in phases 2 to 9 starts with "with key K set to V"; if
the binding of V is wrong, every one of those scenarios is graded against the
wrong input. So this change specifies the loading and binding pipeline in full,
and leaves what each key *does* to the change that implements the behaviour
(`keys.md`, column "Wired to behaviour by").

## What changes

`crates/frevo-config` gains:

1. **A configuration loader** with Spring Boot 3.5.14's sources and order:
   the starter's packaged `application.yaml`, `application-cds.yaml` and
   `application-elastic.yaml` as the `classpath:` location; `./`, `./config/`
   and `./config/*/`; `spring.config.location`, `additional-location`,
   `import` and `name`; `optional:`; profiles, profile files, multi-document
   files, `on-profile` expressions, `include`, `group` and `default`;
   `.properties`, `.yml` and `.yaml`; environment variables with both of
   Spring's name mappers; `--name=value` arguments; `SPRING_APPLICATION_JSON`;
   Java system properties — the JVM's own (`user.dir`, `java.io.tmpdir`,
   `PID`, the image's `loader.path`, …) and the `-D` tokens of
   `JAVA_TOOL_OPTIONS` and `JDK_JAVA_OPTIONS` with the JVM's quoting;
   `spring.application.json` from any source; the image's
   `/app/extra-classes/` as part of `classpath:`;
   `${...}` placeholders, lenient for binding and strict for named reads,
   including `random.*` (config.source, .yaml, .properties, .profile, .env,
   .args, .placeholder).
2. **A binder** for the starter's nine properties classes with Spring's
   relaxed names (per element, case- and separator-insensitive, against bean
   property names, not field names), its conversion rules (YAML 1.1 typing,
   lenient enums, comma lists, hex and whitespace in numbers, empty keeps the
   default), its object, list, set and map semantics across sources, and its
   failures (setter-less properties, index gaps, unconvertible values)
   (config.relaxed, .coerce, .collection, .object, .unknown, .key).
3. **The named reads and the gates.** The starter decides whether to load the
   FHIR servlet, clinical reasoning, CDS Hooks, MDM, IPS, binary storage,
   CORS, partitioning, the tester and others from `Environment.getProperty`
   and from its `Condition` classes, not from the bound properties. Those
   reads are exact-name for names containing `_`, so they can disagree with
   the binding: `mdm-enabled: true` binds `true` while the MDM gate reads the
   packaged `false`; `cr.enabled: "yes"` (quoted, so the read is the text
   `yes`) binds `true` while `Boolean.parseBoolean` turns the CR gate off; `fhir_version: ""` keeps
   `R4` bound and turns on the **DSTU2** gate. This change computes every gate
   with HAPI's own predicate (config.read, .gate), so later changes wire
   features from the answer HAPI reaches.
4. **Failure output** as HAPI writes it: on standard output through the
   starter's logback console format, either Spring Boot's failure analysis
   (`APPLICATION FAILED TO START`, verbatim including Java type names) or
   `Application run failed` with the exception chain, by failure kind; exit
   status `1`, before anything else starts (config.error). Java stack frames
   and the non-config log lines (startup info, Tomcat, servlet context, HAPI
   components) are not reproduced (`DEV-0033`, approved by the owner on
   2026-11-22, scope extended the same day).
5. **`frevo-config effective`**, a command that prints the effective
   configuration as JSON (`profiles`, `properties` per bean prefix, `reads`,
   `gates`). It is the observable surface for every scenario here and the
   Frevo side of the `config-roundtrip` oracle (config.dump).

Scenario count: **232** across 19 areas (see [`spec.md`](spec.md)).

### How the behaviour was established

Not from memory of Spring Boot and not from the starter's comments. A probe —
the starter's nine properties classes compiled from the pinned commit, booted
by Spring Boot 3.5.14 with the HAPI 8.12.0, clinical-reasoning 4.13.0 and CQL
5.4.0 jars, dumping every getter — was run against about 150 configurations,
and every value, message and line:column in a THEN clause is what it printed
([`design.md`](design.md) section 2). The `config-roundtrip` oracle re-runs the
same cases against the jars inside the pinned image on Linux; it, not this
text, is the standard. Its first full run (419 cases) produced thirteen
findings against the first draft; the owner ruled on 2026-11-22 that the
recorded HAPI behaviour wins, and the delta now quotes the record
(design.md section 2, "Oracle findings of 2026-11-22").

The probe found that the configuration surface is not what the inventory says:

- **`hapi.fhir.cr.cql.compiler.error_level` binds nothing.** The property is
  `errorSeverityLevel`; the packaged `error_level: Info` is a no-op, and
  `error_level: Error` silently stays `Info` (config.relaxed.004).
- **`hapi.fhir.cr.cql.runtime.enable_validation: true` fails to start**:
  `CqlRuntimeProperties` has `EnableValidation(boolean)`, which is not a setter
  (config.unknown.005).
- **`hapi.fhir.search-coord-*`, the packaged file's three "Search
  Orchestration" keys, are read by nothing** in the starter or in HAPI
  8.12.0 (config.unknown.002).
- **Seven bound keys are read by nothing** (`cr_enabled`,
  `install_additional_resources_from_ig_folders`, three `subscription.email.*`,
  `cr.cql.compiler.translator_format`, `cr.cql.use_embedded_libraries`)
  (config.key.010).
- **Five `terminologyServerClientSettings` keys, six `implementationguides`
  fields and `normalized_quantity_search_level` bind but are not in the
  inventory**, and three inventory rows are not keys at all (`keys.md`
  sections 6 and 7).

## Upstream module and pinned SHA for each piece

Pins are from [`reference/PINS.md`](../../../reference/PINS.md).

| Piece | Upstream repo | Upstream path | Pinned SHA | Relationship |
|---|---|---|---|---|
| Key names, types, defaults and quirks of the nine properties classes | `hapifhir/hapi-fhir-jpaserver-starter` | `src/main/java/ca/uhn/fhir/jpa/starter/AppProperties.java`, `cdshooks/CdsHooksProperties.java`, `cr/CrProperties.java`, `cr/CareGapsProperties.java`, `cr/CqlProperties.java`, `cr/CqlCompilerProperties.java`, `cr/CqlRuntimeProperties.java`, `cr/CqlTerminologyProperties.java`, `cr/CqlData.java` | `377e6eba316920082e6a90551b488e4599a71eb8` | **derived** — descriptor tables, provenance header required |
| Packaged configuration | same | `src/main/resources/application.yaml`, `application-cds.yaml`, `application-elastic.yaml`, after the `pom.xml:463-468,513-522` resource filtering | same | **vendored data**, byte-identical to the pinned image's `WEB-INF/classes/` (config.source.001) |
| Named reads and gates | same | `annotations/On{DSTU2,DSTU3,R4,R4B,R5}Condition.java`, `OnEitherVersion.java`, `OnCorsPresent.java`, `OnImplementationGuidesPresent.java`, `common/OnPartitionModeEnabled.java`, `common/FhirTesterConfigCondition.java`, `common/validation/OnRemoteTerminologyPresent.java`, `cr/CrConfigCondition.java`, `cdshooks/CdsHooksConfigCondition.java`, `elastic/ElasticConfigCondition.java`, `util/EnvironmentHelper.java`, and the `@ConditionalOnProperty` sites in `FhirServerConfigCommon.java:371,393`, `StarterJpaConfig.java:109,247`, `ig/ImplementationGuideR{4,5}OperationProvider.java:17`, `ips/StarterIpsConfig.java:14`, `mcp/McpServerConfig.java:30,53`, `mdm/MdmConfig.java:23`, `web/CustomContentFilesConfigurer.java:14`, `web/WebAppFilesConfigurer.java:17` | same | **derived** |
| Version gate predicate | `hapifhir/hapi-fhir` | `hapi-fhir-base/src/main/java/ca/uhn/fhir/context/FhirVersionEnum.java:283-315` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | **derived** |
| Implementation-guide entry fields | same | `hapi-fhir-storage/src/main/java/ca/uhn/fhir/jpa/packages/PackageInstallationSpec.java` | same | **derived** (descriptor) |
| Enum domains for lenient conversion | same; `hapifhir/org.hl7.fhir.core`; `cqframework/clinical-reasoning`; `cqframework/clinical_quality_language` | `JpaStorageSettings` (`ClientIdStrategyEnum`, `IdStrategyEnum`, `StoreMetaSourceInformationEnum`), `NormalizedQuantitySearchLevel`, `EncodingEnum`, `FhirVersionEnum`; `org.hl7.fhir.r4.model.Bundle.BundleType`; `RetrieveSettings`, `TerminologySettings`, `TerminologyServerClientSettings` (cqf-fhir-cql, cqf-fhir-utility); `CqlCompilerException.ErrorSeverity`, `LibraryBuilder.SignatureLevel`, `CqlTranslator.Format` | `7ab4c7c7…`, `73c432b5…`, `ccba474a…`, `c41c21f5…` | **derived** (constant lists) |
| Loading, binding, conversion and YAML resolution rules | `spring-projects/spring-boot` 3.5.14, `spring-projects/spring-framework` 6.2.19 (what the pinned image ships; owner ruling of 2026-11-22), `snakeyaml` 2.4 | `ConfigDataEnvironment`, `StandardConfigDataLocationResolver`, `OriginTrackedYamlLoader`, `OriginTrackedPropertiesLoader`, `Binder`, `JavaBeanBinder`, `MapBinder`, `IndexedElementsBinder`, `ConfigurationPropertyName`, `SystemEnvironmentPropertyMapper`, `DefaultPropertyMapper`, `LenientStringToEnumConverterFactory`, `BindFailureAnalyzer`; Spring's `YamlProcessor`, `SystemEnvironmentPropertySource`, `PropertyPlaceholderHelper`, `NumberUtils`, `StringToBooleanConverter`; SnakeYAML's `Resolver` | `reference/PINS.md` pins `v3.5.14`, `snakeyaml-2.4` and, for Spring Framework, `v6.2.18` (#38); the re-pin to `v6.2.19` is owed on a `chore/*` branch (task 5) | **derived behaviour** — measured by the probe, judged by the oracle; no source is ported |

## Oracles

| Scenario group | Oracle | Case selector |
|---|---|---|
| `config.source.*`, `.yaml.*`, `.properties.*`, `.profile.*`, `.env.*`, `.args.*`, `.placeholder.*`, `.relaxed.*`, `.coerce.*`, `.collection.*`, `.object.*`, `.unknown.*` (except `.007`), `.read.*`, `.gate.*`, `.key.*` | **`config-roundtrip`** — new, `harness/config-roundtrip/` (design.md section 7): each case run by Spring Boot 3.5.14 with the starter's classes and conditions, using the jars inside `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`, on Linux; documents compared member for member | every case under `harness/config-roundtrip/cases/` tagged `covers: <id>`; `config.oracle.005` requires one per scenario with a configuration |
| `config.key.005`, `.006`, `.007` | `config-roundtrip` | the three whole-surface cases `all-keys.yaml`, `all-keys.env`, `all-keys-kebab.yaml` |
| `config.key.002` (CORS defaults) | `config-roundtrip`, plus HAPI's JUnit `AppPropertiesCorsDefaultsTest` (starter `src/test/java/ca/uhn/fhir/jpa/starter/`) for the defaults the class declares | `defaultCorsHeadersIncludeFhirOptimisticLockingHeaders`, `nullCorsListsFallBackToDefaults` |
| `config.error.*` | `config-roundtrip` failure comparison (`config.oracle.003`); the config-related lines compared verbatim, the rest stripped (owner rulings of 2026-11-22, `DEV-0033`) | cases whose HAPI run fails |
| `config.source.001` | **self-checking** against the pinned image: the three packaged files extracted from `/app/main.war!/WEB-INF/classes/` | byte comparison |
| `config.dump.*`, `config.unknown.007` | **Frevo-only surface**, tested directly; the document's shape is what the oracle's HAPI side emits | — |
| `config.oracle.*` | the harness's own self-test | — |
| `config.perf.*` | `bench/micro/` workloads `config-load-packaged`, `config-get` (new) | — |

No vendored HL7 suite covers configuration; the gate is the differential one.
There is no HL7 suite pass rate to meet for this change.

## Prerequisites this change does not create

1. **`config-roundtrip` is a new oracle.** It does not exist in
   `oracles/PINS.md` and needs no new pin: its HAPI side is the already-pinned
   image. It lives in `harness/` (oracle-builder), not `oracles/`.
2. **The Spring sources in `reference/`** (task 5). Pinning
   `spring-projects/spring-boot@v3.5.14`, `spring-framework` and
   `snakeyaml@snakeyaml-2.4` is a `chore/*` branch and an owner decision,
   because it adds to what "correct" refers to. Ruled 2026-11-22: it happens
   on a `chore/*` branch, not in this change, and does not block it. #38
   pinned Spring Framework `v6.2.18`; the owner ruled on 2026-11-22 that the
   reference is **6.2.19**, the version the pinned image ships, so
   `reference/PINS.md` and `reference/clone.sh` are re-pinned to `v6.2.19` on
   a `chore/*` branch.
3. **`docs/inventory.md` needs correcting** (`keys.md` section 7) on a
   `chore/*` branch: `harness/scripts/gen-inventory.py` is not this branch's to
   edit.
4. **ADR 0007** (the YAML parser dependency) was accepted by the owner on
   2026-11-22 (option A). It stays at
   [`adr-0007-yaml-event-parser.md`](adr-0007-yaml-event-parser.md) until the
   owner moves it to `docs/adr/`.

## Out of scope

| Out of scope | Why | Covered by |
|---|---|---|
| What each `hapi.fhir.*` key does | this change binds, validates, reports and gates; behaviour belongs to the feature | the change named per key in `keys.md` section 5 |
| Typed binding of `spring.datasource.*`, `spring.jpa.*`, `server.*` (port, context path, `tomcat.relaxed-query-chars`), `management.*`, `spring.flyway.*`, `spring.main.*`, `spring.ai.mcp.server.*` other than `enabled`, `logging.*` | their meaning is the consumer's; this change loads them through the same pipeline and makes them available to reads (config.unknown.004); ruled 2026-11-22 | `add-server-skeleton` (`server.*`, `spring.main.*`), `add-store-schema` (`spring.datasource.*` and the JDBC URL form), `add-observability` (`management.*`, `logging.*`), `add-mcp` |
| Loading `custom_interceptor_classes`, `custom_provider_classes`, `custom-bean-packages` | they name Java classes and packages; this change binds the lists and the read. HAPI fails to start on a class not on its class path (`StarterJpaConfig.java:572-577`), which is what Frevo's empty class path will give | `add-server-skeleton` |
| `/actuator/configprops`, `/actuator/env` | not exposed by the packaged `management.*` settings; the document of config.dump is the diagnostic | `add-observability` |
| Loading `configtree:` imports and `.xml` properties files; activating `spring.config.activate.on-cloud-platform` documents; `classpath:` resources inside `WEB-INF/lib` jars | HAPI supports them; Frevo fails loudly on the first two and never activates the third (config.source.020, config.profile.014), recorded as `DEV-0031`; the oracle asserts that no jar in the pinned image carries an application file, so the fourth cannot differ | none until a deployment needs one (`DEV-0031`'s revisit condition) |
| Typed binding of Spring Boot's own auto-configuration properties (`server.port`, `spring.datasource.*`, …) | HAPI's auto-configuration binds them at startup, so an invalid `server.port` fails HAPI; here they are loaded and exposed only (config.unknown.004), except `spring.elasticsearch.*`, which a starter condition binds (config.gate.024) | each consuming change, with this change's binder |
| The Tomcat variant of the image (`catalina.properties`, `server.xml`, JNDI, servlet init parameters) | a deployment form, not a configuration source of the default (distroless) image the harness pins | none |
| Re-reading configuration while running | Spring Boot does not reload `@ConfigurationProperties` in the starter either | none |

## Target crate

`frevo-config` only, the crate `tasks/graph.yaml` names for this change. The
`frevo-config effective` binary lives in it. `harness/config-roundtrip/` is
test infrastructure. No PostgreSQL schema change.

## Risks

1. **The probe's jars were not the image's.** Spring Boot's binder and
   SnakeYAML's resolver define most of the behaviour. The probe used jars from
   a local Maven cache whose versions follow the starter's `pom.xml`
   (`spring_boot_version` 3.5.14, `pom.xml:22`; the parent's 3.5.15 is
   overridden), with the Spring Framework 6.2.18 that Boot 3.5.14's BOM
   declares. Task 1 read the versions out of the image: `spring-boot` 3.5.14
   and `snakeyaml` 2.4 as expected, but `spring-core`, `-beans`, `-context`,
   `-aop` and `-expression` **6.2.19**, `slf4j-api` 2.0.16 (not 2.0.17) and
   no `spring-jcl` jar. The owner ruled on 2026-11-22 that **Spring
   Framework 6.2.19 is the reference**, because it is what HAPI 8.12.0's image
   runs; Spring Boot 3.5.14 and SnakeYAML 2.4 are unchanged. The residual
   risk is a 6.2.18-to-6.2.19 difference the cache probe could not show: the
   oracle's record, taken against 6.2.19, is the standard, so any such
   difference already shows up as a recorded result, and `reference/` is
   re-pinned to `v6.2.19` (task 5).
2. **The probe ran on Windows.** Environment-variable names are
   case-insensitive there. The first draft expected the lower-case
   `hapi_fhir_default_page_size` not to bind on Linux; the oracle's Linux run
   shows that it does, because Spring Boot 3.5.14 looks up lower-case
   environment forms too, and only mixed case does not bind (config.env.004,
   finding O-2). Linux semantics, as ruled, are therefore HAPI's recorded
   ones.
3. **Faithful parity reproduces hazards.** The split between named reads and
   binding (config.read.005, config.gate.003, .006, .007), request-logger
   templates expanded from the environment (config.placeholder.009), and a
   silently ignored `error_level` are HAPI's behaviour and the constitution
   says to match it. Each is a `DEBT` entry, not a flag (`DEBT-0022` to
   `DEBT-0026`, [`register-entries.md`](register-entries.md)).
4. **This is a small Spring Boot.** The binder's rules are many and
   individually odd (an empty value keeps an `Integer`'s default but empties a
   `List`; a scalar where an object is expected is ignored only if some other
   source has keys beneath it). The descriptor tables and the probe's case
   list are what keep it honest; the whole-surface round-trips
   (config.key.005-.007) are what catch a missed key.
5. **The parity denominator moves** (`keys.md` section 7); the inventory is
   corrected on a `chore/*` branch before this change archives (task 6).

## Decisions

Every question the first draft left open was decided on 2026-11-22, either
explicitly by the owner or by the standing rulings (HAPI's behaviour wins; no
speculative flag; no `DEV` entry for matching HAPI). No open question remains.
The numbers are those of the first draft, kept so that review history stays
readable.

| # | Question | Decision | Source | Scenarios | Register |
|---|---|---|---|---|---|
| 1 | Observable surface | `frevo-config effective` prints the effective configuration as JSON; `hapi.fhir.subscription.email.password` is redacted | owner | config.dump.001-.006 | none (a diagnostic command, not a configuration key) |
| 2 | Non-packaged `classpath:` locations | first ruled "fail loudly"; revised by the coordinator's direction after review finding 8 to match HAPI: `classpath:` is the packaged files, then the image's fixed `/app/extra-classes/` directory, no setting; a resource in neither fails with the `does not exist` report | owner, then coordinator direction 2026-11-22 | config.source.017, .030 | none (HAPI-matching) |
| 3 | Hazards that parity reproduces | reproduce them exactly | standing ruling | config.read.005, config.gate.003, .006, .007, config.placeholder.009, config.relaxed.004, config.unknown.005 | `DEBT-0022` to `DEBT-0026` |
| 4 | Keys this build does not act on yet | silent, exactly as HAPI: bound and reported, no warning | standing ruling | config.unknown.007 | none |
| 5 | Pinning the Spring sources | on a `chore/*` branch, as a prerequisite task, not in this change | standing ruling | — | none |
| 6 | Java system properties | honour `-D` tokens of `JAVA_TOOL_OPTIONS` and `JDK_JAVA_OPTIONS` with the JVM's quoting, and provide the JVM's own system properties with the image's values | standing ruling, coordinator direction | config.source.019, .023-.025, config.placeholder.012, .013 | none |
| 7 | `configtree:`, `.xml`, `on-cloud-platform` | fail loudly on `configtree:` and `.xml`; `on-cloud-platform` documents never activate | owner | config.source.020, config.profile.014 | `DEV-0031` (the same entry as 2) |
| 8 | Environment-variable case | Linux semantics as the oracle records them: all-lower-case names bind, mixed case does not (finding O-2) | standing ruling; owner ruling 2026-11-22 on O-2 | config.env.004 | none |
| 9 | Failure report text | HAPI's failure output verbatim for the analysis block or exception chain (Java type names included) and the config-related `WARN`/`INFO` lines, on standard output; no Java stack frames, no logback status line and no other log line (startup info, Tomcat, servlet context, components) | standing ruling; owner ruling 2026-11-22 (`DEV-0033` approved, scope extended the same day, and to logback's status lines on O-1) | config.error.002, .007-.010, config.dump.006, config.oracle.003 | `DEV-0033` |
| 10 | Two non-kebab spellings in one source | fail to start, naming both spellings and their origins | owner | config.relaxed.007 | `DEV-0032` |
| 11 | Inventory denominator | corrected on a `chore/*` branch before archiving, as a prerequisite task | standing ruling | config.key.011 | none |
| 12 | Non-`hapi.fhir` keys | loaded through the same pipeline and exposed to reads; typed binding left to the consuming changes | standing ruling | config.unknown.004 | none |
| 13 | YAML parser | ADR 0007 option A: `yaml-rust2` event parser plus Frevo's own SnakeYAML 1.1 resolver; the unused `serde_yaml_ng` workspace entry is dropped | owner (ADR accepted 2026-11-22) | config.yaml.* | ADR 0007 |
| 14 | README's `-e "--spring.config.location=…"` | match HAPI as the oracle records it: the variable is applied as the location (finding O-3; the first draft's "ignored" was wrong) | standing ruling; owner ruling 2026-11-22 on O-3 | config.env.014 | none |

### Review of 2026-11-22 (13 findings) and the coordinator's direction

| Finding | Resolution | Scenarios |
|---|---|---|
| 1 `scheduling_disabled` | added as the thirteenth read and a case-sensitive gate; wired by `add-jobs-engine` and `add-subscriptions-resthook` | config.read.001, .008, config.gate.001, .021 |
| 2 JVM system properties | the JVM's standard properties, `PID` and the image's `loader.path` are a source and resolve in placeholders, with the image's values as the oracle records them; they count for the tester gate | config.placeholder.012, .013, config.gate.022 |
| 3 `spring.application.json` | as an argument and as a system property, with rank and origin | config.args.008, .009 |
| 4 JVM option quoting | the JVM's quoting, and its stderr notices | config.source.023, .024, .025 |
| 5 profile validation and keys | measured messages and forms | config.profile.015-.018 |
| 6 `--=x`, `--`, `a=b` | measured | config.args.010-.012 |
| 7 location syntax | profile variants of named files, `[.ext]` hints, directory imports, wildcards | config.source.026-.029 |
| 8 `/app/extra-classes/` | part of `classpath:` as in HAPI (fixed path, no setting); `DEV-0031` narrowed; the oracle runs the image's launcher | config.source.017 (rewritten), .030, config.oracle.001 |
| 9 setter-less derived properties, `packageContents` | bound as HAPI binds them | config.unknown.008, config.key.009, .012 |
| 10 `ElasticsearchProperties` | a tenth descriptor, with `Duration` | config.gate.020 (rewritten), .024, config.unknown.004 |
| 11 `cr.enabled: yes` | corrected: the quoted text turns the gate off, the YAML boolean turns it on | config.gate.006 (rewritten), .023; `DEBT-0022` replacement text |
| 12 failure output | measured with the starter's `logback.xml`: standard output, log header lines, failure analysis or `Application run failed` by kind; `config.gate.005` is HAPI's `NullPointerException`; `config.placeholder.011` is the condition's own failure | config.error.002 (rewritten), .007-.010, config.dump.006, config.gate.005, config.placeholder.011, config.yaml.003, .016, config.args.007, config.profile.012 (rewritten) |
| 13 untestable text | the escape and the fixture are in the change | config.properties.004, config.yaml.016 (rewritten), `probe/fixtures/` |

### Oracle findings of 2026-11-22 and the owner's rulings

The oracle-builder's first full `config-roundtrip` run raised thirteen
findings (O-1 to O-13) and three test contracts (`tasks/blockers.md`, entry
of 2026-11-22). The owner ruled the same day:

| Item | Ruling | Scenarios and text |
|---|---|---|
| Recording blocked by Spring Framework 6.2.19 | 6.2.19 is the reference; Boot 3.5.14 and SnakeYAML 2.4 unchanged | Risk 1, design.md section 2, oracle.001, task 5 |
| Probe build | compiled with `javac` in a Docker build stage against the image's own `WEB-INF/lib` and classes; no Maven `pom.xml` | design.md section 7, task 1 |
| O-1 logback status lines | stripped and counted by the harness; Frevo writes none; `DEV-0033` extended | config.dump.006, .error.007, .error.009, .oracle.003; `register-entries.md` |
| O-2, O-3 | follow HAPI: lower-case names bind on Linux; a variable named `--spring.config.location` is applied | config.env.004, .014; design.md section 3.4 |
| O-4, O-5 | quote the recorded Jackson cause lines and the unprefixed, indented SnakeYAML lines | config.args.007, .009, config.yaml.003, .016, config.error.007 |
| O-6 | state the no-version failure and the version-set variant, as recorded | config.source.007, .009, .010, .011, .013, config.key.001, .003 |
| O-7 | the gate site names are the probe's choice, not measured in a real context | config.placeholder.011; design.md section 7 |
| O-8 | the identity hash of `Bindable@<hex>` is masked on both sides | config.collection.002, config.oracle.002, .003 |
| O-9 | compare the bean-creation wrapper only through `Error creating bean with name` | config.unknown.005, .008, config.oracle.003 |
| O-10 | reworded: `4.0`/`4.0.1` fail to bind first | config.gate.002 |
| O-11 | 220 inventory rows | config.key.011 |
| O-12, O-13 | clarifying notes | config.relaxed.007, config.key.007 |
| Test contracts | accepted: `FREVO_CONFIG_TEST_ROOT` (test-only), embedded copies in `crates/frevo-config/packaged/`, binary target `frevo-config` | design.md section 11 |

No scenario was added or removed; the count stays 232.

## Registers this change touches

The append-ready text is in [`register-entries.md`](register-entries.md); the
coordinator appends it, because the proposer writes nothing outside this
directory.

- `docs/deviations.md`: `DEV-0031` (unsupported configuration location kinds
  and `on-cloud-platform`; **replacement text** in `register-entries.md`,
  narrowed after finding 8), `DEV-0032` (two non-kebab spellings of one
  property in one source; unchanged), and `DEV-0033` (failure output keeps
  only the config-related lines; approved by the owner on 2026-11-22 and
  registered; **replacement text** in `register-entries.md` for the scope
  extended the same day, now also covering logback's status lines, O-1).
- `docs/tech-debt.md`: `DEBT-0022` (**replacement text**, finding 11) and
  `DEBT-0023` to `DEBT-0026` (unchanged).
- `docs/provenance.md`: one row per ported starter, hapi-fhir and
  clinical-reasoning file and for the three packaged YAML files (task 24).
