CHANGES_REQUESTED

Second review of `add-config-application-yaml`, at `071c2bb` (`f840f2a` plus
the register commit). I re-checked each finding of the first review against
upstream and against new probe runs of my own. I compiled the change's
`probe/Probe.java` and `probe/Gates.java` unchanged with the starter's
condition classes, `spring-boot-autoconfigure` 3.5.14 and, for the
failure-output runs, the starter's `logback.xml` with `logback-classic` /
`logback-core` 1.5.25 (the starter's `logback_version`, `pom.xml:25`).

## First-round findings

Eleven of the thirteen are resolved, and one more (13) is resolved for
scenarios but not for the register (finding 2 below). The THEN clauses quote
what I measured:

- **1:** resolved by `config.read.001`, `.008` and `config.gate.021`.
  `SCHEDULING_DISABLED=true`, `TRUE`/`yes` YAML read `"true"`;
  `scheduling-disabled` reads `null`.
- **2:** resolved by `config.placeholder.012`, `.013`, `config.gate.022` and
  design 3.6.
- **3:** resolved by `config.args.008`, `.009`.
- **4:** resolved by `config.source.024`, `.025`. I measured
  `a"b c"d` → `"ab cd"`, `e"f g"h` → `"ef gh"`, and the stderr order
  `NOTE: Picked up JDK_JAVA_OPTIONS` before `Picked up JAVA_TOOL_OPTIONS`.
- **5:** resolved by `config.profile.015` to `.018`. I measured `a,,b` →
  `Invalid empty profile`.
- **6:** resolved by `config.args.010` to `.012`. I measured
  `--hapi.fhir.default_page_size=3 -- --hapi.fhir.max_page_size=4` → `3` and
  `2147483647`.
- **7:** resolved by `config.source.026` to `.029`. I measured
  `Location 'file:./conf/*/*/application.yaml' cannot contain multiple wildcards`
  and `Config data location 'conf2/' does not exist`.
- **8:** resolved by `config.source.017`, `.030`, `config.oracle.001` and the
  narrowed `DEV-0031`.
- **9:** resolved by `config.unknown.008`, `config.key.012`, `config.error.010`
  and `keys.md` section 6.
- **10:** resolved by `config.gate.020`, `.024` and the exception in
  `config.unknown.004`.
- **11:** resolved by `config.gate.006`, `.023` and the corrected `DEBT-0022`.
- **12:** the stream (standard output), the failure-analysis block, and the
  `Application run failed` + exception-chain form all match my logback runs:
  - `graphql_enabled: "Y"` gives the exact block of `config.error.002`.
  - `SPRING_APPLICATION_JSON={bad` gives `ERROR  --- [           main] o.s.boot.SpringApplication … Application run failed`
    with no PID, as `config.error.008` states.
  - The `config.gate.005` NPE text matches.

  Finding 1 below covers what the rewrite introduced.
- **13:** resolved for the scenarios. The fixtures are in `probe/fixtures/`
  and are byte-correct (`Abc`, the trailing `\`, `x\ty`), and the
  `config.yaml.016` text is inline.

## Scenario IDs

232 IDs, unique, contiguous in every area. All 204 first-round IDs are kept,
and none is renumbered.

## New findings

1. **The "line for line" failure output is not HAPI's.**
   - **What changed.** `config.error.002` ("after the log lines of
     config.error.008"), `config.error.008` and `config.oracle.003` ("Frevo's
     standard output equals HAPI's line for line after masking … and removing
     Java stack-frame lines") now require all of standard output to equal
     HAPI's.
   - **The gap.** The HAPI side that defines "HAPI's" output is the probe. The
     probe is a `WebApplicationType.NONE` application with
     `setLogStartupInfo(false)` (`probe/Probe.java`), not the starter's
     `Application` (`Application.java:46`, `SpringApplication.run` with
     defaults, servlet web application).
   - **The evidence.** I changed only `setLogStartupInfo(true)` and ran
     `hapi.fhir.cr.cql.runtime.enable_validation: true` with logback. Standard
     output then starts with two lines that `config.error.008` does not list
     and the oracle never records:
     - `INFO … probe.Probe : Starting Probe using Java 25.0.1 with PID … (… started by … in …)`
     - `INFO … probe.Probe : No active profile set, falling back to 1 default profile: "default"`

     HAPI prints these as `Starting Application …` and the profile line.
   - **Beyond the probe.** In HAPI, a failure that surfaces at bean
     instantiation rather than in a condition is preceded by more lines.
     `CqlRuntimeProperties`, `CqlData` and every `cr`/`cdshooks` class are bean
     instantiations, because no starter condition binds them. HAPI's
     `ServletWebServerApplicationContext` starts Tomcat in `onRefresh`, so the
     embedded-Tomcat startup lines come first, plus whatever HAPI beans log
     before the failing one is created.
   - **Logger names.** HAPI's `WARN … cancelling refresh attempt` line comes
     from the servlet application context's logger. The probe's comes from
     `s.c.a.AnnotationConfigApplicationContext`. `config.error.008` says
     Frevo uses "the logger names the oracle records from the image", which is
     the probe's name, not HAPI's.
   - **The result.** As specified, Frevo would match the probe and differ from
     HAPI, and the line-for-line oracle cannot detect it.
   - **The fix.** Either:
     - (a) scope the verbatim requirement to the lines this change can own: the
       `ERROR` header line, the analysis block or the `Application run failed`
       exception chain, and the `WARN`/`INFO` lines with HAPI's servlet-context
       logger name. Then record the rest in a `DEV` entry (startup-info lines,
       Tomcat and HAPI component log lines, and their order), extending
       `DEV-0033` or adding a new entry, with the owner's approval. Make
       `config.oracle.003` compare only the scoped lines.
     - (b) or have the oracle's HAPI side run the starter's real `Application`
       in the image, and specify every line Frevo must print.

     Under either option, also state what Frevo prints for the two
     startup-info lines.

2. **`DEV-0033` is not actually registered, and its status contradicts the
   owner's ruling.**
   - **Fenced out.** In `docs/deviations.md`, the text appended from
     `register-entries.md` brought its code fences with it. Line 1384 opens a
     fence after `DEV-0032` and line 1414 closes it, so all of `DEV-0033` sits
     inside a fenced block.
   - **The checker skips it.** `harness/scripts/check-registries.sh` blanks
     fenced blocks (`strip_fences`, lines 76-81). It reports "29 entry(ies)"
     while the file has 31 `### DEV-` headings, so it never checks `DEV-0033`.
   - **Status.** Its **Reviewed** field still reads "proposed … **needs the
     owner's approval**". `config.error.009`, `spec.md:95`, `proposal.md:69`,
     `:222`, `design.md` 6a, `register-entries.md:13` and `tasks.md:170` all
     still say "proposed". The coordinator reports owner approval on
     2026-11-22. Constitution section 1 makes `docs/deviations.md` the only
     exception mechanism, so the entry has to be parseable and its approval
     has to be recorded.
   - **The fix.** Remove the two stray fences. Record the owner's approval and
     its date in `DEV-0033`'s **Reviewed** field. Drop "proposed" everywhere it
     appears. Re-run `check-registries.sh` and confirm it counts the entry. If
     finding 1 is resolved by widening `DEV-0033`, make that edit in the same
     change, with the owner's approval.
