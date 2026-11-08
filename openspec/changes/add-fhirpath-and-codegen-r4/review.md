APPROVED

Third review of `add-fhirpath-and-codegen-r4`, at commit `258a5b9`. All six
findings from the second review are **resolved**, and the eight substantive
claims the revision makes about the pinned source are **correct as written**.
Nothing blocks. Six items are recorded at the end as non-blocking; one of them
(`tasks.md:674`) is a wrong register id that should be corrected before merge.

Reviewed against `reference/org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094`, `reference/hapi-fhir` at
`7ab4c7c7c49281496faa1b3c2498a652457669ae`, and
`oracles/fhirpath/tests-fhir-r4.xml` at
`c95adbee099c875de97d25ed7616aae7d66dd85b`, all read from
`C:/gointerop/git/frevo/reference/` and `C:/gointerop/git/frevo/oracles/`.
Unqualified `:NNN` references are `FHIRPathEngine.java`; `ParserState`,
`BaseParser`, `JsonParser`, `LenientErrorHandler` and `StrictErrorHandler` are
in `reference/hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/`.

## The record of the three passes

| Pass | Commit graded | Verdict | Findings | Disposition |
|---|---|---|---|---|
| 1 | `3c3f092` | `CHANGES_REQUESTED` | 21 | all 21 resolved by `1bbe673`, verified in pass 2 |
| 2 | `1bbe673` | `CHANGES_REQUESTED` | 6 (2 blocking) | all 6 resolved by `258a5b9`, verified below |
| 3 | `258a5b9` | **`APPROVED`** | 0 | 6 non-blocking notes |

Pass 1's findings and their evidence are in the git history of this file at
`93898b1`; pass 2's are in its parent. Both are recoverable with
`git show 93898b1:openspec/changes/add-fhirpath-and-codegen-r4/review.md`.

## Scenario integrity — clean

Verified by extracting the `#### Scenario: [id]` set from `3c3f092`, `1bbe673`
and the working tree:

- 454 -> 486 -> **493** ids;
- `comm -23` of `1bbe673` against the tree is **empty**, and of `3c3f092`
  against the tree is **empty** — nothing was removed or renumbered at either
  revision;
- exactly seven added, and they are exactly the seven claimed:
  `model.canonical.007`, `model.json.024`, `model.json.025`, `path.env.016`,
  `path.env.017`, `path.oracle.017`, `path.utility.028`;
- the six rewritten in place (`path.oracle.002`, `.007`, `.008`,
  `path.env.015`, `path.utility.001`, `path.check.015`) kept their headings;
  only their bodies changed;
- all 493 match `^[a-z]+\.[a-z]+\.[0-9]{3}$`, all are unique across the three
  delta files, and **every one of the 45 areas is contiguous from `001`** with
  no gaps;
- per-capability totals are `codegen` 49, `model` 83, `path` 361 = **493**,
  matching `spec.md:8-13`, and every row of the per-area table at
  `spec.md:51-96` matches the files exactly;
- every one of the 493 is named by at least one task in `tasks.md` (expanding
  its `X through Y` ranges), and `tasks.md` names **no** id that does not
  exist.

The nineteen scenarios `spec.md:124-131` lists as having no live case are
exactly the nineteen whose own text says `differential-only` — checked in both
directions. `path.check.015` and `path.oracle.013` mention the differential
harness for other reasons and are correctly excluded from that list.

## Disposition of the six standing findings

| # | Prior finding | Disposition | Evidence I checked |
|---|---|---|---|
| 1 | Three of `IParserErrorHandler`'s ten callbacks have no scenario (blocking) | **addressed** | `model.json.024` (`containedResourceWithNoId`), `model.json.025` (`unknownReference`) and `model.canonical.007` (`invalidInternalReference`) added, each with the lenient diagnostic, the surviving resource, and the strict error with its HAPI code. Codes verified: `1819` (`StrictErrorHandler.java:38-40`), `1826` (`:94-97`), `2724` (`:100-105`); lenient log texts at `LenientErrorHandler.java:75-79`, `:207-211`, `:214-221`. `design.md:200-209` now carries the full ten-callback table and every callback maps to a scenario |
| 2 | `path.oracle.002`'s attribute inventory incomplete, fatally (blocking) | **addressed, and now exhaustive** | I re-enumerated every attribute in the pinned file by XML walk. Result, per element: `tests` {`name`,`description`,`reference`}; `group` {`name`(103),`description`(4),`reference`(1),`mode`(2)}; `test` {`name`(1191),`inputfile`(1101),`description`(106),`predicate`(2),`mode`(11),`checkOrderedFunctions`(1),`ordered`(1),`version`(31),`testing`(153),`context`(38),`contextType`(35)}; `output` {`type`(1032)}; `expression` {`invalid`(63),`mode`(1)}. The scenario's five lists match exactly, and every value domain it records matches: `expression/@invalid` {syntax 7, semantic 29, execution 27}, `expression/@mode` {strict}, `test/@mode` {strict 5, element 6}, `group/@mode` {semantic 1, custom 1}, `test/@predicate` {true,false}, `test/@ordered` {false}, `test/@checkOrderedFunctions` {true}, `test/@version` {2.1.0}. The expression-level `mode` is no longer unhandled: `path.oracle.017` was added for it, and its claim that the outcome cannot depend on the attribute is right — `doSQLOnFHIRCheck` aside, nothing in the type pass gates on a mode, and `iif`'s first parameter is typed against a singleton boolean unconditionally |
| 3 | The `@`/`@T` prefix on a temporal expectation | **addressed, both halves** | `path.oracle.008` now states the prefix and names the rule. Verified: `BaseDateTimeType.java:1069-1072` returns `"@" + primitiveValue()`, `TimeType.java:147-150` returns `"@T" + primitiveValue()`, `PrimitiveType.java:259-261` returns `primitiveValue()`. The ordered branch at `FHIRPathTests.java:301-308` compares against `fpValue()`. `path.oracle.007` now states the unordered branch separately and correctly: `FHIRPathTests.java:265-283` uses `fp.convertToString` for a `Quantity` and `((PrimitiveType) …).asStringValue()` otherwise — no prefix — and matches existentially on `(type absent or equal) AND (body absent or equal)`, consuming nothing. I confirmed the one `ordered="false"` case is `testMiscellaneousAccessorTests/testExpressions` at `:321`, and that 71 of 1,103 `<output>` elements carry no `type` |
| 4 | `path.env.015` mis-described `NOVALUE` and omitted the implicit override | **addressed, now correct** | The mode sites are exactly as the rewritten scenario states. `EXPLICIT`: two sites only, `:1497` (evaluation constant node) and `:1610` (check constant node). `NOVALUE`: one site only, `:1586`, reached under `atEntry && focus == null` in the type pass's `case Name` (`:1578-1586`), and `executeContextType` at `:3222-3227` throws `FHIRPATH_HO_HOST_SERVICES` with no host service, which is the clause `path.env.015` adds. `IMPLICIT_BEFORE`: `:3184-3191`, gated on `atEntry && context.appInfo != null && hostServices != null`, **before** the uppercase leading-type-name special case (`:3192`) and before `getChildrenByName` (`:3209`), returning the host answer and never navigating when it is non-empty. `IMPLICIT_AFTER`: `:3211-3215`, gated additionally on `result.isEmpty()`. New `path.env.016` and `path.env.017` state both, and `DEV-0010` records the divergence |
| 5 | `log` unspecified, `takeLog` destructive | **addressed** | `path.utility.001` now states the take-and-clear semantics (`:6692-6698`: `takeLog` returns the buffer then assigns a fresh `StringBuilder`; `hasLog` at `:6687-6689`) and the entry format (`:927-944`: name, `": "`, items joined by `","`, entries joined by `"; "`). New `path.utility.028` states the host hook: `:928` is `if (hostServices == null || !hostServices.log(name, contents))`, so a `true` return suppresses the buffer. The private `log(...)` has exactly two callers, both in `funcTrace` (`:5628`, `:5630`); `Math.log` at `:4209` and `customLog` at `:4244` are unrelated, as the scenario says |
| 6 | `path.check.015-019` named an oracle that cannot grade them | **addressed** | `spec.md:101` now maps `path.check.015`-`019` to `path-check-warnings`, an in-process unit oracle whose selector is the warning message id, and `spec.md:100` narrows the suite row to "`path.check.*` except `015`-`019`". `model.json.024`, `.025` and `model.canonical.007` get the matching `model-parser-errors` row at `spec.md:102`. Both names are used consistently in `spec.md`, `tasks.md` and `design.md`, and both have a build task (`4a`, `5a`) with the scenario ids they satisfy. `spec.md:133-137` restates that these eight are graded by neither the suite nor the differential, so no scenario in the delta is now without a reachable oracle |

## The revision's own claims about the source — all verified

Each of these is load-bearing in the new text, so I checked each against the
pinned code rather than against the delta.

| Claim | Verdict | Evidence |
|---|---|---|
| In the R4 contained-resource state the resource **is** appended to `contained` with no `id`, merely withheld from the id map, and HAPI's own comment contradicts the code | **correct** | `ParserState.java:436-455` (`ContainedResourcesStateHl7Org.wereBack`). The comment at `:442-443` says "we don't keep the resource because it's useless"; `:454` then runs `def.getChildByName("contained").getMutator().addValue(preResCurrentElement, res)` **unconditionally**, outside the `if`/`else`. Only the `getContainedResources().put(...)` at `:447-449` is inside the `else`. The HAPI-structures sibling at `:395-416` is the same shape |
| `weaveContainedResources` resolves only refs longer than a bare `#` | **correct** | `ParserState.java:1277-1292`: `if (isNotBlank(ref))` then `if (ref.startsWith("#") && ref.length() > 1)`. A bare `#` and a blank reference are neither resolved nor diagnosed. The pass runs from `postProcess` (`:1319`, `:1364`), i.e. after the whole document, so a forward reference resolves — which is what `model.json.025` asserts |
| `isValidInternalReference` exempts exactly `"#"` and non-local references | **correct** | `BaseParser.java:269-274`: `return !ref.isLocal() || ref.getValue().equals("#") || theContext.getContainedResources().referenceMatchesAContainedResource(ref);`. The call is guarded by `else` on `if (isBlank(ref.getIdPart()))` at `:200`, so a blank id part takes the contained-resource-naming branch and is never diagnosed — exactly `model.canonical.007`'s third clause |
| `invalidInternalReference` is the only encode-side condition | **correct** | `grep 'myErrorHandler\.' BaseParser.java JsonParser.java XmlParser.java` returns one hit, `BaseParser.java:247`. Every other callback fires from `ParserState` |
| `fpValue()` has no caller in the engine | **correct** | Across the whole of `reference/org.hl7.fhir.core`, the only callers are the three per-version `FHIRPathTests` harnesses (R4 at `:301`, `:304`, `:306`, `:308`). The three implementations are the only definitions. So the prefix really is a property of the harness's comparison, as `path.oracle.008` and `docs/deviations.md`'s "considered and not recorded" entry both say |
| The private `log(...)` has exactly two callers, both in `funcTrace` | **correct** | `:5628` and `:5630`, in `funcTrace` (`:5622-5632`) |
| `IHostApplicationServices` has exactly ten methods | **correct** | `resolveConstant`, `resolveConstantType`, `log`, `resolveFunction`, `checkFunction`, `executeFunction`, `resolveReference`, `conformsToProfile`, `resolveValueSet`, `paramIsType` (`IHostApplicationServices.java:51`, `71`, `80`, `88`, `97`, `105`, `123`, `125`, `130`, `138`). `path.utility.028`'s closing inventory assigns a scenario to each, and I checked the nine cross-references it names resolve to scenarios that exist |
| Exactly nine distinct check-pass warning ids | **correct** | The 12 `typeWarnings.add` sites (`:1646`, `:1649`, `:1972`, `:1974`, `:1980`, `:1983`, `:3302`, `:3579`, `:3588`, `:3783`, `:3834`, `:3841`) carry 9 distinct `I18nConstants` ids: `FHIRPATH_CHOICE_NO_TYPE_SPECIFIER`, `_SPURIOUS_TYPE_SPECIFIER`, `FHIRPATH_COLLECTION_STATUS_OPERATION_LEFT`, `_RIGHT`, `_PARAMETER`, `_CONTEXT`, `FHIRPATH_NOT_A_COLLECTION`, `FHIRPATH_UNKNOWN_EXTENSION`, `FHIRPATH_OFTYPE_IMPOSSIBLE`. A tenth would fail `path.check.015`, as it requires |
| `atEntry` propagates into function arguments, so `IMPLICIT_BEFORE`'s call sites are broader than the leading name of the whole expression | **correct** | The list-level `execute` (`:1467`) passes `atEntry` straight into the per-item `execute` (`:1482`) inside the per-focus-item loop, and 56 call sites in the engine evaluate a function parameter with `execute(..., exp.getParameters().get(n), true)` — e.g. `funcWhere` at `:6255`, `funcTrace` at `:5623`. So the host is asked once per focus item at the entry of each sub-expression, which is what `path.env.016` and `design.md:434-440` say |
| `unknownAttribute` is reachable in JSON mode via an `id` property | **correct** | `ParserState.java:1517-1527` (`PrimitiveState.attributeValue`): an `id` on an instance that is none of `IIdentifiableElement`, `IBaseElement`, `IBaseResource` calls `unknownAttribute` with a `ParseLocation`. The generic `BaseState.attributeValue` at `:282-284` is the other site. Worth knowing for the implementer: `ElementCompositeState.attributeValue` at `:616-621` routes a **non**-`id` attribute to `incorrectJsonType` in JSON mode and to `unknownAttribute` only in XML mode — so `model.json.004`'s JSON wording must not be read as covering that third site |
| The four constant modes map one-for-one onto HAPI's server-facing wrapper | **correct** | `IFhirPathEvaluationContext.java:38-47` declares the same four, with the javadoc at `:31-32` stating it "matches org.hl7.fhir.core 1-to-1"; `FhirPathR4.java:106-113` is an exhaustive four-arm `switch`. The wrapper's own `log` returns `false` (`FhirPathR4.java:128-131`), so HAPI's server never suppresses the engine buffer — consistent with `path.utility.028` |

## The new and rewritten scenarios say what the code does

Spot-checked beyond the claims above:

- `path.check.016` against `checkCardinalityForComparabilitySame` (`:1971-1976`)
  and `checkCardinalityForSingle` (`:1978-1986`) — the left/right split is on
  `isList()`, and the parameter and context variants are `checkParamTypes`
  (`:3833-3835`) and `checkSingleton` (`:3839-3843`). All warn, none throws.
- `path.check.017` against `doSQLOnFHIRCheck` (`:1641-1652`): gated on
  `emitSQLonFHIRWarning`, `FHIRPATH_CHOICE_NO_TYPE_SPECIFIER` when the focus is
  a choice and the inner node is not `OfType`, `_SPURIOUS_TYPE_SPECIFIER` when
  it is not a choice and the inner node is `OfType`. Exactly the scenario's
  three cases.
- `path.check.019` against the surfaces: only the three `checkOnTypes`
  overloads (`:477`, `:532`, `:539`) `typeWarnings.clear()` and then
  `warnings.addAll(typeWarnings)` (`:528`, `:535`, `:542`); the six plain
  `check(...)` overloads (`:405`, `:421`, `:557`, `:596`, `:625`, `:631`) take
  no list and never clear. The clear is at the **start** of a
  warning-taking check, which makes the scenario's no-leak clause observably
  exact.
- `model.json.014` against `JsonParser.java:1459-1462`: a repeating element
  given a single object calls `incorrectJsonType(ARRAY expected, OBJECT found)`
  and then parses the one entry anyway — which is what the scenario says. (See
  note 2 below for `design.md`'s table.)
- `model.json.015` against `ParserState.java:660-673`: a `0..1` child seen
  twice calls `unexpectedRepeatingElement` and pushes
  `SwallowChildrenWholeState`, so the first repetition is kept.
- `path.api.*` against the engine's public surface. I listed every `public`
  member of `FHIRPathEngine` (61 of them) and confirmed the delta's omissions
  are deliberate and harmless: `parsePartial`, `evaluateDefinition`,
  `getElementDefinition`, `bytesToHex`/`hexStringToByteArray`,
  `makeIdentifier`, `forLog`, `getWorker`, `getProfileUtilities` and
  `getTerminologyServiceOptions` are not on the starter's `IFhirPath` surface
  and are not reached by any caller this change ships. `parseQuantityString`
  is covered by `path.oracle.009`, `convertToString` by `path.oracle.007`.

## Noted, no action required to unblock

1. **`tasks.md:674` still cites `DEBT-0006`.** `docs/deviations.md`'s `DEV-0010`
   correctly carries `Debt: DEBT-0010`, and `DEBT-0010` exists on
   `chore/tech-debt-0010` (`### DEBT-0010 — a Frevo host service can shadow a
   real element, and nothing reports it`, origin `DEV-0010`, landing at task
   47a). Task 61a still reads "`DEV-0010` needs `DEBT-0006` added to
   `docs/tech-debt.md`". `DEBT-0006` on `main` is the **paid**
   `r4-example-roundtrip` entry; following that sentence would edit an
   unrelated register entry. Correct the id to `DEBT-0010` before merge, and
   drop the "cannot be written from this branch" clause — the entry already
   exists on its own branch. This is the only place in the artifacts that still
   names `DEBT-0006`.
2. **`design.md:204` maps `unexpectedRepeatingElement` to `model.json.014`.**
   `model.json.014` is an `incorrectJsonType` case
   (`JsonParser.java:1461`); `unexpectedRepeatingElement` is `model.json.015`
   alone (`ParserState.java:669`). The callback is still covered and the
   normative scenario is right, so nothing is missing — but the design table
   would send an implementer to the wrong diagnostic for `.014`.
3. **`model.json.016` and `model.json.018` give strict mode two errors HAPI
   has no callback for.** Both scenarios say so themselves (a duplicate JSON
   key, and `null` in a non-primitive array), and `model.json.021` makes every
   lenient diagnostic a strict error by rule, so the behaviour is specified and
   testable. It costs no parity: `reference/hapi-fhir-jpaserver-starter/src`
   contains no occurrence of `ParserErrorHandler`, `StrictErrorHandler` or
   `LenientErrorHandler`, so the starter never selects strict mode and the
   differential harness cannot reach it. `docs/deviations.md`'s "Lenient
   request parsing" entry currently says Frevo "keeps both modes and the same
   default, so it matches HAPI rather than differing from it"; a clause noting
   these two strict-only additions would make the register exactly true.
4. **`path.check.018`'s not-a-collection clause is vaguer than the code.** It
   says "a navigation into a non-collection where a collection is expected".
   The actual trigger is `first`, `last`, `tail`, `skip` or `take` applied to a
   focus whose collection status is `SINGLETON` (`:3300-3303`). An oracle
   author would probably land on a working expression, but naming the five
   functions would remove the guess.
5. **`setLocation` (`:236`) still has no scenario.** Carried unchanged from pass
   2: it is observable (`:574-576`, `:586-588` append a `FHIRPATH_LOCATION`
   clause when `location` is non-null), but it defaults to `null`, no HAPI
   wrapper and no caller this change ships sets it, and `path.errors.002`
   specifies the rest of the message. Worth a line when `add-validator-core`
   arrives.
6. **`design.md`'s `path-extract-patient` paragraph says the engine asks the
   host service "once per expression".** It is once per focus item at the entry
   of each sub-expression (`:1481-1483` inside the per-item loop). The budget
   argument the paragraph makes is strengthened by that, not weakened.

## Deviation register — no approval is claimed that did not happen

- `docs/deviations.md` references `DEV-0001` … `DEV-0010`; all ten exist, each
  with an `Area`, `Scenario IDs` line naming scenarios that exist, a chosen
  behaviour, a `Debt:` field and a `Reviewed:` line.
- `DEV-0001` … `DEV-0009` carry `Reviewed: 2026-11-08, owner — accepted`.
- `DEV-0010` carries `Reviewed: pending — raised by add-fhirpath-and-codegen-r4
  after the 2026-11-08 review of DEV-0001..0009, so it has not been seen by the
  owner`, and the Entries preamble (`:56-60`) says the same: nine accepted,
  `DEV-0010` still `pending` and needing its own sign-off.
- I grepped the whole change directory and `docs/deviations.md` for any claim
  that `DEV-0010` is accepted, or that all ten were reviewed on 2026-11-08.
  There is none. `tasks.md:665-675` treats all ten as still to be raised at the
  checkpoint. The register claims no approval that did not happen.
