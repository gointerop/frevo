# Technical debt

Work this project has deliberately not done, each with the condition that would
make it due.

An agent with no conversation memory cannot tell the difference between "nobody
thought of this" and "we thought about it and chose to wait". This file is that
difference. An entry here is a decision, not a wish: it says what is owed, why
we are not paying it yet, what it costs while unpaid, and the observable trigger
that turns it back into work.

## How this differs from its neighbours

| File | Holds |
|---|---|
| [`docs/deviations.md`](deviations.md) | HAPI and the specification disagree; which behaviour Frevo implements |
| [`docs/known-gaps.md`](known-gaps.md) | an oracle case that cannot be made to pass, with owner approval |
| [`docs/adr/`](adr/) | an architectural decision and the reasoning behind it |
| **this file** | something owed and deliberately deferred, with its trigger |

The four overlap on purpose. A deviation can leave debt — accepting HAPI's
behaviour is not the same as deciding it is harmless — and an ADR can leave debt
in the option it declined. When it does, the two cross-reference each other and
`harness/scripts/check-registries.sh` enforces that they agree.

A known gap is **not** debt. A gap is a red case made visible; debt is work not
started. Never move a failing case here to get a gate green.

## Rules

- IDs are `DEBT-NNNN`, allocated in order, never reused, never renumbered.
- **Any branch may append to this file.** Debt is incurred by the work that
  incurs it, and a `change/<id>` branch that defers something records it here in
  the same commit — the same standing exception the branch table makes for
  `docs/deviations.md`, `docs/known-gaps.md` and `tasks/blockers.md`.
- Every entry needs a **Trigger**. "Later" is not a trigger. If no observable
  condition would make this due, it is not debt — either do it or drop it.
- Every entry needs a **Cost if unpaid** stated as what goes wrong for whom.
  Debt with no cost is not debt.
- An entry whose `Status` is `blocking` is not debt at all: it is work in the
  way of the current change. It stays here only until it is done.
- Paying an entry sets `Status: paid` with the commit or pull request that paid
  it. Entries are never deleted; the record of having waited is the point.

## Entry format

```markdown
### DEBT-0001 — short title

- **Area:** path | search | store | bench | governance | oracles | ...
- **Origin:** what created it — `DEV-0005`, `ADR-0002 §6a`, a change id, a file
- **Owed:** the work itself, concretely enough to start
- **Why deferred:** the reasoning at the time
- **Cost if unpaid:** what goes wrong, for whom
- **Trigger:** the observable condition that makes it due
- **Size:** trivial | small | medium | large, with what drives it
- **Status:** open | blocking | paid | dropped
- **Reviewed:** date, by whom
```

## Entries

Entries `DEBT-0001` through `DEBT-0005` all come from the owner's review of the
nine FHIRPath deviations on 2026-11-08. The decision there was **match HAPI, no
configuration flag** — so these five are the flags we did not build, the bug
reports we owe upstream, and the behaviour we have not characterised. Recording
them is the whole reason that decision is safe to make.

### DEBT-0001 — ordered quantity comparison has no dimension check

- **Area:** path
- **Origin:** `DEV-0005`, change `add-fhirpath-and-codegen-r4`
- **Owed:** a UCUM commensurability check on `<`, `<=`, `>` and `>=` over two
  quantities, behind a configuration flag that defaults to HAPI's behaviour.
  `qtyToCanonicalDecimal` returns the canonical *value* and discards the
  canonical *unit*; the check is one comparison of the two canonical unit codes
  before the numeric comparison runs.
- **Why deferred:** no consumer has asked for specification semantics, and a
  flag is a second code path, a second set of differential expectations, and a
  setting that no HAPI configuration file sets.
- **Cost if unpaid:** `1 'cm' < 1 's'` evaluates to `true` — the engine
  canonicalises to `0.01` and `1` and compares those. A profile invariant that
  compares a dose to a duration gets a confident, meaningless boolean instead of
  an error. The failure is silent, it looks like a successful validation, and it
  is wrong in the clinical direction.
- **Trigger:** any of — a deployment reports a validation result traceable to
  this; a published profile is found to depend on the comparison erroring; HL7
  fixes it upstream and we re-pin; a Frevo consumer asks for specification
  semantics.
- **Size:** small. One comparison in the quantity comparison path, plus
  `path.quantity.015` asserted both ways across the flag.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0002 — `extension(url)` does not separate modifier extensions

- **Area:** path
- **Origin:** `DEV-0007`, change `add-fhirpath-and-codegen-r4`
- **Owed:** two things. First, a modifier-extension case in `harness/corpus/` —
  the upstream `testExtension` group has none, so nothing we vendor can detect
  this divergence in either direction. Second, excluding `modifierExtension`
  from `extension(url)` behind a configuration flag that defaults to HAPI's
  behaviour.
- **Why deferred:** the flag, for the reason in `DEBT-0001`. The oracle case is
  deferred only until `frevo-path` can evaluate an expression.
- **Cost if unpaid:** an invariant written as `extension('...')` also receives
  modifier extensions. A modifier extension exists specifically to change the
  meaning of the element it sits on, so the expression is handed data it may be
  built to misread. Worse, without the oracle case we would ship green and
  diverge afterwards without noticing.
- **Trigger:** the oracle case is due as soon as `frevo-path` evaluates
  expressions; the flag on the same conditions as `DEBT-0001`.
- **Size:** small for the flag, trivial for the oracle case.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0003 — report upstream: `<` rejects `unsignedInt` and `positiveInt`

- **Area:** path
- **Origin:** `DEV-0003`, change `add-fhirpath-and-codegen-r4`
- **Owed:** an issue against `hapifhir/org.hl7.fhir.core` citing
  `FHIRPathEngine.opLessThan` gated on `integer || decimal` while `opGreater`,
  `opLessOrEqual` and `opGreaterOrEqual` are gated on
  `integer, decimal, unsignedInt, positiveInt`, with the reachable types
  (`Media.height`, `Timing.repeat.count`) named.
- **Why deferred:** this reads as an oversight rather than a decision, and a
  report that cites a port nobody can run yet is weaker than one that cites a
  released implementation and its scenario IDs.
- **Cost if unpaid:** none to Frevo, which matches the behaviour either way.
  Upstream keeps an inconsistency reachable from real resources, and every other
  implementation that chases HAPI inherits it.
- **Trigger:** before the first public release of Frevo.
- **Size:** trivial.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0004 — report upstream: `toDate()` is missing from the R4 engine

- **Area:** path
- **Origin:** `DEV-0009`, change `add-fhirpath-and-codegen-r4`
- **Owed:** an issue against `hapifhir/org.hl7.fhir.core`:
  `ExpressionNode.Function` declares `ConvertsToDate` but no `ToDate`, and
  `Function.fromCode` has no `"toDate"` case, so the name resolves to no
  function at parse time. `toTime` and `toBoolean` both exist.
- **Why deferred:** as `DEBT-0003`.
- **Cost if unpaid:** none to Frevo. Upstream keeps a conversion function the
  specification defines and the engine silently does not have.
- **Trigger:** before the first public release of Frevo.
- **Size:** trivial.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0005 — double-precision comparison is accepted but uncharacterised

- **Area:** path
- **Origin:** `DEV-0004`, change `add-fhirpath-and-codegen-r4`
- **Owed:** a differential corpus that exercises where HAPI's
  `Double.valueOf(left) OP Double.valueOf(right)` and an exact-decimal
  comparison part company — values beyond 15 significant digits, values near
  `2^53`, values whose decimal and binary roundings differ in the last place —
  so the divergence is measured rather than assumed.
- **Why deferred:** needs `frevo-path` to exist before anything can be compared.
- **Cost if unpaid:** Frevo must reproduce HAPI's double rounding exactly, and
  without a corpus the first evidence of getting it wrong would be an
  unexplained differential diff on a real resource. That is the slowest and
  least informative way to find it.
- **Trigger:** before the gatekeeper runs the differential gate on `frevo-path`.
- **Size:** small, and mechanical once the engine evaluates expressions.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0006 — the `r4-example-roundtrip` oracle does not exist

- **Area:** oracles
- **Origin:** `tasks/graph.yaml` names it as a gate for
  `add-fhirpath-and-codegen-r4`; `oracles/PINS.md` pins no such suite.
- **Owed:** pin `hl7.fhir.r4.examples#4.0.1` by digest, record its licence in
  `oracles/PINS.md`, decide vendored-or-fetched on redistributability, and write
  the runner.
- **Why deferred:** not deferred by choice. The task graph was written before
  the suites were pinned and this one was missed.
- **Cost if unpaid:** one of the two gates named for the current change cannot
  run, so the change cannot go green. This is in the way, not merely owed.
- **Trigger:** now — before `oracle-builder` starts on
  `add-fhirpath-and-codegen-r4`.
- **Size:** small. One `chore/oracles-*` branch.
- **Status:** paid — pull request 13, commit `1098b31`. The suite is pinned by
  SHA-256 as `r4-example-roundtrip`, licence determined (CC0-1.0, declared in
  the package manifest rather than on HL7's licence pages), fetched rather than
  vendored on size, and wired into `oracles/run.sh` as a stub that exits 2. The
  licence trail is recorded in `oracles/PINS.md`, because the two obvious
  sources do not answer the question and the next agent to re-pin this suite
  will hit the same dead ends.
- **Reviewed:** 2026-11-08, owner

### DEBT-0007 — `enforce_admins` is off on `main`

- **Area:** governance
- **Origin:** `ADR-0002 §6a`
- **Owed:** enable `enforce_admins` on the `main` branch protection rule.
- **Why deferred:** deliberate. The project's premise is that an agent opens a
  pull request, waits for CI and merges it green. With one maintainer,
  `enforce_admins` would require a second human reviewer and stall every change
  until the owner is available, which is the opposite of the intent.
- **Cost if unpaid:** an account with admin rights can push to `main` past all
  eight required checks. The eight checks still gate every ordinary merge, so
  the exposure is exactly "the owner can bypass their own rules by accident".
- **Trigger:** a second maintainer with admin rights on the repository.
- **Size:** trivial.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0008 — the `SubagentStop` hook infers the role from the working tree

- **Area:** governance
- **Origin:** `ADR-0002 §2` and its "Revisit when"
- **Owed:** read the subagent identity from the hook payload and enforce the
  role's deliverable, instead of inferring the role from what changed in the
  working tree and allowing the stop when that is ambiguous.
- **Why deferred:** Claude Code does not carry the subagent identity in the
  payload reliably across versions. `subagent-guard.sh` already reads five
  different keys looking for it.
- **Cost if unpaid:** H8 is a completeness check that can be fooled, and it
  fails open by design — blocking the wrong agent is worse than missing one.
  The exact boundaries are the per-role tool allowlists and `protect-paths.sh`,
  which do not depend on identity, so the residual cost is that a role can stop
  without having produced its deliverable.
- **Trigger:** Claude Code exposes the subagent identity in the hook payload
  reliably.
- **Size:** small, once the payload is dependable.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0009 — the 10,000-patient baseline has never been measured

- **Area:** bench
- **Origin:** `bench/targets.md`, `harness/scripts/synthea.sh` (`large` = 10 000)
- **Owed:** a HAPI 8.12.0 baseline on the `large` dataset, and the Frevo targets
  in `bench/targets.md` re-derived from it.
- **Why deferred:** 10 000 patients is about 31 GB of bundles and several times
  that to load. The bootstrap machine did not have the disk, which is why
  `medium` was resized from 10 000 to 1 000 patients.
- **Cost if unpaid:** every target in `bench/targets.md` is extrapolated from
  1 128 patients. The medium run is itself the warning: seven of fourteen
  workloads got *faster* on twelve times the data once warmup was corrected, so
  scaling behaviour here is not monotonic and extrapolation past the measured
  range is not safe. Any public performance claim beyond 1 128 patients would be
  unevidenced.
- **Trigger:** before any public performance claim; or whenever a machine or CI
  runner with the disk is available.
- **Size:** medium — a day of machine time, no code. The dataset regenerates
  from the pinned Synthea version and fixed seeds.
- **Status:** open
- **Reviewed:** 2026-11-08, owner

### DEBT-0010 — a Frevo host service can shadow a real element, and nothing reports it

- **Area:** path
- **Origin:** `DEV-0010`, change `add-fhirpath-and-codegen-r4`
- **Owed:** two things. First, the rule that every host service Frevo ships
  answers empty for any name it does not own, stated once and tested, because
  the engine returns a non-empty implicit-before answer *instead of* navigating
  the element. Second, a diagnostic that reports when a host constant shadowed a
  name the model knows, which is the only way the mistake becomes visible.
- **Why deferred:** needs `frevo-path` and the extraction host service to exist
  before either can be written. `tasks.md` task 47a is where it lands.
- **Cost if unpaid:** a host service that answers for a bare element name makes
  that element unreachable through the expression that names it — answer for
  `name` and `Patient.name` evaluated as `name` returns the host value. Every
  caller that passes an application context travels this path:
  search-parameter extraction, the validator, later CQL. The result is wrong
  index rows written silently on the ingest path, with no error and no log line,
  which is the most expensive shape of parity bug this project can have.
- **Trigger:** when the host-service trait lands in `frevo-path` — the rule is
  due with it, not after. The diagnostic is due before the first release that
  lets an operator configure a host service.
- **Size:** small for the rule and its test; the diagnostic is small but needs a
  decision about where it goes, since the engine has no warning channel on the
  evaluation pass, only on the check pass.
- **Status:** open
- **Reviewed:** 2026-11-08, owner — accepted with `DEV-0010`. The behaviour is
  HAPI's with no configuration flag, so this entry is what carries the
  obligation forward: it was recorded before the sign-off rather than after,
  because the obligation exists the moment the behaviour is matched.

### DEBT-0011 — a decimal literal above 38 significant digits is refused, not carried

- **Area:** path
- **Origin:** `DEV-0011`, change `add-fhirpath-and-codegen-r4`
- **Owed:** an arbitrary-precision decimal behind `frevo-path`'s literal and
  arithmetic paths, so that a literal HAPI carries as a `BigDecimal` is carried
  rather than refused. `design.md` section 6 names the 128-bit mantissa as a
  fallback, and this is the obligation that fallback incurs.
- **Why deferred:** the fallback was chosen so that tasks 28-30 could land
  without an arbitrary-precision type, which `frevo-model`'s codec also needs
  (`model.json.010` asserts that a 40-digit decimal survives a round trip
  without passing through an `f64`). Building one type for both, once, is
  cheaper and less likely to disagree with itself than building two.
- **Cost if unpaid:** `Utilities.isDecimal` accepts a decimal literal of any
  width and `new DecimalType(String)` holds it as a `BigDecimal`, so HAPI
  evaluates an expression Frevo refuses to parse. An invariant or a search
  parameter carrying such a literal fails at parse time on Frevo and works on
  HAPI — a differential diff on an expression that is valid FHIRPath, which is
  the kind of gap that is found by a customer rather than by the gate, because
  no case in the pinned suite has one.
- **Trigger:** whichever comes first — the arbitrary-precision decimal
  `frevo-model` needs for `model.json.010`, since the same type serves both; or
  the first differential diff, pinned-suite case or consumer expression
  carrying a decimal literal above 38 significant digits.
- **Size:** medium. The type is the work, not the wiring: the literal path, the
  arithmetic operators and the comparison operators all have to take it, and
  `path.literal.004`'s precision-preservation requirement means the presented
  form has to survive alongside the value.
- **Status:** paid — in `a900207` (task 44b, ticked after the two stale
  in-crate tests were re-aligned in `50da39e`). The trigger fired on 2026-11-21: the in-process HAPI
  differential found `1 '[pi]' * 1 '1'` (`path.quantity.020#27`, `#42`, `#43`,
  `#46`) answering HAPI's 65-digit value while Frevo refuses it as decimal
  arithmetic beyond 38 significant digits. The owner ruled that HAPI's
  behaviour wins, so this is now work in the way of
  `add-fhirpath-and-codegen-r4`: its task 44b pays it, against
  `path.literal.017` and `path.quantity.020`, and sets this entry to `paid`
  with the commit. Implemented in `a900207` (2026-11-21): `Decimal`'s
  mantissa is `Unscaled`, of any number of digits
  (`crates/frevo-path/src/seam/unscaled.rs`), and every `frevo-path` test of
  `path.literal.017` and `path.quantity.020` passes.
- **Reviewed:** not yet. Raised 2026-11-12 with `DEV-0011`, which is itself
  awaiting the owner at the next phase boundary. Recorded now rather than at
  sign-off because the obligation exists the moment the fallback ships.

### DEBT-0012 — the check pass cannot run over `frevo-model`'s `R4Model`

- **Area:** path
- **Origin:** change `add-fhirpath-and-codegen-r4`, task 50
  (`crates/frevo-path/src/check/`, `crates/frevo-path/src/seam/model.rs`)
- **Owed:** implement the six `Model` methods the check pass added —
  `elements`, `specializations`, `resource_type_names`, `type_names`,
  `has_definition_url` and `extension_value_types` (the value types of the
  extension a known `extension(url)` names; `None` types `.value` as the whole
  `Extension` choice) — on `R4Model` in `crates/frevo-model/src/codec/nav.rs`,
  from the generated tables (which already carry each element's cardinality and
  types), producing backbone types named by path and both type lists
  (`ElementInfo::types` and `::codes`) as the harness's StructureDefinition
  model does in `crates/frevo-path/tests/oracle/engine.rs`.
- **Why deferred:** the task-50 implementer may not edit `crates/frevo-model`.
  The methods default to `None`, so `frevo-model` compiles unchanged and the
  check pass answers `CheckError::NotImplemented` over it — never a verdict.
- **Cost if unpaid:** nothing can type-check an expression over the generated
  model: every check over `R4Model` is not-implemented, so a consumer that
  validates a search parameter or an invariant before evaluating it gets no
  check at all. The oracle is unaffected; it checks over the pinned package.
- **Trigger:** the first caller that checks over `R4Model` — task 57, if it
  checks the generated search-parameter expressions as well as parsing them,
  or the server's search-parameter registration, whichever comes first.
- **Size:** small. The tables hold the data; the work is the five accessors and
  a differential test that `R4Model` and the package model answer the same.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-12 by the task-50 implementer.

### DEBT-0013 — a lone UTF-16 surrogate is carried as a private-use code point

- **Area:** path
- **Origin:** change `add-fhirpath-and-codegen-r4`, commit `44227f4`
  (`crates/frevo-path/src/eval/`), required by `path.string.002`/`.022`
- **Owed:** a string value that holds UTF-16 units natively. Today a lone
  surrogate is stored inside a Rust `String` as U+10F800 + (unit − 0xD800), and
  `utf16()`/`from_utf16()` map it back, because the harness matches `Value`
  exhaustively and a new variant was not available.
- **Why deferred:** HAPI's string type is UTF-16, so `toChars`, `length`,
  `indexOf` and `substring` must see units and may split a pair (the spec, per
  review f397da5). The encoding satisfies every scenario and leaves ordinary
  strings on the old path.
- **Cost if unpaid:** a real character in U+10F800..U+10FFFF is misread as a
  surrogate; `+` does not re-join a high and a low half into a pair. Both
  return a wrong answer rather than an error.
- **Trigger:** the first pinned-suite case, differential diff or consumer
  expression involving a supplementary-plane private-use character or a
  concatenation of split halves; or the first change that adds a `Value`
  variant.
- **Size:** small to medium.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-17.

### DEBT-0014 — a decimal's negative scale lives only in its lexical text

- **Area:** path
- **Origin:** change `add-fhirpath-and-codegen-r4`, commit `9235a33`
- **Owed:** carry a signed scale through `Decimal` so `12345.round(-2) * 1`
  keeps HAPI's scale (`Decimal` stores `scale` as `u32`; negative scale is kept
  only in the rendered text).
- **Why deferred:** the engine's `Decimal` is the interim fixed-precision type
  of DEBT-0011; the rendering that the scenarios need works from the lexical
  text, and widening the type belongs with the arbitrary-precision decimal.
- **Cost if unpaid:** arithmetic on a rounded-to-tens-or-higher decimal renders
  as a plain number where HAPI's `BigDecimal` result may differ.
- **Trigger:** the first differential diff or suite case doing arithmetic on a
  decimal with negative scale; or the arbitrary-precision decimal of DEBT-0011.
- **Size:** small; best done with DEBT-0011.
- **Status:** open — the DEBT-0011 decimal arrived in `a900207` (task 44b) and
  this was not folded in, because it is not free: a signed scale changes how
  an arithmetic result over a negative-scale value renders (`BigDecimal`'s
  `1.23E+4` against today's plain digits), which needs HAPI observations no
  scenario states yet. One more consequence of the non-negative scale since
  `a900207`: a value is shifted into it by appending zeros, so a shift of
  more than 10,000 places (`MAX_SHIFT`, an exponent such as `1e20000` read
  from data, or `round(20000)`) is `EvalError::Unsupported` where
  `BigDecimal` keeps one digit and a scale. A signed scale removes that
  bound too.
- **Reviewed:** not yet. Raised 2026-11-17.

### DEBT-0015 — the regular-expression functions differ from `java.util.regex` in corners where Frevo answers rather than errors

- **Area:** path
- **Origin:** change `add-fhirpath-and-codegen-r4`, task 41a, ADR 0005, and
  `DEV-0023` (the deviation these corners are part of)
- **Owed:** a closer match to Java in the places the `regex` crate cannot be
  made exact by rewriting the pattern:
  - `$` without `(?m)` does not match before a final line terminator (Java
    does) in all three functions;
  - `(?i)` folds Unicode case (`K` U+212A matches `k`); Java's default is ASCII;
  - `.` in `replaceMatches` is rewritten to exclude Java's five line
    terminators only when the pattern has no inline flag group; with one, Rust's
    `.` (excluding only `\n`) applies;
  - `\u` escapes and other escapes `regex` accepts that Java rejects (`\U`)
    are accepted;
  - a lone surrogate carried as a private-use code point (DEBT-0013) is an
    ordinary character to the engine;
  - there is no compiled-pattern cache beyond a 64-entry per-thread one, and
    `replace_all` iteration can be O(m*n^2) on an adversarial pattern.
  Constructs Java has and `regex` lacks (backreferences, lookaround, atomic
  groups, `\Q..\E`, `\R`, `\G`, `\Z`, `(?d)`), and the ones Frevo rejects as
  silently different (`(?u)`, `(?U)`, `(?x)`, `\v`, possessive quantifiers,
  POSIX `\p{..}`, `--` and `~~` after a range in a class), are errors, not
  answers; they are not wrong-answer hazards, but they are still departures from
  HAPI, recorded in `DEV-0023` (a pattern that works in HAPI fails in Frevo;
  paid only by a Java-exact engine, the same trigger). A pattern HAPI times out
  on gets an answer.
- **Why deferred:** each needs either an unavailable engine feature or a
  Java-exact engine (ADR 0005 option (c)); no suite case or consumer uses
  them. No flag is added (constitution section 1).
- **Cost if unpaid:** a wrong answer, not an error, for the cases above.
- **Trigger:** a differential diff, corpus search parameter or IG invariant
  that uses them; or fuzzing finding the O(m*n^2) case.
- **Size:** small each; large if a Java-exact engine is needed.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-17.

### DEBT-0016 — boundary functions give wrong answers, not errors, for exponent-form decimals, date types and sub-millisecond time precisions

- **Area:** path
- **Origin:** `DEV-0025`, change `add-fhirpath-and-codegen-r4`, task 45
- **Owed:** the specification's boundary, behind a configuration flag that
  defaults to HAPI's behaviour, for the cases where matching HAPI answers
  wrongly rather than failing:
  - a decimal stored with a lower-case exponent: the exponent's digits are
    appended without the `e` (`9.5e2` has `lowBoundary(0)` `92`, not `945`;
    `highBoundary()` `9.550000002`, not `955.00000000`; `-9.5e2` has
    `highBoundary(0)` `-92`), `path.boundary.005`, `.011`;
  - a decimal stored with an upper-case `E`: the `E` is treated as a digit
    position (`1.5E2` has `lowBoundary(1)` `1.5` and `highBoundary(1)` `1.6`,
    not `145.0` and `155.0`);
  - a borrow keeps a leading zero (`10.lowBoundary(0)` is `09`): numerically
    right, but the lexical form, and so string conversion, differs;
  - the boundary of a date is a `dateTime`, so a type test or `type()` on it
    reports a dateTime (`path.boundary.009`);
  - a time boundary at precision 2, 4 or 6 is a malformed time (`10:`,
    `10:30:`, `10:30:00.`) rather than an hour, minute or second value
    (`path.boundary.016`);
  - a dateTime boundary at precision 14 is a malformed dateTime that parses
    for a `+` offset or a `-0x` offset (`2014-01-01T08:00:+14:00`, seconds
    read as `1`), where the specification gives a seconds-precision value
    (`path.boundary.015`);
  - a quantity's boundary at a negative precision is a Quantity with no value
    (`1.587 'cm'.lowBoundary(-1)`) or a truncated one
    (`12.5 'cm'.lowBoundary(-1)` is `1 'cm'`), where the specification gives
    empty (`path.boundary.012`).
  Also owed: an upstream report to `org.hl7.fhir.core` for all of the above
  and for the carry, `highBoundary(14)` and `-2` quantity-precision errors,
  which are errors and not hazards.
- **Why deferred:** constitution section 1 and the owner's decision of
  2026-11-08: no flag is added speculatively. HAPI fails the same 18 suite
  cases, so parity is exact; no consumer, search parameter or IG invariant is
  known to call a boundary function on an exponent-form decimal or with a time
  precision below 9.
- **Cost if unpaid:** an invariant or search expression using `lowBoundary`,
  `highBoundary` or a type test on their result gets a wrong number or type,
  silently, for a resource whose decimal was written with an exponent, or for
  a time at reduced precision; the same answer HAPI gives.
- **Trigger:** a differential diff or consumer report involving a boundary
  function on such a value; an IG invariant or generated search-parameter
  expression calling `lowBoundary` or `highBoundary`; or a re-pin of
  `org.hl7.fhir.core` that changes `Utilities.applyPrecision` or
  `DateTimeUtil.applyTimePrecision`.
- **Size:** small; the change is confined to the text helpers in
  `crates/frevo-path/src/eval/boundary.rs` and the flag plumbing.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-18.

### DEBT-0017 — an ordered comparison changes a `dateTime`'s string to its Zulu form

- **Area:** path
- **Origin:** `DEV-0017`, change `add-fhirpath-and-codegen-r4`, raised from the
  implementer's findings of 2026-11-20 (`tasks/blockers.md`)
- **Owed:** a configuration flag, defaulting to HAPI's behaviour, under which
  `<`, `<=`, `>` and `>=` compare a `dateTime` without changing it, so that a
  later read of the compared element (`path.compare.013`, `testLessThan28`) or
  of the compared literal's value through `$this` (`path.compare.015`), and
  the evaluation's result when it returns the compared value (`where`,
  `select($this)`), and any `dateTime` value the evaluation built itself
  (`now()`, `dateTime` arithmetic, a `date` or `dateTime` boundary at
  precision 14 or 17) and then compared,
  returns the string the resource or the expression carried, and not the Zulu
  rendering of the same instant. Also owed: an upstream report to
  `org.hl7.fhir.core` (`compareDateTimeElements`, `FHIRPathEngine.java:635-648`,
  writes to its operand instead of to a copy).
- **Why deferred:** constitution section 1 and the owner's decision of
  2026-11-08: no flag is added speculatively. HAPI fails `testLessThan28` the
  same way, so parity is exact; no consumer, search parameter or IG invariant is
  known to compare a `dateTime` and then read the same value's string within
  one expression.
- **Cost if unpaid:** an expression that compares a `dateTime` and then renders,
  string-compares or returns the same value gets `2012-04-15T08:00:00Z` where
  the input said `2012-04-15T10:00:00+02:00`: the same instant, a different
  string, silently. A `toString()`, `=` on strings, `startsWith`, or a returned
  value shown to a user is wrong; the same answer HAPI gives.
- **Trigger:** a differential diff or consumer report involving a compared
  `dateTime` read back in the same expression; an IG invariant or generated
  search-parameter expression that compares a `dateTime` and reads it again; or
  a re-pin of `org.hl7.fhir.core` that changes `compareDateTimeElements` or
  `BaseDateTimeType.setTimeZoneZulu`.
- **Size:** small; the rewrite is confined to
  `crates/frevo-path/src/eval/rewrite.rs`, and the flag would skip it.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-20.

### DEBT-0018 — the string of a narrative's `xhtml` item is not HAPI's HTML-mode composition

- **Area:** path
- **Origin:** `DEV-0028`, change `add-fhirpath-and-codegen-r4`, raised by the
  forty-third review of `path.navigation.025`
- **Owed:** for the `xhtml` item `children()` or `descendants()` yields for
  `Narrative.div`, HAPI's string: the node composed by `XhtmlComposer` in HTML
  mode (`XhtmlType.java:107-108`), including `breakBlocksWithLines`' CR LF
  insertion exactly as its loop works (from the last child down to the second,
  never the first child nor below it; `XhtmlComposer.java:85-104`), HTML-mode
  void elements and `&nbsp;` (`:228-229`, `:313-350`), and the edit of the
  stored node that makes a later `text.div` include the inserted CR LFs; and,
  for a narrative with no `div`, the empty `div` that reading the item's string
  creates (`<div></div>`, after which `text.div` is `<div/>`).
- **Why deferred:** constitution section 1 and the owner's decision of
  2026-11-08: no flag or second code path is built speculatively; the HTML-mode
  composer is a second serialiser, and the edit of the input conflicts with the
  stored-bytes design. No suite case reads such a string, and no known
  search-parameter expression or invariant does.
- **Cost if unpaid:** an expression that reads the string of a narrative's
  `xhtml` item (`Patient.text.children().ofType(xhtml).toString()`, a
  `contains()` or a comparison on it) gets the XML-mode text where HAPI gives
  the HTML-mode text with inserted CR LFs, silently; and a later `text.div` in
  the same evaluation lacks the CR LFs HAPI's has.
- **Trigger:** a differential diff or consumer report on such an expression;
  `add-narrative` porting `XhtmlComposer`'s HTML mode; or a re-pin of
  `org.hl7.fhir.core` that changes `XhtmlType` or `breakBlocksWithLines`.
- **Size:** medium; it needs the HTML-mode composer, which `add-narrative`
  needs anyway.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-20.

### DEBT-0019 — equality and equivalence of quantities with unit text and no code answer by which side has the unit

- **Area:** path
- **Origin:** `DEV-0017`, change `add-fhirpath-and-codegen-r4`, raised by the
  oracle-builder's classification of the `path.quantity` tests (2026-11-20)
- **Owed:** a configuration flag, defaulting to HAPI's behaviour, under which
  two quantities with no UCUM system and no `code` compare as the
  specification says: equal and equivalent when both lack a unit
  (`path.quantity.009`), empty under `=` and `false` under `~` when exactly one
  has a unit (`path.quantity.011`, `.012`), regardless of side. Also owed: an
  upstream report to `org.hl7.fhir.core` for the guard
  `else if (!left.hasUnit() || right.hasUnit())` (`FHIRPathEngine.java:2282`,
  `:2355`).
- **Why deferred:** constitution section 1 and the owner's decision of
  2026-11-08: no flag is added speculatively. HAPI fails the same five suite
  cases, so parity is exact; no consumer is known to compare unit-text
  quantities with `=` or `~`.
- **Cost if unpaid:** `1 'tablets' = 1` and `1 'tablets' ~ 1` are `true`, a
  wrong answer, silently, while the same comparison written the other way round
  is empty or fails; and two unitless quantities are never `=`. An invariant or
  search expression comparing such quantities gets HAPI's answer.
- **Trigger:** a differential diff or consumer report involving `=` or `~` on
  quantities without a UCUM system; an IG invariant doing so; or a re-pin of
  `org.hl7.fhir.core` that changes `qtyEqual` or `qtyEquivalent`.
- **Size:** small; confined to the quantity equality rule in
  `crates/frevo-path/src/eval/equality.rs` and the flag plumbing.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-20.

### DEBT-0020 — decimal division, `div`, `mod` and canonical quantity values give HAPI's wrong answers

- **Area:** path
- **Origin:** `DEV-0030`, change `add-fhirpath-and-codegen-r4`, raised by the
  proposer on reading the pinned `org.fhir.ucum.Decimal` (2026-11-21)
- **Owed:** a configuration flag, defaulting to HAPI's behaviour, under which
  `/` over numbers is exact decimal division, `div` and `mod` with a decimal
  operand are exact truncated division and remainder (no `-0.0`), and two
  quantities with the same canonical unit compare their canonical magnitudes
  numerically, and special units convert as UCUM defines them
  (`path.math.015`, `path.math.016`, `path.quantity.022`, `path.quantity.023`). Also
  owed: an upstream report to `FHIR/Ucum-java` (present in both 1.0.9 and
  1.0.10) for the rounding carry in
  `Decimal.divide` (`Decimal.java:751-754`, the decimal point moves one place
  too far, so `1.9 / 0.2` is `1.0`) and the negative zero of `subtract`
  (`:381-384` with `asDecimal`'s all-zeros test, `:305-306`).
- **Why deferred:** constitution section 1 and the owner's decision of
  2026-11-08: no flag is added speculatively. HAPI passes every suite case
  these mechanisms touch, so parity is exact; no consumer is known to divide
  decimals in an invariant or a search expression.
- **Cost if unpaid:** silently wrong numbers for anyone evaluating FHIRPath
  through Frevo, as through HAPI: `1.9 / 0.2` is `1.0` (a factor of ten),
  `1.25 / 0.5` is `3`, `1.0 / 8` is `0.13`, `1.9 div 0.2` is `1`,
  `4.0 mod 2 = 0` is `false`, `0.5 mod 0.03` is `-0.01`, `0.05 mod 0.03`
  fails with `Unhandled`, `16777216e0 / 2` is empty, and `1 'mg/dL' = 10 'mg/L'` and
  `60.0 '/min' = 1 '/s'` are `false`, while `1 'B' = 1 '1'` is `true` (a
  logarithmic unit read as a ratio unit, `path.quantity.023`). An invariant or calculated value using
  them passes or fails on the wrong number.
- **Trigger:** a differential diff or consumer report involving decimal `/`,
  `div`, `mod` or quantity equality across units; an IG invariant dividing
  decimals; or a change of the `org.fhir:ucum` that live HAPI resolves
  (1.0.9, `reference/hapi-fhir/pom.xml:1148`) or that `org.hl7.fhir.core`
  declares (1.0.10, the suite run's), or of `org.hl7.fhir.core`, that touches
  `Decimal.divide`, `Decimal.subtract` or `decEqual`.
- **Size:** small; an exact-arithmetic arm in `crates/frevo-path/src/eval/arith.rs`,
  a numeric comparison in the quantity equality rule, and the flag plumbing.
- **Status:** open
- **Reviewed:** 2026-11-21, owner: accepted as written (with DEV-0030). Raised 2026-11-21.

### DEBT-0021 — the R4 model class table is unchecked by path only for choice-typed elements

- **Area:** codegen
- **Origin:** change `add-fhirpath-and-codegen-r4`, the owner's choice of
  option (c) for the R4 Java class ancestry table (2026-11-21),
  `codegen.ancestry.004`
- **Owed:** a full comparison of the generated class table with HAPI's runtime
  classes, element by element, so that every place HAPI's R4 Java model
  differs from the FHIR type hierarchy is an entry in
  `crates/frevo-codegen/data/r4-class-table-exceptions.toml`. The proposer's
  comparison (2026-11-21, against the pinned Java model, outside the
  repository) and the path-based comparisons of the fifty-fourth and
  fifty-fifth reviews covered the `Enumeration` set (324 predicted, 323 held
  as `Enumeration`, one exception), every nested component class name
  (thirteen exceptions), `Resource.id`, the superclass of every datatype, and
  every single-typed element's class by path; the path-based passes found
  nothing beyond the entries already in `codegen.ancestry.004`. What remains
  unchecked by path is the **choice-typed** elements (`[x]`, held by HAPI in a
  field of the abstract `Type`): the class of each chosen type's value is
  stated by rule (`codegen.ancestry.002`) and not yet compared.
- **Why deferred:** the owner ruled out reading the Java sources in the
  generator; the remaining check is what the live differential does for free
  once `path.api.005`-style typed evaluation is exercised over every element.
- **Cost if unpaid:** a typed evaluation (`IFhirPath.evaluate` with a FHIR
  type name, or the `Enumeration` decode check of `codegen.binding.002`)
  accepts or rejects an element differently from HAPI where an exception is
  missing: a wrong `HAPI-0256: ` answer, or a code accepted or refused on
  decode that HAPI refuses or accepts.
- **Trigger:** a differential diff on a typed evaluation or on a coded
  element's decode; a consumer report; or a re-pin of `hl7.fhir.r4.core` or
  of `org.hl7.fhir.core`'s R4 model.
- **Size:** small per entry; medium for the full element-by-element
  comparison, which needs the differential harness's typed-evaluation probe.
- **Status:** open
- **Reviewed:** not yet. Raised 2026-11-21.
