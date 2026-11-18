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
- **Status:** open
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
- **Status:** open
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
    (`path.boundary.016`).
  Also owed: an upstream report to `org.hl7.fhir.core` for all of the above
  and for the carry and date-precision-14 errors, which are errors and not
  hazards.
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
