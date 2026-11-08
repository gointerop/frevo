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

| Repository | Ref | Commit SHA | Role |
|---|---|---|---|
| `hapifhir/hapi-fhir-jpaserver-starter` | `image/v8.12.0-2` | `377e6eba316920082e6a90551b488e4599a71eb8` | Configuration surface and wiring being matched (`application.yaml`, `AppProperties`) |
| `hapifhir/hapi-fhir` | `v8.12.0` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | Reference behaviour for storage, search, jobs, subscriptions, MDM, providers |
| `hapifhir/org.hl7.fhir.core` | `6.9.12` | `73c432b5f43ddbce979978a760ec4aa81e929094` | Validator, FHIRPath engine, converters, model definitions |
| `cqframework/clinical_quality_language` | `v5.4.0` | `c41c21f5616a0da82d49e32a9cc3a5b3ca299bc2` | CQL to ELM translator and ELM engine |
| `cqframework/clinical-reasoning` | `v4.13.0` | `ccba474a2fba73329353b41d270f9d0001d6cb52` | Measures, care gaps, `$apply`, SDC, CDS Hooks |

> `image/v8.12.0-2` is an **annotated** tag: its tag-object SHA is
> `787185517037226c5355018f6bf5a902d830ecae`, and the commit it points at is
> `377e6eba316920082e6a90551b488e4599a71eb8`. The table records commits, which
> is what `git rev-parse HEAD` reports in a clone and what provenance headers
> must cite. The other four tags are lightweight, so tag and commit coincide.

## Version note

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
