# The binding probe

Evidence for the spec delta, kept so that the next reader can rerun it rather
than trust it. It is **not** the `config-roundtrip` oracle (that is task 1,
in `harness/config-roundtrip/`, run from the pinned image on Linux); it is the
tool the proposer used to measure Spring Boot 3.5.14 before writing any THEN
clause. See [`../design.md`](../design.md) section 2 for what it is and its
two limits (Windows host, jars from a Maven cache rather than the image).

## Files

| File | What it is |
|---|---|
| `Probe.java` | a `@Configuration` with `@EnableConfigurationProperties` for the starter's nine classes; boots `SpringApplication` with `WebApplicationType.NONE`, then prints every getter reachable from each bean as `Bean.getter.path = Type:value`, the named reads as `ENV[name] = value`, and the active profiles. `PROBE_FILTER` (a Java regex) limits the lines printed |
| `out-class-defaults.txt` | run with `--spring.config.location=optional:file:./none.yaml` — the class defaults (`keys.md` sections 1, 2) |
| `out-packaged.txt` | run with the packaged files on the class path and an empty working directory (`keys.md` section 3) |
| `out-profile-cds.txt` | the same with `SPRING_PROFILES_ACTIVE=cds` (`keys.md` section 4) |

## Running it

Compile `Probe.java` together with, unchanged, the nine properties classes
from `reference/hapi-fhir-jpaserver-starter/src/main/java/ca/uhn/fhir/jpa/starter/`
(`AppProperties.java`, `cdshooks/CdsHooksProperties.java`, `cr/CrProperties.java`,
`cr/CareGapsProperties.java`, `cr/CqlProperties.java`,
`cr/CqlCompilerProperties.java`, `cr/CqlRuntimeProperties.java`,
`cr/CqlTerminologyProperties.java`, `cr/CqlData.java`), with
`javac -proc:none`, against this class path (Maven coordinates):

```
org.springframework.boot:spring-boot:3.5.14
org.springframework:spring-core:6.2.18      (and -beans, -context, -aop, -expression, -jcl at 6.2.18)
org.yaml:snakeyaml:2.4
ca.uhn.hapi.fhir:hapi-fhir-base:8.12.0
ca.uhn.hapi.fhir:hapi-fhir-storage:8.12.0
ca.uhn.hapi.fhir:hapi-fhir-jpaserver-model:8.12.0
ca.uhn.hapi.fhir:org.hl7.fhir.r4:6.9.12
ca.uhn.hapi.fhir:org.hl7.fhir.utilities:6.9.12
org.apache.commons:commons-lang3:3.18.0
org.cqframework:cql-to-elm-jvm:5.4.0
org.opencds.cqf.fhir:cqf-fhir-cql:4.13.0
org.opencds.cqf.fhir:cqf-fhir-utility:4.13.0
org.jetbrains.kotlin:kotlin-stdlib:2.3.21
```

Run it from the case's working directory, with a directory holding the
starter's three `application*.yaml` files first on the class path when a case
needs the packaged files:

```
java -cp "<classes>:<packaged-resources-dir>:<jars>" probe.Probe [--name=value ...]
```

A failing configuration prints Spring Boot's failure analysis on standard
error and exits `1`.
