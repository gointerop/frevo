APPROVED

Ninth review of `add-fhirpath-and-codegen-r4`. It covers commit `0f2a419`, which
answers the single finding of the eighth review (`d9fcfc5`): `path.oracle.014`
wrongly said all 115 `documentFunctions` cases fail. The commit also changes
`path.check.002`, `path.errors.001`, the `spec.md` index, tasks 3, 30 and 53,
and `proposal.md`. Before reading the revised text, I recounted every figure by
running an XML parse (`xml.etree`, so commented-out elements are not counted)
over `oracles/fhirpath/tests-fhir-r4.xml`. I then read the upstream code paths
again.

## Recount (script over the pinned suite)

- 1,191 cases in total. The `invalid` counts are 7 `syntax`, 29 `semantic` and
  27 `execution`. Outside `documentFunctions` they are 3 `syntax`, 27
  `semantic` and 21 `execution`.
- `documentFunctions` has 115 cases. 103 expect success, 6 expect an
  `execution` failure, 4 expect a `syntax` failure and 2 expect a `semantic`
  failure.
- Each of the 115 contains a call `name(` to at least one of `section`,
  `entry`, `entryIn`, `resources`, `resourcesIn`, `sameSubject` or
  `hasCoding`. I checked for the call itself, not just the name, because
  `Bundle.entry` is navigation and is not a call. No case outside the group
  calls any of the seven.
- The four syntax cases are `testSection13` (:1860), `testEntry12` (:1879),
  `testSameSubject16` (:1930) and `testHasCoding10` (:1950). The six execution
  cases are `testSection11`, `testSection12`, `testEntryIn5`, `testEntryIn6`,
  `testResources7` and `testResourcesIn3`. The two semantic cases are
  `testResourcesAbstract3` (:1940) and `testHasCoding11` (:1951). All of these
  names and line numbers match the spec.
- 111 / 1,191 = 9.32%, which matches the "9.3%" in `proposal.md`. The old
  9.7% figure was 115 / 1,191.
- Task 30's arithmetic holds: 3 syntax cases outside the group plus the
  group's 115 cases gives 118 graded, and 1,191 - 118 = 1,073 did-not-run.
  This agrees with task 50's baseline of "1,073 did-not-run / 111 failed / 7
  passed": 111 + 7 = 118, and the 7 passed are 3 + 4 syntax cases. Task 50's
  post-check figures of 1,019 + 126 + 46 also sum to 1,191.

## Upstream behaviour

- `FHIRPathEngine.java:1158-1166` (R4): `Function.fromCode` returns null for
  all seven functions. The engine then calls `hostServices.resolveFunction`.
  The suite's host service throws `NotImplementedException` from that call
  (`FHIRPathTests.java:72-76`), so `fp.parse` throws.
- `FHIRPathTests.java:196-203`: a parse exception passes the case only when
  `fail == SYNTAX`. So HAPI gets 4 passed and 111 failed in the group. This
  matches `path.oracle.014`.
- Frevo's `crates/frevo-path/tests/oracle/run.rs` (`classify` and the
  `Diagnostic` arm): an `EngineError::NotImplemented` is graded `did-not-run`,
  and only a `Diagnostic` is graded against the expected stage. That is why
  `path.oracle.014` requires the unknown function to be a real parse
  diagnostic and not "not implemented". The requirement is correct and
  necessary: with a not-implemented error, the four syntax cases would never
  reach `passed`, and Frevo would fall 4 below HAPI. The requirement is also
  testable: the stage and status are observable in the run report.
- `path.errors.001`: I listed the 21 execution-invalid cases outside the group
  and sampled their check paths in the R4 engine. `operateTypes` for
  `Minus`/`Concatenate`/`LessThan` (`:1987-2085`) can only add warnings
  (`checkCardinalityForSingle`). The `as`/`ofType` checks (`:3368-3385`,
  `checkType` `:3767-3786`) also only warn, here about an impossible cast. So
  the claim that these cases parse, check and then fail at evaluation is
  consistent with the engine. The six cases inside the group fail at parse
  and are graded `failed` under `path.oracle.005`'s stage rule, as the
  scenario says.

## Checklist

1. **Coverage / fidelity.** `path.oracle.014`, `path.check.002` and
   `path.errors.001` now each state HAPI's actual outcome, and the three agree
   with each other: the 111 failed in the group are 103 + 6 + 2, and these are
   the same 6 and 2 that `path.errors.001` and `path.check.002` carve out.
   `path.oracle.016`'s closing bullet agrees too. No scenario, task (3, 28-30,
   50, 53) or `proposal.md` passage repeats a statistic that is now false. I
   grepped every occurrence of 115, 111, 118, 1,073, 27, 29, 21, 7 and the
   percentages.
2. **Scenario IDs.** The commit changes only two scenario titles. Both keep
   their IDs (`path.errors.001`, `path.oracle.014`). No ID is renumbered or
   duplicated.
3. **Oracles.** The scenarios are unchanged and still graded by the same
   suite selectors.
4. **Deviations.** None claimed. The behaviour is parity with HAPI.

## Notes (not findings)

- `design.md:519-521` and `docs/deviations.md:~563-565` still say "115 fail".
  No scenario depends on either. Both are recorded for the owner in
  `tasks/blockers.md`, item (c).
- The existing test for `path.errors.001`
  (`crates/frevo-path/tests/path_parse_errors.rs:467`,
  `every_execution_invalid_case_parses_and_checks_and_then_fails_to_evaluate`)
  still asserts that all 27 cases pass at the evaluate stage. That now
  contradicts the revised scenario, and the oracle-builder must align it: 21
  `passed` at evaluate, and the 6 named cases `failed` at parse. The
  `path.oracle.014` test (`path_oracle.rs:429`) already asserts 4 passed / 111
  failed. Its function name still says "not as passing", which is harmless.
