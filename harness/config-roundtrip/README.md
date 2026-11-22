# config-roundtrip

The oracle of `add-config-application-yaml` (design.md section 7): every
configuration case is run by HAPI FHIR JPA Server Starter 8.12.0 inside its
pinned image, and `frevo-config effective` must produce the same
effective-configuration document, or fail the same way.

| Path | What it is |
|---|---|
| `Dockerfile` | builds the HAPI side: the probe compiled against the pinned image's own `WEB-INF/lib` and `WEB-INF/classes`, added to the unchanged image at `/app/extra-classes/` |
| `src/main/java/dev/frevo/harness/config/ConfigRoundtrip.java` | the probe: boots Spring Boot with the starter's nine properties classes, dumps every bean property to the document of config.dump.001 (derived getters excluded, `CqlTerminologyProperties` flattened, the e-mail password redacted), reads the thirteen named keys, records `System.getProperties()` |
| `src/main/java/dev/frevo/harness/config/GateEvaluator.java` | the twenty-five gates: the starter's own condition classes, called on the annotation metadata of the starter class or `@Bean` method that carries them, during context refresh |
| `src/main/java/dev/frevo/harness/config/Schema.java` | prints the bean-property schema of the image's classes (`run.py schema`); `schema.tsv` is its committed output, read by `make_cases.py` |
| `loader.properties` | `loader.main` for the image's `PropertiesLauncher` (read from its home directory, `/app`; not a system property) |
| `make_cases.py` | writes `cases/`; edit it, not the case directories |
| `cases/<id>/` | one case: `case.yaml` (`covers:` tags), `files/` (into the working directory `/app`), `root/` (absolute paths), `env`, `args` |
| `packaged/` | the three `application*.yaml` files exactly as `main.war!/WEB-INF/classes/` holds them (config.source.001); Apache-2.0, from `hapi-fhir-jpaserver-starter@377e6eba` after Maven resource filtering, unmodified |
| `run.py` | `inspect`, `build`, `record`, `self-test`, `schema`, `case <id>` |
| `hapi-8.12.0-results.json` | the record the Rust tests compare against (`crates/frevo-config/tests/`); written only by `run.py record` |

## How a case runs on the HAPI side

`run.py` creates a container of the probe image with the image's own
`ENTRYPOINT` (`java --class-path /app/main.war
-Dloader.path=main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes
org.springframework.boot.loader.PropertiesLauncher`) and the case's arguments,
the image's environment plus the case's variables, copies `files/` into `/app`
(the image's `WORKDIR`, so `./application.yaml` is `/app/application.yaml` as
in HAPI) and `root/` to `/`, starts it, and records the exit status, standard
output and standard error separately, and the document the probe wrote. The
starter's classes and the packaged `application*.yaml` and `logback.xml` are
taken from the image's `WEB-INF/classes`, which precedes `/app/extra-classes`
on the class path, so nothing of the starter is recompiled.

Logback's own status lines (`HH:MM:SS,mmm |-INFO in …`, printed because the
packaged `logback.xml` sets `scan="true"`) are removed from both streams before
recording and counted in `logback_status_lines`.

## Inspection (task 1)

`run.py inspect` reads `main.war` out of the pinned image and fails if:

- `spring-boot`, `spring-core` or `snakeyaml` in `WEB-INF/lib` is not 3.5.14,
  6.2.18, 2.4 (proposal Risk 1);
- any `WEB-INF/lib` jar carries `application*.{properties,xml,yml,yaml}` or
  `config/application*` (config.oracle.001);
- `packaged/` is not byte-identical to `WEB-INF/classes/application*.yaml`.

`record` and `self-test` refuse to run while `inspect` fails. `--diagnostic`
runs anyway and writes `target/diagnostic-results.json`, which is evidence for
an owner decision and is never read by a test.

## Frevo side

`crates/frevo-config/tests/common/mod.rs` runs `frevo-config effective` on the
same case with a cleared environment plus the case's variables, in
`<tmp>/root/app/` with `root/` under `<tmp>/root/`, and passes
`FREVO_CONFIG_TEST_ROOT=<tmp>/root` so absolute paths resolve there (a test
seam; see `tasks/blockers.md`). Comparison: config.oracle.004 for documents
(sets sorted, member order ignored, `random:`/`process:` paths compared by
shape), config.oracle.003 for failures (the masked config-related lines).

## Host-dependent values in the record

`system_properties` comes from case `config.oracle.001`. `os.version` is the
kernel of the Docker host that recorded it, not a property of the image.
`PID` is `1`, `user.dir` `/app`, `user.name` `nonroot`, as in HAPI's container.

## Checking the tests against the record

`replay/` is a stand-in for `frevo-config effective` that prints a case's
recorded HAPI result (the document, or the failure output with stack frames and
non-config log lines removed). Running the tests with it shows which assertions
disagree with HAPI before any Frevo code exists:

```sh
cargo build --manifest-path harness/config-roundtrip/replay/Cargo.toml
FREVO_CONFIG_TEST_BIN=harness/config-roundtrip/replay/target/debug/config-roundtrip-replay \
FREVO_CONFIG_TEST_RECORD=harness/config-roundtrip/target/diagnostic-results.json \
  cargo test -p frevo-config
```

Every test that fails under the replay is either a scenario with no HAPI
counterpart (a `DEV-nnnn` deviation, `config.perf.*`, the packaged-file copy of
config.source.001, the servlet logger name of config.error.008, the
process-dependent values of config.placeholder.012), a check on the record
itself (`diagnostic` must be `false`), or a test that disagrees with HAPI and
must be fixed.
