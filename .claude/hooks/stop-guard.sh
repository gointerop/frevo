#!/usr/bin/env bash
# H7 -- Stop
#
# Stops a session from ending with work in an unrecoverable state. Exit 2 means
# "keep working" and the reason is returned to the agent.
#
# Blocks when, on a change/* branch:
#   - the working tree has uncommitted changes
#   - code changed since tasks.md was last updated
#   - the gate report for HEAD is red and no fix has been attempted since
#   - a task is ticked but has no corresponding commit
#
# Loop safety: Claude Code sets stop_hook_active when it is re-prompting us
# after a block. After 3 consecutive blocks we allow the stop and append a
# BLOCKED entry to tasks/blockers.md for the owner, because an agent that cannot
# satisfy the hook will not satisfy it on the fourth try either -- it needs a
# human.

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
cd "$REPO_ROOT" 2>/dev/null || exit 0

STATE_DIR="$(git rev-parse --git-dir 2>/dev/null || echo .git)"
COUNTER="$STATE_DIR/frevo-stop-blocks"

stop_hook_active="$(json_get "$INPUT" stop_hook_active false)"
branch="$(current_branch)"
change="$(active_change)"

# Not a product branch: nothing here to protect.
if [ -z "$change" ]; then
  rm -f "$COUNTER" 2>/dev/null
  allow
fi

count=0
[ -f "$COUNTER" ] && count="$(cat "$COUNTER" 2>/dev/null || echo 0)"
[ "$stop_hook_active" != "true" ] && count=0

reasons=""

# --- 1. uncommitted changes ------------------------------------------------
dirty_list="$(git status --porcelain 2>/dev/null)"
if [ -n "$dirty_list" ]; then
  reasons="$reasons
* Uncommitted changes. A reset context cannot recover unstaged work, and the
  gate report is keyed to a commit SHA, so uncommitted work is invisible to
  every gate.

$(printf '%s' "$dirty_list" | head -15)

  Commit it:  git add -A && git commit -s -m \"feat(<crate>): <what now works>\""
fi

# --- 2. code changed after tasks.md was last touched -----------------------
tasks="openspec/changes/$change/tasks.md"
if [ -f "$tasks" ]; then
  tasks_commit="$(git log -1 --format=%ct -- "$tasks" 2>/dev/null || echo 0)"
  code_commit="$(git log -1 --format=%ct -- crates 2>/dev/null || echo 0)"
  if [ "${code_commit:-0}" -gt "${tasks_commit:-0}" ]; then
    reasons="$reasons
* crates/ was committed after $tasks was last updated. The task list is how the
  next session knows what is already done; if it is stale, work gets repeated or
  skipped.

  Tick the task you completed in $tasks and commit it."
  fi

  # --- 4. ticked task with no commit --------------------------------------
  n_done="$(grep -cE '^\s*- \[[xX]\]' "$tasks" 2>/dev/null || true)"
  n_commits="$(git rev-list --count HEAD 2>/dev/null || echo 0)"
  base="$(git merge-base HEAD main 2>/dev/null || echo "")"
  if [ -n "$base" ]; then
    n_commits="$(git rev-list --count "$base"..HEAD 2>/dev/null || echo 0)"
  fi
  if [ "${n_done:-0}" -gt 0 ] && [ "${n_commits:-0}" -eq 0 ]; then
    reasons="$reasons
* $n_done task(s) are ticked in $tasks but this branch has no commit of its own.
  A ticked box with no commit is a claim with no evidence.

  Commit the work, or untick what was not actually done."
  fi
fi

# --- 3. red gate report with no fix since ---------------------------------
report="gates/$change/report.json"
if [ -f "$report" ]; then
  rj="$(cat "$report" 2>/dev/null)"
  rgreen="$(json_get "$rj" green false)"
  rsha="$(json_get "$rj" head_sha)"
  head_sha="$(git rev-parse HEAD 2>/dev/null)"
  if [ "$rgreen" != "true" ] && [ "$rsha" = "$head_sha" ]; then
    reasons="$reasons
* The gate report for this exact commit (${head_sha:0:8}) is red, and nothing has
  changed since it was written.

$(printf '%s' "$rj" | grep -E '\"(green|detail|failures|diffs)\"' | head -12)

  Fix the failures and re-run gatekeeper. If a case genuinely cannot pass, record
  it in docs/known-gaps.md and stop for owner approval -- do not weaken it."
  fi
fi

# --- verdict ---------------------------------------------------------------
if [ -z "$reasons" ]; then
  rm -f "$COUNTER" 2>/dev/null
  allow
fi

count=$(( count + 1 ))
printf '%s' "$count" > "$COUNTER" 2>/dev/null

if [ "$count" -ge 3 ]; then
  rm -f "$COUNTER" 2>/dev/null
  mkdir -p tasks 2>/dev/null
  {
    echo
    echo "## BLOCKED: $change ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo
    echo "The stop hook blocked three consecutive times and then allowed the stop."
    echo "This needs the owner, not another attempt."
    echo
    echo "Branch: $branch"
    echo "HEAD:   $(git rev-parse HEAD 2>/dev/null)"
    echo
    echo "Unresolved:"
    printf '%s\n' "$reasons"
  } >> tasks/blockers.md 2>/dev/null
  echo "Stop allowed after 3 blocks. Appended a BLOCKED entry to tasks/blockers.md for owner review." >&2
  exit 0
fi

block <<EOF
Not done yet (block $count of 3). The change '$change' would be left in a state a
reset context cannot pick up:
$reasons

If you believe these cannot be resolved, say what you tried; after three blocks
this hook records a BLOCKED entry in tasks/blockers.md and lets the session end.
EOF
