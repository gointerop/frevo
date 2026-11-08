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
  `oracles/fhirpath/tests-fhir-r4.xml` into cases: assert 104 groups and 1,193
  `<test>` elements, and read every attribute a case carries — `name`,
  `description`, `inputfile`, `expression` plus its `invalid` value,
  `predicate`, `ordered`, `mode`, `context`, `contextType`, `version`,
  `testing`, `checkOrderedFunctions` — failing on an attribute the harness does
  not understand. Load each of the 15 referenced input files from
  `oracles/fhir-test-cases/fetched/r4/` (the directory
  `TestingUtilities.loadTestResource("r4", input)` resolves to), both the XML
  and the JSON ones, through a **test-only** reader. Implement the three-stage
  run — parse, check, evaluate — with failures attributed to the right stage,
  the `predicate="true"` boolean conversion, the `ordered="false"` unordered
  comparison, the count/type/value comparison and the `Quantity` deep
  comparison, and a per-case status of `passed`, `failed`, `did-not-run` or
  `skipped-for-licence`. Fix the evaluation time zone to `UTC+1100`, as
  `FHIRPathTests.test` does. The harness runs against a trait object, so it
  works before any engine exists.
  Satisfies `path.oracle.001`, `path.oracle.002`, `path.oracle.003`,
  `path.oracle.004`, `path.oracle.005`, `path.oracle.006`, `path.oracle.007`,
  `path.oracle.008`, `path.oracle.009`, `path.oracle.010`, `path.oracle.015`,
  `path.datetime.004`.

- [ ] **2. Measure `helios-fhirpath` and `haste-fhirpath` through the harness and
  record the numbers in ADR 0004.** Create `harness/fhirpath-compare/`, listed
  in `[workspace] exclude` so `haste-fhirpath`'s wildcard dependencies never
  enter the product graph or the `cargo deny` gate. Run all 1,193 cases through
  `helios-fhirpath` 0.2.7 and `haste-fhirpath` 0.129.3, record per-case pass or
  fail and the wall-clock of a full run, and establish whether
  `IntoEvaluationResult` can be implemented over a lazy, span-based node without
  materialising whole resources. Write the measured pass rates, the throughput
  and that finding into ADR 0004's "What would change this" section, and state
  the resulting verdict: keep decision 1, or reverse it. **Stop for the owner
  here** — this is the checkpoint the ADR asks for, and reversing it changes the
  scope of every task below.
  Satisfies `path.oracle.013`.

- [ ] **3. Measure HAPI 8.12.0's own pass rate on the same 1,193 cases and wire
  it into the gate.** Run `org.hl7.fhir.r4.test.FHIRPathTests` at pin
  `73c432b5` against the same suite file, record each case's status, and load
  that record into the harness so every case reports Frevo's status beside
  HAPI's. Implement the gate as "Frevo's `passed` count at least HAPI's", with
  every case HAPI passes and Frevo fails listed individually regardless of the
  aggregate. Confirm and record that all 114 `documentFunctions` cases fail for
  HAPI too, with the reason that the R4 `ExpressionNode.Function` enum
  implements none of the seven functions.
  Satisfies `path.oracle.011`, `path.oracle.012`, `path.oracle.014`.

- [ ] **4. Write the failing tests for every `path.*` scenario.** One test per
  scenario ID, each carrying a `// covers: path.<area>.<nnn>` comment, with the
  oracle-backed scenarios driven from the harness's case selectors and the rest
  written directly. All 329 must fail, and none by `todo!()`, `#[ignore]` or a
  bare `#[should_panic]`.
  Covers all 329 `path.*` scenarios.

- [ ] **5. Write the failing tests for every `codegen.*` and `model.*`
  scenario.** Same discipline: one test per scenario ID with a `covers:`
  comment, 48 plus 77 of them, including the `bench/micro/` workloads as
  benchmarks with assertions on their budgets. Interim round-trip selector is
  the 103 examples in `oracles/fhir-test-cases/fetched/r4/examples/`, with the
  test reporting that it is the interim set.
  Covers all 48 `codegen.*` and 77 `model.*` scenarios.

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
  three error paths, unknown properties with a path, choice decoding from the
  typed property name, contained resources, Bundle entries of mixed types, and
  element `id` and `extension` on any element.
  Satisfies `model.json.001` through `model.json.007`, `model.json.020`.

- [ ] **20. Implement JSON decoding: primitive values.** Opaque `xhtml`,
  lexical-form decimals with no binary-float path, `integer`, `positiveInt` and
  `unsignedInt` bounds, `base64Binary` carried as its encoded text.
  Satisfies `model.json.008`, `model.json.009`, `model.json.010`,
  `model.json.011`, `model.json.012`, `model.json.017`.

- [ ] **21. Implement JSON decoding: rejection paths.** Type mismatch with both
  types named, array-for-scalar and scalar-for-array, duplicate keys as an error
  rather than last-wins, `null` in a non-primitive array, non-UTF-8 input.
  Satisfies `model.json.013`, `model.json.014`, `model.json.015`,
  `model.json.016`, `model.json.018`, `model.json.019`.

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
  Satisfies `model.nav.001` through `model.nav.011`.

- [ ] **26. Implement decode error reporting.** Element path and byte offset on
  every error; first error in document order; no panic on any truncation prefix;
  a depth limit instead of a stack overflow.
  Satisfies `model.errors.001` through `model.errors.005`.

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
  form; `single`, `first`, `last`, `tail`, `skip`, `take`, `intersect`,
  `exclude`.
  Satisfies `path.filter.001` through `path.filter.009`, `path.subset.001`
  through `path.subset.012`.

- [ ] **34. Implement combining and membership.** The union operator, `union`,
  `combine`, `in`, `contains`, and the empty-operand rule.
  Satisfies `path.combine.001` through `path.combine.006`.

- [ ] **35. Implement equality, equivalence and ordered comparison.** Equality
  and inequality with their empty-collection result; equivalence and
  non-equivalence always returning a boolean; collection comparison in order;
  decimal comparison by value; deep comparison of complex elements; the
  differing-precision date rules; `hasValue`; the four ordered operators with
  their type, arity and precision failures; and the two `Period` invariant forms.
  Satisfies `path.equality.001` through `path.equality.012`, `path.compare.001`
  through `path.compare.010`.

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
  and the offset comparison case.
  Satisfies `path.math.012`, `path.math.013`, `path.math.014`,
  `path.datetime.001`, `path.datetime.002`, `path.datetime.003`,
  `path.datetime.005`, `path.datetime.006`.

- [ ] **39. Implement math functions.** `abs`, `ceiling`, `floor`, `round`,
  `sqrt`, `exp`, `ln`, `log`, `power`, `truncate`, with the empty-focus and
  multi-item rules.
  Satisfies `path.mathfn.001` through `path.mathfn.011`.

- [ ] **40. Implement string functions: the base set.** `upper`, `lower`,
  `toChars`, `indexOf`, `substring` with its out-of-range rule, `startsWith`,
  `endsWith`, `contains`, `length`, and the non-string and empty-focus rules,
  counting unicode scalar values rather than bytes.
  Satisfies `path.string.001` through `path.string.007`, `path.string.012`,
  `path.string.020`, `path.string.021`, `path.string.022`.

- [ ] **41. Implement string functions: regular expressions and the R3
  additions.** `matches`, `matchesFull`, `replaceMatches`, `replace`, `trim`,
  `split` with its empty segments, `join`, `encode` and `decode` for base64,
  urlbase64 and hex, `escape` and `unescape` for html and json.
  Satisfies `path.string.008` through `path.string.011`, `path.string.013`
  through `path.string.019`.

- [ ] **42. Implement type conversion.** Each `toX` and its `convertsToX`,
  including every date, dateTime and time precision string, the boolean
  spellings, quantities, the failed-conversion asymmetry, the multi-item error,
  and precision-preserving widening.
  Satisfies `path.convert.001` through `path.convert.011`.

- [ ] **43. Implement the UCUM unit algebra.** Canonical-form conversion from the
  pinned `ucum-essence.xml`, commensurability, multiplication and division of
  units, and the reported-version and missing-file failure paths.
  Satisfies `path.quantity.016`.

- [ ] **44. Implement quantity semantics on top of it.** Equality after
  conversion, equivalence rounding to the lower precision, calendar-duration
  commensurability, magnitude comparison, unit arithmetic, the dimensionless
  unit, the unitless and unit-text cases from
  `parameters-example-quantities.xml` and `parameters-quantity-unit-text.xml`,
  `comparable()`, the incommensurable-comparison error, and the unknown-unit
  behaviour.
  Satisfies `path.quantity.001` through `path.quantity.015`,
  `path.quantity.017`.

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
  returning a collection, and `defineVariable` with its scoping.
  Satisfies `path.env.001` through `path.env.014`.

- [ ] **48. Implement the utility functions.** `trace` with its retrievable log,
  `check`, `aggregate` with and without an initial value, `sort` with no
  argument, one key and several keys including descending, and `extension(url)`
  including on a primitive.
  Satisfies `path.utility.001` through `path.utility.011`.

- [ ] **49. Implement the host-service functions.** `conformsTo` against the
  harness's service, its unknown-profile error and `FhirPathR4`'s
  always-false default; `memberOf` with and without a resolvable value set;
  `resolve()` for `#id` into `contained`, for a Bundle `fullUrl`, for an
  unresolvable reference, and delegated to the host service.
  Satisfies `path.utility.012` through `path.utility.021`.

- [ ] **50. Implement the static type-check pass.** Separate from evaluation;
  all 29 `invalid="semantic"` cases failing it with the offending path named;
  non-existent elements; choice-element type inference; and the pass being
  skippable as `FhirPathR4` skips it.
  Satisfies `path.check.001`, `path.check.002`, `path.check.003`,
  `path.check.004`, `path.check.013`, `path.check.014`.

- [ ] **51. Implement type-guard narrowing in the check pass.** The `is` operator
  and `is()` function narrowing the guarded operand under `implies`, `and`, `or`
  and `iif`; the unreachable-operand rule when the narrowed set is empty; and a
  multi-type `contextType`. This is the `isSemanticChecks` group.
  Satisfies `path.check.005` through `path.check.012`.

- [ ] **52. Implement the strictness configuration.** The server-facing default
  matching `FhirPathR4`; the strict configuration the harness selects; the
  as-singleton rule enforced and not; case-sensitive `as`; lenient polymorphic
  names behaving strictly in R4; legacy mode off.
  Satisfies `path.strictness.001` through `path.strictness.007`.

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

- [ ] **62. Update `tasks/graph.yaml` and finalise ADR 0004.** Set this change's
  `status` to `done`, so `add-server-skeleton` and `add-store-schema` unblock.
  Confirm ADR 0004 carries task 2's measured numbers and a recorded verdict, as
  its own closing line requires before this change is archived. Then archive.
