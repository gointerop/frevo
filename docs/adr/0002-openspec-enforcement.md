# 2. Enforcing the OpenSpec methodology

- **Status:** accepted
- **Date:** 2026-11-08
- **Context phase:** Phase 0 (bootstrap)

## Context

This project is built by agents, with the owner reviewing only at phase
boundaries. OpenSpec supplies the commands and the artifact layout but enforces
nothing: an agent can write code before a spec exists, mark a task done without
a commit, add `#[ignore]` to turn a gate green, or archive a change whose gates
never ran. Any of those silently converts "spec-first with hard gates" into
"whatever the agent felt like", and nobody would notice until the differential
harness started failing in bulk.

Enforcement therefore has to be mechanical, and it has to exist in two places:
inside the agent loop, where it can stop a mistake before it happens, and in CI,
where it survives an agent disabling its own hooks.

### Versions confirmed on the bootstrap machine

Verified rather than assumed, because the hook scripts call these directly:

| Tool | Version | Notes |
|---|---|---|
| OpenSpec CLI | **1.16.1** (`@fission-ai/openspec`) | installed globally via npm |
| Claude Code slash commands | `/opsx:propose`, `/opsx:apply`, `/opsx:archive`, `/opsx:explore`, `/opsx:sync`, `/opsx:update` | created by `openspec init --tools claude,agents` |
| `openspec validate` | supports `--all`, `--changes`, `--specs`, `--strict`, `--json`, `--no-interactive` | the hooks use `validate <id> --strict --no-interactive` |
| `openspec archive` | `archive [change-name]`, with `-y/--yes`, `--skip-specs`, `--no-validate` | guarded by H5 |
| `openspec show` | `show [item-name]` | used by the session-context hook |
| jq | 1.8.2 | see "jq resolution" below |

`openspec init` also writes skills to both `.claude/skills/` and `.agents/skills/`,
which is why the constitution is published as both `CLAUDE.md` and `AGENTS.md`
and CI asserts the two files are identical.

## Decision

### 1. Eight hooks, under `.claude/hooks/`

| # | Event | Script | Enforces |
|---|---|---|---|
| H1 | `SessionStart` | `session-context.sh` | prints branch, active change, unchecked tasks, gate status, next unblocked work |
| H2 | `PreToolUse` `Edit\|Write\|MultiEdit` | `require-active-change.sh` | no `crates/` edit without a `change/<id>` branch, a valid strict-validating proposal, and `review.md` = `APPROVED` |
| H3 | `PreToolUse` `Edit\|Write\|MultiEdit` | `protect-paths.sh` | settled specs, generated code, `reference/`, `oracles/`, `.claude/`, `gates/` and the guard workflow are only editable from the right branch, or not at all |
| H4 | `PreToolUse` `Edit\|Write\|MultiEdit` | `test-integrity.sh` | no `#[ignore]`, no bare `#[should_panic]`, no `todo!()` in tests, no deleting tests, every test tagged `// covers:`, every tag resolvable |
| H5 | `PreToolUse` `Bash` | `guard-bash.sh` | no premature `openspec archive`, no push to `main`, no force push, no `--no-verify`, no filtered test run recorded as a gate, no deleting tests or oracle data |
| H6 | `PostToolUse` `Edit\|Write\|MultiEdit` | `post-edit.sh` | returns strict-validation errors and `clippy -D warnings` immediately |
| H7 | `Stop` | `stop-guard.sh` | no ending with uncommitted work, a stale `tasks.md`, a red gate for HEAD, or a ticked task with no commit |
| H8 | `SubagentStop` | `subagent-guard.sh` | each role actually produced its deliverable |

Exit code 2 blocks the action and returns stderr to the agent, so **every block
message states what to do instead.** A hook that blocks without a remedy only
produces a retry loop.

### 2. Role separation is enforced by tool allowlist, not by hook payload

The prompt allowed either mechanism. We use **both, with the allowlist as the
primary control**, because the subagent identity is not reliably present in the
hook payload across Claude Code versions:

- Each role's `.claude/agents/<role>.md` declares its `tools`. `gatekeeper` has
  no `Edit` tool at all, so it cannot modify product code even if it wanted to.
- `protect-paths.sh` enforces the path boundaries independently of who is
  acting, which covers the case where the orchestrator itself does the editing.
- `subagent-guard.sh` reads `agent_type`, `subagent_type`, `subagent_name`,
  `agent_name` and `tool_input.subagent_type`, and when none is present infers
  the role from what changed in the working tree. Where even that is ambiguous
  it **allows** the stop rather than blocking the wrong agent — it is a
  completeness check, and the real boundaries are the two mechanisms above.

### 3. Path normalisation is treated as load-bearing

The first implementation of these hooks passed its own tests while enforcing
nothing. On Windows, the hook payload carries a native path
(`C:\repo\crates\x.rs`) while bash reports an MSYS path (`/c/repo`), so the
string prefix strip never matched and every path rule **failed open**. Git Bash
also aliases mount points, so `/tmp` is not literally `C:/Users/.../Temp`.

`rel_path` now resolves both sides physically — it walks up to the deepest
existing ancestor, `cd`s into it and asks the shell where it landed — and
compares case-insensitively. The self-tests exercise this on Windows paths
specifically, because it is the single point where the whole mechanism can
silently stop working.

### 4. jq resolution falls back, but never fails open

jq is the documented dependency, but on the bootstrap machine winget installed
it into a shim directory that an already-running shell never picked up. `_lib.sh`
resolves jq from `PATH` and then from the known winget locations, falls back to
`python3`, and if neither is available **exits 2 with an explanation** rather
than allowing an unchecked action.

### 5. Hook self-tests are themselves a gate

`harness/hook-tests/run.sh` runs each hook against crafted stdin inside a
throwaway git repository, with at least one positive and one negative case per
rule — 74 cases. It runs in CI as part of `openspec-guard`. A hook change that
stops blocking fails the build.

### 6. The CI backstop, `openspec-guard`

Required on `main` through branch protection. It re-checks branch naming, that a
`change/` branch carries its change, that a `chore/` branch changes no product
source, that settled specs move only in an archive commit, that the enforcement
machinery is governance-only, strict validation of everything, the hook
self-tests, scenario coverage in both directions (posted to the pull request),
the review verdict, and the gate report. **CI results always win over a
committed gate report**, which is only an agent's claim.

### 6a. Branch protection, as actually configured

Enabled on `main` at the end of Phase 0:

| Setting | Value |
|---|---|
| Required status checks | `Guard`, `Format and lint`, `Tests`, `Build (release profile)`, `Repository consistency`, `Hook self-tests`, `Licences and advisories`, `Container image builds` |
| Branch must be up to date before merging | yes |
| Required linear history | yes |
| Force pushes | blocked |
| Branch deletion | blocked |
| Conversation resolution required | yes |
| `enforce_admins` | **no** |

`enforce_admins` is deliberately off. This project's premise is that an agent
opens a pull request, waits for CI, and merges it when green; requiring a
second human reviewer would stall every change until the owner is available,
which is the opposite of the intent. The protection that matters —
**merging is impossible until all eight checks pass** — applies regardless.
Turning `enforce_admins` on is the right move the moment a second maintainer
exists.

Note that this makes the required checks the real gate, which is why
`openspec-guard` is in the list: it is the only check that enforces the
methodology rather than the code.

### 7. Bootstrap order

Phase 0 predates enforcement, so Phase 0 runs on `chore/bootstrap-*` branches.
The hooks are written and self-tested first but **not wired in**: the ready
configuration lives in `.claude/hooks/settings.hooks.json` and is merged into
`.claude/settings.json` in the final Phase 0 step, after the self-tests pass.
The first change proposed under live enforcement is itself the test that
enforcement works.

## Consequences

Good:

- The methodology holds without the owner policing it.
- Block messages are instructions, so an agent can recover unaided.
- The two cheapest ways to fake progress — weakening a test, and archiving
  without gates — are closed off at the point of action.

Bad, and accepted:

- Eight hooks on the edit path cost latency. H6 is the expensive one; it is
  bounded with `timeout` and scoped to the touched crate.
- The hooks are bash, so they need Git Bash on Windows. Justified by CI being
  Linux and by the self-tests running on both.
- Inferring a subagent's role from the working tree is approximate. Mitigated by
  the tool allowlists, which are exact.
- Rule evasion remains possible by a sufficiently determined agent (for example
  constructing a path that normalises unexpectedly). `openspec-guard` is the
  answer: it inspects the diff that actually arrives, not the intent.

## Revisit when

- Claude Code exposes the subagent identity in hook payloads reliably, at which
  point H8 can enforce rather than infer.
- The OpenSpec CLI changes `validate` or `archive` flags — the hooks call them
  directly and would need updating in the same commit.
