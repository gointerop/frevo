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
