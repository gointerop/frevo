CHANGES_REQUESTED

Thirteenth review (commits `fa4b9be` and `ade70c6`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
and `LEXER` is `FHIRLexer.java` in the same directory, at `73c432b5`. Detail is
in the dated section at the end. The twelfth review was `APPROVED`; earlier
findings follow, kept for the record.

1. **DEV-0016 and `path.navigation.001` misstate the mechanism. HAPI does
   decode the escape, but in base 32.** A backtick name is read by
   `readFixedName` (`ENGINE:1148-1149` calls `LEXER:418-423`), which calls
   `processFixedName` (`LEXER:477`). Its `\u` branch is
   `Integer.parseInt(s.substring(i, i+4), 32)` (`LEXER:509-513`). `006e` in
   base 32 is 6*32 + 14 = 206, which is U+00CE `Î`. That is why the name
   becomes `Îame` and why the oracle reason
   (`hapi-8.12.0-results.json:20`) names `'Îame'`.
   - DEV-0016's heading says "is not decoded". That is false.
   - Its HAPI-behaviour field says "Not diagnosed to a lexer line here". The
     line is `LEXER:511`, which `tasks.md:734` and `docs/deviations.md:625-627`
     already cite.
   - Its chosen-behaviour field says "the escape is not decoded to `n`".
   - `navigation.001` says HAPI "does not decode the escape … to `n`".

   This is not wording. An implementer who reads "not decoded" leaves
   `` `name` `` raw, and the check then reports `'name'`, not
   `'Îame'`. The pass/fail grade agrees, but the error text differs, so the
   differential sees a diff. Frevo's lexer happens to be right
   (`crates/frevo-path/src/syntax/lexer.rs:589`, `unicode_escape(.., 32, ..)`),
   but the spec must state what HAPI does: `\u` in a delimited identifier is
   read as four base-32 digits, so `` `name` `` names `Îame`. Fix the
   DEV-0016 heading, its HAPI-behaviour field and its chosen-behaviour field,
   and the `navigation.001` clause.

2. **DEV-0016 contradicts the "Considered and not recorded as deviations"
   entry for the same defect.** `docs/deviations.md:622-630` records the
   base-32 `processFixedName` as "reproduced as they are", and so not a
   deviation. It also says "None is reachable from an expression that is not
   already pathological". DEV-0016 now records the same `LEXER:511` behaviour
   as a deviation, reached by a pinned suite case
   (`testBasics/testEscapedIdentifierUnicode`). Choose one of these:
   - keep DEV-0016 as the HAPI-vs-specification record, and change the bullet
     to point at it and drop the "not reachable" claim; or
   - withdraw DEV-0016, and correct the bullet to cite the suite case.

   Do not leave both. If you remove DEV-0016, update the `## Entries` preamble
   and `check-registries.sh` counts to match.

3. **`path.navigation.003` names the wrong identifier in the check failure.**
   The check rejects `Observation`, not `value`. In `Observation.value`, the
   Name node `Observation` is evaluated at entry with a non-null focus
   (`Patient`), so it takes the `else` branch at `ENGINE:1587-1589`.
   - In `executeType(String…)` (`ENGINE:3231-3238`), the start-up special case
     needs `hashTail(type)` to equal the name, or `isAncestor` to be true. For
     `Patient` and `Observation` neither holds.
   - `getChildTypesByName(Patient, "Observation")` adds no type.
   - So `ENGINE:1591-1593` throws `FHIRPATH_UNKNOWN_NAME` with
     `exp.getName()` = `Observation`. Evaluation never reaches `value`.

   This is the same route the oracle shows for `Îame`, which is also a name
   unknown on Patient. The scenario says "`value` not valid for any of the
   possible types". A test written from it would assert the wrong name. Change
   it to `Observation`, against types `[http://hl7.org/fhir/StructureDefinition/Patient]`.

## Eleventh-review findings as raised (commits `2f07196`, `5425f6c`)

Findings of the eleventh review (commits `2f07196`, `5425f6c`, 2026-11-17;
detail in the dated section at the end). `ENGINE` is the same file at
`73c432b5`. The tenth review's findings follow, kept for the record.

1. **`path.filter.002` gives the wrong answer with the check skipped, and
   contradicts `path.filter.003`.** The amendment says that
   `Patient.name.where(given[1] = 'James')` with the check skipped "is neither
   true nor false, so no name is kept". `asBool` (`ENGINE:6886-6894`) returns
   `Equality.True` for any single non-boolean item (`:6891-6892`), and
   `funcWhere` (`ENGINE:6249-6260`) keeps the item on `True`. Over
   `patient-example.xml` (`oracles/fhir-test-cases/fetched/r4/examples/patient-example.xml:50-65`),
   `name[0]` (`Peter, James`) and `name[2]` (`Peter, James`) give the single
   string `James`, so both are kept. `name[1]` (`Jim`) gives empty, which is
   `Null`, so it is dropped. The answer is therefore two names, not none.
   `path.filter.003` (`:538-541`) already states the same `asBool` rule
   ("answers `True` for any single non-boolean item"), so the two scenarios
   now disagree. The differential observes the check-skipped answer
   (`FhirPathR4`, `path.check.013`), so a test written from this text would
   encode a diff. Also still missing from tenth-review finding 1: the
   first-term examples that prove the rule on each side. These are
   `name[0].given = 'Peter'` (Patient context), which yields the given names,
   and `(1 | 2)[0] = 5`, which yields `1` because the group's content and the
   indexed group are both a proximal first term (`ENGINE:1135-1138`,
   `1188-1213`). The scenario names "expression, argument or group" but
   exemplifies only the argument case, so the expression and group cases have
   no testable THEN.

2. **Left-over text still contradicts the amendments.**
   - `openspec/changes/add-fhirpath-and-codegen-r4/spec.md:424` (the index)
     still reads "`path.navigation.010` — a negative index yields empty".
   - `spec.md:454` still says that `path.collection.013` "keeps
     first-occurrence order". The scenario title (`specs/path/spec.md:477`)
     and `ENGINE:5665-5676` say last occurrence. This one dates from
     `d8d783b` and the tenth review missed it.
   - `docs/deviations.md:406-408`: DEV-0012's "Specification requirement"
     field still says that "the approved scenario `path.navigation.010` states
     empty for a negative index", which is no longer true.
   - The `## Entries` preamble (`docs/deviations.md:48`) still says "All
     thirteen entries". There are fifteen (`check-registries.sh`: 15). It
     names DEV-0013 as the only pending entry, although DEV-0015 is now
     "pending owner re-confirmation" (`:510-511`). This was part of
     tenth-review finding 2 and is unresolved.
   - `tasks.md:779` still lists DEV-0012 as "pending the owner".

---

## Tenth-review findings as raised (commit `d8d783b`)

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

## Eleventh review — commits `2f07196` and `5425f6c` (2026-11-17)

The scope is `specs/path/spec.md` and `docs/deviations.md` from `e6e6a96`
through `5425f6c`; no other file changed. Every claim was checked against
`ENGINE` at `73c432b5`, `Utilities.java:83-100`, and
`oracles/fhirpath/hapi-8.12.0-results.json`.

**Tenth-review finding 1 (DEV-0015 scope): partly resolved.**
- The rule is now proximal-first-term-only in both the spec and DEV-0015, and
  the mechanism is correct. A group's content is parsed proximal
  (`ENGINE:1138`). The indexer moves `focus` to `item` (`:1188-1197`). A
  `.`-chained segment is parsed with `proximal = false` (`:1201-1203`), so its
  indexer is never followed by the operator loop. The loop runs only when
  `proximal` (`:1205-1213`) and attaches the operator to `focus`. `execute`
  applies the operation only on a proximal node (`:1508`).
- The later-term counter-examples (`testIndexer1/2`, `testSort9`,
  `testQuantityUnitTextEqual`) are stated, and this agrees with
  `path.navigation.008`. DEV-0015 is correctly re-marked as pending the
  owner's re-confirmation.
- Still open: the check-skipped answer is wrong, and the expression and group
  examples are missing (new finding 1).

**Tenth-review finding 2 (negative index): resolved in substance.**
- `path.navigation.010` now raises. This is correct: `funcItem`
  (`ENGINE:6277-6283`) guards only the upper bound, and `Utilities.isInteger`
  strips a leading `-` (`Utilities.java:87`), so `focus.get(-1)` is reached.
  HAPI records `testIndexer3` as failed with "Index -1 out of bounds for
  length 3" (results `:283`), the same form as `testSkip5` (`:294`).
- DEV-0012 now matches DEV-0014 and `path.subset.006`: an error, not a panic
  (`path.errors.003`), and parity under `path.oracle.012`. The Subsetting
  requirement sentence (`specs/path/spec.md:595`) and `subset.006/.008/.009`
  remain consistent.
- Left-over text is new finding 2.

**Tenth-review finding 3 (distinct): resolved.** The null-equality exit
(`ENGINE:5668-5670`) is stated and marked differential-only. The
`size <= 1` early return (`:5660-5661`) is not observable separately, because
a single item has no pair to compare, so it needs no clause.

**Scenario IDs.** Only the title of `path.navigation.010` changes. No ID is
added, removed, renumbered or duplicated.

**Registers.** `harness/scripts/check-registries.sh` reports "registers agree"
(15 deviations, 12 debt entries).

## Twelfth review — commit `305d4df` (2026-11-17)

This commit changes `specs/path/spec.md`, the `spec.md` index, `tasks.md` and
`docs/deviations.md`. I checked each claim against `ENGINE` at `73c432b5`.

**Eleventh-review finding 1: resolved.**
- `path.filter.002`, check skipped, now yields two names:
  - `asBool` returns `True` for a single non-boolean item (`ENGINE:6891-6892`);
  - `funcWhere` keeps an item on `True` (`:6255-6257`);
  - `name[1]` has no second given name, so it is empty and `Null`
    (`:6888-6889`) and is dropped;
  - this matches the fixture (`patient-example.xml:50-65`) and agrees with
    `path.filter.003`.
- The new examples are correct:
  - `name[0].given = 'Peter'`: `name` is the proximal first term. The indexer
    moves `focus` to `item` (`:1188-1197`). `.given` becomes `item`'s inner,
    parsed non-proximal (`:1201-1203`). The operator goes on `item`
    (`:1206-1213`), which `execute` ignores (`:1508`). So the result is
    `Peter, James`.
  - `(1 | 2)[0] = 5`: the group's content is a proximal parse (`:1138`), so
    the union works. The indexer on the group then drops `= 5`, giving `1`.
  - `$this.given[1] = 'James'`: `$this` has no indexer, so `focus` stays the
    proximal head. `.given[1]` is parsed non-proximal, and the operator
    attaches to `$this`, so this is a real comparison.
- Each example has a concrete WHEN and THEN.

**Eleventh-review finding 2: resolved.**
- `spec.md:424` and `:454` now match `navigation.010` and `collection.013`.
- DEV-0012's specification-requirement field is corrected.
- The `## Entries` preamble says fifteen and names DEV-0013 and DEV-0015 as
  pending. `check-registries.sh` reports "registers agree" (15 deviations,
  12 debt entries).
- `tasks.md:779` is updated.
- DEV-0015's normalizer-impact text is narrowed to the first term.

**Grep for other stale statements.** I searched the change directory, `docs/`
and `tasks/blockers.md` for negative index, first/last occurrence, and an
operator after or following an indexer. The only remaining hits are:
- `tasks/blockers.md:1281`, `:1562` and `:1578`. These are dated historical
  log entries, not requirements.
- DEV-0015's heading (`docs/deviations.md:483`), which still reads "an
  operator that follows an indexer is silently dropped". Its body states the
  first-term scope precisely, so this is wording, not a finding. The owner's
  re-confirmation should be read against the body.

**Scenario IDs.** No scenario heading changed in `305d4df`. No ID is added,
removed, renumbered or duplicated.

**Notes for the owner (not findings).**
- DEV-0015 and DEV-0013 remain pending owner review.
- DEV-0014 and DEV-0015 still have no referencing test. That is the
  oracle-builder's work.

## Thirteenth review — commits `fa4b9be` and `ade70c6` (2026-11-17)

I checked every claim against `ENGINE`/`LEXER` at `73c432b5` and against
`oracles/fhirpath/hapi-8.12.0-results.json`.

**`fa4b9be`, `path.navigation.008`: correct.** The `testIndexer` group has
three cases (`tests-fhir-r4.xml:567-570`). The results file has `testIndexer1`
and `testIndexer2` `passed` (`:281-282`), and `testIndexer3` `failed` with
"Index -1 out of bounds for length 3" (`:283`). This agrees with
`navigation.010` and DEV-0012.

**`navigation.001` and DEV-0016: wrong mechanism (finding 1) and a
contradiction with the register (finding 2).** I traced the check-stage
failure. `name` is the inner of `Patient`, so it is typed by
`ENGINE:1618`. That call passes `canBeNone` = `false` as a literal,
whatever the surface. The focus is `Patient`, and
`getChildTypesByName(Patient, "Îame")` is empty, so `:1591-1593` throws. The
outcome (`failed` at check on both sides) and the quoted message are right.
The cause is base-32 decoding (`LEXER:511`). The escape is decoded; the
claim that it is not is wrong.

**`navigation.003`: outcome right, name wrong (finding 3).**
`testSimpleWithWrongContext` (`Encounter.name.given`,
`invalid="semantic"`) is `passed` for HAPI (`results:24`), which is consistent.

**`navigation.011`: correct.**
- `case Item` (`ENGINE:3363-3367`) calls `checkOrdered`. That passes, because
  `name` is not UNORDERED.
- It then calls `checkParamTypes` against a singleton `integer`. The
  `string` argument raises `FHIRPATH_WRONG_PARAM_TYPE`.
- The scenario cites `:3828`. The method starts at `:3820` and the `throw` is
  at `:3830`. This is a citation nit, not a finding.

**`navigation.019`: correct, including `canBeNone`.**
- `{}` parses to a `null` constant (`ENGINE:1332-1334`).
- `resolveConstantType(null)` returns `new TypeDetails(SINGLETON)`, which has
  no types (`ENGINE:3122-3123`).
- `.name` is the inner, typed at `ENGINE:1618`, where `canBeNone` is
  hard-coded `false`. So no check surface can make it true for `.name` here.
- `focus.getTypes()` is empty, the result has no types, and `:1591-1593`
  throws `FHIRPATH_UNKNOWN_NAME`.
- The scenario's "with `canBeNone` false" holds. `:1618` is the more exact
  citation.

**Scenario IDs.** No `#### Scenario:` heading changed in either commit. No ID
is added, removed, renumbered or duplicated.

**DEV format and registers.** DEV-0016 has every field the other entries have.
The preamble now says sixteen and lists DEV-0016 as pending.
`check-registries.sh` reports "registers agree" (16 deviations, 12 debt
entries).

**Stale text.**
- `tasks.md:769` ("`path.navigation.001` through `.024`") is still accurate.
- `tasks/blockers.md:1641-1647` is a dated log. Its item 3 labels
  `{}.name.given` as `.004`. It is `.019`. That is historical, not a
  requirement.
- No other navigation, lexer or escaped-identifier text in the change
  directory contradicts the amendments. The exception is
  `docs/deviations.md:622-630` (finding 2).

**Note (not a finding).** `crates/frevo-path/tests/zz_tmp_verify.rs` is
untracked in this worktree. It is not part of either commit.
