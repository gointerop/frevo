# config spec delta

## Purpose

Reads the HAPI FHIR JPA Server Starter 8.12.0 configuration (`application.yaml`,
profile files, environment variables, command-line arguments) with the same
`hapi.fhir.*` key names, defaults, binding rules and failure behaviour as the
starter running on Spring Boot 3.5.14, and exposes the result as one effective
configuration.

## ADDED Requirements

<!--
Notation used in every scenario below.

"The effective configuration reports P = V" means: `frevo-config effective`
(config.dump.001) prints a document whose `properties` member holds value V
(JSON) at path P. P is written as the starter's bean prefix followed by the
property path in canonical kebab-case, with map keys verbatim, for example
`hapi.fhir.default-page-size`, `hapi.fhir.cors.allowed-origin`,
`hapi.fhir.tester.home.name`, `hapi.fhir.cr.cql.namespaces[hl7.fhir.uv.cql]`.

"The read R is V" refers to the document's `reads` member (config.read.*), and
"gate G is on/off" to its `gates` member (config.gate.*).

"Starts" means `frevo-config effective` exits 0. "Fails to start" means it
exits 1, prints no document, and writes the failure report of config.error.*
to standard error.

Unless a scenario says otherwise, the working directory contains only the
files the scenario names, no `SPRING_*`, `HAPI_*` or `hapi.*` environment
variable is set, no argument is given, and the packaged files of
config.source.001 are present.

Every value quoted in a THEN clause was measured on Spring Boot 3.5.14 with the
starter's nine properties classes compiled from
`hapi-fhir-jpaserver-starter@377e6eba`; see design.md section 2.
-->

### Requirement: Effective configuration document

`frevo-config` SHALL provide a command, `frevo-config effective`, that loads the
configuration exactly as startup does and prints the result as one JSON
document, so that every scenario in this capability is observable without a
running server and can be compared with the HAPI side of the `config-roundtrip`
oracle.

#### Scenario: [config.dump.001] a valid configuration prints one document and exits 0

- **WHEN** `frevo-config effective` runs in an empty working directory
- **THEN** it exits `0`
- **AND** standard output is exactly one JSON object with the members
  `profiles`, `properties`, `reads` and `gates`, and no other member
- **AND** `profiles` is an object with `active` (array of strings, in
  activation order) and `default` (array of strings)

#### Scenario: [config.dump.002] properties are reported under the starter's nine bean prefixes

- **WHEN** the effective configuration is printed
- **THEN** `properties` has exactly these nine members, one per starter
  properties class: `hapi.fhir` (`AppProperties`), `hapi.fhir.cdshooks`
  (`CdsHooksProperties`), `hapi.fhir.cr` (`CrProperties`),
  `hapi.fhir.cr.caregaps` (`CareGapsProperties`), `hapi.fhir.cr.cql`
  (`CqlProperties`), `hapi.fhir.cr.cql.compiler` (`CqlCompilerProperties`),
  `hapi.fhir.cr.cql.runtime` (`CqlRuntimeProperties`),
  `hapi.fhir.cr.cql.terminology` (`CqlTerminologyProperties`) and
  `hapi.fhir.cr.cql.data` (`CqlData`)
- **AND** where two classes bind the same key (for example
  `hapi.fhir.cr.cql.compiler.validate-units`, bound by `CrProperties`,
  `CqlProperties` and `CqlCompilerProperties`), every class reports the same
  value

#### Scenario: [config.dump.003] property paths are canonical whatever spelling was written

- **WHEN** `./application.yaml` contains `hapi: { fhir: { defaultPageSize: 6 } }`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `6`
- **AND** no path `defaultPageSize` or `default_page_size` appears in the document

#### Scenario: [config.dump.004] values are typed as the starter's property types

- **WHEN** the effective configuration is printed with no external file
- **THEN** a `Boolean`/`boolean` property is a JSON boolean, an
  `Integer`/`Long`/`int`/`long` property is a JSON integer, a `String` is a
  JSON string, an enum is the constant's name as a string (for example
  `hapi.fhir.fhir-version` = `"R4"`), a `List` is an array in binding order,
  a `Set` is an array sorted by code point, a `Map` is an object whose keys are
  the map keys verbatim, and an absent value or object is `null`
- **AND** `hapi.fhir.max-page-size` = `2147483647` and
  `hapi.fhir.retain-cached-searches-mins` = `60`

#### Scenario: [config.dump.005] the e-mail password is redacted

- **WHEN** `hapi.fhir.subscription.email.password` is set to `s3cret`
- **THEN** the effective configuration reports
  `hapi.fhir.subscription.email.password` = `"******"`
- **AND** when it is set to an empty value it reports `""`

#### Scenario: [config.dump.006] a failure prints no document

- **WHEN** `./application.yaml` contains `hapi.fhir.default_page_size: abc`
- **THEN** `frevo-config effective` exits `1`
- **AND** standard output is empty
- **AND** standard error carries the report of config.error.002

### Requirement: Configuration sources and locations

`frevo-config` SHALL load configuration files from the same locations, with the
same file names, in the same order of precedence, and with the same handling of
missing and optional locations as Spring Boot 3.5.14 does for the starter,
treating the starter's packaged files as the `classpath:` location.

#### Scenario: [config.source.001] the starter's packaged files are built in

- **WHEN** the Frevo build is inspected
- **THEN** it carries the starter's `application.yaml`, `application-cds.yaml`
  and `application-elastic.yaml` from
  `hapi-fhir-jpaserver-starter@377e6eba:src/main/resources/`, after the
  starter's Maven resource filtering (delimiter `@`), so that
  `spring.application.name` reads `hapi-fhir-jpaserver-starter`
- **AND** each is byte-identical to the same path under `WEB-INF/classes/` in
  the pinned image `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`

#### Scenario: [config.source.002] with no external file the packaged application.yaml applies

- **WHEN** Frevo starts in an empty working directory
- **THEN** the effective configuration reports `hapi.fhir.openapi-enabled` =
  `true`, `hapi.fhir.narrative-enabled` = `false`,
  `hapi.fhir.fhir-version` = `"R4"`, `hapi.fhir.ig-runtime-upload-enabled` =
  `false`, `hapi.fhir.mdm-enabled` = `false`,
  `hapi.fhir.mdm-rules-json-location` = `"mdm-rules.json"`,
  `hapi.fhir.bulk-export-file-retention-period-hours` = `2`,
  `hapi.fhir.binary-storage-minimum-binary-size` = `4000`,
  `hapi.fhir.search-prefetch-thresholds` = `[13, 503, 2003, -1]`,
  `hapi.fhir.cors.allowed-origin` = `["*"]`, and
  `hapi.fhir.logical-urls` = the eight `http(s)://` patterns of the packaged
  file, sorted
- **AND** `hapi.fhir.tester` = `{"home": {"name": "Local Tester",
  "server-address": "http://localhost:8080/fhir",
  "refuse-to-fetch-third-party-urls": false, "fhir-version": "R4"}}`
- **AND** `hapi.fhir.cr.enabled` = `false`, `hapi.fhir.cdshooks.enabled` =
  `false`, `hapi.fhir.cdshooks.client-id-header-name` = `"client_id"`

#### Scenario: [config.source.003] default locations are searched in Spring Boot order

- **WHEN** `./application.yaml` sets `hapi.fhir.default_page_size: 1` and
  `hapi.fhir.max_page_size: 1`, `./config/application.yaml` sets both to `2`,
  and `./config/sub/application.yaml` sets only `default_page_size: 3`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` =
  `3` and `hapi.fhir.max-page-size` = `2`
- **AND** the order, lowest to highest, is the packaged files, `./`,
  `./config/`, then each immediate subdirectory of `./config/`

#### Scenario: [config.source.004] config subdirectories apply in name order

- **WHEN** `./config/a/application.yaml` sets `hapi.fhir.max_page_size: 21` and
  `./config/b/application.yaml` sets it to `22`
- **THEN** the effective configuration reports `hapi.fhir.max-page-size` = `22`

#### Scenario: [config.source.005] within one location .properties beats .yml beats .yaml

- **WHEN** `./application.properties` sets `hapi.fhir.default_page_size=4`,
  `./application.yml` sets it to `6` and `./application.yaml` sets it to `1`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `4`
- **AND** with `./application.properties` removed it reports `6`

#### Scenario: [config.source.006] absent default locations are not an error

- **WHEN** neither `./`, `./config/` nor any `./config/*/` holds an application file
- **THEN** Frevo starts and the packaged defaults of config.source.002 apply

#### Scenario: [config.source.007] spring.config.location replaces the default locations entirely

- **WHEN** Frevo is started with `--spring.config.location=file:./other/x.yaml`
  and `./other/x.yaml` sets `hapi.fhir.default_page_size: 5`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `5`
- **AND** the packaged `application.yaml` is not loaded: it reports
  `hapi.fhir.openapi-enabled` = `false`, `hapi.fhir.tester` = `null`,
  `hapi.fhir.cors` = `null`, `hapi.fhir.narrative-enabled` = `true` (the class
  defaults)
- **AND** `./application.yaml`, if present, is not loaded either

#### Scenario: [config.source.008] a missing mandatory location fails to start

- **WHEN** Frevo is started with `--spring.config.location=file:./nope.yaml` and
  the file does not exist
- **THEN** Frevo fails to start
- **AND** the report's description is `Config data resource 'file [nope.yaml]'
  via location 'file:./nope.yaml' does not exist`
- **AND** its action is `Check that the value 'file:./nope.yaml' is correct, or
  prefix it with 'optional:'`

#### Scenario: [config.source.009] an optional: location may be missing

- **WHEN** Frevo is started with `--spring.config.location=optional:file:./nope.yaml`
- **THEN** Frevo starts and every property has its class default
  (config.key.001)

#### Scenario: [config.source.010] a location ending in / is a directory

- **WHEN** Frevo is started with `--spring.config.location=file:./dir/` and
  `./dir/application.yaml` sets `hapi.fhir.default_page_size: 33`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `33`
- **AND** `application-<profile>.yaml` files in `./dir/` apply for active profiles

#### Scenario: [config.source.011] spring.config.name changes the file base name

- **WHEN** Frevo is started with `--spring.config.location=file:./dir/
  --spring.config.name=frevo` and `./dir/frevo.yaml` sets
  `hapi.fhir.max_page_size: 44` while `./dir/application.yaml` sets
  `default_page_size: 33`
- **THEN** the effective configuration reports `hapi.fhir.max-page-size` = `44`
  and `hapi.fhir.default-page-size` = `20`

#### Scenario: [config.source.012] spring.config.additional-location layers over the defaults

- **WHEN** Frevo is started with
  `--spring.config.additional-location=file:./other/x.yaml`, `./other/x.yaml`
  sets `hapi.fhir.default_page_size: 5` and `./application.yaml` sets
  `default_page_size: 1` and `max_page_size: 2`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` =
  `5`, `hapi.fhir.max-page-size` = `2` and `hapi.fhir.openapi-enabled` = `true`

#### Scenario: [config.source.013] in a location list later entries win

- **WHEN** Frevo is started with
  `--spring.config.location=file:./l/1.yaml,file:./l/2.yaml`, the files setting
  `hapi.fhir.default_page_size` to `31` and `32`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `32`

#### Scenario: [config.source.014] SPRING_CONFIG_IMPORT imports a plain path over the defaults

- **WHEN** the environment has `SPRING_CONFIG_IMPORT=/app/config/application-extra.yaml`
  (the form the starter's Helm chart sets, no `file:` prefix), that file sets
  `hapi.fhir.default_page_size: 5`, and `./application.yaml` sets it to `1`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `5`
- **AND** a relative path in `SPRING_CONFIG_IMPORT` resolves against the working directory

#### Scenario: [config.source.015] spring.config.import in a file resolves relative to that file and overrides it

- **WHEN** `./config/application.yaml` contains `spring.config.import:
  extra/x.yaml`, `hapi.fhir.default_page_size: 1` and `hapi.fhir.max_page_size:
  1`, and `./config/extra/x.yaml` sets `default_page_size: 2`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` =
  `2` and `hapi.fhir.max-page-size` = `1`

#### Scenario: [config.source.016] a missing mandatory import fails; an optional one does not

- **WHEN** `./config/application.yaml` contains
  `spring.config.import: optional:nope.yaml,missing.yaml` and neither file exists
- **THEN** Frevo fails to start
- **AND** the description names `missing.yaml` resolved against `./config/`
  (`Config data resource 'file [config/missing.yaml]' via location
  'missing.yaml' does not exist`) and does not name `nope.yaml`

#### Scenario: [config.source.017] classpath: resolves to the packaged files only

- **WHEN** Frevo is started with `--spring.config.location=classpath:/application-cds.yaml`
- **THEN** only the packaged `application-cds.yaml` is loaded and
  `hapi.fhir.cr.enabled` = `true`
- **AND** with `--spring.config.location=classpath:/another.application.yaml`
  Frevo fails to start with the `does not exist` report of config.source.008
  (`DEV-0031`: HAPI resolves any resource on the war's class path, including
  `/app/extra-classes/` in the distroless image)

#### Scenario: [config.source.018] file: URLs with an authority-less absolute path are accepted

- **WHEN** Frevo is started with
  `--spring.config.location=file:///configs/another.application.yaml` (the
  starter README's form) and that file sets `hapi.fhir.default_page_size: 9`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `9`

#### Scenario: [config.source.019] precedence across source kinds

- **WHEN** `./application.yaml` sets `hapi.fhir.default_page_size`,
  `max_page_size`, `expunge_thread_count` and `reindex_thread_count` to `1`;
  the environment has `HAPI_FHIR_DEFAULT_PAGE_SIZE=2`,
  `HAPI_FHIR_MAX_PAGE_SIZE=2`, `HAPI_FHIR_EXPUNGE_THREAD_COUNT=2` and
  `SPRING_APPLICATION_JSON={"hapi":{"fhir":{"default_page_size":3,"max_page_size":3}}}`;
  and Frevo is started with `--hapi.fhir.default_page_size=4
  --hapi.fhir.reindex-thread-count=4`
- **THEN** the effective configuration reports `default-page-size` = `4`,
  `max-page-size` = `3`, `expunge-thread-count` = `2` and
  `reindex-thread-count` = `4`
- **AND** so the order, highest first, is command-line arguments,
  `SPRING_APPLICATION_JSON`, Java system properties (config.source.023),
  environment variables, configuration files, class defaults

#### Scenario: [config.source.020] unsupported location kinds fail loudly

- **WHEN** `spring.config.import` names `configtree:./tree/` or a location with
  an `.xml` extension
- **THEN** Frevo fails to start with a report naming the location and stating
  that the location kind is not supported (`DEV-0031`: HAPI loads both)
- **AND** a location no loader recognises (for example `configserver:`, or a
  file name with an extension other than `.properties`, `.xml`, `.yml` and
  `.yaml`) fails to start in both, with `Unable to load config data from
  'configserver:'` caused by `Incorrect ConfigDataLocationResolver chosen or
  file extension is not known to any PropertySourceLoader`, and starts when
  prefixed `optional:`

#### Scenario: [config.source.021] an empty or comment-only file contributes nothing

- **WHEN** `./application.yaml` is empty or holds only comments
- **THEN** Frevo starts and the effective configuration equals config.source.002's

#### Scenario: [config.source.022] the active sources are reported

- **WHEN** the effective configuration is printed with `./application.yaml`
  present and `HAPI_FHIR_DEFAULT_PAGE_SIZE` set
- **THEN** the document's `profiles.active` is `[]` and its `profiles.default`
  is `["default"]`

#### Scenario: [config.source.023] -D tokens in JAVA_TOOL_OPTIONS and JDK_JAVA_OPTIONS are system properties

- **WHEN** `JAVA_TOOL_OPTIONS=-Xmx3g -Dhapi.fhir.default_page_size=6
  -XX:+UseG1GC`, `HAPI_FHIR_DEFAULT_PAGE_SIZE=2` and `./application.yaml`
  sets `default_page_size: 1`
- **THEN** `hapi.fhir.default-page-size` = `6`, because the JVM turns the
  `-D` token into a Java system property, which ranks above environment
  variables and below `SPRING_APPLICATION_JSON`
- **AND** the same holds for a `-D` token in `JDK_JAVA_OPTIONS`; when both
  variables define the same name, `JDK_JAVA_OPTIONS` wins (the launcher applies
  it after the VM has read `JAVA_TOOL_OPTIONS`), and within one variable the
  last definition wins; the oracle's recorded HAPI run decides each clause
- **AND** system-property names bind with relaxed matching like arguments
  (`-Dhapi.fhir.default-page-size=6` also works)

#### Scenario: [config.source.024] only -D tokens are read from the JVM option variables

- **WHEN** `JAVA_TOOL_OPTIONS=-Xmx3g -XX:+UseG1GC -Dhapi.fhir.server_address`
- **THEN** Frevo starts, `hapi.fhir.server-address` = `""` (a `-D` token with
  no `=` defines the empty string), and every other token changes nothing
- **AND** tokens are separated by whitespace; a token that is not `-D` is
  ignored, whatever it is

### Requirement: YAML documents

`frevo-config` SHALL read YAML with the document structure and the YAML 1.1
scalar resolution that SnakeYAML 2.4 applies under Spring Boot 3.5.14's
`OriginTrackedYamlLoader` (timestamps not resolved, duplicate keys rejected),
and SHALL flatten it into properties the way Spring's `YamlProcessor` does.

#### Scenario: [config.yaml.001] nested mappings and dotted keys are equivalent

- **WHEN** one file holds `hapi.fhir.default_page_size: 3` and another run holds
  `hapi: { fhir: { default_page_size: 3 } }` or `hapi.fhir: { default_page_size: 3 }`
- **THEN** every run reports `hapi.fhir.default-page-size` = `3`

#### Scenario: [config.yaml.002] multiple documents in one file apply in order

- **WHEN** `./application.yaml` holds two documents separated by `---`, the first
  setting `hapi.fhir.default_page_size: 1` and the second `default_page_size: 2`
  with no `spring.config.activate.*` key
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` = `2`

#### Scenario: [config.yaml.003] a duplicate key in one mapping fails to start

- **WHEN** one mapping holds `default_page_size: 3` and, on the next line,
  `default_page_size: 4`
- **THEN** Frevo fails to start
- **AND** the report contains `found duplicate key default_page_size` with the
  line and column of both occurrences (`line 3, column 5` and `line 4, column 5`)

#### Scenario: [config.yaml.004] YAML 1.1 booleans

- **WHEN** a plain scalar is one of `true True TRUE false False FALSE yes Yes
  YES no No NO on On ON off Off OFF`
- **THEN** it is a boolean (and binds to a `String` property as `"true"` or
  `"false"`, config.coerce.013)
- **AND** `y`, `Y`, `n`, `tRUE` are strings

#### Scenario: [config.yaml.005] YAML 1.1 integers

- **WHEN** plain scalars `0x1F`, `010`, `0777`, `0b101`, `1_000` and `190:20:30`
  bind to a `String` list
- **THEN** they read `"31"`, `"8"`, `"511"`, `"5"`, `"1000"` and `"685230"`
- **AND** `09` and `0o17` are strings, `"09"` and `"0o17"`

#### Scenario: [config.yaml.006] YAML 1.1 floats render as Java doubles

- **WHEN** plain scalars `1.10`, `1.0`, `.5`, `+.5`, `1_0.5`, `1e3`, `1.0e+3`,
  `-.inf` and `.NaN` bind to a `String` list
- **THEN** they read `"1.1"`, `"1.0"`, `"0.5"`, `"0.5"`, `"10.5"`, `"1000.0"`,
  `"1000.0"`, `"-Infinity"` and `"NaN"`, which is Java's `Double.toString`
  (so a value of 1e7 or more, or below 1e-3, reads in `E` notation, for example
  `1e7` reads `"1.0E7"`)

#### Scenario: [config.yaml.007] timestamps, quoted scalars and !!str stay text

- **WHEN** plain scalars `2026-01-02` and `2026-01-02T10:00:00Z`, the quoted
  scalars `"010"` and `'yes'`, and `!!str 010` bind to `String` properties
- **THEN** they read `"2026-01-02"`, `"2026-01-02T10:00:00Z"`, `"010"`,
  `"yes"` and `"010"`

#### Scenario: [config.yaml.008] null, ~ and an absent value are the empty string

- **WHEN** a key is written as `key:`, `key: ~`, `key: null`, `key: Null` or
  `key: NULL`
- **THEN** its value is the empty string, with the binding consequences of
  config.coerce.005, .011, .012, .014, .020 and .025

#### Scenario: [config.yaml.009] anchors, aliases and merge keys

- **WHEN** `./application.yaml` holds `base: &b { default_page_size: 5 }` and
  `hapi: { fhir: { <<: *b, max_page_size: 9 } }`
- **THEN** the effective configuration reports `hapi.fhir.default-page-size` =
  `5` and `hapi.fhir.max-page-size` = `9`

#### Scenario: [config.yaml.010] an empty sequence is a value, an empty mapping is nothing

- **WHEN** a file holds `hapi.fhir.cors.allowed_origin: []` and
  `hapi.fhir.partitioning: {}`
- **THEN** `hapi.fhir.cors.allowed-origin` = `[]` (config.coerce.025) and
  `hapi.fhir.partitioning` = `null` (config.object.001)

#### Scenario: [config.yaml.011] a sequence flattens to indexed elements

- **WHEN** a file holds `supported_resource_types: [Patient, Observation]` under
  `hapi.fhir`
- **THEN** the file contributes `hapi.fhir.supported_resource_types[0]` and
  `[1]`, and the effective configuration reports
  `hapi.fhir.supported-resource-types` = `["Patient", "Observation"]`

#### Scenario: [config.yaml.012] non-string mapping keys take their resolved text

- **WHEN** `hapi.fhir.implementationguides` holds an entry whose key is the
  plain scalar `no`
- **THEN** the implementation guide's map key is `"false"`

#### Scenario: [config.yaml.013] bracketed keys keep their dots

- **WHEN** `hapi.fhir.implementationguides` holds the key `"[a.b]"` with
  `name: e`
- **THEN** `hapi.fhir.implementation-guides[a.b].name` = `"e"`

#### Scenario: [config.yaml.014] block scalars fold as YAML specifies

- **WHEN** `hapi.fhir.logger.format` is written as the `>-` folded block of the
  starter's commented example (four lines)
- **THEN** `hapi.fhir.logger.format` is the four lines joined by single spaces
  with no trailing newline

#### Scenario: [config.yaml.015] commented examples in the packaged file have no effect

- **WHEN** the packaged `application.yaml` is loaded
- **THEN** none of its commented keys (for example `# max_page_size: 200`,
  `# server_address: http://hapi.fhir.org/baseR4`) appears in the effective
  configuration with the commented value

#### Scenario: [config.yaml.016] a YAML syntax error fails to start with its position

- **WHEN** `./application.yaml` holds `supported_resource_types: [${nope3}]`
- **THEN** Frevo fails to start
- **AND** the report contains `while parsing a flow sequence` with `line 5,
  column 31` and `expected ',' or ']', but got {` with `line 5, column 33`
  (for the five-line fixture `config.yaml.016.yaml`)

### Requirement: Properties files

`frevo-config` SHALL read `.properties` configuration files with Spring Boot
3.5.14's `OriginTrackedPropertiesLoader` syntax.

#### Scenario: [config.properties.001] the three separators

- **WHEN** `./application.properties` holds
  `hapi.fhir.server_address = http://a`, `hapi.fhir.mdm_rules_json_location: x`
  and `hapi.fhir.custom_content_path abc`
- **THEN** the three properties read `"http://a"`, `"x"` and `"abc"`

#### Scenario: [config.properties.002] comments

- **WHEN** lines start with `#` or `!` (other than the `#---` separator)
- **THEN** they contribute nothing

#### Scenario: [config.properties.003] trailing whitespace is kept, leading whitespace is not

- **WHEN** the line is `hapi.fhir.server_address = http://a   `
- **THEN** `hapi.fhir.server-address` = `"http://a   "`

#### Scenario: [config.properties.004] unicode escapes and line continuation

- **WHEN** the line is `hapi.fhir.custom_content_path Abc`, and another
  property is split over two lines with a trailing `\` and an indented
  continuation
- **THEN** `hapi.fhir.custom-content-path` = `"Abc"`
- **AND** the continued value is the two parts joined with the continuation
  line's leading whitespace removed

#### Scenario: [config.properties.005] #--- separates documents

- **WHEN** the file holds `hapi.fhir.default_page_size=5`, a `#---` line,
  `spring.config.activate.on-profile=p` and `hapi.fhir.default_page_size=6`
- **THEN** with no profile active `default-page-size` = `5`, and with profile
  `p` active it is `6`

#### Scenario: [config.properties.006] every value is text

- **WHEN** `./application.properties` holds `hapi.fhir.graphql_enabled=1`
- **THEN** `hapi.fhir.graphql-enabled` = `true` (the string `"1"`,
  config.coerce.002), unlike the YAML integer `1` of config.coerce.004

#### Scenario: [config.properties.007] an empty value is the empty string

- **WHEN** the line is `hapi.fhir.app_content_path=`
- **THEN** `hapi.fhir.app-content-path` = `""`

### Requirement: Profiles

`frevo-config` SHALL activate profiles and load profile-specific files and
documents as Spring Boot 3.5.14 does, including the starter's packaged `cds` and
`elastic` profiles.

#### Scenario: [config.profile.001] the packaged cds profile

- **WHEN** `SPRING_PROFILES_ACTIVE=cds`
- **THEN** `profiles.active` = `["cds"]`
- **AND** `hapi.fhir.cr.enabled` = `true`, `hapi.fhir.cdshooks.enabled` =
  `true`, `hapi.fhir.allow-external-references` = `true`,
  `hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode` = `"REQUIRE"`,
  `hapi.fhir.cr.cql.terminology.code-lookup-mode` = `"USE_CODESYSTEM_URL"`,
  `hapi.fhir.cr.cql.namespaces` = `{"hl7.fhir.uv.cql":
  "http://hl7.org/fhir/uv/cql", "hl7.fhir.us.cql": "http://hl7.org/fhir/us/cql"}`
- **AND** `hapi.fhir.implementation-guides` has the entries `cql`
  (`hl7.fhir.uv.cql` `2.0.0`) and `uscql` (`hl7.fhir.us.cql` `1.0.0`), each
  with `install-mode` = `"STORE_AND_INSTALL"` and the eight
  `install-resource-types` of the packaged file
- **AND** keys the profile file does not set keep their packaged
  `application.yaml` value, so `hapi.fhir.tester.home.name` = `"Local Tester"`

#### Scenario: [config.profile.002] the packaged elastic profile

- **WHEN** `SPRING_PROFILES_ACTIVE=elastic`
- **THEN** `hapi.fhir.advanced-lucene-indexing` = `true` and
  `hapi.fhir.search-index-full-text-enabled` = `true`
- **AND** gate `elasticsearch` is on (config.gate.020)

#### Scenario: [config.profile.003] external base files beat packaged profile files

- **WHEN** `SPRING_PROFILES_ACTIVE=cds` and `./application.yaml` sets
  `hapi.fhir.cdshooks.enabled: false`
- **THEN** `hapi.fhir.cdshooks.enabled` = `false`
- **AND** the order, lowest first, is packaged base, packaged profile, external
  base, external profile

#### Scenario: [config.profile.004] the last active profile wins

- **WHEN** `./application-a.yaml` sets `hapi.fhir.default_page_size: 11` and
  `./application-b.yaml` sets it to `12`
- **THEN** with `--spring.profiles.active=a,b` it is `12`, and with
  `--spring.profiles.active=b,a` it is `11`

#### Scenario: [config.profile.005] a profile-specific file beats a profile document in a base file

- **WHEN** `./application.yaml` has a document with
  `spring.config.activate.on-profile: a` setting `default_page_size: 2`, and
  `./application-b.yaml` sets it to `10`
- **THEN** with profiles `a,b` and with `b,a` it is `10`

#### Scenario: [config.profile.006] on-profile documents, negation and expressions

- **WHEN** `./application.yaml` holds a plain document setting
  `default_page_size: 1`, a document `on-profile: a` setting
  `default_page_size: 2` and `max_page_size: 2`, a document `on-profile: "!a"`
  setting `max_page_size: 3`, and a document `on-profile: a & b` setting
  `server_address: a-and-b`
- **THEN** with no profile: `default-page-size` = `1`, `max-page-size` = `3`,
  `server-address` = `null`
- **AND** with profile `a`: `2`, `2`, `null`
- **AND** with profiles `a,b`: `max-page-size` = `2` and `server-address` = `"a-and-b"`

#### Scenario: [config.profile.007] the default profile

- **WHEN** no profile is active and `./application-default.yaml` sets
  `hapi.fhir.server_address: default-profile`
- **THEN** `hapi.fhir.server-address` = `"default-profile"` and
  `profiles.active` = `[]`
- **AND** with any profile active `application-default.yaml` is not loaded

#### Scenario: [config.profile.008] spring.profiles.default renames the default profile

- **WHEN** Frevo is started with `--spring.profiles.default=dd` and
  `./application-dd.yaml` sets `hapi.fhir.server_address: dflt`
- **THEN** `hapi.fhir.server-address` = `"dflt"` and `profiles.default` = `["dd"]`

#### Scenario: [config.profile.009] spring.profiles.include adds profiles ahead of the active ones

- **WHEN** `./application.yaml` holds `spring.profiles.include: b` and
  `SPRING_PROFILES_ACTIVE=cds`
- **THEN** `profiles.active` = `["b", "cds"]` and `./application-b.yaml` applies

#### Scenario: [config.profile.010] spring.profiles.active set in a file activates profiles

- **WHEN** `./application.yaml` holds `spring.profiles.active: cds`
- **THEN** `profiles.active` = `["cds"]` and the packaged `application-cds.yaml` applies

#### Scenario: [config.profile.011] profile groups expand in place

- **WHEN** `./application.yaml` holds `spring.profiles.group.g: b,a`, the profile
  files of config.profile.004 exist, and `--spring.profiles.active=g`
- **THEN** `profiles.active` = `["g", "b", "a"]` and `default-page-size` = `11`

#### Scenario: [config.profile.012] spring.profiles.active in a profile-specific file fails to start

- **WHEN** `SPRING_PROFILES_ACTIVE=q` and `./application-q.yaml` holds
  `spring.profiles.active: x`
- **THEN** Frevo fails to start with `Property 'spring.profiles.active'
  imported from location 'file [application-q.yaml]' is invalid in a profile
  specific resource`

#### Scenario: [config.profile.013] spring.profiles.active in an on-profile document is ignored

- **WHEN** `./application.yaml` has a document `on-profile: q` holding
  `spring.profiles.active: r`, and `SPRING_PROFILES_ACTIVE=q`
- **THEN** Frevo starts and `profiles.active` = `["q"]`

#### Scenario: [config.profile.014] on-cloud-platform documents never activate

- **WHEN** `./application.yaml` has a document with
  `spring.config.activate.on-cloud-platform: kubernetes` setting
  `hapi.fhir.default_page_size: 7`, and the environment has
  `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT` set
- **THEN** Frevo starts and `default-page-size` = `20`: the document never
  applies, on any platform (`DEV-0031`: HAPI applies it when it detects
  Kubernetes)

### Requirement: Environment variables

`frevo-config` SHALL map environment variables onto configuration properties
with Spring Boot 3.5.14's `SystemEnvironmentPropertyMapper` and
`DefaultPropertyMapper` on a case-sensitive (Linux) environment.

#### Scenario: [config.env.001] the upper-case underscore form

- **WHEN** `HAPI_FHIR_DEFAULT_PAGE_SIZE=11`
- **THEN** `hapi.fhir.default-page-size` = `11`

#### Scenario: [config.env.002] the upper-case form with the property's separators removed

- **WHEN** `HAPI_FHIR_DEFAULTPAGESIZE=12`
- **THEN** `hapi.fhir.default-page-size` = `12`

#### Scenario: [config.env.003] dotted variable names bind with relaxed matching

- **WHEN** one of `hapi.fhir.default_page_size=13`,
  `hapi.fhir.default-page-size=14` or `hapi.fhir.defaultPageSize=17` is set
  (the starter README's `-e hapi.fhir.default_encoding=xml` form and the Helm
  chart's `spring.jpa.properties.hibernate.dialect` form)
- **THEN** `hapi.fhir.default-page-size` is `13`, `14` or `17` respectively

#### Scenario: [config.env.004] forms that do not bind

- **WHEN** `HAPI_FHIR__DEFAULT_PAGE_SIZE=18` is set
- **THEN** `hapi.fhir.default-page-size` = `20`
- **AND** on Linux, where variable names are case-sensitive and where HAPI
  runs, `hapi_fhir_default_page_size=15` and `Hapi_Fhir_Default_Page_Size=16`
  do not bind either; the oracle's Linux run confirms this clause (on Windows,
  where names are case-insensitive, the lower-case form binds, in HAPI and in
  Frevo alike)

#### Scenario: [config.env.005] nested properties and camelCase upstream names

- **WHEN** `HAPI_FHIR_CORS_ALLOW_CREDENTIALS=true`
- **THEN** `hapi.fhir.cors.allow-credentials` = `true`

#### Scenario: [config.env.006] list elements by index

- **WHEN** `HAPI_FHIR_SUPPORTEDRESOURCETYPES_0=Patient` and
  `HAPI_FHIR_SUPPORTEDRESOURCETYPES_1=Obs`
- **THEN** `hapi.fhir.supported-resource-types` = `["Patient", "Obs"]`
- **AND** `HAPI_FHIR_SUPPORTED_RESOURCE_TYPES_0=Patient` alone gives `["Patient"]`
- **AND** `HAPI_FHIR_SUPPORTED_RESOURCE_TYPES_0_=Patient` binds nothing

#### Scenario: [config.env.007] an indexed variable replaces the whole collection

- **WHEN** `HAPI_FHIR_LOGICAL_URLS_0=x`
- **THEN** `hapi.fhir.logical-urls` = `["x"]`, discarding the packaged eight

#### Scenario: [config.env.008] comma-separated values

- **WHEN** `HAPI_FHIR_SEARCH_PREFETCH_THRESHOLDS=1,2`
- **THEN** `hapi.fhir.search-prefetch-thresholds` = `[1, 2]`

#### Scenario: [config.env.009] map entries through the upper-case form are lower-cased

- **WHEN** `HAPI_FHIR_IMPLEMENTATIONGUIDES_SWISS_NAME=swiss.x` and
  `HAPI_FHIR_CR_CQL_NAMESPACES_FOO=http://foo`
- **THEN** `hapi.fhir.implementation-guides.swiss.name` = `"swiss.x"` and
  `hapi.fhir.cr.cql.namespaces.foo` = `"http://foo"`
- **AND** `hapi.fhir.implementationguides.someIg.name=com.org.something` (the
  README's dotted form) creates the entry `someIg` with its case kept

#### Scenario: [config.env.010] map entry fields merge with lower sources

- **WHEN** `HAPI_FHIR_TESTER_HOME_NAME=EnvTester`
- **THEN** `hapi.fhir.tester.home.name` = `"EnvTester"` and
  `hapi.fhir.tester.home.server-address` = `"http://localhost:8080/fhir"`
  (from the packaged file)

#### Scenario: [config.env.011] a map key with digits or underscores cannot be addressed in upper-case form

- **WHEN** `HAPI_FHIR_IMPLEMENTATIONGUIDES_IPS_1_0_0_NAME=ips.x` is the only
  implementation-guide setting
- **THEN** `hapi.fhir.implementation-guides` = `null`

#### Scenario: [config.env.012] variable values are text

- **WHEN** `HAPI_FHIR_GRAPHQL_ENABLED=1`
- **THEN** `hapi.fhir.graphql-enabled` = `true`

#### Scenario: [config.env.013] the oracle's own environment binds

- **WHEN** Frevo is given exactly the `hapi` service environment of
  `harness/docker-compose.yml`
- **THEN** every `HAPI_FHIR_*` variable there that names a bound property takes
  effect, and `HAPI_FHIR_ALLOW_PLACEHOLDER_REFERENCES` (no such property)
  changes nothing (config.unknown.001)

#### Scenario: [config.env.014] a variable named like an argument is not an argument

- **WHEN** the environment has a variable whose name is
  `--spring.config.location` and whose value is `file:///configs/x.yaml` (the
  starter README's `docker run -e "--spring.config.location=…"` form)
- **THEN** Frevo does exactly what the oracle's recorded HAPI run does for the
  same case; by Spring's two name mappers that is to ignore it, so the default
  locations apply and `/configs/x.yaml` is not loaded

### Requirement: Command-line arguments and SPRING_APPLICATION_JSON

`frevo-config` SHALL accept `--name=value` command-line arguments and a
`SPRING_APPLICATION_JSON` environment variable with Spring Boot 3.5.14's
semantics.

#### Scenario: [config.args.001] --name=value arguments override every other source

- **WHEN** Frevo is started with `--hapi.fhir.default_page_size=4`
- **THEN** `hapi.fhir.default-page-size` = `4` whatever the files and
  environment say (config.source.019)

#### Scenario: [config.args.002] argument names are relaxed

- **WHEN** Frevo is started with `--hapi.fhir.reindex-thread-count=4`
- **THEN** `hapi.fhir.reindex-thread-count` = `4`

#### Scenario: [config.args.003] other arguments are ignored

- **WHEN** Frevo is started with the extra arguments `notanoption` and `-x`
- **THEN** it starts and they change nothing

#### Scenario: [config.args.004] an argument with no value is the empty string

- **WHEN** Frevo is started with `--hapi.fhir.server_address`
- **THEN** `hapi.fhir.server-address` = `""`

#### Scenario: [config.args.005] a repeated argument joins its values with commas

- **WHEN** Frevo is started with `--hapi.fhir.supported_resource_types=A
  --hapi.fhir.supported_resource_types=B`
- **THEN** `hapi.fhir.supported-resource-types` = `["A", "B"]`

#### Scenario: [config.args.006] SPRING_APPLICATION_JSON is flattened like YAML

- **WHEN** `SPRING_APPLICATION_JSON={"hapi":{"fhir":{"server_address":"json"}}}`
- **THEN** `hapi.fhir.server-address` = `"json"`
- **AND** a JSON number or boolean binds as the YAML integer or boolean would

#### Scenario: [config.args.007] invalid SPRING_APPLICATION_JSON fails to start

- **WHEN** `SPRING_APPLICATION_JSON={bad`
- **THEN** Frevo fails to start with a report containing `Cannot parse JSON`

### Requirement: Placeholders

`frevo-config` SHALL resolve `${...}` placeholders in configuration values as
Spring Boot 3.5.14 does: leniently when binding a properties class, strictly
when the starter reads a key by name.

#### Scenario: [config.placeholder.001] a placeholder with a default

- **WHEN** `hapi.fhir.server_address: ${MY_BASE:http://fallback}/fhir`
- **THEN** with `MY_BASE` unset `server-address` = `"http://fallback/fhir"`, and
  with `MY_BASE=http://env` it is `"http://env/fhir"`

#### Scenario: [config.placeholder.002] nested defaults

- **WHEN** `hapi.fhir.mdm_rules_json_location: ${MDMX:${MDMY:nested}}`
- **THEN** it is `"nested"`, and `"y"` with `MDMY=y`

#### Scenario: [config.placeholder.003] a placeholder names another property by any spelling

- **WHEN** `hapi.fhir.custom_content_path: ${hapi.fhir.mdm-rules-json-location}`
  and `mdm_rules_json_location` is written in snake case with the value of
  config.placeholder.002
- **THEN** `hapi.fhir.custom-content-path` = `"nested"` (the referenced value
  after its own resolution)

#### Scenario: [config.placeholder.004] an unresolvable placeholder in a bound string stays literal

- **WHEN** `hapi.fhir.supported_resource_types: ["${nope3}"]`
- **THEN** `hapi.fhir.supported-resource-types` = `["${nope3}"]`

#### Scenario: [config.placeholder.005] an unresolvable placeholder in a bound number fails conversion

- **WHEN** `hapi.fhir.default_page_size: ${NOPE}`
- **THEN** Frevo fails to start with `Property: hapi.fhir.default-page-size`,
  `Value: "${NOPE}"` and `Reason: failed to convert java.lang.String to
  java.lang.Integer (caused by java.lang.NumberFormatException: For input
  string: "${NOPE}")`

#### Scenario: [config.placeholder.006] an escaped placeholder is literal

- **WHEN** `hapi.fhir.logger.name: \${escaped}`
- **THEN** `hapi.fhir.logger.name` = `"${escaped}"`

#### Scenario: [config.placeholder.007] a circular reference fails to start

- **WHEN** `hapi.fhir.server_address: ${hapi.fhir.server_address}`
- **THEN** Frevo fails to start with `Circular placeholder reference
  'hapi.fhir.server_address' in value "${hapi.fhir.server_address}"`

#### Scenario: [config.placeholder.008] placeholders resolve before comma splitting

- **WHEN** `hapi.fhir.cors.allowed_origin: ${ORIGINS:a,b}`
- **THEN** it is `["a", "b"]`, and `["x", "y"]` with `ORIGINS=x, y`

#### Scenario: [config.placeholder.009] the request-logger templates are placeholders too

- **WHEN** `hapi.fhir.logger.error_format: "ERROR - ${requestVerb} ${requestUrl}"`
  and the environment has `requestVerb=GET`
- **THEN** `hapi.fhir.logger.error-format` = `"ERROR - GET ${requestUrl}"`
- **AND** without that variable it is the text as written (HAPI's behaviour,
  reproduced; the hazard is `DEBT-0024`)

#### Scenario: [config.placeholder.010] random values

- **WHEN** `hapi.fhir.server_address: ${random.int(10)}` and
  `hapi.fhir.default_page_size: ${random.int[5,6]}`
- **THEN** `server-address` is a decimal string of an integer in `[0, 10)` and
  `default-page-size` = `5`
- **AND** `${random.uuid}`, `${random.long}`, `${random.int}` and
  `${random.value}` resolve to a UUID, a long, an int and a 32-hex-digit string

#### Scenario: [config.placeholder.011] a key read by name resolves strictly

- **WHEN** `hapi.fhir.app_content_path: ${nope}`
- **THEN** Frevo fails to start with `Could not resolve placeholder 'nope' in
  value "${nope}"`, because the starter reads `hapi.fhir.app_content_path` by
  name (gate `app-content`, config.gate.010)

### Requirement: Relaxed property names

`frevo-config` SHALL match a configured name to a starter property with Spring
Boot 3.5.14's relaxed rules: per dot-separated element, ignoring case and every
character that is not a letter or a digit, against the starter's Java bean
property names (not its field names).

#### Scenario: [config.relaxed.001] kebab, snake, camel and upper case all bind

- **WHEN** the key under `hapi.fhir` is written `default-page-size: 5`,
  `defaultPageSize: 6`, `DEFAULT_PAGE_SIZE: 7` (under `HAPI: FHIR:`) or
  `Default__Page--Size: 9`
- **THEN** `hapi.fhir.default-page-size` is `5`, `6`, `7` or `9`

#### Scenario: [config.relaxed.002] a dot is an element separator, not a word separator

- **WHEN** the key is written `hapi.fhir.default.page.size: 8`
- **THEN** `hapi.fhir.default-page-size` = `20`

#### Scenario: [config.relaxed.003] upstream camelCase names accept every spelling

- **WHEN** `uplifted_refchains_enabled: true`,
  `user-request-retry-version-conflicts-interceptor-enabled: true`,
  `cdshooks.client-id-header-name: x`, `cors.allow-credentials: true`,
  `cr.terminology_server_client_settings.max_retry_count: 5`,
  `implementationguides.a.install_mode: STORE_ONLY` and
  `subscription.email.start_tls_enable: true` are set under `hapi.fhir`
- **THEN** each binds to its property

#### Scenario: [config.relaxed.004] names follow the bean property, not the field

- **WHEN** `hapi.fhir.cr.cql.compiler.error_level: Error` is set
- **THEN** `hapi.fhir.cr.cql.compiler.error-severity-level` = `"Info"` (the
  key is unknown and ignored, as in HAPI; the hazard is `DEBT-0025`)
- **AND** `hapi.fhir.cr.cql.compiler.error_severity_level: error` gives `"Error"`

#### Scenario: [config.relaxed.005] a higher source overrides a lower source's other spelling

- **WHEN** the packaged file has `fhir_version: R4` and the environment has
  `hapi.fhir.fhir-version=R5`
- **THEN** `hapi.fhir.fhir-version` = `"R5"`

#### Scenario: [config.relaxed.006] two spellings in one source: kebab-case wins

- **WHEN** one mapping holds `default_page_size: 3` and then `default-page-size: 4`
- **THEN** Frevo starts and `default-page-size` = `4`

#### Scenario: [config.relaxed.007] two non-kebab spellings in one source

- **WHEN** one mapping holds `defaultPageSize: 4` and `default_page_size: 3`
- **THEN** Frevo fails to start with a report naming both spellings and their
  origins (`DEV-0032`: HAPI starts and picks `3` by `java.util.HashSet`
  iteration order; the same holds for `max_page_size` / `maxPageSize`, where it
  picks the snake-case spelling)

#### Scenario: [config.relaxed.008] map keys are matched case-insensitively but kept verbatim

- **WHEN** `./application.yaml` holds `hapi.fhir.tester.Home.name: Upper` and
  the packaged file holds `tester.home`
- **THEN** `hapi.fhir.tester` has two entries, `Home` and `home`
- **AND** both have `name` = `"Upper"` and `server-address` =
  `"http://localhost:8080/fhir"`

### Requirement: Value conversion

`frevo-config` SHALL convert configured values to the starter's property types
with Spring Boot 3.5.14's conversion rules, and SHALL keep a property's
existing value when the configured value is empty and the type is not text or a
collection.

#### Scenario: [config.coerce.001] YAML booleans bind to Boolean

- **WHEN** `hapi.fhir.graphql_enabled` is `yes`, `on`, `TRUE` or `Yes`
- **THEN** `hapi.fhir.graphql-enabled` = `true`

#### Scenario: [config.coerce.002] boolean text

- **WHEN** the value is the string `"yes"`, `"on"`, `"1"`, `"true "` (trailing
  space) or `"TRUE"`
- **THEN** it is `true`
- **AND** `"no"`, `"off"`, `"0"` and `"false"` are `false`

#### Scenario: [config.coerce.003] other boolean text fails

- **WHEN** the value is `"Y"` (or `"2"`)
- **THEN** Frevo fails to start with `Reason: failed to convert
  java.lang.String to java.lang.Boolean (caused by
  java.lang.IllegalArgumentException: Invalid boolean value 'Y')`

#### Scenario: [config.coerce.004] a YAML integer does not convert to Boolean

- **WHEN** `hapi.fhir.graphql_enabled: 1` (unquoted)
- **THEN** Frevo fails to start with `Value: "1"` and `Reason:
  org.springframework.core.convert.ConverterNotFoundException: No converter
  found capable of converting from type [java.lang.Integer] to type
  [java.lang.Boolean]`

#### Scenario: [config.coerce.005] an empty boolean keeps the existing value

- **WHEN** `hapi.fhir.etag_support_enabled` is `""`, `~` or `''`
- **THEN** `hapi.fhir.etag-support-enabled` = `true` (its default), not `false`

#### Scenario: [config.coerce.006] integer text: sign and whitespace

- **WHEN** `hapi.fhir.default_page_size` is `"+5"`, `" 5 "` or `"5 0"`
- **THEN** it is `5`, `5` or `50` (all whitespace is removed before parsing)

#### Scenario: [config.coerce.007] hexadecimal integer text

- **WHEN** the value is `"0x10"`, `"#10"` or `"-0x10"`
- **THEN** it is `16`, `16` or `-16`
- **AND** the string `"010"` is `10`, while the YAML scalar `010` is `8`
  (config.yaml.005)

#### Scenario: [config.coerce.008] underscores and decimals in integer text fail

- **WHEN** the value is `"1_000"` or `"5.0"`
- **THEN** Frevo fails to start with `Reason: failed to convert
  java.lang.String to java.lang.Integer (caused by
  java.lang.NumberFormatException: For input string: "1_000")` (or `"5.0"`)

#### Scenario: [config.coerce.009] YAML numbers convert to Integer by truncation

- **WHEN** the YAML scalar is `1_000`, `1:20`, `5.7`, `1e3` or `0x10`
- **THEN** `default-page-size` is `1000`, `80`, `5`, `1000` or `16`

#### Scenario: [config.coerce.010] out-of-range numbers fail

- **WHEN** `default_page_size` is the YAML scalar `99999999999` or `.inf`
- **THEN** Frevo fails to start with `Reason: failed to convert java.lang.Long
  to java.lang.Integer (caused by java.lang.IllegalArgumentException: Could not
  convert number [99999999999] of type [java.lang.Long] to target class
  [java.lang.Integer]: overflow)` (for `.inf`: `java.lang.Double`,
  `[Infinity]`)
- **AND** `retain_cached_searches_mins: 99999999999` (a `Long`) is accepted

#### Scenario: [config.coerce.011] an empty number keeps the existing value

- **WHEN** `default_page_size` and `max_binary_size` are `""` or `~`
- **THEN** `default-page-size` = `20` and `max-binary-size` = `null`

#### Scenario: [config.coerce.012] an empty value in an object property is skipped

- **WHEN** `hapi.fhir.subscription.email.port` is empty and
  `hapi.fhir.subscription.email.username` is empty
- **THEN** `email.port` = `25` (default kept) and `email.username` = `""`

#### Scenario: [config.coerce.013] YAML booleans and numbers bound to text

- **WHEN** `hapi.fhir.mdm_rules_json_location` is the plain scalar `off`, `yes`,
  `1.10`, `1.0`, `0.8.0`, `010`, `0x1F`, `1:20`, `.5` or `1e3`
- **THEN** it is `"false"`, `"true"`, `"1.1"`, `"1.0"`, `"0.8.0"`, `"8"`,
  `"31"`, `"80"`, `"0.5"` or `"1000.0"`

#### Scenario: [config.coerce.014] empty text is the empty string, not absent

- **WHEN** `hapi.fhir.server_address` is `""`, `~` or `null`
- **THEN** `hapi.fhir.server-address` = `""`

#### Scenario: [config.coerce.015] text is not trimmed

- **WHEN** `hapi.fhir.server_address: " http://x "` (quoted)
- **THEN** it is `" http://x "`

#### Scenario: [config.coerce.016] enums by exact name

- **WHEN** `hapi.fhir.fhir_version: R4B` or `DSTU2_HL7ORG`
- **THEN** `hapi.fhir.fhir-version` is `"R4B"` or `"DSTU2_HL7ORG"`

#### Scenario: [config.coerce.017] enums leniently: case, separators and whitespace

- **WHEN** the value is `r4`, `r-4`, `Dstu3` or `" R5 "` for `fhir_version`,
  `store-and-install` for `implementationguides.x.installMode`, or `en-forced`
  and `Declared` for `cr.cql.data.profile_mode`
- **THEN** they bind to `R4`, `R4`, `DSTU3`, `R5`, `STORE_AND_INSTALL`,
  `ENFORCED` and `DECLARED`

#### Scenario: [config.coerce.018] an unknown enum value fails

- **WHEN** `fhir_version` is `"4.0.1"` or `STU3`
- **THEN** Frevo fails to start with `Reason: failed to convert
  java.lang.String to ca.uhn.fhir.context.FhirVersionEnum (caused by
  java.lang.IllegalArgumentException: No enum constant
  ca.uhn.fhir.context.FhirVersionEnum.4.0.1)` (or `.STU3`)

#### Scenario: [config.coerce.019] a YAML boolean binds to an enum's ON or OFF

- **WHEN** `hapi.fhir.cr.cql.data.profile_mode` is the plain scalar `OFF`,
  `off` or `false`
- **THEN** `hapi.fhir.cr.cql.data.profile-mode` = `"OFF"`

#### Scenario: [config.coerce.020] an empty enum keeps the existing value

- **WHEN** `fhir_version: ""`
- **THEN** `hapi.fhir.fhir-version` = `"R4"` (but see config.gate.003)

#### Scenario: [config.coerce.021] comma-separated text becomes a list, trimmed, empties kept

- **WHEN** `supported_resource_types: " Patient , Observation,,Encounter "`
- **THEN** `hapi.fhir.supported-resource-types` = `["Patient", "Observation",
  "", "Encounter"]`

#### Scenario: [config.coerce.022] list elements convert individually

- **WHEN** `search_prefetch_thresholds: "13, 503 ,-1"` and
  `allowed_bundle_types: "collection, transaction-response"`
- **THEN** `search-prefetch-thresholds` = `[13, 503, -1]` and
  `allowed-bundle-types` = `["COLLECTION", "TRANSACTIONRESPONSE"]`

#### Scenario: [config.coerce.023] YAML sequence elements convert individually

- **WHEN** `supported_resource_types: [Patient, 1.10, yes, ~]`
- **THEN** it is `["Patient", "1.1", "true", ""]`

#### Scenario: [config.coerce.024] sets drop duplicates

- **WHEN** `implementationguides.x.additional_resource_folders: [ex1, ex1]`
- **THEN** `hapi.fhir.implementation-guides.x.additional-resource-folders` = `["ex1"]`

#### Scenario: [config.coerce.025] an empty collection value is an empty collection

- **WHEN** `hapi.fhir.cors.allowed_origin` is `[]` or empty, and
  `hapi.fhir.supported_resource_types` is empty
- **THEN** `cors.allowed-origin` = `[]` (not the packaged `["*"]` nor the class
  default `["*"]`) and `supported-resource-types` = `[]`
- **AND** the same holds for `cors.allowed-headers`, `exposed-headers` and
  `allowed-methods`: an empty value gives `[]`, not the class defaults of
  config.key.002

#### Scenario: [config.coerce.026] a YAML integer binds to an enum by ordinal

- **WHEN** `hapi.fhir.fhir_version: 6` and `hapi.fhir.binary_storage_mode: 1`
  (unquoted)
- **THEN** `hapi.fhir.fhir-version` = `"R5"` (the seventh constant of
  `FhirVersionEnum`) and `hapi.fhir.binary-storage-mode` = `"FILESYSTEM"`
- **AND** the reads are `"6"` and `"1"`, so every `fhir-*` gate is off and both
  binary-storage gates are off (config.gate.002, .009)
- **AND** the quoted string `"1"` for `client_id_strategy` fails as an unknown
  constant (config.coerce.018)

#### Scenario: [config.coerce.027] an out-of-range ordinal fails

- **WHEN** `hapi.fhir.binary_storage_mode: 7` (unquoted)
- **THEN** Frevo fails to start with `Value: "7"` and `Reason: failed to
  convert java.lang.Integer to
  ca.uhn.fhir.jpa.starter.AppProperties$BinaryStorageMode (caused by
  java.lang.ArrayIndexOutOfBoundsException: Index 7 out of bounds for length 2)`

### Requirement: Collections and maps across sources

`frevo-config` SHALL merge collection and map properties across sources as
Spring Boot 3.5.14 does: a list or set comes whole from the highest source that
has any element; a map merges entry by entry.

#### Scenario: [config.collection.001] the highest source replaces a whole list

- **WHEN** `./application.yaml` holds only `hapi.fhir.logical_urls[0]: http://x/*`
- **THEN** `hapi.fhir.logical-urls` = `["http://x/*"]`

#### Scenario: [config.collection.002] an index gap fails to start

- **WHEN** `./application.yaml` holds `hapi.fhir.supported_resource_types[1]: Patient`
  and no `[0]`
- **THEN** Frevo fails to start with `Property:
  hapi.fhir.supportedresourcetypes[1]`, `Value: "Patient"` and `Reason: The
  elements [hapi.fhir.supportedresourcetypes[1]] were left unbound.`

#### Scenario: [config.collection.003] maps merge entries across sources

- **WHEN** `./application.yaml` holds `hapi.fhir.tester.other.name: Other`
- **THEN** `hapi.fhir.tester` has `home` (from the packaged file) and `other`
- **AND** `tester.other.refuse-to-fetch-third-party-urls` = `true` and
  `tester.other.fhir-version` = `"R4"` (the `Tester` class defaults) and
  `tester.other.server-address` = `null`

#### Scenario: [config.collection.004] map keys are kept verbatim

- **WHEN** `hapi.fhir.implementationguides` has the keys `ips_1_0_0` and `Swiss`,
  and `hapi.fhir.cr.cql.namespaces` has `Mixed_Key`
- **THEN** those exact keys appear in the effective configuration

#### Scenario: [config.collection.005] a string map takes dotted keys whole

- **WHEN** `hapi.fhir.cr.cql.namespaces` holds `hl7.fhir.uv.cql:
  http://hl7.org/fhir/uv/cql` and `"[hl7.fhir.us.cql]": http://hl7.org/fhir/us/cql`
- **THEN** `namespaces` = `{"hl7.fhir.uv.cql": "http://hl7.org/fhir/uv/cql",
  "hl7.fhir.us.cql": "http://hl7.org/fhir/us/cql"}`

#### Scenario: [config.collection.006] collections with no setter still bind

- **WHEN** `hapi.fhir.custom_interceptor_classes: a.B`,
  `custom_provider_classes: c.D`, `local_base_urls: http://l` and
  `auto_version_reference_at_paths: "Device.patient, Observation.subject"` are set
- **THEN** `custom-interceptor-classes` = `["a.B"]`,
  `custom-provider-classes` = `["c.D"]`, `local-base-urls` = `["http://l"]` and
  `auto-version-reference-at-paths` = `["Device.patient", "Observation.subject"]`

### Requirement: Nested objects

`frevo-config` SHALL create a nested properties object exactly when Spring Boot
3.5.14 would, and SHALL report the starter's `null` objects as `null`.

#### Scenario: [config.object.001] an object with nothing bound beneath it stays null

- **WHEN** `hapi.fhir.partitioning: {}`, or `partitioning:` with only
  `default_partition_id:` (empty) beneath it, or `subscription.email: {}`
- **THEN** `hapi.fhir.partitioning` = `null` and
  `hapi.fhir.subscription.email` = `null`

#### Scenario: [config.object.002] any bound value beneath creates the object with its defaults

- **WHEN** `hapi.fhir.subscription.email.username:` is empty
- **THEN** `hapi.fhir.subscription.email` is an object with `username` = `""`,
  `port` = `25`, `auth` = `false`, `start-tls-enable` = `false`,
  `start-tls-required` = `false`, `quit-wait` = `false`, and `from`, `host`,
  `password` = `null`

#### Scenario: [config.object.003] a scalar where an object is expected fails when nothing is beneath

- **WHEN** `hapi.fhir.partitioning:` is written with no value and no source has
  a `hapi.fhir.partitioning.*` key
- **THEN** Frevo fails to start with `Failed to bind properties under
  'hapi.fhir.partitioning' to ca.uhn.fhir.jpa.starter.AppProperties$Partitioning`
  and `Reason: org.springframework.core.convert.ConverterNotFoundException: No
  converter found capable of converting from type [java.lang.String] to type
  [ca.uhn.fhir.jpa.starter.AppProperties$Partitioning]`

#### Scenario: [config.object.004] a scalar where an object is expected is ignored when keys exist beneath

- **WHEN** `./application.yaml` holds `hapi.fhir.cors: true`
- **THEN** Frevo starts and `hapi.fhir.cors.allowed-origin` = `["*"]` (from
  the packaged file)

#### Scenario: [config.object.005] which objects exist by default

- **WHEN** no source configures them
- **THEN** `hapi.fhir.validation`, `logger`, `subscription` and `client` are
  objects with their class defaults
- **AND** `hapi.fhir.cors`, `partitioning`, `elasticsearch`,
  `subscription.email`, `tester`, `implementation-guides`,
  `remote-terminology-service` and `allowed-bundle-types` are `null`
  (with the packaged file, `cors` and `tester` exist: config.source.002)

### Requirement: Unknown, unbound and setter-less keys

`frevo-config` SHALL ignore configuration keys that no starter properties class
binds, exactly as the starter's `ignoreUnknownFields=true` binding does, and
SHALL fail on a key whose property has no setter exactly when the starter does.

#### Scenario: [config.unknown.001] unknown keys are ignored

- **WHEN** `hapi.fhir.no_such_key: 1`, `hapi.fhir.cors.no_such: x` and
  `hapi.fhir.default_page_size: 3` are set
- **THEN** Frevo starts, `default-page-size` = `3`, and nothing else differs
  from config.source.002

#### Scenario: [config.unknown.002] the packaged search-coord keys bind to nothing

- **WHEN** the packaged `application.yaml` sets
  `hapi.fhir.search-coord-core-pool-size: 20`,
  `search-coord-max-pool-size: 100` and `search-coord-queue-capacity: 200`
- **THEN** no property in `properties` carries them and no read or gate
  depends on them: neither the starter nor `hapi-fhir@7ab4c7c7`'s main sources
  read any of the three names (`SearchConfig.searchCoordinatorSvc()` takes no
  pool setting)
- **AND** setting them to any value, including `abc`, changes nothing and
  starts

#### Scenario: [config.unknown.003] keys documented at the wrong level bind to nothing

- **WHEN** `hapi.fhir.database_partition_mode_enabled: true`,
  `hapi.fhir.patient_id_partitioning_mode: true`,
  `hapi.fhir.request_tenant_partitioning_mode: false` and
  `hapi.fhir.additionalResourceFolders: [example]` are set (the packaged file's
  commented forms)
- **THEN** `hapi.fhir.partitioning` = `null` and
  `install-additional-resources-from-ig-folders` = `[]`
- **AND** the same three keys under `hapi.fhir.partitioning` do bind

#### Scenario: [config.unknown.004] keys outside hapi.fhir are kept for reads

- **WHEN** `spring.*`, `server.*` and `management.*` keys are set
- **THEN** they are loaded through the same sources, profiles, placeholders
  and name mapping as `hapi.fhir.*`, they do not appear under `properties`,
  their values are available to reads (config.read.*), and no type check is
  applied to them by this change; typed binding is the consuming change's

#### Scenario: [config.unknown.005] a setter-less property fails when set to a new value

- **WHEN** `hapi.fhir.cr.cql.runtime.enable_validation: true`
- **THEN** Frevo fails to start with `Failed to bind properties under
  'hapi.fhir.cr.cql.runtime' to ca.uhn.fhir.jpa.starter.cr.CqlRuntimeProperties`,
  `Property: hapi.fhir.cr.cql.runtime.enable-validation`, `Value: "true"` and
  `Reason: java.lang.IllegalStateException: No setter found for property:
  enable-validation`
- **AND** `enable_validation: false` (its current value) starts (HAPI's
  behaviour, reproduced; the hazard is `DEBT-0026`)

#### Scenario: [config.unknown.006] the getter-only remote terminology map

- **WHEN** `hapi.fhir.remote_terminology_services_map.a.url: http://x`
- **THEN** Frevo fails to start with `Failed to bind properties under
  'hapi.fhir' to ca.uhn.fhir.jpa.starter.AppProperties` and `Reason:
  java.lang.IllegalStateException: No setter found for property:
  remote-terminology-services-map`
- **AND** `hapi.fhir.remote_terminology_service.a.url: http://x` binds

#### Scenario: [config.unknown.007] a set key that this build does not act on yet is accepted silently

- **WHEN** a bound key whose behaviour no merged change implements yet (for
  example `hapi.fhir.bulk_export_enabled: true`) is set by any source to a
  value other than its default
- **THEN** Frevo starts, the key is bound and reported in the effective
  configuration, and nothing about it is written to standard output or
  standard error, exactly as HAPI writes nothing about a key it binds

### Requirement: Failure reporting

`frevo-config` SHALL refuse to start on an invalid configuration, before any
other component starts, and SHALL report the failure in the form the starter
prints.

#### Scenario: [config.error.001] an invalid configuration stops before anything else

- **WHEN** any scenario in this capability says Frevo fails to start
- **THEN** the process exits with status `1`
- **AND** no database connection is opened and no listening socket is bound

#### Scenario: [config.error.002] the bind failure report

- **WHEN** `./config/application.yaml` holds `hapi.fhir.graphql_enabled: "Y"`
  on line 3 with the value at column 22
- **THEN** standard error contains, line for line:
  `***************************`, `APPLICATION FAILED TO START`,
  `***************************`, a blank line, `Description:`, a blank line,
  `Failed to bind properties under 'hapi.fhir.graphql-enabled' to
  java.lang.Boolean:`, a blank line, `    Property: hapi.fhir.graphql-enabled`,
  `    Value: "Y"`, `    Origin: URL [file:config/application.yaml] - 3:22`,
  `    Reason: failed to convert java.lang.String to java.lang.Boolean (caused by
  java.lang.IllegalArgumentException: Invalid boolean value 'Y')`, a blank
  line, `Action:`, a blank line, `Update your application's configuration`
  — verbatim, including the Java type names, which are fixed strings per
  property type and failure kind

#### Scenario: [config.error.003] the property is named canonically whatever was written

- **WHEN** the invalid value was written as `graphqlEnabled`, `GRAPHQL_ENABLED`
  or `graphql-enabled`
- **THEN** the report's `Property:` line is `hapi.fhir.graphql-enabled`

#### Scenario: [config.error.004] the origin of an environment variable, an argument and SPRING_APPLICATION_JSON

- **WHEN** the invalid value `abc` for `default_page_size` comes from
  `HAPI_FHIR_DEFAULT_PAGE_SIZE`, from `--hapi.fhir.default_page_size=abc`, or
  from `SPRING_APPLICATION_JSON`
- **THEN** the `Origin:` line is `System Environment Property
  "HAPI_FHIR_DEFAULT_PAGE_SIZE"`, `"hapi.fhir.default_page_size" from property
  source "commandLineArgs"`, or `System Environment Property
  "SPRING_APPLICATION_JSON"`

#### Scenario: [config.error.005] the origin line and column are of the value

- **WHEN** the invalid value is on line `L` starting at column `C` (1-based) of a YAML file
- **THEN** the `Origin:` line ends ` - L:C`

#### Scenario: [config.error.006] several invalid values

- **WHEN** two different properties have invalid values
- **THEN** Frevo fails to start and the report names at least one of them; it
  never starts with either value defaulted

### Requirement: Keys the starter reads by name

`frevo-config` SHALL expose, as `reads`, the value of every key the starter
reads by name from the Spring `Environment` rather than through a properties
class, with that lookup's own matching rules: relaxed for a name in canonical
form, exact (plus the environment-variable variants) for a name that contains
`_` or upper case.

#### Scenario: [config.read.001] the reads reported

- **WHEN** the effective configuration is printed
- **THEN** `reads` has exactly the members `hapi.fhir.fhir_version`,
  `hapi.fhir.cr.enabled`, `hapi.fhir.cdshooks.enabled`, `hapi.fhir.mdm_enabled`,
  `hapi.fhir.ips_enabled`, `hapi.fhir.ig_runtime_upload_enabled`,
  `hapi.fhir.binary_storage_mode`, `hapi.fhir.custom_content_path`,
  `hapi.fhir.app_content_path`,
  `hapi.fhir.enable_repository_validating_interceptor`,
  `hapi.fhir.custom-bean-packages` and `spring.ai.mcp.server.enabled`
- **AND** each is the text value or `null` when absent; with only the packaged
  file, `hapi.fhir.fhir_version` = `"R4"`, `hapi.fhir.cr.enabled` = `"false"`,
  `hapi.fhir.mdm_enabled` = `"false"`, `hapi.fhir.ips_enabled` = `null`,
  `spring.ai.mcp.server.enabled` = `"true"`

#### Scenario: [config.read.002] an underscore name is read exactly

- **WHEN** `./application.yaml` holds `hapi.fhir.mdm-enabled: true` and
  `IPS_ENABLED: "true"` under `hapi.fhir`
- **THEN** `hapi.fhir.mdm-enabled` = `true` and `hapi.fhir.ips-enabled` =
  `true` in `properties`
- **AND** the read `hapi.fhir.mdm_enabled` is `"false"` (the packaged spelling)
  and the read `hapi.fhir.ips_enabled` is `null` (`DEBT-0022`)

#### Scenario: [config.read.003] a canonical name is read relaxed

- **WHEN** `./application.yaml` holds `hapi.fhir.cr.Enabled: true` and
  `hapi.fhir.CdsHooks.enabled: true`
- **THEN** the reads `hapi.fhir.cr.enabled` and `hapi.fhir.cdshooks.enabled`
  are `"true"`
- **AND** the read `hapi.fhir.custom-bean-packages` is `"a.b"` for
  `customBeanPackages: a.b` under `hapi.fhir`

#### Scenario: [config.read.004] environment-variable variants for an exact read

- **WHEN** `HAPI_FHIR_MDM_ENABLED=true`
- **THEN** the read `hapi.fhir.mdm_enabled` is `"true"`
- **AND** with `HAPI_FHIR_IPSENABLED=true` the read `hapi.fhir.ips_enabled` is
  `null` while `hapi.fhir.ips-enabled` = `true`
- **AND** the variants looked up for an exact read of `n` are `n`, `n` with `.`
  replaced by `_`, `n` with `-` replaced by `_`, `n` with both replaced, and
  the upper-case form of each

#### Scenario: [config.read.005] a dashed spelling splits the read from the binding

- **WHEN** the environment has `hapi.fhir.fhir-version=R5`
- **THEN** `hapi.fhir.fhir-version` = `"R5"` while the read
  `hapi.fhir.fhir_version` is `"R4"` (HAPI's behaviour, reproduced; the hazard
  is `DEBT-0022`)

#### Scenario: [config.read.006] reads see YAML values as their text

- **WHEN** `hapi.fhir.mdm_enabled: yes` (a YAML boolean) or `mdm_enabled: ~`
- **THEN** the read `hapi.fhir.mdm_enabled` is `"true"` or `""`

#### Scenario: [config.read.007] reads resolve placeholders strictly

- **WHEN** the value of a read key holds an unresolvable placeholder
- **THEN** Frevo fails to start (config.placeholder.011)

### Requirement: Starter gates

`frevo-config` SHALL compute, as `gates`, each condition the starter evaluates
to decide whether a feature's configuration is loaded, with the starter's exact
predicate, so that later changes decide wiring from the same answer HAPI
reaches, including where that answer disagrees with the bound property.

#### Scenario: [config.gate.001] the gates reported

- **WHEN** the effective configuration is printed
- **THEN** `gates` has exactly the boolean members `fhir-dstu2`, `fhir-dstu3`,
  `fhir-r4`, `fhir-r4b`, `fhir-r5`, `fhir-any`, `cr`, `cdshooks`, `mdm`,
  `ips`, `ig-runtime-upload`, `repository-validating-interceptor`,
  `binary-storage-filesystem`, `binary-storage-database`, `custom-content`,
  `app-content`, `mcp`, `mcp-cdshooks`, `cors`, `implementation-guides`,
  `partitioning`, `remote-terminology`, `tester` and `elasticsearch`
- **AND** with only the packaged file the members that are `true` are
  `fhir-r4`, `fhir-any`, `binary-storage-database`, `mcp`, `cors` and `tester`

#### Scenario: [config.gate.002] the version gates follow FhirVersionEnum.forVersionString

- **WHEN** the read `hapi.fhir.fhir_version` is `R4`, `r4`, `R4B`, `R5`,
  `DSTU2`, `DSTU3` or `Dstu3`
- **THEN** exactly one of `fhir-r4`, `fhir-r4`, `fhir-r4b`, `fhir-r5`,
  `fhir-dstu2`, `fhir-dstu3`, `fhir-dstu3` is on, and `fhir-any` is on
- **AND** the read is upper-cased, then the first enum constant (in the order
  `DSTU2`, `DSTU2_HL7ORG`, `DSTU2_1`, `DSTU3`, `R4`, `R4B`, `R5`) whose FHIR
  version string (`1.0.2`, `1.0.2`, `1.4.0`, then each model's
  `Constants.VERSION` in the pinned image: `3.0.2`, `4.0.1`, `4.3.0`, `5.0.0`)
  starts with the read text cut before its second `.` is chosen, else the
  names `DSTU2`, `DSTU3`, `STU3`, `R4`, `R4B`, `R5` are tried
- **AND** so `4.0` and `4.0.1` turn on `fhir-r4` (although neither binds to
  `hapi.fhir.fhir-version`, config.coerce.018)

#### Scenario: [config.gate.003] an empty version turns on DSTU2

- **WHEN** `fhir_version: ""`
- **THEN** `hapi.fhir.fhir-version` = `"R4"` but gate `fhir-dstu2` is on and
  `fhir-r4` is off (every version string starts with the empty string; HAPI's
  behaviour, reproduced; the hazard is `DEBT-0023`)

#### Scenario: [config.gate.004] a version the gates do not know turns every version gate off

- **WHEN** `fhir_version` is `r-4` (binds `R4`) or `DSTU2_HL7ORG`
- **THEN** every `fhir-*` gate is off, including `fhir-any`

#### Scenario: [config.gate.005] no fhir_version anywhere fails to start

- **WHEN** Frevo is started with `--spring.config.location=file:./x.yaml` and no
  source sets `hapi.fhir.fhir_version` in a spelling the read sees
- **THEN** Frevo fails to start with a report naming `hapi.fhir.fhir_version`
  (HAPI fails with a `NullPointerException` in `OnR4Condition`)

#### Scenario: [config.gate.006] cr and cdshooks use Boolean.parseBoolean

- **WHEN** the read `hapi.fhir.cr.enabled` is `"true"`, `"TRUE"`, `"yes"`, `"1"`
  or absent
- **THEN** gate `cr` is on, on, off, off, off
- **AND** gate `cdshooks` applies the same rule to `hapi.fhir.cdshooks.enabled`
- **AND** so `hapi.fhir.cr.enabled: yes` binds `true` while gate `cr` is off
  (HAPI's behaviour, reproduced; `DEBT-0022`)

#### Scenario: [config.gate.007] mdm is on for any present value but false

- **WHEN** the read `hapi.fhir.mdm_enabled` is `"true"`, `"no"`, `"0"`, `""`,
  `"FALSE"` or absent
- **THEN** gate `mdm` is on, on, on, on, off, off
- **AND** so `mdm_enabled: "no"` binds `false` while gate `mdm` is on
  (HAPI's behaviour, reproduced; `DEBT-0022`)

#### Scenario: [config.gate.008] having-value gates compare ignoring case

- **WHEN** the reads `hapi.fhir.ips_enabled`, `hapi.fhir.ig_runtime_upload_enabled`
  and `hapi.fhir.enable_repository_validating_interceptor` are `"TRUE"`, and
  in a second run `"yes"`
- **THEN** gates `ips`, `ig-runtime-upload` and
  `repository-validating-interceptor` are on in the first run and off in the second

#### Scenario: [config.gate.009] binary storage gates

- **WHEN** the read `hapi.fhir.binary_storage_mode` is absent, `"DATABASE"`,
  `"filesystem"` or `"file-system"`
- **THEN** (`binary-storage-database`, `binary-storage-filesystem`) is (on,
  off), (on, off), (off, on), (off, off)
- **AND** for `"file-system"` the property `hapi.fhir.binary-storage-mode` =
  `"FILESYSTEM"` (config.coerce.017)

#### Scenario: [config.gate.010] custom and app content gates

- **WHEN** the read `hapi.fhir.custom_content_path` is `"./custom"`, `""`,
  `"false"` or absent
- **THEN** gate `custom-content` is on, on, off, off
- **AND** gate `app-content` applies the same rule to `hapi.fhir.app_content_path`

#### Scenario: [config.gate.011] mcp gates

- **WHEN** the read `spring.ai.mcp.server.enabled` is `"true"` (packaged) or `"false"`
- **THEN** gate `mcp` is on or off
- **AND** gate `mcp-cdshooks` is on exactly when the read
  `hapi.fhir.cdshooks.enabled` equals `true` ignoring case

#### Scenario: [config.gate.012] the cors gate follows the bound object

- **WHEN** `hapi.fhir.cors` is non-null in `properties`
- **THEN** gate `cors` is on, whatever spelling configured it
- **AND** with `--spring.config.location=file:./x.yaml` and no cors key it is off

#### Scenario: [config.gate.013] the implementation-guides gate

- **WHEN** `hapi.fhir.implementation-guides` is `null`, has no entry, or has an entry
- **THEN** gate `implementation-guides` is off, off, on

#### Scenario: [config.gate.014] the partitioning gate

- **WHEN** `hapi.fhir.partitioning` is `null` (including config.object.001's
  cases) or an object
- **THEN** gate `partitioning` is off or on

#### Scenario: [config.gate.015] the remote-terminology gate

- **WHEN** `hapi.fhir.remote-terminology-service` is `null`, empty, or has an entry
- **THEN** gate `remote-terminology` is off, off, on

#### Scenario: [config.gate.016] the tester gate is a raw name-prefix test

- **WHEN** some source holds a property whose name, as that source spells it,
  starts with the text `hapi.fhir.tester`
- **THEN** gate `tester` is on (so `hapi.fhir.testerx: 1` also turns it on)
- **AND** `HAPI_FHIR_TESTER_HOME_NAME=x` alone, with
  `--spring.config.location=file:./x.yaml`, leaves it off while
  `hapi.fhir.tester.home.name` = `"x"`

#### Scenario: [config.gate.017] cds hooks need both gates

- **WHEN** gate `cdshooks` is on and gate `cr` is off
- **THEN** the document's `gates` reports `cdshooks` on and `cr` off; the
  combined condition of `StarterCdsHooksConfig` is their conjunction and is
  reported by no separate member

#### Scenario: [config.gate.018] gates are computed after every source is applied

- **WHEN** a gate's key is set in a profile file, an imported file, an
  environment variable or an argument
- **THEN** the gate reflects that highest-precedence value

#### Scenario: [config.gate.019] a failure while computing a gate fails to start

- **WHEN** binding `AppProperties` fails (for example config.unknown.006)
- **THEN** Frevo fails to start with the binding report, whether the failure
  surfaces first in a gate or in the properties

#### Scenario: [config.gate.020] the elasticsearch gate

- **WHEN** any `spring.elasticsearch.*` property binds to Spring Boot's
  `ElasticsearchProperties` (for example under profile `elastic`)
- **THEN** gate `elasticsearch` is on; with no such property it is off

### Requirement: Every starter key and its defaults

`frevo-config` SHALL bind every key of the starter's nine properties classes,
with the class defaults the starter declares, and SHALL report each in the
effective configuration.

#### Scenario: [config.key.001] AppProperties class defaults

- **WHEN** Frevo is started with `--spring.config.location=optional:file:./none.yaml`
- **THEN** every `hapi.fhir.*` property reports the class default listed in
  [`keys.md`](../../keys.md) section 1, including `openapi-enabled` = `false`,
  `narrative-enabled` = `true`, `default-page-size` = `20`, `max-page-size` =
  `2147483647`, `client-id-strategy` = `"ALPHANUMERIC"`,
  `server-id-strategy` = `null`, `default-encoding` = `"JSON"`,
  `normalized-quantity-search-level` =
  `"NORMALIZED_QUANTITY_SEARCH_NOT_SUPPORTED"`,
  `store-meta-source-information` = `"NONE"`, `binary-storage-mode` =
  `"DATABASE"`, `search-prefetch-thresholds` = `[]`,
  `reuse-cached-search-results-millis` = `60000`

#### Scenario: [config.key.002] nested class defaults

- **WHEN** each nested object is created with no keys beneath it set except one
- **THEN** `Cors` defaults are `allow-credentials` = `false`,
  `allowed-origin` = `["*"]`, `allowed-headers` = `["Origin", "Accept",
  "Content-Type", "Authorization", "Cache-Control", "If-Match",
  "If-None-Match", "x-fhir-starter", "X-Requested-With", "Prefer"]`,
  `exposed-headers` = `["Location", "Content-Location", "ETag", "Date",
  "Retry-After", "X-Correlation-Id", "X-Progress", "X-Request-Id"]`,
  `allowed-methods` = `["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH",
  "HEAD"]` (HAPI's `AppPropertiesCorsDefaultsTest`)
- **AND** `Logger`: `name` `"fhirtest.access"`, `error-format` `"ERROR -
  ${requestVerb} ${requestUrl}"`, `format` the 407-character default of
  `AppProperties.java:970-971` verbatim (including its duplicated second half),
  `log-exceptions` `true`
- **AND** `Tester`: `refuse-to-fetch-third-party-urls` `true`, `fhir-version`
  `"R4"`; `Validation`: both `false`; `Partitioning`: all `false` except
  `request-tenant-partitioning-mode` `true`, `default-partition-id` `0`;
  `Subscription`: all `false`, `polling-interval-ms` `null`, `email` `null`;
  `Elasticsearch`: `index-prefix` `""`; `Client`: `socket-timeout`,
  `connect-timeout`, `connection-request-timeout` `10000`, `connection-ttl`
  `5000`, `pool-max-total` and `pool-max-per-route` `20`; `RemoteSystem`:
  `system` and `url` `null`

#### Scenario: [config.key.003] clinical reasoning and CDS Hooks class defaults

- **WHEN** Frevo is started as in config.key.001
- **THEN** `hapi.fhir.cr.enabled` = `null`, `hapi.fhir.cdshooks.enabled` =
  `false`, `hapi.fhir.cdshooks.client-id-header-name` = `null`, and every
  `hapi.fhir.cr.*` property reports the default listed in `keys.md` section 2,
  including `cr.terminology-server-client-settings` = `{"max-retry-count": 3,
  "retry-interval-millis": 1000, "timeout-seconds": 30, "socket-timeout": 60,
  "crmi-version": "1.0.0", "expansions-per-page": 1000,
  "max-expansion-pages": 1000, "tx-resource-mode": "AUTO",
  "max-tx-resource-code-system-concepts": 1000}`,
  `cr.cql.data.profile-mode` = `"OFF"`, `cr.cql.terminology.code-lookup-mode`
  = `"AUTO"`, `cr.cql.compiler.error-severity-level` = `"Info"`

#### Scenario: [config.key.004] the packaged application.yaml's differences from the class defaults

- **WHEN** Frevo starts with only the packaged files
- **THEN** the properties that differ from config.key.001 and .003 are exactly
  those listed in `keys.md` section 3 (for example `openapi-enabled` `true`,
  `narrative-enabled` `false`, `cr.cql.data.profile-mode` `"DECLARED"`,
  `cr.cql.terminology.code-lookup-mode` `"USE_VALIDATE_CODE_OPERATION"`)

#### Scenario: [config.key.005] every bound key round-trips from YAML

- **WHEN** Frevo starts with `harness/config-roundtrip/cases/all-keys.yaml`,
  which sets every bound key of `keys.md` to a non-default value in the
  spelling the packaged file or the starter documentation uses
- **THEN** the effective configuration equals the HAPI side's for the same
  case, member for member

#### Scenario: [config.key.006] every bound key round-trips from the environment

- **WHEN** Frevo starts with `harness/config-roundtrip/cases/all-keys.env`,
  the upper-case environment form of the same values where config.env.011
  allows one, and the dotted form otherwise
- **THEN** the effective configuration equals the HAPI side's for the same case

#### Scenario: [config.key.007] every bound key accepts its kebab-case spelling

- **WHEN** Frevo starts with `harness/config-roundtrip/cases/all-keys-kebab.yaml`
- **THEN** the effective configuration equals the one for config.key.005

#### Scenario: [config.key.008] zero cache-reuse milliseconds means absent

- **WHEN** `hapi.fhir.reuse_cached_search_results_millis: 0`
- **THEN** `hapi.fhir.reuse-cached-search-results-millis` = `null`
- **AND** `1` gives `1`

#### Scenario: [config.key.009] implementation guide entries

- **WHEN** an implementation guide entry sets only `name`
- **THEN** it reports `name`, `package-url` `null`, `version` `null`,
  `install-mode` `null`, `install-resource-types` `[]`, `fetch-dependencies`
  `false`, `reload-existing` `true`, `dependency-excludes` `[]`,
  `additional-resource-folders` `[]`, `version-policy` `"MULTI_VERSION"`,
  `dry-run` `false`, `overwrite-content-not-present-code-systems` `false`
- **AND** `installResourceTypes: CodeSystem, ValueSet` and
  `fetch_dependencies: yes` and `reloadExisting: false` bind

#### Scenario: [config.key.010] bound keys the starter never reads

- **WHEN** `hapi.fhir.cr_enabled`, `install_additional_resources_from_ig_folders`,
  `subscription.email.auth`, `subscription.email.quitWait`,
  `subscription.email.startTlsRequired`, `cr.cql.compiler.translator_format`
  and `cr.cql.use_embedded_libraries` are set
- **THEN** they are bound and reported, and `keys.md` records that HAPI 8.12.0
  reads none of them, so no later change wires them to behaviour

#### Scenario: [config.key.011] every inventory row is accounted for

- **WHEN** `keys.md` is compared with the 209 configuration rows of
  `docs/inventory.md`
- **THEN** every row appears in `keys.md` with one of the dispositions
  `bound`, `bound, never read`, `bound, setter-less`, `map entry`, `read only`,
  `not bound` or `not a key`, the scenarios that cover it, and the change that
  wires it to behaviour (or `none`)
- **AND** `keys.md` also lists the bound keys the inventory misses

### Requirement: The config-roundtrip oracle

The `config-roundtrip` oracle SHALL run each case against Spring Boot 3.5.14
with the starter's nine properties classes and gates, on Linux, and produce the
same effective-configuration document, so that every `config.*` scenario with a
fixture is judged against HAPI rather than against this text.

#### Scenario: [config.oracle.001] the HAPI side is the starter's own classes

- **WHEN** the HAPI side of a case runs
- **THEN** it boots a Spring Boot 3.5.14 application with
  `WebApplicationType.NONE`, the nine properties classes compiled from
  `hapi-fhir-jpaserver-starter@377e6eba`, the starter's condition classes and
  the packaged resources, with the case's working directory, environment and
  arguments
- **AND** its classpath jar versions are those inside the pinned image

#### Scenario: [config.oracle.002] self-test

- **WHEN** the HAPI side runs the whole case corpus twice
- **THEN** both runs produce byte-identical documents, except for values a case
  marks as random (config.placeholder.010)

#### Scenario: [config.oracle.003] failures are compared as failures

- **WHEN** the HAPI side fails to start for a case
- **THEN** the case passes only if Frevo also fails to start, and, when the
  HAPI report has one `Property:` line, Frevo's report names the same property

#### Scenario: [config.oracle.004] nothing is normalised but set order

- **WHEN** two documents are compared
- **THEN** sets are compared as sorted arrays and object member order is
  ignored; every value, every `null` and every gate is compared exactly

#### Scenario: [config.oracle.005] the case corpus covers the scenarios

- **WHEN** the corpus under `harness/config-roundtrip/cases/` is listed
- **THEN** every scenario in this capability that states a configuration and
  its result has at least one case tagged `covers: <id>`

### Requirement: Configuration load performance

Loading configuration SHALL be a startup-only cost, and reading a bound value
after startup SHALL not parse or allocate.

#### Scenario: [config.perf.001] loading the packaged configuration is fast

- **WHEN** the `config-load-packaged` workload of `bench/micro/workloads.yaml`
  loads and binds the packaged files plus the 120-line
  `bench/micro/fixtures/config-all-keys.yaml`
- **THEN** its median is at most 5 ms in the bench profile on the CI runner

#### Scenario: [config.perf.002] reading a bound value costs a field read

- **WHEN** the `config-get` workload reads `hapi.fhir.default-page-size` from
  the loaded configuration
- **THEN** its median is at most 5 ns and it performs no allocation

#### Scenario: [config.perf.003] no regression against the baseline

- **WHEN** the two workloads run on a pull request
- **THEN** neither median exceeds its `bench/micro/baseline.json` value by more
  than 10%
