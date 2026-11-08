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

- [ ] **1. Build the FHIRPath oracle harness.** Parse
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
  weakening the oracle under constitution rule 3. Fix the evaluation time zone to `UTC+1100`, as
  `FHIRPathTests.test` does. The harness runs against a trait object, so it
  works before any engine exists.
  Satisfies `path.oracle.001`, `path.oracle.002`, `path.oracle.003`,
  `path.oracle.004`, `path.oracle.005`, `path.oracle.006`, `path.oracle.007`,
  `path.oracle.008`, `path.oracle.009`, `path.oracle.010`, `path.oracle.015`,
  `path.oracle.017`, `path.datetime.004`.

- [ ] **2. Measure `helios-fhirpath` and `haste-fhirpath` through the harness and
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

- [ ] **3. Measure HAPI 8.12.0's own pass rate on the same 1,191 cases and wire
  it into the gate.** Run `org.hl7.fhir.r4.test.FHIRPathTests` at pin
  `73c432b5` against the same suite file, record each case's status, and load
  that record into the harness so every case reports Frevo's status beside
  HAPI's. Implement the gate as "Frevo's `passed` count at least HAPI's", with
  every case HAPI passes and Frevo fails listed individually regardless of the
  aggregate. Confirm and record that all 115 `documentFunctions` cases fail for
  HAPI too, with the reason that the R4 `ExpressionNode.Function` enum
  implements none of the seven functions. Record a status of `hapi-cannot-pass`,
  distinct from `failed`, for the `isSemanticChecks` pairs whose expressions are
  byte-identical and differ only by `context`/`contextType` —
  `isEffectOnLogic2Base`/`isEffectOnLogic3Base`, `isGuardOr`/`isGuardOrBad`,
  `isGuardXorBad`/`isGuardXorBad2`,
  `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad` — because the pinned R4
  harness reads neither attribute and so necessarily fails one of each pair.
  Satisfies `path.oracle.011`, `path.oracle.012`, `path.oracle.014`,
  `path.oracle.016`.

- [ ] **4. Write the failing tests for every `path.*` scenario.** One test per
  scenario ID, each carrying a `// covers: path.<area>.<nnn>` comment, with the
  oracle-backed scenarios driven from the harness's case selectors and the rest
  written directly. All 361 must fail, and none by `todo!()`, `#[ignore]` or a
  bare `#[should_panic]`. Where a scenario says it is differential-only, the
  test is written against a recorded HAPI response rather than against a suite
  case, and says so. `path.check.015`-`019` are driven from the in-process
  `path-check-warnings` oracle of task 4a, not from a suite selector.
  Covers all 361 `path.*` scenarios.

- [ ] **4a. Build the `path-check-warnings` in-process oracle.** The check
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

- [ ] **5. Write the failing tests for every `codegen.*` and `model.*`
  scenario.** Same discipline: one test per scenario ID with a `covers:`
  comment, 49 plus 83 of them, including the `bench/micro/` workloads as
  benchmarks with assertions on their budgets. Interim round-trip selector is
  the 103 examples in `oracles/fhir-test-cases/fetched/r4/examples/`, with the
  test reporting that it is the interim set.
  Covers all 49 `codegen.*` and 83 `model.*` scenarios.

- [ ] **5a. Build the `model-parser-errors` in-process oracle.** Three of HAPI's
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

---

## Phase B — `frevo-codegen`

- [ ] **6. Fetch and pin the R4 definition package.** A script that fetches
  `hl7.fhir.r4.core#4.0.1` into a git-ignored directory and verifies it against a
  SHA-256 recorded in the repository, plus the generator's refusal paths.
  Satisfies `codegen.inputs.001`, `codegen.inputs.002`, `codegen.inputs.003`,
  `codegen.inputs.004`.

- [ ] **7. Read StructureDefinitions and assert the input inventory.** Parse the
  package's definitions, classify them by `kind`, `abstract` and `derivation`,
  and fail the run on a count other than 63 datatypes, 149 resource definitions
  (146 concrete, 2 abstract, 1 logical) and 1,375 SearchParameters. Add the
  `reference/`-conditional cross-check against the HAPI copy at `7ab4c7c7`.
  Satisfies `codegen.inputs.005`, `codegen.inputs.006`.

- [ ] **8. Generate primitive and complex datatypes.** The 20 primitives with
  their value regexes, the complex types, and the metadata types.
  Satisfies `codegen.types.001`, `codegen.types.002`, `codegen.types.003`.

- [ ] **9. Generate resources, abstract element sets and backbone elements.** The
  146 concrete resources, the shared `Resource` and `DomainResource` element
  sets, nested backbone types at every level, and termination on recursive
  backbones and datatype cycles.
  Satisfies `codegen.types.004`, `codegen.types.005`, `codegen.types.006`,
  `codegen.types.007`, `codegen.types.008`, `codegen.types.009`.

- [ ] **10. Generate choice elements and cardinality.** `value[x]` under its base
  name with its full property-name map and its declared type set, the
  single-permitted-type case, the two-values-for-one-choice rejection, and the
  four cardinality shapes.
  Satisfies `codegen.choice.001`, `codegen.choice.002`, `codegen.choice.003`,
  `codegen.choice.004`, `codegen.cardinality.001`, `codegen.cardinality.002`,
  `codegen.cardinality.003`, `codegen.cardinality.004`.

- [ ] **11. Generate the Rust naming map.** snake_case fields, deterministic
  keyword escaping, collision detection that fails rather than loses an element,
  verbatim type names, and enumeration variants whose original code strings are
  recoverable.
  Satisfies `codegen.naming.001`, `codegen.naming.002`, `codegen.naming.003`,
  `codegen.naming.004`, `codegen.naming.005`.

- [ ] **12. Generate required-binding enumerations.** Closed enumerations only
  for resolvable `required` bindings; codes outside a required binding preserved
  rather than dropped; nothing generated for extensible, preferred or example
  bindings or for an unresolvable value set.
  Satisfies `codegen.binding.001`, `codegen.binding.002`,
  `codegen.binding.003`, `codegen.binding.004`.

- [ ] **13. Generate the search-parameter table.** All 1,375 rows with code, URL,
  type, bases, expression, composite components, reference targets and the
  modifier flags, with absent distinguished from false, and a row per base for a
  multi-base parameter.
  Satisfies `codegen.searchparam.001`, `codegen.searchparam.002`,
  `codegen.searchparam.003`, `codegen.searchparam.004`,
  `codegen.searchparam.006`.

- [ ] **14. Generate the element-order tables and enforce output discipline.**
  Per-type element order for serialisation; byte-identical regeneration on the
  same platform and across platforms; the `@generated` header naming the
  generator, the package, the version and the digest; `fmt` and `clippy -D
  warnings` clean with no added `allow`; the CI drift check; no `unsafe`.
  Satisfies `codegen.roundtrip.004`, `codegen.output.001`,
  `codegen.output.002`, `codegen.output.003`, `codegen.output.004`,
  `codegen.output.005`, `codegen.output.006`.

- [ ] **15. Check the compile-time budget and decide.** Measure a cold
  `cargo build -p frevo-model --release` on the reference environment against
  the 120 s and 40 MB budgets in `design.md` section 8. If either is missed,
  write the ADR for moving the codec from generated code to runtime-interpreted
  tables, and stop for the owner rather than improvising.

---

## Phase C — `frevo-model`

- [ ] **16. Define the `Value`, `Node`, `Model` and `HostServices` traits in
  `frevo-path`.** The seam from `design.md` section 4, with no implementation
  behind it. Adding it here rather than in Phase D is deliberate: `frevo-model`
  depends on these definitions, and `frevo-path` must not depend on
  `frevo-model`.
  Satisfies `path.api.007`, `path.api.008`.

- [ ] **17. Implement the span-based buffer, the lazy index and pass-through.**
  `Arc<[u8]>` plus per-object lazily built index; load-and-serialise-unchanged as
  a byte copy with a zero token counter; reading an element not disturbing
  pass-through; the constructed form serialising in generated element order; the
  two forms indistinguishable to a reader.
  Satisfies `model.bytes.001`, `model.bytes.002`, `model.bytes.003`,
  `model.bytes.004`, `model.bytes.005`, `model.bytes.006`.

- [ ] **18. Implement lazy materialisation and its instrumentation.** Unread
  siblings, indexed access, counting without materialising, idempotent cached
  reads, contained resources and narrative untouched by unrelated reads — each
  observable through a not-materialised report.
  Satisfies `model.lazy.001` through `model.lazy.006`.

- [ ] **19. Implement JSON decoding: structure.** `resourceType` dispatch and its
  three error paths, choice decoding from the typed property name, contained
  resources, Bundle entries of mixed types, and element `id` and `extension` on
  any element. The unknown-property path is task 21's, because its outcome
  depends on the parse mode.
  Satisfies `model.json.001`, `model.json.002`, `model.json.003`,
  `model.json.005`, `model.json.006`, `model.json.007`, `model.json.020`.

- [ ] **20. Implement JSON decoding: primitive values.** Opaque `xhtml`,
  lexical-form decimals with no binary-float path, `integer`, `positiveInt` and
  `unsignedInt` bounds, `base64Binary` carried as its encoded text.
  Satisfies `model.json.008`, `model.json.009`, `model.json.010`,
  `model.json.011`, `model.json.012`, `model.json.017`.

- [ ] **21. Implement the two parse modes and the diagnostic channel.** Lenient
  by default, matching the handler `FhirContext` installs: an unknown property,
  a JSON type mismatch, a single object where an array belongs, an array where a
  single value belongs, a duplicate key (last wins), a `null` in a non-primitive
  array and a missing required element all **succeed** and return a diagnostic
  beside the resource. The two genuinely fatal conditions fail in both modes: an
  extension carrying both a value and nested extensions, and a non-blank invalid
  primitive value. Strict mode turns every diagnostic into an error and changes
  nothing else, with identical paths and offsets. Non-UTF-8 input fails in both.
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

- [ ] **21a. Implement the two contained-resource decode diagnostics.** A
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

- [ ] **22. Implement the `_field` primitive-extension representation.** Two
  independent spans per node; singleton and index-aligned array forms; `null`
  holes in either array; `hasValue` as the presence of a value span; a
  value-less element that still exists; unequal-length arrays reported; in-place
  round-trip; reading a value without materialising its extensions; `_` rejected
  on a complex element.
  Satisfies `model.primext.001` through `model.primext.009`.

- [ ] **23. Implement canonical and normal serialisation.** Lexicographic key
  order with no whitespace for canonical; generated element order for normal;
  array order preserved in both; presented decimal form preserved;
  idempotence; and the direct-versus-round-trip equality the oracle compares.
  Satisfies `model.canonical.001` through `model.canonical.006`.

- [ ] **23a. Implement the encode-side dangling-internal-reference diagnostic.**
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

- [ ] **24. Implement copy-on-write mutation and byte splicing.** Untouched
  subtrees keeping their original bytes on element change, append and remove;
  the modified flag; a same-value write still marking modified; splicing at a
  contained resource's boundary; the `meta` update path.
  Satisfies `model.mutate.001` through `model.mutate.007`.

- [ ] **25. Implement `Node` and `Model` over `frevo-model`.** Type names at
  resource, backbone and primitive level; children by element name in document
  order; a choice element by base name and nothing for the typed spelling;
  absent elements as an empty list; `all_children` with names; owned values that
  outlive their source; the resource flag; `primitive()` returning none for a
  value-less element; the type hierarchy and element-type answers.
  Satisfies `model.nav.001` through `model.nav.011`, and `model.nav.012` by
  running the same expression over a `frevo-model` node and over the harness's
  XML-backed test node and requiring equal results.

- [ ] **26. Implement decode error reporting.** Element path and byte offset on
  every error and on every lenient-mode diagnostic; no panic on any truncation
  prefix; a depth limit instead of a stack overflow.
  Satisfies `model.errors.001`, `model.errors.002`, `model.errors.004`,
  `model.errors.005`.

- [ ] **27. Land the model benchmarks and record the baseline.** The five
  `bench/micro/` model workloads with their budgets and the first baseline for
  the regression policy.
  Satisfies `model.perf.001` through `model.perf.006`.

---

## Phase D — `frevo-path`

- [ ] **28. Implement the lexer.** Both comment forms and their interaction with
  operators, backtick-delimited identifiers, the string escape set, and the
  rejection paths.
  Satisfies `path.lexer.001` through `path.lexer.014`.

- [ ] **29. Implement literals.** Booleans, strings, integers including the
  negative-literal and max cases, precision-preserving decimals, dates,
  dateTimes and times at every precision and offset, quantity literals with
  UCUM codes and with calendar-duration keywords, and the empty collection.
  Satisfies `path.literal.001` through `path.literal.015`.

- [ ] **30. Implement the parser and precedence.** The reusable parsed form, the
  precedence table, parentheses, and the parse-time rejections — unbalanced
  parentheses, unknown function, wrong arity — each with an offset, and all seven
  `invalid="syntax"` oracle cases failing.
  Satisfies `path.parse.001` through `path.parse.008`.

- [ ] **31. Implement navigation.** Leading type name matched, omitted or
  mismatched; flattening across repeating elements; absent elements as empty;
  polymorphic base-name navigation; the indexer with its out-of-range, negative
  and non-integer cases; contained resources; the logical-id rule for
  `Resource.id`; primitive extensions; `$this` and `$index`; `children()` and
  `descendants()` with the unordered-collection rule; document order preserved.
  Satisfies `path.navigation.001` through `path.navigation.024`.

- [ ] **32. Implement collection predicates.** `empty`, `exists` with and without
  criteria, `not`, `all`, the four boolean folds and their non-boolean execution
  error, `subsetOf`, `supersetOf`, `isDistinct`, `distinct`, `count`.
  Satisfies `path.collection.001` through `path.collection.015`.

- [ ] **33. Implement filtering, projection and subsetting.** `where`, `select`,
  `repeat` with cycle termination, `ofType` including the namespace-qualified
  form; `single` — which raises for a focus of size 0 as well as for one larger
  than 1 — `first`, `last`, `tail`, `skip`, `take`, `intersect`, `exclude`; and
  the unconditional ordered-collection rejection that applies to the indexer,
  `first`, `last`, `tail`, `skip` and `take` alike.
  Satisfies `path.filter.001` through `path.filter.009`, `path.subset.001`
  through `path.subset.013`.

- [ ] **34. Implement combining and membership.** The union operator, `union`,
  `combine`, `in`, `contains`, and the empty-operand rule.
  Satisfies `path.combine.001` through `path.combine.006`.

- [ ] **35. Implement equality, equivalence and ordered comparison.** Equality
  and inequality with their empty-collection result; equivalence and
  non-equivalence always returning a boolean; collection comparison in order;
  decimal comparison by value; deep comparison of complex elements; the
  differing-precision date rules; `hasValue`; the four ordered operators,
  including the three facts an implementer would otherwise get wrong — a
  multi-item operand yields **empty** and not an error, `<` rejects
  `unsignedInt` and `positiveInt` while its three mirrors accept them, and all
  four compare numerics through a binary `double`; and the two `Period`
  invariant forms.
  Satisfies `path.equality.001` through `path.equality.012`, `path.compare.001`
  through `path.compare.012`.

- [ ] **36. Implement boolean logic.** The four operators' three-valued truth
  tables, the singleton-evaluation rule for a non-boolean operand, both
  short-circuits, and `iif` with its empty-condition and missing-else cases.
  Satisfies `path.logic.001` through `path.logic.010`.

- [ ] **37. Implement arithmetic.** The operators over integers, decimals,
  strings and quantities using decimal arithmetic; concatenation's empty-as-empty
  string rule; `div` and `mod`; division by zero as empty; empty operands;
  incompatible types; and the overflow and date-difference behaviours recorded
  as the pinned engine's.
  Satisfies `path.math.001` through `path.math.011`.

- [ ] **38. Implement date and dateTime arithmetic.** Calendar-duration addition
  and subtraction with the engine's month-end behaviour, precision preservation,
  and the offset comparison case. `today()` and `now()` read the engine's
  configured time zone rather than the process default, and `now()` is read
  afresh on every call — no evaluation instant is cached, because the engine
  caches none.
  Satisfies `path.math.012`, `path.math.013`, `path.math.014`,
  `path.datetime.001` through `path.datetime.007`.

- [ ] **39. Implement math functions.** `abs`, `ceiling`, `floor`, `round`,
  `sqrt`, `exp`, `ln`, `log`, `power`, `truncate`, with the empty-focus and
  multi-item rules, and with the precision boundary the engine actually has:
  `sqrt`, `abs`, `ceiling`, `floor`, `exp`, `ln`, `log` and `power` go through a
  binary `double`, while `round` and `truncate` are exact.
  Satisfies `path.mathfn.001` through `path.mathfn.011`.

- [ ] **40. Implement string functions: the base set.** `upper`, `lower`,
  `toChars`, `indexOf`, `substring` with its out-of-range rule, `startsWith`,
  `endsWith`, `contains`, `length`, counting unicode scalar values rather than
  bytes. A non-string focus yields **empty**, not an error, for all 15 guarded
  functions; the functions that do reject a bad focus — `round`, `truncate`,
  `single` — reject it with an error, and that difference is the point of the
  two scenarios.
  Satisfies `path.string.001` through `path.string.007`, `path.string.012`,
  `path.string.020` through `path.string.024`.

- [ ] **41. Implement string functions: regular expressions and the R3
  additions.** `matches`, `matchesFull`, `replaceMatches`, `replace`, `trim`,
  `split` with its empty segments, `join`, `encode` and `decode` for base64,
  urlbase64 and hex, `escape` and `unescape` for html and json.
  Satisfies `path.string.008` through `path.string.011`, `path.string.013`
  through `path.string.019`.

- [ ] **42. Implement type conversion.** Each `toX` the R4 engine has and each
  `convertsToX`, including every date, dateTime and time precision string, the
  boolean spellings, quantities, the failed-conversion asymmetry, the multi-item
  error, and precision-preserving widening. `toDate` must **not** exist:
  `'2015'.toDate()` is a parse failure while `'2015'.convertsToDate()` is
  `true`, and shipping the symmetric pair would make Frevo accept an expression
  HAPI rejects.
  Satisfies `path.convert.001` through `path.convert.014`.

- [ ] **43. Implement the UCUM unit algebra.** Canonical-form conversion from the
  pinned `ucum-essence.xml`, commensurability, multiplication and division of
  units, and the reported-version and missing-file failure paths.
  Satisfies `path.quantity.016`.

- [ ] **44. Implement quantity semantics on top of it.** Note before starting:
  canonicalisation performs **no** dimension check, so incommensurable units
  compare by magnitude and return a boolean, and with no UCUM data two
  quantities with differing unit codes compare `false`. Neither is an error.
 Equality after
  conversion, equivalence rounding to the lower precision, calendar-duration
  commensurability, magnitude comparison, unit arithmetic, the dimensionless
  unit, the unitless and unit-text cases from
  `parameters-example-quantities.xml` and `parameters-quantity-unit-text.xml`,
  `comparable()`, the incommensurable-comparison result, and the unknown-unit
  and non-UCUM-system behaviours.
  Satisfies `path.quantity.001` through `path.quantity.018`.

- [ ] **45. Implement the boundary functions.** `lowBoundary`, `highBoundary` and
  `precision` over decimals with their carry and borrow cases, out-of-range
  precision arguments as empty, negatives, exponent-form inputs read from stored
  lexical form, quantities, dates, dateTimes and times, and integers.
  Satisfies `path.boundary.001` through `path.boundary.010`.

- [ ] **46. Implement type operators and reflection.** `is`, `as`, `ofType` and
  `type()` across the `System` and `FHIR` namespaces; the FHIR type hierarchy;
  `uri` versus `url`; an extension value's concrete choice type; `as` yielding
  empty on mismatch; the operator and function forms agreeing; unknown type
  names as a semantic error; the empty-focus rule.
  Satisfies `path.types.001` through `path.types.013`.

- [ ] **47. Implement environment variables.** The built-in system URLs, the
  three backtick shorthands, `%us-zip`, `%resource` and `%rootResource` with
  their no-focus errors, `%context`, the unknown-constant error, a host service
  returning a collection, the constant-evaluation mode passed to the host
  service, and `defineVariable` with its scoping.
  Satisfies `path.env.001` through `path.env.015`.

- [ ] **47a. Implement the implicit constant-resolution sites.** Four
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

- [ ] **48. Implement the utility functions.** `trace` with its retrievable log,
  `check`, `aggregate` with and without an initial value, `sort` with no
  argument, one key and several keys including descending, and `extension(url)`
  including on a primitive. The log is `name`, `: `, then the traced items'
  string values joined by `,`, with entries joined by `; `; `trace` is the only
  construct that writes to it. Expose a has-log predicate and a take-log
  accessor, and make **taking the log clear it** — a second take with no
  intervening `trace` returns the empty string. The oracle harness drains the
  log after every case and the engine is shared across all 1,191 of them, so
  without the clear-on-take one case's lines leak into the next.
  Satisfies `path.utility.001` through `path.utility.011`.

- [ ] **49. Implement the host-service functions.** `conformsTo` against the
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

- [ ] **50. Implement the static type-check pass.** Separate from evaluation;
  all 29 `invalid="semantic"` cases failing it with the offending path named;
  non-existent elements; choice-element type inference; and the pass being
  skippable as `FhirPathR4` skips it.
  Satisfies `path.check.001`, `path.check.002`, `path.check.003`,
  `path.check.004`, `path.check.013`, `path.check.014`.

- [ ] **50a. Implement the check pass's warning channel.** The surface that
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

- [ ] **51. Implement type-guard narrowing in the check pass.** The `is` operator
  and `is()` function narrowing the guarded operand under `implies`, `and`, `or`
  and `iif`; the unreachable-operand rule when the narrowed set is empty; and a
  multi-type `contextType`. This is the `isSemanticChecks` group.
  Satisfies `path.check.005` through `path.check.012`.

- [ ] **52. Implement the strictness configuration and the four remaining engine
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

- [ ] **53. Implement error reporting and the bounds.** All 27
  `invalid="execution"` cases failing at evaluation after a clean parse and
  check; errors naming the sub-expression and offset; no panic anywhere in the
  suite; depth and step limits; and a model decode error surfacing as an error
  rather than as an empty collection.
  Satisfies `path.errors.001` through `path.errors.005`.

- [ ] **54. Implement the public evaluation surface.** Evaluate from text,
  evaluate a parsed expression repeatedly, evaluate-first with its no-match
  case, the unexpected-return-type error, and synchronous evaluation callable
  from inside a transaction.
  Satisfies `path.api.001` through `path.api.006`.

- [ ] **55. Land the path benchmarks and record the baseline.** The four
  `bench/micro/` path workloads plus the suite wall-clock, with their budgets
  and the first baseline.
  Satisfies `path.perf.001` through `path.perf.006`.

---

## Phase E — the round-trip oracle and closing out

- [ ] **56. Wire the `r4-example-roundtrip` oracle.** Decode, canonically
  re-encode and compare byte-for-byte for every example, with per-file
  reporting, per-resource-type coverage counts including types with no example,
  and XML inputs reported `not-applicable-xml` rather than `passed`. Run against
  the interim 103-example selector until task 59 lands the full package.
  Satisfies `codegen.roundtrip.001`, `codegen.roundtrip.002`,
  `codegen.roundtrip.003`.

- [ ] **57. Check every generated search-parameter expression against the
  parser.** All 1,375 expressions parse; a parameter with no expression recorded
  as having none rather than an empty one. This is the first real integration of
  `frevo-codegen`'s output with `frevo-path`, and the first place the two can
  disagree.
  Satisfies `codegen.searchparam.005`.

- [ ] **58. Run the full gate set and close the loop.** `cargo fmt --all
  --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
  nextest run --workspace`, `cargo deny check`, the FHIRPath suite against the
  HAPI rate, the round-trip oracle, and the `bench/micro/` budgets. Record
  every case Frevo fails that HAPI passes. Anything that genuinely cannot pass
  goes to `docs/known-gaps.md` with its exact case identifiers and stops for
  owner approval — it is not skipped, ignored or filtered out.

- [ ] **59. [chore] Pin the full R4 example package as an oracle.** On a
  `chore/oracles-*` branch: add `hl7.fhir.r4.examples#4.0.1` to
  `oracles/PINS.md` with its licence determination and digest, add it to
  `oracles/fetch.sh`, add the `r4-example-roundtrip` suite to `oracles/run.sh`'s
  suite list, and switch task 56's selector from the interim 103 examples to the
  full set. Owner checkpoint: it changes what the round-trip gate measures.

- [ ] **60. [chore] Record the UCUM notice.** On the same branch: add
  Regenstrief's UCUM copyright notice for `ucum-essence.xml` to `NOTICE`, and
  record in `oracles/PINS.md` that the file is fetched inside the Apache-2.0
  `fhir-test-cases` tree but carries its own licence terms.

- [ ] **61. Update `docs/inventory.md` and `docs/provenance.md`.** In
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

- [ ] **61a. Confirm the ten deviation entries against the implementation.**
  `DEV-0001` to `DEV-0010` in `docs/deviations.md` were written from the pinned
  Java source before any Rust existed. Re-run each one's reproducing expression
  through both engines, confirm the recorded HAPI behaviour is what HAPI
  actually does, fill in the `Reviewed` line, and raise all ten at the
  checkpoint. An entry whose recorded behaviour turns out to be wrong is
  corrected or deleted, not left standing — an inaccurate deviation is worse
  than none, because it licenses a difference nobody checked. `DEV-0010` needs
  `DEBT-0006` added to `docs/tech-debt.md`, which lives on `main` and so cannot
  be written from this branch; open that as part of the merge.

- [ ] **62. Update `tasks/graph.yaml` and finalise ADR 0004.** Set this change's
  `status` to `done`, so `add-server-skeleton` and `add-store-schema` unblock.
  Confirm ADR 0004 carries task 2's measured numbers and a recorded verdict, as
  its own closing line requires before this change is archived. Then archive.
