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
