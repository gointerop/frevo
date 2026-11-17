APPROVED

Twentieth review (commit `2419520`, 2026-11-17) resolves all three findings of
the nineteenth review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Nineteenth-review findings as raised (commit `fd47948`)

Nineteenth review (commit `fd47948`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at `73c432b5`. DEV-0019 and
the `math.012`, `mathfn.010`, `string.004` and `string.023` amendments are
correct. The three findings below are all in DEV-0020 and the scenarios it
touches. Detail is in the dated section at the end. The eighteenth review was
`APPROVED`; earlier findings follow, kept for the record.

1. **DEV-0020's chosen behaviour departs from HAPI on a false premise. The
   spec should require HAPI's behaviour.**
   - Constitution section 1: HAPI 8.12.0 defines correct. Where it disagrees
     with the specification, "choose HAPI's behaviour". `docs/deviations.md`
     is the only exception mechanism. So recording a Frevo departure there is
     the right *place*. But the entry chooses the specification's answer
     against HAPI, and both of its reasons for doing so are wrong.
   - **`toChars`.** HAPI iterates `s.toCharArray()` (`ENGINE:5835-5837`) and
     yields one `StringType` per UTF-16 unit. "A lone surrogate cannot be a
     Rust `String`" is true of `String`, but it is not a limit of the engine.
     A string value carried as UTF-16 (`Vec<u16>`), as WTF-8, or as a
     dedicated item variant for an unpaired unit holds it exactly. That is a
     representation choice, and HAPI's observable output is matchable.
   - **Exponent rendering.**
     - `new DecimalType(double)` is `setValue(BigDecimal.valueOf(d))`
       (`DecimalType.java:66-69`), and `encode` is
       `getValue().toString()` (`:101-102`). So `abs`, `sqrt`, `ceiling`,
       `floor`, `exp`, `ln`, `log` and `power` (`ENGINE:4104`, `:4124`, …)
       render `1.0E+10`. `round` renders `1.23E+4` for `12345.round(-2)`,
       because `setScale` takes a negative scale (`:4411-4412`).
     - `BigDecimal.toString` is a fixed function of the unscaled value and the
       scale: scientific when the scale is below zero or the adjusted exponent
       is below -6.
     - Frevo's decimal already carries a mantissa and a scale (`design.md`
       section 6; `DEBT-0011`). So "its decimal type carries no exponent" is
       not an obstacle to producing the same string.
     - The rendering is observable through `toString()`, implicit string
       conversion and the serialized result. "Changes the string and not the
       value" is the very difference a differential sees.
   - **My view.** Require HAPI's behaviour in the spec: `toChars` per UTF-16
     unit, and Java `BigDecimal.toString` rendering. Withdraw DEV-0020, or
     reduce it to whatever residue the owner explicitly accepts.
   - If the owner does accept a departure, "Debt: none" is wrong. The entry's
     own rationale names what would then be owed ("an exponent-capable decimal
     rendering is due"). That belongs in `docs/tech-debt.md` with its trigger.

2. **`substring` can split a surrogate pair, and no scenario or entry says
   so.** `path.string.022` now requires UTF-16 counting, which matches HAPI.
   `funcSubstring` slices with `String.substring` (`ENGINE:5884-5886`), so
   `'😀x'.substring(1)` or `'😀'.substring(0, 1)` yields a lone surrogate
   in HAPI. That is the same unpaired-unit case as `toChars`. Neither DEV-0020
   nor `string.022` (which says only that a BMP character "is never split")
   states the outcome for a supplementary character. Whatever finding 1
   resolves to, state the `substring` result for a supplementary character,
   with a WHEN and a THEN.

3. **The exponent rendering is in no scenario.** DEV-0020 lists
   `path.mathfn.004`, but `mathfn.004` (`spec.md:1286-1289`) says only "every
   `testRound` case yields its stated output". No suite case exercises an
   exponent. So, whichever rendering is chosen, a tester has no concrete
   input or output. Add a differential-only scenario with concrete cases, for
   example `12345.round(-2)` → `1.23E+4` and `(1e20).abs()`-style
   double-derived values → `1.0E+20`, each stated as the string HAPI
   produces. Also correct DEV-0020's scenario-ID field to name it.

## Eighteenth review header as issued (commit `c35959a`)

Eighteenth review (commit `c35959a`, 2026-11-17) resolves the one finding of
the seventeenth review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Seventeenth-review findings as raised (commit `9fea32e`)

Seventeenth review (commit `9fea32e`, 2026-11-17). All six findings of the
sixteenth review are resolved in the scenario text, and the two new scenarios
are justified. What remains is that the new IDs are not wired into the
change's index or its task graph. Detail is in the dated section at the end.
The earlier findings follow, kept for the record.

1. **`path.compare.013` and `.014` belong to no task, and the change index
   does not list them.**
   - `tasks.md:812-813` says task "Satisfies ... `path.compare.001` through
     `path.compare.012`". No task claims the two new IDs, so no implementer
     is assigned the behaviour they specify.
   - The `spec.md` count table still reads `path.compare.*` | 12 |
     `001`-`012` (`spec.md:96`), with a **Total** of 493 (`spec.md:13`). It should
     read 14 and 495.
   - The scenario list (`spec.md:512-523`) has no entries for `013` and `014`.
   - The same list still gives the pre-`bdfc6dd` title for `path.math.008`
     (`spec.md:547`, "division by zero yields empty"). That title is the
     behaviour `bdfc6dd` reversed for `0.0 / 0`, `0 div 0.0`, `0.0 mod 0`,
     `5.5 mod 0` and `5 mod 0.0`, so a reader of the index gets the wrong
     answer.
   - Extend the `Satisfies` range at `tasks.md:812-813`, and correct the
     table, the total and the list.

## Sixteenth-review findings as raised (commit `bdfc6dd`)

Sixteenth review (commit `bdfc6dd`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` and `BDT` is
`org.hl7.fhir.r4/.../model/BaseDateTimeType.java`, both at `73c432b5`. Detail
is in the dated section at the end. The fifteenth review was `APPROVED`; the
earlier findings follow, kept for the record.

1. **`path.compare.001` is truncated, and two of its mechanisms are wrong.**
   - **The sentence is cut off.** `spec.md:896` ends "and `testLessThan33`,
     which raises", with no outcome and no cause.
     - The cause: `opLessThan` compares `code` (`ENGINE:2438-2440`). The
       unit-text-only quantity has no code and the coded one has one, so the
       lists differ.
     - `qtyToCanonicalDecimal` then returns `null` for the quantity with no
       UCUM system (`ENGINE:2302-2304`). The recursive call dereferences it
       (`:2413`): an NPE on `isPrimitive()` (`results:631`).
   - **`testLessThan29` (`1 year < 2 months`) does not reach the
     canonical-decimal branch.** A calendar-duration literal sets `unit` and
     no `code` (`ENGINE:1131`). So both `code` lists are empty,
     `compareDeep` is true, and the raw values are compared: `1 < 2` gives
     `true`. That is one item, where the suite expects none.
     `testLessThan30` (`kg` against `m`) is the canonical-decimal case, and
     answers `false`.
   - **`testLessThan28` is wrongly grouped** under "yields one item where the
     suite expects none". It expects `true` and gets `false`.
   - **A behaviour is missing: ordered comparison mutates its operand.**
     `compareDateTimeElements` calls `setTimeZoneZulu(true)` on the operand
     itself when it is already a `DateTimeType` (`ENGINE:636`, `:643-648`).
     `setTimeZoneZulu` rewrites the stored string (`BDT:690-694`,
     `updateStringValue`).
     - So after `value < @1975-...Z`, the same element's `toString()` is the
       Zulu form, not `1974-12-25T14:35:45-05:00`. That is why the second
       conjunct of `testLessThan28` is `false`.
     - The results file does not say which conjunct failed. The spec's "HAPI's
       recorded outcome is that the second conjunct is false" is an inference,
       and this is its source.
     - The mutation is observable to every later read of the element in the
       same evaluation. `<`, `>`, `<=` and `>=` all reach it through their
       date branches. Add a scenario stating it.

2. **`path.compare.002` and `.004` give the wrong mechanism for
   `testLessOrEqual29` and `testGreatorOrEqual29`. The scenarios also leave
   each case's outcome unstated.**
   - A UCUM-quoted literal (`1 'kg'`) sets `code` and `system` and leaves
     `unit` null (`ENGINE:1131`). `<=`, `>` and `>=` compare `unit`
     (`:2533-2537`, `:2485-2487`, `:2583-2585`), so null against null is
     "equal" and the raw values are compared.
     - `1 'kg' <= 999 'g'` gives `true` (`results:668`).
     - `2 'kg' >= 1 'm'` gives `true` (`results:698`).
     - Neither reaches `qtyToCanonicalDecimal`.
   - The NPE route applies to the others: `testLessOrEqual28`,
     `testGreaterThan28`/`29` and `testGreatorOrEqual28` (`results:667`,
     `726`, `727`, `697`).
   - "raises, or answers a different boolean" does not let a tester tell
     which case does which. State the outcome per case.
   - **Missing behaviour:** `opLessOrEqual` evaluates `lunit.equals(runit)`
     when the left unit is null and the right is not (`ENGINE:2537`). For
     example, `1 'kg' <= 1 year` raises an NPE on `String.equals`. That is a
     distinct failure, and differential-only. It needs a scenario.

3. **The error message is wrong for four cases, and the `copy()` citation is
   off.**
   - `DateTimeType.copy()` (`DateTimeType.java:164-165`) re-parses through
     `new DateTimeType(String)`. What that raises depends on the offset.
     - With no offset, parsing fails with `Invalid date/time format`
       (`BDT:797`).
     - With an offset, parsing succeeds and
       `validatePrecisionAndThrowIllegalArgumentException` raises "Invalid
       date/time string (datatype DateTimeType does not support MINUTE
       precision)" (`BDT:103-106`).
   - So `testEquality40` (`results:513`), `testPlusDate23`/`24` (`:823-824`)
     and `testMinus7` (`:835`) fail with the second message.
     `path.equality.001` and `path.math.001` say `Invalid date/time format` for
     all of them, and DEV-0017 repeats it.
   - The `copy()` call in `equalsUsingFhirPathRules` is `BDT:924`, not
     `:916`. `dateAdd` calls it at `ENGINE:2716`.
   - State both messages and which input gives which.

4. **DEV-0018 covers equality only. Ordered comparison of an offset-less
   dateTime is zone-dependent too.**
   - `BDT.parse` places an offset-less value in `TimeZone.getDefault()`
     (`BDT:434`).
   - `compareDateTimeElements` then converts both operands to Zulu when the
     precision is finer than DAY (`ENGINE:643-648`). `compareTimes` compares
     the Zulu fields (`BDT:993ff`).
   - So `@2015-02-04T10:00:00 < @2015-02-04T05:30:00Z` depends on the
     machine's zone.
   - DEV-0018's scenario list (`path.datetime.004`) and its "`=` of an
     offset-less and an offset `dateTime`" scope both miss this. Extend the
     entry to `<`, `>`, `<=` and `>=`, and say what Frevo does.

5. **`path.math.003` leaves `0.0 - 5 'mg'` unspecified.** "is not matched"
   does not give an outcome. The left's string is `0.0`, so the guard at
   `ENGINE:2953` fails and `:2954` raises `FHIRPATH_RIGHT_VALUE_WRONG_TYPE`,
   just as `5 - 3 'mg'` does. Say so.

6. **The `docs/deviations.md` preamble is stale.**
   - `:48-49` still says "All fifteen entries". `check-registries.sh` counts
     17.
   - It names only DEV-0013 and DEV-0015 as pending. DEV-0017 and DEV-0018
     are both "pending owner".

## Fifteenth review header as issued (commits `16d970c`, `eb10763`)

Fifteenth review (commits `16d970c` and `eb10763`, 2026-11-17) resolves all
three findings of the fourteenth review. Detail is in the dated section at the
end. The earlier findings follow, kept for the record.

## Fourteenth-review findings as raised (commit `cbe278c`)

Fourteenth review (commit `cbe278c`, 2026-11-17). Thirteenth-review finding 3
is resolved. Findings 1 and 2 are only partly resolved, and the same area has
one new defect. Detail is in the dated section at the end. The earlier findings
follow, kept for the record.

1. **The escaped expression is printed without its escape.** The pinned case is
   ``Patient.`name`.given.first()`` (`oracles/fhirpath/tests-fhir-r4.xml:100`).
   Two places print it wrongly:
   - `specs/path/spec.md:250` writes ``Patient.`name`.given.first()``;
   - `docs/deviations.md:602` writes "``Patient.`name` `` looks up `Îame`".

   Both read `` `name` ``, with no `n`. As written, the claim is false:
   `` `name` `` has no escape, so it looks up `name` and yields `Peter`. A test
   written from the scenario text would therefore assert a check failure for
   an expression that passes. Write `` `name` `` in both places.

2. **The register bullet still says the defect is unreachable
   (thirteenth-review finding 2, partly resolved).** `docs/deviations.md:605-606`
   still reads "None is reachable from an expression that is not already
   pathological". The same bullet now cites a pinned suite case that reaches
   it. A `\u` escape in a delimited identifier is the specification's own
   example, not a pathological input. Narrow the sentence to the other two
   decoders, or drop it.

3. **A test comment still cites the withdrawn DEV-0016 and the wrong
   mechanism.** `crates/frevo-path/tests/path_navigation.rs:487-489` reads
   "is NOT a pass (DEV-0016): HAPI does not decode the escape to `n`". DEV-0016
   no longer exists, and HAPI does decode the escape, in base 32
   (`FHIRLexer.java:511`). The assertion at `:491` (`"Îame"`) is right. Only
   the comment is stale, but it names a register entry that does not exist,
   which is the dangling reference `check-registries.sh` exists to prevent.
   This is in a test file, so the oracle-builder must fix it.

## Thirteenth-review findings as raised (commits `fa4b9be`, `ade70c6`)

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

## Fourteenth review — commit `cbe278c` (2026-11-17)

**Thirteenth-review finding 1: resolved in the spec, but not in the test
comment (new finding 3).** `navigation.001` now states the base-32 decode
(`FHIRLexer.java:511`, `parseInt(..., 32)`), which I verified at `73c432b5`.
`006e` in base 32 is 206, which is U+00CE. It also states the check-stage
failure, and gives parity under `path.oracle.012`. The expression text is
wrong (new finding 1).

**Thirteenth-review finding 2: partly resolved (finding 2).** DEV-0016 is
withdrawn, and the "Considered and not recorded" bullet now cites the suite
case. Withdrawal fits constitution section 1, because that section of
`docs/deviations.md` records the HAPI-vs-specification difference. The bullet's
"not reachable" sentence remains.

**Thirteenth-review finding 3: resolved.** `navigation.003` now names
`Observation`, through `executeType` (`ENGINE:3231-3238`) and `:1591-1593`.
I re-verified this against the pin.

**`navigation.011`: resolved.** The citation is now `:3830`, which is the
`throw`.

**Stale-text grep** (change directory, `docs/`, `tasks/`, `crates/frevo-path`):
- `DEV-0016` appears only at `crates/frevo-path/tests/path_navigation.rs:487`
  (finding 3).
- "does not decode" appears in that same comment and nowhere else.
- "`value` not valid" appears nowhere.
- The deviations preamble says fifteen and lists DEV-0013 and DEV-0015 as
  pending.

**Note (not a finding).** `:3828` remains as a `checkParamTypes` citation in
three places. The `throw` is at `:3830`.
- `specs/path/spec.md:492` (`subsetOf`);
- `crates/frevo-path/tests/path_collection.rs:479`;
- `crates/frevo-path/tests/path_navigation.rs:766`.

**Scenario IDs.** No `#### Scenario:` heading changed in `cbe278c`.

**Registers.** `check-registries.sh` reports "registers agree" (15
deviations, 12 debt entries).

## Fifteenth review — commits `16d970c` and `eb10763` (2026-11-17)

**Fourteenth-review finding 1: resolved.** I compared the bytes with `od -c`:

- `specs/path/spec.md:250` contains ``Patient.`name`.given.first()``.
- `docs/deviations.md:603` contains ``Patient.`name` ``.
- Both are the literal sequence backslash, `u`, `0`, `0`, `6`, `e`, with no
  decoded character in place of the escape.
- This is byte-identical to `oracles/fhirpath/tests-fhir-r4.xml:100`.

**Fourteenth-review finding 2: resolved.** The "not reachable" sentence is
replaced.

- The new text says the `processFixedName` base-32 defect is reachable and
  observable, decoding `n` as U+00CE. This matches `FHIRLexer.java:511`
  at `73c432b5`.
- It confines the claim of non-reachability to the other two defects: the
  quantity-unit missing advance, and the missing `` \` `` escape in a
  delimited name. No real expression reaches either.

**Fourteenth-review finding 3: resolved.**
`crates/frevo-path/tests/path_navigation.rs:487-490` no longer cites DEV-0016.
It states the base-32 decode (`FHIRLexer.java:511`, `006e` -> U+00CE). The
assertion is unchanged.

**Stale-text grep.** I searched the change directory, `docs/`,
`crates/frevo-path` and `tasks/graph.yaml` for `DEV-0016`, "does not decode"
and "already pathological". There are no hits outside this file. The hits for
"not reachable" in `tasks.md` are unrelated: they describe task reachability
in codegen.

**Registers.** `check-registries.sh` reports "registers agree" (15
deviations, 12 debt entries).

**Coverage since `a53fc9a`.** Four commits have touched the spec delta or the
registers since the last `APPROVED`: `fa4b9be`, `ade70c6`, `cbe278c` and
`16d970c`. Their combined diff touches only `specs/path/spec.md` (39 lines),
`docs/deviations.md` and this file. Every hunk falls in `navigation.001`,
`.003`, `.008`, `.011` or `.019`, or in the DEV-0016 entry and the escape
bullet. The thirteenth to fifteenth reviews cover all of them. `tasks.md`,
`design.md`, `proposal.md` and the other spec files are unchanged since
`a53fc9a`. Nothing is left unreviewed.

**Note (not a finding).** The `:3828` `checkParamTypes` citation nits from the
fourteenth review's note remain.

## Sixteenth review - commit `bdfc6dd` (2026-11-17)

**Per-group failed-case lists: complete and correct.** I extracted every
non-`passed` row of `hapi-8.12.0-results.json` for the 16 amended groups. There
are exactly 38. Each case the spec names is `failed` there, and no failed case
in those groups is missing:

| Group | Failed cases |
|---|---|
| Equality | 31, 32, 33, 35, 37, 38, 39, 40 |
| NEquality | 27 |
| HasValue | `testHasCodingValue2` |
| Equivalent | 24-30 |
| NotEquivalent | 23, 24 |
| LessThan | 28, 29, 30, 33 |
| LessOrEqual | 28, 29 |
| GreaterThan | 28, 29 |
| GreatorOrEqual | 28, 29 |
| Union | 12 |
| Plus | `testPlusDate23`, `24` |
| Minus | 7 |
| Divide | 7 |
| Div | 6 |
| Mod | 6, 7, 8 |
| Iif | none (all 11 pass) |

`testQuantityUnitComparison` (5 failed) is not amended by this commit. Its
existing scenarios (`spec.md:1632-1660`) were not re-checked here.

**Verified against the source.**
- **`removeTrailingZeros`** (`ENGINE:2161-2178`). `"100"` becomes `"1"` and
  `"1.0"` becomes `"1"`. `"10.0"` becomes `"10"`, because the strip stops at
  the first non-zero once the dot is seen. So `100 = 1.0` and `10 = 1.0` are
  `true` and `10 = 10.0` is `false`, as stated. `doEquals` takes the decimal
  path when either side is a `DecimalType`.
- **`doEquivalent`** unboxes `qtyEquivalent`'s `Boolean` (`:2204-2206`), and
  `qtyEquivalent` can return `null`. The NPE text in `results:568-573` names
  `qtyEquivalent`. Correct.
- **`funcHasValue`** (`:5776-5784`): `!noString(convertToString(...))`.
  Correct, consistent with `results:542`.
- **`iif` check** (`:3441-3443`): `checkSingleton(focus, ...)` dereferences
  `focus.getCollectionStatus()` (`:3839-3840`). A `null` context gives
  `types = null` (`:425-426`), so the NPE follows. All 11 `testIif` cases have
  `inputfile` (`tests-fhir-r4.xml:612-651`), which is why HAPI passes them.
  `logic.008` to `.010`: correct.
- **`opMinus`.** The right-hand guard is at `:2953`. The `0 - q` branch
  (`:2965-2970`) adds `qty.copy().setValue(abs)` only when the left's string
  is `"0"`. `q - q` takes the same branch, its `primitiveValue()` is not
  `"0"`, and the result is empty. Correct, apart from finding 5.
- **Divide, `div`, `mod`.**
  - `/` swallows `UcumException` (`:3005-3011`).
  - Integer `div` guards a zero divisor (`:3050-3054`). Decimal `div` swallows
    the exception.
  - Integer `mod` guards zero (`:3090-3094`). Decimal `mod` rethrows as
    `PathEngineException` (`:3102`).
  - These match `results:845`, `851` and `857-859`. Correct.
  - The claim that `Decimal.divide` "returns zero for a zero dividend before it
    checks the divisor" cannot be verified from `reference/`. The observable
    part (one item) is backed by the oracle, and the value is deferred to the
    new blocker. I accept that as recorded, not as a finding.
- **`equalsUsingFhirPathRules` and `couldBeTheSameTime`** (`BDT:916-957`).
  - The +-14 h window and the false/null outcome are as stated.
  - An offset-less value's `getValue().getTime()` comes from a parse in the
    default zone (`BDT:434`). `setValue` falls back to the default
    (`:721-723`).
  - The equality half of DEV-0018 is correct. Finding 4 covers the ordering
    half.

**Scenario IDs.** Only the title of `path.math.008` changed. No ID is added,
removed, renumbered or duplicated.

**DEV format.** DEV-0017 and DEV-0018 carry every field. DEV-0018 is a real
choice that differs from HAPI (a configured zone in place of the JVM default),
so it is correctly an entry. `check-registries.sh` reports "registers agree"
(17 deviations, 12 debt entries). The preamble is stale (finding 6).

**Stale `:3828` citations (flagged, not fixed: this role writes only
`review.md`).** `checkParamTypes` begins at `ENGINE:3820` and throws at
`:3830`. Three places still cite `:3828`:
- `specs/path/spec.md:492` (`path.collection`, `subsetOf`): for the proposer;
- `crates/frevo-path/tests/path_collection.rs:479`: for the oracle-builder;
- `crates/frevo-path/tests/path_navigation.rs:766`: for the oracle-builder.

These are citation errors, not behaviour.

**Blocker entry.** The `tasks/blockers.md` entry (2026-11-17,
`org.fhir.ucum.Decimal`) cites accurate lines and states the decision needed.

## Seventeenth review - commit `9fea32e` (2026-11-17)

I checked each change against `FHIRPathEngine.java` (`ENGINE`) and
`BaseDateTimeType.java` (`BDT`) at `73c432b5`, and against
`hapi-8.12.0-results.json`.

**Sixteenth-review finding 1: resolved.**
- `compare.001` now gives each failed case its own outcome.
  - `testLessThan29`: equal empty `code` lists (`ENGINE:1131`, `:2438-2440`)
    lead to a raw `1 < 2`, which is `true`. Matches `results:627`.
  - `testLessThan30`: the codes differ, so canonical decimals are compared
    (`:2447-2450`), giving `false`. Matches `results:628`.
  - `testLessThan33`: an NPE on `isPrimitive()`. Matches `results:631`. The
    dereference is at `:2413`, not `:2414` (a citation nit).
- The new `path.compare.013` states the in-place Zulu rewrite
  (`ENGINE:636-648`; `BDT:690-694`). Its THEN is right: the birth-time value
  (`-05:00`, second precision) is a `DateTimeType`, so it is mutated, and
  `1974-12-25T19:35:45Z < 1975-01-01T00:00:00Z` is `true`.
- **Justified and non-duplicative.** No earlier scenario states the mutation.
  `compare.001` now only refers to it.

**Sixteenth-review finding 2: resolved.**
- `testLessOrEqual29` and `testGreatorOrEqual29` now give the null-`unit`
  raw-value mechanism (`ENGINE:1131`, `:2533-2537`, `:2583-2585`) and the
  `true` outcome. Matches `results:668` and `698`.
- `LessOrEqual28`, `GreaterThan28`/`29` and `GreatorOrEqual28` now give the
  NPE (`:2301-2304`, `:2494-2496`, `:2544-2546`, `:2592-2594`). Matches
  `results:667`, `726`, `727` and `697`.
- The new `path.compare.014` (`1 'kg' <= 1 year`) is right: `lunit` is null and
  `runit` is `"year"`, so `(null && null)` is false and `lunit.equals` raises
  (`:2537`).
- **Justified and non-duplicative.** Only `opLessOrEqual` has this form.
  `<`, `>` and `>=` use `Base.compareDeep` (`:2440`, `:2487`, `:2586`), so the
  scenario is specific to `<=`. `compare.002` refers to it rather than
  restating it.

**Sixteenth-review finding 3: resolved.**
- The two messages are now given: `BDT:797` with no offset, and `BDT:103-106`
  with an offset.
- `copy()` is cited correctly in `equality.001`, `math.001` and DEV-0017:
  `DateTimeType.java:164-165`, called at `BDT:924` and `ENGINE:2716`.

**Sixteenth-review finding 4: resolved.**
- DEV-0018 and `datetime.004` now cover `<`, `>`, `<=` and `>=` through
  `ENGINE:643-648`.
- The scenario-ID field stays `path.datetime.004`, which is where the
  behaviour is stated, so that is consistent.

**Sixteenth-review finding 5: resolved.** `0.0 - 5 'mg'` now raises
the right-value wrong-type error (`ENGINE:2953-2954`).

**Sixteenth-review finding 6: resolved.**
- The preamble says seventeen and lists DEV-0013, DEV-0015, DEV-0017 and
  DEV-0018 as pending.
- `check-registries.sh` reports "registers agree" (17 deviations, 12 debt
  entries).
- DEV-0017's scenario list adds `compare.013` and `.014`.

**Scenario IDs.** `9fea32e` adds two headings, `path.compare.013` and `.014`.
Neither collides, and no ID in `specs/` is duplicated. No existing heading
changed. The stale index and task range are finding 1.

**Stale-text grep.** No hits remain in `specs/path/spec.md` or
`docs/deviations.md` for any of the following: "raises, or answers",
`BaseDateTimeType.java:916`, `java:2440` as the sole `<` mechanism, and
"is not matched". `spec.md:492` now cites `:3830`. The two test-file `:3828`
citations are being fixed by the builder, as instructed, and were not
re-checked.

**Note for the owner (not a finding).** The mutation in `compare.013` is not
confined to one evaluation in HAPI. `compareDateTimeElements` rewrites the
model object itself. So a `<`, `>`, `<=` or `>=` over a resource's dateTime,
inside an invariant or a search expression, leaves the in-memory resource in
Zulu form for whatever reads it next. Frevo serves stored bytes (constitution
section 6), so it cannot reproduce that past the evaluation. The spec's
"in the same evaluation" is the right scope for Frevo. Whether HAPI's
in-memory rewrite ever reaches a response body is a question for the
differential harness, not this spec.

## Eighteenth review - commit `c35959a` (2026-11-17)

**Seventeenth-review finding 1: resolved.**
- `tasks.md:812-813` (task 35) now satisfies `path.compare.001` through `.014`.
- Task 4's two counts (`tasks.md:118`, `:123`) read 363.
- The `spec.md` index is updated in four places:
  - the `path.compare.*` row reads 14, `001`-`014`;
  - the scenario list has `013` and `014`;
  - `path.math.008` carries its current title;
  - the totals read `path` 363 and **Total** 495.

**Counts, verified independently.**
- **Headings in this change.** Counting `#### Scenario: [` headings in the
  change's delta specs gives `codegen` 49, `model` 83 and `path` 363, which is
  495. No ID is duplicated.
- **Index cross-check.** A script compared every `path.<area>.*` row of the
  index table with the declared IDs: count, first, last and contiguity. All 27
  areas match. The index's ID list equals the declared set exactly: none
  missing, none extra.
- **The 495-before / 497-now question.** With no argument,
  `harness/scripts/scenario-coverage.sh` declares from both `openspec/specs`
  and `openspec/changes` (script lines 41-46). `openspec/specs/server/spec.md`
  holds two settled scenarios, `server.metadata.001` and `.002`.
  - The script's earlier 495 was this change's 493 plus those 2.
  - Today it reports 497 in scope, 495 covered, and 2 uncovered
    (`path.compare.013`, `.014`, expected until the oracle-builder runs).
  - `--change add-fhirpath-and-codegen-r4` reports 495 in scope.
  - So the change's **Total** of 495 is right, and it is consistent with the
    script.
- **Other counts.** I grepped the change directory, `docs/inventory.md`,
  `tasks/` and `gates/` for 493, 361, 495 and 363. The only other hit is a
  dated status comment at `tasks/graph.yaml:56`: "proposal APPROVED on the
  third review, 493 scenarios". It is historical, and outside this change's
  artifacts. It is not a finding, but whoever next edits that entry may want to
  refresh it.

**Scenario IDs.** `c35959a` changes no `#### Scenario:` heading.

## Nineteenth review - commit `fd47948` (2026-11-17)

**Results file: every case in the three named groups, checked.**
- **`testSubstring`** has 16 cases (`results:344-359`). `1`-`5` and `7`-`9`
  are `passed`. `11`-`18` are `failed`. That is "8 of 16", as stated.
- **`testMinus`**: `1`-`6` `passed` and `7` `failed` (`:829-835`).
- **`testPlus`**: 27 `passed`. `testPlusDate23` and `24` are `failed`
  (`:823-824`).
- **Messages.** `testMinus7`, `testPlusDate23` and `testPlusDate24` all fail
  with "does not support MINUTE precision", as `math.012` now states.
  `testMinus6` is `invalid="execution"`, and it passes because HAPI raises.

**`path.string.004` / DEV-0019 substring half: correct.** I traced each case
through `funcSubstring` (`ENGINE:5864-5895`) and the reason text in the
results.
- `11` and `12`: `n1.get(0)` on an empty list (`:5867`). The reason is
  "Index 0 out of bounds for length 0".
- `13`, `14` and `15`: the end is `min(len, i1+i2)` (`:5884`), giving
  `[3,2)`, `[0,-1)` and the overflowed `[1,-2147483648)`.
- `16`: `""` is dropped (`:5888`).
- `17` and `18`: an empty `n2` returns empty (`:5871-5872`).
- `string.021`'s `{}.substring(0)` stays empty, because `i1` parses and the
  focus guard returns nothing. That is consistent.

**`path.mathfn.010` / DEV-0019 math half: correct.**
- `if (focus.size() != 1) throw makeExceptionPlural(...)` opens `funcSqrt`
  `:4096`, `funcAbs` `:4116`, `funcCeiling` `:4139`, `funcFloor` `:4157`,
  `funcLn` `:4201`, `funcLog` `:4221`, `funcPower` `:4249`, `funcTruncate`
  `:4273` and `funcRound` `:4397`.
- `funcExp` alone returns empty for size 0 (`:4177-4179`) and raises for more
  than one (`:4180-4182`).
- The requirement sentence (`spec.md:1269`) is updated to match.

**`path.string.023`: correct.**
- `case Round` calls `checkContextDecimal` (`ENGINE:3690`). It throws
  `FHIRPATH_DECIMAL_ONLY` for a typed non-numeric focus (`:3890-3892`). The
  message template is `Messages.properties:313` (`org.hl7.fhir.utilities`).
- With the check skipped, the else arm of `funcRound` calls `makeException`
  without `throw` (`:4414`), so the result is empty.
- Not in this commit: the same unthrown pattern is in `sqrt`, `abs`,
  `ceiling`, `floor`, `exp`, `ln`, `log`, `power` and `truncate` (`:4109`,
  `:4132`, `:4151`, `:4170`, `:4194`, `:4214`, `:4239`, `:4267`, `:4285`).

**`path.string.022` counting: correct.** `length` is `s.length()` (`:5771`),
`indexOf` is `s.indexOf` (`:5858`) and `substring` is `String.substring`
(`:5884`). All three count UTF-16 code units. The gaps are findings 1 and 2.

**`path.math.012`: correct**, given the statuses above.

**Scenario IDs.** Two titles change: `path.mathfn.010` and `path.string.022`.
The index (`spec.md:568`, `:596`) carries the new titles. No ID is added,
removed or renumbered. `path` has 363 headings, unchanged.

**DEV format and preamble.**
- DEV-0019 and DEV-0020 carry every field.
- The preamble says nineteen and lists DEV-0013, DEV-0015 and DEV-0017 to
  DEV-0020 as pending.
- `check-registries.sh` reports "registers agree" (19 deviations, 12 debt
  entries). DEV-0020's "Debt: none" is finding 1.

**Stale text.** `tasks.md:845-846` (task 40) now says UTF-16 code units, and
says `toChars` follows DEV-0020. It must follow whatever finding 1 resolves to.
No other "unicode scalar" wording remains outside DEV-0020 and `string.022`.

**Process note for the coordinator (not a spec finding).** `fd47948` is a
`docs(openspec)` commit, but it also changes
`crates/frevo-path/src/eval/tests.rs` and
`crates/frevo-path/tests/oracle/engine.rs`. The edits are:
- the evaluation-zone wiring in the oracle harness;
- removal of now-implemented functions from the "unsupported" lists.

Neither looks like a weakened assertion. Both are outside the proposer's write
set (constitution, Roles), and they should land under the oracle-builder or
the implementer.

## Twentieth review - commit `2419520` (2026-11-17)

Each example the spec now states was traced through the pinned source. Tests
will be written from them.

**`path.mathfn.004`: decimal rendering, each example checked.**

The rendering path: `DecimalType(double)` is
`setValue(BigDecimal.valueOf(d))` (`DecimalType.java:66-69`).
`BigDecimal.valueOf` parses `Double.toString(d)`, and `encode` is
`getValue().toString()` (`:101-102`). The stated rule for
`BigDecimal.toString` is Java's: plain when the scale is at least 0 and the
adjusted exponent is at least -6, otherwise scientific with a signed exponent.

| Example | Trace | Result |
|---|---|---|
| `12345.round(-2)` | `funcRound` takes the integer branch. `new BigDecimal("12345").setScale(-2, HALF_UP)` (`ENGINE:4411-4412`) gives unscaled 123, scale -2. | `1.23E+4` |
| `1234.5.round(-3)` | `1234.5` rounds `HALF_UP` to unscaled 1, scale -3. | `1E+3` |
| `10000000000.abs()` | The literal has 11 digits, so `Utilities.isInteger` rejects it (`Utilities.java:97-98`) and it is a `DecimalType` (`ENGINE:1326-1329`). `Double.toString` gives `1.0E10`, which is unscaled 10, scale -9. | `1.0E+10` |
| `0.0000001.abs()` | `Double.toString` gives `1.0E-7`: unscaled 10, scale 8, adjusted exponent -7. | `1.0E-7` |
| `0.0001.abs()` | `Double.toString` gives `1.0E-4`: unscaled 10, scale 5, adjusted exponent -4. | `0.00010` |
| `12345678.abs()` | `Double.toString` gives `1.2345678E7`, which is scale 0. | `12345678` |

The `-2` and `-3` arguments raise no check error. A leading `-` is parsed as
`Unary` (`ENGINE:1081-1102`), whose type set includes `integer` (`:1604-1607`).
`Minus` over `integer` and `integer` is typed `integer` (`:2067-2071`), so
`checkParamTypes(… integer)` passes. At evaluation the value is `0 - 2`, which
is `-2`.

**`path.string.002`: correct.** `funcToChars` iterates `s.toCharArray()`
(`ENGINE:5835-5837`), so `'a😀'.toChars()` is `a`, U+D83D, U+DE00, and
`count()` is 3.

**`path.string.022`: correct.**
- `length` is `s.length()` (`:5771`), so `'a😀b'.length()` is 4.
- `indexOf` is `s.indexOf` (`:5858`), so `indexOf('b')` is 3.
- `substring(1, 1)` is `sw.substring(1, min(4, 2))`, which is U+D83D.
- `substring(2)` is `sw.substring(2)` (`:5884-5886`), which is U+DE00 then
  `b`.
- Neither result is empty, so `:5888` keeps both.

**`path.string.004`: correct.** The clause now says that offsets count
UTF-16 units and that a surrogate pair can be split.

**DEV-0020: withdrawn.**
- The preamble says eighteen and lists DEV-0013, DEV-0015, DEV-0017,
  DEV-0018 and DEV-0019 as pending.
- `check-registries.sh` reports "registers agree" (18 deviations, 12 debt
  entries).
- `DEV-0020` appears nowhere else in the repository (outside this file).
- Task 40 (`tasks.md:845-848`) now requires the UTF-16 behaviour.

**Scenario IDs.** No `#### Scenario:` heading changed. `path` has 363
headings.

**Notes (not findings).**
- `crates/frevo-path/tests/path_string.rs:868` still labels `path.string.022`
  "unicode scalar values, not bytes". It is a test comment, so the
  oracle-builder should refresh it.
- The scenarios fix the engine's value: a one-unit string holding a lone
  surrogate. They do not say how such a value is serialized when it leaves the
  server, because no response surface in this change emits one. Whoever adds
  an endpoint that returns FHIRPath results should give it a differential
  case.
