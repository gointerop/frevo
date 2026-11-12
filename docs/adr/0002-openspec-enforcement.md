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

### 3a. It happened again, in a second dimension: linked worktrees

`REPO_ROOT` was derived from the hook script's own location:

```sh
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
```

The hooks live in one place; a session works in a linked `git worktree`. So for
every worktree session `REPO_ROOT` was the **primary** worktree:
`current_branch()` reported `main`'s branch, `active_change()` came back empty,
and every path rule compared the edited file against a tree nobody was editing.

The symptom was the opposite of failing open. `test-integrity.sh` searched only
`openspec/specs/` for scenario ids, so it blocked **every** test tagged
`// covers: path.*` while those ids sat in the delta of the change being worked
on. `oracle-builder` hit it writing the three oracle harnesses and recorded it
in `tasks/blockers.md` rather than working around the rule.

That was the loud half. Running the new self-tests against the pre-fix hooks
shows the quiet half, which is worse: **`protect-paths.sh` did not fire at all
in a worktree.** A write to `openspec/specs/store/spec.md` — a settled
capability spec, which rule 8 says changes only in an archive commit — was
allowed, exit 0, no message. Nine of the new cases fail against the pre-fix
hooks; the one that matters most is the one where the guard said nothing.

A blocking failure gets reported by whoever it blocks. A guard that silently
stops applying is found only if something looks for it, which is the argument
for the environmental self-tests below.

**Why the self-tests could not see it.** They copy `.claude` into a throwaway
repository and run the hooks from its root, so there the script location *is*
the repository root and the two derivations coincide. The fixture could not
express the layout that breaks. That is the same shape of failure as the Windows
path bug above — both times the hooks passed their own tests while
enforcing against the wrong thing, and both times the fixture was the reason.

The root now comes from `git rev-parse --show-toplevel` in the working
directory, falling back to the script location only when the working directory
is not inside a work tree at all. The self-tests gained cases that add a real
linked worktree with `git worktree add` and run the **primary** worktree's hook
with the working directory inside the linked one — the actual layout
— asserting both directions: a `covers:` id from the worktree's
delta resolves, and an id that exists nowhere is still blocked.

**Known limitation, asserted rather than fixed -- since closed (section 3d).**
A path inside a *different* worktree of the same repository, while the working
directory is in another, matched no rule. This was judged hypothetical, on the
belief that Claude Code hands the hook a path inside the session's own tree.
That belief was wrong; see 3d.

### 3d. The rules are judged in the worktree that contains the edited file

The limitation in 3a was not hypothetical. The orchestrator's session sits in
the primary tree on `main`, and the agents it launches edit files by absolute
path inside sibling worktrees (`frevo-change` on `change/<id>`, and so on).
Claude Code runs the session's hooks with the session's directory, so for every
such edit:

- `test-integrity.sh` read the branch as `main`, found no active change,
  searched only `openspec/specs/`, and **blocked** a `covers:` id that sat in
  the edited worktree's delta. An agent got past it by writing the file through
  the shell, which is precisely the workaround the hooks exist to make
  unnecessary.
- `require-active-change.sh` and `protect-paths.sh` saw an absolute path
  outside "the" repository and matched no rule, so they **allowed** it. An edit
  to `crates/` on a change whose review was not `APPROVED`, a settled spec, or
  `.claude/**` from a product branch all went through silently, as long as the
  session happened to sit in a different worktree.

The loud half again hid the quiet half: one hook blocking was reported, three
hooks not applying at all was not.

`_lib.sh` now has `use_input_root`, which every hook that reads a payload calls
before `require_repo_root`. It re-points `REPO_ROOT` at the worktree the call is
about: the one containing `tool_input.file_path` (or `notebook_path`/`path`),
else the one containing the payload's `cwd` (the session's current directory,
which need not be the directory the hook process started in). Branch, active
change, change delta, relative paths, and H8's search of `crates/` and
`harness/` for `covers:` tags all follow from that root.

Two constraints keep it from becoming a way round the rules:

- It only moves to another worktree of the **same** repository, compared by
  `git rev-parse --git-common-dir`. A path in an unrelated repository, or in
  none, leaves `REPO_ROOT` alone and matches no repo-relative rule, as before.
- It never fails open. When nothing qualifies, `REPO_ROOT` is unchanged, and
  `require_repo_root` still refuses a root it cannot reach (3b).

H8 (`SubagentStop`) has no file path to go by and relies on the payload's
`cwd`. A subagent whose `cwd` is the primary tree but that edited a sibling by
absolute path is still judged in the primary tree, where no change is active,
so H8 allows the stop. That is H8's documented fail-open as a completeness
check; the per-edit guards above, which now judge the right tree, are the
primary control.

The self-test that pinned "a path in another worktree matches no rule" now
asserts the opposite, and a new group runs the hooks with both the process
directory and the payload `cwd` on `main` while the edited file is in a sibling
worktree on `change/<id>` whose delta alone defines the `covers:` id. Six of
those cases fail against the pre-fix hooks.

### 3b. The hooks no longer fail open on an unreachable root

Every hook began with `cd "$REPO_ROOT" 2>/dev/null || exit 0`, and exit 0
**allows** the action — the same fail-open that the jq resolution
below was written to avoid. The four `PreToolUse` guards and the `Stop` guard
now call `require_repo_root`, which exits 2 with an explanation. The
informational hooks (`SessionStart`, `PostToolUse`, `SubagentStop`) keep the
lenient behaviour deliberately: blocking a session start or a post-edit report
buys nothing, and H8 is documented to fail open.

### 3c. Generated code is `src/`, not the whole crate

`protect-paths.sh` matched `crates/frevo-model/*` and refused
`crates/frevo-model/tests/**` as generated code. Only `src/` is generated —
and not all of it, since `design.md` puts the hand-written codec under
`crates/frevo-model/src/codec/`. The rule is now
`crates/frevo-model/src/*|crates/frevo-convert/src/*`, and the `@generated`
marker check still catches the real outputs file by file, which is the check
that does not depend on guessing where a generator writes.

### 4. jq resolution falls back, but never fails open

jq is the documented dependency, but on the bootstrap machine winget installed
it into a shim directory that an already-running shell never picked up. `_lib.sh`
resolves jq from `PATH` and then from the known winget locations, falls back to
`python3`, and if neither is available **exits 2 with an explanation** rather
than allowing an unchecked action.

### 5. Hook self-tests are themselves a gate

`harness/hook-tests/run.sh` runs each hook against crafted stdin inside a
throwaway git repository, with at least one positive and one negative case per
rule — 102 cases. It runs in CI as part of `openspec-guard`. A hook change that
stops blocking fails the build.

The count went from 74 to 87 when the linked-worktree cases were added (and to
102 with the cross-worktree cases of section 3d), and
that increase is the lesson of section 3a: a self-test suite is only as good
as the layouts its fixture can express. These hooks have now twice been green
while enforcing against the wrong repository. Where a rule depends on
something environmental — a path form, a working directory, a worktree —
the test has to reproduce the environment rather than approximate it.

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
- Claude Code puts the edited paths, or the subagent's own worktree, in the
  `SubagentStop` payload, at which point H8 can be judged in the tree the
  subagent actually edited rather than its `cwd` (section 3d).
- The OpenSpec CLI changes `validate` or `archive` flags — the hooks call them
  directly and would need updating in the same commit.
