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

The nine entries below all come from `add-fhirpath-and-codegen-r4` and are the
same kind of case: the pinned R4 `FHIRPathEngine` does something the FHIRPath
specification does not describe. In every one the chosen behaviour is **HAPI's**,
because the point of the port is that an existing deployment can switch without
changing anything. None is a deviation *from HAPI*; each is a place where
following HAPI means not following the specification, which is what rule 1 of
the constitution says to record here.

All nine were reviewed and accepted by the owner on 2026-11-08: **match HAPI,
no configuration flag.** Five of them leave something owed even so — a flag we
may need, a bug report we owe upstream, or behaviour we have not characterised
— and those carry a `Debt:` field pointing at `docs/tech-debt.md`. The other
four owe nothing beyond parity. Accepting a deviation is not the same as
deciding it is harmless; the registry is what keeps the difference visible.

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
- **HAPI 8.12.0 behaviour:** `funcExtension` (`ENGINE:5462`) collects
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

## Considered and not recorded as deviations

Recorded so the reasoning is not re-derived:

- **Lenient request parsing.** HAPI's `FhirContext` installs
  `LenientErrorHandler` by default, so the server accepts an unknown element, a
  JSON type mismatch, a wrong-cardinality element and a missing required element
  with a `2xx`. That is not a deviation, because HAPI also ships
  `StrictErrorHandler` and chooses between them with
  `FhirContext.setParserErrorHandler`. Frevo keeps both modes and the same
  default, so it matches HAPI rather than differing from it. See
  `model.json.004`, `.013`, `.014`, `.015`, `.016`, `.018`, `.021`, `.022`,
  `.023` and `codegen.cardinality.002`.
- **Double quotes rejected as string delimiters.** `allowDoubleQuotes` is off by
  default and both HAPI wrappers leave it off; FHIRPath uses single quotes, so
  the default agrees with the specification. No divergence.
- **The `documentFunctions` oracle group.** The R4 engine implements none of its
  seven functions, so 115 of the 1,191 cases fail for HAPI too. That is parity —
  not a gap and not a deviation — and belongs in a note on the suite's runner.

Deviations are expected to concentrate in:

- search result ordering where the specification leaves order undefined;
- `_total` accuracy modes;
- validator severity levels on profile-slicing edge cases;
- `$expand` paging and `ValueSet.expansion.total` on pre-expanded value sets;
- HAPI's identifier and placeholder-reference handling in transactions.
