# Design — `add-config-application-yaml`

Rules this file answers to, from `openspec/config.yaml` `rules.design`: state
the performance budget with the benchmark that measures it; justify any new
dependency against the PostgreSQL-only rule; say which parts are generated and
by what; record the PostgreSQL schema changes.

- **Schema changes:** none. Configuration is read before any connection is
  opened (config.error.001).
- **Generated code:** none. The descriptor tables of section 5 are hand-written
  ports of the starter's Java classes, with provenance headers. They are not
  marked `@generated`; if a later change wants them generated from the Java
  sources, that is a generator decision for then, not a reason to hand-edit
  generated output now.
- **New dependency:** one YAML event parser, `yaml-rust2`, ADR 0007 (accepted
  by the owner on 2026-11-22; the file stays in this directory until the owner
  moves it to `docs/adr/`), section 4.

## 1. The model in one paragraph

Spring Boot does not "parse application.yaml". It assembles an ordered list of
*property sources* — command-line arguments, `SPRING_APPLICATION_JSON`, Java
system properties, the OS environment, then every configuration document it
loaded, highest first — each a flat map from a source-specific name to a
value with an origin. Binding walks a target class's bean properties and, for
each, asks the sources in order for the first one holding a name equal to the
property's canonical name under relaxed equality; the found value is
placeholder-resolved, converted, and set. Separately, about twenty `Condition`
classes call `Environment.getProperty(name)` with names the binder never sees
in that form. `frevo-config` reproduces those three layers — sources, binder,
named reads — and nothing else of Spring.

## 2. How the behaviour was measured

Every THEN clause in the spec delta that quotes a value, a message or a
`line:column` comes from a probe, not from reading Spring's source or its
documentation:

| Probe input | Version | Where it came from |
|---|---|---|
| The nine properties classes | `hapi-fhir-jpaserver-starter@377e6eba` | `reference/`, compiled unchanged |
| `spring-boot`, `spring-core`, `-beans`, `-context`, `-aop`, `-expression`, `-jcl` | 3.5.14 / 6.2.18 | local Maven cache; 3.5.14 is the starter's `spring_boot_version` (`pom.xml:22`, overriding the parent's 3.5.15) and declares Spring 6.2.18 |
| `snakeyaml` | 2.4 | the version Spring Boot 3.5.x manages |
| `hapi-fhir-base`, `-storage`, `-jpaserver-model`, `org.hl7.fhir.r4`, `.utilities` | 8.12.0 / 6.9.12 | local Maven cache |
| `cqf-fhir-cql`, `cqf-fhir-utility` | 4.13.0 | local Maven cache |
| `cql-to-elm-jvm` | 5.4.0 | local Maven cache |
| Packaged resources | the starter's three YAML files on the class path | `reference/`, unfiltered (only `spring.application.name` differs) |

The probe is a `@Configuration` class with
`@EnableConfigurationProperties` for the nine classes, run by
`SpringApplication` with `WebApplicationType.NONE`, which gives the real config
data pipeline (locations, profiles, imports, environment, arguments,
placeholders, failure analysers). After startup it dumps every getter
reachable from each bean, and the `Environment.getProperty` result for each
named read. About 150 configurations were run.

Two limits, both handed to the oracle (section 7):

1. **It ran on Windows.** Environment-variable names are case-insensitive
   there; config.env.004's Linux clause is reasoned from Spring's mappers and
   is confirmed by the oracle on Linux (owner ruling: Linux semantics).
2. **The jars are from a cache, not from the image.** The oracle's HAPI side
   uses the jars inside `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`
   (`/app/main.war!/WEB-INF/lib/`), and asserts their versions.

## 3. Loading

### 3.1 Sources, highest precedence first

| # | Source | Name form | Origin text in a report |
|---|---|---|---|
| 1 | command-line `--name=value` (config.args) | as written | `"<name>" from property source "commandLineArgs"` |
| 2 | `spring.application.json` / `SPRING_APPLICATION_JSON` JSON, flattened like YAML, placed just above the source that held it (so `--spring.application.json` ranks with the arguments, `-Dspring.application.json` above `SPRING_APPLICATION_JSON`) (config.args.006, .008, .009) | flattened JSON path | `"spring.application.json" from property source "<source>"` or `System Environment Property "SPRING_APPLICATION_JSON"` |
| 3 | Java system properties: the JVM's own properties and `PID` (3.6), then the `-D` tokens of `JAVA_TOOL_OPTIONS`, then of `JDK_JAVA_OPTIONS` (later wins), tokenised with the JVM's quoting, other tokens ignored (config.source.023-.025) | as written | `"<name>" from property source "systemProperties"` |
| 4 | OS environment (config.env) | as set | `System Environment Property "<NAME>"` |
| 5 | `random.*` (only for placeholders) | `random.int`, … | — |
| 6.. | configuration documents, in config-data order (3.2) | flattened key | `URL [file:<path>] - L:C` / `class path resource [<name>] - L:C` |
| last | class defaults | — | — |

`spring.config.*` and `spring.profiles.*` are themselves read from sources 1
to 4 before any file is loaded, which is how `SPRING_CONFIG_IMPORT` and
`SPRING_PROFILES_ACTIVE` work (config.source.014, config.profile.001).

### 3.2 Config data order

Initial contributors, lowest to highest: `spring.config.location` (default
`optional:classpath:/;optional:classpath:/config/;optional:file:./;optional:file:./config/;optional:file:./config/*/`),
then `spring.config.additional-location`, then `spring.config.import` from
sources 1 to 4. Within a location list, later entries win (config.source.013);
`config/*/` subdirectories in name order (config.source.004). Each location
resolves `<name>.properties`, `.xml`, `.yml`, `.yaml` in that order of
*precedence* (config.source.005), `<name>` from `spring.config.name`.
`spring.config.import` inside a document is resolved relative to that
document's directory and ranks just above it (config.source.015).

Profiles: once the non-profile documents are loaded, active profiles are
computed (`spring.profiles.active`, `.include`, `.group.*`, `.default`), and
for every location the `<name>-<profile>.*` files and the `on-profile`
documents are added. The resulting order is: packaged base < packaged
profile < external base < external profile (config.profile.003), later
profiles over earlier (config.profile.004), and a profile *file* over an
`on-profile` *document* of a base file (config.profile.005).

`classpath:` resolves to the three packaged files embedded in the binary
(`include_bytes!`), then to the fixed directory `/app/extra-classes/`, the
last entry of the image's `loader.path`; the first match wins, so a packaged
name shadows the same name there, while `application.properties`,
`application.yml`, `config/application.*` and `application-<profile>.*` for
other profiles are found there (config.source.017, .030). This is not a
setting: it is where the image's class path is. `configtree:` and `.xml`
locations fail to start with a message naming the location and the supported
kinds, and `spring.config.activate.on-cloud-platform` never matches
(`DEV-0031`).

A named file location or import also loads `<file>-<profile>.<ext>` for each
active profile (config.source.026); a location may carry an extension hint
`[.yaml]` (config.source.027); a directory location or import loads
`<name>.*` and its profile variants (config.source.028); one `*/` wildcard
expands to the immediate subdirectories in name order, two fail
(config.source.029).

### 3.3 Document readers

- **YAML** (section 4): event stream, YAML 1.1 resolution, anchors and merge
  keys, duplicate-key rejection, then Spring's `YamlProcessor` flattening:
  nested keys joined with `.`, a key beginning `[` appended without a dot,
  sequences as `[i]`, an empty sequence as an empty value, an empty mapping as
  nothing, `null` as the empty string, and a non-string key as its resolved
  text (config.yaml.012).
- **Properties**: `OriginTrackedPropertiesLoader` syntax (config.properties).
- **JSON** (`SPRING_APPLICATION_JSON` only): `serde_json`, flattened like YAML,
  numbers and booleans keeping their JSON type (config.args.006).

Every value keeps its origin: source description plus 1-based line and
column of the value's first character (config.error.005).

### 3.4 Name mapping per source

A configuration name is a list of *elements* split at `.` (with `[...]`
indexes as elements). Two elements are equal when their lower-cased
letters-and-digits are equal (`ConfigurationPropertyName` "uniform" form).

- Documents, arguments, JSON: the source name is split at `.` (Spring's
  `DefaultPropertyMapper`).
- Environment: each variable is mapped twice. As above (so `hapi.fhir.x_y`
  works), and by `SystemEnvironmentPropertyMapper`: split at `_`, lower-cased,
  an all-digit element becoming an index. A property is looked up in the
  environment under its uniform form (`HAPI_FHIR_DEFAULTPAGESIZE`) and its
  legacy form (`HAPI_FHIR_DEFAULT_PAGE_SIZE`, dashes to underscores,
  upper-cased) (config.env.001-.011).

Within one source, the canonical kebab-case name is looked up first, so it
wins over other spellings (config.relaxed.006); two other spellings of one
property in one source fail to start, naming both (`DEV-0032`).

### 3.5 Placeholders

`${name}`, `${name:default}`, nested, `\${` escaping, circular detection. A
placeholder name is resolved against all sources like a named read, so a valid
canonical name is relaxed (config.placeholder.003). The binder resolves
leniently — an unresolvable placeholder stays as text and may then fail
conversion (config.placeholder.004, .005). Named reads resolve strictly
(config.placeholder.011). `random.*` follows `RandomValuePropertySource`.

### 3.6 The JVM's system properties

HAPI's system-properties source holds more than `-D` tokens: every standard
JVM property, `PID` (set by Spring's `LoggingApplicationListener` before
binding) and `loader.path` (set by the image's ENTRYPOINT). They resolve in
placeholders and their names count for the tester gate's prefix scan
(config.placeholder.012, .013, config.gate.022). Frevo provides the set the
oracle records from `System.getProperties()` inside the pinned image, by two
rules: `user.dir` (the working directory), `user.home`, `user.name` (the
process user) and `PID` (the process id) take the running process's values;
every other name (`java.version`, `java.vendor`, `java.home`,
`java.class.path` = `/app/main.war`, `java.io.tmpdir` = `/tmp`, `os.name` =
`Linux`, `os.arch`, `file.separator` = `/`, `path.separator` = `:`,
`line.separator`, `loader.path`, …) takes the value recorded from the image,
as a table generated from the oracle's record. The JVM-option notices
(`NOTE: Picked up JDK_JAVA_OPTIONS: …`, `Picked up JAVA_TOOL_OPTIONS: …`) go
to standard error first, as the JVM writes them (config.source.023).

## 4. YAML: the parser and the resolver (ADR 0007)

SnakeYAML under Spring Boot is YAML 1.1 with timestamps turned off. That is
not what Rust's YAML crates do. `serde_yaml_ng` (already named, unused, in the
workspace `Cargo.toml`) resolves plain scalars by YAML 1.2 rules inside
`unsafe-libyaml`, and discards the original text before Frevo can see it:
`off` would become the string `"off"` instead of `false` → `"false"`
(config.coerce.013), `010` would be 10 instead of 8, `1:20` a string instead
of 80. None of that is recoverable after deserialisation.

So the design needs an **event-level** parser that reports, per scalar, the
raw text, its style (plain or quoted), any explicit tag and its position.
Resolution is then Frevo's own code: SnakeYAML 2.4's `Resolver` regular
expressions for `bool`, `int` (decimal, `0x`, leading-`0` octal, `0b`,
underscores, sexagesimal), `float` (with `.inf`, `.nan`, sexagesimal), `null`,
and `merge`, with `timestamp` removed as Spring's `NoTimestampResolver` does.
The resolver is small (one table) and is covered by config.yaml.004-.008.

**Rendering numbers as text.** A YAML float bound to a `String` reads as Java's
`Double.toString`; an integer as `Long.toString`/`BigInteger.toString`. JDK 21
(the distroless image's runtime) prints the shortest round-tripping digits
(the same digits Rust's `{}` produces for `f64`) in Java's layout: always a
`.`, `E` notation outside `[1e-3, 1e7)`. A small formatter over Rust's digits
gives that layout (config.yaml.006).

ADR 0007's options: (A) `yaml-rust2` event parser — pure Rust, MIT/Apache-2.0,
positions and styles on every event; (B) `saphyr-parser`, the same lineage,
younger; (C) a hand-written YAML parser — no dependency, but YAML's block and
flow grammar is the largest single piece of this change; (D) `serde_yaml_ng`
— ruled out above. **Decided: A**, accepted by the owner on 2026-11-22
(constitution section 5); the unused `serde_yaml_ng` workspace entry is
removed.

## 5. Binding

### 5.1 Descriptor tables, not `serde` derive

Each of the nine classes is ported to a static descriptor: for each bean
property, its canonical name, type, class default, whether it has a setter,
a getter, or neither, and its quirk if any. `serde` derive cannot express
this binder: relaxed names per element, "empty keeps the existing value" for
some types and not others, objects that exist only if something beneath them
bound, collections taken whole from one source while maps merge, setter-less
properties that fail only when the value changes, and two beans binding the
same keys. The descriptor is data; one binder walks it.

The quirks the tables must carry, each with its scenario:

| Quirk | Where | Scenario |
|---|---|---|
| bean property name differs from field (`errorSeverityLevel` vs `error_level`) | `CqlCompilerProperties` | config.relaxed.004 |
| not a setter (`EnableValidation(boolean)`) | `CqlRuntimeProperties` | config.unknown.005 |
| getter-only, `null` map (`getRemoteTerminologyServicesMap`) | `AppProperties` | config.unknown.006 |
| setter maps `0` to `null` | `setReuse_cached_search_results_millis` | config.key.008 |
| getter-only mutable collections | `auto_version_reference_at_paths`, `local_base_urls`, `logical_urls`, `custom_interceptor_classes`, `custom_provider_classes` | config.collection.006 |
| setter-only properties | `CqlTerminologyProperties` | config.profile.001 |
| fluent setters returning `this` | `PackageInstallationSpec`, `TerminologyServerClientSettings` | config.key.009, config.key.003 |
| object defaults `null` vs instance | `cors`, `partitioning`, `elasticsearch`, `subscription.email`, `tester`, … | config.object.005 |
| derived getter returning a new object, no setter: binding anything beneath fails with `No setter found` | `CqlData.getRetrieveSettings` (`cr/CqlData.java:39`), `CqlProperties.getRetrieveSettings` (`cr/CqlProperties.java:70`), `CqlTerminologyProperties.getTerminologySettings` (`cr/CqlTerminologyProperties.java:35`) | config.unknown.008 |
| `byte[]` property bound from a comma list of byte values | `PackageInstallationSpec.setPackageContents` (`PackageInstallationSpec.java:210`) | config.key.012 |
| a tenth descriptor, bound only for a gate | Spring Boot 3.5.14 `ElasticsearchProperties` (and its `Restclient.Sniffer`, `Restclient.Ssl`), `Duration` typed | config.gate.020, .024 |

### 5.2 Binder rules

Per property, in the order Spring's `Binder.bindObject` applies them:

1. Find the first source with a name equal to the property name. If none and
   no source has any name beneath it, leave the property as it is.
2. Collections: the whole list or set comes from the highest source that has
   the property or any `[i]` beneath it; a comma-separated scalar splits and
   trims; `[i]` must be contiguous from 0 (config.collection.001, .002). Maps:
   every source contributes entries; entry keys are the original-form element
   (case kept), and a scalar-valued map takes the remaining elements as one
   dotted key (config.collection.003-.005, config.relaxed.008).
3. Scalars: resolve placeholders, convert (5.3). An empty value for a
   non-`String`, non-collection type converts to "no value" and leaves the
   property as it is (config.coerce.005, .011, .020).
4. Objects: if a scalar converter is not found, try binding the object's own
   properties; if none bound, fail with the converter error, else use the
   object (config.object.003, .004). An object property that was `null` is
   created only if at least one of its properties bound (config.object.001,
   .002).
5. A bound value with no setter is an error unless it equals the current
   value (config.unknown.005).

### 5.3 Conversion

`Boolean`: YAML booleans; text `true/on/yes/1`, `false/off/no/0`, any case,
trimmed; no Integer→Boolean conversion. Integers: text with all whitespace
removed, optional sign, `0x`/`#` hex; YAML numbers narrowed with overflow
checks and truncation. Enums: exact name, then letters-and-digits
case-insensitive match; YAML booleans map to constants `ON`/`OFF`; YAML
integers by ordinal. `Duration` (only in `ElasticsearchProperties`):
ISO-8601 in any case, or a signed integer with an optional unit
`ns|us|ms|s|m|h|d` in any case, milliseconds by default (config.gate.024).
`byte[]`: a comma list of decimal byte values. `String` from any YAML scalar
via 4's rendering. All
failure texts as the spec quotes them, verbatim, Java type names included:
a table from (target type, failure kind) to Spring's text.

## 6. Named reads and gates

A named read is `Environment.getProperty(name)`: if `name` is a valid
canonical name (lower-case, `-`, digits), it is looked up relaxed exactly like
the binder; otherwise each source is asked for the literal `name`, and the
environment source also tries `name` with `.`→`_`, `-`→`_`, both, and the
upper-case form of each (config.read.004). Values are text; placeholders are
resolved strictly.

Gates are pure functions of the reads and of the bound properties, one per
starter condition (config.gate.001), with the starter's exact predicate:
`Boolean.parseBoolean` for `CrConfigCondition`/`CdsHooksConfigCondition`,
`havingValue` with `equalsIgnoreCase` for `@ConditionalOnProperty`, "present
and not `false`" when there is no `havingValue`, `forVersionString` for the
version conditions, non-null/non-empty bound objects for the `On*Present`
conditions, a raw string-prefix scan of enumerable source names for
`FhirTesterConfigCondition`, a bind of `ElasticsearchProperties` for
`ElasticConfigCondition`, and HAPI's own case-sensitive `"true".equals` for
`scheduling_disabled` (`BaseSchedulerServiceImpl.java:319`,
`SubscriptionMatcherInterceptorConfig.java:47`). There are thirteen reads and
twenty-five gates. Later changes read gates, never re-derive them.

## 6a. Failure output

Measured with the starter's `logback.xml` (which includes Spring Boot's
`console-appender.xml`, a `ConsoleAppender` on standard output). Every
startup failure goes to **standard output** as log lines; nothing goes to
standard error except the JVM-option notices. Two forms:

- **Failure analysis** (config.error.002): one `ERROR` line from
  `o.s.b.d.LoggingFailureAnalysisReporter` with an empty message, then the
  `APPLICATION FAILED TO START` block. Used for property binding failures and
  for missing config data resources and locations.
- **Run failed** (config.error.007): one `ERROR` line from
  `o.s.boot.SpringApplication` with `Application run failed`, a blank line,
  and the exception chain (`<class>: <message>`, `Caused by: …`). Used for
  everything else, including HAPI's own `NullPointerException` when no
  version is configured (config.gate.005).

Before either, a failure during context refresh is preceded by HAPI's `WARN`
line from the application context and, for binding and gate-placeholder
failures, an `INFO` line from `ConditionEvaluationReportLogger`
(config.error.008). Log lines use the console pattern; the process id and
application name are absent when the failure precedes logging
initialisation. Java stack frames are not reproduced (`DEV-0033`, proposed).
A table maps each failure kind to its exception class names and messages,
generated from the oracle's record.

## 7. The `config-roundtrip` oracle

New, in `harness/config-roundtrip/` (oracle-builder), shaped like
`harness/hapi-fhirpath-results/`:

- **HAPI side:** a Maven module containing the probe class, a dumper that maps
  each getter to the canonical path of config.dump.002, the starter's
  condition classes evaluated against the started `Environment`, and a
  runner, packaged into the image's `/app/extra-classes/` and started by the
  image's own launcher (`PropertiesLauncher` with the ENTRYPOINT's
  `loader.path`), so the class path and its order, `/app/extra-classes/`, the
  packaged `logback.xml` and the exact jars HAPI 8.12.0 runs are all HAPI's,
  on Linux. It also records `System.getProperties()` for section 3.6, and
  asserts that no `WEB-INF/lib` jar carries an application file.
- **Cases:** one directory per case: `files/` (copied into an empty working
  directory), `env` (one `NAME=value` per line, nothing inherited), `args`,
  and `covers:` tags. The three whole-surface cases of config.key.005-.007
  are generated from `keys.md` so that a key added there cannot be missed.
- **Recording:** the HAPI side's documents are recorded in
  `harness/config-roundtrip/hapi-8.12.0-results.json` with the image digest,
  so CI compares Frevo against the record without Docker, and a scheduled job
  re-records to catch drift. This mirrors
  `oracles/fhirpath/hapi-8.12.0-results.json`.
- **Comparison:** member for member; sets sorted; nothing else normalised
  (config.oracle.004). A failing HAPI case passes only if Frevo also fails and
  its standard output equals HAPI's line for line after masking timestamps and
  process ids and removing Java stack frames (config.oracle.003).

## 8. The effective-configuration document

```json
{
  "profiles": { "active": ["cds"], "default": ["default"] },
  "properties": {
    "hapi.fhir": { "default-page-size": 20, "cors": { "allowed-origin": ["*"], "...": "..." }, "...": "..." },
    "hapi.fhir.cdshooks": { "enabled": true, "client-id-header-name": "client_id" },
    "hapi.fhir.cr": { "...": "..." },
    "hapi.fhir.cr.caregaps": { "...": "..." },
    "hapi.fhir.cr.cql": { "...": "..." },
    "hapi.fhir.cr.cql.compiler": { "...": "..." },
    "hapi.fhir.cr.cql.runtime": { "...": "..." },
    "hapi.fhir.cr.cql.terminology": { "...": "..." },
    "hapi.fhir.cr.cql.data": { "...": "..." }
  },
  "reads": { "hapi.fhir.fhir_version": "R4", "...": "..." },
  "gates": { "fhir-r4": true, "...": false }
}
```

Derived getters (`getRetrieveSettings`, and
`CqlTerminologyProperties.getTerminologySettings` as a wrapper) are not
reported, the last flattened into its four modes; binding beneath them fails
(config.unknown.008). `package-contents` is a real property and is reported as
an array of integers or `null` (config.key.012). The
password redaction of config.dump.005 is applied on both sides of the oracle.

`frevo-config effective` is a binary target of `frevo-config` (owner
decision of 2026-11-22).
Arguments after `effective` are the configuration's command-line arguments.

## 9. Performance budget

Configuration is not on a read, search or ingest path. It is a startup cost
and, afterwards, field reads.

| Workload (`bench/micro/workloads.yaml`) | Measures | Budget | Scenario |
|---|---|---|---|
| `config-load-packaged` | load and bind the three packaged files plus `bench/micro/fixtures/config-all-keys.yaml` (120 lines), no environment | median ≤ 5 ms | config.perf.001 |
| `config-get` | read `hapi.fhir.default-page-size` from the loaded, immutable configuration | median ≤ 5 ns, zero allocations | config.perf.002 |

Both compare against `bench/micro/baseline.json` with the 10% rule
(config.perf.003). The loaded configuration is an immutable value shared by
`Arc`; nothing re-parses or re-binds after startup, so constitution section 6
is met trivially and there is no before/after number for any HTTP workload.

## 10. What a consumer change gets

A typed, immutable `Config` with one accessor per descriptor property, the
reads, the gates, and a generic `bind::<T>(prefix)` over the same sources for
the consumer's own `spring.*`/`server.*` classes (ruled 2026-11-22: typed
binding of those keys belongs to the consuming change). The
accessors are the only API consumers use; nothing outside `frevo-config` sees
a source, a name mapper or a YAML event.
