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
| `spring-boot`, `spring-core`, `-beans`, `-context`, `-aop`, `-expression`, `-jcl` | 3.5.14 / 6.2.18 | local Maven cache; 3.5.14 is the starter's `spring_boot_version` (`pom.xml:22`, overriding the parent's 3.5.15), whose BOM declares Spring 6.2.18. **The pinned image ships 6.2.19 instead** (see below) |
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

Two limits, both handed to the oracle (section 7), and both since settled by
its record:

1. **It ran on Windows.** Environment-variable names are case-insensitive
   there. The oracle's Linux run shows that the all-lower-case form binds on
   Linux as well, because Spring Boot 3.5.14's mapper looks up lower-case
   forms too (section 3.4); only mixed case does not (config.env.004).
2. **The jars are from a cache, not from the image.** The oracle's HAPI side
   uses the jars inside `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`
   (`/app/main.war!/WEB-INF/lib/`), and asserts their versions. They are
   `spring-boot` 3.5.14 and `snakeyaml` 2.4, as the probe used, but
   `spring-core`, `spring-beans`, `spring-context`, `spring-aop` and
   `spring-expression` **6.2.19**, `slf4j-api` 2.0.16, and no `spring-jcl`
   jar. **The owner ruled on 2026-11-22 that Spring Framework 6.2.19 is the
   reference**, because it is what HAPI 8.12.0's image runs; Spring Boot
   3.5.14 and SnakeYAML 2.4 are unchanged. The oracle checks those three
   versions and records nothing if they differ.

### Oracle findings of 2026-11-22

The oracle's first full run (`harness/config-roundtrip/diagnostic-spring-core-6.2.19.json`,
recorded against the 6.2.19 image before the ruling) contradicted or
sharpened thirteen points of the first probe's text (`tasks/blockers.md`,
entry of 2026-11-22, O-1 to O-13). The owner ruled the same day that HAPI's
recorded behaviour wins; the delta now says:

| Finding | What the record shows | Where the delta says it |
|---|---|---|
| O-1 | logback prints its own status lines because the packaged `logback.xml` sets `scan="true"` inside a jar: on standard output on every run (57 lines), on standard error on every run except the 23 that fail before the application context exists (2 lines in the other 396) | config.dump.006, .error.007, .error.009, .oracle.003; `DEV-0033` extended (`register-entries.md`) |
| O-2 | lower-case `hapi_fhir_default_page_size` binds on Linux; mixed case does not | config.env.004, section 3.4 |
| O-3 | an environment variable named `--spring.config.location` is applied as the location | config.env.014, section 3.4 |
| O-4 | invalid JSON is reported through Jackson's `JsonParseException` | config.args.007, .009 |
| O-5 | SnakeYAML errors print without a class prefix, source lines indented four spaces plus their own | config.yaml.003, .016, config.error.007 |
| O-6 | replacing the default locations without a version fails as config.gate.005 does | config.source.007, .009, .010, .011, .013, config.key.001, .003, and the notation comment |
| O-7 | the class a strict-placeholder gate failure names was the probe's condition site; superseded by the re-record below | config.placeholder.011, section 7 |
| O-8 | `Bindable@<hex>` is an identity hash that changes between runs | config.collection.002, config.oracle.002, .003 |
| O-9 | the bean-creation wrapper's wording after `Error creating bean with name` was the probe context's; superseded by the re-record below | config.unknown.005, .008, config.oracle.003 |
| O-10 | `4.0` and `4.0.1` fail to bind before any gate is reported | config.gate.002 |
| O-11 | `docs/inventory.md` has 220 configuration rows after #39 | config.key.011, `keys.md` section 7 |
| O-12 | HAPI picks the snake-case spelling in config.relaxed.007's second run, as stated | config.relaxed.007 (a note) |
| O-13 | the kebab-case corpus's reads and gates differ from config.key.005's, in HAPI too | config.key.007 (a note) |

The record also shows the action text of an enum conversion failure (`The
following values are valid:` and the constants), which the first draft did not
quote; config.error.002 now states it.

### Re-record from HAPI's real `Application` (review of `831c5ba`)

The fourth review found that the probe is not HAPI's application context: it
does not run the starter's component scan, so a failure during context
refresh is wrapped differently, or is a different failure, in the real
`ca.uhn.fhir.jpa.starter.Application`. The lead ruled on 2026-11-22 that the
HAPI reference is the image's real `Application`, and the oracle-builder
re-recorded (`tasks/blockers.md`, entry "re-recorded from HAPI's real
`Application`"): every case the probe fails (80 of 419) is recorded from a
second run of the unchanged image (config.oracle.001). 23 of the 80 fail
before the context exists and print the same config lines in both runners;
the other 57 differ, and the delta now quotes the real `Application`:

| What the real `Application` prints | Scenarios |
|---|---|
| a refresh-time failure is wrapped in `BeanDefinitionStoreException: Failed to read candidate component class: URL [jar:file:/app/main.war!/WEB-INF/classes!/…]`, naming the class whose condition the component scan was evaluating: `common/PartitionModeConfigurer.class` for every `AppProperties` binding failure, `common/FhirServerConfigDstu2.class` for a missing version, `elastic/ElasticsearchBootSvcImpl.class` for `spring.elasticsearch.*`, the configurer itself for a strict placeholder | config.error.008 (table), config.error.002, config.gate.005 and its family, config.gate.024, config.placeholder.011, config.collection.002, config.dump.006 |
| the `ConditionEvaluationReportLogger` `INFO` line appears only for the `spring.elasticsearch.*`, strict-placeholder and CQL-bean failures, never for an `AppProperties` binding failure or a missing version | config.error.008 |
| five of config.placeholder.011's eight keys never reach a gate: `AppProperties` binds `${nope}` leniently first and fails to convert it | config.placeholder.011, config.read.007 |
| the CQL beans are `cqlProperties`, `cqlData` and `cqlTerminologyProperties`, and a bean's description names its CGLIB proxy class (`CqlData$$SpringCGLIB$$0`) | config.unknown.005, .008 |
| the `WARN` logger is `ConfigServletWebServerApplicationContext` in all 57 refresh-time runs, and the `INFO` logger `.s.b.a.l.ConditionEvaluationReportLogger` in all 13 that have one | config.error.008, config.oracle.003 |

Review finding 2 is folded in at the same time: Jackson's ` at [Source:
REDACTED …; line: 1, column: 2]` line is a message line (led by a space), and
a stack frame is tab-led only, including `\t... N common frames omitted`
(config.args.007, .009, config.error.009, config.oracle.003). With the
record now HAPI's own, the two comparison rules that existed only because
the probe was not HAPI's context are dropped: the `WARN`/`INFO` logger field
is compared, and the bean-creation wrapper is compared in full. The
`Bindable@<hex>` mask stays: the hash differs between the probe and
application runs of config.collection.002 (`15f47664`, `19b89d4`).

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
(`include_bytes!` of `crates/frevo-config/packaged/`, section 11), then to the
fixed directory `/app/extra-classes/`, the
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
  environment under four names, in this order: its uniform form upper-cased
  (`HAPI_FHIR_DEFAULTPAGESIZE`), its legacy form upper-cased
  (`HAPI_FHIR_DEFAULT_PAGE_SIZE`, dashes to underscores), then the same two
  lower-cased (`hapi_fhir_defaultpagesize`, `hapi_fhir_default_page_size`)
  (Spring Boot 3.5.14's `SystemEnvironmentPropertyMapper.map`, read from the
  3.5.14 jar). So on a case-sensitive environment an all-lower-case name
  binds and a mixed-case one does not (config.env.001-.011, config.env.004).
- The dotted mapper applies to every variable name, including one that
  starts with dashes: the element `--spring` has the uniform form `spring`
  (dashes are not letters or digits), so a variable named
  `--spring.config.location` is `spring.config.location` (config.env.014).

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
standard error except the JVM-option notices and logback's own status lines.
Those status lines (`HH:MM:SS,mmm |-<LEVEL> in …`, among them `Missing
watchable .xml or .properties files.`) appear because the packaged
`logback.xml` sets `scan="true"` while it is read from inside a jar: on
standard output on every HAPI run, and on standard error on every run except
those that fail before the application context exists (23 of the record's
419 runs have none there). They say nothing about the configuration. Frevo
writes none of them; the oracle strips and counts them (section 7), and
`DEV-0033`'s scope covers them. Two forms:

- **Failure analysis** (config.error.002): one `ERROR` line from
  `o.s.b.d.LoggingFailureAnalysisReporter` with an empty message, then the
  `APPLICATION FAILED TO START` block. Used for property binding failures and
  for missing config data resources and locations. For an enum-typed
  property the action lists the valid constants, sorted by name.
- **Run failed** (config.error.007): one `ERROR` line from
  `o.s.boot.SpringApplication` with `Application run failed`, a blank line,
  and the exception chain (`<class>: <message>`, `Caused by: …`). Used for
  everything else, including the missing version (config.gate.005:
  `BeanDefinitionStoreException` naming `FhirServerConfigDstu2.class`, then
  `Caused by:` HAPI's `NullPointerException`) and the strict placeholders of
  config.placeholder.011 (three exception lines). A SnakeYAML error's chain is
  its message lines alone: no class prefix, no `Caused by:`, echoed source
  lines indented four spaces plus their own indentation, and a trailing blank
  line (config.yaml.003). Invalid JSON is Jackson's `JsonParseException`,
  because the image ships Jackson 2, with Jackson's ` at [Source: REDACTED …;
  line: 1, column: 2]` location line as the cause's second message line
  (config.args.007).

Which log lines Frevo writes, exactly, per failure kind (config.error.008):

| Failure kind | Lines Frevo writes, in order |
|---|---|
| environment preparation: invalid JSON, `--=x`, profile names and profile keys, missing or unloadable locations and imports, wildcards, YAML errors | the `ERROR` line with no process id and no application name, then the block or chain |
| `AppProperties` binding | `WARN` (`BeanDefinitionStoreException … common/PartitionModeConfigurer.class`), `ERROR` (`LoggingFailureAnalysisReporter`), the analysis block |
| no `hapi.fhir.fhir_version` | `WARN` (`… common/FhirServerConfigDstu2.class`), `ERROR` (`SpringApplication`), two exception lines |
| `ElasticsearchProperties` binding | `WARN` (`… elastic/ElasticsearchBootSvcImpl.class`), `INFO`, `ERROR` (`LoggingFailureAnalysisReporter`), the analysis block |
| strict placeholder in `app_content_path`, `custom_content_path`, `spring.ai.mcp.server.enabled` | `WARN` (`… web/WebAppFilesConfigurer.class`, `… web/CustomContentFilesConfigurer.class`, `… mcp/McpServerConfig.class`), `INFO`, `ERROR` (`SpringApplication`), three exception lines |
| `CqlProperties`, `CqlData`, `CqlTerminologyProperties` binding | `WARN` (`ConfigurationPropertiesBindException: Error creating bean with name 'cqlProperties'` / `'cqlData'` / `'cqlTerminologyProperties'` …), `INFO`, `ERROR` (`LoggingFailureAnalysisReporter`), the analysis block |

Log lines use the console pattern; the process id and application name are
absent when the failure precedes the application context, and the
application name is absent whenever the effective configuration does not set
`spring.application.name` (the gate.005 family). The `WARN` logger is
`ConfigServletWebServerApplicationContext` and the `INFO` logger
`.s.b.a.l.ConditionEvaluationReportLogger`, fixed strings, as HAPI's
`Application` prints them; Frevo has no servlet context of its own, and
writing HAPI's names is part of `DEV-0033`'s chosen behaviour. The class-file
URLs are HAPI's (`jar:file:/app/main.war!/WEB-INF/classes!/…`), fixed
strings per failure kind. Java stack frames (tab-led only: `\tat …`, `\t... N
more`, `\t... N common frames omitted`) and every non-config log line (the
startup-info lines `Starting Application …` and `No active profile set …`,
Tomcat, the servlet context, HAPI components and libraries, the shutdown
lines between `WARN` and `INFO`) are not reproduced; Frevo writes none of
them (`DEV-0033`, approved by the owner on 2026-11-22, scope extended the
same day, and extended again to logback's status lines). A table maps each
failure kind to its exception class names and messages, generated from the
oracle's record. A binding failure of `CrProperties`, `CareGapsProperties`,
`CqlCompilerProperties` or `CdsHooksProperties`, and the order of several
kinds in one configuration, are not yet recorded (tasks.md task 2).

## 7. The `config-roundtrip` oracle

New, in `harness/config-roundtrip/` (oracle-builder), shaped like
`harness/hapi-fhirpath-results/`:

- **HAPI side:** the probe class, a dumper that maps each getter to the
  canonical path of config.dump.002, the starter's condition classes
  evaluated against the started `Environment`, and a runner. They are
  compiled with `javac` in a Docker build stage
  (`eclipse-temurin:21-jdk@sha256:8ad118b1…`) against the image's own
  `WEB-INF/lib` and `WEB-INF/classes`, not by Maven, so they compile against
  exactly the jars the image runs rather than Maven Central copies; there is
  no `pom.xml`. The classes are added to the unchanged image at
  `/app/extra-classes/` and started by the image's own launcher
  (`PropertiesLauncher` with the ENTRYPOINT's `loader.path`, `loader.main`
  from `/app/loader.properties`, working directory `/app`), so the class path
  and its order, `/app/extra-classes/`, the packaged `logback.xml` and the
  exact jars HAPI 8.12.0 runs are all HAPI's, on Linux. It also records
  `System.getProperties()` for section 3.6, asserts that no `WEB-INF/lib`
  jar carries an application file, and refuses to record unless
  `spring-boot`, `spring-core` and `snakeyaml` are 3.5.14, 6.2.19 and 2.4.
- **Two runners** (lead ruling of 2026-11-22, after review finding 1). The
  probe evaluates each starter condition on the starter's own annotation
  metadata, which is enough for documents, reads and gates but is not HAPI's
  component scan. So every case the probe fails is run again in a fresh
  container of the unchanged image, whose launcher then starts the war's own
  `Start-Class`, `ca.uhn.fhir.jpa.starter.Application`; that run is the
  case's record (`"runner": "application"`), the probe's run is kept under
  `"probe"`, and a case where the real `Application` starts the server
  instead would be listed in `runner_disagreements` (none is). A case the
  probe does not fail is recorded from the probe (config.oracle.001). The
  probe's gate site names (for example `MdmConfig` for `mdm_enabled`) are
  therefore no longer quoted anywhere: config.placeholder.011 quotes the real
  `Application`.
- **Cases:** one directory per case: `files/` (copied into an empty working
  directory), `root/` (copied to `/`, for absolute paths), `env` (one
  `NAME=value` per line, nothing inherited), `args`, and `covers:` tags. The
  three whole-surface cases of config.key.005-.007 are generated from
  `keys.md` so that a key added there cannot be missed. A scenario that
  replaces the default locations without a version has a case exactly as
  stated (it fails as config.gate.005 does) and a `-v` case with
  `--hapi.fhir.fhir_version=R4` added (config.source.007, .009, .010,
  config.key.001); config.source.011 and .013 have only the version-set
  case, and the other location-replacing cases set the version in their
  arguments or their named file (the notation comment of the delta lists
  them).
- **Recording:** the HAPI side's documents are recorded in
  `harness/config-roundtrip/hapi-8.12.0-results.json` with the image digest,
  so CI compares Frevo against the record without Docker, and a scheduled job
  re-records to catch drift. This mirrors
  `oracles/fhirpath/hapi-8.12.0-results.json`.
- **Comparison:** member for member; sets sorted; nothing else normalised
  (config.oracle.004). A failing HAPI case passes only if Frevo also fails and
  the config-related lines of the two standard outputs are equal: the `ERROR`
  line and everything after it with stack frames (tab-led lines only)
  removed, and the `WARN`/`INFO` lines above it, logger field included;
  timestamps and process ids masked; every other HAPI line stripped
  (config.oracle.003). Two further normalisations, each because the HAPI side
  varies: logback status lines are removed from both HAPI streams and
  counted, and the hexadecimal identity hash in `Bindable@<hex>` is masked on
  both sides (config.collection.002). The record's `config_lines` still mask
  the `WARN`/`INFO` logger field as `<logger>` (`run.py` `mask`); comparing
  it, as config.oracle.003 now requires, is owed by the oracle-builder
  (tasks.md task 2).

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

`frevo-config effective` is a binary target of `frevo-config`, named
`frevo-config` (owner decision of 2026-11-22; section 11).
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

## 11. Test contracts

The oracle-builder's tests (task 3) fix three points of the implementation's
shape. All three were accepted on 2026-11-22.

1. **`FREVO_CONFIG_TEST_ROOT`, a test seam.** A test cannot create the
   absolute paths HAPI's cases use (`/app/extra-classes/`, `/app/config/…`,
   `/configs/…`). The tests therefore run `frevo-config effective` with the
   working directory `<tmp>/root/app/`, mirroring the image's `WORKDIR /app`,
   and set `FREVO_CONFIG_TEST_ROOT=<tmp>/root`. When the variable is set,
   every absolute path Frevo resolves — `file:` locations and imports, plain
   absolute paths in `SPRING_CONFIG_IMPORT`, and the fixed
   `/app/extra-classes/` of `classpath:` — resolves beneath it, so
   `/configs/x.yaml` is `<tmp>/root/configs/x.yaml`. It is **test-only and
   not a HAPI setting**: it is not a configuration property, no source can
   set it as one, it never appears in the effective-configuration document or
   in a failure report (paths in `Origin:` lines and `does not exist` reports
   are written as if the root were `/`), and production deployments never set
   it. It is not a deviation, because it changes nothing HAPI's
   configuration can express.
2. **Embedded packaged files.** The three packaged files are embedded from
   `crates/frevo-config/packaged/` (`application.yaml`,
   `application-cds.yaml`, `application-elastic.yaml`), each byte-identical
   to the copy in `harness/config-roundtrip/packaged/`, which the harness
   extracted from the pinned image and checked against
   `hapi-fhir-jpaserver-starter@377e6eba` after filtering
   (config.source.001).
3. **Binary target name.** The binary target is named `frevo-config`; the
   tests find it through `CARGO_BIN_EXE_frevo-config`.
