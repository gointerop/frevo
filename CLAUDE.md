# Frevo constitution

This file governs all work in this repository, by agents and humans alike. It is
published unchanged as `AGENTS.md`; CI fails if the two files differ.

Frevo is a feature-for-feature port of HAPI FHIR JPA Server Starter 8.12.0 to
Rust on PostgreSQL. The project is developed by agents with an owner who reviews
only at defined checkpoints. That makes the written record — specs, task graph,
decisions, gate reports — the only reliable memory. Treat it that way.

---

## 1. The reference is HAPI FHIR 8.12.0

HAPI 8.12.0's observable behaviour defines correct, not the FHIR specification
prose. The pins in [`reference/PINS.md`](reference/PINS.md) fix exactly which
code that means.

Where HAPI and the specification disagree, record the case in
[`docs/deviations.md`](docs/deviations.md), choose a behaviour (default: HAPI's,
behind a configuration flag when that is cheap), and raise it at the next
checkpoint. **`docs/deviations.md` is the only exception mechanism.** An
undocumented difference from HAPI is a bug.

## 2. A change is done only when its gates are green

All of these, with no exceptions and no partial credit:

| Gate | Requirement |
|---|---|
| HL7 suite | the relevant vendored suite passes at or above HAPI 8.12.0's own rate |
| Differential | zero unexplained diffs against live HAPI 8.12.0 |
| Performance | the change's stated budget is met; no unexplained regression against `bench/` baselines |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all --check` |
| Tests | `cargo nextest run --workspace` passes |

A green local run is not evidence. CI results decide; where a gate report and CI
disagree, CI wins.

## 3. Never weaken a test or an oracle to make a gate pass

Do not add `#[ignore]`, add `#[should_panic]` without an `expected`, leave
`todo!()`/`unimplemented!()` in a tested path, delete a `#[test]`, narrow an
oracle filter, or relax an assertion to accommodate the implementation.

A case that genuinely cannot be made to pass goes to
[`docs/known-gaps.md`](docs/known-gaps.md) with its exact case identifiers, the
cause, and a decision — and it needs the owner's approval. Recording a gap makes
a failure visible; it does not make a red gate green.

## 4. Generated code is never hand-edited

`crates/frevo-model`, the cross-version converters and the search-parameter
tables are produced by `frevo-codegen` from specification artifacts. Files
carrying an `@generated` marker are outputs. If an output is wrong, fix the
generator and regenerate. FHIR models are **regenerated from
StructureDefinitions, never translated from upstream Java.**

## 5. Dependencies

- No `unsafe` without a `// SAFETY:` comment justifying it and a test exercising
  it. Crates default to `#![forbid(unsafe_code)]`.
- **PostgreSQL is the only runtime dependency.** No broker, no external search
  engine, no separate scheduler, no cache server.
- The embedded replacements for what the starter delegates:

  | Upstream dependency | Frevo |
  |---|---|
  | Elasticsearch / Lucene fulltext | `tantivy` |
  | message broker | PostgreSQL outbox plus `LISTEN`/`NOTIFY` |
  | Spring Batch (Batch2) | PostgreSQL job engine using `SKIP LOCKED` |
  | Thymeleaf narratives | `minijinja` |
  | Spring Boot / Tomcat | `axum` on `hyper` and `tokio` |
  | JPA / Hibernate | `sqlx` or `tokio-postgres`, no ORM |

Adding any other runtime dependency requires an ADR and owner approval.

## 6. Performance principles

These are design constraints, not optimisations to retrofit:

1. Store and serve canonical resource JSON **as bytes**. A read that returns a
   resource unchanged must not parse or re-serialize it.
2. Parse lazily. Materialise only the elements a request actually needs.
3. Extract only the search parameters that are enabled.
4. Bulk writes go through `COPY`, not row-by-row inserts.
5. Paginate by keyset, never by `OFFSET`.
6. Prepared statements everywhere; no query string building per request.
7. No ORM.

Anything touching a read, search or ingest path states its performance budget in
its spec and reports measured before/after numbers in its pull request.

## 7. Scenarios and coverage

Every behaviour has a scenario ID, and every scenario ID has at least one
executable test.

- Format: `<capability>.<area>.<nnn>` — for example `search.chain.014`.
- In a spec delta: `#### Scenario: [search.chain.014] <description>`.
- In a test: a `// covers: search.chain.014` comment (several allowed, one per
  line or comma-separated).

A spec scenario with no test, or a `covers:` tag naming a scenario that does not
exist, fails `openspec-guard`.

## 8. Persist everything in the repository

**Assume your context will be reset at any time. Never rely on conversation
memory.** If a decision, finding or next step is not written to a file, it does
not exist.

| What | Where |
|---|---|
| requirements, in flight | `openspec/changes/<id>/` |
| requirements, settled | `openspec/specs/<capability>/` (only via `openspec archive`) |
| work queue and ordering | `tasks/graph.yaml` |
| blocked work needing the owner | `tasks/blockers.md` |
| architectural decisions | `docs/adr/NNNN-*.md` |
| parity checklist | `docs/inventory.md` |
| gate results | `gates/<id>/report.json` |

On starting a session, read the `SessionStart` hook output: it prints the branch,
the active change, unchecked tasks, the last gate status and the next unblocked
work. That is how you recover your place.

## 9. Architectural decisions are ADRs

Record them in `docs/adr/NNNN-title.md`: context, the options considered, the
decision, consequences (good and bad), and what would make you revisit it. Write
the ADR when the decision is made, not afterwards.

## 10. This repository is public

Never commit:

- secrets, tokens or credentials;
- real patient data or PHI — Synthea output and specification examples only;
- licensed terminology content (SNOMED CT, LOINC, ICD-10-CM, UMLS
  distributions). Ship loaders and tests that fetch or accept operator-supplied
  files, and commit only samples whose licence permits redistribution.

Check every vendored oracle's licence before committing it. If it is not
redistributable, fetch it in CI instead.

Naming and trademarks: the product, repository, binary, image and crate names
never contain "FHIR", "HL7" or "HAPI". All crates use the `frevo-` prefix. Refer
to the standards descriptively only ("an HL7® FHIR® server", "compatible with
HAPI FHIR JPA Server Starter configuration"). Never use the FHIR flame design.

Apache-2.0 obligations: keep upstream `NOTICE` texts in [`NOTICE`](NOTICE), never
remove an upstream copyright line, give every ported file a header naming the
upstream path and pinned SHA and stating that it was modified, and record each
port in [`docs/provenance.md`](docs/provenance.md).

## 11. Every commit is made by an agent

- Conventional commits, scoped where useful: `feat(frevo-search): ...`.
- Sign off every commit (`git commit -s`) and include the session trailer.
- **Never push to `main`.** Open a pull request; merge only when CI is green.
- Never use `--no-verify`, never force-push a shared branch.

---

## The standing loop

For each change, in this order:

```
orchestrator picks the next unblocked change from tasks/graph.yaml
  -> proposer        writes proposal, spec delta (scenario IDs), design, tasks
  -> spec-reviewer   checks the proposal against the Java source; writes review.md
  -> oracle-builder  writes the differential and oracle tests FIRST
  -> implementer(s)  implement one task each, in isolated worktrees
  -> gatekeeper      runs every gate; writes gates/<id>/report.json
  -> fix loop        until green
  -> openspec archive
  -> update docs/inventory.md and tasks/graph.yaml
```

Pause for the owner only at phase boundaries, on a new deviation or known gap,
and on a performance regression you cannot resolve.

## Branches and paths

One active change per branch and worktree.

| Branch | Purpose | May modify |
|---|---|---|
| `change/<id>` | product work | `crates/**`, `openspec/changes/<id>/**`, tests |
| `chore/<slug>` | CI, harness, docs, tooling | everything except behavioural `crates/**/src` edits |
| `chore/governance-*` | the enforcement machinery | `.claude/**`, `gates/**`, `.github/workflows/openspec-guard.yml` |
| `chore/oracles-*` | vendoring or re-pinning oracles | `oracles/**` |

Paths that are never hand-edited on any branch: `openspec/specs/**` (changes only
through `openspec archive`), `reference/**` (read-only upstream), and anything
marked `@generated`.

## Roles

Each role has a tool allowlist in `.claude/agents/<role>.md`, and the hooks
enforce the boundaries:

| Role | Writes | Must not |
|---|---|---|
| `proposer` | `openspec/changes/<id>/**` | touch `crates/**`, `review.md`, `gates/**` |
| `spec-reviewer` | `openspec/changes/<id>/review.md` | touch `crates/**`; see the proposer's reasoning |
| `oracle-builder` | tests, `harness/corpus/**` | touch non-test `crates/**/src` |
| `implementer` | `crates/**`, `tasks.md` | write `review.md`, `gates/**`, or spec deltas |
| `gatekeeper` | `gates/<id>/report.json` | edit product code at all |

`spec-reviewer` reviews artifacts, never the proposer's reasoning. Its verdict is
the first line of `review.md`: `APPROVED` or `CHANGES_REQUESTED`, the latter with
a numbered list of findings. **No file under `crates/` may be edited until
`review.md` reads `APPROVED`.**

## When you are blocked

Append to `tasks/blockers.md` with the change id, what you tried, and the
decision needed. Do not work around a gate, invent a behaviour, or guess at a
deviation. Stop and write it down.
