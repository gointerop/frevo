CHANGES_REQUESTED

Sixth review of `add-fhirpath-and-codegen-r4`. It covers commit `0d32fae`, which
changes only `specs/path/spec.md` (`path.check.005`, `.006`, `.009`,
`path.oracle.016`) and `tasks.md` (task text at `:92-108`). The fifth pass is
recoverable with
`git show 9edf103:openspec/changes/add-fhirpath-and-codegen-r4/review.md`.

Reviewed against `reference/org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094` and `oracles/fhirpath/tests-fhir-r4.xml`
at `c95adbee099c875de97d25ed7616aae7d66dd85b`. `SUITE` means that file and
`HARNESS` means
`org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java`.

The fifth pass's finding is resolved: the enumeration in `path.oracle.016` and
task 3 is now exactly the set its rule selects. One new slip, introduced by the
revision's new Frevo-status sentence, remains.

## Findings

1. **`path.oracle.016` says "the six 'Bad' cases" fail the check, but only five
   of the 13 do. `isGuardXorBad2` is counted twice and given two contradictory
   Frevo outcomes.** The new final AND bullet partitions the 13 cases by Frevo's
   outcome. It reads: three fail the check graded `failed`; "the six 'Bad' cases
   and `isEffectOnLogic3Base` fail the check and are graded `passed`"; and
   "evaluation decides `isEffectOnLogic2Base`, `isGuardOr2`, `isGuardXorBad2`
   and `isGuardOrTypesImpliesAll`". That is 3 + 6 + 1 + 4 = 14 cases for a
   13-case set.

   Six of the 13 names contain "Bad": `isGuardOrBad`, `isGuardXorBad`,
   `isGuardXorBad2`, `isGuardOrInnerImpliesBad`, `isGuardOrTypesImpliesBad`
   and `isGuardNotImpliesBad`. `isGuardXorBad2`, despite its name, carries no
   `invalid` attribute and expects `<output type="boolean">false</output>`
   (`SUITE:1777-1779`). `path.check.009` and `path.check.012` both say it
   **passes** the check with `Period|dateTime`, and the same bullet lists it
   under "evaluation decides". So "fails the check and is graded `passed`" is
   wrong for it. An oracle-builder who writes a test from "the six Bad cases"
   would assert a check failure that `path.check.009`/`.012` forbid, or would
   have to guess which "Bad" is meant.

   **Fix:** say "the five `invalid="semantic"` cases (`isGuardOrBad`,
   `isGuardXorBad`, `isGuardOrInnerImpliesBad`, `isGuardOrTypesImpliesBad`,
   `isGuardNotImpliesBad`) and `isEffectOnLogic3Base`". Then 3 + 5 + 1 + 4 = 13.
   Task 3's text does not repeat the count, so it needs no change.

## Verified, no finding

**(1) The `hapi-cannot-pass` list is complete and exact, across the whole
suite.** I computed this with my own script rather than from the proposal. It
parses every `<test>` and `<modeTest>` child of every `<group>`, as
`HARNESS:127-144` does (1,191 cases). It groups them on the inputs the harness
reads: expression text, `inputfile`, `predicate` and `ordered`
(`HARNESS:178-180`, `:203-213`, `:219-224`, `:255-265`). It then flags groups
whose expected outcome (`invalid` value plus the ordered list of `type`/body
outputs) differs between members.

- The harness reads none of `mode`, `context`, `contextType`, `version` or
  `checkOrderedFunctions`. It checks against `res.getResourceType()`, which is
  `Observation` for every case concerned (`HARNESS:219-224`). So any group in
  which those inputs are identical gets one HAPI outcome per stage.
- **Exactly six groups have differing expected outcomes, 13 cases, all in
  `isSemanticChecks`.** In each one, the members have the same `context`
  (`Observation.effective`) and differ only by `contextType`. They are
  `isEffectOnLogic2Base`/`3Base`, `isGuardOr`/`isGuardOr2`/`isGuardOrBad`,
  `isGuardXorBad`/`isGuardXorBad2`,
  `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad`,
  `isGuardOrTypesImpliesBad`/`isGuardOrTypesImpliesAll` and
  `isGuardNotImplies`/`isGuardNotImpliesBad`. This matches `path.oracle.016`
  and task 3 (`tasks.md:92-108`) member for member.
- **The two exclusions are correct.** `isEffectOnLogic1`/`isGuardImpliesEmpty`
  (both `true`) and `isEffectOnLogic2`/`isEffectOnLogic3` (both `false`) share
  an expression, `inputfile` and `context`, differ only by `contextType`, and
  have identical expected outcomes.
- **No other group differing in `mode`, `version` or anything else is wrongly
  omitted.** The remaining 11 groups with identical HAPI-visible inputs all have
  identical expected outcomes. Ten of them differ in nothing the harness ignores
  (for example `testLiterals/testLiteralInteger1` and
  `testTypes/testIntegerLiteralConvertsToInteger`). One,
  `documentFunctions/testSameSubject12` and `testDocumentElement6`, differs only
  by `mode`. Both expect `true`, so HAPI is not forced to fail either. No
  version-only group exists. The rule's "differ only by `context` and
  `contextType`" restriction therefore loses nothing in the pinned suite.
- Grouping on whitespace-normalised expressions adds one more group,
  `testTrim1`/`testTrim3`. It has identical outcomes and no attribute
  difference, so it changes nothing.

**(2) HAPI's statuses in `005`, `006`, `009` and `016` agree with each other and
with the Java.** At the `Observation` root, `$this.toString()` fails
`FHIRPATH_PRIMITIVE_ONLY` in every one of the 13 expressions. So HAPI passes
exactly the `invalid="semantic"` member(s) of each group and fails the rest.
At least one case per group fails, as the rule says.

- `005`: `isGuardNotImplies`, `isGuardOrTypesImpliesAll`,
  `isGuardOrTypesImpliesBad` and `isGuardNotImpliesBad` are `hapi-cannot-pass`.
  `isEffectOnLogic3` and `isGuardOrTypesImplies` are `failed`. The second is
  unique (`SUITE:1717`), and the first is in an excluded same-outcome group that
  HAPI fails. The statement that HAPI's run fails all three expect-success
  cases is true.
- `006`: `isEffectOnLogic2Base`/`3Base` are `hapi-cannot-pass`.
  `isEffectOnLogic1Base` has a unique expression and is not in the list, which
  is correct.
- `009`: `isGuardOr`, `isGuardOr2`, `isGuardOrInnerImplies`, `isGuardOrBad`,
  `isGuardOrInnerImpliesBad`, `isGuardXorBad` and `isGuardXorBad2` are
  `hapi-cannot-pass`. `isGuardOrBoth` is unique and `failed`.
  `isGuardOrOtherImpliesBad` and `isGuardOrOtherImpliesNoParensBad` are unique
  and carry no HAPI claim.
- Frevo's statuses in `005`, `009` and `016` agree, apart from finding 1.
  Frevo grades `isGuardOr`, `isGuardOrInnerImplies` and `isGuardNotImplies`
  `failed` at the check. The four check-passing cases have a primitive in focus:
  `dateTime` for `isEffectOnLogic2Base`, `Period|dateTime` for `isGuardOr2` and
  `isGuardXorBad2`, and the `effective[x]` set for `isGuardOrTypesImpliesAll`.

A wording point, not a finding: `009` still calls `isGuardOr` "one half of a
`path.oracle.016` pair". Its group is a triple, which the next bullet states. The
status it gives is right.

**(3) No other scenario names an affected case with a stale status.** I grepped
the change for all 17 names (13 listed plus 4 excluded).

- `path.env.006` (`spec.md:1653`) names `isGuardOrInnerImplies`/`Bad` only for
  evaluation of `%resource`, with no HAPI status.
- `path.check.011` and `.012` give `isGuardImpliesEmpty` `failed` "as for HAPI",
  which is correct (excluded group, HAPI fails both members). `.012` gives the
  listed `Bad` cases Frevo's grade only.
- Task 51's 14 expect-success cases (`tasks.md:997-1003`) are said to end at the
  check graded `failed` "which is parity with HAPI, not a gap". HAPI does fail
  `isGuardOr`, `isGuardOrInnerImplies` and `isGuardNotImplies`. Under
  `path.oracle.012`, a Frevo-failed, HAPI-`hapi-cannot-pass` case is neither
  listed as a regression nor `ahead-of-hapi`, so the claim holds.
- `proposal.md` and `design.md` name none of these cases.

**(4) Scenario ids.** The `path` scenario headings at `0d32fae` are identical,
in order, to those at `9edf103`, with no duplicates.
