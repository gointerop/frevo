CHANGES_REQUESTED

Findings of the tenth review (commit `d8d783b`, 2026-11-14; detail in the dated
section at the end). `ENGINE` = `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java` at `73c432b5`.

1. **DEV-0015 and `path.filter.002` overstate the indexer defect and contradict
   `path.navigation.008`.** The operator is lost only when the indexer is on
   the **proximal term** of a `parseExpression(…, true)` call. That means the
   first term of the whole expression, of a function argument, or of a
   parenthesised group (`ENGINE:1135-1138` parses a group's content with
   `proximal = true`). In that case `focus = item` (`ENGINE:1199`) and the
   operator loop (`ENGINE:1206-1213`) attaches the operator to the
   non-proximal `item` node, which `execute` (`ENGINE:1508`) and `executeType`
   (`ENGINE:1621`) ignore. An indexer on a `.`-chained segment is parsed with
   `proximal = false` (`ENGINE:1203`), so the operator stays on the proximal
   head and works. HAPI **passes** `testIndexer1`
   `Patient.name[0].given = 'Peter' | 'James'` and `testIndexer2`
   (`oracles/fhirpath/hapi-8.12.0-results.json:281-282`), `testSort9` (`:472`)
   and `testQuantityUnitTextEqual` (`:634`). All of them have an operator after
   an indexer. "An operator that follows an indexer is dropped" (spec
   `filter.002`, DEV-0015 title, HAPI-behaviour and normalizer-impact text)
   would make an implementer fail them. The owner's ruling rests on this
   wording, so DEV-0015 should be re-raised once it is corrected. Required:
   - Restate the rule as proximal-term-only, in both the spec and DEV-0015.
   - Add both-sides examples:
     - `Patient.name[0].given = 'Peter' | 'James'` is `true`;
     - `name[0].given = 'Peter'` (Patient context) yields the given names;
     - `(1 | 2)[0] = 5` yields `1`;
     - `Patient.name.where($this.given[1] = 'James')` is a real comparison.
   - State the answer with the check skipped (`FhirPathR4`,
     `path.check.013`), which is what the differential observes. For
     `Patient.name.where(given[1] = 'James')` the argument is a single
     string, `asBool` answers `True` (`ENGINE:6891-6892`), and the two names
     with a second given name are kept while `name[1]` (`Jim`) is dropped.
     The scenario currently gives only the check-pass answer.

2. **DEV-0014 and DEV-0012 decide one HAPI behaviour two ways.** `skip(-1)`
   (`ENGINE:6106-6107`) and `name[-1]` (`funcItem`, `ENGINE:6280-6281`) both
   end in an unchecked `List.get(-1)` `IndexOutOfBoundsException`. HAPI fails
   `testSkip5` and `testIndexer3` the same way (results `:294`, `:283`). The
   amendment makes `skip(-1)` raise "as an error, not a panic", for HAPI.
   `path.navigation.010` and DEV-0012 still make `[-1]` empty. DEV-0012's
   rationale ("the only non-crashing answer the suite grades is empty") is
   refuted by DEV-0014's own approach. Either align `navigation.010` and
   DEV-0012 with the 2026-11-14 ruling (raise), or state in DEV-0014 why the
   two are decided differently. Otherwise the differential gets one
   unexplained divergence per side. The `## Entries` preamble ("All thirteen
   entries", pending list) also needs to account for DEV-0014 and DEV-0015.

3. **`path.collection.013` re-specifies `distinct` but omits its
   empty-on-incomparable exit.** `funcDistinct` returns an **empty
   collection** as soon as any pairwise `doEquals` is `null`
   (`ENGINE:5668-5670`). That happens for dateTimes of different precision
   (`ENGINE:2193-2194`, `equalsUsingFhirPathRules`) or for incomparable
   quantities (`ENGINE:2191-2192`). It also returns the focus unchanged when
   `size <= 1` (`ENGINE:5660-5661`). Add an AND clause, for example
   `@2012.combine(@2012-01).distinct()` is empty, not two items, marked
   differential-only.

---

Ninth review of `add-fhirpath-and-codegen-r4` (verdict at the time:
APPROVED). It covers commit `0f2a419`, which
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

## Tenth review — commit `d8d783b` (2026-11-14)

The scope is the amendment to `specs/path/spec.md` (11 scenarios) and
DEV-0014/DEV-0015. Every claim was checked against `ENGINE` at `73c432b5` and
against `oracles/fhirpath/hapi-8.12.0-results.json`.

**Verified correct:**

- `collection.013`: `funcDistinct` `ENGINE:5659-5680`, `j = i+1` keeps the
  last occurrence. `Peter, James, Jim, Peter, James` gives
  `Jim, Peter, James`. `testDistinct4` does not grade order.
- `collection.010`, `{}.subsetOf(1 | 2)`:
  - `{}` is a null constant typed with no types (`ENGINE:1332-1334`,
    `3122-3123`);
  - `case SubsetOf` checks it against `focus.toUnordered()` (`ENGINE:3315-3316`);
  - `checkParamTypes` throws `FHIRPATH_WRONG_PARAM_TYPE` (`ENGINE:3828-3830`);
  - `funcSubsetOf` gives `true` on an empty focus (`ENGINE:5360-5363`).
  - No `testSubSetOf` case has an empty focus, so it is differential-only as
    stated.
- `subset.012`: `funcExclude` `ENGINE:5074-5080` does not deduplicate.
  `funcCombine` `ENGINE:5051-5060` concatenates. `opUnion` `ENGINE:2831-2840`
  deduplicates. The new WHEN equals `testExclude4`, which passes in HAPI.
- `combine.006`: `opIn` `ENGINE:2630-2636` is empty for an empty left side
  and `false` for an empty right side. `opContains` `ENGINE:2655-2657` is
  empty when either side is empty.
- `filter.003`: `case Where` `ENGINE:3329-3330` rejects `string` at check.
  `funcWhere` `ENGINE:6249-6260` with `asBool` `ENGINE:6886-6894` keeps all
  three names when the check is skipped.
- `filter.007`: `funcRepeat` `ENGINE:5261-5292` starts from an empty
  `result` and deduplicates with `equalsDeep`. So `repeat(item).repeat(item)`
  holds only items at depth 2 or deeper, and the equality given is `false`.
  This does not conflict with `testRepeat1-5` / `filter.006`.
- `filter.009`: `funcOfType` `ENGINE:5198-5208` raises `FHIRPATH_INVALID_TYPE`
  when `isKnownType` is false. The `System.` list is at `ENGINE:1921-1922`
  (the scenario cites `:1904`, where the function starts; the list matches).
  At check, `checkType` `ENGINE:3767-3786` only warns, so the raise happens
  at evaluation, as the scenario says.
- `subset.006`/`subset.008`: `funcSkip` `ENGINE:6101-6109` and `funcTake`
  `ENGINE:5024-5032` call `n1.get(0)` with no guard. `skip(-1)` reaches
  `get(-1)`. `take(-1)` gives `Math.min(size, -1)` = no iterations, which is
  consistent with `subset.009`. HAPI records `testSkip5/7/8` and
  `testTake8/9` as `failed` with `IndexOutOfBounds` (results `:294-306`).
  The other skip/take cases pass. The revised Subsetting requirement
  sentence is consistent with `subset.007`/`subset.009`.
- DEV-0015's mechanism (operator on `item`, `ENGINE:1188-1213`; proximal-only
  operation at `ENGINE:1508`, and at `ENGINE:1621` for the check) is correct,
  but its scope is not (finding 1).

**Scenario IDs.** Only titles and bodies change. All 11 IDs are kept, none is
renumbered or duplicated, and every cross-reference (`path.check.013`,
`path.errors.003`, `path.oracle.012`, `path.subset.009`) exists.

**Deviation format.** DEV-0014 and DEV-0015 carry every field in the
`docs/deviations.md` entry format. DEV-0015's HAPI-behaviour field is
inaccurate (finding 1). Neither entry has a referencing test yet; the rules
require one before the entry is complete. That is for the oracle-builder.

**Not findings:** `subset.012` cites `testUnion3` (`(1|1).count() = 1`).
`testUnion2` is the closer case, but both grade the deduplication.
