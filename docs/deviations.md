# Deviations

Cases where HAPI FHIR 8.12.0 and the FHIR specification disagree, or where Frevo
deliberately differs from HAPI.

This file is the **only** exception mechanism to "HAPI 8.12.0 is the reference".
A behaviour that is not recorded here and differs from HAPI is a bug, not a
decision.

## Rules

- The default resolution is **HAPI's behaviour**, because the point of the port
  is that existing deployments can switch without changing anything.
- **A configuration flag is not added speculatively.** Owner decision of
  2026-11-08: where matching HAPI is a latent hazard, Frevo still matches HAPI
  with no flag, and the flag we did not build is recorded as an entry in
  [`docs/tech-debt.md`](tech-debt.md) with the trigger that would make it due.
  A flag ships when a consumer needs the specification behaviour, not because
  the hazard is foreseeable. The reason is that every flag is a second code
  path, a second set of differential expectations and a setting no HAPI
  configuration file sets; carrying that cost before anyone needs it buys
  nothing and makes the parity surface larger than HAPI's own.
- Every entry needs a reproducing request, both responses, and a decision. An
  entry without a test referencing its ID is incomplete.
- New entries require owner review. They are a phase-boundary checkpoint.

## Entry format

```markdown
### DEV-0001 — short title

- **Area:** search | store | validator | terminology | ...
- **Scenario IDs:** search.chain.014
- **HAPI 8.12.0 behaviour:** what HAPI does, with the request and response.
- **Specification requirement:** the clause, quoted and cited.
- **Chosen behaviour:** HAPI | specification | configurable
- **Configuration flag:** `hapi.fhir.<key>` or none
- **Debt:** `DEBT-NNNN` in `docs/tech-debt.md`, or none if nothing is owed
  beyond matching HAPI.
- **Rationale:** why.
- **Normalizer impact:** whether the differential harness must treat this field
  as expected-to-differ, and under which rule.
- **Reviewed:** date, by whom.
```

## Entries

All fifteen entries below come from `add-fhirpath-and-codegen-r4`;
`DEV-0013` and `DEV-0015` are pending the owner's review (see their entries);
`DEV-0012` and `DEV-0014` were ruled for HAPI (2026-11-17 and 2026-11-14).
`DEV-0001` through `DEV-0010` are the same kind of case: the pinned R4
`FHIRPathEngine` does something the FHIRPath specification does not describe. In
every one of those the chosen behaviour is **HAPI's**, because the point of the
port is that an existing deployment can switch without changing anything. None
of the ten is a deviation *from HAPI*; each is a place where following HAPI
means not following the specification, which is what rule 1 of the constitution
says to record here. `DEV-0011` is the one entry of the other kind — a place
where Frevo deliberately does **not** match HAPI, because what HAPI does there
is end the call with an unchecked Java exception.

The first ten were reviewed and accepted by the owner on 2026-11-08: **match
HAPI, no configuration flag.** `DEV-0001` through `DEV-0009` were signed off
together; `DEV-0010` was raised afterwards by the second revision of this change
and signed off separately, because a new deviation is a phase-boundary
checkpoint and does not inherit an earlier decision. `DEV-0011` was raised on
2026-11-12 by the implementation of tasks 28-30 and signed off by the owner the
same day: **match HAPI where it has a defined result, diagnose the six inputs.**

Six of the first ten leave something owed even where the behaviour is settled — a
flag we may need, a bug report we owe upstream, or behaviour we have not
characterised — and those carry a `Debt:` field pointing at
`docs/tech-debt.md`. The other four owe nothing beyond parity. Accepting a
deviation is not the same as deciding it is harmless; the register is what keeps
the difference visible.

Short path below: `ENGINE` =
`reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`
at `73c432b5f43ddbce979978a760ec4aa81e929094`. Specification clauses are cited
by section rather than quoted: the `HL7/FHIRPath` repository carries no licence
file, so its prose is not redistributable here (see `oracles/PINS.md`).

### DEV-0001 — `single()` on an empty collection throws instead of returning empty

- **Area:** path
- **Scenario IDs:** path.subset.003
- **HAPI 8.12.0 behaviour:** `Patient.name.prefix.single()` over
  `patient-example.xml` raises `FHIRPATH_NO_COLLECTION`. `funcSingle`
  (`ENGINE:5087`) returns the focus only when `focus.size() == 1` and throws
  otherwise, so size 0 takes the throw.
- **Specification requirement:** FHIRPath 2.0.0, Existence — `single()` returns
  the single item, and returns an empty collection when the input is empty.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** validation invariants are written against HAPI's engine. An
  invariant relying on `single()` being empty-safe already fails on HAPI today,
  so matching the specification would change which resources validate.
- **Normalizer impact:** none. It surfaces as an `OperationOutcome` from a
  validation invariant, which the differential harness already compares.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; nothing owed beyond parity.

### DEV-0002 — an ordered comparison with a multi-item operand returns empty instead of erroring

- **Area:** path
- **Scenario IDs:** path.compare.007
- **HAPI 8.12.0 behaviour:** `(1 | 2) < 3` returns an empty collection.
  `opLessThan` (`ENGINE:2409`) handles the empty-operand case and two
  singleton-and-singleton cases, then falls through to
  `return new ArrayList<Base>()`. `opGreater`, `opLessOrEqual` and
  `opGreaterOrEqual` have the same shape.
- **Specification requirement:** FHIRPath 2.0.0, Comparison — the comparison
  operators are defined for singletons; a collection of more than one item is an
  error.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** an invariant comparing a repeating element without `.first()`
  currently evaluates to empty and so passes `all(...)` on HAPI. Making it an
  error would fail resources HAPI accepts.
- **Normalizer impact:** none directly; it affects validation outcomes, which
  the differential harness compares.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; nothing owed beyond parity.

### DEV-0003 — `<` rejects `unsignedInt` and `positiveInt` while `<=`, `>` and `>=` accept them

- **Area:** path
- **Scenario IDs:** path.compare.001, path.compare.011
- **HAPI 8.12.0 behaviour:** `opLessThan`'s numeric branch (`ENGINE:2418`) is
  gated on `l.hasType("integer") || l.hasType("decimal")`, while `opGreater`
  (`ENGINE:2465`), `opLessOrEqual` (`ENGINE:2513`) and `opGreaterOrEqual`
  (`ENGINE:2563`) are gated on
  `hasType("integer", "decimal", "unsignedInt", "positiveInt")`. Comparing two
  `positiveInt` values with `<` raises `FHIRPATH_CANT_COMPARE`; the same
  comparison with `>` returns a boolean.
- **Specification requirement:** FHIRPath 2.0.0, Comparison — comparison is
  defined on the numeric types uniformly; there is no notion of one operator
  accepting fewer types than its mirror.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0003`
- **Rationale:** this reads as an upstream oversight rather than a decision, and
  it is reachable from real data (`Media.height`, `Timing.repeat.count`).
  Matching it is still right for the port: an invariant written with `<` over a
  `positiveInt` errors on HAPI today and Frevo must produce the same
  `OperationOutcome`. Worth reporting upstream.
- **Normalizer impact:** none.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; the residual hazard is tracked as `DEBT-0003`.

### DEV-0004 — numeric comparison and the transcendental math functions go through binary floating point

- **Area:** path
- **Scenario IDs:** path.compare.012
- **HAPI 8.12.0 behaviour:** all four comparison operators compute
  `Double.valueOf(left) OP Double.valueOf(right)` (`ENGINE:2419`, `:2466`,
  `:2514`, `:2564`). `funcSqrt`, `funcAbs`, `funcCeiling`, `funcFloor`,
  `funcExp`, `funcLn`, `funcLog` and `funcPower` parse the focus with
  `Double.parseDouble` (`ENGINE:4102`–`:4260`), and `sort`'s numeric comparator
  uses `Double.compare` (`ENGINE:6186`). `+`, `-`, `*`, `/`, `div`, `mod`,
  `round` and `truncate` use `BigDecimal` and are exact.
- **Specification requirement:** FHIRPath 2.0.0, Types and Literals — `Decimal`
  carries arbitrary precision and operations on it are decimal operations.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0005`
- **Rationale:** an implementation using exact decimals for comparison would
  differ from HAPI on operands beyond double precision — two
  `Observation.valueQuantity.value` readings with 20 significant digits, say.
  The divergence is silent and would show only as a differential diff, so it is
  cheaper to match deliberately than to discover.
- **Normalizer impact:** none. Stored bytes are unaffected; only comparison
  outcomes are.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; the residual hazard is tracked as `DEBT-0005`.

### DEV-0005 — comparing incommensurable quantities compares magnitudes instead of erroring

- **Area:** path
- **Scenario IDs:** path.quantity.015
- **HAPI 8.12.0 behaviour:** `1 'cm' < 1 's'` returns `true`.
  `qtyToCanonicalDecimal` (`ENGINE:2301`) asks UCUM for each operand's canonical
  form and returns the bare decimal with **no** dimension check; `opLessThan`
  then compares `0.01 < 1`. With no UCUM service configured, `ENGINE:2444`
  returns `false` outright for two quantities whose unit codes differ.
- **Specification requirement:** FHIRPath 2.0.0, Quantity — comparison is
  defined for commensurable units; units that cannot be converted give an empty
  result or an error.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0001`
- **Rationale:** an invariant comparing a dose to a duration is a modelling
  error, but such invariants exist in published profiles and currently yield a
  boolean on HAPI. Erroring instead would fail resources HAPI accepts.
- **Normalizer impact:** none.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; the residual hazard is tracked as `DEBT-0001`.

### DEV-0006 — a string function over a non-string focus returns empty instead of erroring

- **Area:** path
- **Scenario IDs:** path.string.020, path.string.024
- **HAPI 8.12.0 behaviour:** `1.startsWith('1')` returns an empty collection.
  Fifteen call sites guard on
  `focus.get(0).hasType(FHIR_TYPES_STRING) || doImplicitStringConversion`, and
  `FHIR_TYPES_STRING` (`ENGINE:2407`) lists only the 12 string-like FHIR types —
  no `integer`, `decimal` or `boolean`. `setDoImplicitStringConversion`
  (`ENGINE:300`) flips this, and both HAPI wrappers leave it off.
- **Specification requirement:** FHIRPath 2.0.0, String Manipulation — the
  string functions require a `String` focus; another type is an error unless it
  implicitly converts.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none for the default. The engine's own
  `doImplicitStringConversion` setting is exposed, matching HAPI's surface, and
  is off by default as HAPI leaves it.
- **Debt:** none
- **Rationale:** returning empty is the silent-failure mode this project
  dislikes most, but it is what the search-parameter and invariant expressions
  in the R4 package were written against. Changing it would alter which
  invariants fire.
- **Normalizer impact:** none.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; nothing owed beyond parity.

### DEV-0007 — `extension(url)` also returns modifier extensions

- **Area:** path
- **Scenario IDs:** path.utility.023
- **HAPI 8.12.0 behaviour:** `funcExtension` (`ENGINE:5464`) collects
  `getChildrenByName(item, "extension", ext)` and then
  `getChildrenByName(item, "modifierExtension", ext)` before filtering by URL,
  so a modifier extension with the requested URL is returned.
- **Specification requirement:** FHIR R4, FHIRPath additions — `extension(url)`
  is shorthand for `extension.where(url = url)`, which does not include
  `modifierExtension`.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0002`
- **Rationale:** every profile invariant written as `extension('...')` over a
  modifier extension currently finds it. A narrower implementation would ship
  green against the oracle — the `testExtension` group has no
  modifier-extension case — and diverge silently afterwards, which is the worst
  shape this bug can take.
- **Normalizer impact:** none.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; the residual hazard is tracked as `DEBT-0002`.

### DEV-0008 — `now()` is not stable within one evaluation, and `today()` reads the process time zone

- **Area:** path
- **Scenario IDs:** path.datetime.003, path.datetime.004
- **HAPI 8.12.0 behaviour:** `funcNow` (`ENGINE:4772`) is
  `result.add(DateTimeType.now())` on every call and nothing in
  `ExecutionContext` caches an evaluation instant, so two `now()` calls in one
  expression may return different values. `funcToday` (`ENGINE:4779`) is
  `new DateType(new Date(), DAY)` in the platform default zone, which is why
  `FHIRPathTests.test` has to call `TimeZone.setDefault`.
- **Specification requirement:** FHIRPath 2.0.0, Utility Functions — `now()`
  returns the current instant and is consistent within one evaluation.
- **Chosen behaviour:** HAPI for the stability question. **Configurable** for
  the time zone: Frevo reads it from engine configuration rather than from the
  process default.
- **Configuration flag:** none exposed as `hapi.fhir.*`; the evaluation time
  zone is an engine setting, which is what the JVM default is for HAPI.
- **Debt:** none
- **Rationale:** caching an instant would be a nicer engine and would differ
  from HAPI in a way no test could see reliably, so it is not asserted. Taking
  the time zone from configuration is the one difference worth having: it
  removes a source of flake from every gate and produces the same results HAPI
  produces when its default zone is set the same way.
- **Normalizer impact:** the differential harness already normalises timestamps;
  no new rule.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; nothing owed beyond parity.

### DEV-0009 — `toDate()` does not exist in the R4 engine

- **Area:** path
- **Scenario IDs:** path.convert.012
- **HAPI 8.12.0 behaviour:** `'2015'.toDate()` fails to parse.
  `ExpressionNode.Function` has `ConvertsToDate` but no `ToDate`, and
  `Function.fromCode` has no `"toDate"` case, so the name resolves to no
  function. `toTime` and `toBoolean` both exist.
- **Specification requirement:** FHIRPath 2.0.0, Conversion — `toDate()` is
  defined alongside `convertsToDate()`.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0004`
- **Rationale:** implementing `toDate()` would make Frevo accept an expression
  HAPI rejects at parse time. A search parameter or invariant using it would
  then work on Frevo and fail on HAPI — a parity break in the direction hardest
  to notice. Worth reporting upstream.
- **Normalizer impact:** none.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no
  configuration flag; the residual hazard is tracked as `DEBT-0004`.

### DEV-0010 — a host constant resolved implicitly before navigation shadows a real element

- **Area:** path
- **Scenario IDs:** path.env.015, path.env.016, path.env.017
- **HAPI 8.12.0 behaviour:** at the entry of every expression, `execute`
  (`ENGINE:3183-3191`) checks `context.appInfo != null && hostServices != null`
  and, if both hold, calls
  `hostServices.resolveConstant(this, context.appInfo, exp.getName(),
  IMPLICIT_BEFORE)` for the leading **name** — not for a `%constant`. If the
  answer is non-empty the engine returns it and **never navigates the element**,
  so a host service that answers for `name` makes `Patient.name` unreachable
  through the expression `name`. A second site (`ENGINE:3210-3214`) asks with
  `IMPLICIT_AFTER` only after navigation produced an empty result, which is a
  harmless fall-through. Both are gated on a non-null application context, so
  with a null `appInfo` neither fires however the host service is configured.
- **Specification requirement:** FHIRPath 2.0.0, Path selection — a leading
  identifier selects children of the context by name; environment variables are
  written `%name` and are a distinct production. Nothing in the grammar lets a
  host pre-empt a property name, and `%`-less shadowing is not described.
- **Chosen behaviour:** HAPI
- **Configuration flag:** none
- **Debt:** `DEBT-0010`
- **Rationale:** every caller this port ships that passes an application context
  — search-parameter extraction and the validator, and later CQL — travels this
  path, so it is live on the write path rather than academic. Not implementing
  it would make Frevo navigate an element where HAPI returns a host value: a
  silent difference in extracted index rows, which is the most expensive shape
  of parity bug to find. Implementing it means Frevo's own host services must
  answer empty for every name they do not own; that obligation is what
  `DEBT-0010` records, together with the diagnostic we have not built that
  would report a host constant shadowing a known element.
- **Normalizer impact:** none — HAPI and Frevo are expected to agree, and the
  differential harness grades the agreement rather than excusing it.
- **Reviewed:** 2026-11-08, owner — accepted. Match HAPI, no configuration
  flag; the residual hazard is tracked as `DEBT-0010`. Raised after the review
  of DEV-0001..0009 and signed off separately, as a new deviation requires.

### DEV-0011 — the front end diagnoses six inputs on which the pinned lexer crashes, accepts or truncates

- **Area:** path
- **Scenario IDs:** path.lexer.013, path.lexer.014, path.errors.003,
  path.errors.004, path.parse.008
- **HAPI 8.12.0 behaviour:** six truncated or malformed inputs leave the pinned
  lexer in a state it does not check for. Five end the call with an **unchecked
  Java exception** rather than a `FHIRLexerException`, so a caller that handles
  the engine's own exception type still gets a crash; the sixth is accepted:
  1. `'\u00'` — `processConstantString` takes `s.substring(i, i + 4)` with no
     bounds check (`FHIRPathEngine.java:1826`, and `FHIRLexer.java:462` for the
     unit decoder), so a `\u` with fewer than four characters after it raises
     `StringIndexOutOfBoundsException`;
  2. `'\u00zz'` — the same `substring` is handed to
     `Integer.parseInt(.., 16)`, which raises `NumberFormatException`;
  3. `'\u+041'` — the same `parseInt` **accepts** a leading sign, so this
     decodes to `A`. The pinned suite records the expression as
     `invalid="syntax"` (`testLiterals/testLiteralUnicodeInvalid`), so HAPI
     fails its own test case here;
  4. `2 -` — `FHIRLexer.java:152` evaluates
     `source.charAt(cursor) == '>'` outside the bounds check when the token is
     `-`, raising `StringIndexOutOfBoundsException` for a trailing `-`;
  5. `{` as the last character — `FHIRLexer.java:205` reads the next character
     unguarded, same exception;
  6. an unterminated `` %`… `` run — `FHIRLexer.java:182` steps over the
     closing backtick unconditionally and the following `substring` is then
     out of bounds, same exception.
  Separately, `Utilities.isDecimal` accepts a decimal literal of any width (it
  returns `RANGE`, which counts as a decimal), and `new DecimalType(String)`
  carries it as a `BigDecimal`.
- **Specification requirement:** FHIRPath 2.0.0, Literals — a unicode escape is
  `\u` followed by four hexadecimal digits; nothing in the grammar admits a
  sign. The specification says nothing about any of the truncated inputs, and
  nothing in it licenses a crash.
- **Chosen behaviour:** neither, in one narrow respect: Frevo returns a parse
  diagnostic with an offset for all six, and requires a unicode escape to be
  exactly four digits in the radix with no sign.
- **Configuration flag:** none
- **Debt:** `DEBT-0011`. None of the six leaves anything owed — each is a
  malformed input refused at parse time. The residual reported with this
  entry does: Frevo's decimal carries a 128-bit mantissa (`design.md`
  section 6's named fallback), so a literal above 38 significant digits is
  refused as `Invalid Constant` where HAPI carries it as a `BigDecimal`.
- **Rationale:** `path.errors.003` forbids a panic anywhere in the suite, and a
  Java unchecked exception is exactly the failure mode it forbids, so items 1,
  2, 4, 5 and 6 are not a choice. Item 3 is: HAPI accepts an escape the
  specification does not describe, and the consequence of matching it would be
  that Frevo also fails `testLiterals/testLiteralUnicodeInvalid`, a case the
  pinned suite marks `invalid="syntax"`. Rejecting it makes Frevo pass a case
  HAPI fails, which the gate permits — "at or above HAPI 8.12.0's own rate" —
  and the expression it rejects cannot occur in a search parameter or an
  invariant that works on HAPI today, because a `+` in a unicode escape is a
  typo in every real case.
- **Normalizer impact:** none. These are parse-time refusals of malformed
  expressions; no stored byte and no response body differs.
- **Reviewed:** owner, 2026-11-12. Raised the same day by the implementation of
  tasks 28-30 of `add-fhirpath-and-codegen-r4`; accepted as a new-deviation
  checkpoint, separately from DEV-0001..0010.

### DEV-0012 — a negative index raises instead of selecting nothing

- **Area:** path
- **Scenario IDs:** path.navigation.010
- **HAPI 8.12.0 behaviour:** `Patient.name[-1]` over `patient-example.xml`
  ends the call with an unchecked `IndexOutOfBoundsException`. `funcItem`
  (`ENGINE:6277-6283`) guards only the upper bound —
  `Utilities.isInteger(s) && Integer.parseInt(s) < focus.size()` — and
  `Utilities.isInteger` accepts a leading `-`, so `-1 < 3` holds and
  `focus.get(-1)` is reached. The pinned suite's `testIndexer/testIndexer3`
  expects empty and carries no `invalid` attribute, so HAPI fails its own
  case.
- **Specification requirement:** FHIRPath 2.0.0, Indexer — an index outside
  the collection yields an empty collection. The scenario
  `path.navigation.010` originally stated empty and now states HAPI's raise.
- **Chosen behaviour:** HAPI — a negative index is an execution error, as a
  returned error and not a panic (`path.errors.003`), consistent with
  `skip(-1)` (`DEV-0014`). Supersedes the earlier choice of empty.
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** owner ruling, 2026-11-17: follow HAPI everywhere for a negative
  index. `testIndexer3` is `failed` on both sides, so it is parity under
  `path.oracle.012`, not `ahead-of-hapi`. The only residual difference from
  HAPI is that the failure is a typed error rather than a Java exception.
- **Normalizer impact:** none on stored bytes. Expressions using a negative
  index fail on both.
- **Reviewed:** owner, 2026-11-17 (ruled for HAPI). Raised 2026-11-12 by task
  31 of `add-fhirpath-and-codegen-r4`.

### DEV-0013 — a non-integer index is an execution error instead of empty

- **Area:** path
- **Scenario IDs:** path.navigation.011
- **HAPI 8.12.0 behaviour:** `funcItem` (`ENGINE:6277-6283`) converts the index
  to text with `convertToString` and selects only when `Utilities.isInteger`
  accepts that text; otherwise it returns the empty result it started with. So
  `Patient.name['x']` is **empty**, not an error; `Patient.name['1']` converts
  to `1` and **selects the second name**; a decimal `[1.0]` is empty; and a
  multi-item index is joined with `,` (`ENGINE:872`), fails `isInteger`, and is
  empty. No suite case grades any of these.
- **Specification requirement:** FHIRPath 2.0.0, Indexer — the index is an
  `Integer`; the approved scenario `path.navigation.011` states that evaluation
  fails for `Patient.name['x']`, and its test adds `'1'`.
- **Chosen behaviour:** specification — the scenario as approved: an index that
  is not a single integer (an integer literal, or a FHIR `integer`,
  `positiveInt` or `unsignedInt`) is an execution error. An empty index still
  selects nothing, as upstream.
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** this is the one place task 31 departs from a *defined* HAPI
  result, which is against the owner's standing rule recorded with DEV-0011
  ("match HAPI where it has a defined result"). It was implemented anyway
  because the approved scenario and its committed test require it, and an
  implementer may change neither; the conflict is raised here and in
  `tasks/blockers.md` for the owner. If the owner rules for HAPI, the scenario,
  the test in `crates/frevo-path/tests/path_navigation.rs` and `func_item` in
  `crates/frevo-path/src/eval/mod.rs` change together and this entry closes.
- **Normalizer impact:** an expression indexing by a string or a decimal
  evaluates on HAPI and fails on Frevo. No R4 search-parameter expression does
  so (checked against the generated table, 2026-11-12); the core invariants
  have not been checked.
- **Reviewed:** pending owner review. Raised 2026-11-12 by the implementation
  of task 31 of `add-fhirpath-and-codegen-r4`.

### DEV-0014 — `skip` with a negative or empty count, and `take` with an empty count, raise

- **Area:** path
- **Scenario IDs:** path.subset.006, path.subset.008
- **HAPI 8.12.0 behaviour:** `funcSkip` (`ENGINE:6101-6109`) and `funcTake`
  (`ENGINE:5024-5032`) read `n1.get(0).primitiveValue()` with no emptiness
  guard, so `skip({})` and `take({})` end with an unchecked
  `IndexOutOfBoundsException`; `funcSkip` starts its loop at the count, so
  `skip(-1)` reaches `focus.get(-1)` and raises too. `take(-1)` does not raise
  (it is empty). HAPI fails five of its own suite cases on this:
  `testSkip5`, `testSkip7`, `testSkip8`, `testTake8`, `testTake9`
  (`oracles/fhirpath/hapi-8.12.0-results.json`).
- **Specification requirement:** FHIRPath 2.0.0 and the pinned suite — a
  non-positive `skip` count returns the input, and an empty count yields empty.
- **Chosen behaviour:** HAPI — the three functions raise an execution error in
  those cases (as an error, not a panic: `path.errors.003`).
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** owner decision, 2026-11-14: HAPI 8.12.0's observable behaviour
  is the reference. The five cases are `failed` on both sides and so are parity
  under `path.oracle.012`, not a gap and not `ahead-of-hapi`. This reverses the
  earlier implementation, which returned the input or empty.
- **Normalizer impact:** none on stored bytes.
- **Reviewed:** owner, 2026-11-14 (ruled for HAPI).

### DEV-0015 — an operator that follows an indexer is silently dropped

- **Area:** path
- **Scenario IDs:** path.filter.002
- **HAPI 8.12.0 behaviour:** when an indexer is on the **first term** of an
  expression, argument or group, the proximal parse (`ENGINE:1199`) attaches
  trailing operators to `focus`, which the indexer has moved to the synthetic
  `item` node (`ENGINE:1188-1197`, `1203`, `1206-1213`); `execute` applies an
  operation only when `exp.isProximal()` (`ENGINE:1508`). So `given[1] = 'x'`
  evaluates as `given[1]`, with no parser error, and
  `where(given[1] = 'James')` fails the check because `where` parameter 1 is
  then a `string`. An indexer on a later term is unaffected:
  `Patient.name[0].given = 'Peter' | 'James'` (`testIndexer1`), `testIndexer2`,
  `testSort9` and `testQuantityUnitTextEqual` pass. Established by reading the pinned source and
  the implementer's failing assertion; not confirmed against a live HAPI.
- **Specification requirement:** FHIRPath 2.0.0 — an operator applies to the
  whole preceding invocation chain, indexer included.
- **Chosen behaviour:** HAPI — the operator is dropped.
- **Configuration flag:** none
- **Debt:** none
- **Rationale:** the owner's 2026-11-14 ruling for HAPI was given for a broader
  wording (any operator after an indexer); this entry is narrowed to the first
  term and needs the owner's re-confirmation. Confirm against live HAPI in the
  differential before closing; if the live engine differs, this entry is wrong.
- **Normalizer impact:** an R4 search-parameter expression whose first term
  carries an indexer followed by an operator would behave differently from the
  specification on HAPI and on Frevo alike; none is known (not checked against the generated table).
- **Reviewed:** pending owner re-confirmation. Ruled for HAPI on 2026-11-14 for
  the broader wording; narrowed after review the same day.

## Considered and not recorded as deviations

Recorded so the reasoning is not re-derived:

- **Lenient request parsing.** HAPI's `FhirContext` installs
  `LenientErrorHandler` by default, so the server accepts an unknown element, a
  JSON type mismatch, a wrong-cardinality element and a missing required element
  with a `2xx`. That is not a deviation, because HAPI also ships
  `StrictErrorHandler` and chooses between them with
  `FhirContext.setParserErrorHandler`. Frevo keeps both modes and the same
  default, so it matches HAPI rather than differing from it. With one
  qualification: `model.json.016` (a duplicate JSON key) and `model.json.018`
  (a `null` inside a non-primitive array) give Frevo's strict mode an error
  HAPI has no callback for, so strict mode is a superset there rather than a
  match. It costs no parity, because the starter never selects strict mode --
  `ParserErrorHandler`, `StrictErrorHandler` and `LenientErrorHandler` appear
  nowhere in `reference/hapi-fhir-jpaserver-starter/src` -- so the differential
  cannot reach it. See
  `model.json.004`, `.013`, `.014`, `.015`, `.016`, `.018`, `.021`, `.022`,
  `.023`, `.024`, `.025`, `model.canonical.007` and `codegen.cardinality.002`.
  The same reasoning covers the three conditions specified last: a contained
  resource with no `id` survives a lenient decode and is still appended to
  `contained` (`model.json.024`), a `#` reference with no matching contained
  resource is a lenient diagnostic and a strict error (`model.json.025`), and
  the encode-side counterpart of that reference is diagnosed rather than
  rejected (`model.canonical.007`). All three are HAPI behaviour in both of the
  handlers HAPI ships, so none is a deviation.
  Implementing tasks 21, 21a, 22 and 26 found **three more strict-mode supersets
  of the same shape** and **one subset**, all four recorded here rather than as
  `DEV-nnnn` entries for the reason above -- the starter never selects strict
  mode, so the differential cannot reach any of them:
  - `model.json.013` where the element is a **primitive** given the wrong JSON
    scalar form (`"active": "true"` for a `boolean`). HAPI has no callback for
    that case: `JsonParser.parseChildren` at `:1489-1492` passes the scalar's
    string form to `attributeValue("value", ...)` whatever the JSON type was,
    and `"true"` then parses as a `boolean`, so nothing is reported.
    `incorrectJsonType` *is* HAPI's for the other three shapes --- a scalar
    where an object belongs (`ParserState.java:617`), a bare object where an
    array belongs (`JsonParser.java:1461`) and a non-object `_field`
    (`JsonParser.java:1298`, `:1409`) --- which is what the design's table row
    for `.013` was describing.
  - `model.primext.006`, unequal `_field` array lengths.
    `JsonParser.parseChildren` at `:1451-1458` iterates the **value** array and
    takes an alternate only `if (nextAlternateArray.size() >= (i + 1))`, so an
    imbalance in either direction is silent and the extra entries of an
    over-long `_field` array are never visited. Reporting it also means strict
    mode rejects `patient-name-extensions.json`, whose `given` has two entries
    and `_given` one; lenient mode accepts it and decodes it exactly as HAPI
    does, which is what `model.primext.002` grades.
  - `model.primext.009`, a `_field` property on a complex element.
    `JsonParser.java:1369` passes the `_name` value through for any element and
    `parseAlternates` (`:1274-1332`) handles `extension`, `modifierExtension`,
    `id` and `fhir_comments` without ever consulting whether the element is
    primitive. `design.md` section 3 states the Frevo behaviour deliberately.
  - `model.json.023`'s **blank** clause, which is a subset rather than a
    superset: Frevo does not report a blank primitive value at all, where
    `StrictErrorHandler.invalidValue` would throw `1821`. It cannot, because
    `model.json.025` requires `{"reference": ""}` to decode cleanly in strict
    mode, and a blank value is reached by the same callback. Lenient mode
    matches HAPI either way --- `LenientErrorHandler.java:100-108` logs a blank
    value and continues --- which is the mode that decides observable
    behaviour.
- **Double quotes rejected as string delimiters.** `allowDoubleQuotes` is off by
  default and both HAPI wrappers leave it off; FHIRPath uses single quotes, so
  the default agrees with the specification. No divergence.
- **An unfinished block comment is not an error.**
  `FHIRLexer.skipWhitespaceAndComments` is declared without `throws`, so
  `FHIRLexer.java:315` constructs the "Unfinished comment" exception and
  **drops** it. The scan then stops one character before the end of the source,
  and that last character becomes the next token. So `2 + 2 /* not finished`
  fails as `Premature ExpressionNode termination at unexpected token "d"` —
  still the syntax error `path.lexer.009` asks for — while `2 + /* c` parses as
  `2 + c` and `2 + 2 /*abc ` parses as `2 + 2`, because a trailing whitespace
  character is then consumed as whitespace. Frevo reproduces all three rather
  than raising on an unterminated comment, so there is nothing to record: an
  expression HAPI accepts is accepted, and an expression HAPI rejects is
  rejected. Writing the error upstream forgot to throw would have made Frevo
  reject two expressions HAPI accepts.
- **`FHIRLexer.isToken` inspects `charAt(1)` in a loop over `i`.**
  `FHIRLexer.java:126-129` tests the second character once per iteration
  instead of the `i`-th, so a name whose second character is `_` is not a token
  name at all — `a_b` is a syntax error on HAPI, `ab_c` is not. Reproduced, so
  no divergence. It is reachable only through an element or variable name
  shaped that way, and no FHIR element name is.
- **The two escape decoders disagree with each other, and with the engine's.**
  `FHIRLexer.processConstant` (a quantity literal's unit) omits the advance past
  the escaped character, so `1 'a\tb'` decodes the unit as `a<TAB>tb` and
  `1 'a\\b'` is rejected as an unknown escape `\b`; `FHIRLexer.processFixedName`
  (a backtick-delimited name) reads `\u` in **base 32**
  (`FHIRLexer.java:511`) and knows no `` \` `` escape (so
  ``Patient.`name` `` looks up `Îame` and fails the check, as
  `testBasics/testEscapedIdentifierUnicode` does on HAPI, `path.navigation.001`); only
  `FHIRPathEngine.processConstantString`, which is what a string literal goes
  through, is self-consistent. All three are reproduced as they are. None is
  reachable from an expression that is not already pathological, and matching
  them costs nothing.
- **The `@` prefix on a temporal expectation in the FHIRPath suite.** The
  pinned suite writes `<output type="date">@1974-12-25</output>`, and the pinned
  harness compares against `PrimitiveType.fpValue()`, which prepends `@` for a
  date, dateTime or instant and `@T` for a time. That is a property of the
  harness's comparison renderer, not of the engine: nothing in the engine's own
  string conversion, in `toString()` or in a value returned to a caller carries
  the prefix, and `fpValue()` has no caller in the engine at all. So there is no
  divergence to record — only a harness requirement, which is
  `path.oracle.008`. Normalising the prefix away on either side would be
  weakening the oracle under constitution rule 3.
- **The `documentFunctions` oracle group.** The R4 engine implements none of its
  seven functions, so 111 of the group's 115 cases fail for HAPI too (its 4
  `invalid="syntax"` cases pass, because the unknown-function error is the
  syntax failure they expect). That is parity —
  not a gap and not a deviation — and belongs in a note on the suite's runner.
- **No type-guard narrowing in the check pass.** The `isSemanticChecks` group's
  note (`tests-fhir-r4.xml:1695-1702`) says an `is` test narrows the type the
  other operand of `implies`, `and`, `or` and `iif` is checked against. The
  pinned engine narrows nothing: `executeType` types every operand of an
  operator chain against the same focus (`ENGINE:1621-1637`) and every `iif`
  argument against the same `$this` (`ENGINE:3261-3271`). So 14 cases that
  expect the check to pass fail it with `FHIRPATH_PRIMITIVE_ONLY` for
  `toString`: `isEffectOnLogic3`, `isGuardFnImplies`, `isGuardImpliesEmpty`,
  `isGuardOrTypesImplies`, `isGuardAndOtherImplies`, `isGuardOtherAndImplies`,
  `isGuardOrInnerImplies`, `isGuardAnd`, `isGuardOr`, `isGuardOrBoth`,
  `isGuardNotImplies`, `isGuardIifTrue`, `isGuardIifElse` and `isGuardNested`.
  HAPI's own run fails all 14 too. Its harness checks against the `Observation`
  root, which is not a primitive either. Not a deviation, because the
  FHIRPath specification defines no static check pass and so no narrowing; the
  requirement exists only in the suite's note. Not a gap, because HAPI fails the
  same cases (`docs/known-gaps.md`, Rules). Frevo matches HAPI by owner ruling
  of 2026-11-12 (`tasks/blockers.md`, 2026-11-12, "task 50", finding 3), and
  `path.check.005` to `path.check.012` specify the non-narrowing behaviour. No
  flag and no debt: matching HAPI rejects an expression rather than giving it a
  wrong answer, and `FhirPathR4` never runs the check (`path.check.013`).

Deviations are expected to concentrate in:

- search result ordering where the specification leaves order undefined;
- `_total` accuracy modes;
- validator severity levels on profile-slicing edge cases;
- `$expand` paging and `ValueSet.expansion.total` on pre-expanded value sets;
- HAPI's identifier and placeholder-reference handling in transactions.
