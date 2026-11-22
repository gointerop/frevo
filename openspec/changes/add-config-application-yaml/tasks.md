# Tasks — `add-config-application-yaml`

Ordered. Each is roughly half a day and independently verifiable. Every task
that changes behaviour names the scenario IDs it satisfies, per
`openspec/config.yaml` `rules.tasks`.

**Role boundaries.** Tasks 1 to 3 are `oracle-builder` work (tests and
`harness/**`). Tasks marked **[chore]** cannot be done on this branch: they
touch `reference/**`, `harness/scripts/**` or `docs/adr/**` on a `chore/*`
branch, and each is an owner checkpoint. Tasks 7 onward are `implementer`
work in `crates/frevo-config/**`. No task after 3 starts before its scenarios
have failing tests, and no file under `crates/` is edited before `review.md`
reads `APPROVED`.

**Decisions.** Every question of the first draft was decided on 2026-11-22
(proposal.md, "Decisions"); no task waits on one.

---

## Phase A — the oracle and the failing tests

- [x] **1. Build the `config-roundtrip` HAPI side.** `harness/config-roundtrip/`
  per design.md section 7: the probe, compiled with `javac` in a Docker build
  stage against the image's own `WEB-INF/lib` and `WEB-INF/classes` (no Maven
  `pom.xml`; owner ruling of 2026-11-22), with the getter-to-path
  dumper of config.dump.002 (including the derived-getter exclusions and the
  `CqlTerminologyProperties` flattening of design.md section 8, and the
  password redaction of config.dump.005), the starter's condition classes
  evaluated as the gates of config.gate.001, and the named reads of
  config.read.001. Build it into a container `FROM
  hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`, with its classes in
  `/app/extra-classes/`, and run it with the image's own launcher
  (`PropertiesLauncher`, the ENTRYPOINT's `-Dloader.path`) so the class path,
  its order, `/app/extra-classes/` and the packaged `logback.xml` are HAPI's
  (config.oracle.001); capture standard output and standard error
  separately; record `System.getProperties()` for design.md section 3.6;
  assert that no `WEB-INF/lib` jar carries
  `application*.{properties,xml,yml,yaml}` or `config/application*`; fail the
  build if
  `spring-boot`, `spring-core` or `snakeyaml` in that directory is not
  3.5.14, 6.2.19, 2.4 (proposal Risk 1; Spring Framework 6.2.19 is the
  reference by the owner's ruling of 2026-11-22). Extract the three packaged YAML files
  from the war and assert them byte-equal to the copies Frevo embeds
  (config.source.001). Self-test: two runs, identical output
  (config.oracle.001, .002).
- [ ] **2. Write the case corpus and record HAPI's results.** One case
  directory per scenario with a configuration (`files/`, `env`, `args`,
  `covers:` tags), including the fixture `config.yaml.016.yaml` and the
  `requestVerb` case; generate `all-keys.yaml`, `all-keys.env` and
  `all-keys-kebab.yaml` from `keys.md` (config.key.005-.007) with a
  non-default value for every bound key and every generic map-entry field;
  add the cases whose recorded HAPI result settles a clause: lower-case and
  mixed-case environment names on Linux (config.env.004), an environment
  variable named `--spring.config.location` (config.env.014), and the
  `JAVA_TOOL_OPTIONS`/`JDK_JAVA_OPTIONS` precedence (config.source.023), the
  `/app/extra-classes/` class-path order (config.source.017, .030), and every
  failure output of config.error.002 and .007-.010 with its exact log lines.
  The fixtures in `probe/fixtures/` are copied verbatim. Record
  `hapi-8.12.0-results.json` with the image digest. Implement the comparison
  of config.oracle.003 and .004, and the coverage check of config.oracle.005.
  Any recorded HAPI result that contradicts a THEN clause in the spec delta
  is written to `tasks/blockers.md` and the delta is corrected before task 3
  (findings O-1 to O-13 of 2026-11-22, folded into the delta the same day;
  proposal.md, "Oracle findings of 2026-11-22 and the owner's rulings").
- [ ] **3. Write the failing Rust tests.** `crates/frevo-config/tests/`, one
  file per area, every scenario tagged `// covers: config.<area>.<nnn>`:
  each config case runs `frevo-config effective` in a temporary directory
  with the case's files, a cleared environment plus the case's variables
  and `FREVO_CONFIG_TEST_ROOT` (design.md section 11), and its arguments,
  and compares the document (or the failure report and exit
  status) with the recorded HAPI result. Scenarios with no HAPI counterpart
  (config.dump.*, config.unknown.007, config.perf.*) assert the spec text
  directly. `harness/scripts/scenario-coverage.sh --change
  add-config-application-yaml` reports 0 uncovered.

## Phase B — prerequisites outside this branch

- [ ] **4. [chore] ADR 0007.** Accepted by the owner on 2026-11-22 (option
  A). The owner moves
  [`adr-0007-yaml-event-parser.md`](adr-0007-yaml-event-parser.md) to
  `docs/adr/0007-yaml-event-parser.md`; add `yaml-rust2` to
  `[workspace.dependencies]`, remove `serde_yaml_ng`, and confirm
  `cargo deny check` passes. Blocks task 7.
- [ ] **5. [chore] Pin the Spring sources** — `spring-projects/spring-boot`
  `v3.5.14`, `spring-projects/spring-framework` `v6.2.19`, SnakeYAML
  `snakeyaml-2.4` — in `reference/PINS.md` and `reference/clone.sh`, on a
  `chore/*` branch (ruling of 2026-11-22). #38 pinned Spring Framework at
  `v6.2.18`; the owner ruled on 2026-11-22 that the reference is 6.2.19, the
  version the pinned image ships, so the Spring Framework pin is moved to
  `v6.2.19` (Spring Boot and SnakeYAML unchanged). A prerequisite recorded
  here, not work of this change; does not block implementation, since the
  oracle is the standard either way.
- [ ] **6. [chore] Correct the inventory.** Fix
  `harness/scripts/gen-inventory.py` to read bean properties (getters and
  setters) instead of fields, to see `TerminologyServerClientSettings` and
  `PackageInstallationSpec`, and to stop reading YAML comments and splitting
  list values; regenerate `docs/inventory.md` and `docs/inventory-totals.json`;
  the result must match `keys.md` sections 5 to 7 (ruling of 2026-11-22: a
  `chore/*` branch before archive). Blocks archiving, not implementation.
  #39 corrected `docs/inventory.md` and `docs/inventory-totals.json` by hand
  (220 configuration rows, config.key.011); the generator fix is still owed.

## Phase C — implementation

- [ ] **7. YAML events, YAML 1.1 resolution and flattening.** Event adapter
  over the ADR 0007 parser; node tree with anchors, aliases and `<<` merge;
  duplicate-key rejection with both positions; the SnakeYAML 2.4 resolver
  table without timestamps; Spring `YamlProcessor` flattening; value origins.
  Satisfies config.yaml.001-.016.
- [ ] **8. Java number and boolean text.** Rendering of YAML ints, big ints,
  floats (JDK 21 `Double.toString` layout over shortest digits) and booleans
  as `String` values. Satisfies config.yaml.004-.006, config.coerce.013,
  config.coerce.023.
- [ ] **9. Properties files.** `OriginTrackedPropertiesLoader` syntax:
  separators, comments, escapes, continuation, `#---` documents, origins.
  Satisfies config.properties.001-.007.
- [ ] **10. Property sources and name mappers.** Command-line arguments
  (repeats joined, bare `--name`), `SPRING_APPLICATION_JSON`, the environment
  with both mappers on a case-sensitive environment, the uniform-form name
  model, kebab-first lookup within a source, the ambiguity failure of
  config.relaxed.007 (`DEV-0032`), the system-properties source (the JVM's
  own properties per design.md 3.6, then the `-D` tokens of
  `JAVA_TOOL_OPTIONS` and `JDK_JAVA_OPTIONS` with the JVM's quoting, and the
  JVM's notices), `spring.application.json` from any source, and the
  argument parser's `--=x`, `--` and `a=b` rules. Satisfies
  config.env.001-.014, config.args.001-.012, config.relaxed.001-.003,
  .005-.008, config.source.023-.025.
- [ ] **11. Config data locations and imports.** Default locations and
  wildcard subdirectories, `location`, `additional-location`, `import` (from
  sources and from documents, relative resolution), `name`, `optional:`,
  extension precedence, `classpath:` as the embedded files (from
  `crates/frevo-config/packaged/`, byte-identical to
  `harness/config-roundtrip/packaged/`) then `/app/extra-classes/`, absolute
  paths beneath `FREVO_CONFIG_TEST_ROOT` when it is set (design.md section
  11), profile variants of named files, extension hints,
  directory imports, one-level wildcards, the not-found and
  unsupported-location failures including `configtree:` and `.xml`
  (`DEV-0031`). Satisfies config.source.001-.022, .026-.030.
- [ ] **12. Profiles.** `active`, `include`, `group`, `default`; profile files
  and `on-profile` documents with `!`, `&`, `|` expressions; ordering; the
  invalid-in-profile-resource failure; `on-cloud-platform` never matching
  (`DEV-0031`); profile-name validation and `spring.profiles.validate`; the
  legacy `spring.profiles` key; profile-control keys in profile-specific
  files. Satisfies config.profile.001-.018.
- [ ] **13. Placeholders.** Lenient and strict resolution, defaults, nesting,
  escaping, circular detection, `random.*`, the JVM's system properties.
  Satisfies config.placeholder.001-.013.
- [ ] **14. Conversion.** Boolean, Integer, Long, String, enum (exact,
  lenient, boolean-to-`ON`/`OFF`, ordinal), comma lists, sets, empty handling,
  and every failure text the spec quotes, verbatim with Java type names.
  Satisfies
  config.coerce.001-.027.
- [ ] **15. Descriptor tables for `AppProperties`.** Every property, nested
  class and quirk of design.md section 5.1, with a provenance header naming
  `AppProperties.java` at `377e6eba` and stating it was modified. Satisfies
  config.key.001, .002, .008, .010 (the `AppProperties` part),
  config.collection.006, config.unknown.006.
- [ ] **16. Descriptor tables for the other eight classes.** `CdsHooksProperties`,
  `CrProperties` (with `TerminologyServerClientSettings` from
  clinical-reasoning `ccba474a`), `CareGapsProperties`, `CqlProperties`,
  `CqlCompilerProperties`, `CqlRuntimeProperties`,
  `CqlTerminologyProperties`, `CqlData`, and `PackageInstallationSpec` from
  `hapi-fhir` `7ab4c7c7` (with `packageContents`), Spring Boot 3.5.14's
  `ElasticsearchProperties` with `Duration` conversion, the setter-less
  derived getters; enum constant lists for every enum-typed property.
  Satisfies config.key.003, .009, .010, .012, config.relaxed.004,
  config.unknown.005, .008, config.gate.024.
- [ ] **17. The binder.** Design.md section 5.2: lookup, collections whole
  from one source, maps merged, objects created only when something beneath
  binds, scalar-for-object handling, setter-less check, index gaps, two beans
  on overlapping prefixes. Satisfies config.collection.001-.005,
  config.object.001-.005, config.unknown.001-.004, config.key.004-.007,
  config.dump.002.
- [ ] **18. Named reads and gates.** The thirteen reads with their lookup
  rules and strict placeholders; the twenty-five gates with the starter's and
  HAPI's predicates, including `forVersionString`, the raw tester prefix scan
  over every enumerable source (system properties included), the
  `ElasticsearchProperties` bind and `scheduling_disabled`'s case-sensitive
  test. Satisfies config.read.001-.008, config.gate.001-.024,
  config.placeholder.011.
- [ ] **19. Failure output and early exit.** Design.md section 6a: standard
  output, the logback console line form, the failure-analysis and run-failed
  forms per failure kind, the preceding `WARN`/`INFO` lines, no stack frames,
  no logback status line and no non-config log line (`DEV-0033`, approved by the owner on 2026-11-22), origins, exit status `1`, nothing
  started before configuration succeeds. Satisfies config.error.001-.010,
  config.dump.006.
- [ ] **20. `frevo-config effective`.** The binary target, named
  `frevo-config` (the tests use `CARGO_BIN_EXE_frevo-config`, design.md
  section 11), the document of config.dump.001-.006, redaction of the e-mail
  password.
- [ ] **21. Silent acceptance of keys this build does not act on.** A test
  over every `bound` key of `keys.md` set to a non-default value asserts that
  startup writes nothing about it, as HAPI writes nothing. Satisfies
  config.unknown.007.
- [ ] **22. Benchmarks.** Add `config-load-packaged` and `config-get` to
  `bench/micro/workloads.yaml` with the fixture
  `bench/micro/fixtures/config-all-keys.yaml`; record results and baseline.
  Satisfies config.perf.001-.003.

## Phase D — records

- [ ] **23. Registers.** The coordinator appends
  [`register-entries.md`](register-entries.md) — `DEV-0031`, `DEV-0032` to
  `docs/deviations.md` and `DEBT-0022` to `DEBT-0026` to `docs/tech-debt.md`;
  every test of a scenario that cites one also carries the entry's ID in a
  comment; `harness/scripts/check-registries.sh` passes.
- [ ] **24. Provenance and notices.** A `docs/provenance.md` row per ported
  starter, hapi-fhir and clinical-reasoning file and for the three packaged
  YAML files; confirm `NOTICE` already carries the starter's Apache-2.0
  notice.
- [ ] **25. Inventory and capability docs.** After task 6: point every
  configuration row of `docs/inventory.md` at its scenario IDs and set the
  status of the rows this change completes; answer the two open questions in
  `docs/capabilities/config.md` with links to this change.
- [ ] **26. Keep `keys.md` reachable after archiving.** `config.key.001`
  links `../../keys.md`, which stops resolving when the delta is promoted to
  `openspec/specs/config/spec.md`. Before `openspec archive`, rewrite that
  link to the archived path
  (`openspec/changes/archive/<date>-add-config-application-yaml/keys.md`) so
  the normative tables stay one click from the settled spec.
