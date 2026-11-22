CHANGES_REQUESTED

Fifth review of `add-config-application-yaml`, at `5c049ba` (the delta
rewrite) over `64d2e4e` (the re-record with two runners). I checked the 20
rewritten scenarios against `harness/config-roundtrip/hapi-8.12.0-results.json`.
I also ran cases of my own choosing on the unchanged pinned image
`hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`, through its ENTRYPOINT and
its own `ca.uhn.fhir.jpa.starter.Application`, with only the case's files
copied to `/app`. I used fresh containers and removed them afterwards.

## The fourth review's findings

1. **Refresh-time failure output: resolved.**
   - I grouped the record's 80 failing cases by `WARN` exception, by whether
     an `INFO` line appears, by `ERROR` form and by application-name field.
     There are exactly nine groups. They match config.error.008's table row
     for row:
     - `PartitionModeConfigurer`: 38 cases, no `INFO`, analysis form.
     - `FhirServerConfigDstu2`: 6 cases, no `INFO`, run-failed form, no
       application name.
     - `ElasticsearchBootSvcImpl`: 6 cases, `INFO`, analysis form.
     - `WebAppFilesConfigurer`, `CustomContentFilesConfigurer` and
       `McpServerConfig`: one case each, `INFO`, run-failed form.
     - The CQL bean wrapper: 4 cases, `INFO`, analysis form.
     - The pre-context failures: 18 in run-failed form and 5 in analysis
       form, 23 in all, with neither `WARN` nor `INFO`. They match the list
       in config.error.008.
   - Each of these quotes matches the record verbatim:
     - config.gate.005's two exception lines;
     - config.placeholder.011's three-line chain, and its five lenient-binding
       blocks (origin columns 24, 32, 53, 38 and 24, and the
       `DATABASE`/`FILESYSTEM` action);
     - config.placeholder.007's full circular-reference reason;
     - config.unknown.005's and .008's `cqlProperties`, `cqlData` and
       `cqlTerminologyProperties` wrappers, the `$$SpringCGLIB$$0`
       descriptions, and `CqlRuntimeProperties` without the suffix;
     - config.gate.024's `ElasticsearchBootSvcImpl` line, followed by
       `INFO`;
     - config.source.016's action line;
     - config.source.020's two causes.
   - **Live image.** I ran config.placeholder.011-g, config.unknown.008-c,
     config.gate.024-h, config.source.010 (with its `args`) and
     config.args.009-e (with its `JAVA_TOOL_OPTIONS`). Each config-related
     line, as `run.py` `config_lines` extracts it, equals the record. The
     only difference is a trailing blank line, which `run.py` pops before it
     records. The `WARN` and `INFO` logger fields are
     `ConfigServletWebServerApplicationContext` and
     `.s.b.a.l.ConditionEvaluationReportLogger`, as the delta states.
2. **Jackson location line and the tab-led frame definition: resolved in the
   delta.**
   - config.args.007 and .009 quote the ` at [Source: …]` line, and the live
     image prints it (`column: 2` for both).
   - config.error.009 and config.oracle.003 define a frame as tab-led, with
     `... N common frames omitted`.
   - `run.py` `FRAME` follows that definition.
   - `tests/common/mod.rs` `is_frame` still accepts any leading whitespace.
     Task 3 owes that fix, which the hook blocked (`tasks/blockers.md`,
     items 1 to 3). That is acceptable: task 3 precedes any implementation.
3. **`DEV-0033` text: resolved, judged on `register-entries.md` only.**
   - The replacement text is consistent with the delta:
     - frames are tab-led, including `common frames omitted`;
     - the Jackson line is kept;
     - the logger field is compared;
     - the `Bindable` mask is kept;
     - the `WARN`/`INFO` kinds are those of config.error.008.
   - It also agrees with the record:
     - `logback_status_lines` is (57, 2) in 396 runs and (57, 0) in 23;
     - config.gate.005 has 35 frame lines.

What the delta says is still owed is acceptable as stated in config.error.008,
proposal.md ("Not in the record") and tasks.md task 2. Task 2 records the
following before task 3 starts:
- a binding failure in `CrProperties`, `CareGapsProperties`,
  `CqlCompilerProperties` and `CdsHooksProperties`;
- the ordering of several refresh-time failures;
- comparing the logger field, and dropping the `Error creating bean with
  name` prefix rule.

Finding 2 below is one place where the delta contradicts that.

## Findings

1. **The oracle does not record HAPI's result for any case the probe starts,
   and on the live image most of the ones I sampled fail to start.** No
   artifact says so.
   - config.oracle.001 records a case "the probe does not fail" from the
     probe. `runner_disagreements` covers only one direction: the probe fails
     and the real `Application` starts. The reverse is never measured, and
     neither design.md section 7 nor the delta notes it.
   - I ran eight probe-passing cases (`exit: 0` in the record) on the
     unchanged image's own `Application`. Six of them fail to start:

     | Case | HAPI's real `Application` |
     |---|---|
     | config.unknown.004 | `Invalid value 'not-a-number' for configuration property 'server.port'` (failure analysis) |
     | config.coerce.026 | `Parameter 0 of method restfulServer in ca.uhn.fhir.jpa.starter.common.StarterJpaConfig required a bean of type 'ca.uhn.fhir.context.FhirContext' that could not be found.` (every `fhir-*` gate off, as the scenario says) |
     | config.collection.006 | `Interceptor class was not found on classpath: a.B` |
     | config.key.012 | `packageInstaller`: `thePackageId must not be blank` |
     | config.gate.024-b | `HAPI-1175: Failed to create document index`, `Connection refused` |
     | all-keys.yaml (config.key.005) | `jpaStorageSettings`: `Invalid path for auto-version reference at path: auto-version-reference-at-paths-a` |

     config.coerce.017-g and config.gate.006-d started the server.
   - **Most of these are probably out of scope**, and the record still
     claims to be HAPI 8.12.0's result:
     - `server.port` is listed in proposal.md "Out of scope", with its
       failure.
     - `custom_interceptor_classes` is listed there too.
     - The package, Elasticsearch-connection and auto-version failures are
       "what each key does".
   - **The problem is that the record labels them "HAPI 8.12.0 results" with
     `exit: 0`.** Nothing guarantees that the other 331 probe-passing cases
     contain no failure this change owns. For example:
     - a binding failure in a bean the probe does not bind;
     - a condition the component scan evaluates that the probe's evaluation
       does not.

     Such a failure would be an unexplained diff that no gate can see. That
     is the failure mode of the fourth review's finding 1, in the other
     direction.
   - config.coerce.026 shows that a scenario's own `THEN` ("every `fhir-*`
     gate is off") leads to a startup failure. The delta neither states it
     nor assigns it to a later change.
   - **Needed:**
     - Run every probe-passing case on the `application` runner too. With
       `APPLICATION_TIMEOUT` it is only wall time. Record that run's outcome
       beside the probe's (for example `application_exit` and its
       config-related lines, or `started: true`).
     - List the cases where the real `Application` fails in a record member,
       the reverse of `runner_disagreements`.
     - In config.oracle.001, state the meaning of a probe-recorded `exit: 0`:
       the configuration layer accepts the case, and it says nothing about
       whether HAPI starts.
     - Classify every such case in the delta:
       - **Out of scope**, with the failing bean and the change that owes the
         failure (`add-server-skeleton` for `server.port`, the missing
         `FhirContext` and the interceptor classes; the package, storage and
         Elasticsearch consumers for the rest).
       - **In scope**: a scenario is rewritten to HAPI's failure, as was done
         for the fourth review's finding 1.
     - The 6 cases above are a measured start.

2. **config.read.007 sends the five other reads out of scope, but every other
   artifact says they are owed before task 3.**
   - `specs/config/spec.md:2012-2015` says `${nope}` in `fhir_version`,
     `cr.enabled`, `cdshooks.enabled`, `custom-bean-packages` and
     `scheduling_disabled` is "not recorded with a placeholder and are out of
     this scenario (proposal.md, 'Out of scope')".
   - proposal.md's "Out of scope" table has no such row. Its "Not in the
     record" row (`proposal.md:314`), tasks.md task 2 (lines 76-79) and the
     config.error.008 note all say these cases are owed by task 2 and stated
     in the delta before task 3.
   - As written, an implementer and the oracle-builder may read
     config.read.007 as excluding these reads, and no scenario would then
     receive their record.
   - This is a real behaviour, not a wording issue. `fhir_version` and
     `cr.enabled` are also bound by `AppProperties` and `CrProperties`, so
     HAPI may hit the lenient-binding failure first, as it does for the five
     keys of config.placeholder.011. `custom-bean-packages` and
     `scheduling_disabled` have no bound counterpart and may fail strictly or
     not at all.
   - **Needed:** make config.read.007 say these five are owed by task 2 and
     will be stated from the record before task 3, like config.error.008,
     instead of out of scope.

## Notes (not blocking)

- **config.oracle.003 against the oracle's current output.** config.oracle.003
  now says the `WARN`/`INFO` logger field is compared, but `run.py` `mask`
  still writes `<logger>`, and so does `tests/common/mod.rs` `masked`. Task 2
  explicitly owes this, and design.md section 7 says so. I confirmed the
  logger values on the live image, so the scenario is right.
- **Scenario IDs.** The count is still 232. No ID was added, removed or
  renumbered by `5c049ba`.

---

## History: fourth review (CHANGES_REQUESTED at `831c5ba`)

CHANGES_REQUESTED

Fourth review of `add-config-application-yaml`, at `ad59a5c` (`c43d5b6`, the
O-1 to O-13 rewrite, merged with `main`). I checked the rewritten scenarios
against `harness/config-roundtrip/diagnostic-spring-core-6.2.19.json` and the
case corpus. I also ran the cases' own files, environment and arguments
against the unchanged pinned image `hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…`,
through its own ENTRYPOINT and its real `ca.uhn.fhir.jpa.starter.Application`,
with nothing but the case's files mounted at `/app` (and `root/` at `/`).

Most of the rewrite matches the record exactly:

- config.yaml.003 and .016: indentation, carets and the trailing blank line
  match. I confirmed .003 on the live image too.
- config.env.004 (20 / 15 / 20) and the `hapi_fhir_ips_enabled` read.
- config.source.007, .009, .010 (with the `-p` profile run: 33 / 34), .011
  (44 / 20), .013 (32) and config.key.001: both runs.
- config.collection.002's `Bindable@…` line; config.gate.002's binding
  failure; config.key.011's 220 rows (209 - 3 + 14);
  config.relaxed.007/-b (3 both times).
- The enum `Action:` block added to config.error.002. It is sorted:
  `DSTU2_1` before `DSTU2_HL7ORG`, against declaration order, and
  `ALPHANUMERIC, ANY, NOT_ALLOWED` and `DATABASE, FILESYSTEM` likewise.

`scenario-coverage.sh --change add-config-application-yaml` reports 232 in
scope, 232 covered, 0 unresolved. IDs are unchanged and unique. Design section
11's three contracts are coherent with the tests: `user.dir` takes the real
working directory, and absolute paths are re-rooted only for resolution.

The findings below are cases where the record itself is not what HAPI 8.12.0
prints. The tests follow the record, so these would pass every gate and still
be parity bugs.

1. **The oracle's HAPI side is not HAPI's application context, and for every
   failure during context refresh the live image prints different
   config-related lines from those the record and the delta quote.**

   The probe (`WebApplicationType.NONE`, the nine classes enabled directly,
   conditions evaluated one by one on annotation metadata) does not reproduce
   the starter's component scan. In the real `Application`,
   `common/PartitionModeConfigurer`'s condition binds `AppProperties` while
   candidate components are read. So binding and condition failures surface
   wrapped in `BeanDefinitionStoreException`. Measured on the live image with
   the corpus's own case files:

   a. **`AppProperties` binding failures**: config.dump.006, config.error.002,
      config.coerce.018, .026-b, .027, config.collection.002,
      config.gate.002-h/-i, and every other `hapi.fhir.*` bind failure that
      inherits config.error.008.
      - The `WARN` line's exception is
        `org.springframework.beans.factory.BeanDefinitionStoreException: Failed to read candidate component class: URL [jar:file:/app/main.war!/WEB-INF/classes!/ca/uhn/fhir/jpa/starter/common/PartitionModeConfigurer.class]`.
        The record has `org.springframework.boot.context.properties.bind.BindException: Failed to bind properties under '…'`.
      - There is **no** `ConditionEvaluationReportLogger` `INFO` line (0 in
        every such run). config.error.008 requires it "for a binding failure".
      - The failure-analysis block itself is identical.

   b. **config.gate.005, and every scenario that "fails as config.gate.005
      does"**: config.source.007, .009, .010, .011, .013, config.key.001 and
      config.env.014.
      - The chain is
        `org.springframework.beans.factory.BeanDefinitionStoreException: Failed to read candidate component class: URL [jar:file:/app/main.war!/WEB-INF/classes!/ca/uhn/fhir/jpa/starter/common/FhirServerConfigDstu2.class]`
        and then `Caused by: java.lang.NullPointerException: Cannot invoke "String.toUpperCase()" …`.
      - The `WARN` line carries the same `BeanDefinitionStoreException`.
      - config.gate.005 says the exception line is the
        `NullPointerException`, and the record has only that line.

   c. **config.placeholder.011**:
      - For `app_content_path`, `custom_content_path` and
        `spring.ai.mcp.server.enabled`, the chain starts with the
        `BeanDefinitionStoreException` naming the class file
        (`…/web/WebAppFilesConfigurer.class`, `…/web/CustomContentFilesConfigurer.class`,
        `…/mcp/McpServerConfig.class`). Then come
        `Caused by: java.lang.IllegalStateException: Error processing condition on …`
        and `Caused by: …PlaceholderResolutionException…`. The `WARN` line
        carries the wrapper. The `INFO` line is present.
      - For `mdm_enabled`, `binary_storage_mode`,
        `enable_repository_validating_interceptor`,
        `ig_runtime_upload_enabled` and `ips_enabled`, **no gate is
        reached.** HAPI fails with config.error.002's failure analysis of the
        lenient binding, for example `Failed to bind properties under 'hapi.fhir.mdm-enabled' to java.lang.Boolean:` …
        `Reason: failed to convert java.lang.String to java.lang.Boolean (caused by java.lang.IllegalArgumentException: Invalid boolean value '${nope}')`.
        For binary storage it is
        `No enum constant ca.uhn.fhir.jpa.starter.AppProperties.BinaryStorageMode.${nope}`
        with the enum `Action:` list. The `WARN` line is the
        `PartitionModeConfigurer` wrapper, and there is no `INFO` line.
      - So the O-7 caveat ("not measured in HAPI's real context") is now
        measured: five of the eight `THEN` names are wrong in kind, not just
        in site.

   d. **config.unknown.008**: the description line names the CGLIB proxy
      class.
      - `Failed to bind properties under 'hapi.fhir.cr.cql.data' to ca.uhn.fhir.jpa.starter.cr.CqlData$$SpringCGLIB$$0:`
      - for `-b`, `… 'hapi.fhir.cr.cql' to ca.uhn.fhir.jpa.starter.cr.CqlProperties$$SpringCGLIB$$0:`
      - for `-c`, `… CqlTerminologyProperties$$SpringCGLIB$$0:`

      The record and the delta have the bare class names. These lines are
      compared verbatim.

   e. **config.unknown.005 and .008, the `WARN` wrapper**: the live bean
      names are `'cqlProperties'` (unknown.005 and .008-b), `'cqlData'` and
      `'cqlTerminologyProperties'`. The probe's names are
      `'<prefix>-<class>'`. The `Error creating bean with name` prefix rule
      of config.oracle.003 hides the difference. But the scenarios tell Frevo
      to write the probe's wording, which no HAPI run prints. With the live
      output available, Frevo should write HAPI's names.

   Failures before the context exists match the live image. I checked
   config.source.008, config.profile.015, config.error.007 (`--=x`),
   config.yaml.003 and config.args.007. Only refresh-time failures are
   affected.

   **Needed:**
   - The HAPI side of every failing case must be the image's real
     `Application`. Run the unchanged image with the case and capture its
     output. The oracle already uses the ENTRYPOINT, so only `loader.main`
     changes for failing cases. Then re-record.
   - Rewrite config.error.008 (the `WARN` exception, and when the `INFO` line
     appears), config.gate.005, config.placeholder.011, config.unknown.005
     and config.unknown.008 to that record.
   - Alternatively, take a `DEV` entry for the difference, with the owner.

   This reopens the round-2 statement that the `WARN`/`INFO` lines before the
   `graphql_enabled: "Y"` block agree with HAPI: on the live image they do
   not.

   Reproduce, for example:

   ```
   docker run --rm -v <case>/files/application.yaml:/app/application.yaml:ro hapiproject/hapi:v8.12.0-2@sha256:0ff86aac…
   ```

2. **A Jackson message line is dropped as if it were a stack frame
   (config.args.007, .009).**
   - On the live image, `SPRING_APPLICATION_JSON={bad` prints, after the two
     quoted lines, a third message line:

     ```
     ` at [Source: REDACTED (`StreamReadFeature.INCLUDE_SOURCE_IN_LOCATION` disabled); line: 1, column: 2]`
     ```

     This is Jackson's location suffix (`jackson-core-2.21.6` in
     `WEB-INF/lib`). It has one leading space, not a tab. The unquoted
     `-Dspring.application.json={"hapi":…}` (case `config.args.009-e`)
     prints the same line, also `column: 2`.
   - `run.py`'s `FRAME = re.compile(r"^\s+at |…")` deletes it from the
     record, and `crates/frevo-config/tests/common/mod.rs` `is_frame` does
     the same on the Frevo side. So the tests cannot see the loss.
   - config.args.007 and .009 nevertheless say the exception lines are
     "exactly" the two lines. config.error.007 requires every exception's
     message lines verbatim, and config.error.009 defines frames as `\tat …`
     and `\t... N more`.
   - **Needed:**
     - Add the location line to both scenarios.
     - Make config.oracle.003 and config.error.009 define a frame as
       tab-led: `\tat …`, `\t... N more` and `\t... N common frames omitted`.
       The last form is what the live image prints after the Jackson chain,
       and the delta does not name it.
     - The oracle and tests then follow that definition, and the record is
       re-made.

3. **`DEV-0033`'s extension to logback status lines is cited but not
   registered.**
   - config.dump.006, config.error.007 and config.oracle.003 cite `DEV-0033`
     for Frevo writing no status lines.
   - `docs/deviations.md:1385` still has the pre-extension title and scope,
     with no "status line" text. `register-entries.md` says the entry is to
     be replaced.
   - Until that is applied (registers may be appended from any branch), the
     difference is undocumented.

## Notes (not blocking)

- **The versionless reading rule.** The rule in the notation comment says the
  oracle's case for such a scenario is written with
  `--hapi.fhir.fhir_version=R4` added. That is not true for config.gate.012-b,
  config.gate.016-b, config.key.002 and config.source.026-c, whose cases set
  `hapi.fhir.fhir_version: R4` inside `x.yaml`. The outcome is the same for
  those scenarios.
- **Status lines are not on both streams on every run.** config.dump.006 and
  design 6a say HAPI prints them on every run on both streams. In the record,
  23 of 419 runs (the environment-preparation failures: config.args.007,
  config.source.008, config.profile.015, config.yaml.003, …) have none on
  standard error.
- **config.env.014's "`/configs/x.yaml` is loaded" cannot be observed.** Both
  sides fail as config.gate.005 does.
- **Stale or owed items outside the delta.**
  - `harness/config-roundtrip/run.py` still has
    `EXPECTED_JARS["spring-core"] = "6.2.18"`. That is task 1, unchecked.
  - `proposal.md:125` says the 6.2.19 re-pin is owed, but #40 has merged it.

---

## History: third review (APPROVED at `98fc5aa`)

APPROVED

Third review of `add-config-application-yaml`, at `d7ee1bf` (`042be7c` plus
the register commits `f81cfd5` and `d7ee1bf`).

### Round-2 findings

1. **Failure output scope: resolved.**
   - `config.error.002`, `.008`, `.009` and `config.oracle.003` now require
     verbatim parity only for the config-related lines:
     - the `ERROR` line and the analysis block or exception chain after it;
     - the `WARN … cancelling refresh attempt` line;
     - the `ConditionEvaluationReportLogger` `INFO` line.
   - Startup-info lines, Tomcat, servlet-context and component lines, and Java
     stack frames are explicitly not reproduced. That is now `DEV-0033`'s
     scope, and the owner approved it on 2026-11-22.
   - The oracle compares the `WARN`/`INFO` lines without their logger field.
     The spec fixes the name `ConfigServletWebServerApplicationContext`, as the
     owner ruled. That name is what logback's `%-40.40logger{39}` gives for
     Spring Boot's `AnnotationConfigServletWebServerApplicationContext`: the
     class name alone is 50 characters, so the left truncation keeps the last
     40.
   - My logback runs (starter `logback.xml`, logback 1.5.25) agree with every
     remaining THEN clause:
     - the `graphql_enabled: "Y"` analysis block;
     - the `WARN` and `INFO` lines before it;
     - `ERROR  --- [           main] o.s.boot.SpringApplication … Application run failed`
       with no PID for invalid JSON;
     - the `NullPointerException` of `config.gate.005`, with no `INFO` line;
     - standard error empty apart from the JVM-option notices.
2. **`DEV-0033` registration: resolved.**
   - The stray fences are gone from `docs/deviations.md`.
   - `check-registries.sh` reports "30 entry(ies)" and "registers agree".
   - **Reviewed** records the owner's approval.
   - No `proposed` or `needs the owner` wording about `DEV-0033` is left in the
     change's artifacts.

### Regression check

- **Scenario IDs.** Still 232, the same ID set as round 2: unique, contiguous
  per area, none renumbered or removed.
- **First-round resolutions.** Unchanged by `042be7c`. Its diff touches only
  the failure-output scenarios, `DEV-0033`, and the matching text in
  `proposal.md`, `spec.md`, `design.md`, `tasks.md` and
  `register-entries.md`.
- **Constitution.** No configuration flag is added. PostgreSQL remains the
  only runtime service. The one new dependency (`yaml-rust2`) has its ADR. The
  performance budget is stated. Every difference from HAPI cites
  `DEV-0031`, `-0032` or `-0033`, and every reproduced hazard cites
  `DEBT-0022` to `-0026`. No secrets or PHI are present: the probe output's
  personal paths are redacted.

No findings.
