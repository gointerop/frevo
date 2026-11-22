CHANGES_REQUESTED

Review of `add-config-application-yaml` against `hapi-fhir-jpaserver-starter@377e6eba`,
`hapi-fhir@7ab4c7c7`, `clinical-reasoning@ccba474a`, and Spring Boot 3.5.14 /
Spring Framework 6.2.18 / SnakeYAML 2.4. I re-ran the proposal's own probe
(`probe/Probe.java`, compiled unchanged with the nine starter classes against the
class path in `probe/README.md`, JDK 25, Windows). Every "probe:" quotation
below comes from that re-run. Where a finding is about a Spring class that is
not yet in `reference/`, the evidence is the probe output.

Checked and found sound: the 209-row disposition table (163 + 7 + 1 + 28 + 7 + 3;
the "never read" set matches a getter-usage scan of the starter); the class
defaults in `keys.md` sections 1-2 (checked against `AppProperties.java:29-144`,
`:895-1370` and the eight other classes); the YAML 1.1 claims I spot-checked
(`09`, `08540` and `0o17` stay text, `010` becomes `"8"`, `1:20` becomes `"80"`,
`1e3` becomes `"1000.0"`); `forVersionString` (`FhirVersionEnum.java:283-315`);
the `@ConditionalOnProperty` predicates; profile include order; scenario IDs
(204, unique, contiguous per area, none collide with `openspec/specs/`);
registers (`check-registries.sh`: "registers agree"); ADR 0007; the performance
budget; the public-repository rules.

The findings below are behaviours that are missing, contradict HAPI, or cannot
be tested as written.

1. **The `scheduling_disabled` named read is missing.** HAPI reads it by exact
   name from the Spring `Environment`:
   `hapi-fhir/hapi-fhir-jpa/src/main/java/ca/uhn/fhir/jpa/sched/BaseSchedulerServiceImpl.java:67`
   (`SCHEDULING_DISABLED = "scheduling_disabled"`) and `:319`
   (`"true".equals(myEnvironment.getProperty(SCHEDULING_DISABLED))`, which turns
   off every scheduled job), and
   `hapi-fhir-jpaserver-subscription/src/main/java/ca/uhn/fhir/jpa/subscription/submit/config/SubscriptionMatcherInterceptorConfig.java:47`
   (the same read swaps in `SynchronousSubscriptionMatcherInterceptor`). Those
   are the only two `Environment` readers in the HAPI and clinical-reasoning main
   sources the starter ships. `config.read.001` says `reads` has "exactly"
   twelve members, so as written it forbids the thirteenth.
   - Fix: add `scheduling_disabled` to `config.read.001`.
   - Fix: add a gate whose predicate is case-sensitive `"true".equals`, so
     `TRUE` and `yes` are off.
   - Fix: add scenarios for `SCHEDULING_DISABLED=true` (the read's
     environment-variable variants of `config.read.004`), `scheduling_disabled: TRUE`
     (off), and the YAML boolean `yes` (the read is `"true"`, so the gate is on).
   - Fix: add a `keys.md` row wiring the read to `add-jobs-engine` and
     `add-subscriptions-resthook`.

2. **The JVM's own system properties are missing from source 3.**
   `design.md` 3.1 builds the system-properties source only from `-D` tokens of
   `JAVA_TOOL_OPTIONS` / `JDK_JAVA_OPTIONS`. In HAPI, that source also holds:
   - every JVM standard property (`user.dir`, `user.home`, `user.name`,
     `java.io.tmpdir`, `os.name`, `file.separator`, `java.version`, and others);
   - `PID`, which Spring's `LoggingApplicationListener` sets before the beans
     bind;
   - `loader.path`, which the image's own ENTRYPOINT sets
     (`hapi-fhir-jpaserver-starter/Dockerfile:51`).

   They resolve in placeholders. Probe:
   `server_address: "${java.io.tmpdir}|${user.dir}|${PID:nopid}|${os.name}|${file.separator}"`
   bound to `"C:\Users\…\Temp\|C:\…\c\sp|9320|Windows 11|\"`. Frevo's lenient
   binding would keep `${java.io.tmpdir}` literal, so a value such as
   `binary_storage_filesystem_base_directory: ${java.io.tmpdir}/bin` silently
   differs.

   These names are also enumerable, so they take part in the raw prefix scan of
   `FhirTesterConfigCondition` (`common/FhirTesterConfigCondition.java:13`).
   - Fix: specify the set Frevo provides and its values in the pinned
     distroless image (`user.dir=/app`, `java.io.tmpdir=/tmp`, and so on, as the
     oracle records them), and add `config.placeholder.*` scenarios for them.
   - Fix: for any name Frevo cannot reproduce, such as `java.version` or `PID`
     semantics, add a `DEV` entry.

3. **`spring.application.json` as an argument or system property is not
   covered.** Spring Boot's `SpringApplicationJsonEnvironmentPostProcessor` looks
   for both `spring.application.json` and `SPRING_APPLICATION_JSON` in every
   source. Probe: `--spring.application.json={"hapi":{"fhir":{"default_page_size":7}}}`
   gave `default_page_size = 7`. `config.args.006`/`.007` and design 3.1 cover
   only the environment variable.
   - Fix: add scenarios for the argument form and the `-Dspring.application.json`
     form, including where each ranks and its `Origin:` text.

4. **JVM option-variable quoting is mis-specified.** `config.source.024` says
   tokens are "separated by whitespace", but the JVM honours quotes in both
   variables. Probe:
   - `JAVA_TOOL_OPTIONS='-Dhapi.fhir.server_address="a b" -Dhapi.fhir.custom_content_path='"'"'c d'"'"''`
     bound `server_address = "a b"` and `custom_content_path = "c d"`.
   - `JDK_JAVA_OPTIONS='-Dhapi.fhir.server_address="x y"'` bound `"x y"`.

   A whitespace-only tokenizer gives `"\"a"` and drops the rest. Fix: specify
   the JVM's quote handling (single and double quotes, quotes removed) and add
   a scenario per variable.

5. **Several profile failure paths are not covered.** Probe:
   - `SPRING_PROFILES_ACTIVE="a b"` fails with
     `Profile 'a b' must contain a letter, digit or allowed char ('-', '_', '.', '+', '@')`.
   - `-x` fails with `Invalid profile property value found in Environment under 'spring.profiles.active'`
     caused by `Profile '-x' must start and end with a letter or digit`.
   - `a/b` and `x!` fail the same way.
   - `--spring.profiles.validate=false` makes `-x` start, with
     `PROFILES = [-x]`. That is a key the change neither binds nor lists.
   - The legacy key `spring.profiles: dev` in a document fails with
     `Property 'spring.profiles' imported from location 'file [application.yaml]' is invalid and should be replaced with 'spring.config.activate.on-profile'`.
     Old Boot 2.3-style multi-document files hit this.
   - `spring.profiles.include` in a profile-specific file fails with
     `... is invalid in a profile specific resource`. `config.profile.012`
     covers only `spring.profiles.active`.

   Fix: add `config.profile.*` scenarios for each of these. Cover the
   `.default` and `.include` siblings of `.012` explicitly.

6. **Two command-line parsing rules are missing.** `config.args.003` says other
   arguments are ignored, but Spring's `SimpleCommandLineArgsParser` has two
   rules it does not cover. Probe:
   - `--=x` fails with `java.lang.IllegalArgumentException: Invalid argument syntax: --=x`, exit 1.
   - `-- --hapi.fhir.default_page_size=3` starts with `default_page_size = 20`,
     because `--` ends option parsing.

   Fix: add both as `config.args.*` scenarios.

7. **Some location syntax is not covered.** Probe, with
   `SPRING_PROFILES_ACTIVE=p` and `SPRING_CONFIG_IMPORT=conf/extra.yaml`, where
   `extra.yaml` sets 5 and `extra-p.yaml` sets 6: `default_page_size = 6`.
   That is, an explicitly named file location or import also loads its
   `<file>-<profile>.<ext>` variant. Design 3.2 says "for every location", but
   no scenario pins this for a file location or import. The starter's Helm chart
   form (`SPRING_CONFIG_IMPORT=/app/config/application-extra.yaml`) plus a
   profile hits exactly this.

   Also uncovered:
   - the `[.ext]` extension hint (probe: `SPRING_CONFIG_IMPORT='conf/noext[.yaml]'`
     loaded `max_page_size = 7`);
   - a directory import (`conf/`);
   - wildcard `*/` in user-supplied locations, where `config.source.004` covers
     only the default `./config/*/`.

   Fix: add `config.source.*` scenarios for each.

8. **The default `classpath:` search difference is silent, and `DEV-0031` does
   not cover it.** The image starts with
   `-Dloader.path=main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes`
   (`Dockerfile:51`). Spring's *default* locations `optional:classpath:/` and
   `optional:classpath:/config/` therefore find:
   - `/app/extra-classes/config/application.yaml`;
   - `/app/extra-classes/application.properties` or `.yml`, which outrank the
     packaged `.yaml` per `config.source.005`;
   - `application-<profile>.*` for any profile other than `cds` and `elastic`;
   - any such resource inside a `WEB-INF/lib` jar.

   An operator who mounts a file in `/app/extra-classes/` (the reason that
   loader-path entry exists) gets it loaded with no flag. `DEV-0031` covers only
   an explicitly named `classpath:` location, which "fails loudly". Its rationale
   ("turns every case where the two servers would differ into a startup
   error") is untrue for the default locations: Frevo silently loads nothing.
   No scenario states Frevo's behaviour for the default `classpath:/config/`.
   The oracle would not catch it either: task 1 builds the HAPI side's class path
   from `WEB-INF/lib/*` plus `WEB-INF/classes`, without `/app/extra-classes` and
   without `PropertiesLauncher`.
   - Fix: add scenarios for the default classpath locations with files under
     `/app/extra-classes/`.
   - Fix: either support that directory or extend `DEV-0031` to state the
     silent case honestly. Under the loud-failure ruling, a better option is to
     fail when `/app/extra-classes/` holds an application file.
   - Fix: make the oracle use the image's real launcher class path.
   - Fix: have task 1 assert that no `WEB-INF/lib` jar carries
     `application*.{properties,xml,yml,yaml}` or `config/application*`.

9. **Several setter-less and unlisted bean properties are unspecified.** Design
   section 8 calls `getRetrieveSettings`, `getTerminologySettings` and
   `getPackageContents` "derived getters … not reported" and says nothing about
   binding. They are not inert. Probe:
   - `hapi.fhir.cr.cql.data.retrieve_settings.profile_mode: ENFORCED` (a *valid*
     value) fails with
     `Failed to bind properties under 'hapi.fhir.cr.cql.data' to ca.uhn.fhir.jpa.starter.cr.CqlData`
     / `Reason: java.lang.IllegalStateException: No setter found for property: retrieve-settings`.
     The getter (`cr/CqlData.java:39`) returns a new object each call.
   - `hapi.fhir.cr.cql.retrieve_settings.profile_mode: OFF` fails the same way
     for `CqlProperties` (`cr/CqlProperties.java:70`).
   - `hapi.fhir.cr.cql.terminology.terminology_settings.code_lookup_mode: AUTO`
     fails with `No setter found for property: terminology-settings`
     (`cr/CqlTerminologyProperties.java:35`).
   - `PackageInstallationSpec.setPackageContents(byte[])`
     (`hapi-fhir-storage/.../PackageInstallationSpec.java:210`) is a real,
     fluent, bindable property. `implementationguides.x.packageContents: "65,66"`
     binds bytes `[65, 66]`, and `package_contents: abc` fails with
     `Property: hapi.fhir.implementationguides.x.package-contents` /
     `Reason: failed to convert java.lang.String to byte (caused by java.lang.NumberFormatException: For input string: "abc")`.
     HAPI acts on it: `JpaPackageCache.java:586,596` and
     `AdditionalResourcesParser.java:51` install from the bytes instead of
     fetching. It is absent from `config.key.009`, the dump and `keys.md`
     section 6.
   - Fix: add `config.unknown.*` scenarios for the three setter-less
     properties.
   - Fix: add `package-contents` to `config.key.009`, `keys.md` section 6 and
     the dump, or record a `DEV` if Frevo will not bind it.
   - Fix: add all four to the design 5.1 quirk table.

10. **The `elasticsearch` gate needs a binder this change says it does not
    have.** `ElasticConfigCondition` (`elastic/ElasticConfigCondition.java:13`)
    binds Spring Boot's `ElasticsearchProperties`, not the raw keys. This
    contradicts `config.unknown.004` ("no type check is applied to `spring.*`").
    Measured with `spring-boot-autoconfigure-3.5.14.jar` and
    `Binder.get(env).bind("spring.elasticsearch", ElasticsearchProperties.class)`:
    - `spring.elasticsearch.uris=http://x` is bound, so the gate is on.
    - `spring.elasticsearch.foo=x` gives `null`, so the gate is off.
    - `spring.elasticsearch.connection-timeout=5s` is bound.
    - `connection-timeout=abc` throws `BindException: Failed to bind properties under 'spring.elasticsearch.connection-timeout' to java.time.Duration`,
      so HAPI fails to start even with Elasticsearch unused.
    - `spring.elasticsearch=` (empty scalar) throws `No converter found … to type [org.springframework.boot.autoconfigure.elasticsearch.ElasticsearchProperties]`.

    `config.gate.020`'s "any `spring.elasticsearch.*` property binds" cannot be
    implemented without `ElasticsearchProperties`' property list and types
    (`uris`, `username`, `password`, `connection-timeout`, `socket-timeout`,
    `socket-keep-alive`, `path-prefix`, `restclient.sniffer.*`,
    `restclient.ssl.bundle`), including `Duration` conversion.
    - Fix: add a tenth descriptor (or a scoped one for this gate) and the
      `Duration` rules.
    - Fix: add scenarios for an unknown sub-key, an invalid `Duration` and an
      empty scalar.
    - Fix: carve the exception out of `config.unknown.004`.

11. **`cr.enabled: yes` is described wrongly in four places.**
    `config.gate.006`'s AND clause, `proposal.md:56`, `DEBT-0022` ("`cr.enabled: yes`
    binding `true` while `Boolean.parseBoolean` turns the `cr` gate off") and
    the `register-entries.md` copy all make this claim. An unquoted YAML `yes`
    is a YAML 1.1 boolean, so the read is `"true"`, as `config.read.006` itself
    states for `mdm_enabled: yes`, and the gate is **on**. Probe:
    - `cr: { enabled: yes }` gives `ENV[hapi.fhir.cr.enabled] = true` and
      `Cr.getEnabled = Boolean:true`.
    - Only the quoted `enabled: "yes"` gives `ENV[...] = yes`, so the gate is off.

    The scenarios contradict each other, and an implementer cannot satisfy both.
    Fix: change the examples to the quoted `"yes"` (or an environment variable /
    `.properties` value), and add an explicit scenario with the unquoted `yes`
    and the gate on.

12. **The failure output's stream and form are not HAPI's.**
    - **Stream.** `config.error.002` says standard error carries the block
      "line for line … verbatim", and `config.dump.006` says standard output is
      empty. But the starter logs through logback
      (`src/main/resources/logback.xml:3-6` includes Spring's
      `console-appender.xml`, a `ConsoleAppender` with the default target
      `System.out`). So HAPI writes the report to **standard output**, after a
      log-pattern header line (`… ERROR … o.s.b.d.LoggingFailureAnalysisReporter :`).
      The probe had no logback on its class path and printed through
      java.util.logging to stderr (`SEVERE:` prefix), so no THEN clause about
      the stream was measured from HAPI.
    - **Form.** Several failures the spec lists have no failure analyzer. In
      HAPI each prints `Application run failed` plus the exception chain, not
      the `APPLICATION FAILED TO START` block. Probe evidence:
      - `config.placeholder.011`: `PlaceholderResolutionException: Could not resolve placeholder 'nope' in value "${nope}"`.
        This was raised by the probe's own `getProperty` call after startup,
        not by HAPI's condition, so it is not HAPI's output.
      - `config.args.007`: `SEVERE: Application run failed` /
        `org.springframework.boot.json.JsonParseException: Cannot parse JSON`.
      - `config.profile.012` and finding 5: `InvalidConfigDataPropertyException`.
      - Finding 6: `IllegalArgumentException`.
      - `config.gate.005`: HAPI throws a `NullPointerException` from
        `OnR4Condition.java:13` (`getProperty(...).toUpperCase()`), but the spec
        has Frevo print "a report naming `hapi.fhir.fhir_version`". That
        message does not exist in HAPI, and no `DEV` entry records it.
    - Fix: for each failure kind, state what HAPI emits and on which stream,
      and match it.
    - Fix: or record one `DEV` entry for the stream and format of startup
      failures, plus `gate.005`'s message.
    - Fix: give the oracle's HAPI side logback and the packaged `logback.xml`,
      so the recorded output is HAPI's.

13. **Two scenarios cannot be tested as written.**
    - `config.properties.004` is titled "unicode escapes", but its WHEN line
      reads literally `hapi.fhir.custom_content_path Abc` (bytes checked). The
      `\u0041`-style escape was lost in the text, so a test written from it
      exercises no escape. Fix: write the escape literally (for example
      `hapi.fhir.custom_content_path \u0041bc`, in a code span that survives
      rendering).
    - `config.yaml.016` asserts `line 5, column 31` and `line 5, column 33` "for
      the five-line fixture `config.yaml.016.yaml`", but the change does not
      contain the fixture. An independent author cannot reproduce the positions.
      Fix: put the five lines in the scenario.
