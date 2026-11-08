CHANGES_REQUESTED

Second review of `add-fhirpath-and-codegen-r4`. All 21 findings from the first
review are **resolved**; six new ones replace them, two of which block.

Reviewed against `reference/org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094`
(`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/`, read from
`C:/gointerop/git/frevo/reference/`), `reference/hapi-fhir` at
`7ab4c7c7c49281496faa1b3c2498a652457669ae`, and
`oracles/fhirpath/tests-fhir-r4.xml` at
`c95adbee099c875de97d25ed7616aae7d66dd85b`. Unqualified `:NNN` line references
below are `FHIRPathEngine.java`.

## What I verified, and how

I re-derived the behaviour from the pinned source before reading the revised
artifacts, and treated the spec delta's claims as assertions to be checked
rather than as evidence.

- **Suite inventory**, by XML parse of `oracles/fhirpath/tests-fhir-r4.xml`:
  103 `<group>`, 1,191 `<test>`, `documentFunctions` = 115, no
  `testDollarResource` group, `invalid` counts 7 syntax / 29 semantic /
  27 execution. I also enumerated every attribute the file actually carries
  (see finding 2).
- **The R4 engine**: `FHIRPathEngine.java` (7,008 lines),
  `ExpressionNode.java`, `FHIRLexer.java`, `IHostApplicationServices.java`,
  `org.hl7.fhir.utilities/.../FHIRPathConstantEvaluationMode.java`. I walked the
  whole `Function` enum (`ExpressionNode.java:48-63`) and the whole `Operation`
  enum (`:272-273`) against the delta: every function and every operator now has
  a scenario or a named out-of-scope row. I also listed every public setter on
  the engine and ticked them off.
- **The pinned harness**,
  `org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java`, for
  everything the delta claims about grading.
- **HAPI's parser error surface**: `IParserErrorHandler` (10 callbacks),
  `LenientErrorHandler`, `StrictErrorHandler`, `ParserState`, `BaseParser`,
  `FhirContext`.
- **Codegen input counts**, by parsing the pinned HAPI copy:
  `profiles-types.xml` 63 SDs of which 20 `primitive-type`;
  `profiles-resources.xml` 149 SDs = 146 concrete + 2 abstract + 1 logical;
  `profiles-others.xml` 44; `sp/search-parameters.json` 1,375 SearchParameters.
  Every number `codegen.inputs.006`, `codegen.types.001/005` and
  `codegen.searchparam.001` assert is correct.

### No scenario was deleted, renumbered or repurposed

I extracted the scenario-ID set from commit `3c3f092` (the state my first review
graded) and from the current working tree:

- old 454 IDs, new 486 IDs;
- `comm -23` of the two sets is **empty** — nothing vanished;
- 32 IDs added, all at the end of their own area:
  `codegen.cardinality.005`, `model.json.021-023`, `path.check.015-019`,
  `path.compare.011-012`, `path.convert.012-014`, `path.datetime.007`,
  `path.env.015`, `path.lexer.015-016`, `path.oracle.016`, `path.quantity.018`,
  `path.strictness.008-010`, `path.string.023-024`, `path.subset.013`,
  `path.utility.022-027`;
- the 14 IDs `spec.md:33-38` says were rewritten in place still carry a
  behaviour of the same shape (same function, same operator, same parse mode) —
  they now state what the engine does rather than what the specification says,
  which is the correct resolution;
- every ID matches `^[a-z]+\.[a-z]+\.[0-9]{3}$`, all 486 are unique across the
  four files, and every area is contiguous from `001` with no gaps;
- the per-area table at `spec.md:40-86` and the totals at `spec.md:8-13`
  (49 / 80 / 357 / 486) match the files exactly.

Deviation hygiene: the spec delta references `DEV-0001` … `DEV-0009` and all
nine exist in `docs/deviations.md` with `Reviewed: 2026-11-08, owner`, a `Debt:`
field, and a `Scenario IDs:` line naming a scenario that exists. `DEBT-0001` …
`DEBT-0005` exist in `docs/tech-debt.md` on `main`. The sixteen scenarios
`spec.md:107-112` lists as having no live case are exactly the sixteen whose own
text says `differential-only` — I checked both directions.

---

## Disposition of the 21 standing findings

| # | Prior finding | Disposition | Evidence I checked |
|---|---|---|---|
| 1 | Suite is 103/1,191, not 104/1,193; `documentFunctions` is 115 | **addressed** | XML parse gives 103/1,191/115; `testDollarResource` absent. `proposal.md` "The suite's size", `spec.md:100-105` and `path.oracle.001` all state the XML-parse figures and name the commented block at `tests-fhir-r4.xml:1576-1582`. `path.env.006` re-pointed to the six live `isSemanticChecks` `%resource.status` cases and `path.navigation.021` to six live `descendants()` cases — both selectors exist in the file |
| 2 | `single()` on empty throws | **addressed** | `path.subset.003` now says evaluation fails. `:5087-5092`: `funcSingle` returns the focus only when `focus.size() == 1`, else throws `FHIRPATH_NO_COLLECTION`. Cites `DEV-0001` |
| 3 | Multi-item ordered comparison yields empty, not an error | **addressed** | `path.compare.007`. `:2410-2454`: `opLessThan` handles empty operands and the two singleton shapes, then falls through to `return new ArrayList<Base>()` at `:2454`. Cites `DEV-0002` |
| 4 | Incommensurable units do not fail | **addressed** | `path.quantity.015` rewritten: boolean with UCUM loaded, `false` with none. `:2443-2451` compares canonical decimals with no dimension check; `:2443-2444` returns `makeBoolean(false)` when `worker.getUcumService() == null`; `qtyToCanonicalDecimal` at `:2301-2312` returns a bare decimal. Cites `DEV-0005`. New `path.quantity.018` adds the non-UCUM-`system` case (`:2302-2303` returns `null`) |
| 5 | String function on a non-string yields empty | **addressed** | `path.string.020` plus new `path.string.024`. `FHIR_TYPES_STRING` at `:2407` lists exactly the 12 types the delta lists; the guard shape is at `:4849` (`hasType(FHIR_TYPES_STRING) \|\| doImplicitStringConversion`). Cites `DEV-0006`. New `path.string.023` draws the line against the functions that *do* raise |
| 6 | `conformsTo()` with no host service throws | **addressed** | New `path.utility.022`. `:5972-5975`: `funcConformsTo` throws `FHIRPATH_HO_HOST_SERVICES` before looking at its argument; `:5977-5979` returns `false` for a non-singleton focus once a service is set. `path.utility.014` now scopes itself to the configured-but-always-false server host service |
| 7 | R4 `resolve()` has no Bundle `fullUrl` resolution | **addressed** | `path.utility.018` rewritten. `funcResolve` at `:5413-5453`: `#`-prefixed only, against `context.rootResource.getChildByName("contained")` (`:5437-5447`), everything else to `hostServices.resolveReference` (`:5448-5453`). `path.utility.017` correctly states the **root**-resource lookup, the primitive focus (`:5430-5432`) and `Reference.identifier` (`:5426-5429`); `path.utility.020` correctly states the swallowed host error (`:5451-5453`) |
| 8 | `now()` is not stable within one evaluation | **addressed** | `path.datetime.003`. `funcNow` at `:4772-4776` is `result.add(DateTimeType.now())` per call, nothing caches an instant. Cites `DEV-0008` |
| 9 | `toDate()` does not exist | **addressed** | `path.convert.012` says it is a parse failure. No `"toDate"` string in `ExpressionNode.java` or `FHIRPathEngine.java`; the enum has `ConvertsToDate` but no `ToDate` (`ExpressionNode.java:53`, `:144`). Cites `DEV-0009`. New `path.convert.013/014` cover `toTime`/`toBoolean`, which do exist |
| 10 | Four public engine settings unspecified | **addressed** | The complete setter set is `hostServices` (`:227`), `location` (`:236`), `legacyMode` (`:291`), `doImplicitStringConversion` (`:300`), `doNotEnforceAsSingletonRule` (`:308`), `doNotEnforceAsCaseSensitive` (`:316`), `allowPolymorphicNames` (`:6977`), `liquidMode` (`:6985`), `allowDoubleQuotes` (`:6996`), `emitSQLonFHIRWarning` (`:7004`). Now covered by `path.api.007`, `path.strictness.007`, `path.string.024`+`path.strictness.009`, `path.strictness.001/003/004/005`, `path.strictness.006`, `path.strictness.010`, `path.lexer.015/016`, `path.check.017`+`path.strictness.008`. Only `setLocation` has no scenario — see the note below |
| 11 | `path.strictness.006` stated the opposite of the engine | **addressed, now correct** | The flag is a plain field defaulting `false` (`:177`), read in `getChildrenByName` during evaluation (`:257`) **and** passed to `getElementDefinition` during the type pass (`:6473`), so the scenario's "read both when resolving children and when resolving an element definition" is right. The suite's "not supported in R4" comment does sit on commented-out `modeTest` elements |
| 12 | Check pass's warning channel absent | **addressed** | `path.check.015-019` added. `typeWarnings` at `:183`; the `checkOnTypes(..., List<IssueMessage> warnings)` overloads at `:477`, `:532`, `:539` each `typeWarnings.clear()` then `warnings.addAll(typeWarnings)` (`:478`/`:528`), which is exactly `path.check.019`'s no-leak clause. Every warning id the delta describes exists: `FHIRPATH_CHOICE_NO_TYPE_SPECIFIER`/`_SPURIOUS_TYPE_SPECIFIER` (`:1646`/`:1649`, gated on `emitSQLonFHIRWarning` at `:1642`), `COLLECTION_STATUS_OPERATION_LEFT`/`RIGHT` (`:1972-1983`), `FHIRPATH_NOT_A_COLLECTION` (`:3302`), `FHIRPATH_UNKNOWN_EXTENSION` (`:3579`/`:3588`), `FHIRPATH_OFTYPE_IMPOSSIBLE` (`:3783`), `COLLECTION_STATUS_PARAMETER` (`:3834`), `COLLECTION_STATUS_CONTEXT` (`:3841`). The plain `check(...)` overloads (`:405`, `:421`, `:557`, `:596`, `:625`, `:631`) take no warning list, as `path.check.019` says. See finding 6 for the oracle |
| 13 | Ordered-collection checking covers six constructs, unconditionally | **addressed** | `path.subset.013` names exactly the six and says no setting disables it. Call sites: `item` `:3364`, `first` `:3408`, `last` `:3412`, `tail` `:3416`, `skip` `:3420`, `take` `:3425`; `checkOrdered` at `:3845-3847` throws `FHIRPATH_ORDERED_ONLY` with no flag in the condition. `path.navigation.022` correctly notes `testDollarOrderNotAllowed` carries `mode="strict" checkOrderedFunctions="true"` (`tests-fhir-r4.xml:213`) and that no R4 harness reads them |
| 14 | `memberOf` binary operator has no scenario and differs from the function | **addressed** | `path.utility.024/025/026` added and all three are right. `opMemberOf` `:2602-2628` loops the whole left collection and ORs, `makeBoolean(false)` when the value set is unresolved; `funcMemberOf` `:4786-4807` returns empty unless both focus and argument are singletons, empty when `vs == null` (`:4794-4796`), empty for an unhandled focus type (`:4804-4806`), `withGuessSystem()` for `code`/`string`/`uri` (`:4799`) and plain options for `Coding`/`CodeableConcept` (`:4801`/`:4803`); both forms fall back to `worker.fetchResource(ValueSet.class, url)` when `hostServices == null` (`:2605`, `:4793`). `Operation.MemberOf` exists (`ExpressionNode.java:273`, `:326`) and types as singleton boolean (`:2095`) |
| 15 | `extension(url)` also returns modifier extensions | **addressed** | `path.utility.023`. `funcExtension` `:5464-5482` collects `extension` then `modifierExtension` before filtering by URL. Cites `DEV-0007` |
| 16 | Host custom functions and the constant-evaluation mode unspecified | **partly addressed** | `path.utility.027` is correct: `case Custom` at `:4020-4022` consults `hostServices.paramIsType` per parameter, arity comes from `resolveFunction`'s `FunctionDetails`, the type from `checkFunction`, the value from `executeFunction`. `path.env.015` names four modes and there are exactly four (`FHIRPathConstantEvaluationMode`), but it mis-describes one of them and omits the behaviour that makes them observable — **new finding 4** |
| 17 | Model delta decoded strictly where HAPI's server is lenient | **addressed** | `FhirContext.java:128` is `new LenientErrorHandler()`, used for `newJsonParser()` at `:946`. `model.json.021` makes lenient the default and strict a mode, and `model.json.004/.013/.014/.015/.016/.018` plus `codegen.cardinality.002/.004/.005` each state the lenient outcome and the strict outcome. Each matches its `LenientErrorHandler` method (`unknownElement`, `incorrectJsonType` `:79-94`, `unexpectedRepeatingElement` `:177-185`, `missingRequiredElement` `:130-136`) and its `StrictErrorHandler` counterpart (`:43`-`:91`) |
| 18 | `extensionContainsValueAndNestedExtensions` had no scenario | **addressed** | `model.json.022`, plus `model.json.023` for the other fatal case. `LenientErrorHandler.java:52-53` sets `myErrorOnInvalidValue = true` and `myErrorOnInvalidExtension = true`; `:97-105` shows the blank-value exemption that `model.json.023` and `codegen.cardinality.005` both rely on; the class javadoc at `:28-45` says these are the only two |
| 19 | `path.oracle.009` keyed the Quantity comparison off the wrong side | **addressed** | Now keyed off the result item's runtime type. `FHIRPathTests.java:291-296`: `if (outcome.get(i) instanceof Quantity) { Quantity q = fp.parseQuantityString(v); assertTrue(outcome.get(i).equalsDeep(q)); }`. The non-Quantity branch asserts `instanceof PrimitiveType` and fails otherwise, which `path.oracle.009`'s last clause now states |
| 20 | Gate treatment of cases HAPI structurally cannot pass | **addressed** | New `path.oracle.016` plus the `hapi-cannot-pass` status in `path.oracle.011` and `ahead-of-hapi` in `path.oracle.012`. `FHIRPathTests.java:218-222` reads neither `context` nor `contextType` and always calls `fp.check(res, "Resource", res.getResourceType(), res.getResourceType(), node)`, so the premise holds, and the four named pairs do differ only by those attributes |
| 21 | `path.strictness.002/005` needed the flag ordering | **addressed** | `path.strictness.002` is exactly right. `FHIRPathTests.java:220/222` calls `check(...)`; `:234-235` then calls `setDoNotEnforceAsCaseSensitive(false)` and `setDoNotEnforceAsSingletonRule(false)`; `:239` evaluates. `fp` is `private static` (`:119`), so the "only the first case's check pass sees the initial `true` values" clause is correct too. `path.strictness.001` correctly attributes the defaults to `initFlags()` (`:211-216`, `!isR5Plus`) |

Also checked and found correct, beyond the standing findings:
`path.compare.001-004` and `DEV-0003` against `:2418` versus `:2465`/`:2513`/
`:2563` (`<` omits `unsignedInt`/`positiveInt`, the three mirrors include them);
`path.datetime.007` against `DateTimeUtil.getDatePrecision` (strips separators
and returns the length, so `today()` gives 8) and `funcPrecision` `:4378-4394`;
`path.lexer.015/016` against `FHIRLexer.java:209` and `FHIRPathEngine.java:334`;
`path.oracle.014` against the R4 `Function` enum, which has no `section`,
`entry`, `entryIn`, `resources`, `resourcesIn`, `sameSubject` or `hasCoding`;
and the whole `Function` and `Operation` enums for uncovered members — none.

---

## Findings

### 1. Three of `IParserErrorHandler`'s ten callbacks have no scenario and no out-of-scope note (blocking)

`specs/model/spec.md:96-100` says `frevo-model` "SHALL decode the R4 JSON
representation as the parser the starter actually uses does … and SHALL offer the
two parse modes HAPI offers". `IParserErrorHandler` has exactly ten callbacks;
seven are now covered (finding 17 above). Three are not, in either mode:

- **`containedResourceWithNoId`.** `ParserState.java:399-415` and `:437-452`: a
  contained resource with no `id` is still appended to the parent's `contained`
  list, but is **not** put into the id→resource map, so the later
  `weaveContainedResources` pass cannot find it. Lenient logs
  (`LenientErrorHandler.java:73-77`); strict throws `DataFormatException`
  `Msg.code(1819)` (`StrictErrorHandler.java:38-40`). Nothing in the delta says
  a contained resource without an `id` survives a lenient decode, and
  `model.json.006` covers only the *with*-`id` case.
- **`unknownReference`.** `ParserState.java:1277-1293`
  (`weaveContainedResources`): a `#id` reference whose target is not among
  `contained` calls `myErrorHandler.unknownReference(null, ref)`. Lenient logs;
  strict throws `Msg.code(1826)` (`StrictErrorHandler.java:94-97`). This
  interacts directly with `path.utility.017`, which specifies the resolve-side
  behaviour of the same dangling reference — the decode-side diagnostic is
  unspecified.
- **`invalidInternalReference`.** `BaseParser.java:245-250`, on the **encode**
  path (`determineReferenceText`): serialising a resource whose reference starts
  with `#` but matches no contained resource calls it. Lenient logs; strict
  throws `Msg.code(2724)` (`StrictErrorHandler.java:100-105`). The canonical
  serialisation requirement (`specs/model/spec.md:355-359`) has no scenario for
  it, and it is the only error condition on the encode side.

The text must either add a scenario for each — lenient diagnostic plus strict
error, with the path and the message's identifying content, in the same shape as
`model.json.015` — or add an explicit out-of-scope row naming the change that
covers it. Leaving them unstated means strict mode ships three error conditions
short of HAPI's and lenient mode three diagnostics short, and the round-trip
oracle cannot catch it because the R4 example package contains no dangling `#`
reference.

### 2. `path.oracle.002`'s attribute inventory is incomplete, and the scenario makes that fatal (blocking)

`specs/path/spec.md:2289-2296` lists thirteen attributes and then says "an
attribute it does not understand fails the run rather than being ignored". An
exhaustive attribute enumeration of the pinned file gives:

| Element | Attributes present |
|---|---|
| `tests` (root) | `name`, `description`, **`reference`** |
| `group` | `name`, `description`, `mode`, **`reference`** |
| `test` | `name`, `description`, `inputfile`, `mode`, `context`, `contextType`, `version`, `testing`, `predicate`, `ordered`, `checkOrderedFunctions` |
| `expression` | `invalid`, **`mode`** |
| `output` | `type` |

Three are unaccounted for. The one that bites hardest is **`mode` on
`<expression>`**: `tests-fhir-r4.xml:625` is
`<expression mode="strict" invalid="semantic">iif('non boolean criteria', …)</expression>`
in `testIif/testIif6` — the only expression-level `mode` in the file, and one of
the 29 cases `path.check.002` requires to fail the check. `reference` appears on
the root `<tests>` element and on `<group name="from-Zulip">`
(`tests-fhir-r4.xml:1583`). A harness built exactly to this scenario fails the
run on the pinned suite, which is the opposite of what the scenario is for.

The text must list the attributes per element, including `<expression mode>` and
`reference`, and say what the harness does with the expression-level `mode` given
that `path.check.002` grades `testIif6`. For what it is worth the outcome does
not change — `:3441-3444` runs `checkParamTypes` against a singleton boolean
unconditionally, so `testIif6` fails the check whatever `mode` says — but the
scenario has to say so rather than leave the attribute unhandled.

### 3. The oracle's expected text for a temporal result carries an `@` prefix, and neither `path.oracle.008` nor `.007` says so

`path.oracle.008` (`specs/path/spec.md:2340-2342`) says the body "must equal the
result item's FHIRPath string value". That value is `fpValue()`, and for temporal
types it is **not** the primitive value:

- `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/BaseDateTimeType.java:1070-1072`
  — `fpValue()` returns `"@" + primitiveValue()`;
- `TimeType.java:148-150` — returns `"@T" + primitiveValue()`;
- `PrimitiveType.java:259-261` — returns `primitiveValue()` for everything else.

That is why the suite's expectations read
`<output type="date">@1974-12-25</output>` (`tests-fhir-r4.xml:59`,
`:1280-1283`). A harness that compares the plain value fails every `date`,
`dateTime`, `instant` and `time` case in the suite, and the obvious "fix" is to
strip the `@` from the expectation — which is weakening an oracle, and rule 3
forbids it.

`path.oracle.007` is under-specified in the same place: the `ordered="false"`
branch at `FHIRPathTests.java:263-281` uses `fp.convertToString(...)` for a
`Quantity` and `((PrimitiveType) …).asStringValue()` — **not** `fpValue()`, so no
`@` prefix — for everything else, and matches each outcome against any expected
element where `(type absent OR type equal) AND (body absent OR body equal)`. The
only `ordered="false"` case in the file (`:321`, `testExpressions`) yields
strings, so the divergence is latent today and surfaces the first time the suite
is re-pinned.

The text must state the `@`/`@T` prefix for temporal results in
`path.oracle.008`, and state in `path.oracle.007` that the unordered branch
compares the plain string value and matches on type-and-value per expected
element.

### 4. `path.env.015` mis-describes `NOVALUE` and omits the implicit-constant override, which can shadow real navigation

`specs/path/spec.md:1693-1706`. Two problems.

**The mode description is wrong.** It says "the no-value mode is in force during
the check pass, where only the type is wanted". `NOVALUE` is used at exactly one
site, `:1585-1586`, and the condition is `atEntry && focus == null` — the type
pass with **no focus type at all**, which is what `fp.check(null, null, null,
node)` produces for the suite's 90 no-`inputfile` cases. An explicit `%name` in
the check pass goes through `EXPLICIT` instead (`:1610`). The enum's own
documentation agrees: `NOVALUE` is "The FHIRPathEngine was invoked with no focus
provided", not "resolved for its type".

**The observable behaviour is missing.** `IMPLICIT_BEFORE` and `IMPLICIT_AFTER`
are not "before or after the expression it appears in":

- `:3183-3191` — at the entry of `execute`, if
  `context.appInfo != null && hostServices != null`, the engine asks
  `resolveConstant(..., IMPLICIT_BEFORE)` for the name, and **if the answer is
  non-empty it returns that answer and never navigates the element**. A host
  constant therefore shadows a real property of the resource.
- `:3210-3214` — `IMPLICIT_AFTER` fires only after navigation produced an empty
  result.
- Both are gated on `context.appInfo != null`. With a null application context
  neither mode ever fires, whatever host service is installed.

This is not academic for this port: search-parameter extraction, validation
invariants and CQL will all pass an `appInfo`, so the shadowing path is live on
the write path. The text must say which site uses which mode, that
`IMPLICIT_BEFORE` pre-empts navigation on a non-empty answer, that
`IMPLICIT_AFTER` is a fall-through on empty, and that both require a non-null
application context.

### 5. `IHostApplicationServices.log` is unspecified, and `takeLog` is destructive

`path.utility.001` (`specs/path/spec.md:1724-1729`) says only that "the trace log
is retrievable after evaluation and contains the names the expressions passed to
`trace`". Two behaviours are missing:

- `:927-928` — `if (hostServices == null || !hostServices.log(name, contents))`
  the engine appends to its own buffer. A host service whose `log` returns
  **`true` suppresses the engine's trace buffer entirely**. `log` is the tenth
  method on `IHostApplicationServices` and now the only one with no scenario;
  the other nine are covered by `path.env.012/015` and
  `path.utility.012/015/020/027`.
- `:6687-6699` — `hasLog()`/`takeLog()`, where `takeLog()` returns the buffer
  **and resets it**. That is what lets `FHIRPathTests.java:250-252` drain it per
  case without leakage between cases, and the harness scenarios depend on it.

Add the host hook and the clear-on-take semantics to `path.utility.001`, or give
them a scenario of their own.

### 6. `path.check.015-019` name an oracle that cannot grade them (minor, but it is a hole in the gate)

`spec.md:93` maps `path.check.*` to "the 29 `invalid="semantic"` cases plus the
`isSemanticChecks` group". That selector grades pass/fail of the check, not its
**warning** channel, and the warning list is not visible to a differential
harness that talks to a live HAPI server over HTTP either. The five new warning
scenarios are therefore the only scenarios in the delta with no oracle that can
reach them, and they are not among the sixteen `differential-only` scenarios
listed at `spec.md:107-112`.

They are perfectly testable — `checkOnTypes(..., warnings)` is a public method
and the message ids are stable constants — so the fix is to name that as the
oracle (an in-process unit oracle over the generated R4 model, with the message
ids as the selector) rather than leave `path.check.*`'s suite selector implying a
grading it cannot perform.

---

## Noted, no action required

- **`setLocation` (`:236`) has no scenario.** It is the one public engine setter
  without one, and it is observable: `:574-576` and `:586-588` append a
  `FHIRPATH_LOCATION` clause to every error message when `location` is non-null.
  It defaults to `null`, no HAPI wrapper and no caller this change ships sets it,
  and `path.errors.002` specifies the rest of the message, so omitting it costs
  no parity here. Worth a line when `add-validator-core` arrives, which is the
  caller that sets it.
- **`DEV-0007` cites `ENGINE:5462`; `funcExtension` begins at `:5464`.** Two
  lines, no ambiguity about which method is meant.
- **`FHIRLexer`'s `metadataFormat` and the engine's `liquidMode` field.** `:334`
  hard-codes `metadataFormat` to `false` and never threads `liquidMode` into the
  lexer it constructs, so `path.strictness.010`'s assertion about the off state
  is correct for a reason slightly stronger than it states. No change needed
  while liquid mode is out of scope.
