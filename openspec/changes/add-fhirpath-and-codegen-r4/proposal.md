# add-fhirpath-and-codegen-r4

## Why

Everything else in the project depends on this change.

`frevo-model` decides whether the constitution's first performance principle —
"store and serve canonical resource JSON **as bytes**; a read that returns a
resource unchanged must not parse or re-serialize it" — is achievable at all.
If it is not, the whole `read-by-id` target of 14× HAPI (see
[`bench/targets.md`](../../../bench/targets.md)) is wrong, and it is far cheaper
to learn that now than after `frevo-store` is built on top of it.

`frevo-path` is on the **write** path for every resource stored: search
parameter extraction evaluates a FHIRPath expression per enabled parameter, and
validation invariants and CQL evaluate FHIRPath too. An engine that silently
returns an empty collection instead of failing is the worst available failure
mode for that job, which is the finding that drove
[ADR 0004](../../../docs/adr/0004-fhirpath-reuse-or-rewrite.md).

`frevo-codegen` exists because constitution rule 4 requires it: models are
**regenerated from StructureDefinitions, never translated from upstream Java**,
and the search-parameter tables come from the same inputs so that the model and
the index agree by construction.

## What changes

Three crates, in dependency order.

### 1. `frevo-codegen`

A generator binary that reads a pinned FHIR R4 definition package and emits the
`frevo-model` sources. Checked-in output carrying an `@generated` marker, not a
`build.rs` step, so that a reviewer can read the model and `openspec-guard` can
see the marker.

Inputs, measured from the pinned copy in
`reference/hapi-fhir/hapi-fhir-validation-resources-r4/src/main/resources/org/hl7/fhir/r4/model/`:

| Input file | Content |
|---|---|
| `profile/profiles-types.xml` | 63 StructureDefinitions — primitive and complex datatypes |
| `profile/profiles-resources.xml` | 149 StructureDefinitions — 146 concrete resources, 2 abstract (`Resource`, `DomainResource`), 1 logical |
| `profile/profiles-others.xml` | 44 StructureDefinitions — the remaining R4 profiles |
| `extension/extension-definitions.xml` | the R4 standard extension definitions |
| `sp/search-parameters.json` | 1,375 SearchParameter resources |

The authoritative input is the published `hl7.fhir.r4.core#4.0.1` package,
pinned by SHA-256; the HAPI copy above is the cross-check. `fhirVersion` in all
four profile files reads `4.0.1`, which is what makes the two sources
comparable.

Only R4 models are in scope. The search-parameter **tables** are generated here
(name, code, type, base, expression, target, composite components,
multiple-or/and flags) because they come from the same input bundle and because
regenerating them later from a different input would let the model and the index
drift; the **extraction and indexing** of those parameters is not in scope.

### 2. `frevo-model`

The generated R4 resources and datatypes, plus a hand-written codec that is the
point of the whole exercise:

- A resource is held as canonical JSON **bytes** plus a lazily built index.
  Returning it unchanged is a byte copy — no parse, no re-serialize.
- Parsing is **lazy**: materialising `Patient.birthDate` must not materialise
  `Patient.contained`.
- Mutation is copy-on-write: a modified subtree becomes owned, and
  serialisation splices the original bytes back for every subtree that was not
  touched.
- FHIR's primitive-extension form (the `_field` sibling) is represented without
  forcing materialisation of either side.
- `frevo-model` implements the navigation trait that `frevo-path` evaluates
  over, so the engine never sees a concrete model type.

### 3. `frevo-path`

A FHIRPath parser, a static type-check pass, and an evaluation engine, written
against three traits — node navigation, model type information, and host
services — and **not** against `frevo-model`'s concrete types. ADR 0004
decision 5 requires this, and the reason is specific: `haste-fhirpath` contains
77 `downcast_ref` calls to its own model's concrete types, and FHIRPath's
semantics turn a failed downcast into an empty collection rather than an error.

HAPI configures the same engine two different ways, and both are observable:

| | `doNotEnforceAsSingletonRule` | `doNotEnforceAsCaseSensitive` |
|---|---|---|
| `FhirPathR4` (what the server uses) | `true` | `true` |
| `FHIRPathTests` (what the suite grades) | `false` | `false` |

`frevo-path` must therefore expose both, defaulting to HAPI's server
configuration, and the oracle harness must select the strict one.

## Upstream module and pinned SHA for each piece

Per `rules.proposal`. Pins are from
[`reference/PINS.md`](../../../reference/PINS.md).

| Piece | Upstream repo | Upstream path | Pinned SHA | Relationship |
|---|---|---|---|---|
| `frevo-path` parser, type-check and evaluator | `hapifhir/org.hl7.fhir.core` | `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java` (7,008 lines), `ExpressionNode.java`, `TypeDetails.java`, `FHIRLexer.java` | `73c432b5f43ddbce979978a760ec4aa81e929094` | **derived** — behaviour ported, provenance header required |
| `frevo-path` public API and strictness defaults | `hapifhir/hapi-fhir` | `hapi-fhir-base/src/main/java/ca/uhn/fhir/fhirpath/IFhirPath.java`, `hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | **derived** |
| `frevo-path` UCUM unit algebra | `hapifhir/org.hl7.fhir.core` | `org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml` (also fetched at `oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml`) | `73c432b5f43ddbce979978a760ec4aa81e929094` | **data input**, not derived code |
| `frevo-model` R4 resources and datatypes | — | generated from `hl7.fhir.r4.core#4.0.1` StructureDefinitions | n/a (package digest, not a commit) | **generated** — not a translation of `org.hl7.fhir.r4.model`, per constitution rule 4 |
| `frevo-model` JSON codec shape | `hapifhir/org.hl7.fhir.core` | `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/formats/JsonParser.java`, `JsonCreatorCanonical.java` | `73c432b5f43ddbce979978a760ec4aa81e929094` | **derived** — the `_field` representation and the canonical key ordering are read from here |
| `frevo-codegen` generator structure | `hapifhir/hapi-fhir` | `hapi-tinder-plugin` (`src/main/resources/vm/resource.vm`, `ts_enum.vm`, `valueset.vm`) | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | **reference only** — read for what HAPI chooses to generate, not ported |
| `frevo-codegen` definition inputs | `hapifhir/hapi-fhir` | `hapi-fhir-validation-resources-r4/src/main/resources/org/hl7/fhir/r4/model/**` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` | **data input** cross-check |

`reference/` is read-only and git-ignored, so the generator must not read it at
build time; it reads the pinned, digest-verified package. The `reference/` copy
is used by a test that asserts the two agree.

## Oracles

Per `rules.proposal`, by name and case selector.

| Scenario group | Oracle | Case selector |
|---|---|---|
| `path.*` (everything except `path.perf.*`) | `oracles/fhirpath` — `tests-fhir-r4.xml`, pinned `c95adbee099c875de97d25ed7616aae7d66dd85b` | all 1,191 `<test>` elements across 103 `<group>`s, addressed as `<group name>/<test name>`; the gate is **pass rate at or above HAPI 8.12.0's measured rate on the same cases** |
| `path.check.*` | same suite | the 29 `<expression invalid="semantic">` cases plus the `isSemanticChecks` group |
| `path.errors.*` | same suite | the 27 `<expression invalid="execution">` and 7 `<expression invalid="syntax">` cases |
| `path.quantity.*`, `path.boundary.*` | same suite | groups `testQuantity`, `testQuantityUnitComparison`, `Comparable`, `LowBoundary`, `HighBoundary`, `Precision` |
| `path.*` cross-check | `helios-fhirpath` 0.2.7 as a **differential reference**, per ADR 0004 decision 3 | same 1,191 cases; a disagreement is a lead, never the standard |
| `model.*`, `codegen.roundtrip.*` | `r4-example-roundtrip` — parse, canonically re-serialize, compare byte-for-byte | every example in `hl7.fhir.r4.examples#4.0.1`; the 103 examples already fetched at `oracles/fhir-test-cases/fetched/r4/examples/` are the interim selector until the full package is pinned (see Prerequisites) |
| `model.primext.*` | `r4-example-roundtrip` plus the FHIRPath suite's own inputs | `patient-name-extensions.json`, `patient-container-example.json`, `parameters-example-decimals.xml` |
| `codegen.*` (other than round-trip) | **self-checking**: regeneration determinism and completeness against the input bundle | differential-free; the generator's own output is the oracle |
| `*.perf.*` | `bench/micro/` (new, in-process) | named in [`design.md`](design.md); the HTTP workloads in `bench/workloads.yaml` cannot run until `frevo-server` exists |

### The suite's size, and how it is counted

**1,191 `<test>` elements in 103 `<group>`s.** That is what an XML parse of the
pinned file returns, and the harness asserts it that way (`path.oracle.001`).

A text match over the same file gives 104 and 1,193, and the first draft of this
proposal used those figures. They are wrong: the `testDollarResource` group at
`tests-fhir-r4.xml:1576-1582` is wrapped in an XML comment, and its bodies are
leftover Java rather than well-formed test elements, so grepping for `<test `
matches inside the comment. `oracles/run.sh` counted the same way and has been
corrected on `main`.

Two consequences carried into the delta. `documentFunctions` is **115** cases,
not 114. And two scenarios that named `testDollarResource` as their grading
selector had to be re-pointed, because that group is not in the suite:
`path.env.006` now grades `%resource` against the six live `isSemanticChecks`
cases that reference `%resource.status`, and `path.navigation.021` grades
`descendants()` against `testDistinct/testDistinct2`, `3`, `5` and `6`,
`testRepeat/testRepeat3` and `testCombine()/testCombine1`.

The `invalid` counts are unaffected, because all of them lie outside the
comment: 7 `syntax`, 29 `semantic`, 27 `execution`.

### Where the oracle's input files actually live

`tests-fhir-r4.xml` references 15 distinct `inputfile` values. All 15 exist in
the **root** of the already-fetched tree:

```
oracles/fhir-test-cases/fetched/r4/
```

That is the directory `TestingUtilities.loadTestResource("r4", input)` resolves
to in `org.hl7.fhir.r4.test.FHIRPathTests`, so using it keeps the harness
comparable with HAPI's. Note that `patient-example.xml` and
`observation-example.xml` *also* exist under `fetched/r4/examples/`; the harness
must use the root copies, because that is what upstream loads.

| `inputfile` | Cases |
|---|---:|
| `patient-example.xml` | 867 |
| `bundle-document-ips.json` | 114 |
| `observation-example.xml` | 70 |
| `questionnaire-example.xml` | 9 |
| `parameters-example-decimals.xml` | 9 |
| `parameters-example-quantities.xml` | 8 |
| `valueset-example-expansion.xml` | 7 |
| `parameters-example-types.xml` | 5 |
| `parameters-quantity-unit-text.xml` | 3 |
| `appointment-examplereq.json` | 3 |
| `patient-example-period.xml` | 2 |
| `patient-name-extensions.json` | 1 |
| `patient-container-example.json` | 1 |
| `explanationofbenefit-example.json` | 1 |
| `codesystem-example.xml` | 1 |
| *(no `inputfile`)* | 90 |

The suite is XML, and six of the 15 inputs are JSON. The harness therefore
needs a FHIR **XML** reader for its inputs even though the product's codec is
JSON. That reader is test-only: it exists to turn an oracle input into a
navigable node, is not part of `frevo-model`'s public surface, and is not on
any request path.

## Prerequisites this change does not create

1. **The `r4-example-roundtrip` oracle does not exist yet.** `oracles/PINS.md`
   pins six suites and none of them is the R4 example package;
   `oracles/fetch.sh` has one line, for `fhirpath`. The full R4 example set
   (`hl7.fhir.r4.examples#4.0.1`) must be added to `oracles/PINS.md` and
   `oracles/fetch.sh` on a `chore/oracles-*` branch, because the constitution
   restricts `oracles/**` to that branch. Until it is, the round-trip gate runs
   against the 103 examples in the already-fetched `fhir-test-cases` tree, and
   that is a weaker gate, not a passing one.
2. **`ucum-essence.xml` needs a NOTICE entry.** It is fetched, not vendored,
   inside the Apache-2.0 `fhir-test-cases` tree but carries Regenstrief's own
   UCUM copyright notice. Same `chore/oracles-*` branch.

Both are recorded as tasks here so they are not lost, and both are owner
checkpoints because they change what the gates measure.

## Out of scope

Each item names the later change that covers it, per `rules.proposal`.

| Out of scope | Why | Covered by |
|---|---|---|
| DSTU2, DSTU3, R4B and R5 models | the generator is written to be version-driven, but only the R4 inputs are pinned and only R4 is graded here | `add-model-r5`, `add-model-dstu3`, `add-model-dstu2`, `add-model-r4b` |
| Cross-version converters | need two generated models to convert between | `add-convert-crossversion` |
| **Search-parameter extraction and indexing** | the tables are generated here; turning a resource into index rows is a different capability with a different oracle | `add-search-params` |
| The `_fhirpath` query parameter and `hapi.fhir.fhirpath_interceptor_enabled` | an HTTP interceptor (`FhirPathFilterInterceptor` in `StarterJpaConfig`), not an engine feature; the key stays spelled exactly as the starter spells it | `add-server-skeleton` |
| FHIRPath Patch, JSON Patch, `$diff` | the three `frevo-path` rows in `docs/inventory.md` marked phase 2 | `add-store-patch`, `add-store-diff` |
| `memberOf()` resolving against a real terminology server | the engine calls the host service; the host service needs `frevo-terminology`. In scope here: the call is made, and an absent host service produces an empty collection exactly as HAPI's does | `add-terminology-validate-code` |
| `conformsTo()` resolving against a real profile | same shape. In scope here: the host-service call and HAPI's error when the profile is unknown | `add-validator-profiles` |
| `resolve()` resolving anything that is not a `#id` reference | the R4 engine's `funcResolve` resolves a `#`-prefixed reference itself, against the **root** resource's `contained` list, and delegates every other form — including a `Bundle` entry `fullUrl` — to the host service. It has no Bundle-entry lookup. In scope here is the `#id` path and the delegation; the host service that answers the rest is `frevo-store`'s. **No live case in the pinned suite calls `resolve()`**, so every `resolve()` scenario is differential-only | `add-store-crud` |
| `htmlChecks()`, `htmlChecks2()`, `hasTemplateIdOf()` | engine-local extensions used only by the validator and by CDA; no oracle case in `tests-fhir-r4.xml` exercises them | `add-validator-narrative` |
| The `documentFunctions` group — `section()`, `entry()`, `entryIn()`, `resources()`, `resourcesIn()`, `sameSubject()`, `hasCoding()` | **the R4 engine does not implement them.** `ExpressionNode.Function.fromCode` in the R4 source has no case for any of the seven; the R5 source has only `section` and `entry`. The group is `mode="custom"` and its own description says engines "run them only if they choose to implement the functions". All 115 cases are therefore failures for HAPI 8.12.0 too, so implementing them would *exceed* the parity gate rather than meet it | `add-ips-document-functions` |
| Narrative generation, validation invariants, CQL | consumers of this engine | `add-narrative`, `add-validator-core`, `add-cql-engine` |
| PostgreSQL storage of the byte-preserved resource | this change produces the codec; the column type, the migration and the compression choice belong with the schema | `add-store-schema` |

## Target crates

`frevo-codegen`, `frevo-model`, `frevo-path` — the three `crates` already listed
for this change in [`tasks/graph.yaml`](../../../tasks/graph.yaml), which also
names the oracles `fhirpath` and `r4-example-roundtrip`.

## Risks

1. **ADR 0004 is provisional on numbers nobody has.** It says so explicitly.
   Tasks 1 and 2 of [`tasks.md`](tasks.md) exist to produce them, and the ADR is
   updated with the measured pass rates **before** any engine code is written.
   If `helios-fhirpath` turns out to be at or above HAPI's rate and
   implementable over the lazy model, decision 1 reverses and most of this
   change's `path.*` work disappears. Sequencing the measurement first is the
   only way that saving is still available.
2. **HAPI's own pass rate is unknown and is the gate.** The `documentFunctions`
   group alone is 115 of 1,191 cases (9.7%) that the R4 engine cannot pass, and
   the R4 harness ignores the `mode`, `context`, `contextType` and `version`
   attributes that the R5 harness honours. The measured HAPI number, not an
   assumption, is what task 2 records.
3. **Generated-code volume.** 146 concrete resources with a full codec each
   could produce a crate that is slow to compile and unpleasant to review.
   `design.md` sets a compile-time budget for exactly this reason.
4. **Ten HAPI-versus-specification divergences are recorded, and none is a
   deviation from HAPI.** `DEV-0001` to `DEV-0010` in
   [`docs/deviations.md`](../../../docs/deviations.md) record places where the
   pinned R4 engine does not do what the FHIRPath specification describes:
   `single()` on an empty collection throws, a multi-item ordered comparison
   returns empty, `<` rejects `unsignedInt` and `positiveInt` while its three
   mirrors accept them, comparison and the transcendental math functions go
   through binary doubles, incommensurable quantities compare by magnitude, a
   string function over a non-string focus returns empty, `extension(url)` also
   returns modifier extensions, `now()` is not stable within an evaluation,
   `toDate()` does not exist, and — the one with the widest blast radius — a
   host application constant resolved implicitly at the entry of an expression
   **shadows a real element of the resource**, so a host service that answers
   for `name` makes `Patient.name` unreachable through the expression `name`.
   That last one is live on the write path, because search-parameter extraction
   and the validator both pass an application context, which is the condition
   the engine gates it on. Every one is resolved as HAPI's behaviour, so the
   port stays faithful; the entries exist because rule 1 of the constitution
   requires a HAPI-versus-specification disagreement to be recorded, and because
   each of these is something an implementer reading the specification would
   otherwise "fix" into a parity break. They are a phase-boundary checkpoint and
   need owner review.

   One difference is deliberate and is marked configurable rather than HAPI:
   `today()` and `now()` read the evaluation time zone from engine
   configuration instead of from the process default. HAPI reads the JVM
   default, which is why its own test harness has to call `TimeZone.setDefault`
   before every case. Reading it from configuration produces the same results
   for the same zone and removes a source of flake from every gate. It is
   recorded under `DEV-0008`.

5. **Request parsing is lenient, which is a match with HAPI and not a
   deviation.** The starter does not parse request bodies with
   `org.hl7.fhir.r4.formats.JsonParser`; it uses `ca.uhn.fhir.parser.JsonParser`
   with the handler `FhirContext` installs by default, which is
   `LenientErrorHandler`. That handler logs and continues for an unknown
   element, an unknown attribute, an incorrect JSON type, an unexpected
   repeating element, a missing required element, a contained resource with no
   `id`, a `#` reference whose target is not contained and — on the encode side
   — a `#` reference being serialised with no matching contained resource; its
   own documentation states that only `invalidValue` and
   `extensionContainsValueAndNestedExtensions` abort a parse. That is the
   **complete** set: HAPI's parser error surface has exactly ten conditions,
   nine on decode and one on encode, and this change specifies all ten in both
   modes, because strict mode one condition short of HAPI's is a parity break
   the round-trip oracle cannot see. So HAPI 8.12.0
   answers `2xx` to a `Patient` carrying `"nmae": []`, to `"active": "true"`,
   to a single object where a `0..*` array belongs, to an array where a `0..1`
   value belongs, and to an `Observation` with no `status`. `frevo-model`
   therefore decodes leniently by default and strictly on request, which is
   exactly the choice HAPI itself offers through `StrictErrorHandler` and
   `FhirContext.setParserErrorHandler`. The validator will want the strict
   mode; the HTTP surface wants the lenient one.
