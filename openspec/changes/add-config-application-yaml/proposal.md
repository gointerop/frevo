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
   packaged `false`; `cr.enabled: yes` binds `true` while
   `Boolean.parseBoolean` turns the CR gate off; `fhir_version: ""` keeps
   `R4` bound and turns on the **DSTU2** gate. This change computes every gate
   with HAPI's own predicate (config.read, .gate), so later changes wire
   features from the answer HAPI reaches.
4. **Failure reports** in the starter's `APPLICATION FAILED TO START` form and
   exit status `1`, before anything else starts (config.error).
5. **`frevo-config effective`**, a command that prints the effective
   configuration as JSON (`profiles`, `properties` per bean prefix, `reads`,
   `gates`). It is the observable surface for every scenario here and the
   Frevo side of the `config-roundtrip` oracle (config.dump).

Scenario count: **200** across 19 areas (see [`spec.md`](spec.md)).

### How the behaviour was established

Not from memory of Spring Boot and not from the starter's comments. A probe —
the starter's nine properties classes compiled from the pinned commit, booted
by Spring Boot 3.5.14 with the HAPI 8.12.0, clinical-reasoning 4.13.0 and CQL
5.4.0 jars, dumping every getter — was run against about 150 configurations,
and every value, message and line:column in a THEN clause is what it printed
([`design.md`](design.md) section 2). The `config-roundtrip` oracle re-runs the
same cases against the jars inside the pinned image on Linux; it, not this
text, is the standard.

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
| Loading, binding, conversion and YAML resolution rules | `spring-projects/spring-boot` 3.5.14, `spring-projects/spring-framework` 6.2.18, `snakeyaml` 2.4 | `ConfigDataEnvironment`, `StandardConfigDataLocationResolver`, `OriginTrackedYamlLoader`, `OriginTrackedPropertiesLoader`, `Binder`, `JavaBeanBinder`, `MapBinder`, `IndexedElementsBinder`, `ConfigurationPropertyName`, `SystemEnvironmentPropertyMapper`, `DefaultPropertyMapper`, `LenientStringToEnumConverterFactory`, `BindFailureAnalyzer`; Spring's `YamlProcessor`, `SystemEnvironmentPropertySource`, `PropertyPlaceholderHelper`, `NumberUtils`, `StringToBooleanConverter`; SnakeYAML's `Resolver` | **not pinned in `reference/`** (Open Question 5) | **derived behaviour** — measured by the probe, judged by the oracle; no source is ported |

## Oracles

| Scenario group | Oracle | Case selector |
|---|---|---|
| `config.source.*`, `.yaml.*`, `.properties.*`, `.profile.*`, `.env.*`, `.args.*`, `.placeholder.*`, `.relaxed.*`, `.coerce.*`, `.collection.*`, `.object.*`, `.unknown.*` (except `.007`), `.read.*`, `.gate.*`, `.key.*` | **`config-roundtrip`** — new, `harness/config-roundtrip/` (design.md section 7): each case run by Spring Boot 3.5.14 with the starter's classes and conditions, using the jars inside `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`, on Linux; documents compared member for member | every case under `harness/config-roundtrip/cases/` tagged `covers: <id>`; `config.oracle.005` requires one per scenario with a configuration |
| `config.key.005`, `.006`, `.007` | `config-roundtrip` | the three whole-surface cases `all-keys.yaml`, `all-keys.env`, `all-keys-kebab.yaml` |
| `config.key.002` (CORS defaults) | `config-roundtrip`, plus HAPI's JUnit `AppPropertiesCorsDefaultsTest` (starter `src/test/java/ca/uhn/fhir/jpa/starter/`) for the defaults the class declares | `defaultCorsHeadersIncludeFhirOptimisticLockingHeaders`, `nullCorsListsFallBackToDefaults` |
| `config.error.*` | `config-roundtrip` failure comparison (`config.oracle.003`); report text compared line for line where Open Question 9 settles on verbatim | cases whose HAPI run fails |
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
2. **Spring Boot is not in `reference/`** (Open Question 5). Adding
   `spring-projects/spring-boot@v3.5.14`, `spring-framework@v6.2.18` and
   `snakeyaml@snakeyaml-2.4` is a `chore/*` branch and an owner decision,
   because it adds to what "correct" refers to.
3. **`docs/inventory.md` needs correcting** (`keys.md` section 7) on a
   `chore/*` branch: `harness/scripts/gen-inventory.py` is not this branch's to
   edit.
4. **ADR 0007** (the YAML parser dependency) needs the owner's approval before
   task 7 can add the crate. The draft is
   [`adr-0007-yaml-event-parser.md`](adr-0007-yaml-event-parser.md), to be
   moved to `docs/adr/` when decided.

## Out of scope

| Out of scope | Why | Covered by |
|---|---|---|
| What each `hapi.fhir.*` key does | this change binds, validates, reports and gates; behaviour belongs to the feature | the change named per key in `keys.md` section 5 |
| Typed binding of `spring.datasource.*`, `spring.jpa.*`, `server.*` (port, context path, `tomcat.relaxed-query-chars`), `management.*`, `spring.flyway.*`, `spring.main.*`, `spring.ai.mcp.server.*` other than `enabled`, `logging.*` | their meaning is the consumer's; this change loads them through the same pipeline and makes them available to reads (config.unknown.004) — Open Question 12 | `add-server-skeleton` (`server.*`, `spring.main.*`), `add-store-schema` (`spring.datasource.*` and the JDBC URL form), `add-observability` (`management.*`, `logging.*`), `add-mcp` |
| Loading `custom_interceptor_classes`, `custom_provider_classes`, `custom-bean-packages` | they name Java classes and packages; this change binds the lists and the read. HAPI fails to start on a class not on its class path (`StarterJpaConfig.java:572-577`), which is what Frevo's empty class path will give | `add-server-skeleton` |
| `/actuator/configprops`, `/actuator/env` | not exposed by the packaged `management.*` settings; the document of config.dump is the diagnostic | `add-observability` |
| Java system properties (`-Dhapi.fhir.x=y` through `JAVA_TOOL_OPTIONS`) | Frevo has no JVM | Open Question 6 |
| `configtree:` imports, `.xml` properties files, `spring.config.activate.on-cloud-platform` | HAPI supports them; their Frevo disposition is Open Question 7 | this change, once decided |
| The Tomcat variant of the image (`catalina.properties`, `server.xml`, JNDI, servlet init parameters) | a deployment form, not a configuration source of the default (distroless) image the harness pins | none |
| Re-reading configuration while running | Spring Boot does not reload `@ConfigurationProperties` in the starter either | none |
| The README's `docker run -e "--spring.config.location=…"` form | an environment variable whose *name* starts with `--`; Spring's mappers do not turn it into `spring.config.location`, so HAPI most likely ignores it | Open Question 14 |

## Target crate

`frevo-config` only, the crate `tasks/graph.yaml` names for this change. The
`frevo-config effective` binary lives in it. `harness/config-roundtrip/` is
test infrastructure. No PostgreSQL schema change.

## Risks

1. **The reference for half of this change is not pinned.** Spring Boot's
   binder and SnakeYAML's resolver define most of the behaviour, and neither
   is in `reference/`. The probe used jars from a local Maven cache whose
   versions match the starter's `pom.xml` (`spring_boot_version` 3.5.14,
   `pom.xml:22`; the parent's 3.5.15 is overridden), but the image might ship
   different ones. Task 1 reads the versions out of the image and fails if
   they differ.
2. **The probe ran on Windows.** Environment-variable names are
   case-insensitive there, so the lower-case `hapi_fhir_default_page_size`
   bound in the probe and almost certainly does not on Linux, where HAPI runs.
   The oracle runs on Linux; config.env.004 is pending its result (Open
   Question 8).
3. **Faithful parity reproduces hazards.** The split between named reads and
   binding (config.read.005, config.gate.003, .006, .007), request-logger
   templates expanded from the environment (config.placeholder.009), and a
   silently ignored `error_level` are HAPI's behaviour and the constitution
   says to match it. Each is proposed as a `DEBT` entry, not a flag (Open
   Question 3).
4. **This is a small Spring Boot.** The binder's rules are many and
   individually odd (an empty value keeps an `Integer`'s default but empties a
   `List`; a scalar where an object is expected is ignored only if some other
   source has keys beneath it). The descriptor tables and the probe's case
   list are what keep it honest; the whole-surface round-trips
   (config.key.005-.007) are what catch a missed key.
5. **The parity denominator moves** (`keys.md` section 7, Open Question 11).

## Open questions

Each lists the recommended option first. None is decided here; scenarios that
depend on one say "pending Open Question N" and state the recommendation.

1. **The observable surface.** *Recommended:* the `frevo-config effective`
   command of config.dump, printing JSON shaped like the HAPI side of the
   oracle, with `hapi.fhir.subscription.email.password` redacted. *Alternative:*
   test only through the crate's Rust API and defer any process-level output
   to `add-observability`. The command is a Frevo-only diagnostic, not a
   configuration flag; without it the differential oracle has nothing to diff.
2. **`classpath:` locations.** *Recommended:* `classpath:` resolves only to the
   three packaged files; any other `classpath:` resource fails with the
   `does not exist` report (config.source.017), recorded as a `DEV` entry
   with a debt trigger "an operator needs `classpath:` resources". HAPI's
   distroless image puts `/app/extra-classes/` on its class path
   (`Dockerfile`, `-Dloader.path=…,/app/extra-classes`), so
   `classpath:/x.yaml` can find a file an operator mounted there.
   *Alternative:* resolve `classpath:` to the packaged files, then
   `/app/extra-classes/` — exact for that image layout, meaningless elsewhere.
3. **Reproducing the read/binding split and the other hazards.**
   *Recommended:* reproduce them exactly (config.read.005, config.gate.003,
   .006, .007, config.placeholder.009, config.relaxed.004) and record five
   `DEBT` entries: (a) a spelling the named reads do not see silently splits a
   feature's gate from its bound value; (b) `fhir_version: ""` selects DSTU2;
   (c) a `requestVerb`-style environment variable rewrites the request-log
   templates; (d) `cr.cql.compiler.error_level` is accepted and ignored;
   (e) `enable_validation: true` cannot be set. Trigger for each: an operator
   report or a consumer change that needs the safer behaviour. *Alternative:*
   fail to start when a gate and its bound property disagree — safer, but a
   `DEV` entry against every one of those cases.
4. **Keys this build does not act on yet.** *Recommended:* bind and report
   them, and print one warning per key set to a non-default value whose
   behaviour no merged change implements (config.unknown.007), so that
   `bulk_export_enabled: true` on an early build is not silently a no-op. The
   list is `keys.md`'s "Wired to behaviour by" column filtered by
   `tasks/graph.yaml` status. Record as a `DEV` entry (HAPI prints no such
   line). *Alternatives:* (b) silent, exactly as HAPI is; (c) fail to start.
5. **Pinning the Spring sources.** *Recommended:* add
   `spring-projects/spring-boot` `v3.5.14`, `spring-projects/spring-framework`
   `v6.2.18` and SnakeYAML `snakeyaml-2.4` to `reference/PINS.md` on a
   `chore/*` branch, and have the oracle assert the image's jar versions.
   *Alternative:* rely on the oracle alone and cite no Spring source line.
6. **Java system properties.** *Recommended:* honour `-Dname=value` tokens of
   `JAVA_TOOL_OPTIONS` and `JDK_JAVA_OPTIONS` as the system-properties source,
   between `SPRING_APPLICATION_JSON` and environment variables, ignoring every
   other token — a deployment that configures HAPI through
   `JAVA_TOOL_OPTIONS` then switches unchanged. *Alternatives:* (b) ignore
   them, with a `DEV` entry; (c) fail to start if either variable carries a
   `-Dhapi.`, `-Dspring.` or `-Dserver.` token, with a `DEV` entry.
7. **`configtree:`, `.xml` and `on-cloud-platform`.** *Recommended:* fail to
   start on `configtree:` and `.xml` with a named, supported-kinds message
   (config.source.020), and treat `spring.config.activate.on-cloud-platform`
   as never matching, all recorded as one `DEV` entry with the trigger "a
   deployment uses one". *Alternative:* implement them now (`configtree:` is
   the Kubernetes-secrets idiom and is small; `.xml` is Java's
   `Properties.loadFromXML`; cloud-platform detection reads
   `KUBERNETES_SERVICE_HOST`/`_PORT`).
8. **Environment-variable case on Linux.** *Recommended:* specify Linux
   semantics (names are case-sensitive; `hapi_fhir_default_page_size` and
   `Hapi_Fhir_…` do not bind), and let the oracle's Linux run confirm before
   config.env.004's second clause is final. *Alternative:* match whatever the
   host OS does.
9. **The failure report's Java type names.** *Recommended:* print the
   starter's report verbatim, including the Java class names in `to
   java.lang.Boolean:` and `Reason:` (they are fixed strings per property
   type and failure kind, so a table reproduces them), so that operators'
   log alerts and runbooks keep working. *Alternative:* the same block
   structure and the same `Property:`, `Value:` and `Origin:` lines, with
   Frevo's own `Reason:` text, recorded as a `DEV` entry.
10. **Two non-kebab spellings of one property in one source.** HAPI starts and
    picks one by `java.util.HashSet` iteration order (it took
    `default_page_size` over `defaultPageSize`, and `max_page_size` over
    `maxPageSize`, whatever their order in the file). *Recommended:* fail to
    start naming both spellings (config.relaxed.007), with a `DEV` entry —
    the YAML is ambiguous and HAPI's answer is an accident of hashing.
    *Alternative:* reproduce Java's `String.hashCode` and `HashSet` iteration
    order exactly.
11. **The inventory denominator.** *Recommended:* correct `docs/inventory.md`
    as `keys.md` section 7 lists (remove 3 artefact rows, reclassify 20, add
    the section 6 keys), on a `chore/*` branch before this change archives,
    and restate the README parity percentage. *Alternative:* keep 209 and
    track the differences only in `keys.md`.
12. **Non-`hapi.fhir` keys.** *Recommended:* load every key through this
    pipeline and expose it to reads, but bind and type-check only the
    starter's nine classes here; each consumer change binds its own
    `spring.*`/`server.*` keys with this binder (out-of-scope table).
    *Alternative:* bind `spring.datasource.*` and `server.port` here, since
    every deployment sets them.
13. **The YAML parser.** *Recommended:* ADR 0007 option A, the `yaml-rust2`
    event parser plus a hand-written SnakeYAML 1.1 resolver and Spring
    flattening, and removal of the unused `serde_yaml_ng` workspace entry.
    Needs owner approval (constitution section 5).
14. **The README's `-e "--spring.config.location=…"`.** *Recommended:* match
    HAPI, which (by Spring's mappers) ignores an environment variable with
    that name, once the oracle confirms it on Linux; add a case. *Alternative:*
    treat a `--`-prefixed environment variable name as an argument — a
    convenience HAPI does not offer, needing a `DEV` entry.

## Registers this change will touch

Proposed, to be written when the open questions are answered (the proposer
does not write outside this directory):

- `docs/deviations.md`: entries for Open Questions 2, 4, 7 and 10 as
  recommended, and 6 and 9 if their alternatives are chosen.
- `docs/tech-debt.md`: the five entries of Open Question 3.
- `docs/provenance.md`: one row per ported starter file and for the three
  packaged YAML files.
