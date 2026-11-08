# 3. The task graph: plain YAML rather than Beads

- **Status:** accepted
- **Date:** 2026-11-08
- **Context phase:** Phase 0 (bootstrap)

## Context

Agent context is assumed to reset at any time, so the work queue has to live
outside the conversation: what the next unblocked piece of work is, what it
depends on, and what is already done. The bootstrap brief asked that Beads be
evaluated for this, with a plain `tasks/graph.yaml` plus a small CLI as the
fallback.

### Beads, evaluated

[`gastownhall/beads`](https://github.com/gastownhall/beads) — MIT, Go, ~28.6k
stars, actively developed (pushed the day of this evaluation). It is a
distributed graph issue tracker built specifically for coding agents, backed by
Dolt, with `bd ready` for claimable work, dependency edges that release
blockers on close, an MCP server, and first-party Claude Code integration via
`bd setup claude`.

It is, on the merits, a better tool than what this ADR chooses. It solves
precisely the stated problem — persistent, dependency-aware, long-horizon
memory for an agent — and `bd ready` is exactly the query the `SessionStart`
hook needs.

Two concrete conflicts with this project's constitution decided it against:

1. **It writes `AGENTS.md` and installs Claude Code hooks.** `bd init` creates
   or updates `AGENTS.md` by default and installs project Claude Code, Codex
   and Cursor integrations. In Frevo, `AGENTS.md` *is* the constitution —
   published byte-identical from `CLAUDE.md`, with a CI job that fails if they
   differ — and `.claude/**` is the enforcement machinery, modifiable only on a
   `chore/governance-*` branch and covered by 74 self-tests. A tool whose
   normal setup path rewrites both is working against the two mechanisms this
   project relies on most. `--skip-agents` and `--stealth` exist, but that
   means adopting a tool while disabling the integration that makes it good.

2. **Task state would not be plain text in the repository.** Constitution rule
   8 requires state to be persisted in the repo, and the point is that it is
   *reviewable*: a change to the work queue should show up in a pull-request
   diff that a human can read. Dolt-backed state is version-controlled but not
   diff-reviewable as text, so the ordering decisions that drive two years of
   porting would stop being visible in review.

Secondary considerations: a Go binary and a Dolt database become a prerequisite
for every contributor and every CI job, and the project already has OpenSpec
owning the specification workflow — adding a second overlapping system invites
them to disagree about what is in flight.

## Decision

Use **`tasks/graph.yaml` plus `tasks/next.sh`**.

- `tasks/graph.yaml` is the single source of truth for the work queue: one node
  per OpenSpec change, with `depends_on`, `phase`, `crates`, `oracles` and
  `status`.
- `tasks/next.sh` answers the only questions that matter in practice: what is
  ready (`--limit N`), what is blocked and by what, and whether the graph is
  internally consistent (`--validate`: no unknown dependency, no cycle, no node
  whose dependency is unsatisfied while it claims to be done).
- The `SessionStart` hook calls `tasks/next.sh --limit 3`, so a reset context
  recovers its place from the repository.
- `tasks/blockers.md` holds anything needing the owner, appended to by the stop
  hook.

Node status moves `pending -> in_progress -> done`, and a node is `done` only
when its change is archived, which the archive guard already ties to green
gates.

## Consequences

Good:

- No new runtime or tooling dependency; works anywhere bash and Python do.
- The work queue is plain text, so re-ordering the port is a reviewable diff.
- No conflict with `AGENTS.md` or with the hook set.
- One system owns "what is in flight" (OpenSpec changes), and the graph only
  orders them.

Bad, and accepted:

- We hand-maintain what Beads would do better: no claim/lease mechanism, so two
  concurrent implementers could pick the same node. Mitigated for now by one
  active change per branch and worktree, which the hooks enforce.
- No cross-machine synchronisation of task state beyond git itself.
- `next.sh` is ours to maintain, and a bug in it misleads every session that
  starts. Hence `--validate`, which CI runs.

## Revisit when

- More than one implementer runs concurrently often enough that the missing
  claim mechanism actually causes duplicated work, **or**
- Beads gains a mode that leaves `AGENTS.md` and `.claude/**` untouched while
  keeping its agent integration, and can export the graph as reviewable text.

At that point the migration is mechanical: `graph.yaml` is a flat node list
with dependency edges, which is exactly Beads' own model.
