CHANGES_REQUESTED

Reviewed against `reference/org.hl7.fhir.core` at `73c432b5f43ddbce979978a760ec4aa81e929094`
(`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/`), `reference/hapi-fhir`
at `7ab4c7c7c49281496faa1b3c2498a652457669ae`, and `oracles/fhirpath/tests-fhir-r4.xml`.

Short path below: `ENGINE` = `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`.

The structure, the ID registry and the oracle-selector discipline are in good
shape. The findings are all cases where the delta states behaviour the pinned
R4 engine does not have, or omits a public surface with no out-of-scope note.

---

1. **The pinned suite has 103 groups and 1,191 cases, not 104 and 1,193 —
   `testDollarResource` is inside an XML comment.**
   `oracles/fhirpath/tests-fhir-r4.xml:1576-1582` wraps the whole
   `testDollarResource` group in `<!-- ... -->`, and its bodies are not even
   well-formed test elements (lines 1578-1579 are leftover Java:
   `testBoolean(patient(), patient().getManagingOrganization(), "Reference", ...`).
   An XML parse of the pinned file yields 103 `<group>` and 1,191 `<test>`
   elements; the textual counts 104/1,193 come from matching inside the comment.
   The `invalid` counts the proposal gives are right (7 syntax, 29 semantic,
   27 execution), so only the group/case totals are wrong.

   Three consequences, all gradeable:
   - `path.oracle.001` asserts "it reports 104 groups and 1,193 cases" **and**
     "a count that differs fails the run". As written the harness fails on the
     pinned suite on its first run. Same number repeated in `path.perf.005`,
     `proposal.md` and `spec.md`.
   - `path.oracle.014` and `proposal.md` risk 2 say `documentFunctions` is 114
     cases. It is **115** (`<group name="documentFunctions">`, counted by
     parse). 115/1,191, not 114/1,193.
   - `path.env.006` ("`testDollarResource`, first case") and
     `path.navigation.021` ("`testDollarResource`, second case") name a group
     that does not exist in the suite, so neither scenario has a grading
     selector. `%resource` and `descendants()` still need coverage — pick cases
     that exist (e.g. `testVariables/*` for `%`, and note that no live case
     exercises `descendants()`).

2. **`single()` on an *empty* collection throws; it does not yield empty.**
   `ENGINE:5087` — `funcSingle` returns the focus only when `focus.size() == 1`
   and otherwise `throw makeException(expr, I18nConstants.FHIRPATH_NO_COLLECTION,
   "single", focus.size())`. Size 0 takes the throw. `path.subset.003`
   ("`single` on an empty collection yields empty",
   `Patient.name.prefix.single()`) specifies the opposite. The suite's only
   negative case is `testSingle/testSingle2` (`Patient.name.single()`,
   `invalid="execution"`), so nothing in the oracle would catch this; it would
   surface as a differential diff on an invariant. Rewrite `path.subset.003` as
   an execution error, or drop it.

3. **An ordered comparison with a multi-item operand yields empty, not an
   error — and the four operators do not accept the same types.**
   `ENGINE:2409-2455` — `opLessThan` handles `size()==0` (empty),
   `size()==1 && size()==1` primitive, and `size()==1 && size()==1` Quantity;
   everything else falls through to `return new ArrayList<Base>()`. So
   `(1 | 2) < 3` is an **empty collection**. `path.compare.007` says evaluation
   fails. Two further mismatches in the same area:
   - `ENGINE:2418` gates the numeric branch of `opLessThan` on
     `l.hasType("integer") || l.hasType("decimal")`, while `ENGINE:2466`
     (`opGreater`) gates on `hasType("integer","decimal","unsignedInt","positiveInt")`.
     A `unsignedInt`/`positiveInt` operand therefore throws
     `FHIRPATH_CANT_COMPARE` under `<` and compares fine under `>`.
     `path.compare.001`-`004` all say the operators "compare the same types",
     which is not true. This is reachable from real FHIR data
     (`Media.height`, `Timing.repeat.count`, …).
   - The numeric comparison is `Double.valueOf(l.primitiveValue()) < Double.valueOf(r.primitiveValue())`
     — binary floating point. The Arithmetic requirement's preamble says the
     engine uses "decimal arithmetic rather than binary floating point"; that
     holds for `opPlus`/`opTimes` but not for the comparison operators, and an
     implementer who uses exact decimals here will differ from HAPI on
     high-precision operands. State it.

4. **Comparing incommensurable units does not fail, so three scenarios rest on
   a false premise.** `ENGINE:2300` — `qtyToCanonicalDecimal` builds a
   `Pair(value, code)`, asks UCUM for the canonical form, and returns the bare
   `DecimalType`; it performs **no** dimension check. `ENGINE:2443-2451` then
   calls `opLessThan` on the two decimals, so `1 'cm' < 1 's'` returns a
   boolean (`0.01 < 1` → `true`). With no UCUM service configured,
   `ENGINE:2444` returns `false` outright. Affected:
   - `path.quantity.015` ("comparing incommensurable units is an execution
     error") — wrong.
   - `path.logic.006` and `path.logic.007` both use
     `false and (1 'cm' < 1 's')` / `true or (...)` as the witness "whose right
     operand alone would fail". It does not fail, so neither scenario tests
     short-circuiting. Pick a right operand that genuinely errors — e.g. one
     that reaches `FHIRPATH_CANT_COMPARE` (`1 < 'a'`) — and keep the
     short-circuit check, because `preOperate` (`ENGINE:1542-1556`) really does
     short-circuit `And`, `Or` and `Implies`.
   No case in the suite uses `1 'cm' < 1 's'`; the only `cm`/`s` pairs are
   `Comparable/Comparable2`, `Comparable3` and `testMinus/testMinus6`.

5. **A string function on a non-string yields an empty collection, not an
   error.** `ENGINE:5787-5805` — `funcStartsWith` only appends a result when
   `focus.get(0).hasType(FHIR_TYPES_STRING) || doImplicitStringConversion`, and
   `FHIR_TYPES_STRING` (`ENGINE:2407`) is
   `{string, uri, code, oid, id, uuid, sid, markdown, base64Binary, canonical, url, xhtml}`
   — no `integer`, `decimal` or `boolean`. So `1.startsWith('1')` returns
   empty. `path.string.020` ("a string function on a non-string is an execution
   error", witness `1.startsWith('1')`) is wrong. The same guard appears in
   `funcSubstring` (`ENGINE:5877`), `funcUpper`/`funcLower`/`funcContains`
   (`ENGINE:4849`, `:4875`, `:4903`). Restate as "yields empty", and keep a
   separate scenario for the cases that really do throw.

6. **`conformsTo()` with no host service throws; it does not return false.**
   `ENGINE:5972-5974` — `if (hostServices == null) throw makeException(expr,
   I18nConstants.FHIRPATH_HO_HOST_SERVICES, "conformsTo")`.
   `path.utility.014`'s title says "with no host service returns false" while
   its body describes "the server-facing configuration, whose
   `conformsToProfile` returns `false` unconditionally" — two different engine
   states. The second is `FhirPathR4.setEvaluationContext`'s anonymous
   `IHostApplicationServices`
   (`hapi-fhir-structures-r4/.../FhirPathR4.java`, `conformsToProfile` returns
   `false`), which only exists **after** `setEvaluationContext` has been
   called. `design.md` §4.4 explicitly lists "none (unset — most oracle cases)"
   as one of the three host-service configurations this change ships, so the
   no-host-service error path is in scope and has no scenario. Split
   `path.utility.014` into the unset case (error, naming `conformsTo`) and the
   FhirPathR4 case (false).

7. **The R4 engine's `resolve()` has no Bundle `fullUrl` resolution.**
   `ENGINE:5413-5460` — `funcResolve` handles exactly two things: a reference
   starting `#`, looked up in `context.rootResource.getChildByName("contained")`,
   and otherwise `hostServices.resolveReference(...)` wrapped in
   `try { } catch (Exception e) { res = null; }`. There is no Bundle-entry
   lookup anywhere in the method. `path.utility.018` ("`resolve()` resolves a
   Bundle-internal reference by `fullUrl` … that entry's resource is returned")
   therefore specifies behaviour the pinned R4 engine does not have, and
   `proposal.md`'s out-of-scope row claims local resolution "is what the oracle
   exercises" (no live case calls `resolve()` at all — the only ones are in the
   commented-out `testDollarResource` and in `documentFunctions`). Either make
   Bundle resolution the host service's job, as R4 does, or cite a `DEV-nnnn`.
   Two related behaviours also need stating because they are easy to get wrong:
   `#id` is resolved against **`rootResource`**, not the focus resource; and a
   host-service exception is swallowed to empty, never surfaced.

8. **`now()` is not stable within one evaluation.** `ENGINE:4772` —
   `funcNow` does `result.add(DateTimeType.now())` on every call; nothing in
   `ExecutionContext` caches an evaluation instant. `path.datetime.003` asserts
   `now() = now()` is `true`; the pinned engine gives no such guarantee, and
   the suite has no such case (`testNow` is only `Patient.birthDate < now()`
   and `now().toString().length() > 10`). Either drop the scenario or state
   that caching is a deliberate difference with a `DEV-nnnn`. Related:
   `funcToday` (`ENGINE:4779`) is `new DateType(new Date(), DAY)` — the JVM
   default zone — which is exactly why `FHIRPathTests.test` calls
   `TimeZone.setDefault`; `path.datetime.004` should say that `today()` and
   `now()` both read the configured zone, not only that the harness fixes it.

9. **`toDate()` does not exist in the R4 engine.**
   `ExpressionNode.Function.fromCode` has no `"toDate"` case and the `Function`
   enum has no `ToDate` member (it has `ConvertsToDate` but not `ToDate`), so
   `'2015'.toDate()` is a **parse failure**. The Type-conversion requirement's
   preamble states "The engine SHALL implement each `toX` function and its
   matching `convertsToX` predicate", which is false for `toDate`, and
   `path.convert.004` covers only `convertsToDate`. Add a scenario for the
   asymmetry. `toTime()` and `toBoolean()` likewise have no scenario of their
   own — only their `convertsTo` partners (`path.convert.006`, `007`) — and
   both are implemented (`checkParamCount` cases at `ENGINE:1436`, `:1438`).

10. **Four public engine settings are neither specified nor declared out of
    scope.** The Strictness requirement says the engine "SHALL expose the
    strictness switches `FHIRPathEngine` exposes" but covers three
    (`doNotEnforceAsSingletonRule`, `doNotEnforceAsCaseSensitive`,
    `legacyMode`). Missing, all with public setters and all behaviourally
    observable:
    - `setAllowDoubleQuotes` (`ENGINE:6996`), threaded into the lexer at
      `ENGINE:334` and `:371` and read at `FHIRLexer.java:209`. With it unset —
      the default and what both HAPI wrappers use — `"foo"` is a syntax error.
      `path.lexer.*` has no double-quote scenario either way.
    - `setDoImplicitStringConversion` (`ENGINE:300`), which flips the outcome of
      every string function (`ENGINE:4849`, `:4875`, `:4903`, `:5799`, `:5877`)
      — i.e. it is the switch that decides finding 5.
    - `setEmitSQLonFHIRWarning` (`ENGINE:7004`), which gates
      `doSQLOnFHIRCheck` (`ENGINE:1641`) and so decides whether a choice
      element navigated without `ofType()` produces
      `FHIRPATH_CHOICE_NO_TYPE_SPECIFIER`. This is the path search-parameter
      and ViewDefinition expressions take.
    - `setLiquidMode` (`ENGINE:6985`).
    Each needs a scenario or a named later change.

11. **`path.strictness.006` states the opposite of what the R4 engine does.**
    It says "lenient polymorphic names are not supported in R4 … the R4
    engine's `allowPolymorphicNames` is left unset by both HAPI wrappers". The
    second half is right; the first is not. The R4 engine has the field
    (`ENGINE:177`), a public getter/setter (`ENGINE:6973-6979`) and honours it
    in both evaluation (`getChildrenByName`, `ENGINE:257`) and the check pass
    (`getElementDefinition(sdi, path, isAllowPolymorphicNames(), expr)`,
    `ENGINE:6473`). The only source for "not supported" is the suite's own
    comment at `tests-fhir-r4.xml:1603`, which guards two `modeTest` elements
    that are themselves commented out and therefore not in the 1,191.
    Respecify as: the flag exists, HAPI leaves it unset, and with it set
    `Observation.valueQuantity` resolves — or declare it out of scope and name
    the change.

12. **The check pass's *warning* channel is absent from `path.check.*`.**
    `check`/`checkOnTypes` take a `List<IssueMessage> warnings`
    (`ENGINE:477`, `:532`, `:539`) and the engine populates it rather than
    throwing, in `checkSingleton` (`ENGINE:3838`,
    `FHIRPATH_COLLECTION_STATUS_CONTEXT`),
    `checkCardinalityForComparabilitySame` / `checkCardinalityForSingle`
    (`ENGINE:1970`, `:1978`,
    `FHIRPATH_COLLECTION_STATUS_OPERATION_LEFT/RIGHT`) and
    `doSQLOnFHIRCheck` (`ENGINE:1641`). `path.check.001`-`014` only ever speaks
    of the check passing or failing. An implementer would either turn these
    into errors (breaking every expression that compares a list to a singleton)
    or drop them (losing the diagnostics the validator depends on). Add
    scenarios for the warning list and for at least one warning of each kind.

13. **Ordered-collection checking applies to six constructs, unconditionally.**
    `checkOrdered` (`ENGINE:3845`) throws `FHIRPATH_ORDERED_ONLY` whenever the
    focus's `CollectionStatus` is `UNORDERED`, and it is called for `item`
    (`ENGINE:3364`), `first` (`:3408`), `last` (`:3412`), `tail` (`:3416`),
    `skip` (`:3420`) and `take` (`:3425`). It is gated on no flag.
    `path.navigation.022` is the only scenario that touches it, covers `skip`
    only, and describes it as conditional on "strict mode with ordered function
    checking enabled" — that is the suite case's
    `mode="strict" checkOrderedFunctions="true"` attributes, which the pinned
    R4 harness never reads (`FHIRPathTests.test` reads only `inputfile`,
    `expression/@invalid`, `predicate` and `ordered`). `path.subset.*` says
    nothing about an unordered focus at all. Specify the rule once, for all six,
    unconditional.

14. **The `memberOf` binary operator has no scenario, and differs from the
    function.** `ExpressionNode.Operation.fromCode` maps `"memberOf"`, and
    `operate` dispatches it at `ENGINE:1855` to `opMemberOf` (`ENGINE:2602`),
    which iterates the **whole** left collection and ORs the results.
    `funcMemberOf` (`ENGINE:4786`) instead returns empty unless
    `focus.size() == 1 && nl.size() == 1`. `path.utility.015`/`016` cover only
    the function form. Both also fall back to
    `worker.fetchResource(ValueSet.class, url)` when `hostServices == null`
    rather than erroring, and both return **empty** (not false) for a focus
    whose type is not one of `code`/`string`/`uri`/`Coding`/`CodeableConcept` —
    neither behaviour is specified.

15. **`extension(url)` also returns modifier extensions.** `funcExtension`
    (`ENGINE:5462-5478`) calls `getChildrenByName(item, "extension", ext)` and
    then `getChildrenByName(item, "modifierExtension", ext)` before filtering
    by URL. `path.utility.009`-`011` describe only `extension`, and the
    `testExtension` group has no modifier-extension case, so the narrower
    implementation would ship green and diverge later (every profile invariant
    written with `extension(...)` over a modifier extension).

16. **Host-supplied custom functions and the constant-evaluation mode are
    neither specified nor deferred.** `IHostApplicationServices` declares
    `resolveFunction`, `checkFunction`, `executeFunction` and `paramIsType`;
    `Function.Custom` takes its arity from
    `details.getMinParameters()/getMaxParameters()` at `ENGINE:1463`, and
    `FHIRPathUtilityClasses.FunctionDetails` is the public descriptor.
    `design.md` §4.4's `HostServices` trait omits all four, and no scenario or
    out-of-scope row names them. Likewise `FHIRPathConstantEvaluationMode`'s
    four values — `EXPLICIT`, `NOVALUE`, `IMPLICIT_BEFORE`, `IMPLICIT_AFTER` —
    which `FhirPathR4.setEvaluationContext` maps one-for-one onto
    `IFhirPathEvaluationContext.ConstantEvaluationMode`: `path.env.012` covers
    a host service returning a collection but never the mode argument, and
    `design.md` has `mode: ConstantMode` in the signature with no scenario
    saying which mode is passed when. An implementer cannot tell from the delta
    when `NOVALUE` versus `EXPLICIT` is in force.

17. **The model delta decodes strictly where HAPI 8.12.0's server is lenient.**
    The FHIR-JSON-decoding requirement binds `frevo-model` to
    `org.hl7.fhir.r4.formats.JsonParser`. The starter does not parse request
    bodies with that class — it uses `ca.uhn.fhir.parser.JsonParser`, and
    `FhirContext.java:128` sets
    `private volatile IParserErrorHandler myParserErrorHandler = new LenientErrorHandler();`.
    In `LenientErrorHandler` (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/LenientErrorHandler.java`),
    `unknownElement`, `unknownAttribute`, `incorrectJsonType`,
    `unexpectedRepeatingElement` and `missingRequiredElement` each log a warning
    and return; the class javadoc (lines 27-44) states that **only**
    `invalidValue` and `extensionContainsValueAndNestedExtensions` throw
    `DataFormatException` by default. So HAPI 8.12.0 accepts, with a 2xx:
    - a `Patient` carrying `"nmae": []` — contradicts `model.json.004`;
    - `"active": "true"` for a `boolean` — contradicts `model.json.013`;
    - `"name": {"family": "x"}` for a `0..*` element — contradicts
      `model.json.014`;
    - `"birthDate": ["1974-12-25"]` for a `0..1` element — contradicts
      `model.json.015`;
    - an `Observation` with no `status` — contradicts `codegen.cardinality.002`
      ("reported as a missing required element rather than silently
      defaulted"). It *is* reported, to the log.
    These are HTTP-observable, so they are exactly the diffs the differential
    gate exists to catch, and `proposal.md` §Risks asserts "No deviations are
    asserted". Either specify HAPI's leniency (and name the switch that changes
    it, so a strict mode remains available to the validator) or record a
    `DEV-nnnn` per `CLAUDE.md` §1. The same question decides `model.json.016`
    (duplicate key) and `model.json.018` (`null` in a non-primitive array),
    neither of which maps to a `LenientErrorHandler` callback at all —
    say what HAPI does with each.

18. **`extensionContainsValueAndNestedExtensions` has no scenario.** It is one
    of only two conditions `LenientErrorHandler` treats as fatal by default: an
    `Extension` carrying both a `value[x]` and nested `extension` children
    raises `DataFormatException`. Nothing in `model.json.*`, `model.primext.*`
    or `codegen.choice.*` covers it (`codegen.choice.004` is two values for one
    choice element, a different case). Add it, with the status code the server
    returns.

19. **`path.oracle.009` keys the Quantity comparison off the wrong side, and
    would fail every Quantity case in the suite.** `FHIRPathTests.test` selects
    the quantity branch on the **outcome's** runtime type —
    `if (outcome.get(i) instanceof Quantity) { Quantity q = fp.parseQuantityString(v); ... equalsDeep(q) }`
    — not on the expected `<output type=...>`. Every Quantity-valued
    expectation in the pinned suite carries **no** `type` attribute:
    `LowBoundary/LowBoundaryQuantity` expects `1.58650000 'cm'` with
    `<output>` bare, and all three `Comparable` cases expect `true`/`false`
    with no type. `path.oracle.009` says "WHEN an expected output's type is
    `Quantity`", so a harness built to that scenario takes the primitive branch,
    hits the `outcome instanceof PrimitiveType` assertion, and fails cases HAPI
    passes. Restate as: branch on the result item's type; and note in
    `path.oracle.008` that an absent `type` attribute means "do not check the
    type" and an empty `<output>` body means "do not check the value", which is
    what `Utilities.noString(...)` does on both sides.

20. **The gate's treatment of cases HAPI structurally cannot pass is
    unspecified, and `path.check.002` depends on it.** The pinned R4 harness
    reads none of `mode`, `context`, `contextType` or `version`, and always
    calls `fp.check(res, "Resource", res.getResourceType(), res.getResourceType(), node)`.
    Several `isSemanticChecks` pairs are byte-identical expressions
    distinguished *only* by `context`/`contextType` —
    `isEffectOnLogic2Base` vs `isEffectOnLogic3Base`, `isGuardOr` vs
    `isGuardOrBad`, `isGuardXorBad` vs `isGuardXorBad2`, `isGuardOrInnerImplies`
    vs `isGuardOrInnerImpliesBad` — so HAPI necessarily fails one of each pair.
    `path.oracle.002` requires Frevo's harness to read all four attributes and
    `path.check.005`/`006`/`007` grade individual cases by them, which is
    correct and better than HAPI. But `path.oracle.011`/`012` only define the
    gate as "Frevo's passed count at least HAPI's" plus a list of cases HAPI
    passes and Frevo fails; nothing says how a case HAPI cannot pass is
    recorded, and `path.check.002` ("the 29 semantic-invalid cases **all** fail
    the check") is then stricter than the gate it is graded by. State the
    contract: which attributes Frevo honours that HAPI does not, and that
    `path.check.002` is graded against Frevo's context-aware run rather than
    against HAPI's.

21. **`path.strictness.002`/`005` need the harness's flag *ordering*, which is
    observable.** `ENGINE:211-217` — `initFlags()` already sets both
    `doNotEnforceAsCaseSensitive` and `doNotEnforceAsSingletonRule` to `true`
    for any non-R5 worker context, so `FhirPathR4`'s constructor is re-asserting
    the default rather than changing it (`path.strictness.001` is right, but for
    a different reason than it gives). More importantly,
    `FHIRPathTests.test` calls `setDoNotEnforceAsCaseSensitive(false)` and
    `setDoNotEnforceAsSingletonRule(false)` **after** `fp.check(...)` and
    before `fp.evaluate(...)`, on a `static` engine shared across all cases. So
    in the pinned harness the *first* case's check pass runs with both flags
    `true` and every subsequent case's check pass runs with both `false`.
    `path.strictness.002` says only "the oracle harness … enforced … before
    evaluating"; `path.strictness.005` hedges on what the engine returns. Pin
    the ordering (which flags are in force during check versus during evaluate)
    or `path.check.*` and `path.strictness.*` can be satisfied by two
    incompatible implementations.

---

Not findings, recorded so they are not re-derived: the `invalid` counts (7/29/27)
are correct; `JsonCreatorCanonical.writeObject` does sort property names with
`Collections.sort` and `JsonCanPresentedNumberValue` does preserve a decimal's
lexical form, so `model.canonical.001`-`003` are faithful; the three R4 resources
that derive from `Resource` rather than `DomainResource` are indeed `Bundle`,
`Binary` and `Parameters` (`codegen.types.004`); the 20 primitives listed in
`codegen.types.001` are exactly R4's; `opIs` returning `false` for a multi-item
focus and empty for an empty focus matches `path.types.012`; `funcNot` returning
empty for a null `Equality` matches `path.collection.005`; `funcTake`'s
`Math.min(size, n)` loop matches `path.subset.009`; `funcIif` throwing for a
focus of more than one and returning empty with no else-branch matches
`path.logic.010`; `1 div 0`, `1 mod 0` and `1 / 0` all yield empty
(`testDiv5`, `testMod5`, `testDivide6`) as `path.math.008` says; `filterIdType`
(`ENGINE:280`) does reduce `Patient.id` to the logical id as
`path.navigation.013` says; and `processConstantString` (`ENGINE:1789`) does
raise a lexer error for an unknown escape as `path.lexer.012` says.
