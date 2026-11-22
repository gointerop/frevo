# Key register — `add-config-application-yaml`

Every configuration key of HAPI FHIR JPA Server Starter 8.12.0, what binds it,
where it appears in the effective configuration (config.dump.*), which
scenarios cover it, and which later change wires it to behaviour. This file is
referenced by `config.key.001`, `.003`, `.004` and `.011`; it is part of the
spec delta's normative text for those four scenarios.

**How the default tables were produced.** Sections 1 to 4 are not
transcribed from the Java source. They are the output of the probe described
in [`design.md`](design.md) section 2: the starter's nine properties classes,
compiled from `hapi-fhir-jpaserver-starter@377e6eba`, bound by Spring Boot
3.5.14 (Spring Framework 6.2.18 from a local cache, SnakeYAML 2.4) against
the HAPI 8.12.0 and clinical-reasoning 4.13.0 jars, with every getter dumped;
the oracle re-records them inside the pinned image, whose Spring Framework
6.2.19 is the reference (owner ruling of 2026-11-22). Sections 1 and 2 ran
with `--spring.config.location=optional:file:./none.yaml` (no file at all)
and `--hapi.fhir.fhir_version=R4`, without which HAPI fails to start
(config.key.001, config.gate.005);
section 3 with the packaged files; section 4 with `SPRING_PROFILES_ACTIVE=cds`.
A JSON value is shown for each path; sets are sorted, as config.dump.004
requires. The `config-roundtrip` oracle (design.md section 7) re-derives every
one of these values from the pinned image; where it disagrees, the oracle wins
and this file is corrected.

Paths follow the bean prefix of config.dump.002. `CrProperties` reports the
care-gaps keys as `hapi.fhir.cr.care-gaps.*` (its property is `careGaps`) and
`CareGapsProperties` reports the same keys at its own prefix as
`hapi.fhir.cr.caregaps.*`; the `hapi.fhir.cr.cql.*` subtree is reported
identically by `CrProperties` (`cr.cql`) and by the six `Cql*` classes at their
own prefixes.

## 1. `AppProperties` class defaults (`config.key.001`, `config.key.002`)

Objects that are `null` by default are listed once as `null`; their members'
defaults when created are in `config.key.002`.

| Path | Class default |
|---|---|
| `hapi.fhir.advanced-lucene-indexing` | `false` |
| `hapi.fhir.allow-cascading-deletes` | `false` |
| `hapi.fhir.allow-contains-searches` | `true` |
| `hapi.fhir.allow-database-validation-override` | `false` |
| `hapi.fhir.allow-external-references` | `false` |
| `hapi.fhir.allow-multiple-delete` | `false` |
| `hapi.fhir.allow-override-default-search-params` | `true` |
| `hapi.fhir.allowed-bundle-types` | `null` |
| `hapi.fhir.app-content-path` | `null` |
| `hapi.fhir.auto-create-placeholder-reference-targets` | `false` |
| `hapi.fhir.auto-version-reference-at-paths` | `[]` |
| `hapi.fhir.binary-storage-enabled` | `false` |
| `hapi.fhir.binary-storage-filesystem-base-directory` | `null` |
| `hapi.fhir.binary-storage-minimum-binary-size` | `null` |
| `hapi.fhir.binary-storage-mode` | `"DATABASE"` |
| `hapi.fhir.bulk-export-enabled` | `false` |
| `hapi.fhir.bulk-export-file-retention-period-hours` | `2` |
| `hapi.fhir.bulk-import-enabled` | `false` |
| `hapi.fhir.bundle-batch-pool-max-size` | `100` |
| `hapi.fhir.bundle-batch-pool-size` | `20` |
| `hapi.fhir.client-id-strategy` | `"ALPHANUMERIC"` |
| `hapi.fhir.client.connect-timeout` | `10000` |
| `hapi.fhir.client.connection-request-timeout` | `10000` |
| `hapi.fhir.client.connection-ttl` | `5000` |
| `hapi.fhir.client.pool-max-per-route` | `20` |
| `hapi.fhir.client.pool-max-total` | `20` |
| `hapi.fhir.client.socket-timeout` | `10000` |
| `hapi.fhir.cors` | `null` |
| `hapi.fhir.cr-enabled` | `false` |
| `hapi.fhir.custom-content-path` | `null` |
| `hapi.fhir.custom-interceptor-classes` | `[]` |
| `hapi.fhir.custom-provider-classes` | `[]` |
| `hapi.fhir.dao-scheduling-enabled` | `true` |
| `hapi.fhir.default-encoding` | `"JSON"` |
| `hapi.fhir.default-page-size` | `20` |
| `hapi.fhir.default-pretty-print` | `true` |
| `hapi.fhir.defer-indexing-for-codesystems-of-size` | `100` |
| `hapi.fhir.delete-enabled` | `true` |
| `hapi.fhir.delete-expunge-enabled` | `false` |
| `hapi.fhir.elasticsearch` | `null` |
| `hapi.fhir.enable-index-contained-resource` | `false` |
| `hapi.fhir.enable-index-missing-fields` | `false` |
| `hapi.fhir.enable-index-of-type` | `false` |
| `hapi.fhir.enable-repository-validating-interceptor` | `false` |
| `hapi.fhir.enable-task-pre-expand-value-sets` | `true` |
| `hapi.fhir.enforce-referential-integrity-on-delete` | `true` |
| `hapi.fhir.enforce-referential-integrity-on-write` | `true` |
| `hapi.fhir.etag-support-enabled` | `true` |
| `hapi.fhir.expunge-enabled` | `true` |
| `hapi.fhir.expunge-thread-count` | `null` |
| `hapi.fhir.fhir-version` | `"R4"` |
| `hapi.fhir.fhirpath-interceptor-enabled` | `false` |
| `hapi.fhir.filter-search-enabled` | `true` |
| `hapi.fhir.graphql-enabled` | `false` |
| `hapi.fhir.ig-runtime-upload-enabled` | `false` |
| `hapi.fhir.implementation-guides` | `null` |
| `hapi.fhir.index-storage-optimized` | `false` |
| `hapi.fhir.install-additional-resources-from-ig-folders` | `[]` |
| `hapi.fhir.install-transitive-ig-dependencies` | `true` |
| `hapi.fhir.ips-enabled` | `false` |
| `hapi.fhir.language-search-parameter-enabled` | `false` |
| `hapi.fhir.lastn-enabled` | `false` |
| `hapi.fhir.local-base-urls` | `[]` |
| `hapi.fhir.logger.error-format` | `"ERROR - ${requestVerb} ${requestUrl}"` |
| `hapi.fhir.logger.format` | `"Path[${servletPath}] Source[${requestHeader.x-forwarded-for}] Operation[${operationType} ${operationName} ${idOrResourceName}] UA[${requestHeader.user-agent}] Params[${requestParameters}] ResponseEncoding[${responseEncodingNoDefault}] Operation[${operationType} ${operationName} ${idOrResourceName}] UA[${requestHeader.user-agent}] Params[${requestParameters}] ResponseEncoding[${responseEncodingNoDefault}]"` |
| `hapi.fhir.logger.log-exceptions` | `true` |
| `hapi.fhir.logger.name` | `"fhirtest.access"` |
| `hapi.fhir.logical-urls` | `[]` |
| `hapi.fhir.mark-resources-for-reindexing-upon-search-parameter-change` | `true` |
| `hapi.fhir.mass-ingestion-mode-enabled` | `false` |
| `hapi.fhir.match-url-cache-enabled` | `false` |
| `hapi.fhir.max-binary-size` | `null` |
| `hapi.fhir.max-page-size` | `2147483647` |
| `hapi.fhir.maximum-expansion-size` | `1000` |
| `hapi.fhir.mdm-enabled` | `false` |
| `hapi.fhir.mdm-rules-json-location` | `"mdm-rules.json"` |
| `hapi.fhir.narrative-enabled` | `true` |
| `hapi.fhir.normalized-quantity-search-level` | `"NORMALIZED_QUANTITY_SEARCH_NOT_SUPPORTED"` |
| `hapi.fhir.openapi-enabled` | `false` |
| `hapi.fhir.partitioning` | `null` |
| `hapi.fhir.pre-expand-value-sets` | `true` |
| `hapi.fhir.pre-expand-value-sets-default-count` | `1000` |
| `hapi.fhir.pre-expand-value-sets-max-count` | `1000` |
| `hapi.fhir.reindex-thread-count` | `null` |
| `hapi.fhir.remote-terminology-service` | `null` |
| `hapi.fhir.resource-dbhistory-enabled` | `true` |
| `hapi.fhir.retain-cached-searches-mins` | `60` |
| `hapi.fhir.reuse-cached-search-results-millis` | `60000` |
| `hapi.fhir.search-index-full-text-enabled` | `false` |
| `hapi.fhir.search-prefetch-thresholds` | `[]` |
| `hapi.fhir.server-address` | `null` |
| `hapi.fhir.server-id-strategy` | `null` |
| `hapi.fhir.store-meta-source-information` | `"NONE"` |
| `hapi.fhir.store-resource-in-lucene-index-enabled` | `false` |
| `hapi.fhir.subscription.cross-partition-enabled` | `false` |
| `hapi.fhir.subscription.email` | `null` |
| `hapi.fhir.subscription.immediately-queued` | `false` |
| `hapi.fhir.subscription.polling-interval-ms` | `null` |
| `hapi.fhir.subscription.resthook-enabled` | `false` |
| `hapi.fhir.subscription.websocket-enabled` | `false` |
| `hapi.fhir.supported-resource-types` | `[]` |
| `hapi.fhir.tester` | `null` |
| `hapi.fhir.uplifted-refchains-enabled` | `false` |
| `hapi.fhir.use-apache-address-strategy` | `false` |
| `hapi.fhir.use-apache-address-strategy-https` | `false` |
| `hapi.fhir.user-request-retry-version-conflicts-interceptor-enabled` | `false` |
| `hapi.fhir.validate-resource-status-for-package-upload` | `true` |
| `hapi.fhir.validation.requests-enabled` | `false` |
| `hapi.fhir.validation.responses-enabled` | `false` |

## 2. `CdsHooksProperties` and clinical-reasoning class defaults (`config.key.003`)

| Path | Class default |
|---|---|
| `hapi.fhir.cdshooks.client-id-header-name` | `null` |
| `hapi.fhir.cdshooks.enabled` | `false` |
| `hapi.fhir.cr.care-gaps.reporter` | `"default"` |
| `hapi.fhir.cr.care-gaps.section-author` | `"default"` |
| `hapi.fhir.cr.caregaps.reporter` | `"default"` |
| `hapi.fhir.cr.caregaps.section-author` | `"default"` |
| `hapi.fhir.cr.cql.compiler.analyze-data-requirements` | `false` |
| `hapi.fhir.cr.cql.compiler.collapse-data-requirements` | `false` |
| `hapi.fhir.cr.cql.compiler.compatibility-level` | `"1.5"` |
| `hapi.fhir.cr.cql.compiler.disable-default-model-info-load` | `false` |
| `hapi.fhir.cr.cql.compiler.disable-list-demotion` | `false` |
| `hapi.fhir.cr.cql.compiler.disable-list-promotion` | `false` |
| `hapi.fhir.cr.cql.compiler.disable-list-traversal` | `false` |
| `hapi.fhir.cr.cql.compiler.disable-method-invocation` | `false` |
| `hapi.fhir.cr.cql.compiler.enable-annotations` | `true` |
| `hapi.fhir.cr.cql.compiler.enable-date-range-optimization` | `true` |
| `hapi.fhir.cr.cql.compiler.enable-detailed-errors` | `true` |
| `hapi.fhir.cr.cql.compiler.enable-interval-demotion` | `false` |
| `hapi.fhir.cr.cql.compiler.enable-interval-promotion` | `false` |
| `hapi.fhir.cr.cql.compiler.enable-locators` | `true` |
| `hapi.fhir.cr.cql.compiler.enable-results-type` | `true` |
| `hapi.fhir.cr.cql.compiler.error-severity-level` | `"Info"` |
| `hapi.fhir.cr.cql.compiler.require-from-keyword` | `false` |
| `hapi.fhir.cr.cql.compiler.signature-level` | `"All"` |
| `hapi.fhir.cr.cql.compiler.translator-format` | `"JSON"` |
| `hapi.fhir.cr.cql.compiler.validate-units` | `true` |
| `hapi.fhir.cr.cql.compiler.verify-only` | `false` |
| `hapi.fhir.cr.cql.data.profile-mode` | `"OFF"` |
| `hapi.fhir.cr.cql.data.search-parameter-mode` | `"AUTO"` |
| `hapi.fhir.cr.cql.data.terminology-parameter-mode` | `"AUTO"` |
| `hapi.fhir.cr.cql.namespaces` | `{}` |
| `hapi.fhir.cr.cql.runtime.debug-logging-enabled` | `false` |
| `hapi.fhir.cr.cql.runtime.enable-expression-caching` | `true` |
| `hapi.fhir.cr.cql.runtime.enable-validation` | `false` |
| `hapi.fhir.cr.cql.terminology.code-lookup-mode` | `"AUTO"` |
| `hapi.fhir.cr.cql.terminology.valueset-expansion-mode` | `"AUTO"` |
| `hapi.fhir.cr.cql.terminology.valueset-membership-mode` | `"AUTO"` |
| `hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode` | `"USE_IF_PRESENT"` |
| `hapi.fhir.cr.cql.use-embedded-libraries` | `true` |
| `hapi.fhir.cr.enabled` | `null` |
| `hapi.fhir.cr.terminology-server-client-settings.crmi-version` | `"1.0.0"` |
| `hapi.fhir.cr.terminology-server-client-settings.expansions-per-page` | `1000` |
| `hapi.fhir.cr.terminology-server-client-settings.max-expansion-pages` | `1000` |
| `hapi.fhir.cr.terminology-server-client-settings.max-retry-count` | `3` |
| `hapi.fhir.cr.terminology-server-client-settings.max-tx-resource-code-system-concepts` | `1000` |
| `hapi.fhir.cr.terminology-server-client-settings.retry-interval-millis` | `1000` |
| `hapi.fhir.cr.terminology-server-client-settings.socket-timeout` | `60` |
| `hapi.fhir.cr.terminology-server-client-settings.timeout-seconds` | `30` |
| `hapi.fhir.cr.terminology-server-client-settings.tx-resource-mode` | `"AUTO"` |

## 3. What the packaged `application.yaml` changes (`config.key.004`, `config.source.002`)

Every path whose value with the packaged files differs from sections 1 and 2.
No other path differs. Keys the packaged file sets to their class default
(for example `ig_runtime_upload_enabled: false`, `mdm_enabled: false`,
`cr.cql.compiler.signature_level: All`) are not listed, and
`cr.cql.compiler.error_level: Info` is not listed because it binds nothing
(config.relaxed.004). The packaged `search-coord-*` keys bind nothing
(config.unknown.002).

| Path | Class default | With the packaged files |
|---|---|---|
| `hapi.fhir.binary-storage-minimum-binary-size` | `null` | `4000` |
| `hapi.fhir.cdshooks.client-id-header-name` | `null` | `"client_id"` |
| `hapi.fhir.cors` | `null` | `(absent)` |
| `hapi.fhir.cors.allow-credentials` | `(absent)` | `false` |
| `hapi.fhir.cors.allowed-headers` | `(absent)` | `["Origin", "Accept", "Content-Type", "Authorization", "Cache-Control", "If-Match", "If-None-Match", "x-fhir-starter", "X-Requested-With", "Prefer"]` |
| `hapi.fhir.cors.allowed-methods` | `(absent)` | `["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH", "HEAD"]` |
| `hapi.fhir.cors.allowed-origin` | `(absent)` | `["*"]` |
| `hapi.fhir.cors.exposed-headers` | `(absent)` | `["Location", "Content-Location", "ETag", "Date", "Retry-After", "X-Correlation-Id", "X-Progress", "X-Request-Id"]` |
| `hapi.fhir.cr.cql.data.profile-mode` | `"OFF"` | `"DECLARED"` |
| `hapi.fhir.cr.cql.data.search-parameter-mode` | `"AUTO"` | `"USE_SEARCH_PARAMETERS"` |
| `hapi.fhir.cr.cql.data.terminology-parameter-mode` | `"AUTO"` | `"FILTER_IN_MEMORY"` |
| `hapi.fhir.cr.cql.terminology.code-lookup-mode` | `"AUTO"` | `"USE_VALIDATE_CODE_OPERATION"` |
| `hapi.fhir.cr.cql.terminology.valueset-expansion-mode` | `"AUTO"` | `"PERFORM_NAIVE_EXPANSION"` |
| `hapi.fhir.cr.cql.terminology.valueset-membership-mode` | `"AUTO"` | `"USE_EXPANSION"` |
| `hapi.fhir.cr.enabled` | `null` | `false` |
| `hapi.fhir.logical-urls` | `[]` | `["http://loinc.org/*", "http://snomed.info/*", "http://terminology.hl7.org/*", "http://unitsofmeasure.org/*", "https://loinc.org/*", "https://snomed.info/*", "https://terminology.hl7.org/*", "https://unitsofmeasure.org/*"]` |
| `hapi.fhir.narrative-enabled` | `true` | `false` |
| `hapi.fhir.openapi-enabled` | `false` | `true` |
| `hapi.fhir.search-prefetch-thresholds` | `[]` | `[13, 503, 2003, -1]` |
| `hapi.fhir.tester` | `null` | `{map, 1 entries}` |
| `hapi.fhir.tester.home.fhir-version` | `(absent)` | `"R4"` |
| `hapi.fhir.tester.home.name` | `(absent)` | `"Local Tester"` |
| `hapi.fhir.tester.home.refuse-to-fetch-third-party-urls` | `(absent)` | `false` |
| `hapi.fhir.tester.home.server-address` | `(absent)` | `"http://localhost:8080/fhir"` |

## 4. What the packaged `cds` profile changes (`config.profile.001`)

Every path whose value under `SPRING_PROFILES_ACTIVE=cds` differs from
section 3.

| Path | Packaged default | With profile `cds` |
|---|---|---|
| `hapi.fhir.allow-external-references` | `false` | `true` |
| `hapi.fhir.cdshooks.enabled` | `false` | `true` |
| `hapi.fhir.cr.cql.namespaces` | `{}` | `{map, 2 entries}` |
| `hapi.fhir.cr.cql.namespaces[hl7.fhir.us.cql]` | `(absent)` | `"http://hl7.org/fhir/us/cql"` |
| `hapi.fhir.cr.cql.namespaces[hl7.fhir.uv.cql]` | `(absent)` | `"http://hl7.org/fhir/uv/cql"` |
| `hapi.fhir.cr.cql.terminology.code-lookup-mode` | `"USE_VALIDATE_CODE_OPERATION"` | `"USE_CODESYSTEM_URL"` |
| `hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode` | `"USE_IF_PRESENT"` | `"REQUIRE"` |
| `hapi.fhir.cr.enabled` | `false` | `true` |
| `hapi.fhir.implementation-guides` | `null` | `{map, 2 entries}` |
| `hapi.fhir.implementation-guides.cql.additional-resource-folders` | `(absent)` | `[]` |
| `hapi.fhir.implementation-guides.cql.dependency-excludes` | `(absent)` | `[]` |
| `hapi.fhir.implementation-guides.cql.dry-run` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.cql.fetch-dependencies` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.cql.install-mode` | `(absent)` | `"STORE_AND_INSTALL"` |
| `hapi.fhir.implementation-guides.cql.install-resource-types` | `(absent)` | `["CodeSystem", "ValueSet", "ConceptMap", "ActivityDefinition", "PlanDefinition", "Library", "Measure", "StructureDefinition"]` |
| `hapi.fhir.implementation-guides.cql.name` | `(absent)` | `"hl7.fhir.uv.cql"` |
| `hapi.fhir.implementation-guides.cql.overwrite-content-not-present-code-systems` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.cql.package-url` | `(absent)` | `null` |
| `hapi.fhir.implementation-guides.cql.reload-existing` | `(absent)` | `true` |
| `hapi.fhir.implementation-guides.cql.version` | `(absent)` | `"2.0.0"` |
| `hapi.fhir.implementation-guides.cql.version-policy` | `(absent)` | `"MULTI_VERSION"` |
| `hapi.fhir.implementation-guides.uscql.additional-resource-folders` | `(absent)` | `[]` |
| `hapi.fhir.implementation-guides.uscql.dependency-excludes` | `(absent)` | `[]` |
| `hapi.fhir.implementation-guides.uscql.dry-run` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.uscql.fetch-dependencies` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.uscql.install-mode` | `(absent)` | `"STORE_AND_INSTALL"` |
| `hapi.fhir.implementation-guides.uscql.install-resource-types` | `(absent)` | `["CodeSystem", "ValueSet", "ConceptMap", "ActivityDefinition", "PlanDefinition", "Library", "Measure", "StructureDefinition"]` |
| `hapi.fhir.implementation-guides.uscql.name` | `(absent)` | `"hl7.fhir.us.cql"` |
| `hapi.fhir.implementation-guides.uscql.overwrite-content-not-present-code-systems` | `(absent)` | `false` |
| `hapi.fhir.implementation-guides.uscql.package-url` | `(absent)` | `null` |
| `hapi.fhir.implementation-guides.uscql.reload-existing` | `(absent)` | `true` |
| `hapi.fhir.implementation-guides.uscql.version` | `(absent)` | `"1.0.0"` |
| `hapi.fhir.implementation-guides.uscql.version-policy` | `(absent)` | `"MULTI_VERSION"` |

## 5. The 209 inventory rows (`config.key.011`)

Dispositions:

| Disposition | Meaning | Rows |
|---|---|---:|
| `bound` | a starter properties class binds it and the starter reads it | 163 |
| `bound, never read` | bound and reported, but nothing in the starter reads it, so it has no effect in HAPI 8.12.0 and none in Frevo | 7 |
| `bound, setter-less` | bound only to its current value; any other value fails to start | 1 |
| `map entry` | a field of one entry of a bound map; the generic field list is in section 6 | 28 |
| `not bound` | nothing binds it (unknown-key rule, config.unknown.*) | 7 |
| `not a key` | an artefact of `harness/scripts/gen-inventory.py` | 3 |
| **Total** | | **209** |

The "Wired to behaviour by" column is the change in `tasks/graph.yaml` whose
scenarios make the key change what the server does. Until that change merges,
the key is bound, validated, reported and gated here, and has no other effect
(config.unknown.007). Every `bound` row is also covered by the whole-corpus
round-trips `config.key.005` to `.007`.

| # | Inventory key | Disposition | Canonical path in the effective configuration | Scenarios | Wired to behaviour by | Note |
|---:|---|---|---|---|---|---|
| 1 | `hapi.fhir.NOTE` | not a key | `-` | config.key.011 | none | inventory artefact: the generator read a YAML comment (`# NOTE: ...`) or split a list value at `:`; see proposal, Inventory corrections |
| 2 | `hapi.fhir.advanced_lucene_indexing` | bound | `hapi.fhir.advanced-lucene-indexing` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-fulltext |  |
| 3 | `hapi.fhir.allow_cascading_deletes` | bound | `hapi.fhir.allow-cascading-deletes` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 4 | `hapi.fhir.allow_contains_searches` | bound | `hapi.fhir.allow-contains-searches` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 5 | `hapi.fhir.allow_database_validation_override` | bound | `hapi.fhir.allow-database-validation-override` | config.key.001, config.key.005, config.key.006, config.key.007 | add-validator-core |  |
| 6 | `hapi.fhir.allow_external_references` | bound | `hapi.fhir.allow-external-references` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 7 | `hapi.fhir.allow_multiple_delete` | bound | `hapi.fhir.allow-multiple-delete` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 8 | `hapi.fhir.allow_override_default_search_params` | bound | `hapi.fhir.allow-override-default-search-params` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-custom-params |  |
| 9 | `hapi.fhir.allowed_bundle_types` | bound | `hapi.fhir.allowed-bundle-types` | config.key.001, config.key.005, config.key.006, config.key.007, config.coerce.022 | add-store-transactions |  |
| 10 | `hapi.fhir.app_content_path` | bound | `hapi.fhir.app-content-path` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.010, config.placeholder.011 | add-tester-ui |  |
| 11 | `hapi.fhir.auto_create_placeholder_reference_targets` | bound | `hapi.fhir.auto-create-placeholder-reference-targets` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 12 | `hapi.fhir.auto_version_reference_at_paths` | bound | `hapi.fhir.auto-version-reference-at-paths` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-transactions |  |
| 13 | `hapi.fhir.binary_storage_enabled` | bound | `hapi.fhir.binary-storage-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-binary-and-offload |  |
| 14 | `hapi.fhir.binary_storage_filesystem_base_directory` | bound | `hapi.fhir.binary-storage-filesystem-base-directory` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-binary-and-offload |  |
| 15 | `hapi.fhir.binary_storage_minimum_binary_size` | bound | `hapi.fhir.binary-storage-minimum-binary-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-binary-and-offload |  |
| 16 | `hapi.fhir.binary_storage_mode` | bound | `hapi.fhir.binary-storage-mode` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.009 | add-store-binary-and-offload |  |
| 17 | `hapi.fhir.bulk_export_enabled` | bound | `hapi.fhir.bulk-export-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-bulk-export |  |
| 18 | `hapi.fhir.bulk_export_file_retention_period_hours` | bound | `hapi.fhir.bulk-export-file-retention-period-hours` | config.key.001, config.key.005, config.key.006, config.key.007 | add-bulk-export |  |
| 19 | `hapi.fhir.bulk_import_enabled` | bound | `hapi.fhir.bulk-import-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-bulk-import |  |
| 20 | `hapi.fhir.bundle_batch_pool_max_size` | bound | `hapi.fhir.bundle-batch-pool-max-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-transactions |  |
| 21 | `hapi.fhir.bundle_batch_pool_size` | bound | `hapi.fhir.bundle-batch-pool-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-transactions |  |
| 22 | `hapi.fhir.cdshooks.clientIdHeaderName` | bound | `hapi.fhir.cdshooks.client-id-header-name` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cds-hooks |  |
| 23 | `hapi.fhir.cdshooks.enabled` | bound | `hapi.fhir.cdshooks.enabled` | config.key.003, config.key.005, config.key.006, config.key.007, config.gate.006, .011 | add-cds-hooks |  |
| 24 | `hapi.fhir.client.connect_timeout` | bound | `hapi.fhir.client.connect-timeout` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 25 | `hapi.fhir.client.connection_request_timeout` | bound | `hapi.fhir.client.connection-request-timeout` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 26 | `hapi.fhir.client.connection_ttl` | bound | `hapi.fhir.client.connection-ttl` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 27 | `hapi.fhir.client.pool_max_per_route` | bound | `hapi.fhir.client.pool-max-per-route` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 28 | `hapi.fhir.client.pool_max_total` | bound | `hapi.fhir.client.pool-max-total` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 29 | `hapi.fhir.client.socket_timeout` | bound | `hapi.fhir.client.socket-timeout` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 30 | `hapi.fhir.client_id_strategy` | bound | `hapi.fhir.client-id-strategy` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 31 | `hapi.fhir.cors.allow_Credentials` | bound | `hapi.fhir.cors.allow-credentials` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 32 | `hapi.fhir.cors.allowed_headers` | bound | `hapi.fhir.cors.allowed-headers` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 33 | `hapi.fhir.cors.allowed_methods` | bound | `hapi.fhir.cors.allowed-methods` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 34 | `hapi.fhir.cors.allowed_origin` | bound | `hapi.fhir.cors.allowed-origin` | config.key.001, config.key.005, config.key.006, config.key.007, config.coerce.025, config.placeholder.008 | add-server-skeleton |  |
| 35 | `hapi.fhir.cors.exposed_headers` | bound | `hapi.fhir.cors.exposed-headers` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 36 | `hapi.fhir.cr.caregaps.reporter` | bound | `hapi.fhir.cr.caregaps.reporter` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures |  |
| 37 | `hapi.fhir.cr.caregaps.section_author` | bound | `hapi.fhir.cr.caregaps.section-author` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures |  |
| 38 | `hapi.fhir.cr.cql.compiler.analyze_data_requirements` | bound | `hapi.fhir.cr.cql.compiler.analyze-data-requirements` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 39 | `hapi.fhir.cr.cql.compiler.collapse_data_requirements` | bound | `hapi.fhir.cr.cql.compiler.collapse-data-requirements` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 40 | `hapi.fhir.cr.cql.compiler.compatibility_level` | bound | `hapi.fhir.cr.cql.compiler.compatibility-level` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 41 | `hapi.fhir.cr.cql.compiler.disable_default_model_info_load` | bound | `hapi.fhir.cr.cql.compiler.disable-default-model-info-load` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 42 | `hapi.fhir.cr.cql.compiler.disable_list_demotion` | bound | `hapi.fhir.cr.cql.compiler.disable-list-demotion` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 43 | `hapi.fhir.cr.cql.compiler.disable_list_promotion` | bound | `hapi.fhir.cr.cql.compiler.disable-list-promotion` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 44 | `hapi.fhir.cr.cql.compiler.disable_list_traversal` | bound | `hapi.fhir.cr.cql.compiler.disable-list-traversal` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 45 | `hapi.fhir.cr.cql.compiler.disable_method_invocation` | bound | `hapi.fhir.cr.cql.compiler.disable-method-invocation` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 46 | `hapi.fhir.cr.cql.compiler.enable_annotations` | bound | `hapi.fhir.cr.cql.compiler.enable-annotations` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 47 | `hapi.fhir.cr.cql.compiler.enable_date_range_optimization` | bound | `hapi.fhir.cr.cql.compiler.enable-date-range-optimization` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 48 | `hapi.fhir.cr.cql.compiler.enable_detailed_errors` | bound | `hapi.fhir.cr.cql.compiler.enable-detailed-errors` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 49 | `hapi.fhir.cr.cql.compiler.enable_interval_demotion` | bound | `hapi.fhir.cr.cql.compiler.enable-interval-demotion` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 50 | `hapi.fhir.cr.cql.compiler.enable_interval_promotion` | bound | `hapi.fhir.cr.cql.compiler.enable-interval-promotion` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 51 | `hapi.fhir.cr.cql.compiler.enable_locators` | bound | `hapi.fhir.cr.cql.compiler.enable-locators` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 52 | `hapi.fhir.cr.cql.compiler.enable_results_type` | bound | `hapi.fhir.cr.cql.compiler.enable-results-type` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 53 | `hapi.fhir.cr.cql.compiler.error_level` | not bound | `-` | config.relaxed.004 | add-cql-translator (as `error_severity_level`) | field name, not the bean property: the bound key is `cr.cql.compiler.error_severity_level` (config.relaxed.004) |
| 54 | `hapi.fhir.cr.cql.compiler.require_from_keyword` | bound | `hapi.fhir.cr.cql.compiler.require-from-keyword` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 55 | `hapi.fhir.cr.cql.compiler.signature_level` | bound | `hapi.fhir.cr.cql.compiler.signature-level` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 56 | `hapi.fhir.cr.cql.compiler.translator_format` | bound, never read | `hapi.fhir.cr.cql.compiler.translator-format` | config.key.003, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 57 | `hapi.fhir.cr.cql.compiler.validate_units` | bound | `hapi.fhir.cr.cql.compiler.validate-units` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 58 | `hapi.fhir.cr.cql.compiler.verify_only` | bound | `hapi.fhir.cr.cql.compiler.verify-only` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-translator |  |
| 59 | `hapi.fhir.cr.cql.data.profileMode` | bound | `hapi.fhir.cr.cql.data.profile-mode` | config.key.003, config.key.005, config.key.006, config.key.007, config.coerce.019 | add-cql-engine |  |
| 60 | `hapi.fhir.cr.cql.data.profile_mode` | bound | `hapi.fhir.cr.cql.data.profile-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | same property as the camelCase row (relaxed spelling) |
| 61 | `hapi.fhir.cr.cql.data.searchParameterMode` | bound | `hapi.fhir.cr.cql.data.search-parameter-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine |  |
| 62 | `hapi.fhir.cr.cql.data.search_parameter_mode` | bound | `hapi.fhir.cr.cql.data.search-parameter-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | same property as the camelCase row (relaxed spelling) |
| 63 | `hapi.fhir.cr.cql.data.terminology_parameter_mode` | bound | `hapi.fhir.cr.cql.data.terminology-parameter-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | same property as the camelCase row (relaxed spelling) |
| 64 | `hapi.fhir.cr.cql.namespaces` | bound | `hapi.fhir.cr.cql.namespaces` | config.key.003, config.key.005, config.key.006, config.key.007, config.collection.005, config.env.009 | add-cql-engine |  |
| 65 | `hapi.fhir.cr.cql.namespaces.hl7.fhir.us.cql` | map entry | `hapi.fhir.cr.cql.namespaces[hl7.fhir.us.cql]` | config.collection.003, config.key.005, config.key.006 | add-cql-engine | entry of `cr.cql.namespaces` |
| 66 | `hapi.fhir.cr.cql.namespaces.hl7.fhir.uv.cql` | map entry | `hapi.fhir.cr.cql.namespaces[hl7.fhir.uv.cql]` | config.collection.003, config.key.005, config.key.006 | add-cql-engine | entry of `cr.cql.namespaces` |
| 67 | `hapi.fhir.cr.cql.runtime.debug_logging_enabled` | bound | `hapi.fhir.cr.cql.runtime.debug-logging-enabled` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine |  |
| 68 | `hapi.fhir.cr.cql.runtime.enable_expression_caching` | bound | `hapi.fhir.cr.cql.runtime.enable-expression-caching` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine |  |
| 69 | `hapi.fhir.cr.cql.runtime.enable_validation` | bound, setter-less | `hapi.fhir.cr.cql.runtime.enable-validation` | config.key.003, config.key.005, config.key.006, config.key.007, config.unknown.005 | none (HAPI cannot enable it) | setting it to `true` fails to start |
| 70 | `hapi.fhir.cr.cql.terminology` | bound | `hapi.fhir.cr.cql.terminology` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine |  |
| 71 | `hapi.fhir.cr.cql.terminology.codeLookupMode` | bound | `hapi.fhir.cr.cql.terminology.code-lookup-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | bound by `CqlTerminologyProperties` and `CqlProperties.terminology` |
| 72 | `hapi.fhir.cr.cql.terminology.code_lookup_mode` | bound | `hapi.fhir.cr.cql.terminology.code-lookup-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | bound by `CqlTerminologyProperties` and `CqlProperties.terminology` |
| 73 | `hapi.fhir.cr.cql.terminology.valueset_expansion_mode` | bound | `hapi.fhir.cr.cql.terminology.valueset-expansion-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | bound by `CqlTerminologyProperties` and `CqlProperties.terminology` |
| 74 | `hapi.fhir.cr.cql.terminology.valueset_membership_mode` | bound | `hapi.fhir.cr.cql.terminology.valueset-membership-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | bound by `CqlTerminologyProperties` and `CqlProperties.terminology` |
| 75 | `hapi.fhir.cr.cql.terminology.valueset_preexpansion_mode` | bound | `hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cql-engine | bound by `CqlTerminologyProperties` and `CqlProperties.terminology` |
| 76 | `hapi.fhir.cr.cql.use_embedded_libraries` | bound, never read | `hapi.fhir.cr.cql.use-embedded-libraries` | config.key.003, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 77 | `hapi.fhir.cr.enabled` | bound | `hapi.fhir.cr.enabled` | config.key.003, config.key.005, config.key.006, config.key.007, config.gate.006 | add-cr-measures |  |
| 78 | `hapi.fhir.cr.terminologyServerClientSettings.maxRetryCount` | bound | `hapi.fhir.cr.terminology-server-client-settings.max-retry-count` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures | bound through `TerminologyServerClientSettings` (clinical-reasoning), whose class the inventory could not see |
| 79 | `hapi.fhir.cr.terminologyServerClientSettings.retryIntervalMillis` | bound | `hapi.fhir.cr.terminology-server-client-settings.retry-interval-millis` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures | bound through `TerminologyServerClientSettings` (clinical-reasoning), whose class the inventory could not see |
| 80 | `hapi.fhir.cr.terminologyServerClientSettings.socketTimeout` | bound | `hapi.fhir.cr.terminology-server-client-settings.socket-timeout` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures | bound through `TerminologyServerClientSettings` (clinical-reasoning), whose class the inventory could not see |
| 81 | `hapi.fhir.cr.terminologyServerClientSettings.timeoutSeconds` | bound | `hapi.fhir.cr.terminology-server-client-settings.timeout-seconds` | config.key.003, config.key.005, config.key.006, config.key.007 | add-cr-measures | bound through `TerminologyServerClientSettings` (clinical-reasoning), whose class the inventory could not see |
| 82 | `hapi.fhir.cr_enabled` | bound, never read | `hapi.fhir.cr-enabled` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 83 | `hapi.fhir.custom_content_path` | bound | `hapi.fhir.custom-content-path` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.010 | add-tester-ui |  |
| 84 | `hapi.fhir.custom_interceptor_classes` | bound | `hapi.fhir.custom-interceptor-classes` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 85 | `hapi.fhir.custom_provider_classes` | bound | `hapi.fhir.custom-provider-classes` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 86 | `hapi.fhir.dao_scheduling_enabled` | bound | `hapi.fhir.dao-scheduling-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-jobs-engine |  |
| 87 | `hapi.fhir.database_partition_mode_enabled` | not bound | `-` | config.unknown.003 | none | commented example at the wrong level; the bound key is `hapi.fhir.partitioning.database_partition_mode_enabled` |
| 88 | `hapi.fhir.default_encoding` | bound | `hapi.fhir.default-encoding` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 89 | `hapi.fhir.default_page_size` | bound | `hapi.fhir.default-page-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 90 | `hapi.fhir.default_pretty_print` | bound | `hapi.fhir.default-pretty-print` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 91 | `hapi.fhir.defer_indexing_for_codesystems_of_size` | bound | `hapi.fhir.defer-indexing-for-codesystems-of-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 92 | `hapi.fhir.delete_enabled` | bound | `hapi.fhir.delete-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 93 | `hapi.fhir.delete_expunge_enabled` | bound | `hapi.fhir.delete-expunge-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-maintenance-jobs |  |
| 94 | `hapi.fhir.elasticsearch.index_prefix` | bound | `hapi.fhir.elasticsearch.index-prefix` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-fulltext |  |
| 95 | `hapi.fhir.enable_index_contained_resource` | bound | `hapi.fhir.enable-index-contained-resource` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 96 | `hapi.fhir.enable_index_missing_fields` | bound | `hapi.fhir.enable-index-missing-fields` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 97 | `hapi.fhir.enable_index_of_type` | bound | `hapi.fhir.enable-index-of-type` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 98 | `hapi.fhir.enable_repository_validating_interceptor` | bound | `hapi.fhir.enable-repository-validating-interceptor` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.008 | add-validator-core |  |
| 99 | `hapi.fhir.enable_task_pre_expand_value_sets` | bound | `hapi.fhir.enable-task-pre-expand-value-sets` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 100 | `hapi.fhir.enforce_referential_integrity_on_delete` | bound | `hapi.fhir.enforce-referential-integrity-on-delete` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 101 | `hapi.fhir.enforce_referential_integrity_on_write` | bound | `hapi.fhir.enforce-referential-integrity-on-write` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 102 | `hapi.fhir.etag_support_enabled` | bound | `hapi.fhir.etag-support-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 103 | `hapi.fhir.expunge_enabled` | bound | `hapi.fhir.expunge-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-maintenance-jobs |  |
| 104 | `hapi.fhir.expunge_thread_count` | bound | `hapi.fhir.expunge-thread-count` | config.key.001, config.key.005, config.key.006, config.key.007 | add-maintenance-jobs |  |
| 105 | `hapi.fhir.fhir_version` | bound | `hapi.fhir.fhir-version` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.002-005, config.read.005 | add-server-skeleton |  |
| 106 | `hapi.fhir.fhirpath_interceptor_enabled` | bound | `hapi.fhir.fhirpath-interceptor-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 107 | `hapi.fhir.filter_search_enabled` | bound | `hapi.fhir.filter-search-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-special |  |
| 108 | `hapi.fhir.graphql_enabled` | bound | `hapi.fhir.graphql-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-graphql |  |
| 109 | `hapi.fhir.ig_runtime_upload_enabled` | bound | `hapi.fhir.ig-runtime-upload-enabled` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.008 | add-package-install |  |
| 110 | `hapi.fhir.implementationGuides` | bound | `hapi.fhir.implementation-guides` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.013 | add-package-install |  |
| 111 | `hapi.fhir.implementationguides.cql.installMode` | map entry | `hapi.fhir.implementation-guides.cql.install-mode` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 112 | `hapi.fhir.implementationguides.cql.installResourceTypes` | map entry | `hapi.fhir.implementation-guides.cql.install-resource-types` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 113 | `hapi.fhir.implementationguides.cql.name` | map entry | `hapi.fhir.implementation-guides.cql.name` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 114 | `hapi.fhir.implementationguides.cql.version` | map entry | `hapi.fhir.implementation-guides.cql.version` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 115 | `hapi.fhir.implementationguides.ips_1_0_0.installMode` | map entry | `hapi.fhir.implementation-guides.ips_1_0_0.install-mode` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install, add-ips | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 116 | `hapi.fhir.implementationguides.ips_1_0_0.name` | map entry | `hapi.fhir.implementation-guides.ips_1_0_0.name` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install, add-ips | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 117 | `hapi.fhir.implementationguides.ips_1_0_0.packageUrl` | map entry | `hapi.fhir.implementation-guides.ips_1_0_0.package-url` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install, add-ips | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 118 | `hapi.fhir.implementationguides.ips_1_0_0.version` | map entry | `hapi.fhir.implementation-guides.ips_1_0_0.version` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install, add-ips | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 119 | `hapi.fhir.implementationguides.swiss.installMode` | map entry | `hapi.fhir.implementation-guides.swiss.install-mode` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 120 | `hapi.fhir.implementationguides.swiss.name` | map entry | `hapi.fhir.implementation-guides.swiss.name` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 121 | `hapi.fhir.implementationguides.swiss.reloadExisting` | map entry | `hapi.fhir.implementation-guides.swiss.reload-existing` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 122 | `hapi.fhir.implementationguides.swiss.version` | map entry | `hapi.fhir.implementation-guides.swiss.version` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 123 | `hapi.fhir.implementationguides.uscql.installMode` | map entry | `hapi.fhir.implementation-guides.uscql.install-mode` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 124 | `hapi.fhir.implementationguides.uscql.installResourceTypes` | map entry | `hapi.fhir.implementation-guides.uscql.install-resource-types` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 125 | `hapi.fhir.implementationguides.uscql.name` | map entry | `hapi.fhir.implementation-guides.uscql.name` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 126 | `hapi.fhir.implementationguides.uscql.version` | map entry | `hapi.fhir.implementation-guides.uscql.version` | config.key.009, config.collection.003, config.key.005, config.key.006 | add-package-install | entry of `implementationGuides` (`PackageInstallationSpec`) |
| 127 | `hapi.fhir.index_storage_optimized` | bound | `hapi.fhir.index-storage-optimized` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 128 | `hapi.fhir.install_additional_resources_from_ig_folders` | bound, never read | `hapi.fhir.install-additional-resources-from-ig-folders` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 129 | `hapi.fhir.install_transitive_ig_dependencies` | bound | `hapi.fhir.install-transitive-ig-dependencies` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 130 | `hapi.fhir.ips_enabled` | bound | `hapi.fhir.ips-enabled` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.008, config.read.002 | add-ips |  |
| 131 | `hapi.fhir.language_search_parameter_enabled` | bound | `hapi.fhir.language-search-parameter-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 132 | `hapi.fhir.lastn_enabled` | bound | `hapi.fhir.lastn-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-special |  |
| 133 | `hapi.fhir.local_base_urls` | bound | `hapi.fhir.local-base-urls` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-conditional-and-integrity |  |
| 134 | `hapi.fhir.logger.error_format` | bound | `hapi.fhir.logger.error-format` | config.key.001, config.key.005, config.key.006, config.key.007, config.placeholder.009 | add-server-skeleton |  |
| 135 | `hapi.fhir.logger.format` | bound | `hapi.fhir.logger.format` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 136 | `hapi.fhir.logger.log_exceptions` | bound | `hapi.fhir.logger.log-exceptions` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 137 | `hapi.fhir.logger.name` | bound | `hapi.fhir.logger.name` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 138 | `hapi.fhir.logical_urls` | bound | `hapi.fhir.logical-urls` | config.key.001, config.key.005, config.key.006, config.key.007, config.collection.001, config.env.007 | add-store-conditional-and-integrity |  |
| 139 | `hapi.fhir.logical_urls.http` | not a key | `-` | config.key.011 | none | inventory artefact: the generator read a YAML comment (`# NOTE: ...`) or split a list value at `:`; see proposal, Inventory corrections |
| 140 | `hapi.fhir.logical_urls.https` | not a key | `-` | config.key.011 | none | inventory artefact: the generator read a YAML comment (`# NOTE: ...`) or split a list value at `:`; see proposal, Inventory corrections |
| 141 | `hapi.fhir.mark_resources_for_reindexing_upon_search_parameter_change` | bound | `hapi.fhir.mark-resources-for-reindexing-upon-search-parameter-change` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-custom-params |  |
| 142 | `hapi.fhir.mass_ingestion_mode_enabled` | bound | `hapi.fhir.mass-ingestion-mode-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 143 | `hapi.fhir.match_url_cache_enabled` | bound | `hapi.fhir.match-url-cache-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 144 | `hapi.fhir.max_binary_size` | bound | `hapi.fhir.max-binary-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-binary-and-offload |  |
| 145 | `hapi.fhir.max_page_size` | bound | `hapi.fhir.max-page-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 146 | `hapi.fhir.maximum_expansion_size` | bound | `hapi.fhir.maximum-expansion-size` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 147 | `hapi.fhir.mdm_enabled` | bound | `hapi.fhir.mdm-enabled` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.007, config.read.002 | add-mdm |  |
| 148 | `hapi.fhir.mdm_rules_json_location` | bound | `hapi.fhir.mdm-rules-json-location` | config.key.001, config.key.005, config.key.006, config.key.007 | add-mdm |  |
| 149 | `hapi.fhir.narrative_enabled` | bound | `hapi.fhir.narrative-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-narrative |  |
| 150 | `hapi.fhir.openapi_enabled` | bound | `hapi.fhir.openapi-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-openapi |  |
| 151 | `hapi.fhir.partitioning.allow_references_across_partitions` | bound | `hapi.fhir.partitioning.allow-references-across-partitions` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 152 | `hapi.fhir.partitioning.conditional_create_duplicate_identifiers_enabled` | bound | `hapi.fhir.partitioning.conditional-create-duplicate-identifiers-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 153 | `hapi.fhir.partitioning.database_partition_mode_enabled` | bound | `hapi.fhir.partitioning.database-partition-mode-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 154 | `hapi.fhir.partitioning.default_partition_id` | bound | `hapi.fhir.partitioning.default-partition-id` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 155 | `hapi.fhir.partitioning.partitioning_include_in_search_hashes` | bound | `hapi.fhir.partitioning.partitioning-include-in-search-hashes` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 156 | `hapi.fhir.partitioning.patient_id_partitioning_mode` | bound | `hapi.fhir.partitioning.patient-id-partitioning-mode` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 157 | `hapi.fhir.partitioning.request_tenant_partitioning_mode` | bound | `hapi.fhir.partitioning.request-tenant-partitioning-mode` | config.key.001, config.key.005, config.key.006, config.key.007 | add-partitioning |  |
| 158 | `hapi.fhir.patient_id_partitioning_mode` | not bound | `-` | config.unknown.003 | none | commented example at the wrong level; the bound key is `hapi.fhir.partitioning.patient_id_partitioning_mode` |
| 159 | `hapi.fhir.pre_expand_value_sets` | bound | `hapi.fhir.pre-expand-value-sets` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 160 | `hapi.fhir.pre_expand_value_sets_default_count` | bound | `hapi.fhir.pre-expand-value-sets-default-count` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 161 | `hapi.fhir.pre_expand_value_sets_max_count` | bound | `hapi.fhir.pre-expand-value-sets-max-count` | config.key.001, config.key.005, config.key.006, config.key.007 | add-terminology-store |  |
| 162 | `hapi.fhir.reindex_thread_count` | bound | `hapi.fhir.reindex-thread-count` | config.key.001, config.key.005, config.key.006, config.key.007 | add-maintenance-jobs |  |
| 163 | `hapi.fhir.remote_terminology_service` | bound | `hapi.fhir.remote-terminology-service` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.015 | add-terminology-operations |  |
| 164 | `hapi.fhir.remote_terminology_service.all.system` | map entry | `hapi.fhir.remote-terminology-service.all.system` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 165 | `hapi.fhir.remote_terminology_service.all.url` | map entry | `hapi.fhir.remote-terminology-service.all.url` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 166 | `hapi.fhir.remote_terminology_service.loinc.system` | map entry | `hapi.fhir.remote-terminology-service.loinc.system` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 167 | `hapi.fhir.remote_terminology_service.loinc.url` | map entry | `hapi.fhir.remote-terminology-service.loinc.url` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 168 | `hapi.fhir.remote_terminology_service.snomed.system` | map entry | `hapi.fhir.remote-terminology-service.snomed.system` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 169 | `hapi.fhir.remote_terminology_service.snomed.url` | map entry | `hapi.fhir.remote-terminology-service.snomed.url` | config.collection.003, config.key.005, config.key.006 | add-terminology-operations | entry of `remote_terminology_service` (`RemoteSystem`) |
| 170 | `hapi.fhir.request_tenant_partitioning_mode` | not bound | `-` | config.unknown.003 | none | commented example at the wrong level; the bound key is `hapi.fhir.partitioning.request_tenant_partitioning_mode` |
| 171 | `hapi.fhir.resource_dbhistory_enabled` | bound | `hapi.fhir.resource-dbhistory-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-history |  |
| 172 | `hapi.fhir.retain_cached_searches_mins` | bound | `hapi.fhir.retain-cached-searches-mins` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-core |  |
| 173 | `hapi.fhir.reuse_cached_search_results_millis` | bound | `hapi.fhir.reuse-cached-search-results-millis` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.008 | add-search-core |  |
| 174 | `hapi.fhir.search-coord-core-pool-size` | not bound | `-` | config.unknown.002 | none | nothing binds or reads it in HAPI 8.12.0 |
| 175 | `hapi.fhir.search-coord-max-pool-size` | not bound | `-` | config.unknown.002 | none | nothing binds or reads it in HAPI 8.12.0 |
| 176 | `hapi.fhir.search-coord-queue-capacity` | not bound | `-` | config.unknown.002 | none | nothing binds or reads it in HAPI 8.12.0 |
| 177 | `hapi.fhir.search_index_full_text_enabled` | bound | `hapi.fhir.search-index-full-text-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-fulltext |  |
| 178 | `hapi.fhir.search_prefetch_thresholds` | bound | `hapi.fhir.search-prefetch-thresholds` | config.key.001, config.key.005, config.key.006, config.key.007, config.coerce.022, config.env.008 | add-search-core |  |
| 179 | `hapi.fhir.server_address` | bound | `hapi.fhir.server-address` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 180 | `hapi.fhir.server_id_strategy` | bound | `hapi.fhir.server-id-strategy` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 181 | `hapi.fhir.store_meta_source_information` | bound | `hapi.fhir.store-meta-source-information` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 182 | `hapi.fhir.store_resource_in_lucene_index_enabled` | bound | `hapi.fhir.store-resource-in-lucene-index-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-fulltext |  |
| 183 | `hapi.fhir.subscription.cross_partition_enabled` | bound | `hapi.fhir.subscription.cross-partition-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 184 | `hapi.fhir.subscription.email.auth` | bound, never read | `hapi.fhir.subscription.email.auth` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 185 | `hapi.fhir.subscription.email.from` | bound | `hapi.fhir.subscription.email.from` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 186 | `hapi.fhir.subscription.email.host` | bound | `hapi.fhir.subscription.email.host` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 187 | `hapi.fhir.subscription.email.password` | bound | `hapi.fhir.subscription.email.password` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 188 | `hapi.fhir.subscription.email.port` | bound | `hapi.fhir.subscription.email.port` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 189 | `hapi.fhir.subscription.email.quitWait` | bound, never read | `hapi.fhir.subscription.email.quit-wait` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 190 | `hapi.fhir.subscription.email.startTlsEnable` | bound | `hapi.fhir.subscription.email.start-tls-enable` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 191 | `hapi.fhir.subscription.email.startTlsRequired` | bound, never read | `hapi.fhir.subscription.email.start-tls-required` | config.key.001, config.key.005, config.key.006, config.key.007, config.key.010 | none | HAPI 8.12.0 binds it and nothing reads it (config.key.010) |
| 192 | `hapi.fhir.subscription.email.username` | bound | `hapi.fhir.subscription.email.username` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 193 | `hapi.fhir.subscription.immediately_queued` | bound | `hapi.fhir.subscription.immediately-queued` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 194 | `hapi.fhir.subscription.polling_interval_ms` | bound | `hapi.fhir.subscription.polling-interval-ms` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 195 | `hapi.fhir.subscription.resthook_enabled` | bound | `hapi.fhir.subscription.resthook-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 196 | `hapi.fhir.subscription.websocket_enabled` | bound | `hapi.fhir.subscription.websocket-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-subscriptions-resthook |  |
| 197 | `hapi.fhir.supported_resource_types` | bound | `hapi.fhir.supported-resource-types` | config.key.001, config.key.005, config.key.006, config.key.007, config.coerce.021, config.env.006 | add-server-skeleton |  |
| 198 | `hapi.fhir.tester` | bound | `hapi.fhir.tester` | config.key.001, config.key.005, config.key.006, config.key.007, config.gate.016 | add-tester-ui |  |
| 199 | `hapi.fhir.tester.home.fhir_version` | map entry | `hapi.fhir.tester.home.fhir-version` | config.source.002, config.collection.003, config.key.005, config.key.006 | add-tester-ui | entry of `tester` (`Tester`) |
| 200 | `hapi.fhir.tester.home.name` | map entry | `hapi.fhir.tester.home.name` | config.source.002, config.collection.003, config.key.005, config.key.006 | add-tester-ui | entry of `tester` (`Tester`) |
| 201 | `hapi.fhir.tester.home.refuse_to_fetch_third_party_urls` | map entry | `hapi.fhir.tester.home.refuse-to-fetch-third-party-urls` | config.source.002, config.collection.003, config.key.005, config.key.006 | add-tester-ui | entry of `tester` (`Tester`) |
| 202 | `hapi.fhir.tester.home.server_address` | map entry | `hapi.fhir.tester.home.server-address` | config.source.002, config.collection.003, config.key.005, config.key.006 | add-tester-ui | entry of `tester` (`Tester`) |
| 203 | `hapi.fhir.upliftedRefchains_enabled` | bound | `hapi.fhir.uplifted-refchains-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-search-chaining |  |
| 204 | `hapi.fhir.use_apache_address_strategy` | bound | `hapi.fhir.use-apache-address-strategy` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 205 | `hapi.fhir.use_apache_address_strategy_https` | bound | `hapi.fhir.use-apache-address-strategy-https` | config.key.001, config.key.005, config.key.006, config.key.007 | add-server-skeleton |  |
| 206 | `hapi.fhir.userRequestRetryVersionConflictsInterceptorEnabled` | bound | `hapi.fhir.user-request-retry-version-conflicts-interceptor-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-store-crud |  |
| 207 | `hapi.fhir.validate_resource_status_for_package_upload` | bound | `hapi.fhir.validate-resource-status-for-package-upload` | config.key.001, config.key.005, config.key.006, config.key.007 | add-package-install |  |
| 208 | `hapi.fhir.validation.requests_enabled` | bound | `hapi.fhir.validation.requests-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-validator-core |  |
| 209 | `hapi.fhir.validation.responses_enabled` | bound | `hapi.fhir.validation.responses-enabled` | config.key.001, config.key.005, config.key.006, config.key.007 | add-validator-core |  |

## 6. Bound keys the inventory does not list

The inventory generator reads field declarations on one line and does not see
classes outside the starter, so it misses these. All of them bind today and
are covered by `config.key.001`, `.003`, `.005`, `.006` and `.007`.

| Key (as the starter documents or names it) | Canonical path | Bound by | Default | Wired to behaviour by |
|---|---|---|---|---|
| `hapi.fhir.normalized_quantity_search_level` | `hapi.fhir.normalized-quantity-search-level` | `AppProperties` (multi-line declaration, `AppProperties.java:106-107`) | `"NORMALIZED_QUANTITY_SEARCH_NOT_SUPPORTED"` | add-search-core |
| `hapi.fhir.cr.cql.compiler.error_severity_level` | `hapi.fhir.cr.cql.compiler.error-severity-level` | `CqlCompilerProperties` (`getErrorSeverityLevel`; the field is `error_level`, which does not bind) | `"Info"` | add-cql-translator |
| `hapi.fhir.cr.terminologyServerClientSettings.crmiVersion` | `hapi.fhir.cr.terminology-server-client-settings.crmi-version` | `TerminologyServerClientSettings` (cqf-fhir-utility 4.13.0) | `"1.0.0"` | add-cr-measures |
| `hapi.fhir.cr.terminologyServerClientSettings.expansionsPerPage` | `hapi.fhir.cr.terminology-server-client-settings.expansions-per-page` | same | `1000` | add-cr-measures |
| `hapi.fhir.cr.terminologyServerClientSettings.maxExpansionPages` | `hapi.fhir.cr.terminology-server-client-settings.max-expansion-pages` | same | `1000` | add-cr-measures |
| `hapi.fhir.cr.terminologyServerClientSettings.txResourceMode` | `hapi.fhir.cr.terminology-server-client-settings.tx-resource-mode` | same | `"AUTO"` | add-cr-measures |
| `hapi.fhir.cr.terminologyServerClientSettings.maxTxResourceCodeSystemConcepts` | `hapi.fhir.cr.terminology-server-client-settings.max-tx-resource-code-system-concepts` | same | `1000` | add-cr-measures |
| `hapi.fhir.implementationguides.<id>.packageUrl`, `.name`, `.version`, `.installMode`, `.installResourceTypes`, `.reloadExisting` (generic; the inventory lists them only for the packaged and commented ids) | `hapi.fhir.implementation-guides.<id>.*` | `PackageInstallationSpec` (hapi-fhir-storage 8.12.0) | config.key.009 | add-package-install |
| `hapi.fhir.implementationguides.<id>.fetchDependencies`, `.dependencyExcludes`, `.additionalResourceFolders`, `.versionPolicy`, `.dryRun`, `.overwriteContentNotPresentCodeSystems` | `hapi.fhir.implementation-guides.<id>.*` | same | config.key.009 | add-package-install |
| `hapi.fhir.tester.<id>.name`, `.server_address`, `.refuse_to_fetch_third_party_urls`, `.fhir_version` (generic) | `hapi.fhir.tester.<id>.*` | `AppProperties.Tester` | config.key.002, config.collection.003 | add-tester-ui |
| `hapi.fhir.remote_terminology_service.<id>.system`, `.url` (generic) | `hapi.fhir.remote-terminology-service.<id>.*` | `AppProperties.RemoteSystem` | `null`, `null` | add-terminology-operations |
| `hapi.fhir.cr.cql.namespaces.<key>` (generic) | `hapi.fhir.cr.cql.namespaces[<key>]` | `CqlProperties` | none | add-cql-engine |
| `hapi.fhir.implementationguides.<id>.packageContents` | `hapi.fhir.implementation-guides.<id>.package-contents` | `PackageInstallationSpec.setPackageContents(byte[])` (`PackageInstallationSpec.java:210`); a comma list of byte values (config.key.012) | `null` | add-package-install (HAPI installs from the bytes: `JpaPackageCache.java:586,596`, `AdditionalResourcesParser.java:51`) |
| `spring.elasticsearch.uris`, `.username`, `.password`, `.connection-timeout`, `.socket-timeout`, `.socket-keep-alive`, `.path-prefix`, `.restclient.sniffer.interval`, `.restclient.sniffer.delay-after-failure`, `.restclient.ssl.bundle` | the same, under the descriptor of `ElasticsearchProperties` | Spring Boot 3.5.14 `ElasticsearchProperties`, bound by `ElasticConfigCondition` (config.gate.024) | `["http://localhost:9200"]`, `null`, `null`, `1s`, `30s`, `false`, `null`, `5m`, `1m`, `null` | add-search-fulltext (gate only; Frevo never connects to Elasticsearch) |

Keys the starter reads by name rather than binding (config.read.001):

| Key | Read by | Covered by | Wired to behaviour by |
|---|---|---|---|
| `hapi.fhir.custom-bean-packages` | `@ComponentScan(basePackages = {"${hapi.fhir.custom-bean-packages:}"})`, `StarterJpaConfig.java:109` | config.read.001, config.read.003 | add-server-skeleton (Java packages cannot be scanned by Frevo; see proposal, Out of scope) |
| `spring.ai.mcp.server.enabled` | `@ConditionalOnProperty`, `McpServerConfig.java:30-33` | config.read.001, config.gate.011 | add-mcp |
| `spring.elasticsearch.*` | `ElasticConfigCondition` (binds `ElasticsearchProperties`) | config.gate.020, config.gate.024, config.profile.002 | add-search-fulltext (Frevo never connects to Elasticsearch; `tantivy` replaces it, constitution section 5) |
| `scheduling_disabled` | HAPI itself, by exact name: `BaseSchedulerServiceImpl.java:67,319` (`"true".equals(...)` turns off every scheduled job) and `SubscriptionMatcherInterceptorConfig.java:47` (swaps in the synchronous subscription matcher) | config.read.001, config.read.008, config.gate.021 | add-jobs-engine, add-subscriptions-resthook |

Keys the starter defines that fail to start when set:

| Key | Why | Covered by |
|---|---|---|
| `hapi.fhir.remote_terminology_services_map.*` | `getRemoteTerminologyServicesMap()` has no setter and returns `null` | config.unknown.006 |
| `hapi.fhir.cr.cql.runtime.enable_validation` (any value but `false`) | `CqlRuntimeProperties` declares `EnableValidation(boolean)`, which is not a setter | config.unknown.005 |
| `hapi.fhir.cr.cql.data.retrieve_settings.*`, `hapi.fhir.cr.cql.retrieve_settings.*` (any value) | `getRetrieveSettings()` has no setter and returns a new object on every call (`cr/CqlData.java:39`, `cr/CqlProperties.java:70`) | config.unknown.008 |
| `hapi.fhir.cr.cql.terminology.terminology_settings.*` (any value) | `CqlTerminologyProperties.getTerminologySettings()` likewise (`cr/CqlTerminologyProperties.java:35`) | config.unknown.008 |

## 7. Corrections the inventory needs

`docs/inventory.md` is generated (`harness/scripts/gen-inventory.py`) and can
only be corrected on a `chore/*` branch. Until it is, this file is the
authority for the configuration rows, and tasks.md carries the chore as a
prerequisite of archiving. The corrections:

1. **Remove three artefact rows**: `hapi.fhir.NOTE` (a YAML comment, `# NOTE:
   Extended Lucene/Elasticsearch indexing is experimental.`) and
   `hapi.fhir.logical_urls.http` / `.https` (list values split at `:`).
2. **Reclassify as bound by a properties class**:
   `hapi.fhir.store_meta_source_information` and `hapi.fhir.logger.format`
   (both multi-line declarations), the four
   `hapi.fhir.cr.terminologyServerClientSettings.*` rows, the three
   `hapi.fhir.cr.cql.data.*_mode` and the four
   `hapi.fhir.cr.cql.terminology.*_mode` rows (relaxed spellings of bound
   properties).
3. **Reclassify as not bound**: `hapi.fhir.cr.cql.compiler.error_level` (the
   bean property is `errorSeverityLevel`), the three
   `hapi.fhir.search-coord-*` rows, and the top-level
   `hapi.fhir.database_partition_mode_enabled`,
   `hapi.fhir.patient_id_partitioning_mode` and
   `hapi.fhir.request_tenant_partitioning_mode`.
4. **Add** the keys of section 6.
5. **Read bean properties, not fields.** The generator should take key names
   from getter and setter names, as Spring's `JavaBeanBinder` does, not from
   field names; that is the root cause of items 2 and 3.

The parity denominator therefore changes from 209: three rows go, and the
bound keys of section 6 come in. #39 applied items 1 to 4 to
`docs/inventory.md` by hand: it now has **220** configuration rows (209 - 3
+ 14), and config.key.011 checks every one of them against sections 5 and 6.
Item 5, the generator fix, is still owed (task 6, a `chore/*` prerequisite of
archiving; owner ruling of 2026-11-22).
