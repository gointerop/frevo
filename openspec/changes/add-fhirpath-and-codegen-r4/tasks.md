# Tasks — `add-fhirpath-and-codegen-r4`

Ordered. Each is roughly half a day and independently verifiable. Every task
that changes behaviour names the scenario IDs it satisfies, per
`openspec/config.yaml` `rules.tasks`.

**On the ordering of tasks 1 to 3.** `rules.tasks` says the first task is always
writing the failing tests for the scenarios. Tasks 1 and 2 come before that for
one reason: ADR 0004 is **accepted, provisional**, and it names task 1 of this
change as what builds the harness and task 2 as what measures the numbers the
decision rests on. If task 2 reverses the decision, most of the `path.*` tests
written in task 3 would be tests of the wrong thing. So task 1 builds the
harness (which is test infrastructure, not implementation), task 2 measures, and
task 3 writes every failing test. No task after 3 starts before its scenarios
have failing tests.

**Role boundaries.** Tasks 1 to 3 are `oracle-builder` work (tests and
`harness/**`). Tasks 4 onward are `implementer` work (`crates/**`). Tasks marked
**[chore]** cannot be done on this branch at all — `oracles/**` is restricted to
`chore/oracles-*` — and are listed here so they are not lost; each is an owner
checkpoint because it changes what a gate measures.

---

## Phase A — the oracle harness and the measurement ADR 0004 is waiting for

- [x] **1. Build the FHIRPath oracle harness.** Parse
  `oracles/fhirpath/tests-fhir-r4.xml` into cases: assert 103 groups and 1,191
  `<test>` elements **counted by XML parse, not by text match** — a grep says
  104 and 1,193 because the `testDollarResource` group at lines 1576-1582 is
  inside an XML comment whose bodies still contain `<test ...>` text — and read
  every attribute, **per element**, that the pinned file actually carries:
  `<tests>` takes `name`, `description`, `reference`; `<group>` takes `name`,
  `description`, `mode`, `reference`; `<test>` takes `name`, `description`,
  `inputfile`, `mode`, `context`, `contextType`, `version`, `testing`,
  `predicate`, `ordered`, `checkOrderedFunctions`; `<expression>` takes
  `invalid` and `mode`; `<output>` takes `type`. Fail the run on an attribute an
  element does not carry in the pinned file, and on an attribute value outside
  the recorded domain, and assert that a run against the pinned file reports
  **zero** unhandled attributes — `reference` on the root and on
  `<group name="from-Zulip">` (line 1583) and `mode` on the one `<expression>`
  (line 625) are the three a thirteen-attribute inventory misses and that would
  otherwise abort the run. Handle the expression-level `mode` by recording it
  and letting it change no grading: `testIif/testIif6` must fail the check pass
  whatever `mode` says, because the engine types `iif`'s condition against a
  singleton boolean with no mode gate. Load each of the 15 referenced input
  files from
  `oracles/fhir-test-cases/fetched/r4/` (the directory
  `TestingUtilities.loadTestResource("r4", input)` resolves to), both the XML
  and the JSON ones, through a **test-only** reader. Implement the three-stage
  run — parse, check, evaluate — with failures attributed to the right stage,
  the `predicate="true"` boolean conversion, the `ordered="false"` unordered
  comparison, the count/type/value comparison and the `Quantity` deep
  comparison, and a per-case status of `passed`, `failed`, `did-not-run` or
  `skipped-for-licence`. The two comparison branches use **different** renderers
  and must not be unified: the ordered branch compares against the FHIRPath
  string value, which prefixes `@` for a `date`, `dateTime` or `instant` and
  `@T` for a `time` — which is why the suite's expectations read `@1974-12-25` —
  while the unordered branch compares the plain string value with no prefix and
  matches each item against any expected element on type-and-value. Never strip
  the `@` from an expectation or normalise it away on either side; that is
  weakening the oracle under constitution rule 3. Fix the evaluation time zone at
  offset 0 (UTC), which is what `FHIRPathTests.test` actually runs at: its
  `TimeZone.getTimeZone("UTC+1100")` is an id Java does not recognise and
  returns `GMT` (`path.datetime.004`, revised 2026-11-20; the harness constant
  `SUITE_TIME_ZONE_OFFSET_MINUTES` is 0, not 660). The harness runs against a trait object, so it
  works before any engine exists.
  Satisfies `path.oracle.001`, `path.oracle.002`, `path.oracle.003`,
  `path.oracle.004`, `path.oracle.005`, `path.oracle.006`, `path.oracle.007`,
  `path.oracle.008`, `path.oracle.009`, `path.oracle.010`, `path.oracle.015`,
  `path.oracle.017`, `path.datetime.004`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** all 13 IDs carry a `covers:`
  tag (`scenario-coverage.sh --change add-fhirpath-and-codegen-r4`: 514 of
  514 covered, 0 unresolved) and every test tagged with one passes:
  `fhirpath_oracle` 19 of 19, `path_datetime` 6 of 6, and `path_oracle`'s
  three tests for this task (its two red ones are `path.oracle.012`, task 3,
  and `.013`, task 2).

- [x] **2. Measure `helios-fhirpath` and `haste-fhirpath` through the harness and
  record the numbers in ADR 0004.** Create `harness/fhirpath-compare/`, listed
  in `[workspace] exclude` so `haste-fhirpath`'s wildcard dependencies never
  enter the product graph or the `cargo deny` gate. Run all 1,191 cases through
  `helios-fhirpath` 0.2.7 and `haste-fhirpath` 0.129.3, record per-case pass or
  fail and the wall-clock of a full run, and establish whether
  `IntoEvaluationResult` can be implemented over a lazy, span-based node without
  materialising whole resources. Write the measured pass rates, the throughput
  and that finding into ADR 0004's "What would change this" section, and state
  the resulting verdict: keep decision 1, or reverse it. **Stop for the owner
  here** — this is the checkpoint the ADR asks for, and reversing it changes the
  scope of every task below.
  Satisfies `path.oracle.013`.
  **Ticked 2026-11-21, owner decision.** Measured through
  `harness/fhirpath-compare/` (`results.json`, `README.md`). Over 1,191 cases:
  - HAPI passes 968.
  - Frevo passes 984, with 0 cases behind HAPI, in 0.28 s.
  - `helios-fhirpath` 0.2.7 passes 1,047, with 27 behind HAPI, in 0.38 s.
  - `haste-fhirpath` 0.129.2 (0.129.3 failed the 14-day rule) passes 878, with
    177 behind HAPI, in 1.08 s.

  `IntoEvaluationResult` cannot be implemented lazily. The owner's verdict,
  recorded in ADR 0004's "Outcome of task 2", is to keep Frevo's own engine.
  `path.oracle.013` passes.

- [x] **3. Measure HAPI 8.12.0's own pass rate on the same 1,191 cases and wire
  it into the gate.** Run `org.hl7.fhir.r4.test.FHIRPathTests` at pin
  `73c432b5` against the same suite file, record each case's status, and load
  that record into the harness so every case reports Frevo's status beside
  HAPI's. Implement the gate as "Frevo's `passed` count at least HAPI's", with
  every case HAPI passes and Frevo fails listed individually regardless of the
  aggregate. Confirm and record that the 115 `documentFunctions` cases all fail
  at parse for HAPI, because the R4 `ExpressionNode.Function` enum implements
  none of the seven functions. That leaves 111 `failed` and the 4
  `invalid="syntax"` ones (`testSection13`, `testEntry12`, `testSameSubject16`,
  `testHasCoding10`) `passed`, for HAPI and Frevo alike (`path.oracle.014`). Record a status of `hapi-cannot-pass`,
  distinct from `failed`, for every case in a group whose expressions are
  byte-identical, which share an `inputfile`, which differ only by
  `context`/`contextType`, and whose expected outcomes differ. The pinned R4
  harness reads neither attribute and so necessarily fails at least one case of
  each group. Computed mechanically over the pinned suite, that is six groups
  and 13 cases, all in `isSemanticChecks`:
  `isEffectOnLogic2Base`/`isEffectOnLogic3Base`,
  `isGuardOr`/`isGuardOr2`/`isGuardOrBad`, `isGuardXorBad`/`isGuardXorBad2`,
  `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad`,
  `isGuardOrTypesImpliesBad`/`isGuardOrTypesImpliesAll` and
  `isGuardNotImplies`/`isGuardNotImpliesBad`. Two groups share an expression
  but have identical expected outcomes, so they are not `hapi-cannot-pass`:
  `isEffectOnLogic1`/`isGuardImpliesEmpty` and
  `isEffectOnLogic2`/`isEffectOnLogic3`. A group is not always `passed` on
  Frevo's run either: `isGuardOr`, `isGuardOrInnerImplies` and
  `isGuardNotImplies` fail the check, because the check pass does not narrow
  (`path.oracle.016`).
  Satisfies `path.oracle.011`, `path.oracle.012`, `path.oracle.014`,
  `path.oracle.016`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.oracle.011`, `.012`, `.014` and `.016` passes (`path_oracle`,
  `fhirpath_oracle`). The gate test passes with 16 cases ahead of HAPI and none
  behind, and the suite run has 0 `did-not-run` cases (984 passed, 207 failed,
  `bench/micro/results.json`).

- [x] **4. Write the failing tests for every `path.*` scenario.** One test per
  scenario ID, each carrying a `// covers: path.<area>.<nnn>` comment, with the
  oracle-backed scenarios driven from the harness's case selectors and the rest
  written directly. All 368 must fail, and none by `todo!()`, `#[ignore]` or a
  bare `#[should_panic]`. Where a scenario says it is differential-only, the
  test is written against a recorded HAPI response rather than against a suite
  case, and says so. `path.check.015`-`019` are driven from the in-process
  `path-check-warnings` oracle of task 4a, not from a suite selector.
  Covers all 368 `path.*` scenarios.
  **Ticked 2026-11-20 (verified at `a5301eb`):** the spec delta now declares
  382 `path.*` IDs (368 when this was written), and every one has an
  executable test tagged `covers:` (`scenario-coverage.sh --change
  add-fhirpath-and-codegen-r4`: 514 of 514 covered, 0 unresolved). No
  `#[ignore]`, bare `#[should_panic]`, `todo!()` or `unimplemented!()` in
  `crates/frevo-path/{src,tests}`. That the tests now pass or fail is the
  implementation tasks' business, not this one's.

- [x] **4a. Build the `path-check-warnings` in-process oracle.** The check
  pass's warning channel is reachable by neither the vendored suite — whose
  `invalid="semantic"` selector grades only whether the check *fails* — nor the
  HTTP differential harness, which never sees the list. Build a unit oracle over
  the generated R4 model whose case selector is the warning message id, with at
  least one expression per id and the complete id set asserted as exactly nine:
  choice-no-type-specifier, choice-spurious-type-specifier,
  collection-status-operation-left, collection-status-operation-right,
  collection-status-parameter, collection-status-context, not-a-collection,
  unknown-extension, of-type-impossible. Assert that a warning never makes the
  check fail, that the warning accumulator is cleared at the start of every
  check so warnings cannot leak between expressions on a reused engine, and that
  the check surface taking no warning list returns the same pass/fail verdict
  while discarding the warnings.
  Satisfies the oracle named by `path.check.015`, `path.check.016`,
  `path.check.017`, `path.check.018`, `path.check.019`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `path_check_warnings` is 5
  of 5, one test per ID `path.check.015`-`019`, each tagged `covers:`.

- [x] **4b. Write the failing tests for the scenarios the in-process HAPI
  differential added or changed.** Owner ruling, 2026-11-21: on the 18
  undocumented diffs of `harness/fhirpath-compare/differential-results.json`
  (`tasks/blockers.md`, 2026-11-21, "the in-process HAPI differential"),
  HAPI's behaviour wins and no deviation is added. The spec delta now has four
  new scenarios, `path.literal.016`, `path.literal.017`, `path.quantity.025`
  and `path.quantity.026`, and five amended ones: `path.compare.014` (the
  exact `lunit` text), `path.quantity.019` (the same text under the
  server-facing worker), `path.quantity.020` (the `[pi]` values through `*`),
  `path.quantity.021` and `path.literal.012` (a cross-reference to
  `path.quantity.025` only), and `path.math.017` (the integer arm's text is
  now graded). `oracle-builder` work, before tasks 44a to 44c:
  - one test per new ID, tagged `// covers:`, asserting the values each
    scenario states, and the amended assertions in the existing
    `path.compare.014`, `path.quantity.019`, `.020` and `path.math.017` tests;
  - in `harness/fhirpath-compare/differential/probes.py`, drop the
    `"grade": "error"` relaxation of `path.math.017#61` and `#62`, whose text
    is now graded, and add the probes the new scenarios name as owed: the
    `\t`, `\u` and `\\` units of `path.literal.016`, the wide literal of
    `path.literal.017` (fractional and the 40-digit integer-shaped one),
    `(1 week).unit`/`.code`/`.system`, `(1 year).code`,
    `(1 year).system`, `(2 'cm' * 1 'm').unit` and `(1 '%' * 1 '1').code`
    of `path.quantity.025`, and `(1 '').code` and `(1 '').system` of
    `path.quantity.026`. Add check-time probes **with a `contextType`**
    (for example `Patient`), so the harness runs HAPI's check, for
    `(1 'cm').code` and `(2 'cm' * 1 'm').code` (`path.quantity.025`, the
    expected `FHIRPATH_UNKNOWN_NAME` texts at `(@char 11)` and
    `(@char 22)`, derived) and for `(1 '').code` and `(1 '').system`
    (`path.quantity.026`, both at `(@char 9)`, derived). Then re-run the HAPI side and the differential. A
    stated value the run contradicts goes back to the proposer, not into
    the test.
  Until this lands, `harness/scripts/scenario-coverage.sh --change
  add-fhirpath-and-codegen-r4` reports the four new IDs as uncovered.
  Satisfies the tests for `path.literal.016`, `path.literal.017`,
  `path.quantity.025` and `path.quantity.026`, and the amended
  `path.compare.014`, `path.quantity.019`, `path.quantity.020` and
  `path.math.017`.
  **Ticked 2026-11-21 (verified at `761a9e7`):** the tests and the owed probes
  landed there, tagged `covers:`; they failed until tasks 44a to 44c.

- [x] **4c. Write the failing tests for `path.math.018` and
  `path.quantity.027`.** These are two HAPI behaviours the implementer of
  tasks 44a to 44c found unspecified. `oracle-builder` work, before task
  44d:
  - one test per ID, tagged `// covers:`, asserting the values each scenario
    states;
  - probes in `harness/fhirpath-compare/differential/probes.py` over
    `parameters-novalue-numbers.xml`:
    - for `path.math.018`: `+`, `-` and `*` with the no-value `integer` `i`
      on the left and on the right of `1`; the no-value `decimal` `d` with
      `1`; `i` with `1.5`; and `1.5 * d`. Also check-time probes **with**
      the `contextType` `Parameters`, so the harness runs HAPI's check:
      `Parameters.parameter.where(name = 'i').value + 1` and
      `Parameters.parameter.where(name = 'd').value + 1` (the check passes,
      and evaluation fails with the scenario's texts);
    - for `path.quantity.027`, with no `contextType`: `children()` and
      `descendants()` on `1 'cm'`, `1 year`, `1 week`, `1 ''` and
      `2 'cm' * 1 'm'`, and their `count()`;
    - also for `path.quantity.027`, check-time probes **with** a
      `contextType`: `(1 'cm').children().count()` (passes),
      `(1 'cm').children().first()` and `(1 'cm').children().code` (both
      rejected at `(@char 22)` with the scenario's texts);
  - then re-run the HAPI side and the differential. A stated value the run
    contradicts goes back to the proposer, not into the test. That includes
    the decimal arm's `"val" is null` text, which has so far been seen only
    on the bare JDK.
  Until this lands, `harness/scripts/scenario-coverage.sh --change
  add-fhirpath-and-codegen-r4` reports `path.math.018` and
  `path.quantity.027` as uncovered.
  Satisfies the tests for `path.math.018` and `path.quantity.027`.

- [x] **5. Write the failing tests for every `codegen.*` and `model.*`
  scenario.** Same discipline: one test per scenario ID with a `covers:`
  comment, 49 plus 83 of them, including the `bench/micro/` workloads as
  benchmarks with assertions on their budgets. Interim round-trip selector is
  the 103 examples in `oracles/fhir-test-cases/fetched/r4/examples/`, with the
  test reporting that it is the interim set.
  Covers all 49 `codegen.*` and 83 `model.*` scenarios.
  **Ticked 2026-11-20 (verified at `a5301eb`):** all 49 `codegen.*` and 83
  `model.*` IDs in the spec delta have an executable test tagged `covers:`
  (`scenario-coverage.sh`: 514 of 514, 0 unresolved). The `bench/micro/`
  model workloads are asserted by `model_bytes_lazy_perf.rs`, and the
  interim selector is stated by the round-trip report
  (`codegen_r4.rs`, `codegen.roundtrip.001` asserts it says "interim").

- [x] **5a. Build the `model-parser-errors` in-process oracle.** Three of HAPI's
  ten parser error conditions cannot be reached by the round-trip oracle,
  because no example in `hl7.fhir.r4.examples#4.0.1` carries a contained
  resource without an `id` or a dangling `#` reference. Build a unit oracle over
  hand-built documents — one per condition — asserting, in **both** parse modes,
  the lenient diagnostic and the surviving resource, and the strict error with
  HAPI's message code. The three conditions and their codes are a contained
  resource with no `id` (code `1819`), a `#` reference whose target is not
  contained (code `1826`, raised by the post-decode weaving pass) and, on the
  **encode** side, serialising a reference beginning `#` that matches no
  contained resource (code `2724`). Assert the exemptions too: a reference whose
  text is exactly `#`, or whose id part is blank, is not diagnosed on either
  side.
  Satisfies the oracle named by `model.json.024`, `model.json.025`,
  `model.canonical.007`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `model_parser_errors` is 6
  of 6, covering `model.json.024`, `model.json.025` and
  `model.canonical.007`.

---

## Phase B — `frevo-codegen`

- [x] **6. Fetch and pin the R4 definition package.** A script that fetches
  `hl7.fhir.r4.core#4.0.1` into a git-ignored directory and verifies it against a
  SHA-256 recorded in the repository, plus the generator's refusal paths.
  Satisfies `codegen.inputs.001`, `codegen.inputs.002`, `codegen.inputs.003`,
  `codegen.inputs.004`.
  Done: `crates/frevo-codegen/packages/{fetch.sh,PINS.md,.gitignore}` and
  `src/{pin,package,sha256,emit,run,error}.rs`. Two digests are recorded, and
  they check different things — the archive digest `ebd7731d…` before
  extraction, and the content digest `8af9fe30…` of the extracted tree on every
  run, because an archive digest says nothing about a tree edited afterwards.
  SHA-256 is written here rather than taken from a crate, so no new dependency
  enters the graph; its output was checked against `sha256sum` and against an
  independent implementation of the same canonical manifest.

- [x] **7. Read StructureDefinitions and assert the input inventory.** Parse the
  package's definitions, classify them by `kind`, `abstract` and `derivation`,
  and fail the run on a count other than 63 datatypes, 149 resource definitions
  (146 concrete, 2 abstract, 1 logical) and 1,375 SearchParameters. Add the
  `reference/`-conditional cross-check against the HAPI copy at `7ab4c7c7`.
  Satisfies `codegen.inputs.005`, `codegen.inputs.006`.
  Done: `src/{inventory,crosscheck,xml}.rs`. The package is **not** split the
  way HAPI's copy is — it ships 658 StructureDefinitions and 1,400
  SearchParameters in one flat directory — so a scoping rule was needed before
  any count could be compared. Measured over the whole package and the whole
  HAPI copy: a definition is the specification's own iff it carries
  `structuredefinition-standards-status` **and** declares a `derivation` or
  `abstract = true` (exactly 256, identical URL set to `profile/*.xml`), and
  within those it is a type rather than a profile on one iff `id == name`
  (exactly 212 = 63 + 149, leaving exactly the 44 of `profiles-others.xml`).
  The same stamp selects exactly 1,375 of the 1,400 SearchParameters. All 256
  definitions agree with HAPI on every snapshot element's path, cardinality and
  type codes — 0 disagreements. `src/inventory.rs` records the derivation
  shapes, so a repinned package whose shape differs fails the assertion rather
  than generating a different model.

- [x] **8. Generate primitive and complex datatypes.** The 20 primitives with
  their value regexes, the complex types, and the metadata types.
  Satisfies `codegen.types.001`, `codegen.types.002`, `codegen.types.003`.
  Done: `src/{model,emit_tables,rustsrc}.rs`, output in
  `crates/frevo-model/src/r4/`. 20 primitives with 19 value regexes (`xhtml`
  declares none) and all 43 `complex-type` definitions, `Element` and the two
  `constraint` profiles on `Quantity` included. `codegen.types.002` and
  `codegen.types.003` pass. **`codegen.types.001` cannot pass as written**: its
  loop requires `xhtml.value` to have `min` 0 and its closing assertion
  requires `min` 1. Recorded in `tasks/blockers.md`, 2026-11-11; no test was
  changed. A primitive's JSON form is read from the **root** of its base chain,
  because `positiveInt.value` and `unsignedInt.value` both declare
  `System.String` in R4 and both are JSON numbers.

- [x] **9. Generate resources, abstract element sets and backbone elements.** The
  146 concrete resources, the shared `Resource` and `DomainResource` element
  sets, nested backbone types at every level, and termination on recursive
  backbones and datatype cycles.
  Satisfies `codegen.types.004`, `codegen.types.005`, `codegen.types.006`,
  `codegen.types.007`, `codegen.types.008`, `codegen.types.009`.
  Done: 146 resources, 2 abstract element sets, 1 logical and **473 nested
  backbone types**, 685 types and 7,500 elements in all. R4 snapshots are fully
  expanded, so `Resource`'s and `DomainResource`'s elements are on each
  resource by construction rather than by inheritance. `codegen.types.004`,
  `.005`, `.006` and `.009` pass. Termination is structural, not guarded:
  generation never follows a type reference — a backbone type comes from a path
  prefix and an element's type is recorded as a name — so a recursive
  `contentReference` resolves to a type that already exists and the
  `Reference`/`Identifier` cycle is two complete types naming each other. A
  depth limit would have been the bug. `codegen.types.008`'s generation half
  passes and its second half needs `decode`, which is task 19's seam.
  **`codegen.types.007` cannot pass as written**: its element-prefix loop
  requires a choice element's `path` to omit the `[x]` that the field's own
  documentation, `codegen.choice.001` and `codegen.choice.003` all require it
  to carry. Recorded in `tasks/blockers.md`, 2026-11-11.

- [x] **10. Generate choice elements and cardinality.** `value[x]` under its base
  name with its full property-name map and its declared type set, the
  single-permitted-type case, the two-values-for-one-choice rejection, and the
  four cardinality shapes.
  Satisfies `codegen.choice.001`, `codegen.choice.002`, `codegen.choice.003`,
  `codegen.choice.004`, `codegen.cardinality.001`, `codegen.cardinality.002`,
  `codegen.cardinality.003`, `codegen.cardinality.004`.
  Done: every `[x]` element is generated under its base name with its declared
  type set and one JSON property name per type — 50 for `Extension.value[x]`,
  11 for `Observation.value[x]` — and no typed spelling is an element name.
  `min` and all three upper bounds (`0`, `1`, `*`) are carried into
  `r4/elements.rs`, which exists because the codec's hand-written `ElementDef`
  has nowhere to put a minimum and cannot express the `0..0` of
  `xhtml.extension`. `codegen.choice.003` passes, through `generate_synthetic`:
  no core element has a `[x]` path with exactly one `type`.
  **Seven of these eight scenarios are not reachable from this task.**
  `codegen.choice.001`, `.002`, `.004` and `codegen.cardinality.001` to `.004`
  all assert on `decode`/`encode`, and every one of them now fails at its first
  `decode` call with every preceding generation assertion satisfied. That seam
  belongs to tasks 19 to 22 (`codegen.cardinality.002`, `.004` and `.005` are
  already assigned to task 21 by this file), and `codegen.choice.004` plus
  `.cardinality.002`/`.004` are the three conflicts with HAPI the oracle records
  at its own call sites. **Task 10 as scoped cannot satisfy them**; the
  decomposition is wrong, not the implementation.

- [x] **11. Generate the Rust naming map.** snake_case fields, deterministic
  keyword escaping, collision detection that fails rather than loses an element,
  verbatim type names, and enumeration variants whose original code strings are
  recoverable.
  Satisfies `codegen.naming.001`, `codegen.naming.002`, `codegen.naming.003`,
  `codegen.naming.004`, `codegen.naming.005`.
  Done: `codegen.naming.001`, `.003` and `.004` pass. `rust_field_name` is
  bound to `model::rust_field_name_of_path`, so the rule answers for a snapshot
  path the package does not contain, and `code_variants` to the new
  `bindings::code_variants`. The variant rule is total, deterministic and
  collision-free: each alphanumeric run upper-cased on its first character and
  concatenated (`entered-in-error` → `EnteredInError`), a code with no
  alphanumeric character spelled from a symbol table (`>=` → `GreaterEqual`),
  an `N` on a name that would start with a digit (`0.5.0` → `N050`), a
  trailing `_` on a keyword, and a numeric disambiguator behind all of it.
  Measured over all **2,066** codes behind R4's required bindings: every
  variant name is a plain non-raw non-keyword Rust identifier, and the only
  value sets where the alphanumeric rule alone would collide are
  `quantity-comparator` and `questionnaire-enable-operator`, whose codes are
  punctuation.
  **`codegen.naming.002` and `.005` are not reachable from this task.** Both
  end in `decode`/`encode` calls that belong to tasks 19 to 23; every
  generation assertion in `.005` passes, and `.002` additionally fails at an
  assertion that is wrong — line 3339 requires `Encounter.class` to get a Rust
  name different from `class`, and `class` is not a Rust keyword in any
  edition. Recorded in `tasks/blockers.md`, 2026-11-11, with the evidence and
  the two ways out; the generator escapes Rust's keywords and reserved words
  and nothing else, because escaping `class` would be inventing a rule the
  spec delta does not state over a name with no observable effect.

- [x] **12. Generate required-binding enumerations.** Closed enumerations only
  for resolvable `required` bindings; codes outside a required binding preserved
  rather than dropped; nothing generated for extensible, preferred or example
  bindings or for an unresolvable value set.
  Satisfies `codegen.binding.001`, `codegen.binding.002`,
  `codegen.binding.003`, `codegen.binding.004`.
  Done: `codegen.binding.004` passes. `package.rs` now also reads the
  package's 1,316 `ValueSet` and 1,062 `CodeSystem` resources — the two inputs
  `design.md` section 2 already lists and nothing had read yet — and
  `bindings.rs` expands a value set only when it can be expanded from the
  package alone: no `exclude`, and every `include` either lists its `concept`
  codes or names one `system` the package ships with `content = complete`. A
  `CodeSystem`'s codes are taken **depth-first pre-order**, which is what makes
  `observation-status` read `… amended, corrected, cancelled …` as
  `codegen.naming.005` lists it, since `corrected` is a child of `amended`.
  Measured: R4 binds **224** distinct value sets `required` across the 212 core
  definitions, all 224 naming them with a `|4.0.1` suffix; **220** expand, to
  2,066 codes; **4** do not and are reported with their element paths (17 of
  them) rather than guessed at — `currencies`, `mimetypes` and `ucum-units`,
  whose code systems are `urn:iso:std:iso:4217`, `urn:ietf:bcp:13` and
  `http://unitsofmeasure.org`, and `http://loinc.org/vs/LL379-9`, which the
  package does not ship at all because LOINC is licensed.
  Emitted as a **data table** in `r4/codes.rs` rather than 220 Rust `enum`s,
  which is `design.md` section 8's stated lever and the shape tasks 8 to 10
  already took; each variant's `rust_name` is still a valid identifier, so a
  later change that wants a real `enum` per value set can generate it from this
  table without renaming anything.
  **`codegen.binding.001`, `.002` and `.003` are not reachable from this
  task.** All three end in `decode`/`encode` calls belonging to tasks 19 to 23,
  and all three now fail at the first of those with every generation assertion
  satisfied: `Patient.gender` carries enumeration `AdministrativeGender` with
  exactly `male, female, other, unknown`; `Observation.code` is `example` and
  carries none; and no enumeration anywhere comes from a binding weaker than
  `required`. `.002` is additionally one of the three conflicts with HAPI the
  oracle records at its own call site.

- [x] **13. Generate the search-parameter table.** All 1,375 rows with code, URL,
  type, bases, expression, composite components, reference targets and the
  modifier flags, with absent distinguished from false, and a row per base for a
  multi-base parameter.
  Satisfies `codegen.searchparam.001`, `codegen.searchparam.002`,
  `codegen.searchparam.003`, `codegen.searchparam.004`,
  `codegen.searchparam.006`.
  Done: all five pass. **1,706 rows** over the 1,375 parameters the
  specification stamps — one row per declared `base`, because the index is
  keyed by resource type — each carrying the whole declared base list so the
  parameters stay recoverable from the rows. `clinical-patient` declares 32
  bases and contributes 32 rows. 46 parameters state `multipleOr` and every
  one states it `false`; 142 state `comparator`, all the same nine codes;
  `multipleAnd`, `modifier` and `chain` are stated by none, which is what makes
  absent-versus-`false` observable, and every flag is an `Option`.
  `DomainResource-text`, `Resource-content` and `Resource-query` declare no
  `expression` and are recorded as `None`, distinctly from an empty one.
  Base resolution happens at **query** time, in `searchparam::applicable`, by
  walking the queried type's `baseDefinition` chain: a row declared on
  `Resource` is stored as `Resource`, so `_id` reaches `Binary` and `_text`,
  declared on `DomainResource`, does not. Storing 146 expanded rows per
  whole-system parameter would have added ~1,300 rows and lost the fact that
  the specification declared one.
  **`SearchParameter.status` is deliberately not a column.** The package and
  HAPI's `sp/search-parameters.json` disagree about it — all 1,375 read
  `"draft"` in `hl7.fhir.r4.core#4.0.1` and `"active"` in HAPI, and it is the
  only field that differs across all 1,375 — and nothing downstream of a
  search-parameter table consults it, because a parameter is enabled by
  configuration rather than by its publication state. No row carries it, so
  there is no source to pick between and **no `docs/deviations.md` entry is
  due**: a deviation records a behaviour difference, and a field neither source
  can be observed through is not one. The reasoning is in
  `r4/searchparam.rs`'s own header so it is not rediscovered.
  `codegen.searchparam.005` is **not** this task's: it hands every expression
  to `frevo-path`'s parser, which is tasks 28 to 30. The table side of it is
  already satisfied — 1,344 distinct non-empty expressions and exactly the
  three absent ones.

- [x] **14. Generate the element-order tables and enforce output discipline.**
  Per-type element order for serialisation; byte-identical regeneration on the
  same platform and across platforms; the `@generated` header naming the
  generator, the package, the version and the digest; `fmt` and `clippy -D
  warnings` clean with no added `allow`; the CI drift check; no `unsafe`.
  Satisfies `codegen.roundtrip.004`, `codegen.output.001`,
  `codegen.output.002`, `codegen.output.003`, `codegen.output.004`,
  `codegen.output.005`, `codegen.output.006`.
  Done: all six `codegen.output.*` pass. The element-order table is `TYPES`
  itself — each entry's elements are in snapshot order, which is the order a
  resource this crate constructed is serialised in — with a named
  `element_order(type_name)` accessor emitted beside it rather than a second
  copy of the same 7,500 property names, which would be a second thing to keep
  in step with the snapshot.
  Five files, **73,464 lines and 2,183,049 bytes**: `mod.rs`, `codes.rs`,
  `elements.rs`, `searchparam.rs`, `types.rs`. Three consecutive runs agree by
  SHA-256, and `elements.rs` is byte-identical to the output tasks 8 to 10
  checked in, so the two new files are additive rather than a reshuffle.
  One layout rule in `rustsrc.rs` was **wrong and is now measured**: it moved
  an over-long right-hand side down a line unconditionally, where `rustfmt`
  moves it only when that makes it fit. A 64-character package digest at
  indent 0 fits once moved and so moves; the longest R4 search-parameter
  expression is 1,386 characters, fits nowhere, and so stays on one line.
  `codegen.output.004` found it, which is what it is for. A new
  `--bin fmtcheck` prints where the emitted bytes and `rustfmt`'s own output
  disagree, so the next such rule is findable rather than guessable.
  No `#[allow]` anywhere in the output: a search-parameter row is a struct
  literal rather than a 14-argument constructor precisely because the
  constructor would have needed
  `#[allow(clippy::too_many_arguments)]`, which `codegen.output.004` forbids.
  Verified out of tree, because `crates/frevo-model/src/lib.rs` does not
  declare `pub mod r4;` and `cargo clippy --workspace` therefore never compiles
  the output: all five files type-check and are `clippy -D warnings` clean
  against the real `codec::tables` shape.
  **`codegen.roundtrip.004` is not reachable from this task.** Its element-order
  assertions pass — every type's order is its element set in snapshot order,
  `Patient` is the 24 the oracle lists with `[x]` resolved, and
  `Patient.contact` is its 10 — and it then fails at `decode`, which is tasks
  19 to 23.

- [x] **14b. Generate the R4 model class table.** Owner decision 2026-11-21,
  option (c) (`tasks/blockers.md`, "the R4 Java class ancestry table"). In
  `frevo-codegen`: derive, from the pinned R4 StructureDefinitions only, one
  `@generated` table (`frevo-codegen class-table --version r4`, written to
  `crates/frevo-path/src/eval/r4_classes.tsv` because `path.api.008` keeps
  `frevo-path` free of `frevo-model`; covered by the drift check) of every R4
  model class row (primitive, complex and
  resource types, nested components, `Enumeration`) with its Java simple and
  qualified names, the FHIR type names it is assignable to and its interface
  kinds, plus each element's value class, by the rules of
  `codegen.ancestry.002` and `.003`; then apply the exception file. The
  implementer creates `crates/frevo-codegen/data/r4-class-table-exceptions.toml`
  with exactly the entries `codegen.ancestry.004` lists, each with its
  `source` citation, and makes the generator refuse an entry with no source,
  an unknown target or no effect. The generator reads no `.java` file and no
  entry is invented: a difference found later is a new reviewed entry. The
  table carries the R4 package's notice as the other generated tables do; no
  Java-derived text is emitted, so no HL7 Java licence notice is owed.
  Satisfies `codegen.ancestry.001` through `codegen.ancestry.005`.
  **Ticked 2026-11-21.** The generator is `src/classtable.rs`, run as
  `frevo-codegen class-table --version r4`; the exception file is
  `data/r4-class-table-exceptions.toml` with its 25 entries; the seam is
  bound through `frevo_codegen::bind_class_table!`. All five
  `codegen.ancestry.*` tests pass (`codegen_r4` 55 of 55), including the 323
  `Enumeration` elements and the three refusals. The table is written to
  `crates/frevo-path/src/eval/r4_classes.tsv` (683 rows, 8,247 element paths)
  rather than into `frevo-model`, because `frevo-path` is the crate that
  evaluates against it and `path.api.008` forbids it depending on
  `frevo-model`. `xhtml.id` declares `System.String` with no fhir-type
  extension and maps to `StringType`, the field `XhtmlType` inherits from
  `Element`, as `codegen.ancestry.002` now states. The generator refuses any
  other `System.*` element that lacks the extension. The drift check covers
  the table (`run::drift_check_outputs`, `codegen.output.005`, `bb94e47`), and
  `codegen_r4` is 56 of 56.

- [x] **15. Check the compile-time budget and decide.** Measure a cold
  `cargo build -p frevo-model --release` on the reference environment against
  the 120 s and 40 MB budgets in `design.md` section 8. If either is missed,
  write the ADR for moving the codec from generated code to runtime-interpreted
  tables, and stop for the owner rather than improvising.
  **Ticked 2026-11-21, owner decision.** The owner accepted these
  measurements: a cold `cargo build -p frevo-model --release` takes about 11 s
  on the development Windows host (budget 120 s); the generated source is
  about 2.3 MB (budget 40 MB); and CI's whole-workspace release build takes
  42-69 s on a GitHub runner. Both budgets are met with wide margin, so the
  ADR for runtime-interpreted tables is not needed.

---

## Phase C — `frevo-model`

- [x] **16. Define the `Value`, `Node`, `Model` and `HostServices` traits in
  `frevo-path`.** The seam from `design.md` section 4, with no implementation
  behind it. Adding it here rather than in Phase D is deliberate: `frevo-model`
  depends on these definitions, and `frevo-path` must not depend on
  `frevo-model`.
  Satisfies `path.api.007`, `path.api.008`.
  Done in `crates/frevo-path/src/seam/`. `Decimal` takes the
  lexical-form-plus-integer-mantissa fallback `design.md` section 6 names, so no
  arbitrary-precision crate was added and no new dependency enters the graph.

- [x] **17. Implement the span-based buffer, the lazy index and pass-through.**
  `Arc<[u8]>` plus per-object lazily built index; load-and-serialise-unchanged as
  a byte copy with a zero token counter; reading an element not disturbing
  pass-through; the constructed form serialising in generated element order; the
  two forms indistinguishable to a reader.
  Satisfies `model.bytes.001`, `model.bytes.002`, `model.bytes.003`,
  `model.bytes.004`, `model.bytes.005`, `model.bytes.006`.
  Done in `crates/frevo-model/src/codec/`. Measured: a pass-through scans **0**
  JSON tokens, makes **0** size-proportional allocations and hands back the
  input allocation itself; a lazy read of `Patient.birthDate` scans **39**
  tokens at 4 KiB and at 64 KiB, against 1,570 for a full decode. **The six
  oracle tests are still red as committed**: the seam in
  `crates/frevo-model/tests/model_bytes_lazy_perf.rs` is hard-coded to
  `UnboundCodec`, which no file under `src/` can replace. They pass when it is
  rebound. See `tasks/blockers.md`, 2026-11-08, finding 1 — and finding 5, which
  records that the element-order table is provisional until task 14.

- [x] **18. Implement lazy materialisation and its instrumentation.** Unread
  siblings, indexed access, counting without materialising, idempotent cached
  reads, contained resources and narrative untouched by unrelated reads — each
  observable through a not-materialised report.
  Satisfies `model.lazy.001` through `model.lazy.006`.
  Done: all six pass. The implementation landed early, with task 17 — the
  incremental per-object cursor in `codec/doc.rs` and the deferred `_field`
  lookup in `codec/stored.rs` are what make all six observable — so what this
  task did was **verify it against the generated tables rather than the
  hand-written stand-in**, which is the only way the claim means anything:
  `codec/mod.rs` now binds `crate::r4::types::TABLES`, 685 types and 7,500
  elements, and the stand-in survives only as the fixture the codec's own unit
  tests read. The switch exposed one real defect, now fixed in
  `codec/built.rs`: an R4 snapshot is **fully expanded**, so concatenating a
  type's base chain emitted `Patient.id` three times — once from `Resource`,
  once from `DomainResource`, once from `Patient`. `ordered_elements` now takes
  each name at its base-chain position and skips a repeat, which reproduces the
  snapshot order for an expanded table and an inherited one alike; measured
  over all 685 generated types, 683 carry a fully expanded list and the rule
  reproduces every one exactly, the other two being `Resource` and `Element`,
  which have no base.
  Measured after the switch, so the task 17 properties are shown intact: a
  pass-through scans **0** tokens, makes **0** size-proportional allocations
  and hands back the input `Arc`; a lazy read of `Patient.birthDate` scans
  **39** tokens and takes **1.6 µs at both 4 KiB and 64 KiB** — size
  independent, which is the property — against **1,570** tokens for a full
  materialise of the 4 KiB document.

- [x] **19. Implement JSON decoding: structure.** `resourceType` dispatch and its
  three error paths, choice decoding from the typed property name, contained
  resources, Bundle entries of mixed types, and element `id` and `extension` on
  any element. The unknown-property path is task 21's, because its outcome
  depends on the parse mode.
  Satisfies `model.json.001`, `model.json.002`, `model.json.003`,
  `model.json.005`, `model.json.006`, `model.json.007`, `model.json.020`.
  Done: all seven pass, in the new `codec/decode.rs`. This is the codec's
  **second** half and not a replacement for the first: `Resource` stays lazy
  and byte-preserving, and `codec::decode` decodes a whole document into an
  owned tree that can be re-emitted in generated element order, because
  `reencoded_in_element_order` is defined as a full materialise-and-re-emit and
  handing back the input buffer is not an answer to it.
  The three `resourceType` error paths are a root that is not a JSON object, a
  missing `resourceType` and a `resourceType` naming no known type; all three
  are errors in both modes, because there is no resource to return and nothing
  for a diagnostic to hang on. The same three apply wherever an element
  declares the abstract `Resource` — a `contained` entry, a
  `Bundle.entry.resource` — which is what makes `model.json.006` and `.007`
  one mechanism rather than two.
  A **value and its `_field` sibling are two independent sides of one
  element**, never merged, with each side's array length and `null` holes
  recorded separately. That representation is not task 19's to choose, but the
  decode cannot be written without it, and writing a stub that could not hold
  it would have had to be thrown away: `model.primext.001` to `.005`, `.007`
  and `.008` pass as a consequence. The two primitive-extension **reports**,
  `.006` and `.009`, are left to task 22.

- [x] **20. Implement JSON decoding: primitive values.** Opaque `xhtml`,
  lexical-form decimals with no binary-float path, `integer`, `positiveInt` and
  `unsignedInt` bounds, `base64Binary` carried as its encoded text.
  Satisfies `model.json.008`, `model.json.009`, `model.json.010`,
  `model.json.011`, `model.json.012`, `model.json.017`.
  Done: all six pass. One decision carries four of them: a primitive is held as
  **the span of the JSON token it was read from**, and re-emitting is a copy of
  those bytes. Nothing re-renders a value, so `1.58700` keeps its trailing
  zeros, `1.00e3` keeps its exponent form, a `base64Binary` keeps the input's
  padding and an `xhtml` div keeps its escaping — not by special-casing any of
  them but because there is no code that could change them. The lexical form is
  built on demand rather than stored, so a decode nobody reads a value out of
  allocates nothing per primitive.
  `Decimal` carries the lexical form, the exact value rendered positionally and
  the significant-digit count, and **no `f64` appears anywhere on the path**:
  `full_precision` is produced by moving the decimal point of the input's own
  digits, which is why the 40-digit case renders all 40 rather than the 17 a
  binary double round-trips. No arbitrary-precision crate was added.
  The bounds are `integer` (−2147483648..2147483647), `positiveInt` (1..) and
  `unsignedInt` (0..), reported as a lenient diagnostic naming the element path,
  the declared type and the offending value — which is the delivery the oracle's
  own fixture table requires, and distinct from `invalidValue`. A value that is
  not an integer literal at all is `invalidValue` and belongs to task 21, so it
  is not reported here. `decimal` is deliberately not bounded: it has no
  declared range.

- [x] **21. Implement the two parse modes and the diagnostic channel.** Lenient
  by default, matching the handler `FhirContext` installs: an unknown property,
  a JSON type mismatch, a single object where an array belongs, an array where a
  single value belongs, a duplicate key (last wins), a `null` in a non-primitive
  array, a blank primitive value and a second value for a choice element all
  **succeed** and return a diagnostic beside the resource; a missing required
  element succeeds in **both** modes with no diagnostic, because HAPI's parser
  checks no minimum cardinality (`codegen.cardinality.002`, `.004`, revised
  2026-11-20). The two genuinely fatal conditions fail in both modes: an
  extension carrying both a value and nested extensions, and a non-blank invalid
  primitive value, which includes a code outside the binding of an element
  HAPI's model holds as an `Enumeration` (`1821`, `codegen.binding.002`, checked
  against `frevo_model::r4::codes`). Strict mode turns every diagnostic into an
  error and changes nothing else, with identical paths and offsets: a blank
  value, a blank `reference` included, is `1821` (`codegen.cardinality.005`,
  `model.json.023`, `model.json.025`), and a second choice value is `1823` at
  the second property (`codegen.choice.004`). Non-UTF-8 input fails in both.
  This task is the one the differential gate would otherwise catch on day one,
  so it comes before any other decoding work lands.
  Cover **all ten** of HAPI's parser error conditions, not the seven that are
  easy to reach: a short strict mode is a silent parity break. The two that
  remain on the decode side are task 21a's; the one on the encode side is task
  23a's.
  Satisfies `model.json.004`, `model.json.013`, `model.json.014`,
  `model.json.015`, `model.json.016`, `model.json.018`, `model.json.019`,
  `model.json.021`, `model.json.022`, `model.json.023`, `model.errors.003`,
  `codegen.cardinality.002`, `codegen.cardinality.004`,
  `codegen.cardinality.005`.
  **Ticked 2026-11-20.** Every scenario it satisfies passes: `model_decode`
  37 of 37 (`model.json.004`, `.013`, `.014`, `.015`, `.016`, `.018`, `.019`,
  `.021`, `.022`, `.023`, `model.errors.003`), `model_parser_errors` 6 of 6,
  and `codegen_r4` 50 of 50 (`codegen.cardinality.002`, `.004`, `.005`, with
  `codegen.choice.004` and `codegen.binding.002` beside them). The
  missing-required-element question that held this box was resolved in the
  spec on 2026-11-20 (no check, either mode). What may still differ from HAPI
  -- the `Enumeration` set and enum names pending the Java class table, and
  the per-type tail of a non-blank invalid value's message -- is in
  `tasks/blockers.md`, 2026-11-20, "codec and `comparable()` landed".

- [x] **21a. Implement the two contained-resource decode diagnostics.** A
  contained resource with no `id` is **still appended** to the parent's
  `contained` list in lenient mode — HAPI's code comment at the call site says
  the opposite of what the code does — but is **not** entered into the
  id-to-resource map, so no `#` reference can ever be woven to it. Strict mode
  fails with message code `1819`. Separately, run a post-decode weaving pass
  over the local references the decode collected: a reference of `#` plus at
  least one character whose target is not among `contained` is a lenient
  diagnostic naming the reference text and a strict error with message code
  `1826`; a reference that is exactly `#`, or blank, is not looked up at all.
  Because the pass runs after the whole document is decoded, a reference that
  appears before its target still resolves. The reference's literal text
  survives in both modes, which is what `path.utility.017` then resolves
  against.
  Satisfies `model.json.024`, `model.json.025`.

- [x] **22. Implement the `_field` primitive-extension representation.** Two
  independent spans per node; singleton and index-aligned array forms; `null`
  holes in either array; `hasValue` as the presence of a value span; a
  value-less element that still exists; unequal-length arrays reported; in-place
  round-trip; reading a value without materialising its extensions; `_` rejected
  on a complex element.
  Satisfies `model.primext.001` through `model.primext.009`.

- [x] **23. Implement canonical and normal serialisation.** Lexicographic key
  order with no whitespace for canonical; generated element order for normal;
  array order preserved in both; presented decimal form preserved;
  idempotence; and the direct-versus-round-trip equality the oracle compares.
  Satisfies `model.canonical.001` through `model.canonical.006`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `model_canonical` is 6 of 6
  in a checkout where `oracles/fetch.sh` has run (the fetched interim
  directory present, so `model.canonical.005`'s corpus branch ran rather than
  being skipped); `model.canonical.006`'s second tag, in
  `frevo-model/src/codec/pretty.rs`, is in the lib tests, 29 of 29. The
  earlier `model.canonical.003`/`.005` conflicts are in `tasks/blockers.md`,
  2026-11-12, "task 23 is green except where its oracle contradicts itself or
  the spec".

- [x] **23a. Implement the encode-side dangling-internal-reference diagnostic.**
  This is the **only** error condition on the encode side, and the round-trip
  oracle cannot reach it. When serialising a reference whose id part is
  non-blank and whose text begins `#`, check it against the resources actually
  contained in the document being written: no match is a lenient diagnostic
  naming the element and the reference and a strict error with message code
  `2724`. The reference is emitted unchanged either way. A reference of exactly
  `#` is a reference to the containing resource and is never diagnosed; a
  non-local reference is not checked on this path at all; a reference with a
  blank id part takes the contained-resource-naming branch instead.
  Satisfies `model.canonical.007`.
  Done: `model_parser_errors` is 6 of 6 (it was 3 of 6), all three
  `model.canonical.007` tests included. `Decoded::encode_canonical`,
  `Built::encode_canonical` and `Resource::encode_canonical` carry the
  check; each reference is checked against the `contained` list of the
  nearest enclosing resource that is not itself contained. The constructed
  (`Built`) path was exercised with a throwaway driver -- `#org1` diagnosed
  with 2724 in both modes, `#`, `Organization/org1` and a blank reference
  not -- but has no committed test: the `covers:` hook resolves ids from the
  session's checkout rather than this worktree, found no active change, and
  refused `model.canonical.007` as nonexistent. Two upstream behaviours stay
  out of scope as recorded in `tasks/blockers.md` (findings 3 and 4 of
  2026-11-12): HAPI's encoder giving an id-less contained resource a UUID,
  and its encode context not being restored after a `Bundle` entry.

- [x] **24. Implement copy-on-write mutation and byte splicing.** Untouched
  subtrees keeping their original bytes on element change, append and remove;
  the modified flag; a same-value write still marking modified; splicing at a
  contained resource's boundary; the `meta` update path.
  Satisfies `model.mutate.001` through `model.mutate.007`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `model_mutate` is 7 of 7,
  `model.mutate.001`-`007` each tagged `covers:`; `.007`'s `bench/micro/`
  assertion went green when task 27's workloads landed. Earlier note, kept
  for its design record: `model_mutate` was 6 of 7 (it was 0 of 7).
  `Stored` records each write as a replaced input span (an insertion for an
  append, an empty replacement for a removal) and `write_traced` reports
  every output run as spliced from the input or newly encoded; a write inside
  a resource below the root re-encodes that resource whole. The red one is
  `model.mutate.007`, and only on its last assertion (`model_mutate.rs:578`):
  every byte outside `meta` is spliced, but the test also requires a file
  under `bench/micro/` naming the `model-meta-update` workload, which is task
  27's deliverable and was not created here. Measured with the `model.perf.004` workload in
  `model_bytes_lazy_perf.rs`, release build, median of five runs each: the
  4 KiB metadata update went from 3.8 us to 4.1 us and the 64 KiB one stayed
  at about 49 us (noise 48-75 us on both sides). That test was already red
  before this task on its 25% spread, and still is: the 64 KiB cost is the
  harness's own two whole-buffer copies, not the splice. Paths address the
  document as loaded, so a write below an element an earlier write removed
  is refused rather than applied to shifted indices. Append and remove are
  not implemented on a constructed (`Built`) resource and return an error
  saying so.

- [x] **25. Implement `Node` and `Model` over `frevo-model`.** Type names at
  resource, backbone and primitive level; children by element name in document
  order; a choice element by base name and nothing for the typed spelling;
  absent elements as an empty list; `all_children` with names; owned values that
  outlive their source; the resource flag; `primitive()` returning none for a
  value-less element; the type hierarchy and element-type answers.
  Satisfies `model.nav.001` through `model.nav.011`, and `model.nav.012` by
  running the same expression over a `frevo-model` node and over the harness's
  XML-backed test node and requiring equal results.
  Delivered by tasks 16 and 17 rather than by work of its own, and finished
  when `codec::R4` was bound to the generated tables: `model_nav` is 12 of 12
  with all twelve `model.nav.*` scenarios tagged, and its seam at
  `model_nav.rs:163` is the real `frevo_model::ResourceNode`, not a stand-in.
  `model.nav.012` runs the same navigation over a frevo-model node and over
  the harness's XML-backed node and asserts equal results, with its own check
  that the XML node is "a working node, not a stub".

- [x] **26. Implement decode error reporting.** Element path and byte offset on
  every error and on every lenient-mode diagnostic; no panic on any truncation
  prefix; a depth limit instead of a stack overflow.
  Satisfies `model.errors.001`, `model.errors.002`, `model.errors.004`,
  `model.errors.005`.

- [x] **27. Land the model benchmarks and record the baseline.** The five
  `bench/micro/` model workloads with their budgets and the first baseline for
  the regression policy. Budgets as amended by the owner on 2026-11-21:
  `model-meta-update` at most 5 µs (4 KiB) and 12 µs (64 KiB), and the
  regression gate on the median of five runs at 15%, the baseline recorded on
  CI or the reference environment (`model.perf.004`, `.006`). The
  oracle-builder moves `FrevoCodec::owned`'s input copies out of
  `model.perf.004`'s timed region.
  Satisfies `model.perf.001` through `model.perf.006`.
  **Implemented, not ticked.** `model_bytes_lazy_perf` is 14 of 18 (it was 12
  of 18) and `model_mutate` 7 of 7 (it was 6 of 7: `model.mutate.007` now
  finds `model-meta-update` declared in `bench/micro/workloads.yaml`). The
  workloads are `crates/frevo-model/benches/model_micro.rs` (`harness = false`,
  `std::time` only, no dependency), run with `cargo bench -p frevo-model
  --bench model_micro`; it wrote `bench/micro/baseline.json` and then
  `bench/micro/results.json` at `40ffd16` on a Windows host, not the reference
  environment. `model.perf.001` (0.197 us, 0 tokens) and `model.perf.002`
  (1.709 us, 4% of the full parse) are green. Red: `model.perf.003` (full
  parse 42.6 us against 20), `model.perf.005` (canonical encode 49.0 us
  against 25, and the test's own constructed fixture is 2,125 bytes, not the
  4 KiB it asserts), `model.perf.004` (3.34 against 7.43 us, the 64 KiB
  surcharge being the output buffer; the in-process half also times the test
  adapter's two input copies) and `model.perf.006` (baseline and results are
  the same commit, yet two rows moved more than 10%: noise). All four are in
  `tasks/blockers.md` (2026-11-12, task 27), with an A/B showing task 24 did
  not regress the metadata update.
  **Update (`8e50594`).** The decode and the canonical encode were profiled
  and fixed (a one-pass structure tape instead of rescanning each container,
  a generated hash index for `Tables::type_def`, no per-value `String`, no
  per-object `ordered_elements`), every observable checked identical to
  `02f309e`, and both records re-run twice at `8e50594`: full parse
  12.83/12.79 us, canonical encode 11.50/11.85 us, lazy read 0.88 us.
  `model_bytes_lazy_perf` is 16 of 18: `model.perf.003` and `model.perf.005`
  are green. Still red, both the owner's: `model.perf.004` (1.62 against
  5.56 us) and `model.perf.006` (pass-through 13.5% apart between two runs of
  the same binary; a noise measurement and a proposal are in
  `tasks/blockers.md`, 2026-11-12, task 27 follow-up). Not ticked.

---

## Phase D — `frevo-path`

  **Ticked 2026-11-21.** `bench/micro/baseline.json` and `results.json` were
  recorded in two sessions at `ccfb14c`, on a clean tree. Every row records
  `run_medians_us`, five per-run medians each from its own cold process.
  Median of five (results / baseline, µs):
  - `model-passthrough` 0.178 / 0.185
  - `model-lazy-read` 0.838 / 0.843
  - `model-full-parse` 13.798 / 14.228
  - `model-meta-update-4kib` 1.694 / 1.697
  - `model-meta-update-64kib` 5.601 / 5.624
  - `model-canonical-encode` 11.405 / 12.153

  `model_bytes_lazy_perf` is 18 of 18 (`model.perf.001` to `.006`) and
  `path_perf` 6 of 6. These recordings were made on the Windows development
  host and are advisory. Per `model.perf.006` the authoritative baseline is
  recorded on CI or the reference environment.

- [x] **28. Implement the lexer.** Both comment forms and their interaction with
  operators, backtick-delimited identifiers, the string escape set, and the
  rejection paths.
  Satisfies `path.lexer.001` through `path.lexer.014`.
  **Ticked 2026-11-20:** `path_lexer` is 15 of 16 and the in-crate
  `syntax::tests` pass; every test covering `.001`-`.014` passes. The one red
  test is `path.lexer.016`, whose local `double_quote_oracle_under_test`
  binding is still unbound (task 52; the product's
  `ParseOptions::allow_double_quotes` exists). Earlier note:
  **Implemented, not ticked.** `path_lexer` is 6 of 16; the other ten
  assert that a pinned suite case is graded `passed`, which needs the
  evaluator, and `path.lexer.016` needs the local
  `double_quote_oracle_under_test` binding that task 52 owns. The six that
  pass are the rejection paths, which are observable at the parse stage.
  See `tasks/blockers.md`, 2026-11-12, "tasks 28-30 cannot turn their own
  tests green": the decomposition implies an independence Phase D does not
  have. Three upstream defects are reproduced deliberately -- an unfinished
  block comment is swallowed because `FHIRLexer.error` *returns* the
  exception and `skipWhitespaceAndComments` discards it at `:314`;
  `isToken` tests `charAt(1)` inside a loop over `i` at `:125-129`; and
  `processFixedName` reads a `\\u` escape in base **32** at `:511`.

- [x] **29. Implement literals.** Booleans, strings, integers including the
  negative-literal and max cases, precision-preserving decimals, dates,
  dateTimes and times at every precision and offset, quantity literals with
  UCUM codes and with calendar-duration keywords, and the empty collection.
  Satisfies `path.literal.001` through `path.literal.015`.
  **Ticked 2026-11-20:** `path_literals` is 15 of 15 and the in-crate
  `syntax::tests` pass. Earlier note: **Implemented, not ticked.** `path_literals` is 0 of 15: every test in
  the file goes through a helper that calls `check` then `evaluate`, so
  none can pass before task 31. The literal forms themselves are covered
  by the 36 in-crate unit tests, which can see what a literal *is*.
  `DEBT-0011` records the one gap: a decimal literal above 38 significant
  digits is refused, because the mantissa is 128-bit.

- [x] **30. Implement the parser and precedence.** The reusable parsed form, the
  precedence table, parentheses, and the parse-time rejections — unbalanced
  parentheses, unknown function, wrong arity — each with an offset, and all seven
  `invalid="syntax"` oracle cases failing.
  **Implemented, not ticked.** `path_parse_errors` is 5 of 13 -- the four
  parse-time rejections with their offsets, and the stated deliverable:
  all seven `invalid="syntax"` cases fail at the parse stage and are
  graded `passed`. The rest evaluate, or belong to task 53. With the
  parser bound, 118 of the 1,191 cases reach a verdict and 1,073 report
  did-not-run; 118 is exactly the 3 syntax-invalid cases outside
  `documentFunctions` plus that group's 115 cases (4 of which are themselves
  syntax-invalid), so the parser rejects what HAPI rejects and
  accepts everything else.
  Satisfies `path.parse.001` through `path.parse.008`.
  **Ticked 2026-11-20:** the eight tests covering `path.parse.001`-`.008`
  in `path_parse_errors` all pass; the three red tests in that binary cover
  `path.errors.002`, `.003` and `.005`, which are task 53's.

- [x] **31. Implement navigation.** Leading type name matched, omitted or
  mismatched; flattening across repeating elements; absent elements as empty;
  polymorphic base-name navigation; the indexer with its out-of-range, negative
  and non-integer cases; contained resources; the logical-id rule for
  `Resource.id`; primitive extensions; `$this` and `$index`; `children()` and
  `descendants()` with the unordered-collection rule; document order preserved.
  Also `Narrative.div`: a `string` holding the XML-mode composition by name, and
  an `xhtml` item through `children()` and `descendants()` whose string is the
  same XML-mode composition (`path.navigation.025`; HAPI's HTML-mode string is
  `DEV-0028`/`DEBT-0018`).
  Satisfies `path.navigation.001` through `path.navigation.025`.
  **Implemented, not ticked.** `path_navigation` is 0 of 24: every test calls
  the check pass (task 50) before evaluating, and most also use functions and
  operators of tasks 32-37, 40, 46 and 48. The evaluator core is
  `crates/frevo-path/src/eval/`, with 36 in-crate unit tests; every function,
  operator and `%` variable it does not implement is `EvalError::Unsupported`,
  which the oracle seam reports as did-not-run. The seam's `evaluate` is bound
  over inputs typed from the pinned R4 package; the oracle still reports 1,073
  did-not-run and 118 graded, because `check` runs first. The unordered rule is
  a check-pass rule (`checkOrdered`) and is task 50's. `path.navigation.010`
  was a contradiction with the pinned engine (`DEV-0012`); the owner ruled for
  HAPI on 2026-11-17 and the scenario now says the index raises. `.011`
  still contradicts it (`DEV-0013`, pending the owner); `.013`'s seam is inside `path_navigation.rs` and was not rebound. See
  `tasks/blockers.md`, 2026-11-12, "task 31".
  `Narrative.div` (`.025`) is implemented in `src/eval/{xhtml,xhtml_entities,navigate}.rs`;
  `path_navigation` is 24 of 25, the red one asserting two answers the pinned
  engine does not give (`is FHIR.string` by operator, and `contains` on an
  `xhtml` passing the check). See `tasks/blockers.md`, 2026-11-20,
  "`path.navigation.025` asserts two results the pinned engine does not give".
  **Ticked 2026-11-20:** `path_navigation` is 25 of 25 and the in-crate
  `eval::xhtml_tests` (`path.navigation.025`) pass in the lib's 130 of 130.
  The last red case was the div-less narrative: `*` navigation over a
  `Narrative` with no `div` now yields the `status` code and one `xhtml` item
  whose string is `<div/>`, creating nothing (`Narrative.java:343`;
  `DEV-0028`/`DEBT-0018`).

- [x] **32. Implement collection predicates.** `empty`, `exists` with and without
  criteria, `not`, `all`, the four boolean folds and their non-boolean execution
  error, `subsetOf`, `supersetOf`, `isDistinct`, `distinct`, `count`.
  Satisfies `path.collection.001` through `path.collection.015`.

- [x] **33. Implement filtering, projection and subsetting.** `where`, `select`,
  `repeat` with cycle termination, `ofType` including the namespace-qualified
  form; `single` — which raises for a focus of size 0 as well as for one larger
  than 1 — `first`, `last`, `tail`, `skip`, `take`, `intersect`, `exclude`; and
  the unconditional ordered-collection rejection that applies to the indexer,
  `first`, `last`, `tail`, `skip` and `take` alike.
  Satisfies `path.filter.001` through `path.filter.009`, `path.subset.001`
  through `path.subset.013`.

- [x] **34. Implement combining and membership.** The union operator, `union`,
  `combine`, `in`, `contains`, and the empty-operand rule.
  Satisfies `path.combine.001` through `path.combine.006`.

- [x] **35. Implement equality, equivalence and ordered comparison.** Equality
  and inequality with their empty-collection result; equivalence and
  non-equivalence always returning a boolean; collection comparison in order;
  decimal comparison by value; deep comparison of complex elements; the
  differing-precision date rules; `hasValue`; the four ordered operators,
  including the three facts an implementer would otherwise get wrong — a
  multi-item operand yields **empty** and not an error, `<` rejects
  `unsignedInt` and `positiveInt` while its three mirrors accept them, and all
  four compare numerics through a binary `double`; and the two `Period`
  invariant forms; and the Zulu rewrite of a compared `dateTime`, for an element
  within the evaluation (`path.compare.013`) and for a literal's value within
  the evaluation, never carried into a later evaluation of the same parse
  (`path.compare.015`), and in both cases visible in the evaluation's returned
  items (`where`, `select($this)`); the same for a `dateTime` value the
  evaluation builds (`now()`, `dateTime` arithmetic, a `date` or `dateTime` boundary at precision 14 or 17)
  and then compares (`path.compare.015`).
  Satisfies `path.equality.001` through `path.equality.012`, `path.compare.001`
  through `path.compare.015`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.equality.001`-`.012` and `path.compare.001`-`.015` passes;
  `path_compare_equality` is 27 of 27.

- [x] **36. Implement boolean logic.** The four operators' three-valued truth
  tables, the singleton-evaluation rule for a non-boolean operand, both
  short-circuits, and `iif` with its empty-condition and missing-else cases.
  Satisfies `path.logic.001` through `path.logic.010`.
  **Ticked 2026-11-20:** `path_logic` is 10 of 10, the only binary carrying
  `path.logic.*` tags. The last red test was `path.logic.008`'s input-less
  `iif` check, which now reports a `NULL_POINTER` diagnostic where HAPI's
  `checkSingleton` dereferences a null focus.

- [x] **37. Implement arithmetic.** The operators over integers, decimals,
  strings and quantities using decimal arithmetic; concatenation's empty-as-empty
  string rule; `div` and `mod`; division by zero as the pinned engine answers it
  (empty for the integer arms and for `/` and `div` with a non-zero dividend,
  one item for a zero dividend, an error for `mod`); empty operands;
  incompatible types; and the overflow and date-difference behaviours recorded
  as the pinned engine's. `/` over numbers, and `div` and `mod` with a decimal
  operand, are a port of `org.fhir.ucum.Decimal` (`reference/Ucum-java`,
  `1.0.10`, `db9d22ad`, the suite run's version; live HAPI 8.12.0 runs 1.0.9,
  whose `Decimal` is bytecode-identical: `Decimal(String)`, `divide`, `divInt`, `trunc`,
  `modulo`, `multiply`, `subtract`, `asDecimal`), with its significant-digit
  rounding, its rounding-carry defect and its negative-zero remainder
  (`path.math.015`, `.016`, `DEV-0030`); the ported file carries the
  section 10 header and a `docs/provenance.md` row, and the BSD-3-Clause notice
  goes to `NOTICE`.
  Satisfies `path.math.001` through `path.math.011`, `path.math.015`,
  `path.math.016` and `path.math.017`.
  **Ticked 2026-11-21:** `org.fhir.ucum.Decimal` is ported in
  `crates/frevo-path/src/eval/ucum_decimal.rs`. `path_math` is 26 of 28: every
  test tagged `path.math.001`, `.002` and `.004` through `.017` passes, and the
  two red tests are `path.math.003`'s, whose `(0 - 5 'mg') = 5 'mg'` stops on
  quantity equality, and `path.mathfn.001`'s `(-5.5 'mg').abs()`. Both are
  `EvalError::Unsupported` on the UCUM quantity rules of task 43, not on
  arithmetic. `path_compare_equality`'s `path.compare.012` test passes; its
  eight red tests are all UCUM quantity cases. Lib is 136 of 136, and the
  scenario-coverage check is 524 of 524.

- [x] **38. Implement date and dateTime arithmetic.** Calendar-duration addition
  and subtraction with the engine's month-end behaviour, precision preservation,
  and the offset comparison case. `today()` and `now()` read the engine's
  configured time zone rather than the process default, and `now()` is read
  afresh on every call — no evaluation instant is cached, because the engine
  caches none.
  Satisfies `path.math.012`, `path.math.013`, `path.math.014`,
  `path.datetime.001` through `path.datetime.007`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** every test tagged with one
  of the ten IDs passes -- `path_datetime` 6 of 6, `fhirpath_oracle` 19 of
  19 (`path.datetime.004`), and the `path_math` tests
  tagged `path.math.012`-`014` (that binary's three red tests are `.003`,
  `.005` and `path.mathfn.001`, tasks 37 and 39).

- [x] **39. Implement math functions.** `abs`, `ceiling`, `floor`, `round`,
  `sqrt`, `exp`, `ln`, `log`, `power`, `truncate`, with the empty-focus and
  multi-item rules, and with the precision boundary the engine actually has:
  `sqrt`, `abs`, `ceiling`, `floor`, `exp`, `ln`, `log` and `power` go through a
  binary `double`, while `round` and `truncate` are exact.
  Satisfies `path.mathfn.001` through `path.mathfn.011`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.mathfn.001`-`.011` passes; `path_math` is 28 of 28.

- [x] **40. Implement string functions: the base set.** `upper`, `lower`,
  `toChars`, `indexOf`, `substring` with its out-of-range rule, `startsWith`,
  `endsWith`, `contains`, `length`, counting UTF-16 code units as a Java `String` does,
  rather than bytes; `toChars` yields one item per UTF-16 unit and `substring`
  may split a surrogate pair (`path.string.002`, `path.string.004`,
  `path.string.022`). A non-string focus yields **empty**, not an error, for all 15 guarded
  functions; the functions that do reject a bad focus — `round`, `truncate`,
  `single` — reject it with an error, and that difference is the point of the
  two scenarios.
  Satisfies `path.string.001` through `path.string.007`, `path.string.012`,
  `path.string.020` through `path.string.024`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `path_string` is 31 of 31,
  which carries every tag for the 13 IDs above; no other test tagged with one
  of them fails.

- [x] **41. Implement string functions: the R3 additions that need no regular
  expression engine.** `replace` (a literal substitution: the pinned
  `funcReplace` calls `String.replace`, not `java.util.regex`), `trim`, `split`
  with its empty segments, `join`, `encode` and `decode` for base64, urlbase64
  and hex, `escape` and `unescape` for html and json and also for url (form
  urlencoding, UTF-8) and md (a backslash before seven characters), and `join`
  taking at most one argument (a second is a parse failure). No crate is added.
  Satisfies `path.string.011`, `path.string.013` through `path.string.019`.

- [x] **41a. Implement string functions: the regular-expression functions.**
  **BLOCKED on ADR 0005**, a `chore/` branch ADR choosing the regular-expression
  engine; the owner approves the dependency (constitution section 5), and until
  then no crate is added and this task is not started. Split from task 41 by
  owner decision. Only three functions in the pinned engine use
  `java.util.regex`, through `RegexTimeout`: `matches`
  (`FHIRPathEngine.java:5683`, `5698`), `matchesFull` (`:5711`, `5725`, the
  pattern prefixed `(?s)`) and `replaceMatches` (`:4879`); the engine's 500 ms
  regex timeout (`regexTimeoutMillis`, `:186`) is part of the behaviour. Any
  pattern in the pinned suite's `testMatches` or `testReplaceMatches` that the
  chosen engine cannot express is recorded as a deviation.
  Satisfies `path.string.008`, `path.string.009`, `path.string.010` and
  `path.string.025` through `path.string.029` (rejected constructs, ASCII class
  rewrites, the replacement syntax, empty-match iteration, the limits). The
  `matches`, `matchesFull` and `replaceMatches` legs of `path.string.020` and
  `path.string.021`, which task 40 otherwise satisfies, are satisfied here.

- [x] **42. Implement type conversion.** Each `toX` the R4 engine has and each
  `convertsToX`, including every date, dateTime and time precision string, the
  boolean spellings, quantities, the failed-conversion asymmetry and the
  multi-item error. `toDateTime` and `toTime` are stubs that raise "not
  implemented" (`path.convert.011`, `path.convert.013`, `DEV-0021`), and
  `convertsToDate` uses the dateTime pattern (`path.convert.004`). `toDate` must **not** exist:
  `'2015'.toDate()` is a parse failure while `'2015'.convertsToDate()` is
  `true`, and shipping the symmetric pair would make Frevo accept an expression
  HAPI rejects.
  Satisfies `path.convert.001` through `path.convert.014`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.convert.001`-`.014` passes; `path_convert` is 14 of 14.

- [x] **43. Implement the UCUM unit algebra.** A port of the parts of
  `org.fhir.ucum` the engine calls (`reference/Ucum-java`, 1.0.10, `db9d22ad`;
  live HAPI 8.12.0 runs 1.0.9, identical in every ported class except
  `XmlDefinitionsParser`'s short `revision-date` branch, `path.quantity.016`):
  loading the essence file as `XmlDefinitionsParser` does, with every load
  failure of `path.quantity.016`; `Lexer` and `ExpressionParser`, with their
  error messages and the `NumberFormatException` that escapes every operator
  but `comparable()` (`path.quantity.024`); `Converter` and `ExpressionComposer`
  for canonical form (`path.quantity.020`); the special-unit `Registry` and its
  handlers (`path.quantity.023`); and `UcumEssenceService.getCanonicalForm`,
  `multiply`, `divideBy`, `getCanonicalUnits` and `isComparable`, over the
  `Decimal` of task 37. The essence file is the suite worker's
  (`org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml`, version
  `2.0.1`), not the `2.2` file `Ucum-java` bundles. Ported files carry the
  section 10 header and `docs/provenance.md` rows; the BSD-3-Clause notice goes
  to `NOTICE` with task 60's.
  Satisfies `path.quantity.016` and `path.quantity.020`, and the
  library-level halves of `path.quantity.023` and `path.quantity.024`: each
  special unit's canonical form or `UcumException`, and every parse outcome
  and message, including the `NumberFormatException`, as the library raises
  them from `getCanonicalForm`, `getCanonicalUnits`, `multiply` and
  `divideBy`. Their engine-level halves are task 44's.
  **Ticked 2026-11-21:** `src/eval/{ucum,ucum_algebra,ucum_decimal}.rs`.
  The tests tagged `path.quantity.016` and `.020` pass. The library halves
  of `.023` and `.024`, which open their two tests through the bound
  `canonical_form` seam, pass up to the first engine-level assertion
  (`1 '[pH]' = 1 'mol/L'` and `1 'm' * 1 'zzz'`), which is task 44's. The
  graded `revision-date` forms of `.016` were checked in-process: the long
  form takes `-0500`, `+0000`, `GMT` and `UTC` and refuses `Z` and
  `GMT-05:00`; the short form takes `2024-6-7`, ` 2024-06-17`,
  `2024-06-17x` and `2024-13-45`, and refuses `2024/06/17`, `+2024-06-17`
  and `""`. Lib is 136 of 136.

- [x] **44. Implement quantity semantics on top of it.** Note before starting:
  the ordered operators perform **no** dimension check, so incommensurable
  units compare by magnitude and return a boolean, while `=` with different
  canonical units is `false`, not empty (`path.quantity.022`); the engine's
  no-UCUM-data branch is unreachable in Frevo (`path.quantity.015`). A unit
  that cannot be canonicalised falls back to comparing codes, and `*` and `/`
  answer in canonical units with no `unit` (`path.quantity.017`, `.018`,
  `.021`).
 Equality after
  conversion, equivalence rounding to the lower precision, calendar-duration
  commensurability, magnitude comparison, unit arithmetic, the dimensionless
  unit, the unitless and unit-text cases from
  `parameters-example-quantities.xml` and `parameters-quantity-unit-text.xml`
  as HAPI answers them (the side-dependent unit guard of `qtyEqual` and
  `qtyEquivalent`, and execution errors carrying HAPI's recorded
  `NullPointerException` text; `path.quantity.009`, `.011`-`.013`, `DEV-0017`,
  `DEBT-0019`), the suite worker's answers in every configuration where HAPI's
  server-facing worker raises `HAPI-0274: ` (`path.quantity.019`,
  `DEV-0029`), `comparable()`, the incommensurable-comparison result, and the unknown-unit
  and non-UCUM-system behaviours.
  Also satisfies the engine-level halves of `path.quantity.023` and
  `path.quantity.024`: what `=`, `~`, the ordered operators, `comparable()`,
  `*` (with its ` (@char C)` suffix) and `/` answer for those units and codes,
  including the `NumberFormatException` under each operator. Each of the two
  scenarios is satisfied only when both tasks' halves are green.
  Satisfies `path.quantity.001` through `path.quantity.015`,
  `path.quantity.017` through `path.quantity.019`, `path.quantity.021` and
  `path.quantity.022`.
  **Ticked 2026-11-21 (verified at `0ffc970`):** `path_quantity` is 24 of
  24, carrying every tag `path.quantity.001` to `.024`, engine halves of
  `.023` and `.024` included. `path_compare_equality` is 27 of 27,
  `path_math` 28 of 28 and `path_convert` 14 of 14, and lib is 136 of 136.
  The UCUM data source is ADR 0006 (proposed).

- [x] **44a. Make an engine-built quantity a navigable `Quantity`, and read an
  empty quoted unit as no code.** From the in-process HAPI differential
  (owner ruling 2026-11-21, HAPI wins). A quantity literal, and the result of
  quantity `*` and `/`, expose `value`, `unit`, `system` and `code` to member
  navigation exactly as a `Quantity` read from a resource does, typed
  `decimal`, `string`, `uri` and `code`, an element the engine did not set
  being empty (`path.quantity.025`; `(1 'cm').code` is `code:cm` where Frevo
  answers empty today). `1 ''` holds no code, as `Quantity.setCode("")`
  leaves it, so it takes every no-code answer of `path.quantity.021`:
  `1 ''.toString()` is `1 'null'`, `1 '' / 1 'm'` fails with the
  `Pair.getCode()` `NullPointerException` text, `1 ''.comparable(1 '1')` is
  `false`, and `*` spells the code `null` (`path.quantity.026`; Frevo keeps
  `""` today and answers `1 ''`, `1.00 'm-1'`, `true` and the parse errors of
  `'.m'` and `'m.'`). The unit escapes of `path.literal.016` already match
  HAPI and must stay so. This is an **evaluation** change only. The static
  check pass must keep rejecting member navigation on a quantity literal
  (typed `System.Quantity`) and on a `*` or `/` result (no type), with
  HAPI's `FHIRPATH_UNKNOWN_NAME` texts (`path.quantity.025`, `.026`). Do
  not give these elements a type in the checker; that would make it accept
  an invariant HAPI rejects. Depends on task 4b.
  Satisfies `path.quantity.025`, `path.quantity.026` and `path.literal.016`.
  **Ticked 2026-11-21:** a named member step on a `Value::Quantity` yields
  `decimal`, `string`, `uri` and `code` elements (`eval/navigate.rs`,
  `quantity_element`), and the parser drops an empty quoted code
  (`syntax/parser.rs`). The check pass is unchanged: both check-rejection
  tests still pass.

- [x] **44b. Close `DEBT-0011`: carry decimals at any number of significant
  digits.** From the in-process HAPI differential: `1 '[pi]' * 1 '1'` and its
  siblings (`path.quantity.020#27`, `#42`, `#43`, `#46`) fail on Frevo with
  `Unsupported("decimal arithmetic beyond 38 significant digits")` where HAPI
  answers the 65-digit value, which fires `DEBT-0011`'s trigger. Replace the
  128-bit mantissa of the `Decimal` port (task 37) and of the literal path
  with the arbitrary-precision representation `design.md` section 6 names, so
  that no literal, arithmetic operator, comparison or canonical value fails on
  a limit of significant digits, and every digit `Decimal.multiply` and
  `Decimal.divide` produce is kept. Fold in `DEBT-0014` (the negative scale)
  if the new type makes it free; otherwise leave it open. When this is
  ticked, set `DEBT-0011`'s `Status` to `paid` with the commit, and correct
  `DEV-0011`'s residual sentence, which says Frevo refuses such a literal.
  This touches the evaluation path, so report `path_perf` before and after:
  the `path.perf.*` budgets still hold and neither `path-eval-compiled` nor
  `path-suite-wallclock` regresses more than 10% against task 55's `c11ac47`
  figures. Depends on task 4b.
  Satisfies `path.literal.017` and the `[pi]` bullet of `path.quantity.020`.
  **Ticked 2026-11-21 (implemented in `a900207`, green at `50da39e`):**
  `Decimal`'s mantissa is `Unscaled` (`crates/frevo-path/src/seam/unscaled.rs`),
  of any number of digits; `DEBT-0011` is paid and `DEBT-0014` left open.
  `path_perf` before/after (release): `path-eval-compiled` 3.564 -> 3.504 us,
  `path-suite-wallclock` 0.233 -> 0.230 s (c11ac47: 3.484 us, 0.219 s).

- [x] **44c. Match the `NullPointerException` and `NumberFormatException`
  texts the differential recorded.** `<=` with no `unit` on the left and one
  on the right fails with exactly
  `Cannot invoke "String.equals(Object)" because "lunit" is null`, not the
  `Cannot invoke "java.lang.String.equals(Object)" because the value is null`
  of `crates/frevo-path/src/eval/compare.rs` (`path.compare.014`,
  `path.quantity.019`; probes `path.compare.014#01`, `path.quantity.019#17`,
  `#18`). The `integer` arms of `div` and `mod` over a no-value operand fail
  with exactly `Cannot parse null string`, not `For input string: ""`
  (`path.math.017`; probes `#61`, `#62`). Both with no offset. Depends on
  task 4b.
  Satisfies the amended `path.compare.014`, `path.quantity.019` and
  `path.math.017`.
  **Ticked 2026-11-21:** `eval/compare.rs` raises the `"lunit" is null` text
  and `eval/arith.rs` (`parse_int`) the `Cannot parse null string` text for
  the integer arms of `div` and `mod`, divisor first, both with no offset.

- [x] **44d. Fail `+`, `-` and `*` over a no-value operand as HAPI does, and
  list an engine-built quantity's children.** Found by the implementer of
  tasks 44a to 44c (owner's standing ruling: HAPI wins, no new deviation).
  The integer arms of `+`, `-` and `*` read the left operand and then the
  right with `Integer.parseInt`, so a no-value `integer` operand fails with
  exactly `Cannot parse null string` (Frevo says `For input string: ""`
  today). The decimal arms build `new BigDecimal` from the left operand and
  then the right, so a no-value `decimal` operand, or a no-value `integer`
  operand beside a decimal, fails with exactly
  `Cannot invoke "String.toCharArray()" because "val" is null`. All of these
  have no offset (`path.math.018`). `children()` and `descendants()` on a
  quantity literal or a `*`/`/` result list its set elements in
  `Quantity.listChildren` order: value, unit, system, code. Frevo answers
  empty today (`path.quantity.027`). The static check must keep accepting
  `children()` and `descendants()` here with no type, and keep rejecting a
  named step after them. Depends on task 4c.
  Satisfies `path.math.018` and `path.quantity.027`.
  **Ticked 2026-11-21:** `eval/arith.rs` reads the `+`, `-` and `*` operands
  with `parse_int` and `big_decimal`, left first; `eval/navigate.rs` lists a
  `Value::Quantity`'s set elements for `*`. The check pass is unchanged.

- [x] **45. Implement the boundary functions.** `lowBoundary`, `highBoundary` and
  `precision` as the pinned engine computes them: decimal boundaries edited as
  text, with **no carry** (a `9` rounded up is `:` and the result is an
  execution error), **no borrow normalisation** (`10.lowBoundary(0)` is `09`),
  and only a lower-case `e` split off as an exponent, its digits appended with
  no `e` (`9.5e2.lowBoundary(0)` is `92`), so the 18 suite cases HAPI fails
  are `failed` with HAPI's values and errors (`DEV-0025`, `DEBT-0016`);
  out-of-range precision (outside `0`-`16`) empty for a decimal or integer but
  not checked for a quantity; negatives; integers; quantities; every date or
  dateTime boundary a `dateTime`; the date precision list 4, 6, 7, 8, 10, 14,
  17 and the time list 2, 4, 6, 9, others an error; precision 14 a dateTime
  with lexical form `...T08:00:+14:00` that parses for a `+` offset or a `-0`
  hour and is `Invalid date/time format` otherwise; time precisions below 9
  keeping a trailing separator (`10:`); a quantity at a negative precision
  truncated or valueless (`-1`) or an error (`-2`); a quantity with no value an
  error; the zero branch (`0.lowBoundary()` is `-0.50000000`); `precision()`
  raising on an empty focus with a message naming `highBoundary`; a focus of
  another `fhirType()`, including `instant`, `positiveInt`, `unsignedInt` and
  `code` elements, empty; the precision argument read as one value with
  `Integer.parseInt`; error messages in full with HAPI's
  `Error evaluating FHIRPath expression: ` prefix. The tests for `.001`, `.004`-`.007` and `.009` were
  written to the specification's values and must be amended to HAPI's by the
  oracle-builder before this task can be ticked; `.011`-`.019` need tests.
  Satisfies `path.boundary.001` through `path.boundary.019`.

- [x] **46. Implement type operators and reflection.** `is`, `as`, `ofType` and
  `type()` across the `System` and `FHIR` namespaces; the FHIR type hierarchy;
  `uri` versus `url`; an extension value's concrete choice type; `as` yielding
  empty on mismatch; the operator and function forms agreeing; unknown type
  names (`is` passes the check and is `false`, `as` and `ofType` raise at
  evaluation: `path.types.011`, `DEV-0024`); the empty-focus rule.
  Satisfies `path.types.001` through `path.types.013`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.types.001`-`.013` passes; `path_types` is 13 of 13.

- [x] **47. Implement environment variables.** The built-in system URLs, the
  three backtick shorthands, `%us-zip`, `%resource` and `%rootResource` with
  their no-focus errors, `%context`, the unknown-constant error, a host service
  returning a collection, the constant-evaluation mode passed to the host
  service, and `defineVariable` with its scoping.
  Satisfies `path.env.001` through `path.env.015`.

- [x] **47a. Implement the implicit constant-resolution sites.** Four
  constant-evaluation modes with exactly the sites the engine has: **explicit**
  for a `%name` the expression wrote, at both the evaluation and the check
  constant node; **no-value** at exactly one site, the check pass's name node
  when it is at the entry of the expression and there is no focus type at all —
  not, as is easy to assume, wherever the check pass resolves a constant;
  **implicit-before** at the entry of evaluation, before any navigation and
  before the leading-type-name match, whose non-empty answer is returned
  immediately so that a host constant **shadows a real element**; and
  **implicit-after** as a fall-through after navigation yielded empty. Gate both
  implicit modes on a non-null application context, which is the condition the
  engine tests — a host service with a null application context never sees
  either. Ship the server-facing host service answering empty for every name it
  does not own, because anything else makes a resource's own elements
  unreachable on the search-parameter extraction path. `DEV-0010` records the
  divergence from the FHIRPath grammar; match HAPI, no flag.
  Satisfies `path.env.016`, `path.env.017`.

- [x] **48. Implement the utility functions.** `trace` with its retrievable log,
  `check`, `aggregate` with and without an initial value, `sort` with no
  argument, one key and several keys including descending, and `extension(url)`
  including on a primitive. The log is `name`, `: `, then the traced items'
  string values joined by `,`, with entries joined by `; `; `trace` is the only
  construct that writes to it. Expose a has-log predicate and a take-log
  accessor, and make **taking the log clear it** (HAPI's `takeLog` does,
  `FHIRPathEngine.java:6692-6698`) — a second take with no intervening `trace`
  returns the empty string. Separately, **every evaluation starts with an empty
  log**, as each `evaluate` overload does (`:702` ... `:795`); a static check
  does not touch it. The oracle harness drains the log after every case and the
  engine is shared across all 1,191 of them, so both behaviours are needed.
  The static check rejects **every boolean condition** of `check` (it types the
  condition as a `string`, `FHIRPathEngine.java:3544-3547`), so its tests
  evaluate with the check skipped (`path.utility.003`, `path.utility.004`).
  The static check of `extension(url)` needs the extension definition from the
  model (`path.utility.009`).
  Satisfies `path.utility.001` through `path.utility.011`.

- [x] **49. Implement the host-service functions.** `conformsTo` against the
  harness's service, its unknown-profile error, `FhirPathR4`'s always-false
  default, and the distinct no-host-service error; both `memberOf` forms — the
  function returning empty unless focus and argument are singletons, the binary
  operator folding the whole left collection to one boolean — with the
  worker-context fallback when no host service is set and the differing answers
  for an unresolvable value set; the `memberOf` focus-type gate; `resolve()` for
  `#id` against the **root** resource's `contained`, for a primitive focus, for
  `Reference.identifier`, for an unresolvable reference, delegated to the host
  service, and with a host-service exception swallowed to empty;
  `extension(url)` over both `extension` and `modifierExtension`; and
  host-supplied custom functions through `resolve_function`, `check_function`,
  `execute_function` and `param_is_type`. Implement the tenth and last method on
  the host-service surface too: the `trace` log hook, which is offered the trace
  name and the traced items and whose **`true` answer suppresses the engine's
  own log entirely** while a `false` answer leaves the engine to append as well.
  With the hook implemented the host-service surface is complete; a missing
  method there is a host integration that silently cannot be written.
  Satisfies `path.utility.012` through `path.utility.028`.
  Done: `path_utility` is 28 of 28. The engine is
  `crates/frevo-path/src/eval/{hostfn,terminology}.rs` and the check pass's
  `custom_function_type`, with `Model::value_set`/`code_system` as the worker
  context; the oracle seam installs the suite's host service (`877538a`).

- [x] **50. Implement the static type-check pass.** Separate from evaluation;
  27 of the 29 `invalid="semantic"` cases failing it with the offending path
  named, and the two `documentFunctions` cases among them failing at parse,
  graded `failed`, as for HAPI;
  non-existent elements; choice-element type inference; and the pass being
  skippable as `FhirPathR4` skips it.
  Satisfies `path.check.001`, `path.check.002`, `path.check.003`,
  `path.check.004`, `path.check.013`, `path.check.014`.
  **Implemented, not ticked.** `path_check` is 2 of 14 (`001`, `013`). The
  pass is `crates/frevo-path/src/check/`, a port of `executeType` and
  `TypeDetails` whose per-function rules are a signature table
  (`check/signature.rs`); anything it cannot type is
  `CheckError::NotImplemented`. The oracle seam's `check` is bound: the run
  goes from 1,073 did-not-run / 111 failed / 7 passed to 1,019 / 126 / 46,
  with 27 of the 29 semantic-invalid cases failing at the check. `002` was
  amended on 2026-11-14 to match: the two parse-stage `documentFunctions`
  cases are graded `failed`, as for HAPI (`tasks/blockers.md`, 2026-11-14).
  `003`, `004`
  and `005`-`012` drive a seam inside `path_check.rs` that was not rebound;
  `014` needs evaluation. The `Model` trait gained five check-pass methods,
  which `R4Model` does not implement yet (`DEBT-0012`). See
  `tasks/blockers.md`, 2026-11-12, "task 50".
  Done (verified 2026-11-20 at `6051795`): `path_check` is 14 of 14, which
  includes `001`-`004`, `013` and `014`; `path.check.002`'s test asserts 27 of
  the 29 semantic-invalid cases graded `passed` at the check stage.

- [x] **50a. Implement the check pass's warning channel.** The surface that
  returns a warning list beside the inferred type; a warning never failing the
  check; collection-status warnings for an operand, a function parameter and a
  focus; the SQL-on-FHIR choice warnings behind their setting;
  unknown-extension, of-type-impossible and not-a-collection warnings; the
  simple surface discarding warnings; and the list being cleared between checks
  on a reused engine. Each warning carries a stable message id, because the
  validator will key on it, and because the message id is the case selector the
  `path-check-warnings` oracle of task 4a grades by — neither the vendored suite
  nor the HTTP differential can see this channel at all.
  Satisfies `path.check.015` through `path.check.019`, `path.strictness.008`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `path_check_warnings` is 5
  of 5 (`015`-`019`) and the `path_strictness` test tagged
  `path.strictness.008` passes (that binary's one red test is
  `path.strictness.002`, task 52). The earlier `NotImplemented` seam for
  `.008` has been rebound.

- [x] **51. Confirm the check pass's non-narrowing behaviour on type guards.**
  **No narrowing is implemented** — the owner ruled on 2026-11-12 to match HAPI
  (`tasks/blockers.md`, 2026-11-12, "task 50", finding 3), because the pinned R4
  engine has none (`FHIRPathEngine.java:1621-1637` types every operand of an
  operator chain, and `:3261-3271` every `iif` argument, against the same
  focus). The task is to confirm, against the rewritten scenarios, that the
  task 50 check pass already behaves as HAPI does: an `is` operator or `is()`
  function guard under `implies`, `and`, `or`, `xor` or `iif` changes nothing
  the check observes, so the guarded `$this.toString()` fails with
  `FHIRPATH_PRIMITIVE_ONLY` exactly as the unguarded one does; `ofType()` and
  `as` are the only constructs that retype the focus; a guard that can never be
  true makes nothing unreachable; and a multi-type `contextType` satisfies a
  focus rule when any of its types does. The 14 `isSemanticChecks` cases that
  expect success — `isEffectOnLogic3`, `isGuardFnImplies`,
  `isGuardImpliesEmpty`, `isGuardOrTypesImplies`, `isGuardAndOtherImplies`,
  `isGuardOtherAndImplies`, `isGuardOrInnerImplies`, `isGuardAnd`, `isGuardOr`,
  `isGuardOrBoth`, `isGuardNotImplies`, `isGuardIifTrue`, `isGuardIifElse`,
  `isGuardNested` — must end at the **check** stage graded `failed`, which is
  parity with HAPI, not a gap. If the confirmation finds a divergence, the fix
  is to the check pass's port of `executeType`, never a narrowing rule.
  Satisfies `path.check.005` through `path.check.012`.
  **Prerequisites:** the spec-reviewer re-reviews the rewritten scenarios; the
  oracle-builder retargets the eight tests in `tests/path_check.rs` that still
  assert narrowing; and the `type_check_engine_under_test()` seam is bound
  (task 50's finding 2). The multi-type `contextType` half already works
  (`checkOnTypes`).
  Done (verified 2026-11-20 at `6051795`): the eight tests are retargeted and
  the seam is bound; `path_check` `005`-`012` pass, including the 14
  `isSemanticChecks` guard cases ending at the check stage graded `failed`.
  No narrowing rule was added.

- [x] **52. Implement the strictness configuration and the four remaining engine
  settings.** The non-strict default, which comes from the engine's own
  initialisation for any pre-R5 version and is what `FhirPathR4`'s constructor
  re-asserts; the harness's per-case ordering, in which the check pass runs
  non-strict and the evaluation runs strict; the as-singleton rule enforced and
  not; `as` case sensitivity in both states; `allowPolymorphicNames` in both
  states, since the R4 engine does honour it in evaluation and in the check
  pass; legacy mode off; implicit string conversion off; the SQL-on-FHIR warning
  off; and liquid mode off and out of scope.
  Satisfies `path.strictness.001` through `path.strictness.010`,
  `path.lexer.015`, `path.lexer.016`, `path.string.024`.
  **Implemented, not ticked.** `EngineSettings` (`src/eval/settings.rs`) with
  `for_version`, `PRE_R5`, `DECLARED` and `server_facing`;
  `Evaluator::{for_version, settings, with_settings, with_polymorphic_names,
  with_legacy_mode}`; polymorphic names in evaluation
  (`navigate::polymorphic_children_by_name`); legacy `!=`; liquid mode
  refused as `Unsupported`; and the server-facing worker's upper-case
  type-name retry (`Run::has_type_definition`). `path_strictness` is still 2
  of 10: `.001`, `.006`-`.010` need the test seam rebound to that surface,
  `.005` has a test defect (`as(quantity)`), and `.002` needs the whole suite
  to run. See `tasks/blockers.md`, 2026-11-20, "tasks 52 and 54".
  **Ticked 2026-11-21 (verified at `c11ac47`):** the seams are rebound and the
  `path.strictness.005` defect is resolved in the spec and tests. Every test
  tagged `path.strictness.001`-`.010`, `path.lexer.015`, `.016` and
  `path.string.024` passes; `path_strictness` is 10 of 10.
  `path.strictness.002`'s zero-`did-not-run` condition now holds over the whole
  suite.

- [x] **53. Implement error reporting and the bounds.** 21 of the 27
  `invalid="execution"` cases failing at evaluation after a clean parse and
  check, with the 6 `documentFunctions` ones failing at parse, graded `failed`,
  as for HAPI; errors carrying HAPI's message, which for an error built through
  `makeException` or `rep` never contains the sub-expression's text (the
  `as`-singleton message, `path.strictness.003`, is the one template that
  renders the holder node) and ends ` (@char N)` only when the failing node has
  a start (none for a precedence group, an indexer node, a null holder or a
  location-less construction such as `opMod`'s `:3102`), `N` being HAPI's lexer
  column (each whitespace character counts 2), with an optional byte offset
  present exactly when the suffix is; no panic anywhere in the
  suite; depth and step limits; and a model decode error surfacing as an error
  rather than as an empty collection.
  Also a unary sign's error position, read just past the operand's first token
  as HAPI does (`path.errors.006`), and the `iif` singleton and `is`
  parameter-kind messages (`path.errors.007`).
  Satisfies `path.errors.001` through `path.errors.007`.
  **Ticked 2026-11-21 (verified at `c11ac47`):** every test tagged
  `path.errors.001`-`.007` passes (`path_parse_errors` 15 of 15, with the
  `path.errors.*` tests in the other binaries). The suite run has 0
  `did-not-run` cases and no panic.

- [x] **54. Implement the public evaluation surface.** Evaluate from text,
  evaluate a parsed expression repeatedly, evaluate-first with its no-match
  case, the unexpected-return-type error, and synchronous evaluation callable
  from inside a transaction. The return-type check is HAPI's (`path.api.005`,
  rewritten after the forty-first review): `Base`, the primitive, composite and
  reference kinds or a FHIR type name standing for its R4 model class, accepted
  by the R4 Java class ancestry from the table task 14b generates
  (`codegen.ancestry.*`), with HAPI's exact `HAPI-0256: ` message and Java
  class names. The implementer replaces the FHIR-type check with a lookup of
  each item's row (by its element, or by the class the engine built) and of
  the requested type's row or kind, and takes the message's names from the
  rows. Depends on task 14b; the owner chose option (c) on 2026-11-21.
  Satisfies `path.api.001` through `path.api.006`.
  **Implemented, not ticked.** `src/eval/api.rs`: `Evaluator::{parse,
  evaluate_text, evaluate_text_as, evaluate_first_text, evaluate_first,
  evaluate_first_as, evaluate_as}` and `PathError`. `path_api` is still 5 of
  8: `.003`-`.005` need the test seam rebound (`tasks/blockers.md`,
  2026-11-20, "tasks 52 and 54").
  **Ticked 2026-11-21 (verified at `8023ed2`).** `evaluate_as` answers from
  the generated R4 class table (task 14b), and every test tagged
  `path.api.001` to `.006` passes: `path_api` 15 of 15, lib 136 of 136.

- [x] **55. Land the path benchmarks and record the baseline.** The four
  `bench/micro/` path workloads plus the suite wall-clock, with their budgets
  and the first baseline.
  Satisfies `path.perf.001` through `path.perf.006`.
  **Implemented, not ticked.** `path_perf` is 5 of 6. The workloads are
  `crates/frevo-path/benches/path_micro.rs` (`path-compile`,
  `path-suite-wallclock`, over the committed oracle harness included by path)
  and `crates/frevo-model/benches/path_bytes.rs` (`path-eval-compiled`,
  `path-extract-patient`, over the byte-backed `ResourceNode`, `R4Model` and
  `ServerHostServices` with an application context -- they live in
  `frevo-model` because `path.perf.004` needs a byte-backed resource and
  `path.api.008` forbids the edge the other way). All three `bench/micro/`
  benches now write through `bench/micro/record.rs`, which replaces only the
  writer's own rows. Baseline and results recorded at `584e668`, clean tree,
  release, on a Windows host and not the reference environment: compile
  7.85 us, eval 3.48 us (one parse), extract 136.7 us with 0 of 19
  materialised elements untouched, suite 0.219 s. `ResourceNode::children`
  now filters by name before building nodes (eval 5.11 -> 3.38 us, extract
  234 -> 136 us) and `primitive()` records the element as materialised, so
  `.004`'s zero is measured rather than vacuous. Red: `path.perf.005`, whose
  in-process half requires zero `did-not-run` and the run has 35 (the
  `toQuantity`/UCUM quantity cases); the recorded 0.219 s is within budget.
  Finding for the oracle: the in-process ratio halves of `path.perf.001` and
  `.002` (last-50/last-100 against first, debug profile) fail intermittently
  under parallel load -- about half of `cargo test` runs, while every
  `--test-threads=1` run and every isolated run passes (e.g. "first 50
  compiles median 35.50 us, the last 50 median 68.95 us" with the other five
  tests running beside it).
  **Ticked 2026-11-21 (verified at `c11ac47`):** `path_perf` is 6 of 6 in 13
  runs: 3 isolated, 4 more of the binary, 1 with `--test-threads=1`, and 5
  inside full-crate runs. The `.001`/`.002` ratio intermittency recorded above
  did not recur. The results were re-recorded at `c11ac47` (release, clean
  tree, same Windows host as the baseline). Against the `584e668` baseline:
  `path-compile` 7.284 us (7.194, +1.3%), `path-eval-compiled` 2.974 us (3.373,
  -11.8%), `path-extract-patient` 143.35 us (142.40, +0.7%) with 0 untouched
  elements materialised, and `path-suite-wallclock` 0.233 s (0.218, +6.9%; it
  now runs the 35 cases that did not run before: 984 passed, 207 failed, 0
  `did-not-run`). Every figure is within its budget and within the 10%
  regression limit.

---

## Phase E — the round-trip oracle and closing out

- [x] **56. Wire the `r4-example-roundtrip` oracle.** Decode, canonically
  re-encode and compare byte-for-byte for every example, with per-file
  reporting, per-resource-type coverage counts including types with no example,
  and XML inputs reported `not-applicable-xml` rather than `passed`. Run against
  the interim 103-example selector until task 59 lands the full package.
  Satisfies `codegen.roundtrip.001`, `codegen.roundtrip.002`,
  `codegen.roundtrip.003`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `codegen_r4` is 50 of 50
  with the interim directory fetched; the three tagged tests assert the 72
  JSON examples byte-identical, the report naming itself interim, one case
  per file, 146 coverage rows with 69 covered and 77 listed at zero, and the
  XML inputs not claimed. Re-run against the full package when task 59
  lands.
  **Re-verified 2026-11-21 on the full package:** `codegen_r4` is 57 of 57,
  and all 5,308 JSON examples of `hl7.fhir.r4.examples#4.0.1` round-trip
  byte-identically, once the codec stopped dropping `resourceType` outside a
  resource's own object (`ExampleScenario-example.json`).

- [x] **57. Check every generated search-parameter expression against the
  parser.** All 1,375 expressions parse; a parameter with no expression recorded
  as having none rather than an empty one. This is the first real integration of
  `frevo-codegen`'s output with `frevo-path`, and the first place the two can
  disagree.
  Satisfies `codegen.searchparam.005`.
  **Ticked 2026-11-20 (verified at `a5301eb`):** `codegen_r4`'s
  `every_table_expression_parses`, the one test tagged
  `codegen.searchparam.005`, passes (50 of 50 in that binary).

- [x] **58. Run the full gate set and close the loop.** `cargo fmt --all
  --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
  nextest run --workspace`, `cargo deny check`, the FHIRPath suite against the
  HAPI rate, the round-trip oracle, and the `bench/micro/` budgets. Record
  every case Frevo fails that HAPI passes. Anything that genuinely cannot pass
  goes to `docs/known-gaps.md` with its exact case identifiers and stops for
  owner approval — it is not skipped, ignored or filtered out.
  **Ticked 2026-11-21:** every gate is green in
  `gates/add-fhirpath-and-codegen-r4/report.json` (head `c54959c`, committed
  in `5493135`), and CI run `44962400658` passes all 8 checks. The
  differential gate is graded by the owner-ruled **in-process probe**: HAPI
  8.12.0's R4 `FHIRPathEngine` run in-process beside Frevo
  (`harness/fhirpath-compare/differential-results.json`), **not** a live
  HAPI server.

- [x] **59. [chore] Pin the full R4 example package as an oracle.** On a
  `chore/oracles-*` branch: add `hl7.fhir.r4.examples#4.0.1` to
  `oracles/PINS.md` with its licence determination and digest, add it to
  `oracles/fetch.sh`, add the `r4-example-roundtrip` suite to `oracles/run.sh`'s
  suite list, and switch task 56's selector from the interim 103 examples to the
  full set. Owner checkpoint: it changes what the round-trip gate measures.
  **Ticked 2026-11-21:** the selector runs the full package (5,308 JSON
  examples, `73c36e6`), and the round trip is fully green: `codegen_r4` is 57
  of 57.

- [x] **60. [chore] Record the UCUM notice.** On the same branch: add
  Regenstrief's UCUM copyright notice for `ucum-essence.xml` to `NOTICE`, and
  record in `oracles/PINS.md` that the file is fetched inside the Apache-2.0
  `fhir-test-cases` tree but carries its own licence terms.
  **Ticked 2026-11-21:** the work merged in PR 31.

- [x] **61. Update `docs/inventory.md` and `docs/provenance.md`.** In
  `docs/inventory.md`: move the `frevo-path` rows this change covers to `done`
  with their scenario IDs, and record that
  `hapi.fhir.fhirpath_interceptor_enabled`, the three patch rows and `$diff`
  stay `not started` against the later changes named in `proposal.md`. In
  `docs/provenance.md`: change `frevo-path` from `empty` to `derived`, naming
  `org.hl7.fhir.core` `org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at
  `73c432b5f43ddbce979978a760ec4aa81e929094` and `hapi-fhir`
  `hapi-fhir-structures-r4/.../FhirPathR4.java` at
  `7ab4c7c7c49281496faa1b3c2498a652457669ae`; change `frevo-model` from `empty`
  to `generated`, naming `frevo-codegen` and the `hl7.fhir.r4.core#4.0.1`
  package with its digest; change `frevo-codegen` from `empty` to `independent`.
  Confirm every derived file carries the required provenance header and every
  generated file the `@generated` marker.
  **Ticked 2026-11-21.** `docs/inventory.md` covers it in its new "Notes by
  change" section. This change covers no inventory row, so the five
  `frevo-path` rows stay `not started` against `add-server-skeleton`
  (`hapi.fhir.fhirpath_interceptor_enabled`), `add-store-diff` (`$diff`) and
  `add-store-patch` (the three patch rows). In `docs/provenance.md`:
  - `frevo-path` is `derived` and names `FHIRPathEngine.java` at `73c432b5`
    and `FhirPathR4.java` at `7ab4c7c7`.
  - `frevo-model` is `generated` from `hl7.fhir.r4.core#4.0.1` with both
    SHA-256 digests. It is also `derived` in `src/codec/{decode,built,nav,pretty}.rs`,
    which carry headers for the R4 `JsonParser` family, BSD-3 (owner decision,
    2026-11-21), as well as `hapi-fhir`.
  - `frevo-codegen` is `independent`.

  Every file in `crates/frevo-model/src/r4/` carries `@generated`. Every
  derived file has its header; `src/eval/error.rs`, the one derived file
  missing it (from `PathEngineException.java`), gained it.

- [x] **61a. Confirm the ten deviation entries against the implementation.**
  `DEV-0001` to `DEV-0010` in `docs/deviations.md` were written from the pinned
  Java source before any Rust existed. Re-run each one's reproducing expression
  through both engines, confirm the recorded HAPI behaviour is what HAPI
  actually does, fill in the `Reviewed` line, and raise all ten at the
  checkpoint. An entry whose recorded behaviour turns out to be wrong is
  corrected or deleted, not left standing — an inaccurate deviation is worse
  than none, because it licenses a difference nobody checked. `DEV-0010` needs
  `DEBT-0010` added to `docs/tech-debt.md`, which lives on `main` and so cannot
  be written from this branch; open that as part of the merge.
  **Ticked 2026-11-21 (done at `505399b`).** Each entry's reproducing
  expressions were re-run through an in-process HAPI 8.12.0 probe (`FhirPathR4`
  and the raw `FHIRPathEngine`, `org.hl7.fhir.r4` 6.9.12) and through Frevo,
  and all ten reproduce. Three entries were corrected: `DEV-0005`
  (`FhirPathR4` throws rather than reaching the no-UCUM branch), `DEV-0009`
  (`toTime` and `toDateTime` are stubs) and `DEV-0010` (unreachable through
  `FhirPathR4`, whose application context is null). The `Reviewed` lines are
  filled in. `DEBT-0010` is present in `docs/tech-debt.md`. The ten are raised
  at the checkpoint.

- [ ] **62. Update `tasks/graph.yaml` and finalise ADR 0004.** Set this change's
  `status` to `done`, so `add-server-skeleton` and `add-store-schema` unblock.
  Confirm ADR 0004 carries task 2's measured numbers and a recorded verdict, as
  its own closing line requires before this change is archived. Then archive.
