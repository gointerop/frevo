APPROVED

Seventh review of `add-fhirpath-and-codegen-r4`. It covers commit `85cabd3`, which
changes only `specs/path/spec.md` (`path.check.009` and `path.oracle.016`). The
sixth pass can be read with
`git show 89cce09:openspec/changes/add-fhirpath-and-codegen-r4/review.md`.

Verified independently against `oracles/fhirpath/tests-fhir-r4.xml` at
`c95adbee099c875de97d25ed7616aae7d66dd85b` and the pinned R4 `FHIRPathEngine`/
`FHIRPathTests` at `73c432b5f43ddbce979978a760ec4aa81e929094`. A script parsed the
suite, grouped every `<test>` on expression text and `inputfile`, and selected
groups with differing `context`/`contextType` and differing expected outcomes.
It gave exactly 6 groups and 13 cases, matching `path.oracle.016` and task 3, plus
the 2 excluded groups (`isEffectOnLogic1`/`isGuardImpliesEmpty`,
`isEffectOnLogic2`/`isEffectOnLogic3`). The suite has 29 `invalid="semantic"`
cases in total. I checked each of the 13 cases' `invalid` attribute,
`contextType` and output, and the revised partition is correct member by member:
- 3 expect success and fail the check on a non-primitive `Period` focus:
  `isGuardOr`, `isGuardOrInnerImplies`, `isGuardNotImplies`.
- 6 are semantic-invalid on `Period` or `Period|Timing`: `isEffectOnLogic3Base`,
  `isGuardOrBad`, `isGuardXorBad`, `isGuardOrInnerImpliesBad`,
  `isGuardOrTypesImpliesBad`, `isGuardNotImpliesBad`.
- 4 have no `invalid` attribute and a focus that includes a primitive:
  `isEffectOnLogic2Base` (`dateTime`), `isGuardOr2` and `isGuardXorBad2`
  (`Period|dateTime`, the latter expecting `false` at `SUITE:1777-1779`), and
  `isGuardOrTypesImpliesAll` (no context type).

3 + 6 + 4 = 13, with no member counted twice.

In `path.check.009`, the `isGuardOr` group really is the three-case group
{`isGuardOr`, `isGuardOr2`, `isGuardOrBad`}, and `isGuardOrInnerImplies` is in a
pair with `isGuardOrInnerImpliesBad`.

Cross-scenario counts agree:
- The `hapi-cannot-pass` attributions across `path.check.005`, `.006` and `.009`
  (4 + 2 + 7) are exactly the 13.
- The semantic-invalid cases that `path.check.005`–`.010` list for
  `isSemanticChecks` (6 + 1 + 1 + 1 + 5 + 1) are exactly that group's 15.
- `path.check.012`'s `Period|Timing` list (6 graded `passed`, 3 graded `failed`)
  covers all 9 such cases, and its `Period|dateTime` pair is exact.
- The 14 expect-success cases that task 51 says end at the check stage are
  exactly the expect-success cases with a non-primitive focus and no
  `ofType`/`as`.

Scenario IDs are unchanged from `89cce09`: the heading lists are identical, and
there are no duplicates across the change's spec deltas.
