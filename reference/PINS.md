# Upstream reference pins

These repositories are cloned **read-only** into `reference/` by
`reference/clone.sh`. They are git-ignored, never modified, and never compiled
into the product. They exist so that agents can read the behaviour being ported
and so that every ported file can name its origin.

Resolution method, so this table can be rebuilt from scratch:

1. The starter release built on HAPI FHIR 8.12.0 is the newest `image/v8.12.0-*`
   release of `hapi-fhir-jpaserver-starter` (`image/v8.12.0-2`, published
   2026-10-02).
2. That starter's `pom.xml` declares parent `ca.uhn.hapi.fhir:hapi-fhir:8.12.0`
   and property `clinical-reasoning.version = 4.13.0`
   (`org.opencds.cqf.fhir:cqf-fhir-cr-hapi`).
3. `hapi-fhir` at `v8.12.0` declares `<fhir_core_version>6.9.12</fhir_core_version>`.
4. `clinical-reasoning` at `v4.13.0` declares, in `gradle/libs.versions.toml`,
   `hapi = "8.12.0"` (consistent with 1) and `cql = "5.4.0"`.
5. `org.hl7.fhir.core` `6.9.12` declares `org.fhir:ucum` `1.0.10` in its `pom.xml`
   (dependencyManagement); the source is `FHIR/Ucum-java` at tag `1.0.10`
   (added 2026-11-20 with the owner's approval).
6. The Spring sources define the configuration binding and property-source
   behaviour that `application.yaml` relies on (ruling of 2026-11-22, task 5 of
   `add-config-application-yaml`): `spring-projects/spring-boot` at `v3.5.14`,
   `spring-projects/spring-framework` at `v6.2.18` and SnakeYAML at
   `snakeyaml-2.4`, the YAML parser Spring Boot 3.5 manages. All three are
   Apache-2.0 (each tree's `LICENSE.txt` and, for SnakeYAML, its `pom.xml`).

| Repository | Ref | Commit SHA | Role |
|---|---|---|---|
| `hapifhir/hapi-fhir-jpaserver-starter` | `image/v8.12.0-2` | `377e6eba316920082e6a90551b488e4599a71eb8` | Configuration surface and wiring being matched (`application.yaml`, `AppProperties`) |
| `hapifhir/hapi-fhir` | `v8.12.0` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | Reference behaviour for storage, search, jobs, subscriptions, MDM, providers |
| `hapifhir/org.hl7.fhir.core` | `6.9.12` | `73c432b5f43ddbce979978a760ec4aa81e929094` | Validator, FHIRPath engine, converters, model definitions |
| `cqframework/clinical_quality_language` | `v5.4.0` | `c41c21f5616a0da82d49e32a9cc3a5b3ca299bc2` | CQL to ELM translator and ELM engine |
| `cqframework/clinical-reasoning` | `v4.13.0` | `ccba474a2fba73329353b41d270f9d0001d6cb52` | Measures, care gaps, `$apply`, SDC, CDS Hooks |
| `FHIR/Ucum-java` | `1.0.10` | `db9d22ade3f0f4a667b45817b85a9ac4534e4224` | UCUM library (`org.fhir:ucum`, BSD-3-Clause) that the FHIRPath engine uses for quantity arithmetic and `Decimal` |
| `spring-projects/spring-boot` | `v3.5.14` | `7d7b3ac12735161f9c096ce6cb415bdd9fc4a0f4` | Configuration binding, relaxed names, placeholder resolution and `application.yaml` loading (Apache-2.0) |
| `spring-projects/spring-framework` | `v6.2.18` | `6b117247d383294662e199c6b47d7bf54c49caaa` | `Environment`, property sources, `PropertyResolver` placeholder and conversion-service behaviour (Apache-2.0) |
| `snakeyaml/snakeyaml` | `snakeyaml-2.4` | `558c81a4893726c3c03f5c433fed69e79001899e` | YAML parser under Spring Boot's YAML loading (Apache-2.0); hosted on Bitbucket, not GitHub |

> `image/v8.12.0-2` is an **annotated** tag: its tag-object SHA is
> `787185517037226c5355018f6bf5a902d830ecae`, and the commit it points at is
> `377e6eba316920082e6a90551b488e4599a71eb8`. The table records commits, which
> is what `git rev-parse HEAD` reports in a clone and what provenance headers
> must cite. `snakeyaml-2.4` is likewise annotated: its tag-object SHA is
> `e39a974bff7f1f75e5af32d172d063b39453d08c` and its commit is
> `558c81a4893726c3c03f5c433fed69e79001899e`. The other tags are lightweight,
> so tag and commit coincide.

## Version note

**Ucum-java.** The pin is 1.0.10, the version `org.hl7.fhir.core` 6.9.12 declares. Live
HAPI 8.12.0 resolves `org.fhir:ucum` to **1.0.9** (`hapi-fhir/pom.xml:1148`, managed
at `:1985-1989`). The two jars are identical for every class the FHIRPath engine
uses except `XmlDefinitionsParser`: the short `yyyy-MM-dd` `revision-date`
form loads only on 1.0.10, and the spec states that branch
(`path.quantity.016`). The owner chose on 2026-11-21 to keep 1.0.10 so that the
citations stay valid; the differential run against live HAPI settles anything
else.


`hapi-fhir` `v8.12.1` exists upstream. The target set by the project owner is
**8.12.0**, and the starter's newest 8.12.x image release is built on 8.12.0, so
8.12.0 is what the differential oracle runs. Moving the pin is a deliberate,
reviewed change, not a routine dependency bump: it changes the definition of
correct for every gate.

## Refreshing

```bash
bash reference/clone.sh
```

The script is idempotent: an existing clone is left alone. To re-pin, delete the
directory and update this table in the same commit.
