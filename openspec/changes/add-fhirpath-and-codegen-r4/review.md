CHANGES_REQUESTED

Fifty-first review (commit `23fbf60`, 2026-11-21, diff against `f2bb275`).
`UCUM` is `reference/Ucum-java/src/main/java/org/fhir/ucum/` at `db9d22ad`,
and `ENGINE` is the R4 `FHIRPathEngine.java` at `73c432b5`.

I re-ran every stated value against the `javac -g` build of the 1.0.10
source and against the published 1.0.10 and 1.0.9 jars. All three outputs
are identical. Every value in the `path.math.016` additions reproduces:
- `0.5 div 0.03` is `17`;
- `0.5 mod 0.03` and `0.15 mod 0.04` are `-0.01`;
- all twelve `Unhandled` cases and all six non-failing counter-examples.

A 300,000-pair fuzz of `modulo` found no `Error` outside the two stated
triggers. `divide` and `divInt` raised no `Error` at all.

Every message and outcome in `path.math.017` reproduces, except the one in
the finding below. The checked points are:
- the `(numeric)` and `(exponent)` messages;
- the quote-less `1e+3' ...` text;
- the `toLowerCase()` NPE text, with the parameter name kept in the
  published jar;
- `Integer.parseInt(null)`, which gives `Cannot parse null string` on JDK 25;
- `1.6777216e7 / 3` = `5592405.3`.

`ENGINE` has no `catch` of `Error` or `Throwable`. The counts are right:
- path 390, total 522, `path.math.*` 17;
- the "Thirty-five" list;
- no duplicates;
- `openspec validate --strict` passes, and the registers agree.

`DEV-0030`, `DEBT-0020`, task 37 and the blocker entry match the spec. One
finding remains. The detail is in the dated section at the end. The earlier
record follows.

1. **The mantissa rule of `path.math.017` is wrong for negative values.**
   The scenario says "a **positive** mantissa of `16777216` (2^24) or more is
   refused", and gives `-16777216e0` as accepted. That implies every negative
   mantissa is read. It is not:
   - `Utilities.isDecimal` (`UCUM/Utilities.java:45-54`) rounds the mantissa
     to a `float`, `f`, and refuses it when `f + 1`, rounded to `float`,
     equals `f`. That holds for negative values too.
   - On the pinned build these are **refused** with
     `'<value>' is not a valid decimal (numeric)`: `-16777220e0`,
     `-33554432e0`, `-1234567890.5e0` and `1234567890.5e0`.
   - These are **accepted**: `-16777217e0` and `-16777218e0` (between 2^24
     and 2^25, `f + 1` is a tie that rounds to even), and `16777215.4e0`.
   - So under HAPI, `-33554432e0 / 2` is empty and `-33554432e0 mod 2`
     fails. An implementation of the rule as written returns a value for
     both.

   `DEV-0030`'s "whose `float` mantissa is 2^24 or more" has the same gap.
   Fix: state the rule as the `float` test itself (round half to even to a
   binary32, then refuse when adding one leaves it unchanged), and add the
   negative and non-integer cases above.

## Fiftieth review header as issued (commit `09f69fb`)

APPROVED

Fiftieth review (commit `09f69fb`, 2026-11-21, diff against `936bee5`). All
seven findings of the forty-ninth review are resolved. I re-ran the probes
against the `Ucum-java` 1.0.10 source build and the published 1.0.9 jar, and
both give the same output. Every changed value reproduces:
- the `path.math.016` negative-zero table, including the `-0.00` cases and
  `10 mod 2.5` = `0`, `12 mod 1.5` = `0`;
- the `.021` code-less `/` NPE, on either side, including `0 / 0`;
- the `.023` and `.024` parse texts;
- the `NumberFormatException` escape, and `comparable()` answering `false`.

The 1.0.9 statements, the corrected citations, `DEV-0030`, `DEBT-0020`, the
`DEV-0017` update and the new blocker entry are accurate. Nothing else
regressed:
- path 389, codegen 49, model 83, total 521, no duplicates;
- `openspec validate --strict` passes, and the registers agree (27/20);
- no stale "exact", `:2414`, `:895-904` or "non-negative dividend" text
  remains.

Two non-blocking notes are in the dated section at the end. The earlier
record follows.

## Forty-ninth-review findings as raised (commit `bdfe0ed`)

Forty-ninth review (commit `bdfe0ed`, 2026-11-21, diff against `36abd5f`).
`UCUM` is `reference/Ucum-java/src/main/java/org/fhir/ucum/` at `db9d22ad`,
and `ENGINE` is the R4 `FHIRPathEngine.java` at `73c432b5`.

I compiled `UCUM` with `javac -g` outside the repository. I then ran every
value the commit states through it, over the suite's `ucum-essence.xml`. I
repeated the run against the published `org.fhir:ucum` 1.0.10 and 1.0.9 jars,
with and without `-ea`. The following all match, value for value:
- every value in `path.math.005` to `.008`, `.015` and `.016`;
- every canonical value and unit string in `path.quantity.020`;
- every `*` and `/` result in `.021`;
- every `=` and `<` result in `.022` and `.023`;
- the parse messages listed in `.024`;
- the four NPE texts in `.018`;
- the two `mod` error texts, and the carry defect at `Decimal.java:751-754`.

I also checked the following, and they are right:
- the essence file facts in `.016` (24 prefixes, 7 base units, 303 units,
  `2.0.1`, and the fetched copy is byte-identical);
- the scenario counts (math 16, quantity 24, path 389, total 521; no
  duplicates);
- the "Thirty-four" list;
- `check-registries.sh` ("registers agree", 27/20);
- `openspec validate --strict`.

Seven findings remain. Each names a behaviour that is stated wrongly, left
out, or contradicted elsewhere in the spec. The detail is in the dated section
at the end. The earlier record follows.

1. **Two normative statements still say `/`, `div` and `mod` are exact.** They
   contradict `path.math.015` and `.016`.
   - The Arithmetic requirement body (`specs/path/spec.md:1358`) says "The
     operators here use exact decimal arithmetic".
   - `path.compare.012` (`:1123-1124`) says `+`, `-`, `*`, `/`, `div`, `mod`,
     `round` and `truncate` "use exact decimal arithmetic".

   An implementer reading either SHALL would build exact division. Fix: say
   that `+`, `-` and `*` are exact, and that `/`, and `div`/`mod` with a
   decimal operand, follow `org.fhir.ucum.Decimal` (`path.math.015`, `.016`,
   `DEV-0030`).

2. **A UCUM quantity with no `code` under `/` fails. It is not empty.**
   `path.quantity.021` says "a `/` is empty".
   - `qtyToPair` passes the `null` code through (`ENGINE:2324`).
   - `divideBy` evaluates `dividend.getValue().divide(divisor.getValue())`
     first, then `dividend.getCode().contains("/")`, then
     `divisor.getCode().contains("/")` (`UCUM/UcumEssenceService.java:322-323`).
   - So with a non-zero divisor value, a missing code on **either** side
     raises a `NullPointerException`, which the `/` arm does not catch
     (`ENGINE:3020`).
   - On every build I ran, the message is `Cannot invoke "String.contains(java.lang.CharSequence)" because the return value of "org.fhir.ucum.Pair.getCode()" is null`.
   - Only a zero divisor value stays empty, because `divide` throws its
     `UcumException` first.

   Fix: correct the clause, state both sides and the zero-divisor exception,
   and add it to `DEV-0030`.

3. **`[m/s2/Hz^(1/2)]` never reaches the special-unit check.**
   `path.quantity.023` lists it among the units that raise
   `Not handled yet (special unit)`. They do not:
   - The lexer does not admit `/` as a symbol character, even inside brackets
     (`UCUM/Lexer.java:136-140`). So the first token is `[m`, and parsing
     fails with
     `Error processing unit '[m/s2/Hz^(1/2)]': The unit '[m' is unknown' at position 0`.
   - Under `*` the message names the joined code, for example
     `'[m/s2/Hz^(1/2)].m'`.

   Fix: drop it from the "no handler" list and give it as a
   `path.quantity.024` example. The other six units in that list are right.

4. **`path.quantity.024` leaves out three parse failures. One of them breaks
   the scenario's "swallow" claim.**
   - `Expected '/' or '.'` (`UCUM/ExpressionParser.java:80`). For example,
     `m(s` gives `Error processing unit 'm(s': Expected '/' or '.'' at position 1`.
   - `unexpected token looking for a symbol or a number` (`:105`). For
     example, `.m` gives `... at position 0`, and `m.)` gives `... at position 2`.
   - **A number token outside the Java `int` range**, such as `10000000000`
     or `m10000000000`. `Lexer.getTokenAsInt` (`UCUM/Lexer.java:199-201`,
     called at `ExpressionParser.java:89` and `:139`) throws
     `java.lang.NumberFormatException: For input string: "10000000000"`.
     That is not a `UcumException`:
     - `qtyToCanonicalPair` and `qtyToCanonicalDecimal` catch only
       `UcumException` (`ENGINE:2296`, `:2309`), and so do the `*` and `/`
       arms (`:2800`, `:3020`).
     - So `=`, `~`, the ordered operators, `*` **and** `/` all fail with that
       message.
     - Only `comparable()` answers `false`, because `getCanonicalUnits`
       re-wraps every `Exception` (`UCUM/UcumEssenceService.java:237-239`).

   The scenario says a code that does not parse "raises `UcumException`", and
   that `=`, `~`, `comparable()` and `/` swallow it. Both are false for this
   input. Fix: add the three messages and the NFE outcome per operator.

5. **The `-0.0` rule in `path.math.016` is stated wrongly.** The scenario says
   "a zero remainder from a non-negative decimal dividend is `-0.0`". The
   sign of a zero difference is negative on every sign path of
   `Decimal.subtract` (`UCUM/Decimal.java:368-386`, with `modulo` `:792-798`).
   `asDecimal` writes the `-` whenever the zero's text contains a `.`
   (`:305-306`, `:157-164`). So:
   - `(0 - 4.0) mod 2`, `4.0 mod (0 - 2)`, `4 mod 2.0`, `6 mod 3.0`,
     `5 mod 2.5` and `7.5 mod 2.5` are all `-0.0`;
   - `10 mod 2.5` is `0`, because `multiply` trims the product to `10`, so the
     difference has no fraction.

   A test writer following the stated rule would expect `(0 - 4.0) mod 2` and
   `4 mod 2.0` not to be `-0.0`. Fix: state the rule as the text of the zero
   difference, and add the cases above to the table.

6. **The live oracle runs `org.fhir:ucum` 1.0.9, not 1.0.10.**
   - `hapi-fhir` `v8.12.0` sets `<ucum_version>1.0.9</ucum_version>`
     (`reference/hapi-fhir/pom.xml:1148`) and manages `org.fhir:ucum` to it
     (`:1985-1989`).
   - The starter's parent is `hapi-fhir:8.12.0` and it has no override
     (`reference/hapi-fhir-jpaserver-starter/pom.xml:36-40`).
   - So the HAPI the differential harness runs against has 1.0.9.
   - Only `org.hl7.fhir.core`'s own build, and
     `harness/hapi-fhirpath-results/pom.xml:18`, use 1.0.10.

   The spec, `DEV-0030`, `DEBT-0020` and tasks 37/43 all name 1.0.10 as
   "the version" behind HAPI's answers. `path.math.015` and `.016` also rest
   "HAPI's server-facing engine answers the same" on it.

   I compared the two published jars by normalised bytecode:
   - These classes are identical: `Decimal`, `Lexer`, `ExpressionParser`,
     `Converter`, `ExpressionComposer`, `UcumEssenceService`, `UcumModel`,
     `Pair`, `Registry` and `HoldingHandler`. Every stated value reproduces on
     1.0.9.
   - `XmlDefinitionsParser` differs: the short `yyyy-MM-dd` `revision-date`
     branch `path.quantity.016` states (`:79-88`) exists only in 1.0.10.

   No value changes, but the record is wrong about what the oracle executes,
   and `DEBT-0020`'s re-pin trigger watches the wrong artifact. Fix:
   - say which version each oracle runs (suite run and recorded results 1.0.10;
     live HAPI 1.0.9);
   - record the equivalence evidence;
   - mark the short-date branch as 1.0.10-only;
   - add a `tasks/blockers.md` note so the owner can annotate `PINS.md`
     step 5.

7. **Wrong citations and register scope.**
   - Quantity `toString()` is cited `Quantity.java:895-904`
     (`path.quantity.021`). `Quantity.java` has 897 lines and no `toString`.
     The text comes from `ENGINE` `convertToString`, `:887-906` (the quoted
     code is at `:898-900`), through `funcToString` `:4914-4919`.
   - The `NullPointerException` in the recursive `opLessThan` is the
     `isPrimitive()` call at `ENGINE:2413`. `path.quantity.017` and
     `DEV-0030` cite `:2414`, which is `Base l = left.get(0);`.
     `path.quantity.013` carries the same off-by-one.
   - `path.quantity.022` cites `:2414-2418`, `:2440-2450` for "the ordered
     operators". `<` compares at `:2419`. `>`, `>=` and `<=` take their
     canonical branches at `:2489-2498`, `:2587-2596` and `:2539-2548`, and
     compare at `:2466`, `:2564` and `:2514`. None of those is cited.
   - `DEV-0030`'s Scenario IDs omit `path.quantity.020` and `.024`.
     `path.quantity.022` cites `DEV-0030` for `60.0 '/min' = 1 '/s'`, which is
     `.020`'s whole-number step. The NFE and parse-failure outcomes (finding 4)
     are `.024`'s.

## Forty-eighth review header as issued (commits `30e32a3`, `f675156`)

APPROVED

Forty-eighth review (commits `30e32a3` and `f675156`, 2026-11-20).
- `30e32a3` resolves the forty-seventh finding.
- `f675156` rewrites the eight codegen and model parser scenarios to HAPI
  8.12.0's parser. Every message text, message code and lenient/strict outcome
  matches the pinned `hapi-fhir` sources.

The detail, and two non-blocking notes, are in the dated section at the end.
The earlier record follows.

## Forty-seventh-review findings as raised (commit `cd6f9a3`)

Forty-seventh review (commit `cd6f9a3`, 2026-11-20, diff against `0503660`).
Both forty-sixth findings are resolved: `path.quantity.019` and `DEV-0029`
record the server-facing `HAPI-0274: `, and `DEV-0017` says 43. One finding
remains. `path.quantity.019` over-states which operations reach the worker in
two places, and the differential relies on that list to explain HAPI's
failures. `ENGINE` is the R4 `FHIRPathEngine.java` at `73c432b5`. The detail
is in the dated section at the end.

1. **Two of `path.quantity.019`'s "raises `HAPI-0274: `" claims are wrong
   for part of their range.**
   - **`comparable()`.** `funcComparable` (`ENGINE:4609-4641`) returns
     without consulting the worker in several cases:
     - `false` when either side is not a single `Quantity`, or when either
       `system` is absent or the two differ;
     - `false` when either `code` is absent;
     - `true` when the two codes are **equal** (`:4627-4629`).
     
     It reaches `worker.getUcumService()` (`:4631`) only for two UCUM-system
     quantities with **different** codes. So under the server-facing worker,
     `1 'cm'.comparable(2 'cm')` is `true`, while the three `Comparable`
     cases raise. The scenario says "in `comparable()` with a UCUM `system`".
   - **`<=` with a unit on one side only.** `opLessOrEqual` (`:2533-2538`)
     computes `lunit`/`runit` and tests
     `(lunit == null && runit == null) || lunit.equals(runit)`. When the left
     has no unit and the right has one, `lunit.equals(...)` throws a
     `NullPointerException` before the worker is reached. For example,
     `Parameters.parameter[0].value <= Parameters.parameter[2].value` over
     `parameters-example-quantities.xml` fails with the JVM's NPE text, under
     **both** workers, not with `HAPI-0274: `. When the right has no unit and
     the left has one, the lists differ and the worker is reached. `<`, `>`
     and `>=` compare lists with `Base.compareDeep` and have no such NPE.
   - Fix: narrow both clauses to what the code does, add the two examples,
     and cross-reference the existing NPE statement (`path.compare.002`,
     spec `:1037`), which already gives `1 'kg' <= 1 year` that NPE.

## Forty-sixth-review findings as raised (commit `b356a10`)

Forty-sixth review (commit `b356a10`, 2026-11-20, diff against `beb1101`).
Every value in the four amended scenarios matches the pinned engine run under
the suite's worker and HAPI's recorded results, and so do both JVM messages.
Two findings remain. `ENGINE` is the R4 `FHIRPathEngine.java` at `73c432b5`,
and `HWC` is
`hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/ctx/HapiWorkerContext.java`
at `v8.12.0`. The detail is in the dated section at the end.

1. **HAPI's server-facing engine never reaches these answers. Its worker
   throws on `getUcumService()`, and no scenario or register entry says so.**
   - `HWC:372-374` is
     `public UcumService getUcumService() { throw new UnsupportedOperationException(Msg.code(274)); }`.
   - `FhirPathR4` builds its engine on `HapiWorkerContext`
     (`FhirPathR4.java:34-36`), and `FhirR4.java:56` makes that the
     `newFhirPath()` of every R4 context.
   - `qtyEqual` and `qtyEquivalent` call `worker.getUcumService()` for every
     pair of valued quantities (`ENGINE:2265`, `:2338`). The ordered operators
     call it whenever the `code` or `unit` lists differ (`:2443`, `:2490`,
     `:2540`, `:2588`). `opTimes`, `opDivideBy` and `comparable()` call it too
     (`:2793`, `:3013`, `:4631`).
   
   So under `FhirPathR4`:
   - Both `path.quantity.009` expressions, both `.011` expressions and both
     `.012` expressions raise `java.lang.UnsupportedOperationException` with
     the message `HAPI-0274: `. They do not give empty, `true` or an NPE.
   - `.013`'s `testLessThan33` raises the same, not the NPE. Its `<=` and `>`
     cases compare equal `unit` lists and never call the worker, so they are
     `true` there too.
   - `.010`, `1 'mg' = 1 'mg'` and every quantity `=`/`~` raise as well.
   - `path.quantity.015`'s "no UCUM data available, the result is `false`"
     describes a `null` service. HAPI's server worker never returns one.
   
   `FhirPathR4` catches only `FHIRException` (`:66-70`). The
   `UnsupportedOperationException` therefore escapes the `HAPI-0255` wrap and
   also `FhirPathFilterInterceptor`'s `FhirPathExecutionException` catch
   (`:80-84`), so it is observable over HTTP through `_fhirpath`.
   `DEV-0026` already establishes that the server-facing worker is what a
   deployment observes.
   
   Fix:
   - State which worker the quantity scenarios grade: the suite's
     `TestingUtilities` context, which has UCUM.
   - Add the server-facing outcome for the cases above.
   - Record Frevo's choice (UCUM-backed answers everywhere, as `DEV-0026` does
     for the type retry) as a deviation pending the owner.
   - Correct `.015`'s "no UCUM" clause to the configuration it actually
     describes.

2. **`DEV-0017` still says HAPI fails "38 cases" on these mechanisms. With the
   five quantity cases added, its own list names 43.** The list counts as
   follows:
   - 4 for `testEquality31`-`33` and `35`;
   - 10 for the minute-precision cases;
   - 8 for the `qtyEquivalent` NPEs;
   - 5 for the new unit-guard cases;
   - 9 for `testLessThan29`/`30`/`33` and the six `<=`/`>`/`>=` cases;
   - 1 for `testHasCodingValue2`;
   - 5 for division and modulo;
   - 1 for `testLessThan28`.
   
   The count is a stated fact in the register, so it must match.

## Forty-fifth review header as issued (commit `23a7568`)

APPROVED

Forty-fifth review (commit `23a7568`, 2026-11-20). The corrected
`path.navigation.025` matches the pinned engine. That includes two claims I
had passed in the forty-third and forty-fourth reviews and which were wrong:
`is FHIR.string` was given as `true`, and `contains` on `div` was said to pass
the check. The detail is in the dated section at the end. The earlier record
follows.

## Forty-fourth review header as issued (commit `e30696c`)

APPROVED

Forty-fourth review (commit `e30696c`, 2026-11-20) resolves both findings of
the forty-third review. It narrows `path.navigation.025` to what Frevo builds,
and records the HTML-mode item string as `DEV-0028` (pending the owner) and
`DEBT-0018`. Two non-blocking notes on the registers are in the dated section
at the end. The earlier findings follow, kept for the record.

## Forty-third-review findings as raised (commit `429b868`)

Forty-third review (commit `429b868`, 2026-11-20, diff against `295b604`). The
forty-second finding is resolved in `path.api.005`. The new
`path.navigation.025` is right on the by-name `string`, the XML-mode
composition and its examples, and the `children()` `xhtml` item. It over-claims
in two places, both in the HTML-mode clauses. `XC` is
`org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/xhtml/XhtmlComposer.java`
at `73c432b5`. The detail is in the dated section at the end.

1. **The CR LF rule skips the first child and never descends into it, so the
   scenario's rule is wrong for the common narrative shape.**
   `breakBlocksWithLines(List)` loops `for (int i = list.size() - 1; i > 0; i--)`
   (`XC:92-103`), and both the insertion and the recursion
   (`breakBlocksWithLines(node)`, `:101`) are inside that loop. So:
   - A block element that is the **first** child gets no CR LF.
   - The first child's own children are **never visited**.
   
   The scenario says a CR LF is inserted "before every child element named
   `li`, ..., `table` that does not already follow a line break". HAPI instead
   gives:
   - `<div xmlns="http://www.w3.org/1999/xhtml"><ul><li>a</li><li>b</li></ul></div>`
     composes in HTML mode with **no** CR LF at all. The `ul` is child 0, so
     its `li`s are never reached. The same holds for a narrative that opens
     with a `<table>`.
   - `<div xmlns="http://www.w3.org/1999/xhtml">x<ul><li>a</li><li>b</li></ul></div>`
     composes as
     `<div xmlns="http://www.w3.org/1999/xhtml">x` CR LF `<ul><li>a</li>` CR LF `<li>b</li></ul></div>`.
     The `ul` gets one because it is child 1, after the text `x`. Inside it,
     only the second `li` gets one.
   
   Fix: state the rule as the loop does: index 1 onwards, recursion only into
   those children, and the preceding-sibling test of `:97`. Add these two
   examples.

2. **The edit happens when the `xhtml` item's value is composed, not when
   `children()` navigates.**
   - `breakBlocksWithLines` runs only inside `XhtmlComposer(false).compose`,
     which `XhtmlType.primitiveValue()` (`XhtmlType.java:107-108`, through
     `getValue`, `:77-78`) calls on `place.getDiv()` (`:82`).
   - `funcChildren` and `funcDescendants` (`ENGINE:4808-4836`) only collect
     `new XhtmlType(this)`. `getChildrenByName` (`:255-279`) and
     `filterIdType` do not compose it either.
   - So in the scenario's example, `text.div` read after `text.children()` in
     the same evaluation is the **unchanged** text. It gains the CR LF only if
     the `xhtml` item's string is read first. For example:
     - `Patient.text.children().ofType(xhtml).toString()` composes it.
     - `Patient.text.children().ofType(xhtml) = ...` composes it.
     - `Patient.text.children().count()` does not.
   
   Fix: state the trigger as "the `xhtml` item's string value is read". Give
   one expression where the later `text.div` carries the CR LF, and one where
   it does not.

## Forty-second-review findings as raised (commit `e54ccc6`)

Forty-second review (commit `e54ccc6`, 2026-11-20, diff against `637010a`).
Both forty-first findings are resolved: `path.api.005` now states HAPI's
semantics, and `DEV-0027` is withdrawn. Every class fact, rendering rule and
table example I checked holds. One finding remains. The detail, and my
assessment of the blocker's framing, are in the dated section at the end.

1. **`Narrative.div` is navigated as a `StringType`, not an `XhtmlType`, so
   the `@Child`-declared class is wrong for it.**
   - `Narrative.getProperty` returns
     `new StringType(new XhtmlComposer(true).composeEx(this.div))` for `div`
     (`org.hl7.fhir.r4/.../model/Narrative.java:364-366`). The field's
     `@Child` declares `XhtmlType` (`:232`), and `listChildren` offers
     `new XhtmlType(this)` (`:343`). FHIRPath navigation goes
     `listChildrenByName` to `getProperty` (`ENGINE:270`), so the item HAPI
     returns is a `StringType` whose `fhirType()` is `string`.
   - The table the scenario says `frevo-codegen` generates "from each `@Child`
     field's declared Java type" would give `XhtmlType`. That reverses two
     results over `patient-example.xml`, which has a `text.div`:
     - HAPI accepts `Patient.text.div` as `string`.
     - As `xhtml` it fails with
       `HAPI-0256: FhirPath expression returned unexpected type StringType - Expected org.hl7.fhir.r4.model.XhtmlType`.
     
     It is accepted as primitive either way.
   - This is the only such substitution in the R4 model: no other
     `getProperty` builds a new object (`grep "new Base[] { new"` over
     `model/*.java`).
   - The class also depends on the route. `children()` and `descendants()`
     navigate by `"*"` (`ENGINE:4830-4836`), and `Base.listChildrenByName("*")`
     collects `listChildren`'s values (`Base.java:227-233`). Those are
     `new XhtmlType(this)` (`Narrative.java:343`), so `Patient.text.children()`
     yields an `XhtmlType` for `div`, where `Patient.text.div` yields a
     `StringType`.
   - The same fact is an engine behaviour that no scenario states. HAPI gives
     `Patient.text.div` the type `string`, and its value is
     `XhtmlComposer(true).composeEx(div)`. That decides `is(string)`,
     `is(xhtml)`, `toString()`, `contains()` and string comparison over the
     narrative.
   - Fix:
     - Add the `div` exception to `path.api.005`, with the two cases above,
       and to the table's generation rule.
     - Add a navigation scenario, or an explicit out-of-scope row, for the
       type and string value of `text.div`. The exact `composeEx` rendering is
       `org.hl7.fhir.utilities` code, so either state it or name the change
       that will.

## Forty-first-review findings as raised (commit `4fa72be`)

Forty-first review (commit `4fa72be`, 2026-11-20, diff against `c692b65`).
`ENGINE` is `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
at `73c432b5`, and `HAPI` is `reference/hapi-fhir` at `v8.12.0`. The new
claims in `path.strictness.005`, `path.strictness.006`, `path.errors.006` and
`path.errors.007` check out, and so does every column value. Two findings
concern `path.api.005` and `DEV-0027`. My assessment of `DEV-0027` and the
verification detail are in the dated section at the end.

1. **`path.api.005` cannot express the result types that HAPI's own callers
   ask for.** Frevo's requested type is "a FHIR type name, or `Base`". HAPI's
   callers of `IFhirPath.evaluate`/`evaluateFirst` mostly pass Java
   interfaces, not model classes:
   - `IPrimitiveType`: `FieldValidatingInterceptor.java:105`,
     `DaoResourceLinkResolver.java:466`/`:468`,
     `ServeMediaResourceRawInterceptor.java:92`/`:100`,
     `JpaPackageCache.java:418` and `PackageInstallerSvcImpl.java:1081`.
   - `ICompositeType`: `SearchParameterAndValueSetRuleImpl.java:232`, which
     raises `HAPI-0256` when a search-parameter path yields a primitive.
   - `IBaseReference`: `BulkExportMdmFullResourceExpander.java:252` and
     `OverridePathBasedReferentialIntegrityForDeletesInterceptor.java:123`.
   
   No FHIR type name has the meaning of "any primitive" or "any complex
   datatype". `Element` accepts both. Not every primitive derives from one
   FHIR type, and an `Enumeration` is an `IPrimitiveType`.
   So the scenario gives a ported caller no way to match HAPI's acceptance:
   - `IPrimitiveType` accepts `Patient.gender`, `Patient.active` and
     `Patient.birthDate`, and rejects `Patient.name`.
   - `ICompositeType` accepts `Observation.code` and rejects
     `Observation.status`.
   
   Fix: state the requested kinds the surface supports beyond a FHIR type
   name. At least add primitive, complex datatype, `Reference` and resource,
   with their acceptance rules and an example for each, or state explicitly
   that these callers are out of scope and where they will be specified.

2. **`DEV-0027`'s rationale is factually wrong.** It says "Frevo accepts
   strictly more than HAPI here and never returns a different value, so this
   is not a wrong-answer hazard". The scenario's own text contradicts this:
   - `IdType extends UriType` (`org.hl7.fhir.r4/.../model/IdType.java:120`),
     so HAPI accepts an `id` result for `UriType`, and Frevo rejects it for
     `uri`. That is strictly less than HAPI.
   - The rationale's "not observable over HTTP" is also unestablished. The
     interface-typed callers in finding 1 surface `HAPI-0256` in server
     behaviour, for example an authorization rule over a search-parameter
     path.
   
   Fix: correct the rationale, and list the cases where Frevo rejects what
   HAPI accepts (`id` as `uri`). Any narrower acceptance is a potential
   failure where HAPI succeeds, and is therefore a hazard to record.

## Fortieth review header as issued (commit `b73a460`)

APPROVED

Fortieth review (commit `b73a460`, 2026-11-20) resolves the one finding of the
thirty-ninth review. The detail is in the dated section at the end. The
earlier findings follow, kept for the record.

## Thirty-ninth-review findings as raised (commit `4331265`)

Thirty-ninth review (commit `4331265`, 2026-11-20, diff against `bc22e8c`).
Findings 2, 3 and 4 of the thirty-eighth review are resolved, and finding 1 is
resolved except for its precision-14 case. The detail is in the dated section
at the end. One finding remains:

1. **`path.compare.015` leaves the precision-14 boundary rewrite ungraded,
   although HAPI's value is fixed, and it cites the wrong scenario.** The new
   AND says "A boundary at precision 14 is malformed (`path.boundary.009`) and
   is rewritten only where it parses; no scenario grades that string". The
   malformed precision-14 forms are in `path.boundary.015`, not `.009`.
   HAPI's result is also derivable, so leaving it ungraded is a gap rather than
   an out-of-scope note. `BaseDateTimeType.parse` works as follows for
   `2014-01-01T08:00:+14:00`:
   - It takes the offset at index 17 (`getOffsetIndex`, `:330-342`, called at `:470`).
   - The time part is `08:00:` (length 6), so it reads the seconds from
     `value.substring(17, 19)` = `+1`, which is 1 (`:491`), at SECOND
     precision.
   - It sets the zone to `+14:00` (`:479`).
   
   The instant is therefore `2014-01-01T08:00:01+14:00`.
   `compareDateTimeElements` then rewrites it (precision SECOND > DAY). So:
   - `@2014-01-01T08.lowBoundary(14).where($this < @2015-01-01T00:00:00Z)`
     returns `2013-12-31T18:00:01Z`.
   - `@2014-01-01T08:05+08:00.lowBoundary(14).where($this < @2015-01-01T00:00:00Z)`
     (seconds `+0`) returns `2014-01-01T00:05:00Z`.
   - `@2014-01-01.lowBoundary(14).where(...)` (`2014-01-01T00:00:+14:00`)
     returns `2013-12-31T10:00:01Z`.
   - The `highBoundary(14)` forms with seconds `-1` fail before any
     comparison, as `.015` already states.
   
   Fix: replace the "no scenario grades that string" sentence with these
   cases, and correct the cross-reference to `path.boundary.015`.

## Thirty-eighth-review findings as raised (commit `56d3bf3`)

Thirty-eighth review (commit `56d3bf3`, 2026-11-20, diff against `16e6719`).
`ENGINE` is `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
at `73c432b5`. The column rule is right, and it corrects a value I passed in the
thirty-sixth and thirty-seventh reviews (`@char 11`/`21`, now `13`/`25`). Every
column value checks out. Four findings remain; the detail is in the dated
section at the end.

1. **`path.compare.015` says "a date boundary ... is never rewritten", but a
   `date` boundary above precision 10 is a millisecond `DateTimeType`.**
   For a `date` focus, `lowBoundary` and `highBoundary` build
   `new DateTimeType(...)` (`ENGINE:4317-4318`, `:4361-4362`). Above precision
   10, `DateTimeUtil.lowBoundaryForDate` (`DateTimeUtil.java:65-90`) pads the
   value to `T00:00:00.000` and appends `+14:00` (`-12:00` for high). The value
   is then finer than a day, so `compareDateTimeElements` rewrites it
   (`ENGINE:643-648`). So
   `@2014-01-01.lowBoundary(17).where($this < @2015-01-01T00:00:00Z)` returns
   `2013-12-31T10:00:00.000Z`, not `2014-01-01T00:00:00.000+14:00`. The
   converse also holds: a `dateTime` boundary at precision 10 or less, such as
   `@2014-01-01T08.lowBoundary(8)` = `2014-01-01`, is DAY precision and is not
   rewritten, so "a `dateTime` boundary builds a millisecond-precision
   `DateTimeType`" is true only at the default precision.
   Fix: decide by the built value's precision, not by the input type, and add
   the `date`-at-17 case and the `dateTime`-at-8 case.

2. **`path.boundary.013` gives the wrong byte offset for
   `(1.5 | 2.5).precision()`.** `precision` starts at byte index 12:
   `(`=0, `1.5`=1-3, space=4, `|`=5, space=6, `2.5`=7-9, `)`=10, `.`=11,
   `p`=12. Its column is 1 + 12 + 2 (the two spaces count 2 each) = 15. The
   column 15 is right; the stated "(byte offset 13)" is wrong. Since Frevo's
   offset is graded (`path.errors.002`), this is a testable error.

3. **`path.errors.002`'s list of direct constructions misnames one site and
   omits another.**
   - `:2801` is `opTimes`. It wraps the `UcumException` from
     `getUcumService().multiply`, so `getOpStart()` there is the `*` token,
     not "the `+` or `-`".
   - `:3102`, `throw new PathEngineException(e)` in the decimal arm of
     `opMod`, is missing from the list. It is the error `5.5 mod 0` and
     `5 mod 0.0` raise (`path.math.007`/`.008`). It passes no location and no
     id: the message is the cause's `toString()`, with no suffix, so Frevo
     carries no offset.
   Fix: state the `*` for `:2801`, and add `:3102` to the no-location list
   with "no suffix, no offset".

4. **"Never the sub-expression's text" (the `path.errors.002` title and task
   53) is false for the `as` singleton error.** `opAs` (`ENGINE:1884-1885`)
   and `funcAs` (`:5166-5167`) format `FHIRPATH_AS_COLLECTION`
   (`Messages.properties:301`,
   `Attempt to use ''as()'' on more than one item (''{0}'', ''{1}'')`) with
   `expr.toString()`. That is the holder node's rendering through
   `ExpressionNode.toString` (`ExpressionNode.java:396-464`), and it puts the
   node's text into the message. With the rule enforced,
   `Patient.name.as(HumanName)` over `patient-example.xml` (three `name`s)
   fails with exactly
   `Attempt to use 'as()' on more than one item ('3', 'as(HumanName)')`, with
   no `Error evaluating...` prefix and no suffix. `path.strictness.003` says
   only "evaluation fails".
   Fix: narrow the title and task 53 to the errors that use
   `rep`/`makeException`, and give `path.strictness.003` (or
   `path.errors.002`) the exact message for the function form and the
   operator form. In `Patient.name as HumanName` the holder is `last`, the
   `Patient` node (`ENGINE:1510-1519`), whose rendering includes its inner
   and its operation: `('3', 'Patient.name as HumanName')`. The operator
   form also needs `isKnownType("HumanName")` to pass through
   `fetchTypeDefinition` (`:1880-1882`).

## Thirty-seventh review header as issued (commit `444560a`)

APPROVED

Thirty-seventh review (commit `444560a`, 2026-11-20) resolves both findings
of the thirty-sixth review. The detail is in the dated section at the end. The
earlier findings follow, kept for the record.

## Thirty-sixth-review findings as raised (commit `ecf0496`)

Thirty-sixth review (commit `ecf0496`, 2026-11-20, diff against `d0b5a1d`).
`ENGINE` is `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
at `73c432b5`. Every other claim in the scope checks out against the pinned
source. The verification detail is in the dated section at the end. Two
findings remain:

1. **`path.errors.002` says every `makeException` error ends in `(@char N)`.
   It does not when the failing operator's node is a precedence group.**
   `newGroup` (`ENGINE:1315-1321`) builds a `Kind.Group` node and never calls
   `setStart`. `parseExpression` sets a start only on nodes it creates itself
   (`ENGINE:1076`). In `1 + 1 < 'a'`, `gatherPrecedence` for `+`
   (`ENGINE:1239-1311`, entry branch `ops.contains(start.getOperation())`)
   wraps `1 + 1` in such a group. It moves `<` onto that group and returns it
   as the root, so the `<` node is the group. `execute` passes that node as
   the holder (`last = exp`, `ENGINE:1510-1522`), and `opLessThan` throws
   `makeException(expr, FHIRPATH_CANT_COMPARE, ...)` (`ENGINE:2435`).
   `makeException` then builds
   `new PathEngineException(fmt, constName, holder.getStart() /* null */, holder.toString())`
   (`ENGINE:584-594`). `rep(null, <non-empty text>)` returns `""`
   (`PathEngineException.java:81-93`). So HAPI's message is exactly
   `Error evaluating FHIRPath expression: Unable to compare values of type integer and string`,
   with no position suffix. The same happens whenever the left operand of the
   failing operator is a run of higher-precedence operators
   (`a + b < c`, `x * 2 > 'y'`, `a | b = ...`, and so on). Real expressions
   are often written this way.
   `(1 + 1) < 'a'` is different: the parenthesised group is created by
   `parseExpression` with a start, so it does carry ` (@char 1)`.
   The scenario's last AND ("this holds for every execution error raised
   through `makeException` or `makeExceptionPlural`") is wrong, and the
   requirement sentence ("carrying ... the position of the failing
   sub-expression") over-promises. An implementer who follows the text would
   append ` (@char 1)` to `1 + 1 < 'a'`, and the differential would show a
   message diff.
   Fix: add cases to `path.errors.002`, or add a new ID:
   - `1 + 1 < 'a'` gives the bare message with no suffix.
   - `(1 + 1) < 'a'` gives the message with ` (@char 1)`.
   - `(1=1) and 1 + 1 < 'a'` gives the bare message. The `<` node is again a
     `newGroup` node, created by the `+` pass.
   Also state what Frevo's error offset is when HAPI has no location, or say
   that it is absent. As written, "the error's offset is the position of the
   failing operator node's start" has no HAPI counterpart for these cases.
   Narrow the final AND to holders with a non-null start.

2. **No scenario states that the evaluation's *result* carries the Zulu
   rewrite.** `path.compare.013` and `.015` grade the rewrite only through a
   later read inside the same expression (`toString()` through `$this`, or a
   second navigation). But the rewritten object is also what the evaluation
   returns when the compared value is the output. `funcWhere` adds the same
   `item` it compared (`ENGINE:6249-6260`), and `compareDateTimeElements` has
   already called `setTimeZoneZulu(true)` on that object (`ENGINE:635-648`,
   `BaseDateTimeType.java:690-694`). So in HAPI:
   - `(@2012-04-15T10:00:00+02:00).where($this < @2013-01-01T00:00:00Z)`
     returns one `dateTime` whose string is `2012-04-15T08:00:00Z`.
   - `Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').value.where($this < @1975-01-01T00:00:00Z)`
     over `patient-example.xml` returns `1974-12-25T19:35:45Z`, not
     `1974-12-25T14:35:45-05:00`.
   This is inside the evaluation: it is the evaluation's output, not the
   "after the evaluation" persistence that the new blocker defers. The phrase
   "for the rest of that evaluation" does not say whether the returned items
   are the rewritten ones. An implementation that drops its per-evaluation
   rewrite when the evaluation ends, before it materialises the result, would
   pass `.013` and `.015` and still diff against HAPI. That affects search
   parameter extraction and any returned value: the "a returned value shown to
   a user" cost that `DEBT-0017` itself names.
   Fix: add an AND to `.013` (element) and `.015` (literal), or add a new ID,
   with both expressions above and their returned strings. State in the same
   place that the result of `select($this)` after a comparison is the
   rewritten value too.

## Thirty-fifth review header as issued (commit `ee2a0d4`)

APPROVED

Thirty-fifth review (commit `ee2a0d4`, 2026-11-18) resolves all six findings
of the thirty-fourth review. Detail is in the dated section at the end. The
earlier findings follow, kept for the record.

## Thirty-fourth-review findings as raised (commit `bd08451`)

Thirty-fourth review (commit `bd08451`, 2026-11-18, diff against `6d38d35`).
It covers the amended `path.boundary.*` scenarios, the counts in `spec.md`,
task 45, `DEV-0025` and `DEBT-0016`. `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at `73c432b5`. `UTIL` is
`org.hl7.fhir.utilities/.../Utilities.java`, `DTU` is `.../DateTimeUtil.java`,
and `BDT` is `org.hl7.fhir.r4/.../model/BaseDateTimeType.java`, all at
`73c432b5`.

Most of the amendment is correct, and every decimal value and error in
`.001`-`.011` checks out (re-derived by simulating `applyPrecision` and
`minusOne`). Two scenarios state an error where HAPI returns a value, and four
behaviours are missing. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

1. **`path.boundary.012`: a quantity at precision `-1` is a valueless
   Quantity, not an error.**
   - For `1.587 'cm'.lowBoundary(-1)`, `applyPrecision` (`UTIL:1653-1680`)
     reaches `ld = 0`, so `s = nv.substring(0, 0)` is `""`, and
     `lowBoundaryForDecimal` returns `""`. `highBoundary(-1)` also gives `""`:
     `nv.charAt(0)` is `'1'`, which is below `'5'`, so there is no round-up.
   - `new DecimalType("")` does not throw. `DecimalType.setValueAsString`
     (`org.hl7.fhir.r4/.../model/DecimalType.java:153-157`) sets a null value
     for a blank string.
   - So `ENGINE:4326` / `:4370` set an empty `value` on the copy. The result
     is one Quantity with unit, code and system copied and no value.
   - The error does start one step further out. At `-2`, `ld = -1`, and
     `nv.charAt(-1)` throws `StringIndexOutOfBoundsException` (the message
     text depends on the JDK).
   - A negative precision does not always wipe the value:
     `12.5 'cm'.lowBoundary(-1)` gives `ld = 1` and the value `1`.
   - Rewrite `.012`: `-1` on `1.587 'cm'` is a single valueless Quantity for
     both functions, `-2` is an execution error, and add one multi-digit case
     such as `12.5 'cm'.lowBoundary(-1)` → `1 'cm'`.
   - Correct `DEV-0025`'s "so `1.587 'cm'.lowBoundary(-1)` is an error". The
     valueless or truncated quantity is a wrong answer, not an error, so it
     belongs in `DEBT-0016`.

2. **`path.boundary.015`: `lowBoundary(14)` succeeds. Only the
   `highBoundary(14)` default-offset case fails.**
   - With precision 14, `@2014-01-01T08.lowBoundary(14)` builds
     `2014-01-01T08:00:+14:00` (`DTU:65-89`, `:138`), and `new DateTimeType`
     parses it.
   - In `BDT.getOffsetIndex` (`BDT:330-341`), `+` is at 17, and the check
     `17-2 == 17-1-1` holds. The time part is `08:00:` (length 6 > 5), so the
     seconds are `value.substring(17, 19)`, which is `"+1"`.
     `Integer.parseInt("+1")` is `1`, which lies in `0..60` (`BDT:537-552`).
     The precision is SECOND, which `DateTimeType` allows.
   - So the result is a `dateTime` whose lexical form is
     `2014-01-01T08:00:+14:00`. `fromStringValue` keeps the input string.
   - `@2014-01-01T08.highBoundary(14)` builds `2014-01-01T08:00:-12:00`, whose
     seconds text is `"-1"`. That is out of range, so `throwBadDateFormat`
     raises `Invalid date/time format: "2014-01-01T08:00:-12:00"`
     (`BDT:796-798`).
   - A stated offset whose hour starts with `0` parses in both directions:
     `"+0"` and `"-0"` read as `0`. So
     `@2014-01-01T08:05+08:00.lowBoundary(14)` is the dateTime
     `2014-01-01T08:05:+08:00`.
   - Rewrite the precision-14 bullet with these three results and the exact
     message.
   - Update `DEV-0025` ("a date precision of 14 … fails"), task 45 ("14 an
     invalid dateTime") and `DEBT-0016`: a malformed dateTime that parses is
     a wrong answer, like the time precisions of `.016`.

3. **The zero branch of the decimal boundary is not specified.**
   - `lowBoundaryForDecimal` and `highBoundaryForDecimal` test `isZero` first
     (`UTIL:1638-1639`, `:1704-1705`, `:1718-1720`). The test strips `.`, `-`
     and `0`, so `0`, `0.0`, `-0.0` and `0.000` all take it. They return a
     fixed `-0.5…`/`0.5…` text instead of going through `minusOne`.
   - A generic implementation of the `minusOne` path would give `9.5…` for
     `0`, so this branch has to be stated.
   - The values are:
     - `0.lowBoundary()` is `-0.50000000`.
     - `0.highBoundary()` is `0.50000000`.
     - `0.lowBoundary(0)` is the decimal `-0`.
     - `0.highBoundary(0)` is `1`, because the `'5'` rounds the leading `0` up.
     - `(-0.0).lowBoundary(1)` is `-0.5`, the same as for `0`.
   - No suite case grades this (`LowBoundaryDecimal14`/`15` are `0.0034`, not
     zero). Add a differential-only scenario.

4. **Dispatch is by exact `fhirType()`, so `instant`, `positiveInt` and
   `unsignedInt` give empty.**
   - `ENGINE:4309-4328`, `:4353-4372` and `:4384-4390` test
     `base.hasType("decimal" | "integer" | "date" | "dateTime" | "time" |
     "Quantity")`. `Base.hasType` (`org.hl7.fhir.r4/.../model/Base.java:159-165`)
     compares `fhirType()` case-insensitively and does not follow subtypes.
   - So model values typed `instant` (`Bundle.timestamp`,
     `meta.lastUpdated`), `positiveInt` or `unsignedInt` (e.g.
     `ImagingStudy.numberOfSeries`) fall through to the unthrown
     `makeException` and give empty, for all three functions.
   - `.014` covers only literals (`1`, `'abc'`, `true`, `1 'cm'`). This
     requirement deliberately resolves types "through the model's type
     hierarchy" (`path.types.*`), so an implementation that treats `instant`
     as a dateTime, or `positiveInt` as an integer, would diff.
   - Add model-element cases to `.014`: an `instant`, a `positiveInt`/
     `unsignedInt` and a `code` element each give empty for `lowBoundary`,
     `highBoundary` and `precision()`.

5. **A Quantity with no `value` is unspecified.**
   - For a Quantity element without a value, `getNamedValue`
     (`ENGINE:4643-4649`) returns `null`.
   - `lowBoundaryForDecimal(null, …)` then throws
     `FHIRException("Unable to calculate lowBoundary for a null decimal
     string")` (`UTIL:1632-1634`). `highBoundary` throws the same with
     `highBoundary` (`UTIL:1698-1700`).
   - This is reachable from ordinary data (an `Observation.valueQuantity`
     carrying only `unit`/`code`). No scenario names it. Add it to `.008` or
     `.012` with both messages.

6. **`path.boundary.017`'s quoted message drops HAPI's prefix.**
   - `FHIRPATH_WRONG_PARAM_TYPE` is `Error evaluating FHIRPath expression: The
     parameter type {2} is not legal for {0} parameter {1}. expecting {3}`
     (`Messages.properties:371`). `makeException` passes it through
     unchanged (`ENGINE:584-594`).
   - The scenario says the message "is" `The parameter type Multiple Values …
     expecting integer`. This change quotes messages exactly elsewhere
     (`path.check.005`, with the prefix), so an implementer following `.017`
     literally would differ from HAPI.
   - The same applies to `.013`. The plural template
     (`Messages.properties:332`, arguments shifted by `I18nBase.formatMessagePlural`,
     `I18nBase.java:228-240`) renders as `Error evaluating FHIRPath expression:
     focus for highBoundary can only have one value, but has 0 values`.
   - Quote the full texts, or say that the quoted text is a suffix.

## Thirty-third review header as issued (commit `27615b7`)

Thirty-third review (commit `27615b7`, 2026-11-17) resolves the finding of the
thirty-second review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Thirty-second-review findings as raised (commit `3cc6bc9`)

Thirty-second review (commit `3cc6bc9`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at `73c432b5`. The
`env.006`, `.008`, `.010`, `.015` and `.016` amendments are correct (dated
section). One scenario has the check-first problem that `env.007`-`.014` had.
The thirty-first review was `APPROVED`; earlier findings follow, kept for the
record.

1. **`path.env.017`: the static check rejects the expression before the host
   is ever asked.**
   - The scenario's expression has a leading name that "is not an element of
     the focus resource".
   - In the check pass, a `Name` at entry with a focus type goes to
     `executeType(type, …)` (`ENGINE:1587-1589`, `:3231-3238`), which has no
     host call. An unknown name then raises `FHIRPATH_UNKNOWN_NAME`
     (`:1591-1593`).
   - The check consults the host for a bare name only when the focus type is
     absent: the `NOVALUE` call at `:1586`. The implicit modes exist only in
     evaluation (`execute`, `:3184-3190`, `:3211-3214`).
   - So both halves of the scenario fail the check with "The name '<n>' is not
     valid for any of the possible types": the implicit-after result, and the
     null-application-context result.
   - The recorded-modes assertion `[ImplicitBefore, ImplicitAfter]` is
     reachable only with the check skipped.
   - State that, as `env.007`-`.014` now do. The same applies to any
     `env.016` variant whose leading name the check cannot resolve.

## Thirty-first review header as issued (commit `2ea4f92`)

Thirty-first review (commit `2ea4f92`, 2026-11-17) resolves both findings of the
thirtieth review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Thirtieth-review findings as raised (commit `caf3579`)

Thirtieth review (commit `caf3579`, 2026-11-17). Finding 2 of the twenty-ninth
review is resolved, and finding 1 is resolved in substance. Two details of the
new `path.utility.003` text are wrong. Detail is in the dated section at the
end. The earlier findings follow, kept for the record.

1. **The quoted wrong-parameter-type message names the type wrongly.**
   - `checkParamTypes` passes `a`, an element of `actual.getTypes()`, as `{2}`
     (`FHIRPathEngine.java:3828-3830`). Type details hold URIs, and
     `FP_Boolean` is `http://hl7.org/fhirpath/System.Boolean`
     (`TypeDetails.java:66`).
   - The template is `The parameter type {2} is not legal for {0} parameter
     {1}. expecting {3}` (`Messages.properties:371`). So HAPI says "The
     parameter type http://hl7.org/fhirpath/System.Boolean is not legal for
     check parameter 1. expecting ...", not "…type boolean…".
   - Quote it with the URI, and give `{3}`, which is `pt.toString()` of the
     singleton string type.

2. **`Patient.name.check(prefix.exists(), 'm')` is the wrong example of the
   wrong-parameter-type failure.**
   - The condition is typed against the context item, `Patient`
     (`:3267`). `Patient` has no `prefix`, so typing the parameter raises
     `FHIRPATH_UNKNOWN_NAME` for `prefix` before `checkParamTypes` runs
     (`:3298`, then `:3545`). That is the same failure as the `given`
     example in the next clause.
   - Only `Patient.name.check(true, 'm')` illustrates the boolean rejection.
   - Replace the second example with a condition that resolves on `Patient`,
     for example `Patient.name.check(active.exists(), 'm')`, or say that
     `prefix` fails on the name.
   - The evaluation clause (check skipped, `prefix` evaluated against the
     focus `Patient.name`) is correct as written.

## Twenty-ninth-review findings as raised (commit `5f37fc7`)

Twenty-ninth review (commit `5f37fc7`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at `73c432b5`. The env,
trace-log, `is`/`as`/`ofType` and `types.013` amendments are correct (dated
section). Two statements about the static check are not. The twenty-eighth
review was `APPROVED`; earlier findings follow, kept for the record.

1. **The static check rejects *every* boolean `check()` condition, not only
   focus-relative ones.**
   - `case Check` (`ENGINE:3544-3547`) is
     `checkParamTypes(exp, "check", paramTypes, new TypeDetails(SINGLETON,
     FP_String))`. That types the **first** parameter, the condition, against
     `string`.
   - A condition typed `boolean` therefore always raises
     `FHIRPATH_WRONG_PARAM_TYPE` (`:3820-3830`), even when it resolves
     against the context item. For example, `Patient.check(name.exists(),
     'm')` passes the name lookup, then fails the check on parameter 1 being
     `System.Boolean`.
   - Only a condition with no types escapes.
   - `path.utility.003` says a test "must therefore skip the check or use a
     condition that resolves against the context item". The second option
     does not exist. `tasks.md:961-962` repeats it ("or use a context-relative
     condition").
   - The `given.exists()` example is still right: it fails earlier, on the
     unknown name, because parameters are typed before `checkParamTypes`
     (`:3298`, then `:3545`).
   - State that the check fails for any typed boolean condition, with the
     wrong-parameter-type message, and that every `check()` evaluation test
     skips the check.

2. **The unknown-extension warning text is truncated.**
   - `FHIRPATH_UNKNOWN_EXTENSION` is `Reference to an unknown extension -
     double check that the URL '{0}' is correct` (`Messages.properties:368`).
   - `path.utility.009` quotes only "Reference to an unknown extension". The
     check-pass warnings are graded through the `path-check-warnings` oracle
     (`path.check.015`-`019`), so the full text is the expected output.
   - Also state the two result shapes: an unfound URL is typed
     `SINGLETON Extension` (`ENGINE:3591`), and a found one
     `ORDERED <profile url>` (`:3586`).

## Twenty-eighth review header as issued (commit `9044efc`)

Twenty-eighth review (commit `9044efc`, 2026-11-17) resolves all three findings
of the twenty-seventh review. Detail is in the dated section at the end. The
earlier findings follow, kept for the record.

## Twenty-seventh-review findings as raised (commit `12bb026`)

Twenty-seventh review (commit `12bb026`, 2026-11-17). The new scenarios'
Java-side claims were probed with `java.util.regex` (Temurin JDK 25.0.1),
using HAPI's exact call shapes: `"(?s)"+p` with `find`, `"(?s)"+p` with
`Pattern.matches`, and `Matcher.replaceAll`. Because the JDK is not HAPI's
runtime, the results are evidence about `java.util.regex`, whose behaviour
here is long-standing, not a measurement of HAPI. `.026`, `.027` and `.029`
are accurate. Three items are not. Detail is in the dated section at the end.
The earlier findings follow, kept for the record.

1. **`path.string.025`'s `[a--b]` example is not a departure, and "class
   intersection" misdescribes Java.**
   - Java parses `[a--b]` as the range `a`..`-`, which is reversed. So
     `Pattern.compile("(?s)[a--b]")` raises `PatternSyntaxException: Illegal
     character range` (probe), and HAPI fails `'a'.matches('[a--b]')` too.
   - Java's class intersection is `&&`. In Java, `--` and `~~` are literals or
     range ends, never operators.
   - Use an example Java accepts and the `regex` crate reads as a set
     operation: `'b'.matches('[a-z--b]')` is `true` in Java (probe: the range
     `-`..`b` is added), and the crate reads it as set difference. `[a~~b]`
     is fine: Java gives `true` (probe).
   - Correct the "HAPI accepts every one of them … class intersection"
     clause.

2. **`path.string.028` claims Java never splits a surrogate pair after an
   empty match. It does.**
   - `Matcher.find` steps one UTF-16 unit past an empty match. The probe
     gives `"a😀".replaceAll("x*", "-")` = `-a-\uD83D-\uDE00-`: seven code
     points, with an empty match between the two halves.
   - Frevo steps a whole code point (`strregex.rs:343-347`,
     `c.len_utf8()`), which gives `-a-😀-`.
   - So the scenario states the wrong HAPI behaviour, and Frevo silently
     differs from HAPI on any supplementary character.
   - Either specify HAPI's split result and build it (`path.string.022`
     already requires UTF-16 semantics and lone-surrogate values), or record
     the difference in DEV-0023 and `DEBT-0015`.

3. **Constructs Java has and the crate lacks are a departure, but DEV-0023
   does not list them.**
   - Backreference `(a)\1`, lookahead `a(?=b)`, atomic groups, `\Q..\E`,
     `\R`, `\X`, `\G`, `\Z`, `\h` and `(?d)` are **answers** in HAPI. The
     probe gives `true` for `'aa'.matches('(a)\\1')` and
     `'ab'.matches('a(?=b)')`. They are **errors** in Frevo.
   - That is a difference from HAPI (constitution section 1), and ADR 0005
     lists it as the first DEV candidate.
   - DEV-0023's HAPI-behaviour and chosen-behaviour fields omit it.
     `path.string.025` points it at `DEBT-0015`, which calls these "not
     hazards".
   - Add them to DEV-0023, and cite DEV-0023 in `.025`'s clause.

## Twenty-sixth-review findings as raised (commit `e3a8ed0`)

Twenty-sixth review (commit `e3a8ed0`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` and `RT` is
`org.hl7.fhir.utilities/.../regex/RegexTimeout.java`, both at `73c432b5`. The
HAPI edge behaviour now stated for `matches`, `matchesFull` and
`replaceMatches` is correct (dated section). The findings concern how Frevo's
regex departures are recorded and specified. The twenty-fifth review was
`APPROVED`; earlier findings follow, kept for the record.

1. **Frevo's silent wrong-answer regex differences are recorded only as debt,
   not as deviations.**
   - `DEBT-0015` (`docs/tech-debt.md:388-418`) lists cases where Frevo
     returns a **different answer** from HAPI:
     - `$` without `(?m)` before a final line terminator;
     - Unicode `(?i)` folding (U+212A against `k`);
     - `.` in `replaceMatches` when the pattern has an inline flag group;
     - `\U` and similar escapes that Java rejects and Frevo accepts (HAPI
       errors, Frevo answers);
     - a lone surrogate treated as an ordinary character;
     - and "a pattern HAPI times out on gets an answer".
   - Constitution section 1 makes `docs/deviations.md` "the only exception
     mechanism" and says "an undocumented difference from HAPI is a bug". Its
     debt rule covers the flag *not built* where Frevo **matches** HAPI. It
     does not cover a difference Frevo ships.
   - ADR 0005 itself labels each of these a "DEV candidate" (Consequences).
   - DEV-0023 mentions them only as "the remaining corners are `DEBT-0015`".
     Its **Debt** field says `none`, and `DEBT-0015`'s Origin does not cite
     DEV-0023. So `check-registries.sh` rule 4 (reciprocity) cannot see the
     link, and passes.
   - Enumerate these differences in DEV-0023's HAPI-behaviour and
     chosen-behaviour fields. Set **Debt** to `DEBT-0015`, and add DEV-0023 to
     `DEBT-0015`'s Origin.

2. **DEV-0023 calls an unbalanced parenthesis a departure. It is not one.**
   - `Pattern.compile` raises `PatternSyntaxException`. Inside
     `executeWithTimeout` that surfaces as an `ExecutionException`, which is
     rethrown as `RuntimeException` (`RT:28-31`).
   - So HAPI also fails evaluation for `'a'.matches('(')`. Both sides fail.
     The entry's "These are departures from HAPI, which accepts them" is
     wrong for that item.
   - Move it out of the departure list, or state that it is parity.

3. **The scenarios do not specify Frevo's regex-layer behaviour, so no test
   can be written from them.**
   - `path.string.008`-`.010` state HAPI's edges, and only "a pattern Java
     cannot compile raises an execution error". None of the following
     appears in any scenario. Each is only in `strregex.rs`, ADR 0005 or the
     DEV text:
     - **The rejections.** `'aaa'.matches('a*+')`, `'\u000b'.matches('\\v')`,
       `(?U)`, `(?u)`, `(?x)`, `'a'.matches('\\p{Alpha}')`, and
       `'a'.matches('[a--b]')` are execution errors in Frevo and answers in
       HAPI.
     - **The exact rewrites.** `\d`, `\w`, `\s` and `\b` are ASCII:
       `'é'.matches('\\w')` is `false`, as in Java.
     - **The Java replacement expander.** `$1a` is group 1 then `a`; a missing
       group is an error; `\$` is a literal `$`.
     - **Empty-match iteration.** `'abc'.replaceMatches('b*', '-')` is
       `-a--c-`. `strregex.rs:503` already asserts it.
     - **The limits.** `size_limit` is 2 MiB and `nest_limit` is 100. A
       pattern over either is an execution error.
   - Add scenarios (differential-only where no suite case grades them) with
     these or similar concrete cases.
   - **ADR consistency.** Rejecting `(?x)` is not in ADR 0005's Decision
     item 5, which lists only `U` and `u`. Either amend the ADR on its
     `chore/` branch, or drop `(?x)` from the rejection list.

## Twenty-fifth review header as issued (commit `14524ee`)

Twenty-fifth review (commit `14524ee`, 2026-11-17) resolves the one finding of
the twenty-fourth review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Twenty-fourth-review findings as raised (commit `8a7b868`)

Twenty-fourth review (commit `8a7b868`, 2026-11-17). Both findings of the
twenty-third review are resolved, except for one new claim that HAPI does not
honour. Detail is in the dated section at the end. The earlier findings follow,
kept for the record.

1. **`join` with two arguments is a parse error in HAPI, not `1,2 and 3`.**
   - `specs/path/spec.md:1584-1587` (`path.string.020`) states that
     `(1 | 2 | 3).join(',', ' and ')` is `1,2 and 3`. `tasks.md:860-861`
     carries "`join` with its separate last separator".
   - But the parser checks arity on every function call
     (`FHIRPathEngine.java:1182`, `checkParameters`). For `Join`,
     `checkParamCount(lexer, location, exp, 0, 1)` (`:1427`) allows at most
     one argument. With two, it raises `The function "join" requires between 0
     and 1 parameters` (`:1355-1358`).
   - So the two-argument branch in `funcJoin` (`:4554-4557`) and the
     `CommaSeparatedStringBuilder` last-separator path are unreachable from an
     expression. The case is a syntax error.
   - Replace the clause with that parse failure and its message. Also remove
     the last-separator wording from task 41.

## Twenty-third-review findings as raised (commit `a06bc4f`)

Twenty-third review (commit `a06bc4f`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` and `UTIL` is
`org.hl7.fhir.utilities/.../utilities/Utilities.java`, both at `73c432b5`. Every
stated mechanism and per-case status checks out. Two behaviours the commit now
asserts are left without outcomes. Detail is in the dated section at the end.
The twenty-second review was `APPROVED`; earlier findings follow, kept for the
record.

1. **`trim`, `split` and `join` on a non-primitive focus: the outcomes are
   unstated, and they differ from one another.**
   - `path.string.020` now says the three "use the item's primitive value".
     For a complex item, that value is `null` (`Base.primitiveValue`,
     `org.hl7.fhir.r4/.../model/Base.java:145-147`). Each function then
     behaves differently:
     - `Patient.name.first().trim()` raises a `NullPointerException` from
       `cnt.trim()` (`ENGINE:4527`).
     - `Patient.name.first().split(',')` is **empty**: `simpleSplit(null, …)`
       returns no parts (`UTIL:1979-1981`).
     - `Patient.name.join(',')` is **one empty string**:
       `CommaSeparatedStringBuilder.commit` skips a `null` pending value
       (`CommaSeparatedStringBuilder.java:72-84`). So in a mixed focus the
       complex items vanish, with no separator.
   - None of these is a check failure. The check does not type the focus of
     any of the three (`ENGINE:3622-3630`). Cite that alongside the evaluation
     lines.
   - Add the three outcomes, with these or similar concrete WHENs.

2. **`escape` / `unescape` with `url` and `md`: the formats are asserted but
   not specified.**
   - `path.string.019` now says both functions "also accept `url` and `md`"
     (`ENGINE:4492-4495`, `4513-4516`), but gives no output rule. These are
     not FHIRPath-defined formats, so a tester cannot derive the outputs. The
     rules are:
     - **`url` escape** is `URLEncoder.encode(s, "UTF-8")` (`UTIL:880-886`).
       That is form encoding: a space becomes `+`, and `*`, `-`, `.`, `_` and
       alphanumerics pass through.
     - **`url` unescape** is `URLDecoder.decode(s, "UTF-8")` (`UTIL:889-895`).
       `+` becomes a space, and a malformed `%` sequence raises
       `IllegalArgumentException`.
     - **`md` escape** is `makeStringSafeAsMarkdown`
       (`MarkDownProcessor.java:254-263`). It puts a backslash before each of
       `* & # [ > < `` ` ``. The set is in `isEscableMarkdownChar`, `:265-278`.
     - **`md` unescape** is `makeMarkdownForString` (`:280-289`). It drops a
       backslash only when it is followed by one of those seven characters.
   - Add concrete examples, such as `'a b*'.escape('url')` → `a+b*` and
     `'a*b'.escape('md')` → `a\*b`.
   - Task 41 (`tasks.md:857-859`) still lists only "`escape` and `unescape`
     for html and json". Bring it into line.

## Twenty-second review header as issued (commit `a2dfc15`)

Twenty-second review (commit `a2dfc15`, 2026-11-17) resolves both findings of the
twenty-first review. Detail is in the dated section at the end. The earlier
findings follow, kept for the record.

## Twenty-first-review findings as raised (commit `65aeb76`)

Twenty-first review (commit `65aeb76`, 2026-11-17). `ENGINE` is
`org.hl7.fhir.r4/.../fhirpath/FHIRPathEngine.java` at `73c432b5`. Every factual
claim in the commit checks out against the source and the results file. Two
gaps remain. Detail is in the dated section at the end. The twentieth review
was `APPROVED`; earlier findings follow, kept for the record.

1. **The Type conversion requirement sentence contradicts the amended
   scenarios.** `specs/path/spec.md:1543` still says the engine implements
   "the `toX` functions the R4 engine implements ... so that a failed
   conversion yields an empty collection".
   - `toDateTime` and `toTime` are now specified as raising
     `FHIRPATH_NOT_IMPLEMENTED` for every focus (`path.convert.011`, `.013`;
     `ENGINE:4976-4987`).
   - So the requirement-level SHALL is false for two of the `toX` functions.
   - Name the two stubs as the exception in the requirement sentence,
     pointing at DEV-0021.

2. **The check-stage message for a non-primitive focus is unstated, and HAPI
   spells the function name differently from every other function.**
   - `case ToDateTime` calls `checkContextPrimitive(focus, "ToDateTime", ...)`
     (`ENGINE:3651`), and `case ToTime` passes `"ToTime"` (`:3655`). Both
     have a capital `T`.
   - Every other conversion passes the lower-case code, either as a literal
     (`"toQuantity"` `:3643`, `"toBoolean"` `:3647`) or through
     `exp.getFunction().toCode()` (`:3669`).
   - The template is `FHIRPATH_PRIMITIVE_ONLY` (`Messages.properties:353`).
     HAPI therefore reports "The function ToDateTime can only be used on
     primitives", and "The function ToTime ..." likewise.
   - `convert.011` and `.013` say only "a non-primitive focus is rejected by
     the check". An implementer following the pattern of the other functions
     would emit `toDateTime`, and the differential would diff.
   - State the message, with its capital `T`, and a concrete non-primitive
     focus, for example `Patient.name.toDateTime()`.

## Twentieth review header as issued (commit `2419520`)

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

## Twenty-first review - commit `65aeb76` (2026-11-17)

**`toDateTime` / `toTime` stubs: correct.**
- `funcToDateTime` (`ENGINE:4976-4981`) and `funcToTime` (`:4983-4987`) have
  commented-out bodies and an unconditional `throw makeException(...,
  FHIRPATH_NOT_IMPLEMENTED, "toDateTime"|"toTime")` (`:4980`, `:4987`).
- Dispatch is direct (`:3992-3993`), with no empty-focus short-circuit, so
  `{}.toDateTime()` raises too.
- The template is `Messages.properties:347`, which reads "Error evaluating
  FHIRPath expression: The function {0} is not implemented".
- The check (`:3650-3656`) calls `checkContextPrimitive`. That is a no-op for
  a type-less focus such as `{}`, and passes a primitive focus (`:3872-3882`).
  It types the result `dateTime` / `time`.
- These are the only two `FHIRPATH_NOT_IMPLEMENTED` throws in the engine. No
  other conversion stub was missed.
- `toDate` has no `Function` value (`ExpressionNode.java:53`), which is
  consistent with the requirement.

**`convertsToDateTime` / `convertsToTime`: correct.**
- `funcIsDateTime` (`:5942-5955`) matches strings against `DATE_TIME_REGEX`,
  which is true for any `DateType` or `DateTimeType` and false for a size
  other than 1.
- `funcIsTime` (`:5986-5990`) uses the inline time regex, exactly as quoted.
  `'14'`, `'14:34'`, `'14:34:28'` and `'14:34:28.123'` match, and `'abc'`
  does not.

**`path.convert.004`: correct, each example traced.**
- `funcIsDate` uses `DATE_TIME_REGEX` with the upstream `FIXME` (`:5957-5965`).
- `DATE_TIME_REGEX` is defined at `RegexConstants.java:27-30`
  (`org.hl7.fhir.utilities`).
- `'2015-02-04T14'`, `'2015-02-04T14:34'` and `'2015-02-04T14:34:28+10:00'`
  match. The minutes and seconds are optional, and `+10:00` matches
  `(0[0-9]|1[0-3]):[0-5][0-9]`.
- `'2015-13'` fails, because the month is `0[1-9]|1[0-2]` and `matches` needs a
  full match. `'2015-02-04T25'` fails, because the hour is
  `[01][0-9]|2[0-3]`. `'abc'` fails.
- The "correct" `DATE_REGEX` (`:21-23`) requires a full date, which is why the
  `FIXME` says "Tests fail" (the suite's year and month cases).

**`path.convert.008`: correct.**
- `testTypes` has 101 cases: 99 `passed` and 2 `failed`. The two failures are
  exactly `testStringQuantityMonthToQuantityEquivalent` and
  `testStringQuantityYearToQuantityEquivalent` (`results:192-193`). Both are
  the `qtyEquivalent` null-unboxing NPE of `path.equality.004`.
- `funcToQuantity` gives an `IntegerType` or `DecimalType` the system UCUM and
  code `1` (`:4957-4973`).

**DEV-0021 framing: correct.** FHIRPath 2.0.0 defines `toDateTime()` and
`toTime()` as real conversions, and limits `convertsToDate` to date forms.
HAPI stubs the first two and widens the third. This is a genuine
specification-against-HAPI difference, resolved for HAPI as constitution
section 1 requires, in the same direction as DEV-0009. Every field is present.

**Registers and counts.**
- The preamble says nineteen and lists DEV-0021 as pending.
- `check-registries.sh` reports "registers agree" (19 deviations, 14 debt
  entries).
- The `spec.md` index carries the three new titles (`path.convert.004`,
  `.011`, `.013`).
- No `#### Scenario:` ID was added, removed or renumbered. `path` has 363
  headings.

**Stale text.**
- No "widens precision", "toTime converts" or "year, month and day precision"
  wording remains outside history.
- `tasks.md` (task 42, `:862-872`) is updated.
- `crates/frevo-path/tests/path_convert.rs:473-477` and `:561-566` already
  assert the stub behaviour.
- The remaining stale sentence is the requirement line (finding 1).

## Twenty-second review - commit `a2dfc15` (2026-11-17)

**Finding 1: resolved.** The Type conversion requirement (`spec.md:1543`)
now excepts `toDateTime` and `toTime` as stubs that always raise, pointing at
`path.convert.011` and `.013`. `path.convert.009` uses `toInteger`, so its
"a failed conversion yields empty" stays correct. No other "yields empty"
statement about `toDateTime` or `toTime` remains in `specs/path/spec.md` or
`tasks.md`.

**Finding 2: resolved.** `Messages.properties:353` (`org.hl7.fhir.utilities`)
is `FHIRPATH_PRIMITIVE_ONLY = Error evaluating FHIRPath expression: The
function {0} can only be used on primitives`.
- The template uses only `{0}`. The `describe()` and type-list arguments that
  `checkContextPrimitive` passes (`ENGINE:3880-3881`) are not rendered.
- So with `{0}` = `ToDateTime` (`:3651`) or `ToTime` (`:3655`), the quoted
  text "The function ToDateTime can only be used on primitives" is exact,
  apart from the standard prefix.

**Scenario IDs.** No `#### Scenario:` heading changed.

**Registers.** `check-registries.sh` reports "registers agree" (19
deviations, 14 debt entries).

## Twenty-third review - commit `a06bc4f` (2026-11-17)

**Statuses in the results file: every case of the affected groups.**

| Group | Status | Results lines |
|---|---|---|
| `testReplace` (6) | all `passed` | `:415-420` |
| `testReplaceMatches` (7) | all `passed` | `:408-414` |
| `testEncodeDecode` (8) | all `passed` | `:427-434` |
| `testEscapeUnescape` (7) | `testUnescapeJson2`, `3`, `4` `failed`, the rest `passed` | `:435-441` |
| `testTrim` (6) | all `passed` | `:442-447` |
| `testSplit` (8) | all `passed` | `:448-455` |
| `testJoin` (2) | `testJoinEmpty` `failed` | `:456-457` |

These match the spec's per-case lists exactly.

**Mechanisms verified.**
- **`funcJoin`** (`ENGINE:4547-4567`) adds `b.toString()` at `:4565`,
  unconditionally. `name.prefix` is empty on `patient-example.xml`, so the
  result is one `""`, which matches `results:457`.
- **`unescapeJson`** (`UTIL:478-528`).
  - `case '"'` appends `b` (`:492-494`).
  - `case 'u'` reads `substring(i+1, i+5)`, appends the character, and
    `break`s (`:516-519`). The only advance is the `i++` at `:525`, so `0041B`
    is copied as text and the result is `A0041B`.
  - `default` throws `Unknown JSON escape` (`:521`). A trailing `\` reads
    `charAt(len)`, which is the index error.
  - All of this matches the reason text at `results:439-441`.
  - The `testUnescapeJson` explanation (the lexer decodes `\"` first) is
    consistent with its `passed` status.
- **`simpleSplit`** (`UTIL:1978-1996`). With `div = ""`, `matches` returns
  `true` at every cursor, because its loop runs zero times (`:1998-2007`).
  `cursor` advances by `div.length()`, which is 0 (`:1988`). So the `while` at
  `:1985` never exits, and each pass appends to `parts`. Non-termination is
  confirmed. A trailing separator leaves `substring(last, len)`, which is
  `""`, as stated.
- **`funcReplace`** (`ENGINE:4840-4861`).
  - An empty focus, target or replacement yields nothing (`:4846`).
  - A `""` focus string yields `""` (`:4851-4852`).
  - Otherwise the result is `String.replace` (`:4854`), so
    `'abc'.replace('', '-')` is `-a-b-c-`.
  - More than one item raises `FHIRPATH_NO_COLLECTION` (`:4859`).
- **`trim`** is `String.trim`, which strips every character at or below U+0020
  (`:4527`). U+00A0 is kept.
- **Encode / decode** (`ENGINE:4440-4479`).
  - The format is compared to `hex`, `base64` and `urlbase64`, and any other
    format adds nothing, so `rot13` yields empty.
  - `Base64.Decoder.decode("!!")` raises `IllegalArgumentException`, which is
    uncaught.
  - The bytes come from `getBytes()` in the platform charset. That is stated.
- **`1.trim()`, `1.split(',')` and `1.join(',')`** use
  `IntegerType.primitiveValue()`, which is `"1"`. Correct. The non-primitive
  case is finding 1.

**DEV-0022 framing: correct.**
- The `join` and `unescape('json')` halves are ordinary
  specification-against-HAPI cases, resolved for HAPI.
- For the empty `split` separator, HAPI's observable behaviour is unbounded
  memory growth: no result, and in a server an eventual OOM or timeout.
  - An undocumented difference from HAPI is a bug (constitution section 1),
    so the departure belongs in the register.
  - The entry says plainly that this is a departure from non-termination, not
    a different defined result, and that it needs the owner's confirmation.
  - Returning an error is the reading of `path.errors.004` ("rather than ...
    running unbounded") applied to a loop inside a function rather than to
    evaluation steps, and of `path.errors.003`.
- The entry has every field.

**Registers and counts.**
- The preamble says twenty and lists DEV-0022 as pending.
- `check-registries.sh` reports "registers agree" (20 deviations, 14 debt
  entries).
- Only the title of `path.string.015` changed, and the index (`spec.md:587`)
  carries it. No ID was added, removed or renumbered. `path` has 363
  headings.

**Stale text.**
- No "turning a unicode escape into its character", "join … yields empty"
  or "harness records which behaviour" text remains for these scenarios.
- The `spec.md:192` and `:1241` hits are other scenarios.
- The commit stat shows `tasks.md` was **not** changed, contrary to its
  description. Task 41 is finding 2.

## Twenty-fourth review - commit `8a7b868` (2026-11-17)

**`url` (`Utilities.java:880-895`): correct.**
- `URLEncoder.encode(s, "UTF-8")` keeps `[A-Za-z0-9.*_-]`, turns a space into
  `+`, and writes everything else as upper-case `%XX` of its UTF-8 bytes. So
  `'a b&c=d/é~'` becomes `a+b%26c%3Dd%2F%C3%A9%7E`. Java encodes `~`.
- `URLDecoder.decode(s, "UTF-8")` turns `+` into a space and decodes `%XX`, so
  `a+b%20c` is `a b c`. `%zz` raises `IllegalArgumentException`.

**`md` (`MarkDownProcessor.java:254-289`): correct.**
- `isEscableMarkdownChar` (`:265-278`) is exactly `*` `&` `#` `[` `>` `<` and
  the backtick: seven characters, with none of `]`, `_`, `(`, `)` or `\`.
- `makeStringSafeAsMarkdown` (`:254-263`) puts a backslash before each of
  them.
- `makeMarkdownForString` (`:280-289`) keeps a character unless it is a `\`
  that is not last and is followed by one of the seven (`:284`). So:
  - `a\*b` gives `a*b`;
  - `a\xb` and a trailing `\` are unchanged;
  - `\*` keeps the first `\` (its successor `\` is not in the set), drops the
    second, and gives `\*`.
- The direction (escape inserts, unescape removes) is right. The escape
  citation `:254-274` overlaps the set's lines. That is a nit.

**Complex focus: correct.**
- `Base.primitiveValue()` is `null` (`Base.java:145-147`), and the check does
  not type the focus (`FHIRPathEngine.java:3622-3630`).
- `trim` then dereferences `null` at `:4527`, giving an NPE.
- `simpleSplit(null, …)` returns no parts (`Utilities.java:1979-1981`), so
  `split` is empty.
- `CommaSeparatedStringBuilder.commit` skips a `null` pending value
  (`:72-84`), so `join` is `''`.

**`join(sep, lastSep)`: wrong (finding 1).** The builder semantics as described
are accurate: the last separator goes before the final item. But HAPI's parser
never admits the second argument.

**Scenario IDs.** No `#### Scenario:` heading changed.

**Registers.** `check-registries.sh` reports "registers agree" (20
deviations, 14 debt entries).

**Stale text.** The only last-separator mentions are the two in finding 1.
Task 41 otherwise now lists `url` and `md`.

## Twenty-fifth review - commit `14524ee` (2026-11-17)

**Finding 1: resolved.**
- **Stage.** The failure happens at **parse**. `checkParameters` runs inside
  the parser right after the argument list (`FHIRPathEngine.java:1182`). For
  `Join` it calls `checkParamCount(…, 0, 1)` (`:1427`), and that throws
  `lexer.error(...)` (`:1355-1359`). Neither the check nor evaluation is
  reached.
- **Message.** The text is `The function "join" requires between 0 and 1
  parameters`, as quoted. `FHIRLexer.error` wraps it as `Error @<location>: `
  + msg (`FHIRLexer.java:139-141`). That is the usual lexer prefix, the same
  one other parse-error scenarios rely on, so it is not a finding.
- `join()` with no argument uses `param = ""` (`:4549`), which is an empty
  separator, as stated.
- The `funcJoin` two-argument branch (`:4553-4556`) is unreachable.

**Stale text.** The change directory has no other two-argument `join` or
last-separator statement. The one remaining hit is the new parse-failure
clause (`spec.md:1586`). Task 41 now says "taking at most one argument".

**Scenario IDs.** No `#### Scenario:` heading changed.

**Registers.** `check-registries.sh` reports "registers agree" (20
deviations, 14 debt entries).

## Twenty-sixth review - commit `e3a8ed0` (2026-11-17)

**Suite and results.** The suite has 23 regex cases:
- 16 in `testMatches` (`tests-fhir-r4.xml:762-778`), including the five
  `matchesFull` cases;
- 7 in `testReplaceMatches` (`:782-788`).

`hapi-8.12.0-results.json` has all 23 `passed` (`testReplaceMatches` at
`:408-414`; `testMatches` has 16 rows, none failed). "HAPI passes all of
them" is right.

**`funcMatches` (`ENGINE:5683-5708`): every stated edge is correct.**
- An empty focus or pattern collection gives empty (`:5688`).
- An empty pattern string or a focus of more than one item gives `false`,
  through the else arm (`:5705-5706`).
- An empty focus string gives `false` (`:5693-5694`).
- `1.matches('1')` gives empty, because no branch adds anything.
- `'abc'.matches(1)` uses `convertToString`, so the pattern is `1` and the
  result is `false`.
- `(?s)` is the prefix and the call is `RegexTimeout.find` (`:5698`).
- A timeout raises `Timeout evaluating regex: ` + the pattern (`:5699-5700`).
- The default is `regexTimeoutMillis = 500` (`:186`).

**`funcMatchesFull` (`:5711-5737`): correct.**
- There is no empty branch. `{}.matchesFull('b')` falls to the else arm and
  gives `false` (`:5733-5734`).
- `'abc'.matchesFull({})` and `'abc'.matchesFull('')` give `false`, because
  `sw` is `""`.
- `''.matchesFull('a')` gives `false`.
- `1.matchesFull('1')` gives empty.
- The call is `Pattern.matches` with `(?s)` (`:5725`; `RT:55-56`).

**`funcReplaceMatches` (`:4865-4889`): correct.**
- An empty collection argument gives empty (`:4872`).
- An empty regex string gives the focus string unchanged, through the else
  arm (`:4885-4886`), which matches `testReplaceMatches2`.
- A multi-item focus also takes that arm and returns the **first** item's
  string, with no error.
- A non-string focus gives empty.
- There is no `(?s)` (`:4879`).
- The call is `Matcher.replaceAll` (`RT:103-104`).
- One nuance for the timeout: `replaceMatches` calls the three-argument
  `RegexTimeout.replaceAll`, so it always uses `DEFAULT_TIMEOUT` = 500
  (`RT:18`, `:90-91`), never `regexTimeoutMillis`. DEV-0023 cites `:186` for
  all three functions. That is accurate for the default, but not for the
  setter. ADR 0005 states it correctly.

**DEV-0023 framing.**
- The `matchesFull`-on-empty half is a genuine specification-against-HAPI
  case, resolved for HAPI. It is legitimate.
- The construct rejections, the limits and the absent timeout are Frevo
  departures from HAPI. The owner accepted the `regex` crate in ADR 0005, and
  that ADR lists them as DEV candidates. So they belong in
  `docs/deviations.md`, with the corners Frevo cannot make exact in debt as
  well. Neither register alone is enough (finding 1).
- Unbalanced parenthesis: finding 2.

**Blockers entry.** It is a status note plus a request to write the regex
limits into `design.md`, which the proposer may not write. That is
appropriate.

**Registers and counts.**
- The preamble says twenty-one and lists DEV-0023 as pending.
- `check-registries.sh` reports "registers agree" (21 deviations, 15 debt
  entries). That result is blind to finding 1.
- No `#### Scenario:` heading changed. `path` has 363 headings.

**Stale text.** None found for these three scenarios.

## Twenty-seventh review - commit `12bb026` (2026-11-17)

**Probe results (Java side)**, from
`scratchpad/rx/Rx.java`, run outside the repository.

`path.string.025`:
- These are accepted, with these results: `(?u)a`, `(?U)a` and `(?x)a` are
  `true`; `\v` on `a` is `false`; `a*+` is `true`; `\p{Alpha}` is `true`;
  `[a~~b]` is `true`.
- `[a--b]` raises `Illegal character range` (finding 1).
- `(a)\1` and `a(?=b)` are `true` (finding 3).
- `(a` raises `Unclosed group`, which is parity, as the scenario says
  (`RegexTimeout.java:28-30`).
- Frevo's layer rejects `U`, `u` and `x` (`strregex.rs:183-185`), `--` and
  `~~` in a class (`:153-154`), possessive quantifiers (`:268`), `\v`
  (`:301`) and the 13 POSIX names (`:114-118`, `:312-314`). That matches the
  scenario's list.

`path.string.026` (all as stated):
- `\d` on `٣` is `false`. `\w` on `é` is `false` and `\W` on `é` is `true`.
- `a\sb` is `false` across a no-break space and `true` across a space.
- `7` matches `\d`, `_` matches `\w`, and `\bab\b` matches `ab`.
- `[\w]` on `é` is `false`.

`path.string.027` (all as stated):
- The results are `a[b]c`, `a<b>c`, `a$c` and `a\c`.
- With one group, `$10` gives `ab0c`. With ten groups, `$10` is group 10.
- These raise: `$2` (`No group 2`), `${nope}`, a trailing `$`, a trailing
  `\`, and `$$` (`Illegal group reference`).
- A non-participating group contributes `""`.

`path.string.028`:
- `x*` gives `-a-b-c-` and `b*` gives `-a--c-`. Correct.
- The surrogate stepping is finding 2.

`path.string.029`:
- Java compiles `(?:a{1000}){1000}` and answers `false`, and it answers
  `true` for 200 nested groups. So HAPI answers where Frevo errors, which is
  a departure, as stated.
- The limits are `SIZE_LIMIT = 2 << 20` and `NEST_LIMIT = 100`
  (`strregex.rs:53-55`).

**Testability.** Every clause of `.026`, `.027` and `.029` has a concrete
input and output. `.025` and `.028` will be too, once findings 1 and 2 are
fixed.

**Counts, independently recounted.**
- `#### Scenario: [` headings: `codegen` 49, `model` 83, `path` 368, for a
  total of 500.
- `path.string.*` has 29 headings, `001`-`029`, with no duplicates.
- `scenario-coverage.sh --change` reports 500 in scope, with 5 uncovered:
  `.025`-`.029`, which await the oracle-builder.
- The index (`spec.md`) and task 4 both say 368. Task 41a satisfies
  `.025`-`.029`.

**DEV / DEBT linkage.**
- DEV-0023's **Debt** field is `DEBT-0015`, and `DEBT-0015`'s Origin cites
  DEV-0023.
- `check-registries.sh` reports "registers agree" (21 deviations, 15 debt
  entries).
- The wrong-answer corners are now listed in DEV-0023. Not yet listed: the
  missing constructs (finding 3) and the surrogate stepping (finding 2).

**Blockers.** The `(?x)` amendment owed to ADR 0005 is recorded. That is
appropriate.

**Scenario IDs.** Five headings are added (`.025`-`.029`). No existing
heading changed.

## Twenty-eighth review - commit `9044efc` (2026-11-17)

These were re-probed with `java.util.regex` (Temurin JDK 25.0.1, HAPI's call
shapes; `scratchpad/rx/Rx2.java`, outside the repository).

**Finding 1: resolved.**
- Java reads `[a-z--b]` as `a-z`, `-` and `b`:
  - it matches `a`, `b`, `-`, `z` and `c`;
  - it does not match `~`;
  - so `'-'` and `'b'` match, as the scenario says.
- `[a-z~~b]` matches `a`, `b`, `z`, `c` and `~`, and does not match `-`. So
  `~` is a literal, as stated.
- The new clause makes `[a--b]` Java's `PatternSyntaxException`, which agrees
  with the earlier probe.
- The "class intersection" wording is gone (`grep` finds no remaining
  hit).

**Finding 2: resolved.**
- `"😀".replaceAll("x*", "-")` is exactly U+002D U+D83D U+002D U+DE00
  U+002D. The scenario's five-unit string is right.
- `"a😀"` gives U+002D U+0061 U+002D U+D83D U+002D U+DE00 U+002D, which is
  also right.
- `Pattern.compile("x*").matcher(…).replaceAll("-")`, the call shape of
  `RegexTimeout.replaceAll`, gives the same result.
- "Whole code point" and "never a split one" no longer appear anywhere.
- The current `strregex.rs:343-347` still steps a code point. The scenario now
  requires the unit step, which is implementer work, not a spec gap.

**Finding 3: resolved.**
- DEV-0023 now lists backreferences, lookaround, atomic groups, `\Q..\E`,
  `\R`, `\G`, `\Z` and `(?d)` as errors where HAPI answers. The probe gives
  `true` for `(?<=a)b`, `(?>a)b`, `\Qa.\E` and `(?d)a`.
- `path.string.025` cites DEV-0023.
- `DEBT-0015` calls them departures recorded in DEV-0023.
- A note, not a finding: ADR 0005 also names `\X` and `\h`, which DEV-0023's
  list omits. Add them next time the entry is touched.

**Linkage.**
- DEV-0023 has `Debt: DEBT-0015` (`deviations.md:708`), and `DEBT-0015`'s
  Origin cites DEV-0023 (`tech-debt.md:391-392`).
- `check-registries.sh` reports "registers agree" (21 deviations, 15 debt
  entries).

**Scenario IDs.** No `#### Scenario:` heading changed. `path` has 368
headings.

## Twenty-ninth review - commit `5f37fc7` (2026-11-17)

**`path.env.007`, `.009`, `.011`, `.014`: correct.**
- The check's `resolveConstantType` throws `FHIRPATH_CANNOT_USE`:
  - for `%resource` when `context.resource == null` (`ENGINE:3142-3145`);
  - for `%rootResource` likewise (`:3147-3150`).
- It throws `FHIRPATH_UNKNOWN_CONSTANT`:
  - with no host service, after the defined-variable lookup (`:3163-3167`);
  - when the host answers `null` (`:3170-3175`).
- Evaluation throws the same messages: `:1761-1763`, `:1766-1768` and,
  with no host service, `:1781-1782`.
- The templates are `Messages.properties:302` and `:365`. The quoted text
  matches, with the standard prefix.
- `env.011` correctly conditions the evaluation error on "no host service".
  With one, evaluation delegates to `hostServices.resolveConstant` (`:1784`).
- `env.014`: the check's defined-variable table is scoped by
  `contextForParameter`, so an out-of-scope variable falls to the same throw.

**`path.utility.001`/`002` trace log: correct, and not additive.**
- `takeLog` returns the log and replaces it (`:6692-6699`).
- Every `evaluate` overload resets it (`log = new StringBuilder()` at `:702`,
  `721`, `740`, `758`, `776`, `795`).
- `log()` defers to `hostServices.log` (`:927-928`).
- So "take clears" and "evaluate resets" are two separate rules, as now
  written, and both hold.

**`path.utility.003`/`004`.**
- Arity is 2 (`:1411`).
- `funcCheck` evaluates the condition against the focus, and on `false`
  raises `FHIRPATH_CHECK_FAILED` with the second argument's value
  (`:5649-5656`; `Messages.properties:304`).
- `isExpressionParameter` (`:3804-3817`) is as listed, except that it also
  answers `true` for `Sort` at every index (`:3805-3807`). That omission is a
  nit.
- Non-expression parameters are typed against `context.thisItem`
  (`:3267`). The check outcome is finding 1.

**`path.utility.009` / `.011`: correct apart from finding 2.**
- `case Extension` (`:3556-3591`) never throws on the URL.
- An absolute or non-Extension-focus URL goes through `findExtensionDefinition`
  (`:3747-3765`):
  - `worker.fetchResource(StructureDefinition, url)` for an absolute URL;
  - otherwise, a fixed-value search of the focus type's snapshot.
- A relative URL on an `Extension` focus walks the focus's profiles for
  `Extension.extension:<url>` (`:3561-3577`).
- `funcExtension` reads `extension` and `modifierExtension` on any item
  (`:5464-5482`).
- **Engine work owed: confirmed.** Typing a constant URL needs a
  StructureDefinition lookup by canonical URL from the model, and, for the two
  relative-URL paths, the snapshots of the focus type and its profiles. That
  is what the blockers entry says. It should name the relative-URL lookups
  too, but that belongs to the engine, not the spec.

**`path.types.011` / DEV-0024: correct, and a legitimate deviation.**
- **Operator `is` and `as` at check time.** The right operand is read by
  `executeTypeName` (`:1627-1628`), which just wraps the name (`:1570-1571`).
  `operateTypes` `case Is` returns boolean after `checkCardinalityForSingle`
  (`:2013-2015`).
- **Function forms at check time.** `as()`, `ofType()` and `is()` type the
  parameter as `string` and at most add a `FHIRPATH_OFTYPE_IMPOSSIBLE`
  warning (`checkType`, `:3368-3403`), so the check passes.
- **Evaluation.**
  - `opIs` compares names and walks base definitions, giving `false`
    (`:1940-1962`).
  - `funcIs` does not consult `isKnownType` (`:5095-5135`), and also gives
    `false`.
  - `opAs` (`:1881-1883`), `funcAs` (`:5163-5165`) and `funcOfType`
    (`:5206-5208`) raise `FHIRPATH_INVALID_TYPE`. That message has no prefix
    (`Messages.properties:336`).
  - The function forms prefix `FHIR.`, as stated.
- **Framing.** FHIRPath makes an unresolvable type identifier an error, and
  HAPI does not for `is`. That is a specification-against-HAPI difference,
  resolved for HAPI. It is legitimate, not parity.

**`path.types.013`: correct.**
- `testTypes` has 101 cases, with 2 `failed` (`results:192-193`).
- `testType` (30) and `testInheritance` (24) have none failed.

**Registers and counts.**
- The preamble says twenty-two and lists DEV-0024 as pending. DEV-0024 has
  every field.
- `check-registries.sh` reports "registers agree" (22 deviations, 15 debt
  entries).
- Only the title of `path.types.011` changed, and the index carries it.
  `path` has 368 headings.

**Stale text.** No "unknown type … semantic error" text remains. The two
"semantic error" hits (`navigation.007`, `check.003`) are other behaviours.
The context-item wording is finding 1.

## Thirtieth review - commit `caf3579` (2026-11-17)

**Twenty-ninth-review finding 1: resolved in substance.**
- The spec now says every boolean condition fails the check (`case Check`,
  `:3544-3547`; `checkParamTypes`, `:3820-3831`), and that the second
  parameter is not type-checked. That second point holds because
  `checkParamTypes` stops at the end of the one-element type set.
- It also says there is no boolean form that passes, and that evaluation is
  graded with the check skipped.
- `path.utility.004` and task 48 (`tasks.md`) agree.
- The remaining errors are the quoted message (finding 1) and one example
  (finding 2).

**Twenty-ninth-review finding 2: resolved.** `path.utility.009` now quotes
`Reference to an unknown extension - double check that the URL '<url>' is
correct`. That matches `Messages.properties:368`, whose `''{0}''` is
MessageFormat's escaped single quote around the URL.

**Stale text.** `grep` for "context-relative" and "resolves against the
context item" across the change directory, `docs/` and `tasks/` finds
nothing.

**Registers.** `check-registries.sh` reports "registers agree" (22
deviations, 15 debt entries). No `#### Scenario:` heading changed.

## Thirty-first review - commit `2ea4f92` (2026-11-17)

**Finding 1: resolved.** The quoted text is exact:
- `{2}` is `FP_Boolean`, which is `http://hl7.org/fhirpath/System.Boolean`
  (`TypeDetails.java:66`).
- `{3}` is `TypeDetails.toString()` (`:474-476`): the `CollectionStatus` enum
  name, `SINGLETON` (`ExpressionNode.java:363-364`, no `toString` override),
  followed by the `HashSet` rendering of `getTypes()` (`:395-400`). That gives
  `SINGLETON[http://hl7.org/fhirpath/System.String]`.
- The template and prefix are `Messages.properties:371`.

**Finding 2: resolved.** `Patient.check(active.exists(), 'm')` reaches the
type failure:
- `Patient` resolves at entry.
- The condition is typed against the context item `Patient` (`:3267`).
- `active` is a `Patient` element, and `exists()` types as `boolean`.
- `checkParamTypes` then rejects it (`:3545`, `:3828-3830`).
- With the check skipped, `patient-example.xml` has
  `<active value="true"/>` (line 48), so evaluation returns the `Patient`, as
  stated.

**Registers.** `check-registries.sh` reports "registers agree" (22
deviations, 15 debt entries). No `#### Scenario:` heading changed.

## Thirty-second review - commit `3cc6bc9` (2026-11-17)

**`path.env.006`: correct.** Results lines `1057`-`1062` (the scenario cites
`:1057-1061`, which is one short):

| Case | Status |
|---|---|
| `isGuardAndOtherImplies` | `failed` |
| `isGuardOtherAndImplies` | `failed` |
| `isGuardOrOtherImpliesBad` | `passed` |
| `isGuardOrOtherImpliesNoParensBad` | `passed` |
| `isGuardOrInnerImplies` | `hapi-cannot-pass` |
| `isGuardOrInnerImpliesBad` | `hapi-cannot-pass` |

- The two failures' reason is "The function toString can only be used on
  primitives". That agrees with `path.check.005`'s message.
- The two `hapi-cannot-pass` cases are among the 13 in `path.oracle.016`'s
  list (the `isGuardOrInnerImplies` pair), with the same expression and
  differing `contextType` (`tests-fhir-r4.xml:1745-1751`).
- Two citation nits:
  - "`executeType` has no guard logic, `:3631`": line `:3631` is
    `toInteger`'s check. The rejecting call is
    `checkContextPrimitive(focus, "toString", …)` at `:3639`.
  - HAPI's recorded run checks against the resource root, not
    `Observation.effective` (`path.oracle.016`), so in HAPI's own run the
    failure is `toString` on `Observation`. In Frevo's context-aware run it is
    `toString` on the un-narrowed `Period`. Both are `failed`, so the parity
    holds.

**`path.env.008` / `.010`: correct.**
- `getChildrenByName` calls `item.listChildrenByName(name, false)`
  (`ENGINE:255-278`).
- With `checkValid = false`, an unknown name falls to `Base.getProperty`,
  which returns `null` (`Base.java:992-996`). `Resource.getProperty` knows
  only `id`, `meta`, `implicitRules` and `language` (`Resource.java:354-364`).
- So `.resourceType` is empty, and the check rejects it as an unknown name
  first, as stated.

**`path.env.015`: correct.**
- The check's explicit call passes `s` with its `%` (`:3173`).
- The no-value call passes the bare name (`:1586`, `executeContextType`).
- Evaluation strips the `%` (`:1784`, `s.substring(1)`; the scenario cites
  the `else` at `:1783`).
- The implicit calls pass `exp.getName()` (`:3186`, `:3214`).
- A host keyed only on the bare name returns `null` to the check, so
  `:3175` throws "Invalid FHIRPath Constant %owned". Correct.

**`path.env.016`: correct.** Implicit-before is asked first, under
`atEntry && appInfo != null && hostServices != null` (`:3184-3190`). A
non-empty answer returns at once, so `Patient.name` with a string for
`Patient` navigates `.name` on the string and is empty. `Patient` alone is the
string.

**`path.env.017`: the evaluation half is correct.** Implicit-after happens
only when navigation is empty (`:3211-3214`), and a host that answers in
implicit-before short-circuits. The check half is finding 1.

**Other env scenarios.**
- `env.001`-`.005` are built-in constants that the check types
  (`:3134-3162`).
- `env.012` and `.013` resolve through the host or the defined-variable
  table at both stages.
- None has the check-first issue.

**Registers and IDs.** `check-registries.sh` reports "registers agree" (22
deviations, 15 debt entries). No `#### Scenario:` heading changed. `path` has
368 headings.

**Stale text.** The other "`testDollarResource` … previously named" sentence
(`spec.md:410`) belongs to `path.navigation.021` and is accurate.

## Thirty-third review - commit `27615b7` (2026-11-17)

**Finding 1: resolved.**
- `path.env.017` now says the check rejects an unknown leading name with
  `FHIRPATH_UNKNOWN_NAME` (`FHIRPathEngine.java:1587-1593`).
- It says the check consults the host only under `atEntry && focus == null`
  (`:1585-1586`, the no-value mode), and that implicit-after is reachable only
  with the check skipped. All three statements are correct.

**Nits fixed.** `env.006` now cites `results:1057-1062` and `:3639`.

**Registers.** `check-registries.sh` reports "registers agree" (22
deviations, 15 debt entries). No `#### Scenario:` heading changed.

## Thirty-fourth review - commit `bd08451` (2026-11-18)

Scope: the diff `6d38d35..bd08451`, which covers `path.boundary.001`-`.017`,
the `spec.md` counts and registry, task 45, `DEV-0025`, `DEBT-0016` and the
`tasks/blockers.md` note. I read the upstream code before I read the delta:
`ENGINE:4290-4392` (`funcLowBoundary`, `funcHighBoundary`, `funcPrecision`),
`:3714-3732` and `:3896-3900` (the static check), `:1674-1735`
(`processDateConstant`), `UTIL:1631-1736`, `DTU:65-215`, `BDT:90-106`,
`:330-342`, `:432-552` and `:640-680`, `TimeType.java:59-146`,
`DecimalType.java:82-160`, `PrimitiveType.java:102-118`, and `I18nBase.java:163-240`.
The decimal arithmetic was checked with a line-for-line simulation of
`applyPrecision`, `minusOne` and `isZero`.

**Confirmed correct.**
- **Suite counts.** LowBoundary has 31 decimal cases, of which HAPI passes 22
  (`results.json:932-953`). The nine failures are at `:954-962`. HighBoundary
  has 27 decimal cases, of which HAPI passes 18 (`:969-986`), with nine
  failures at `:987-995`. All ten date, dateTime and time cases pass
  (`:964-968`, `:997-1001`). The 18 failures in `DEV-0025` add up.
- **`.001`-`.003`, `.006`, `.010`.** Every quoted value matches
  `applyPrecision`. The range check is `precision >= 0 && precision < 17`, for
  decimal and integer only (`ENGINE:4310`, `:4314`, `:4354`, `:4358`). The
  defaults are 8 for a decimal, 10 for a date, 17 for a dateTime and 9 for a
  time.
- **`.004` and `.007`.** `9`+1 is `:`, and `new BigDecimal` raises the
  quoted `Character : …` message. `minusOne("10")` is `"09"`, and
  `DecimalType` keeps that representation. `1.lowBoundary(0)` is `0` and
  `1.highBoundary(0)` is `2`.
- **`.005` and `.011`.** The fixture's values are `9.5e2`, `1.5e-3`,
  `-9.5e2` and `1.5E2` (`parameters-example-decimals.xml:7-19`). The results
  are:
  - `92`, then the `Character -` error on `1.45000000-3`, then `-:2`, which
    raises the `Character :` error, then `1.5`;
  - `:2`, which raises an error, then `9.550000002`, then the `Character -`
    error, then `-92`, then `1.6`. In the last one, `'E'` (0x45) is at least
    `'5'`, so the digit before it rounds up.
- **`.008`.** `base.copy()` keeps the unit, code and system, and the value is
  computed as for a decimal.
- **`.009`.**
  - The results are `DateTimeType` for both date and dateTime input.
  - At precision 10 or less the offset is dropped. Otherwise a stated offset is
    kept, or `+14:00`/`-12:00` is added.
  - `@2014-01-01T08` is held as `2014-01-01T08:00` after `setPrecision(MINUTE)`,
    hence `08:00:59.999-12:00`.
  - `dayCount` applies the Gregorian leap-year rule.
- **`.013`.** `funcPrecision` throws for `size != 1`, naming `highBoundary`.
  Under English plural rules, 0 and 2 both take `_other`. The boundaries
  return empty for an empty focus and name themselves for more than one item.
- **`.014`.** Literals of other types give empty, because `makeException` is
  not thrown. The static-check claims match `checkContextContinuous` and the
  `allowInteger` flag (`true` for the boundaries, `false` for `precision`).
- **`.015`, other than precision 14 (finding 2).** The precisions 4, 6, 7, 8,
  10, 14 and 17 are accepted, and any other throws
  `Unsupported Date precision for boundary operation: <n>`.
- **`.016`.** Precisions 2, 4 and 6 give `10:`, `10:30:` and `10:30:00.`, and
  `highBoundary(6)` gives `10:30:59.`. Other precisions throw
  `Unsupported Time precision for boundary operation: <n>`. `TimeType(String)`
  does not validate. `@T10:30` is held as `10:30`
  (`TimeType.setPrecision(MINUTE)`), which is why `precision()` is 4.
- **`.017`.** An argument with zero items or more than one raises the
  `Multiple Values` error naming `lowBoundary` (`ENGINE:4301`, `:4345`).
  `'2'` parses as 2. `1.5` raises `NumberFormatException`
  `For input string: "1.5"`. The message prefix is finding 6.

**Counts and IDs.**
- The `#### Scenario: [` headings number codegen 49, model 83 and path 375 (368
  at `6d38d35`, plus 7). That totals 507, as `spec.md` states.
- There are 17 `path.boundary.*` headings. No ID is duplicated across the
  three delta files, and no existing ID was renumbered or repurposed.
- The "twenty-five without a live case" list gains `.012`-`.017`, and each of
  those says so in its own text.

**Registers.**
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 16 debt entries).
- `DEV-0025` names `Debt: DEBT-0016`, and `DEBT-0016` names `Origin: DEV-0025`.
- The linkage is coherent. The content of both entries needs the corrections
  in findings 1 and 2.

**Oracles.**
- `.001`-`.011` name their suite group and cases.
- `.012`-`.017` are marked differential-only, with seam assertions.

## Thirty-fifth review - commit `ee2a0d4` (2026-11-18)

Scope: the diff `d7f8901..ee2a0d4`, which covers `path.boundary.012`-`.015`,
`.017`, the new `.018` and `.019`, the `spec.md` counts and registry, task 45,
`DEV-0025` and `DEBT-0016`. I re-derived every new value from the pinned source,
using the same line-for-line simulation of `applyPrecision`, `minusOne` and
`isZero` (`UTIL`) and a hand trace of `BDT.parse`.

**Finding 1 (`.012`): resolved.**
- `1.587 'cm'.lowBoundary(-1)` and `highBoundary(-1)` both cut the text to
  `""`. `DecimalType.setValueAsString` reads a blank string as null, so each
  result is a Quantity with no value.
- `12.5 'cm'.lowBoundary(-1)` and `highBoundary(-1)` both give `1`. The cut
  index is 1 and `'2'` is below `'5'`, so neither rounds.
- At `-2` the cut index is `-1`, and `nv.charAt(-1)` throws before the
  `down` test is evaluated. Leaving the JDK's message uncompared is
  reasonable.

**Finding 2 (`.015`): resolved.** Each case traced through `BDT.parse`
(`:432-552`) and `getOffsetIndex` (`:330-342`):
- `...T08:00:+14:00` has seconds `+1`, which reads as 1, and parses at SECOND
  precision.
- `...T08:00:-12:00` has seconds `-1`, which is out of range, so
  `throwBadDateFormat` raises `Invalid date/time format: "<text>"`.
- `+08:00` and `-05:00` give seconds `+0` and `-0`, which both read as 0 and
  parse. `-10:00` gives `-1`, the error.
- `@2014-01-01T08:05Z` is held as `2014-01-01T08:05Z`. `splitTimezone` takes
  the `Z` branch, so the text is `2014-01-01T08:05:Z` (18 characters). The
  offset index is 17, which passes, and the time part `08:05:` has length 6.
  So `validateLengthIsAtLeast(value, 19)` (`BDT:863-867`) raises the
  `Invalid date/time format` error for that text. Correct.

**Finding 3 (`.018`): resolved.** The values are as stated:
- `0.lowBoundary()` is `-0.50000000`, and `0.highBoundary()` is `0.50000000`.
- `0.lowBoundary(0)` is `-0`, and `0.highBoundary(0)` is `1`.
- `0.0.lowBoundary(1)` and `(-0.0).lowBoundary(1)` are `-0.5`, and
  `0.000.highBoundary(1)` is `0.5`.
- The zero branch drops the exponent (`UTIL:1638-1639`, `:1704-1705`).

**Finding 4 (`.014`): resolved.**
- The fixtures exist and hold the cited elements:
  - `patient-example.xml:78` has `telecom.rank` (`positiveInt`).
  - `patient-example.xml:95` has the Patient's `gender` (`code`).
  - `patient-glossy-example.xml:4` has `meta.lastUpdated` (`instant`).
- `Base.hasType` compares the exact `fhirType()`, so every arm misses. The
  unthrown `makeException` then gives empty for all three functions.

**Finding 5 (`.019`): resolved.** `getNamedValue` (`ENGINE:4643-4649`)
returns `null` for a Quantity with no value. `UTIL:1632-1634` and `:1698-1700`
then throw the two quoted messages.

**Finding 6 (`.013`, `.017`): resolved.** The full texts match
`Messages.properties:332` and `:371`, with the arguments shifted by
`I18nBase.formatMessagePlural`.

**Registers and documents.**
- `DEV-0025`'s HAPI paragraph and `DEBT-0016`'s new bullets match the
  scenarios. Task 45 now lists `.001`-`.019` and the corrected behaviours.
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 16 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.

**Counts and IDs.**
- The `#### Scenario: [` headings number codegen 49, model 83 and path 377
  (19 of them `path.boundary.*`), 509 in total, as `spec.md` states.
- No ID is duplicated, and no heading was removed or renumbered.
- The no-live-case list (27) now includes `.012`-`.019`, and each says
  "differential-only" in its own text.

## Thirty-sixth review - commit `ecf0496` (2026-11-20)

Scope: the diff `d0b5a1d..ecf0496`. It covers `path.datetime.004`, `DEV-0018`,
`path.navigation.001`/`.005`/`.024`, `path.string.015`, `path.errors.002`, the
`path.oracle.009` note, `path.compare.013` and the new `.015`,
`DEV-0017`/`DEBT-0017`, the `spec.md` counts, the text of tasks 1 and 35, and
the new blocker "element rewrite after the evaluation". I read `reference/`
before the delta.

**Suite zone (`path.datetime.004`, `DEV-0018`, task 1): correct.**
- `FHIRPathTests.java:174` calls
  `TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"))`.
- On a local JDK 25, `TimeZone.getTimeZone("UTC+1100")` returns id `GMT` with
  raw offset 0. A custom id needs the `GMT` prefix, and the unknown id falls
  back to GMT.
- `couldBeTheSameTime` (`BaseDateTimeType.java:937-956`) widens an
  offset-less operand by 14 hours each way. With the operand at `10:00Z`, the
  window contains `15:00Z`, so the answer is empty. With it at `23:00Z` the
  previous day, the window does not, so the answer is `false`.
  `testEquality23` and `testNEquality17` expect empty and are recorded
  `passed` (`hapi-8.12.0-results.json:496`, `:530`). So offset 0 is the zone
  that produced the record.
- `testDateTimeGreaterThanDate2` (`:104`) and `testToday1`/`2` and
  `testNow1`/`2` (`:460-463`) are `passed`. At offset 0, `today()` and Zulu
  `now()` always fall on the same day.
- The `@2015-02-04T10:00:00 < @2015-02-04T05:30:00Z` values are right for both
  zones.
- `JUnit` creates a new `FHIRPathTests` instance per method (there is no
  `@TestInstance`), so the `resources` cache (`:120`, `:207-214`) does not
  carry `testLessThan28`'s mutation into later cases. `testExtension1`-`3`
  only test `exists()` and `empty()` anyway.

**Navigation and join: correct.**
- `testJoinEmpty` is `failed` (`:457`). `funcJoin` (`ENGINE:4547-4567`)
  always adds one `StringType`.
- `testSimpleNone` (`name.suffix`, no output) is `passed` (`:18`).
- `testEscapedIdentifierUnicode` failed at the check stage (`:20`). It is the
  only non-pass in `testBasics` (8 cases); `testMiscellaneousAccessorTests` (3)
  and `testObservations` (10) are all `passed`.
- `.001`, `.005`, `.024` and `path.string.015` are consistent with each other.

**`path.oracle.009`: correct.**
- `tests-fhir-r4.xml:413` is the only `type="Quantity"` output, in
  `testTypes`.
- It is recorded `passed` (`:186`), as are `LowBoundaryQuantity` (`:963`) and
  `HighBoundaryQuantity` (`:996`).

**`path.errors.002`: correct as far as it goes; see finding 1.**
- The message text matches `Messages.properties:303`, and `FHIRPATH_LOCATION`
  is `:340`.
- `rep` behaves as quoted (`PathEngineException.java:81-93`).
- `SourceLocation` starts at (1, 1) (`FHIRLexer.java:56`), and `next()` records
  the start after skipping whitespace (`:143-147`).
- For the three stated expressions the holder is the literal `1` node. In
  `(1=1) and 1 < 'a'`, `gatherPrecedence` for `<` takes the `else` branch and
  wraps the `1` node in a `newGroup`, but `<` stays on the inner `1` node,
  which has a start. So `@char 1`, `11` and `21` are right.
- `HapiWorkerContext` extends `I18nBase`, the same message source the suite
  uses.

**`path.compare.013`/`.015`: correct as far as they go; see finding 2.**
- `processConstant` returns a `FHIRConstant` for `@` (`ENGINE:1335-1336`).
- `resolveConstant`/`processDateConstant` (`:1653-1736`) build a new
  `DateTimeType` on each evaluation.
- `funcSelect` (`:6263-6273`) and `changeThis` (`:4726-4734`) pass the same
  object as `$this`.
- `preOperate` short-circuits `and` only on a `false` left (`:1542-1549`).
- `isDateChar` (`FHIRLexer.java:335-338`) ends `@...+02:00` before `.toString`.
- `compareDateTimeElements` copies non-`DateTimeType` operands (`:636-637`)
  and rewrites only above DAY precision.
- Its only callers are the four ordered operators (`:2421`, `:2468`, `:2516`,
  `:2566`) and equivalence (`:2218`, which does not rewrite).
- `convertToString` returns `primitiveValue()`.
- I traced every stated value of `.015`, including the offset-0 offset-less
  case and the parse-once, evaluate-twice case, and each one holds.

**Registers.**
- `DEV-0017` now lists `path.compare.015` and `DEBT-0017`.
- Moving `testLessThan28` out of the quantity list is right: it is the
  `dateTime` case (`tests-fhir-r4.xml:1052`).
- `DEBT-0017` names a trigger, a size and an upstream report. It is the
  flag-not-built record that constitution section 1 requires.
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.

**Blocker.** "Element rewrite after the evaluation" is correctly raised as an
owner decision, and leaving it ungraded until the owner rules is acceptable.
Finding 2 is a separate point: it concerns the evaluation's own output.

**Counts and IDs.**
- The `#### Scenario: [` headings number codegen 49, model 83 and path 378,
  510 in total, as stated.
- `path.compare.*` has 15 headings, and no ID is duplicated, removed or
  renumbered.
- The no-live-case list has 28 entries and includes `.015`, which says
  "differential-only".
- `spec.md` says 6 scenarios were rewritten. The commit subject says "five";
  the record is `spec.md`, and the six it lists are correct.

## Thirty-seventh review - commit `444560a` (2026-11-20)

Scope: the diff `8efea6d..444560a`. It covers `path.errors.002` and its
requirement sentence, `path.compare.013` and `.015`, the revision note and
registry lines in `spec.md`, tasks 35 and 53, `DEBT-0017` and the blockers
note. I traced the parse trees and functions again from the pinned source
(`ENGINE` at `73c432b5`).

**Finding 1 (`path.errors.002`): resolved.**
- `1 + 1 < 'a'`: the `+` pass of `organisePrecedence` (`ENGINE:1224-1237`)
  takes the entry branch of `gatherPrecedence`. It wraps `1 + 1` in `G`, a
  `newGroup` node (`:1315-1321`) with no start. It then moves `<` and
  `opNext 'a'` onto `G`, which becomes the root. The `<` pass finds no work.
  `execute` reaches `operate(..., last = G)` (`:1510-1522`). `makeException`
  (`:589-590`) passes `G.getStart() == null`, and `rep(null, non-empty)` is
  `""`. The message is bare.
- `(1=1) and 1 + 1 < 'a'`: the `+` pass takes the `else` branch. It builds
  `G1 = newGroup(1)` as `opNext` of `(1=1)`, moves `<` and `'a'` onto `G1`,
  and clears the inner `1`'s operation. The `<` pass then wraps `G1` in a
  second group `G2`, but `<` stays on `G1`, and `G1` has no start. So `G1` is
  the holder and the message is bare. The scenario's statement that "the `<`
  node is again the group the `+` level built" is exact.
- `(1 + 1) < 'a'`: the `(` branch of `parseExpression` (`:1135-1143`) turns
  `result` into the group, and that node's start is set at `:1076`, column 1.
  `<` sits on it, and no precedence pass has work to do. So the message ends
  ` (@char 1)`, and the offset is 0.
- `a | b = c` and `x * 2 > 'y'` gather the same way: union and
  multiplicative operators bind tighter than `=` and `>`.
- A `null` holder goes through `:591-592` with no location, so the message is
  bare.
- "No Frevo offset" matches HAPI. No `PathEngineException` constructor
  assigns `location`. Within the R4 `fhirpath` package, the only
  `setLocation` is the engine's own `setLocation(String)` (`:236`), which
  feeds `FHIRPATH_LOCATION`, not the exception.
- The narrowed final AND and the requirement sentence are now consistent with
  `rep`.

**Finding 2 (`path.compare.013`, `.015`): resolved.**
- `funcWhere` (`ENGINE:6249-6260`) adds the very `item` it compared.
- `$this` resolves to `context.getThisItem()`, the object `changeThis` set
  (`:4726-4734`), so `compareDateTimeElements` (`:635-648`) rewrites that
  object.
- `select($this)` (`:6263-6273`) returns the same object again.
- `.013`: the birthTime `valueDateTime` is the resource's own `DateTimeType`.
  `1974-12-25T14:35:45-05:00` in Zulu is `1974-12-25T19:35:45Z`.
- `.015`: `2012-04-15T10:00:00+02:00` becomes `2012-04-15T08:00:00Z`. An
  uncompared `(@...).select($this)` comes from a fresh `processDateConstant`
  value and keeps its text.
- Both scenarios now say that dropping the rewrite before the result is
  produced does not meet them.

**Registers, tasks and blocker.**
- `DEBT-0017`'s owed text names the returned value.
- Tasks 35 and 53 match the scenarios.
- The blockers note lists the tests the oracle-builder owes
  (`path_parse_errors.rs`, `path_compare_equality.rs`).
- The open decision "element rewrite after the evaluation" remains correctly
  an owner question. It concerns persistence past the result, which no
  scenario grades.
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.

**Counts and IDs.**
- The headings number codegen 49, model 83 and path 378, 510 in total,
  unchanged and as stated.
- No ID is duplicated, added, removed or renumbered, and the registry lines
  for `.015` and `errors.002` match the headings.

## Thirty-eighth review - commit `56d3bf3` (2026-11-20)

Scope: the diff `16e6719..56d3bf3`. It covers the column rule and offset in
`path.errors.002`; the suffixes in `path.boundary.013`/`.017`,
`path.check.005`/`.006` and `path.utility.003`; the start-less and
direct-construction lists; the `path.compare.015` extension; `DEV-0017`,
`DEBT-0017`, tasks 35 and 53, `spec.md` and the blockers note. I re-derived
every column by hand from `FHIRLexer.java` and `SourceLocation.java` at
`73c432b5`.

**Column rule: correct, and I had it wrong before.**
- `skipWhitespaceAndComments` calls `checkChar` (`setColumn(column + 1)` for a
  non-newline character) and then `incColumn()` (`FHIRLexer.java:321-324`), so
  each whitespace character adds 2.
- A token adds `cursor - currentStart` once (`:285`).
- CR or a lone LF: `newLine()` sets column 1, then `incColumn()` makes it 2.
  An LF after a CR adds only 1.
- A `//` comment adds 1 per character from the first `/` (`:294-303`). A
  `/* */` comment adds 2 per character, plus 2 for the closing `*/`
  (`:304-320`).
- HAPI's own record confirms the rule: `isEffectOnLogic1` reports
  ` (@char 37)` at byte offset 32 after four spaces (`results.json:1046`), and
  `isEffectOnLogic1Base` reports ` (@char 7)` (`:1043`). The other recorded
  suffixes are consistent; for example ``Patient.`name` `` is
  ` (@char 9)` (`:20`).
- My thirty-sixth and thirty-seventh reviews accepted `@char 11`/`21`. That
  was wrong, and this commit fixes it.

**Values re-derived.**
- `(1=1) and 1 < 'a'`: `(1=1)` reaches 6, the space 8, `and` 11, the space 13.
  So ` (@char 13)`, byte offset 10.
- `(1=1) and (1=1) and 1 < 'a'`: 25, byte offset 20.
- `(1=1) and` LF `1 < 'a'`: ` (@line 2 char 2)`.
- `{}.precision()`: `{}` is one token (`:203-208`), so the column is 4 and the
  offset 3.
- `(1.5 | 2.5).precision()`: column 15 is right, but the offset is 12, not 13
  (finding 2). The `lowBoundary` form is the same column 15.
- `1.587.lowBoundary({})`: the number loop stops at the second `.`, so the
  column is 7 and the offset 6. The holder is the function node (`:4301`,
  `:4345`).
- `Patient.check(true, 'm')`: column 9. The holder is the `check` node,
  through `checkParamTypes(exp, ...)` (`:3545`, `:3830`).
- `path.check.005`: column 37, offset 32. `path.check.006`: column 7.

**Start-less holders and direct constructions.**
- `newGroup` (`:1315-1321`) and the indexer node (`:1188-1192`) set no start.
- A null holder goes through `:580`/`:592`.
- `gatherPrecedence` moves an operation with `setOperation` only, so a
  `newGroup` node has no `opStart` either.
- `getOpStart()` is used exactly at `:2063`, `:2080`, `:2725`, `:2731`,
  `:2763` and `:2801` (`:2801` is `*`; finding 3).
- `opStart` is a fresh `currentStartLocation` object for each token (`:147`),
  so nothing is aliased.
- Every direct `new PathEngineException` is listed except `:3102`
  (finding 3).
- `PathEngineException` never sets `location`, so an optional Frevo offset,
  present exactly when the suffix is, matches what HAPI exposes.

**`path.compare.015` extension.**
- `funcNow` returns `DateTimeType.now()` (`:4772-4776`).
- `dateAdd` copies (`:2715-2716`). A `dateTime` copy keeps `+02:00`, so
  `+ 1 day` and then the rewrite gives `2012-04-16T08:00:00Z`.
- A `date + quantity` result stays a `DateType` (`:2703-2704`).
- `today()` is a `DateType` (`:4779-4783`).
- `lowBoundary` of `@2012-04-15T10:00:00+02:00` at the default 17 is
  `2012-04-15T10:00:00.000+02:00`, which becomes `2012-04-15T08:00:00.000Z`.
- `toDateTime` throws `FHIRPATH_NOT_IMPLEMENTED` (`:4976-4980`, `DEV-0021`).
- The one wrong claim is the `date` boundary (finding 1).

**Registers, counts and validators.**
- `DEV-0017` and `DEBT-0017` match the scenario text, so finding 1 will need
  their "a `dateTime` boundary" wording widened the same way.
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 378, 510 in total. No ID
  is duplicated, added, removed or renumbered.

## Thirty-ninth review - commit `4331265` (2026-11-20)

Scope: the diff `bc22e8c..4331265`. It covers `path.compare.015`'s boundary
rule, the `path.boundary.013` offset, the direct-construction list in
`path.errors.002` and its title, `path.strictness.003`, `DEV-0017`,
`DEBT-0017`, tasks 35 and 53, and `spec.md`.

**Thirty-eighth finding 1 (boundary rewrite): resolved except for precision
14 (the new finding).**
- `applyDatePrecision` (`DateTimeUtil.java:130-144`) keeps 4, 7 or 10
  characters at precision 4, 6/7 and 8/10. Below 11 no offset is appended
  (`:84-86`, `:174-176`), so the built value is YEAR, MONTH or DAY precision
  and is not rewritten.
- At 17 it keeps the whole text, with a stated offset or `+14:00`/`-12:00`.
- `@2014-01-01.lowBoundary(17)` is `2014-01-01T00:00:00.000+14:00`, which in
  Zulu is `2013-12-31T10:00:00.000Z`. `compareTimes` then sees 2013 < 2015,
  so `where` keeps it.
- `@2014-01-01T08.lowBoundary(8)` is `2014-01-01` at DAY precision.
  `compareTimes` returns -1 on the year before any precision test, so `where`
  returns it unchanged.
- `date + quantity` (`ENGINE:2703-2704`) and `today()` (`:4779-4783`) are
  `DateType` values.

**Thirty-eighth finding 2: resolved.** The byte offset is now 12, and the
column 15 is unchanged.

**Thirty-eighth finding 3: resolved.**
- `:2801` is now named as `opTimes`, with its operator start on the `*`.
- `:3102` (`new PathEngineException(e)` in `opMod`) is listed as having no
  location, no suffix and no offset.
- The `getOpStart()` list (`:2063`, `:2080`, `:2725`, `:2731`, `:2763`) is
  complete.

**Thirty-eighth finding 4: resolved.**
- `Messages.properties:301` through `MessageFormat` (`I18nBase.java:206`)
  gives `Attempt to use 'as()' on more than one item ('3', ...)`.
- `patient-example.xml` has three `Patient.name` elements (`:50`, `:56`,
  `:60`; `:122` is the contact's).
- `funcAs`'s holder renders as `as(HumanName)` (`ExpressionNode.java:402-421`).
- `opAs`'s holder is the `Patient` node that carries the operation, and it
  renders as `Patient.name as HumanName` (`:451-461`).
- Both are constructed without a location. `funcAs` builds `FHIR.HumanName`
  and `opAs` uses the bare name, each through `isKnownType`.
- The narrowed title, the new AND clause and the task 53 text are accurate.

**Registers, counts and validators.**
- `DEV-0017` and `DEBT-0017` now state the precision rule (14 or 17
  rewritten, 10 or less not).
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 378, 510 in total. No ID
  is duplicated, added, removed or renumbered.

## Fortieth review - commit `b73a460` (2026-11-20)

Scope: the diff `5c510fb..b73a460`: the precision-14 clause of
`path.compare.015`, and its note in `spec.md`.

**Thirty-ninth finding: resolved.**
- The clause now references `path.boundary.015`.
- I traced each of the five values through `DateTimeUtil.lowBoundaryForDate`
  and `highBoundaryForDate` (`:65-90`, `:156-181`, `applyDatePrecision`
  `:130-144`, `splitTimezone` `:183-200`). I then traced the resulting text
  through `BaseDateTimeType.parse` (`getOffsetIndex` `:330-342`, called at
  `:470`; zone `:479`; seconds `:491`).
- For every offset text below, `getOffsetIndex` passes its consistency test:
  the one sign found after index 16 is at 17, and 17 - 2 equals the sum of the
  three indexes, with -1 for each one not found.

The five values:
- `@2014-01-01T08.lowBoundary(14)`:
  - The text is `2014-01-01T08:00:+14:00`.
  - The time part is `08:00:` (length 6), so the seconds are `+1` = 1, at
    SECOND precision, and the zone is +14:00.
  - The instant is `2014-01-01T08:00:01+14:00`, which in Zulu is
    `2013-12-31T18:00:01Z`.
  - 2013 < 2015, so `where` keeps it. Correct.
- `@2014-01-01T08:05+08:00.lowBoundary(14)`:
  - `splitTimezone` takes the `+` branch, giving
    `2014-01-01T08:05:+08:00`.
  - The seconds are `+0` = 0, so the instant is 08:05:00+08:00, which in Zulu
    is `2014-01-01T00:05:00Z`. Correct.
- `@2014-01-01.lowBoundary(14)`:
  - The `date` focus is widened to `2014-01-01T00:00:00.000` and cut to 17
    characters.
  - It has no offset and the precision is above 10, so `+14:00` is appended.
  - The seconds are `+1`, which gives `2013-12-31T10:00:01Z`. Correct.
- `@2014-01-01T08:05-05:00.highBoundary(14)`:
  - `splitTimezone` takes the `-` branch, because the last `-` is after the
    `T`. That gives `2014-01-01T08:05` and `-05:00`.
  - It is widened to `...08:05:59.999` and cut to `2014-01-01T08:05:`, then
    the offset is appended.
  - The seconds text `-0` reads as 0, which is in range. The zone is -05:00.
  - 08:05:00-05:00 is `2014-01-01T13:05:00Z`. Correct.
- `@2014-01-01T08.highBoundary(14)`:
  - The focus has no offset, so the text is `2014-01-01T08:00:-12:00`.
  - The seconds `-1` fall below the lower bound 0, so `parseInt` calls
    `throwBadDateFormat`. That happens inside `new DateTimeType`
    (`ENGINE:4364`), before `where` runs. Correct.

All five operands carry an offset, so none of the values depends on the
evaluation zone. The Zulu form renders at SECOND precision with no fractional
seconds, because `parse` leaves `myFractionalSeconds` empty when
`timeLength <= 8`.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (23
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 378, 510 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-first review - commit `4fa72be` (2026-11-20)

Scope: the diff `c692b65..4fa72be`: `path.strictness.005`, `.006`,
`path.api.005`, the new `path.errors.006` and `.007`, the note in
`path.errors.002`, `DEV-0026`, `DEV-0027`, task 53, `spec.md` and the
blockers note.

**`path.strictness.005` and `DEV-0026`: correct.**
- `initFlags` (`ENGINE:211-216`) and `FhirPathR4`'s constructor
  (`HAPI .../fluentpath/FhirPathR4.java:34-40`) both disable case sensitivity.
- `funcAs` (`:5155-5193`) builds `FHIR.Code`. `isKnownType` (`:1904-1935`)
  reaches `HapiWorkerContext.fetchTypeDefinition` (`:412-414`), which goes
  through `fetchResource` (`:437-444`) to the validation support.
- `DefaultProfileValidationSupportBundleStrategy.fetchStructureDefinition`
  (`:276-297`) retries an upper-case initial lower-cased for R4, R4B and R5.
- The loop then compares `fetchTypeDefinition("code").getType()` with
  `compareTypeNames`, which ignores case only when case is not enforced
  (`:1896-1902`). So the result is `male` in the default state and empty when
  enforced.
- `opAs` (`:1874-1893`) does the same against `fhirType()`.
- `funcOfType` uses `equals` (`:5226`), so it is empty in both states.
- `as(quantity)` gets no retry and so fails `isKnownType`. The error is
  `FHIRPATH_INVALID_TYPE` (`Messages.properties:336`), built without a
  location (`:5164`).
- The suite's worker resolves exactly (`BaseWorkerContext.java:1183-1188`).
- `FhirPathEngineR4Test.testAs` (`:93-115`) is as quoted.
- The suite's mis-cased names cannot reach the retry: every `as`/`ofType`
  argument names a real type (`HumanName`, `Period`, `Quantity`, ...), and
  `is` does not call `isKnownType` (`funcIs`, `:5102-5150`).
- `DEV-0026` is HAPI's server-facing behaviour, so choosing it follows the
  owner's ruling.

**`path.strictness.006`: correct.** In `getChildrenByName` (`:255-279`),
`"value".startsWith("value")` sets `tn = ""`. The test
`v.fhirType().equalsIgnoreCase("")` then drops the `Quantity`, so the result
is empty and `exists()` is `false`.

**`path.errors.006`: correct.**
- `-` is a single token (`FHIRLexer.java:149-153`).
- The `Unary` wrapper takes its operation from `lexer.take()`, which has
  already run `next()` (`FHIRLexer.java:101-105`). It is then given
  `setStart(getCurrentLocation())` (`ENGINE:1082-1088`).
- For `-'a'`: `-` takes the column to 2 and `'a'` to 5, so the suffix is
  ` (@char 5)` and the byte offset 4.
- For `- 'a'`: 2, then 4 after the space, then 7, so ` (@char 7)` and offset 5.
- The wrapper is proximal and carries the operation, and `case Unary` pushes
  `0` (`:1470-1471`). So `last` is the wrapper (`:1510-1522`).
- `opMinus` passes every guard for `0` and `'a'` and reaches
  `FHIRPATH_OP_INCOMPATIBLE` (`:2977-2978`, `Messages.properties:350`). The
  quoted message, ` -:` included, is right.

**`path.errors.007`: correct.**
- `funcIif` raises `FHIRPATH_NO_COLLECTION` with the `iif` node (`:5007-5009`,
  `Messages.properties:344`).
- In `(1 | 2).iif(true, 'a')`: `(` takes the column to 2, `1` to 3, the space
  to 5, `|` to 6, the space to 8, `2` to 9, `)` to 10 and `.` to 11. So `iif`
  is at column 11, byte offset 8.
- `checkSingleton` (`:3839-3843`) only adds a warning.
- In `1.is('x')`, the number loop steps back over a trailing `.`
  (`FHIRLexer.java:160-170`), so `1` is one token and `is` is at column 3,
  offset 2.
- `funcIs` (`:5102-5109`) formats `texp.getKind()`, an enum (`Kind`,
  `ExpressionNode.java:43-45`), so its text is `Constant` or `Name`. The
  outer node's kind is passed in the second branch too, so `Name`. The
  `Messages.properties:352` text matches.

**`path.api.005` and `DEV-0027`: see findings 1 and 2.**
- The HAPI facts are right:
  - `FhirPathR4.java:72-77` uses `isAssignableFrom`, with `getSimpleName()`
    and `getName()`, and `Msg.code(256)`.
  - `IdType extends UriType`, and `CodeType` and `MarkdownType` extend
    `StringType`.
  - `Enumeration extends PrimitiveType`, so it is neither a `CodeType` nor a
    `StringType`.
  - `Age`, `Count`, `Distance` and `Duration` extend `Quantity`.
- The FHIR-side derivations (`id`, `code` and `markdown` from `string`;
  `url`, `canonical`, `oid` and `uuid` from `uri`) match R4's
  `baseDefinition`s.
- Assessment of the choice, as the coordinator asked. Constitution section 1
  makes `docs/deviations.md` the exception mechanism for cases where **HAPI
  and the specification disagree**, and tells us to choose HAPI's behaviour
  there. `DEV-0027` states "Specification requirement: none in FHIRPath". So
  it is not that kind of case, and the standing ruling points to HAPI's
  semantics.
- Recording it as "pending owner" is the right form. The choice itself has a
  plausible case: Java classes do not exist in Rust, and HAPI rejects a `code`
  held as an `Enumeration` when a `CodeType` is requested. But the case
  rests on the two incorrect premises in finding 2, and the surface lacks the
  interface kinds that HAPI's real callers use (finding 1).
- An equivalent of HAPI's semantics is buildable without Java: a fixed table
  that maps each FHIR type to HAPI's R4 class ancestry, including
  `Enumeration` for a bound `code`, plus the interface kinds. So
  "reproducing the Java class tree" is a cost, not an impossibility.
- Until the owner rules, I would expect the open decision to be an entry in
  `tasks/blockers.md`, not only a mention in a resolution note.
- Not a finding for this commit: `FhirPathR4` also wraps engine errors as
  `HAPI-0255: ` + message (`:69`) and parse errors as `HAPI-2409: ` + the
  exception (`:49`). The starter registers `FhirPathFilterInterceptor`
  (`StarterJpaConfig.java:432`), which surfaces them as
  `HAPI-0327: Error parsing FHIRPath expression: ...`
  (`FhirPathFilterInterceptor.java:80-84`). That is where the
  `path.errors.*` messages become observable over HTTP. The server change
  that ports the interceptor must specify that wrapping.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (25
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 380, 512 in total.
- `path.errors.*` has 7 headings. No ID is duplicated, removed or renumbered.
- The no-live-case list (30) adds `.006` and `.007`, and each of them says
  "differential-only".

## Forty-second review - commit `e54ccc6` (2026-11-20)

Scope: the diff `637010a..e54ccc6`. It covers `path.api.005`, the withdrawal
of `DEV-0027`, the new out-of-scope row in `proposal.md`, task 54, `spec.md`
and the blocker "the R4 Java class ancestry table". `MODEL` is
`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model` at `73c432b5`, and `API`
is `hapi-fhir-base/.../org/hl7/fhir/instance/model/api` at `v8.12.0`.

**Forty-first finding 1 (interface kinds): resolved.**
- The primitive, composite and reference kinds are now specified, with the
  callers cited. I re-checked those citations against the HAPI sources.
- `PrimitiveType` implements `IPrimitiveType` (`MODEL/PrimitiveType.java:46-47`).
- `IBaseEnumeration` extends `IPrimitiveType`, so an `Enumeration` is a
  primitive.
- `IBaseReference`, `IBaseMetaType`, `INarrative` and `IBaseExtension` each
  extend `ICompositeType` (`API`).
- `Reference` reaches `ICompositeType` through `BaseReference`, `Narrative`
  through `BaseNarrative`, and `Extension` through `BaseExtension`.
- I scanned every `MODEL` class that extends `Type`, `BackboneType`,
  `Quantity` or a `Base*` datatype. All of them implement `ICompositeType`,
  directly or through one of those interfaces, except `BackboneType` itself
  (abstract) and `MoneyQuantity` and `SimpleQuantity`, which inherit it from
  `Quantity`.
- `BackboneElement` (`:49`) implements only `IBaseBackboneElement`, which does
  not extend `ICompositeType`. So `ContactComponent` (`Patient.java:230`) is
  not composite.

**Forty-first finding 2: resolved.** `DEV-0027` is removed. The register
count and pending list are updated, and `spec.md` says the number is not
reused.

**Class facts.**
- `IdType extends UriType` (`IdType.java:120`).
- `CodeType` and `MarkdownType` extend `StringType`.
- `UrlType`, `CanonicalType`, `OidType`, `UuidType` and `SidType` extend
  `UriType`.
- `PositiveIntType` and `UnsignedIntType` extend `IntegerType`.
- `DateType`, `DateTimeType` and `InstantType` extend `BaseDateTimeType`.
- `Enumeration extends PrimitiveType` (`:76`).
- `Attachment.contentType` is a `CodeType` with a binding (`Attachment.java:53-58`).
- `Observation.status` is an `Enumeration` (`Observation.java:1715`).
- `Patient.gender` is an `Enumeration`, but `Patient.java:269` is the
  **contact's** `gender`. The `Patient` field is at `:1508`. This is a
  citation slip, not a finding.
- `Patient`, `DomainResource` and `Resource` are at the cited lines.
- `filterIdType` (`ENGINE:280-284`) keeps an `IdType`.

**Message.**
- `Msg.code` is `String.format("%s-%04d: ", "HAPI", code)` (`Msg.java:23`,
  `:31-32`), so the prefix is `HAPI-0256: `.
- `FhirPathR4.java:72-77` uses `getSimpleName()` for the item and `getName()`
  for the requested type, so a nested class renders as `ContactComponent` and
  `org.hl7.fhir.r4.model.Patient$ContactComponent`.
- `evaluateFirst` is `evaluate(...).stream().findFirst()` (`:80-89`), so it
  checks the whole result.
- `FhirPathTest.testEvaluateWrongType` (`hapi-fhir-validation`, `:68-80`)
  asserts the `HumanName` / `StringType` message suffix.
- `:49` and `:69` are the `2409` and `0255` wraps, and
  `FhirPathFilterInterceptor.java:84` is `0327`. The out-of-scope row names
  `add-server-skeleton`.

**Table examples.** All are correct under HAPI. In detail:
- Primitive: accepts `gender`, `active` and `birthDate`, and rejects `name`
  (`HumanName`). The union puts `active` first, so the first rejected item is
  the `HumanName`.
- Composite: accepts `code` and `subject`, and rejects `status`
  (`Enumeration`) and `contact` (`ContactComponent`).
- Reference: accepts `subject`, and rejects `code` (`CodeableConcept`).
- FHIR type names:
  - `family` as `string` is accepted, and so is `id` as `uri`.
  - `name` as `string` is rejected (`HumanName`), and so is `id` as `string`
    (`IdType`).
  - `gender` as `code` is rejected (`Enumeration`), and so is `birthDate` as
    `string` (`DateType`).
- The exception is `text.div` (finding 1).

**The ancestry-table blocker: framed as an owner decision, with thin options.**
- The blocker correctly identifies the conflict with constitution section 4
  ("never translated from upstream Java"). It correctly notes that the
  `Enumeration`-versus-`CodeType` split does not follow from the
  StructureDefinitions (`Attachment.contentType`). It correctly forbids a
  hand-written table meanwhile, and asks for an ADR and a provenance entry
  under option (a).
- Section 10 adds a further point. `org.hl7.fhir.core`'s model sources carry
  HL7's BSD-style licence header, not Apache-2.0, so a derived table needs
  that notice carried and the derivation recorded in `docs/provenance.md`.
  The blocker should say so.
- I would ask the proposer to add two concrete alternatives to "(b) names
  another source":
  - **(c)** Generate the table once in CI by **reflection over the pinned
    HAPI 8.12.0 jars**: `Class.getSuperclass`/`getInterfaces`, the `@Child`
    field types, and a live `getProperty` probe. Commit it as a recorded
    oracle artifact, as `hapi-8.12.0-results.json` is. This reads no Java
    source, and it captures runtime substitutions such as `Narrative.div`,
    which a source scan of `@Child` declarations misses (finding 1).
  - **(d)** Derive the table from the StructureDefinitions plus a small,
    reviewed exception list of HAPI generator choices. This keeps section 4
    intact at the cost of a maintained list.
  
  Neither alternative changes the scenario, so this is not a finding.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (24
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 380, 512 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-third review - commit `429b868` (2026-11-20)

Scope: the diff `295b604..429b868`. It covers the `Narrative.div` exception
in `path.api.005`, the new `path.navigation.025`, the
`Patient.java:1508` citation fix, the counts, task text, and the
ancestry-table blocker's options and licence paragraph.

**Forty-second finding: resolved.** `path.api.005` states that `div` by name
is a `StringType` (`Narrative.java:364-366`) and through `*` an `XhtmlType`
(`:343`, `Base.java:226-236`, `ENGINE:4819`, `:4833`). The two `HAPI-0256`
cases are right, and the generation rule now carries the exception.

**`path.navigation.025`, the parts that hold.**
- `getProperty` composes with `new XhtmlComposer(true)`, which is XML mode
  and not pretty (`XC:65-69`). `composeEx` calls `compose(XhtmlNode)`
  (`XC:486-488`, `:105-116`), which skips `breakBlocksWithLines` when `xml`
  is set. So the by-name value has no inserted CR LF.
- In XML mode:
  - An element with no children and no content is concise (`XC:318-320`,
    `:330`).
  - A `"` in text becomes `&quot;` (`XC:222-224`), while `&`, `<` and `>` are
    escaped in both modes.
  - So `say "a"<br></br>b` becomes `say &quot;a&quot;<br/>b`. Correct.
- Attributes are written by iterating `getAttributes()` (`XC:306-311`), which
  is a `java.util.HashMap` (`XhtmlNode.java:89`, `:139-143`). The order claim
  is correct, and it is not over-claimed: the scenario does not state an
  order.
- `XhtmlNode.setValueAsString` parses with `XhtmlParser` (`:592-622`), and the
  parser puts `xmlns` into the attribute map (`XhtmlParser.java:366`, `:400`,
  `:409`, `:563`). So the composed `div` keeps its `xmlns`.
- `is string`, `is FHIR.string`, `ofType(string)`, `=`, `toString()` and
  `contains` all follow from `fhirType()` = `string`. `is xhtml` is `false`.
- `children()` gives `status` and then `div`, in `listChildren` order
  (`Narrative.java:340-343`).
- `XhtmlType.primitiveValue()` is `new XhtmlComposer(false).compose(getXhtml())`
  (`XhtmlType.java:107-108`), which is HTML mode, and it edits
  `place.getDiv()` (`:82`).
- The `BLOCK_NAMES` set is exactly `li`, `ul`, `ol`, `tr`, `td`, `th`, `div`
  and `table` (`XC:49`).
- An inserted `"\r\n"` text node is written unescaped in XML mode
  (`XC:176-247`, not canonical). So once the node has been edited, a later
  by-name read does contain CR LF.

**`path.navigation.025`, the parts that do not hold:** see findings 1 and 2.

**Blocker "the R4 Java class ancestry table": now properly framed.**
- It gives three options, each with its consequence:
  - (a) read the pinned Java sources, with an ADR, the BSD notice, a
    provenance entry and a reviewed exception list;
  - (b) reflection over the pinned jars in CI, committed as a recorded
    artifact;
  - (c) the StructureDefinitions plus a cited exception list.
- The licence paragraph quotes the HL7 BSD header and names `NOTICE` and
  `docs/provenance.md`, as constitution section 10 requires.
- The implementer is blocked from hand-writing the table in the meantime. No
  further request.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (24
  deviations, 17 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 381, 513 in total.
- `path.navigation.*` has 25 headings. No ID is duplicated, removed or
  renumbered.
- The no-live-case list (31) includes `.025`, which says "differential-only".

## Forty-fourth review - commit `e30696c` (2026-11-20)

Scope: the diff `db58962..e30696c`. It covers `path.navigation.025`, the new
`DEV-0028` and `DEBT-0018`, the `spec.md` note and task text. `XC` is
`org.hl7.fhir.utilities/.../xhtml/XhtmlComposer.java` and `XT` is
`org.hl7.fhir.r4/.../model/XhtmlType.java`, both at `73c432b5`.

**Forty-third finding 1 (the loop): resolved.**
- `DEV-0028` and `DEBT-0018` now state the loop as it runs (`XC:92-103`):
  from the last child down to the second, never the first, and never
  descending into the first.
- I re-derived both examples:
  - `<div ...><ul><li>a</li><li>b</li></ul></div>`: the root's children are
    `[ul]`, so the loop body never runs and nothing is inserted.
  - `<div ...>x<ul><li>a</li><li>b</li></ul></div>`: the root's children are
    `[x, ul]`. At `i = 1`, `ul` follows the text `x`, which does not end in CR
    or LF, so one CR LF is inserted before it. The recursion into `ul` sees
    `[li, li]`. At `i = 1`, the second `li` follows an element, so a second CR
    LF is inserted. The first `li` is never visited.
  - That is **two** insertions, as the proposer says. My forty-third finding
    gave the same two.

**Forty-third finding 2 (the trigger): resolved.**
- The edit happens only in `XhtmlComposer(false).compose` (`XC:105-108`),
  reached from `XT.primitiveValue()` (`:107-108`) or `getValue()`
  (`:77-78`), on `place.getDiv()` (`:82`).
- `funcChildren` and `funcDescendants` only collect the `XhtmlType`, and
  `count()` and `ofType()` read only `fhirType()`. The spec and `DEV-0028` now
  say this, with `toString()` as the example that triggers the edit and
  `count()` as the example that does not.

**The graded part of `path.navigation.025`.**
- By name: `XhtmlComposer(true)` never runs the insertion (`XC:105-108`).
  The `x<ul>...` example is therefore returned verbatim, and reading it never
  edits the narrative. Correct.
- `children()` yields `status`, then the `XhtmlType`
  (`Narrative.java:340-343`), so `count()` is `2`.
- `XT.fhirType()` is `"xhtml"` (`:55-57`), so `ofType(xhtml).count()` is `1`.
- `XT.listChildren` is empty (`:60-61`), so `descendants()` adds nothing
  below the item. With no `id` or extensions on the fixture narrative, its
  count is also `2`.
- These are testable as written.

**`DEV-0028` and `DEBT-0018`: coherent, and framed as a deviation pending the
owner.**
- `DEV-0028`'s HAPI paragraph matches the source.
- Its chosen behaviour says, in bold, that the `xhtml` item's string is "a
  **difference from HAPI**". It is marked "Reviewed: pending owner". It names
  `DEBT-0018`, which names `DEV-0028` as its origin.
- `harness/scripts/check-registries.sh` agrees.
- Under constitution section 1, a difference from HAPI is acceptable only
  through this register and only until the owner rules. That is how it is
  recorded.

Non-blocking notes, for the owner's checkpoint:
1. `DEBT-0018`'s "Why deferred" cites the owner's 2026-11-08 decision that no
   flag is added speculatively. That decision covers not building a flag for
   the **specification's** behaviour where Frevo already matches HAPI. Here
   Frevo does not match HAPI, so the decision does not apply. The deferral
   rests only on `DEV-0028` being pending, and the entry should say so.
2. If the owner rules for HAPI, `DEBT-0018` stops being deferred work. It
   becomes work owed by the change that implements `path.navigation.025` or
   `add-narrative`. The trigger list could name that ruling explicitly.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (25
  deviations, 18 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 381, 513 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-fifth review - commit `23a7568` (2026-11-20)

Scope: the diff `0c631b1..23a7568`. It covers `path.navigation.025`,
`DEV-0028`, `DEBT-0018`, the `spec.md` note and the blockers note. `ENGINE`
is the R4 `FHIRPathEngine.java` at `73c432b5`.

**`is` on the by-name `div`: correct.**
- The `is` operator's right operand goes through `executeTypeName`
  (`ENGINE:1531-1539`), which yields the plain text: `string`, or
  `FHIR.string` for a qualified name.
- `opIs` (`:1940-1965`) takes neither the `elementmodel` branch nor the
  `disallowExtensions` branch. The `StringType` built by `Narrative.getProperty`
  is created without `noExtensions()`. So `opIs` compares the text with
  `fhirType()`, which is `string`, and then with `sd.getType()` up the base
  chain of the `string` StructureDefinition.
- That gives `true` for `string`, and `false` for `FHIR.string` and for
  `xhtml`.
- `funcIs` (`:5095-5145`) splits a namespace and compares the name, so
  `.is(string)` and `.is(FHIR.string)` are `true` and `.is(xhtml)` is
  `false`. Correct.
- My forty-third and forty-fourth reviews accepted "`is FHIR.string` is
  `true`". That was wrong, and this commit fixes it.

**The static check on `contains`: correct.**
- `case Contains` calls `checkContextString(focus, "contains", exp, true)`
  (`:3498-3502`).
- `checkContextString` (`:3865-3869`) tests `TypeDetails.hasType` for
  `string`, `code`, `uri`, `url`, `canonical` and `id`. `hasType`
  (`TypeDetails.java:243-300`) walks each **requested** type's base chain:
  `code` → `string`, `uri` → `string`, and so on. It never reaches `xhtml`,
  whose type URL is the only one in the focus.
- So it throws `FHIRPATH_STRING_SING_ONLY` (`Messages.properties:363`). The
  focus renders through `describe()`, which is `Utilities.sorted(types)`
  printed as a list (`TypeDetails.java:450-452`).
- The holder is `exp`, the `contains` function node. In
  `Patient.text.div.contains('>hi<')`, `Patient` takes the column to 8, `.`
  to 9, `text` to 13, `.` to 14, `div` to 17 and `.` to 18. `contains`
  therefore starts at column 18, byte offset 17, with no whitespace.
- The message is exactly:
  `Error evaluating FHIRPath expression: The function contains can only be used on string, uri, url, code, id but found [http://hl7.org/fhir/StructureDefinition/xhtml] (@char 18)`.
- At evaluation, `contains` on the `StringType` succeeds.

**Narrative with no `div`: correct.**
- `getProperty` returns no value for a null `div` (`Narrative.java:364-366`).
  `listChildren` always adds `new XhtmlType(this)` (`:343`). So `text.div` is
  empty and `children().count()` is `2`.
- `XhtmlType.getXhtml()` calls `place.getDiv()` (`XhtmlType.java:82`).
  `getDiv` auto-creates `new XhtmlNode(Element, "div")`
  (`Narrative.java:316-323`), because `Configuration`'s static `status`
  defaults to 0, auto-create (`Configuration.java:85-98`).
- In HTML mode an element with no children or content that is not a void
  element is written open/close. `allChildrenAreText()` is `true` for no
  children (`XhtmlNode.java:353-361`), so the result is `<div></div>`.
- A later by-name read composes in XML mode, which is self-closing and has no
  `xmlns` because none was created, so it gives `<div/>`. Correct.
- Frevo's `<div/>`, with nothing created, is recorded as part of `DEV-0028`
  and `DEBT-0018`, both extended consistently.

**Unparseable narrative.**
- `XhtmlNode.setValueAsString` rethrows any parser failure as a
  `RuntimeException` (`XhtmlNode.java:613-626`).
- HAPI's `ParserState.XhtmlStateHl7Org.doPop` calls it during resource
  parsing (`ParserState.java:1830-1834`). So HAPI rejects such a resource at
  read, and "no counterpart" is right.
- The Frevo-side behaviour is stated and testable. Admission is correctly
  deferred to `add-store-crud`.

**Registers, counts and validators.**
- `DEV-0028` remains a stated difference from HAPI, pending the owner.
- My forty-fourth note on `DEBT-0018`'s "Why deferred" still stands, and it
  is non-blocking.
- `harness/scripts/check-registries.sh` reports "registers agree" (25
  deviations, 18 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 381, 513 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-sixth review - commit `b356a10` (2026-11-20)

Scope: the diff `beb1101..b356a10`. It covers `path.quantity.009`, `.011`,
`.012` and `.013`, `DEV-0017`, the new `DEBT-0019`, task 44 and the blockers
note.

**The unit guard (`ENGINE:2282`, `:2355`): correct as stated.**
The test `else if (!left.hasUnit() || right.hasUnit())` is entered when the
left side has no unit or the right side has one. Inside it, the result is
`null` unless both have units and they are equal. With the fixtures in
`parameters-example-quantities.xml` (values `1`, no unit; `1 'tablets'`, unit
text only):
- Unitless = unitless: `null`. `opEquals` (`:2103-2128`) maps it to empty.
  `doEquivalent` (`:2204-2206`) unboxes it, which is an NPE.
- `1 'tablets'` against unitless: the guard is skipped, so the values compare
  and the result is `true` for both `=` and `~`.
- Unitless against `1 'tablets'`: `null`. That gives empty for `=` and an NPE
  for `~`.
- Neither side has a `system`, so `qtyToCanonicalPair` returns `null`
  (`:2292-2296`) and the UCUM path is skipped.

**Recorded results.**
- `:632` (`NoUnitEqual`): failed, with "found 0".
- `:633` (`NoUnitEquivalent`): failed, with the NPE.
- `:636` (`UnitVsNoUnitEqual`): failed, with "found 1".
- `:637` (`NoUnitVsUnitEqual`): passed.
- `:638` (`UnitVsNoUnitEquivalent`): failed, with "was true".
- `:639` (`NoUnitVsUnitEquivalent`): failed, with the NPE.
- `:631` (`testLessThan33`): failed, with the `isPrimitive()` NPE.
- `:669` (`testLessOrEqual30`) and `:728` (`testGreaterThan30`): passed.

Each scenario's passed and failed assignment matches these.

**`path.quantity.013`.**
- `<` compares the `code` lists (`:2438-2440`): `[]` against `[mg/dL]`.
  With the suite's UCUM service it builds
  `[qtyToCanonicalDecimal(left)] = [null]` (`:2301-2303`), and `opLessThan`'s
  `left.get(0).isPrimitive()` (`:2413`) raises the NPE.
- `>` (`:2485-2488`) compares `unit` lists. `<=` (`:2533-2538`) compares the
  `unit` strings. Both are `mg/dL` on each side, so both reach the values and
  the answer is `true`.

**NPE texts.** Both match the recorded reasons verbatim. They are the JVM's
helpful-NPE messages, with the method descriptor in the form
`org.hl7.fhir.r4.fhirpath.FHIRPathEngine.qtyEquivalent(org.hl7.fhir.r4.model.Quantity, org.hl7.fhir.r4.model.Quantity)`.

**`DEV-0017` and `DEBT-0019`: coherent, apart from finding 2.**
- `DEBT-0019` is a wrong-answer hazard where Frevo matches HAPI, so the
  2026-11-08 no-speculative-flag rule is the right basis for deferring it.
- Its five cases are the five new `failed` entries.
- It names the upstream report.

**Not covered by the spec (finding 1).** I checked how `FhirPathR4` runs
these. `HapiWorkerContext.getUcumService()` throws `HAPI-0274` unconditionally
(`HWC:372-374`). Every quantity `=`/`~`, ordered comparison across differing
units, `*`, `/` and `comparable()` reaches that call. The spec's quantity
scenarios, `DEV-0005`, and `.015`'s "no UCUM" clause all assume a worker that
returns a service or `null`.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (25
  deviations, 19 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 381, 513 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-seventh review - commit `cd6f9a3` (2026-11-20)

Scope: the diff `0503660..cd6f9a3`. It covers the new `path.quantity.019`,
`.015`'s null-service clause, the requirement sentence, `DEV-0029` and its
blocker, the `DEV-0017` count, and `spec.md` and task text.

**Call sites (`ENGINE`).** `grep getUcumService()` gives `:2265`, `:2294`,
`:2307`, `:2338`, `:2443`, `:2490`, `:2540`, `:2588`, `:2793`, `:2798`,
`:3013`, `:3018`, `:4631` and `:4633`. In detail:
- **Equality and equivalence.** In `qtyEqual` (`:2258-2290`) and
  `qtyEquivalent` (`:2331-2363`), the worker is consulted right after the two
  `hasValue` tests, so for every pair of valued quantities. `doEquals`
  (`:2190-2192`) and `doEquivalent` (`:2204-2206`) route every
  `Quantity`/`Quantity` pair there, so `=`, `!=`, `~` and `!~` all raise.
  Correct.
  - That covers `.001`-`.004`, `.009`-`.012`, `.017` and `testQuantity1`-`6`.
  - It also covers `testQuantity9`-`11`: each is `... = <quantity>` after the
    `*` or `/`.
- **Ordered comparisons.**
  - `<` (`:2437-2443`) consults the worker when the `code` lists differ.
  - `>` (`:2485-2490`) and `>=` (`:2582-2588`) consult it when the `unit`
    lists differ, compared with `compareDeep`.
  - `<=` (`:2533-2540`) compares the `unit` strings, with the NPE in
    finding 1.
  - `6 days < 1 week` has codes `d` and `wk`, and `8 days > 1 week` has units
    `days` and `week`. So both raise. Correct.
- **Multiplication and division.** `opTimes` (`:2793`) and `opDivideBy`
  (`:3013`) evaluate `worker.getUcumService() != null` only for two
  `Quantity` operands, after the integer and decimal arms. So
  `.006`-`.008` raise. Correct.
- **`comparable()`.** See finding 1.

**What never reaches the worker.** `testLessOrEqual30` and
`testGreaterThan30` have equal `unit` lists, so they are `true` under both
workers. Correct.

**Exception and unwrapping.**
- `HapiWorkerContext.java:372-374` throws
  `new UnsupportedOperationException(Msg.code(274))`. `Msg.code` renders
  `"HAPI-0274: "` (`Msg.java:31-32`), so the message is exactly `HAPI-0274: `.
- The exception is a `RuntimeException`, not a `FHIRException`.
  `FhirPathR4.java:66-70` catches only `FHIRException`, and
  `FhirPathFilterInterceptor.java:80-84` catches only
  `FhirPathExecutionException`. So it escapes both. Correct.

**Workers.**
- `FhirR4.java:54-57` returns `new FhirPathR4(theFhirContext)`, which builds
  on `HapiWorkerContext` (`FhirPathR4.java:34-36`).
- `SearchParamExtractorR4.initFhirPath` (`:99-101`) builds a plain
  `FHIRPathEngine` on a new `HapiWorkerContext`.
- `TestingUtilities.java:84-89` builds the suite's `SimpleWorkerContext` and
  calls `setUcumService(new UcumEssenceService(...ucum-essence.xml))`.
- All as cited.

**`.015` reframed: correct.** The `getUcumService() == null` branches are at
`:2443-2445`, `:2490-2492`, `:2540-2542` and `:2588-2590`. Neither HAPI
worker reaches them, and Frevo always has UCUM data. "Not graded" is right for
an unreachable branch.

**`DEV-0029` framing.**
- The HAPI paragraph and the specification paragraph are right.
- Under constitution section 1 this is a case where HAPI (its server-facing
  worker) and the specification disagree. The rule is to choose HAPI and raise
  the case at the checkpoint.
- The entry instead chooses the suite's UCUM answers. It says so plainly, and
  it marks itself "pending owner". It has its own decision entry in
  `tasks/blockers.md`, with both options and their consequences: under (b), a
  server-facing configuration that raises, while the oracle harness keeps the
  suite's answers. That is the correct way to hold a non-HAPI choice until
  the owner rules.
- "Configuration flag: none" and "Debt: none" are coherent. The debt register
  records a flag not built where Frevo **matches** HAPI and HAPI is the
  hazard. Here Frevo does not match HAPI, and any work owed under option (b)
  is the blocker's.
- The rationale's "never answers differently from the suite's recorded value"
  is accurate.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (26
  deviations, 19 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 382, 514 in total. No ID
  is duplicated, removed or renumbered.
- The oracle-builder's uncommitted `Cargo.lock` and `crates/frevo-codegen`
  edits in the worktree were left untouched.

## Forty-eighth review - commits `30e32a3`, `f675156` (2026-11-20)

Scope: the diff `aec41a9..f675156`.
- `path.quantity.019`.
- The codegen scenarios `codegen.naming.002`, `codegen.cardinality.002`,
  `.004`, `.005`, `codegen.choice.004` and `codegen.binding.002`.
- The model scenarios `model.json.023` and `.025`.
- The "Considered and not recorded" note in `docs/deviations.md`, task 21,
  `spec.md` and the blockers note.

`P` is `reference/hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/parser`
at `v8.12.0`.

**Forty-seventh finding: resolved.**
- `comparable()` now consults the worker only for two UCUM-system quantities
  whose codes are both present and differ (`ENGINE:4609-4641`).
  `1 'cm'.comparable(2 'cm')` is `true`.
- `<=` with no left unit fails at `lunit.equals` (`:2533-2538`) before the
  worker, under both workers, with the `parameters-example-quantities.xml`
  example.
- `<=` with a left unit only reaches the worker, because
  `lunit.equals(null)` is `false`.

**The handlers.**
- `LenientErrorHandler` sets `myErrorOnInvalidValue = true` (`:52`), and its
  no-argument constructor sets `myLogErrors = true` (`:59-61`).
- `invalidValue` (`:100-108`) logs when the value is blank or the flag is off.
  Otherwise it **passes the call to `STRICT_ERROR_HANDLER.invalidValue`**. So
  with the default handler a non-blank invalid value is fatal and a blank one
  is not.
- `missingRequiredElement` (`:137-141`) and `unexpectedRepeatingElement`
  (`:183-190`) only log.
- `StrictErrorHandler.invalidValue` (`:56-59`) throws
  `Msg.code(1821) + describeLocation(loc) + "Invalid attribute value \"" + sanitizeUrlPart(value) + "\": " + error`.
- `unexpectedRepeatingElement` (`:76-79`) throws `1823`,
  `... Multiple repetitions of non-repeatable element '<name>' found during parse`.
- `describeLocation(null)` is `""`, and otherwise it is `toString() + " "`
  (`ParseErrorHandler.java:24-30`).
- `ParseLocation.toString()` is `[element="<name>"]` (`:55-57`).
- The JPA server's request parsing uses `FhirContext`'s default lenient
  handler. The `disableAllErrors()` handlers in `JpaStorageResourceParser`,
  `TolerantJsonParser` and `BatchResourceLoader` read stored resources, not
  requests.

**`codegen.cardinality.005`, `model.json.023` and `.025` (blank value):
correct.**
- A JSON scalar goes through `enteringNewElement` and then
  `attributeValue("value", asString)` (`P/JsonParser.java:1485-1491`).
- For `""`, `PrimitiveState.attributeValue` (`P/ParserState.java:1454-1457`)
  calls `invalidValue(ParseLocation.fromElementName(childName), "", "Attribute value must not be empty (\"\")")`
  and sets no value.
- Lenient mode logs, so the element has no value. Strict mode gives
  `HAPI-1821: [element="status"] Invalid attribute value "": Attribute value must not be empty ("")`,
  and for a blank `reference` the same with `[element="reference"]`.

**`codegen.cardinality.002` and `.004`: correct.**
- No `getMin()` is read in `ParserState`.
- The JSON parser's `missingRequiredElement` sites are only the extension
  `url`/`value` checks (`:1503`, `:1514`, `:1995`, `:2004`), and the XML
  parser's are likewise (`XmlParser.java:205`, `:217`, `:514`, `:597`).
- An empty array runs its loop zero times (`P/JsonParser.java:1439-1457`). So
  there is no diagnostic in either mode.

**`codegen.choice.004`: correct.**
- `enteringNewElement` (`P/ParserState.java:640-672`) records a `0..1`
  child's name, and for a choice child that is
  `RuntimeChildChoiceDefinition.getField().getName()`, which is `value`.
- A second `value[x]` property fails `myParsedNonRepeatableNames.add`, so it
  calls `unexpectedRepeatingElement(null, "value")` and pushes
  `SwallowChildrenWholeState`.
- Strict mode gives
  `HAPI-1823: Multiple repetitions of non-repeatable element 'value' found during parse`,
  with no location. Lenient mode logs and drops the second property, keeping
  the first in document order.

**`codegen.binding.002`: correct.**
- `Enumeration.parse` calls `AdministrativeGenderEnumFactory.fromCode`
  (`Enumeration.java:163-167`), which throws
  `IllegalArgumentException("Unknown AdministrativeGender code 'M'")`
  (`Enumerations.java:257-269`).
- `ParserState.java:1482-1514` catches it. `getCodeCaseInsensitive("M")`
  looks up `m` and finds no match, so it reports
  `invalidValue(loc, "M", e.getMessage())`.
- The default lenient handler hands that to the strict handler. The message
  is `HAPI-1821: [element="gender"] Invalid attribute value "M": Unknown AdministrativeGender code 'M'`
  in both modes.
- `"Male"` matches `male` ignoring case. `invalidValue` is called **first**,
  with `Case discrepancy detected in code value 'Male' (should be 'male')`,
  so with the default handlers it is fatal before the correction runs.

**`codegen.naming.002`: correct.**
- Rust 2024 has 38 strict keywords (`as` through `while`, with `Self`) and 14
  reserved ones (`abstract`, `become`, `box`, `do`, `final`, `gen`, `macro`,
  `override`, `priv`, `try`, `typeof`, `unsized`, `virtual`, `yield`). That is
  52 names, as listed, with `gen` (2024 edition) included.
- `union`, `raw`, `safe` and `macro_rules` are weak keywords.
- `class` is not a Rust keyword.
- FHIR R4 does hit the list beyond `type` and `use`, with
  `StructureDefinition.abstract`. It is covered by the rule.

**No `DEV` entry, against constitution section 1.**
- The register is for cases where HAPI and the FHIR specification disagree.
- `binding.002` now agrees with the specification, which requires a code in
  the required value set.
- The blank-value, missing-`1..1` and two-choice-values cases are documents the
  specification calls non-conformant. HAPI's parser accepts them leniently,
  and in the choice case it drops data. The specification assigns detection to
  validation, and FHIR REST's create allows a server to accept without full
  validation. So recording these under "Considered and not recorded", with the
  reasoning, is defensible.
- I do not find a scenario that contradicts a FHIR **requirement** placed on a
  parser.

Non-blocking notes:
1. `codegen.binding.002` makes the decode check depend on "the set
   `path.api.005` describes", the `code` elements HAPI holds as `Enumeration`.
   That set is the subject of the open blocker "the R4 Java class ancestry
   table". Task 21's binding part is therefore blocked with it, and the
   blocker entry could say so.
2. The choice case loses data silently under the default handler. It is HAPI
   parity, but the "Considered and not recorded" note could name the data
   loss, as `DEBT-*` entries do for wrong answers, so the owner sees it at the
   checkpoint.

**Registers, counts and validators.**
- `harness/scripts/check-registries.sh` reports "registers agree" (26
  deviations, 19 debt entries).
- `openspec validate add-fhirpath-and-codegen-r4 --strict` reports the change
  is valid.
- The headings number codegen 49, model 83 and path 382, 514 in total. No ID
  is duplicated, added, removed or renumbered.

## Forty-ninth review - commit `bdfe0ed` (2026-11-21)

Scope:
- `path.math.005` to `.008`, `.015` and `.016`;
- `path.quantity.016` to `.024`;
- tasks 37, 43 and 44;
- `DEV-0030`, `DEBT-0020` and the `DEV-0017` update;
- the `spec.md` registry and counts.

`UCUM` and `ENGINE` are as defined in the header.

**Method.**
1. I compiled `reference/Ucum-java/src/main/java` with `javac -g` (JDK 25)
   outside the repository.
2. A driver reproduced `ENGINE`'s wrappers:
   - `opDivideBy` `:3004-3012`;
   - the `opDiv` and `opMod` decimal arms `:3055-3063` and `:3095-3103`,
     with `IntegerType` parsing;
   - `qtyEqual` with `removeTrailingZeros`/`decEqual` `:2161-2184`,
     `:2258-2286`;
   - `opLessThan`'s canonical branch;
   - `pairToQty`, and `convertToString`'s quantity text.
3. The driver ran against
   `org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml`.
4. I repeated the run against the published `ucum-1.0.10.jar` and
   `ucum-1.0.9.jar`, and with `-ea`. All four outputs are identical.

**Confirmed.**

Numbers:
- Every quotient in the `path.math.015` table, the carry cases
  (`1.9 / 0.2` = `1.0`, `99.9 / 1.0` = `10.0`, `1.99 / 2.0` = `0.100`), the
  sign cases, and `1e3 / 3` = `300`, `1.5e-1 / 3` = `0.050`.
- Every `div` and `mod` value in `path.math.016`, including `3000000000.0 div 1`
  → `java.lang.NumberFormatException: For input string: "3000000000"`
  (`IntegerType.java:65-67`, `:86-92`).
- Every zero-dividend and zero-divisor value in `.005` to `.008`. `0.0 mod 0`
  is `-0.0`, derived through `subtract` `:381-384` and `asDecimal` `:305-306`,
  as stated.
- The `mod` error texts, which match `hapi-8.12.0-results.json:857-858`.

Canonical forms and quantity arithmetic:
- Every canonical value and unit string in `path.quantity.020`, including
  `60 '/min'` → `1 s-1` and `60.0 '/min'` →
  `1.0000000000000000000000000000002 s-1`.
- Every `*` and `/` result in `.021`, including `'null'` for a dimensionless
  result (`ENGINE:898-900`).
- Every equality and ordering result in `.022`, `.017` and `.023`.
- The handler mapping in `.023`, with `[hp_X]`/`[hp_C]` keys not matching the
  file's codes (`special/Registry.java:59-60`).

Parsing and the unknown-unit and non-UCUM cases:
- The `.024` messages for an unknown unit, `]` without `[`, an unterminated
  annotation (NO_CHAR is `\0`, not ASCII), and a stray `)`.
- The four `NullPointerException` texts of `.018`. The published 1.0.10 jar
  keeps the parameter names.
- The `.017` and `.018` fallbacks against `ENGINE:2258-2360`, `:2409-2600` and
  `:4609-4641`. `Base.compareDeep` of two empty lists with `allowNull` is
  `true` (`Base.java:263-265`), so `1 year < 2 years` takes the value branch.

`.016`:
- The 24/7/303 counts and the base-unit list.
- `version="2.0.1"`, and the fetched copy is byte-identical (`cmp`).
- `TestingUtilities.java:82-93`, `:481-492`.
- `XmlDefinitionsParser.java:64-159`, `XmlUtils.java:19-27`,
  `UcumModel.java:127-137`.

Every `Decimal.java`, `Converter.java`, `ExpressionParser.java`,
`Lexer.java`, `ExpressionComposer.java`, `UcumEssenceService.java`,
`DecimalType.java`, `IntegerType.java`, `PathEngineException.java` and
`pom.xml:121-129` citation in the diff is right. The exceptions are those in
finding 7.

**The findings, with the probes.**
- Finding 2. `svc.divideBy(new Pair(1, null), new Pair(1, "m"))` and the
  reverse both throw the `String.contains` NPE on 1.0.9, 1.0.10 source and
  1.0.10 jar. `divideBy(m, new Pair(0, null))` throws
  `UcumException: Attempt to divide 1 by zero`, which the arm turns into
  empty.
- Finding 3. `getCanonicalForm(1, "[m/s2/Hz^(1/2)]")` throws
  `Error processing unit '[m/s2/Hz^(1/2)]': The unit '[m' is unknown' at position 0`.
  `B[10.nV]` does reach the special-unit check (`.` is admitted inside
  brackets) and throws `Not handled yet (special unit)`, as stated.
- Finding 4. `m(s`, `.m`, `m.)`, `10000000000` and `m10000000000` give the
  messages quoted in the header. `isComparable("cm", "10000000000")` is
  wrapped by `getCanonicalUnits`, so `comparable()` is `false`.
- Finding 5. `modulo` results:
  - `-4.0 mod 2`, `-4.0 mod -2`, `4.0 mod -2`, `4 mod 2.0`, `6 mod 3.0`,
    `-6 mod 3.0`, `-0.0 mod 2`, `7.5 mod 2.5` and `5 mod 2.5` are `-0.0`;
  - `10 mod 2.5` is `0`.
- Finding 6. I compared `javap -c -p` output, normalised:
  - constant-pool indices removed;
  - `ldc_w` folded to `ldc`;
  - branch targets removed;
  - whitespace collapsed.

  Results:
  - The classes named in the header are identical between the two jars.
  - Enum classes differ only by the compiler's synthetic `$values()`.
  - `XmlDefinitionsParser.parse` differs in the `revision-date` length branch.

**Interplay checked, no finding.**
- `DEV-0029` is consistent with the new text.
  - `path.quantity.019`'s new bullet sends every UCUM-reaching operation of
    `.017`, `.018` and `.020`-`.024` to `HAPI-0274: ` on the server-facing
    worker. That is right, because each fallback comes after
    `getUcumService()` (`:2265`, `:2338`, `:2443`, `:2490`, `:2540`, `:2588`,
    `:2793`, `:3013`, `:4631`).
  - `path.math.015`/`.016` correctly say no worker is consulted.
- Task 43 satisfies `.016`, `.020`, `.023` and `.024`. Task 44 satisfies
  `.001`-`.015`, `.017`-`.019`, `.021` and `.022`. All 24 quantity IDs are
  owned, and task 37 owns `path.math.015` and `.016`.
- The suite grading claims hold (`testDivide1`-`7`, `testDiv1`-`6`,
  `testMod1`-`8` in `tests-fhir-r4.xml`). Only `testDiv4` and `testMod4`
  have a decimal operand with a non-zero result.

**Non-blocking notes.**
1. `path.quantity.016` says an absent file "raises a `UcumException` from the
   constructor". In the suite worker an absent resource fails earlier:
   `getResource(...)` is `null`, and `.getPath()` raises an NPE
   (`TestingUtilities.java:484`). `TestingUtilities.context()` still turns
   that into an `Error`, so the outcome (the run fails) is as stated, and the
   text is not graded.
2. `path.math.008` cites the integer `mod` guard as `3092-3095`. It is
   `:3091-3094`. This line predates the commit.

## Fiftieth review - commit `09f69fb` (2026-11-21)

Scope: the corrections in `09f69fb` to the seven forty-ninth-review findings.
`UCUM` and `ENGINE` are as in the forty-ninth review. The probes ran on the
`javac -g` build of `reference/Ucum-java` and on `ucum-1.0.9.jar`, and the two
outputs are identical.

**Finding 1, resolved.**
- The Arithmetic requirement body now says `+`, `-` and `*` are exact, and
  that `/` and decimal `div`/`mod` follow `Decimal`.
- `path.compare.012` excludes `/`, `div` and `mod` from both the `double` and
  the exact group.

**Finding 2, resolved.** `divideBy` with a code-less pair throws the
`String.contains` NPE in four cases: `(1, null) / (1, m)`, `(1, m) / (1, null)`,
`(0, null) / (0, m)` and `(0, m) / (0, null)`. `(1, null) / (0, m)` throws
`UcumException: Attempt to divide 1 by zero`, which the arm turns into empty.
This matches the new text, including "a zero dividend over a zero divisor still
reaches the code and fails", because `divide` returns `zero()` first
(`Decimal.java:631-632`).

**Finding 3, resolved.** `[m/s2/Hz^(1/2)]` is moved to a parse failure, with
the exact text. `special/Registry.java:59-60` is right.

**Finding 4, resolved.**
- `m(s`, `.m` and `m.)` give the messages and positions stated.
- `getCanonicalForm(1, "10000000000")`, `multiply` and `divideBy` with
  `m10000000000` throw `java.lang.NumberFormatException: For input string: "10000000000"`.
- `isComparable("10000000000", "m")` throws
  `UcumException: Error processing 10000000000: ...`, which `funcComparable`
  turns into `false`.
- `qtyEqual` and `qtyEquivalent` try the canonical form before comparing codes
  (`ENGINE:2265-2275`, `:2338-2348`). So equal NFE codes fail too, as stated.

**Finding 5, resolved.** `modulo` gives:
- `-0.0` for `4.0 mod 2`, `6.0 mod 2`, `0.0 mod 2`, `-4.0 mod 2`,
  `4.0 mod -2`, `4 mod 2.0`, `6 mod 3.0`, `-6 mod 3.0`, `4.0 mod 2.0`,
  `5 mod 2.5`, `7.5 mod 2.5`, `10.0 mod 2.5` and `3000000000.0 mod 1`;
- `-0.00` for `4.00 mod 2`, `4 mod 2.00` and `1.25 mod 0.25`;
- `0` for `10 mod 2.5` and `12 mod 1.5`.

The stated rule predicts each of these: the zero's text has as many fractional
`0`s as the wider fractional part of the dividend and of the trimmed product.
I also checked it on extra cases, `2.50 mod 1.25` = `-0.00` and
`9 mod 0.75` = `-0.0` (product `9.0`). The `doSubtract` citation `:439-486` is
right.

**Finding 6, resolved.**
- The spec (`path.math.015` and `path.quantity.020` source notes, and `.016`'s
  date branch), `DEV-0030`, `DEV-0017`, `DEBT-0020` and tasks 37/43 now name
  1.0.9 as live HAPI's version and 1.0.10 as the suite run's, with the
  equivalence evidence.
- The `tasks/blockers.md` entry asks the owner about `PINS.md` and
  `clone.sh`, and its facts match what I found.

**Finding 7, resolved.**
- `convertToString` `:887-906`/`:898-900` and `funcToString` `:4914-4919` are
  cited.
- `:2413` is corrected in `.013`, `.017`, `path.compare.001` and `DEV-0030`.
- `.022` gives the four ordered-operator branches and comparison lines, and
  each one is right.
- `DEV-0030` lists `.020` and `.024`.
- The `path.math.008` `mod` guard is now `:3091-3094`, and the
  absent-resource NPE (`TestingUtilities.java:484`) is stated. Those were the
  two notes from the forty-ninth review.

**Non-blocking notes.**
1. The Arithmetic requirement body says "comparisons use a `double`". That
   holds for the ordered operators. `=` on decimals compares strings
   (`decEqual`, `path.equality.001`). The pointer to `path.compare.012` makes
   the intent clear.
2. In `path.quantity.024` and `DEV-0030`, "only `comparable()` answers
   `false`" for an out-of-range number holds when the two codes differ. With
   equal codes, `funcComparable` answers `true` before any UCUM call
   (`ENGINE:4628-4630`), under both workers.

## Fifty-first review - commit `23fbf60` (2026-11-21)

Scope:
- `path.math.015`, the pointer to `.017`;
- `path.math.016`, its additions;
- new `path.math.017`;
- `DEV-0030`, `DEBT-0020`, task 37, `spec.md` and the `tasks/blockers.md`
  entry.

**Method.** The probe drivers were run on three builds of `Decimal`: the
`javac -g` build of `reference/Ucum-java` and the published 1.0.10 and 1.0.9
jars. The three outputs are identical. The drivers covered:
- `new Decimal(String)`;
- `divide`, `divInt` and `modulo`;
- a 300,000-pair random fuzz of all three operations, over sign, integer and
  fractional digits, and exponent forms;
- for each `modulo` `Error`, a reflection check of the larger point position
  of the dividend and of the product `trunc(q) * other`.

**Confirmed.**
- Round-up before truncation: `0.5 / 0.03` is `17` and `0.5 div 0.03` is
  `17`. `0.5 mod 0.03` and `0.15 mod 0.04` are `-0.01`.
- `java.lang.Error: Unhandled` is raised for:
  - `0.05 mod 0.03`, `0.08 mod 0.03`, `0.05 mod 0.05`, `0.099 mod 0.04`,
    `-0.05 mod 0.03` and `0.05 mod -0.03`;
  - `1e3 mod 7`, `1e3 mod 3`, `1e3 mod 1`, `2e3 mod 7`, `-1e3 mod 7` and
    `3.5e38 mod 1`.
- The counter-examples give: `0.1 mod 0.03` = `0.01`, `0.05 mod 0.3` =
  `0.05`, `1e2 mod 30` = `10`, `1e3 mod 400` = `200`, `3.5e1 mod 1` = `0`.
- Fuzz: every `Error` (1,221 in all) was `mod`, with message `Unhandled`. 213
  were below-`0.1` operands with a negative point position. 1,008 had an
  exponent-form dividend. None fell outside the two triggers, and there was
  no negative point position without an `Error`. `/` and `div` raised none.
- `Decimal.java:405-411`, `:462-468`, `:368-386` are right. So are
  `FHIRPathEngine.java:3101`, `:3007-3008`, `:3051`, `:3058-3059`, `:3091`
  and `:3098-3099`, `FhirPathR4.java:66-70`, and `FHIRLexer.java:160-170`.
  No `catch (Error` or `catch (Throwable` exists in `ENGINE`.
- Refusal messages:
  - `'16777216e0' ... (numeric)` and `'16777215.5e0' ... (numeric)`;
  - `'e3'` and `'-e3' ... (numeric)`;
  - `'1e' ... (exponent)`, and also `'1e-'` and `'1e99999999999'`;
  - `1e+3' is not a valid decimal`, which `1E+3` also gives;
  - digit-check texts after stripping the sign and leading zeros
    (`'1d'`, `'1.'`, `'+1'`, `'a1'` for `00a1`, `'x'` for `-00x`).
- `Utilities.java:45-54`, `:56-63` and `Decimal.java:187-205` are right.
- Accepted values: `16777215e0`, `-16777216e0` and `1.6777216e7`.
- `16777216e0 mod 2` and `1e+3 mod 2` throw the stated `UcumException`s.
- `new Decimal(null)` throws
  `Cannot invoke "String.toLowerCase()" because "value" is null` on all three
  builds.
- `Integer.parseInt(null)` throws `Cannot parse null string` on JDK 25, the
  only JDK on this machine.

**Finding 1, probes.** These are refused as `(numeric)`: `-16777220e0`,
`-33554432e0`, `-1234567890.5e0` and `1234567890.5e0`. These are accepted:
`-16777217e0`, `-16777218e0` and `16777215.4e0`. The spec's rule says "a
positive mantissa of 2^24 or more", so it would accept the refused negative
values.

**Registers and counts.**
- Path has 390 headings, codegen 49 and model 83: 522 in total, with no
  duplicate.
- The "Thirty-five" list includes `path.math.017`.
- `check-registries.sh` reports "registers agree".
- `openspec validate --strict` reports the change is valid.
- `DEV-0030` lists `path.math.017`. Task 37 satisfies it.

**Non-blocking notes.**
1. `path.math.017` gives the `Integer.parseInt(null)` text as
   `Cannot parse null string: null` "on earlier releases". JDK 8 gives
   `null`. The text is ungraded, so this does not matter now. It matters only
   when the differential records the live JVM's text.
2. For a non-ASCII digit, `path.math.017` says `Decimal` "then computes with
   `c - '0'`". The observable result, for example Arabic-Indic one-two over
   `1` (U+0661 U+0662), is
   `java.lang.StringIndexOutOfBoundsException: Index 2 out of bounds for length 2`
   from `divide`. That escapes every arm, where Frevo's refusal leaves `/`
   empty. The scenario already says this input is unreachable from a valid
   resource, and `DEV-0030` records the difference. Naming the actual HAPI
   outcome would make the deviation entry exact.
