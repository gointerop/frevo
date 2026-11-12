CHANGES_REQUESTED

Fifth review of `add-fhirpath-and-codegen-r4`. It covers the revision in commit
`f1ccbfb`, which touches only `specs/path/spec.md` (`path.check.005` and
`path.check.009`). I checked it against `path.oracle.016` and
`tasks.md:92-98`. The earlier passes' text is recoverable with
`git show 6863598:openspec/changes/add-fhirpath-and-codegen-r4/review.md`
(fourth pass) and `git show 8f3aca9:...` (third pass).

Reviewed against `reference/org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094` and `oracles/fhirpath/tests-fhir-r4.xml`
at `c95adbee099c875de97d25ed7616aae7d66dd85b`. `ENGINE` means
`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`, and
`SUITE` means `oracles/fhirpath/tests-fhir-r4.xml`.

Both findings from the fourth pass are resolved. Checking the case lists
against the suite turned up one new finding. Its root cause is in
`path.oracle.016` and task 4, and `path.check.005` and `path.check.009` inherit
it.

## Findings

1. **`path.oracle.016` lists four pairs, but its own rule picks out more cases.
   So `path.check.005` and `path.check.009` give some cases a HAPI status that
   the rule contradicts.** `path.oracle.016` (and task 4, `tasks.md:92-98`)
   defines `hapi-cannot-pass` as a status for cases "whose expressions are
   byte-identical and which differ only by their `context` and `contextType`
   attributes". The reason given is that the pinned harness ignores both
   attributes and "necessarily fails one of each pair". The harness confirms
   that reasoning. `FHIRPathTests.java:222` always calls
   `fp.check(res, "Resource", "Observation", "Observation", node)`, and the
   pass/fail assertion depends only on the expected `invalid` attribute
   (`:224`, `:228`). So any identical expressions with opposite `invalid`
   expectations get the same HAPI check outcome, and HAPI fails exactly one of
   them.

   The suite has three such groups that the enumeration leaves out:

   - **`isGuardOrTypesImpliesBad` and `isGuardOrTypesImpliesAll`.** Both have
     the expression `($this is dateTime or $this is Period) implies $this.toString().length() >= 10`
     (`SUITE:1721-1722`, `:1724-1725`). The first has `contextType="Period"`
     and expects `invalid="semantic"`. The second has no `contextType` and
     expects success.
   - **`isGuardNotImplies` and `isGuardNotImpliesBad`.** Both have the
     expression `($this is Period).not() implies $this.toString().length() >= 10`
     (`SUITE:1781-1782`, `:1785-1786`). The first has `Period` and expects
     success. The second has `Period|Timing` and expects `invalid="semantic"`.
   - **`isGuardOr2`.** It has the same expression as `isGuardOr` and
     `isGuardOrBad` (`$this is Period or $this.toString().length() >= 10`,
     `SUITE:1759-1768`), with `Period|dateTime`, and it expects success. That
     makes three cases, not a pair. `isGuardOr2` is identical to `isGuardOrBad`
     except for `contextType`, and their `invalid` expectations are opposite.

   The rule and the list now give different HAPI statuses, and the revised
   scenarios follow the list:

   - `path.check.005` says of `isGuardNotImplies` that "HAPI's own run fails all
     three too ... this is parity". By the rule, its recorded status is
     `hapi-cannot-pass`.
   - `path.check.009` records `isGuardXorBad2` as `hapi-cannot-pass`. It gives
     no HAPI status for `isGuardOr2`, which is in exactly the same position.
   - The `isGuardOrTypesImpliesAll`/`isGuardOrTypesImpliesBad` and
     `isGuardNotImpliesBad` mentions in `005` and `012` give Frevo's grade only.
     They are not wrong, but under the rule their HAPI column is
     `hapi-cannot-pass`. An oracle-builder reading the list would instead
     record `failed`/`passed`.

   This matters for the gate. Under `path.oracle.012` and `path.oracle.011`,
   `isGuardOr2` and `isGuardOrTypesImpliesAll` pass for Frevo (the check passes,
   and evaluation gives `true`). If HAPI's status is `failed`, they are
   `ahead-of-hapi`. If it is `hapi-cannot-pass`, they are not. So the per-case
   report depends on which text the oracle-builder follows. The existing
   `crates/frevo-path/tests/path_oracle.rs` asserts the pair property (`:538-549`)
   only for the four pairs it lists at `:143-148`.

   **Fix:** extend the enumeration in `path.oracle.016` and in task 4
   (`tasks.md:92-98`) to cover every group the rule selects. The full list is:
   `isEffectOnLogic2Base`/`isEffectOnLogic3Base`,
   `isGuardOrTypesImpliesBad`/`isGuardOrTypesImpliesAll`,
   `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad`,
   `isGuardOr`/`isGuardOr2`/`isGuardOrBad`, `isGuardXorBad`/`isGuardXorBad2`, and
   `isGuardNotImplies`/`isGuardNotImpliesBad`. Then make `path.check.005` give
   HAPI's recorded status for `isGuardNotImplies` (and, if it states one, for
   `isGuardOrTypesImpliesAll`/`Bad` and `isGuardNotImpliesBad`) as
   `hapi-cannot-pass`. Make `path.check.009` give it for `isGuardOr2`. The
   alternative is to narrow the rule's wording so that it selects only the
   listed pairs, and to say why the other groups are excluded. That needs a
   reason grounded in HAPI's harness, and I found none: those groups behave
   exactly like the listed pairs. Either way, the rule and the list must agree.

## Verified, no finding

**(1) `path.check.009`'s statuses, given `path.oracle.016` as enumerated.**

- **Frevo's statuses are correct.** Frevo grades `isGuardOr`, `isGuardOrBoth`
  and `isGuardOrInnerImplies` `failed`. It grades `isGuardOrBad`,
  `isGuardOrOtherImpliesBad`, `isGuardOrOtherImpliesNoParensBad`,
  `isGuardOrInnerImpliesBad` and `isGuardXorBad` `passed`. These follow from
  non-narrowing typing with the case's context type.
- **HAPI's statuses match the listed pairs.** HAPI's recorded status is
  `failed` for `isGuardOrBoth` (its expression, `SUITE:1770-1771`, is unique).
  It is `hapi-cannot-pass` for both halves of the `isGuardOr`/`isGuardOrBad`,
  `isGuardOrInnerImplies`/`isGuardOrInnerImpliesBad` and
  `isGuardXorBad`/`isGuardXorBad2` pairs. That is three of the four listed
  pairs. The fourth, `isEffectOnLogic2Base`/`isEffectOnLogic3Base`, is outside
  `009`, and `006` states no HAPI status for it, so there is no conflict.
- **What remains open** is only finding 1, about `isGuardOr2`.

**(2) `path.check.005`'s quoted message.** `Messages.properties:353` reads
`FHIRPATH_PRIMITIVE_ONLY = Error evaluating FHIRPath expression: The function {0} can only be used on primitives`.
Both throw sites (`ENGINE:3876`, `:3879`) pass `name` as `{0}`. The extra
arguments have no placeholder.

`makeException` (`ENGINE:584-594`) formats the template and adds a
`FHIRPATH_LOCATION` suffix only when `location != null`. The only way to set
`location` is `setLocation` (`ENGINE:236-237`), and `FHIRPathTests.java` never
calls it. So the quoted
`Error evaluating FHIRPath expression: The function toString can only be used on primitives`
is exact.

`006`, `007`, `008`, `009`, `010` and `011` each refer to "the
`FHIRPATH_PRIMITIVE_ONLY` diagnostic for `toString`". The new sentence in `005`
binds every one of them to that exact message. None of them quotes divergent
text.

**(3) No other slip in `005` to `012`.** I re-checked every "as for HAPI" and
"graded" claim against my 38-case classification (fourth pass, section (b)) and
against the suite's expressions.

- **`007`.** `isGuardFnImplies` and `isGuardFnImpliesBad` have different
  expressions (`is(dateTime)` against `is(Period)`), so `failed` for HAPI is
  correct.
- **`008`.** `isGuardAnd` and `isGuardAndBad` differ (`dateTime` against
  `Period`). `isGuardAndOtherImplies` and `isGuardOtherAndImplies` are unique.
- **`010`.** The three `iif` expressions are distinct.
- **`011`.** `isGuardImpliesEmpty`'s expression is shared only with
  `isEffectOnLogic1`, which also expects success. `isGuardNested` is unique.
  Both are correctly `failed` for HAPI.
- **`012`.** `isGuardImpliesEmpty`, `isGuardOrBoth` and `isGuardNested`
  `failed` for HAPI is correct. The `passed` list gives Frevo's grade only.
- **`005`.** `isEffectOnLogic3` and `isGuardOrTypesImplies` are `failed` for
  HAPI, which is correct. `isEffectOnLogic3` shares its expression with
  `isEffectOnLogic2`, but both expect success, so the rule does not select them.
  The exception is `isGuardNotImplies` (finding 1).
- **Case lists.** Every case list still matches the classification. The 14, 15
  and 9 split is intact, and no case is lost or duplicated.

**(4) Scenario ids.** The 361 `path` scenario headings are identical to the
ones at `6863598`, with no duplicates. `f1ccbfb` changes bullet text in `005`
and `009` only.
