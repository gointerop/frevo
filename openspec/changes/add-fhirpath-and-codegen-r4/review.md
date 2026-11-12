CHANGES_REQUESTED

Fourth review of `add-fhirpath-and-codegen-r4`. It is scoped to the revision in
commit `e6f57cd`: `path.check.005` to `path.check.012` in `specs/path/spec.md`,
task 51 in `tasks.md`, and the new out-of-scope row in `proposal.md`. Everything
else stays as the third pass approved it (`8f3aca9`; that pass's text is
recoverable with `git show 8f3aca9:openspec/changes/add-fhirpath-and-codegen-r4/review.md`).

The revision gets the substance right. The pinned engine does not narrow, the 38
cases are classified correctly, and none of them is lost. Two findings remain.
Both are small, but each would give an oracle-builder a test that contradicts
another scenario or HAPI's actual output.

Reviewed against `reference/org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094` and `oracles/fhirpath/tests-fhir-r4.xml`
at `c95adbee099c875de97d25ed7616aae7d66dd85b`. `ENGINE` means
`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`.

## Findings

1. **`path.check.009` gives HAPI's status for `isGuardOr` and
   `isGuardOrInnerImplies` as `failed`. `path.oracle.016` records it as
   `hapi-cannot-pass`.** `path.check.009` says `isGuardOr`, `isGuardOrBoth` and
   `isGuardOrInnerImplies` "end at the check stage and are graded `failed`, as
   they are for HAPI". But `path.oracle.016` and task 4's text
   (`tasks.md:92-98`) require HAPI's status for both halves of the
   `isGuardOr`/`isGuardOrBad` and
   `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad` pairs to be recorded as
   `hapi-cannot-pass`, "a status distinct from `failed`". `path.oracle.011`
   says the report carries that HAPI status per case, so two scenarios give the
   same report field different values. An oracle-builder who asserts the HAPI
   column from `path.check.009` writes a test that `path.oracle.016` makes fail.

   Frevo's own status (`failed`) is correct: the pinned R4 harness checks
   against the `Observation` root (`FHIRPathTests.java:222`), and Frevo's
   context-aware check with `Period` fails both cases on `toString`. The other
   "as for HAPI" claims in `005`, `007`, `008`, `010` and `011` name no case in
   a `path.oracle.016` pair, so they are consistent. **Fix:** in
   `path.check.009`, say that HAPI's recorded status for `isGuardOr` and
   `isGuardOrInnerImplies` is `hapi-cannot-pass` (`path.oracle.016`), and that
   it is `failed` for `isGuardOrBoth`. Then the case-level oracle has one
   answer.

2. **The quoted `FHIRPATH_PRIMITIVE_ONLY` text is not the message HAPI
   produces.** `path.check.005` quotes the diagnostic as "The function toString
   can only be used on primitives" and cites `Messages.properties:353`. That
   line reads
   `FHIRPATH_PRIMITIVE_ONLY = Error evaluating FHIRPath expression: The function {0} can only be used on primitives`.
   `ENGINE:584-594` (`makeException`) passes it through `formatMessage`
   unchanged, adding only a location suffix when `location` is set (the harness
   sets none). So HAPI's message is "Error evaluating FHIRPath expression: The
   function toString can only be used on primitives". The `006` to `011`
   scenarios refer back to "the same `FHIRPATH_PRIMITIVE_ONLY` diagnostic". A
   test that asserts equality with the quoted text therefore fails against a
   faithful port, and one that passes against the quote reports a different
   message from HAPI. **Fix:** quote the full formatted message, or state that
   the message id `FHIRPATH_PRIMITIVE_ONLY` is what is asserted and the text is
   not.

## Verified, no finding

**(a) There is no narrowing in the pinned engine.** I checked each mechanism:

- **Operator chains:** `ENGINE:1621-1637` types every operand against the same
  `focus` and a fresh `contextForParameter(inContext)`. That context copies
  `thisItem` unchanged (`ENGINE:4761-4770`). `$this` resolves to
  `context.getThisItem()` (`ENGINE:1579-1580`).
- **`is`, the logical operators and `iif`:** the `is` operator and the `is()`
  function type to `Boolean` and leave the focus alone (`ENGINE:2013-2015`,
  `:3401-3404`). The same holds for `or`, `and`, `xor` and `implies`
  (`:2024-2027`). The `iif` arguments are not expression parameters
  (`:3804-3816`), so they are typed against `context.thisItem` (`:3261-3271`).
  `iif`'s result is the union of its branches (`:3441-3449`).
- **What does retype:** only the `ofType()` function, the `as()` function and
  the `as` operator retype (`:3368-3384`, `:2016-2021`). `where` narrows only
  `Reference` targets (`:3329-3352`), and no case uses that.
- **R4B and R5:** the same chain loop appears at R4B `:1535-1538` and R5
  `:1740-1743`. Neither file has narrowing logic (the only `narrow` identifier
  is `asBool`'s evaluation-time flag). So the proposal row's R4B/R5 claim
  holds.

**(b) My own classification of the 38 `isSemanticChecks` cases**
(`tests-fhir-r4.xml:1664-1817`). I assumed a non-narrowing check with the case's
`context`/`contextType`. `toString` passes only if the focus includes a
primitive type, per `checkContextPrimitive` at `:3872-3883` with
`TypeDetails.hasType(Set)` at `:436-448`:

- **Check fails, suite expects success, graded `failed` (14).** These are
  exactly the scenarios' 14: `isEffectOnLogic3`, `isGuardFnImplies`,
  `isGuardImpliesEmpty`, `isGuardOrTypesImplies`, `isGuardAndOtherImplies`,
  `isGuardOtherAndImplies`, `isGuardOrInnerImplies`, `isGuardAnd`, `isGuardOr`,
  `isGuardOrBoth`, `isGuardNotImplies`, `isGuardIifTrue`, `isGuardIifElse`,
  `isGuardNested`.
- **Check fails, suite expects failure, graded `passed` (15).**
  `isEffectOnLogic3Base`, `isGuardFnImpliesBad`, `isGuardImpliesBad`,
  `isGuardOrTypesImpliesBad`, `isGuardOrFnTypesImpliesBad`,
  `isGuardOrOtherImpliesBad`, `isGuardOrOtherImpliesNoParensBad`,
  `isGuardOrInnerImpliesBad`, `isGuardAndBad`, `isGuardOrBad`, `isGuardXorBad`,
  `isGuardNotImpliesBad`, `isGuardIifBad`, `isGuardSupertypeBad`,
  `isGuardOtherExpressionBad`.
- **Check passes, so evaluation decides (9).**
  - `isEffectOnLogic1Base`, `isEffectOnLogic1`, `isGuardOrTypesImpliesAll`:
    there is no `contextType`, so the focus is the `effective[x]` set, which
    includes `dateTime`.
  - `isEffectOnLogic2Base`, `isEffectOnLogic2`: the context type is `dateTime`.
  - `isGuardOr2`, `isGuardXorBad2`: the context type is `Period|dateTime`.
  - `isGuardOfType`, `isGuardAs`: these retype the focus to `dateTime`.

  The data is `effectiveDateTime value="2016-03-28"`, whose length is 10, so
  `>= 10` is true and `> 10` is false, consistent with the expected outputs.

Each scenario's case list matches this classification. Together the lists
assign every one of the 38 cases to a scenario. `%resource.status = 'final'`
types without error, because `Equals` only adds warnings (`:1989-1991`). The
`ofType(dateTime)` of-type-impossible warning is real (`:3781-3784`), and the
`as` operator emits none. All cited line ranges resolve to the code described.

**(c) Nothing HAPI-observable was lost.** The 361 `path` scenario ids are the
same before and after `e6f57cd`, and only the six headings of `005` and
`007`-`011` changed. The previous `011` stated behaviour HAPI does not have.
The multi-type `contextType` rule survives in `012` with HAPI's any-type
semantics. `design.md` contains no narrowing text that contradicts the
revision.

**(d) Task 51 is coherent.** It satisfies the same `005`-`012` range, and its
prose matches the scenarios. It does not reach into task 50's scope, and its
prerequisites are stated. Those prerequisites are the eight `path_check.rs`
tests that still assert narrowing, and the `type_check_engine_under_test()`
seam. The tests are the oracle-builder's to retarget. No other task still
describes narrowing.
