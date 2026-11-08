#!/usr/bin/env bash
# H5 -- PreToolUse on Bash
#
# Blocks shell commands that would bypass the process rather than follow it:
#   - openspec archive before the tasks are done and the gates are green
#   - git push to main, or any force push
#   - git commit --no-verify
#   - narrowing a test run inside a gate script
#   - rm / git rm on tests or oracle data

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
require_repo_root

CMD="$(json_get "$INPUT" tool_input.command)"
[ -z "$CMD" ] && allow

# Collapse line continuations and runs of whitespace so patterns cannot be
# evaded by wrapping the command across lines.
FLAT="$(printf '%s' "$CMD" | tr '\n' ' ' | sed 's/\\ / /g; s/[[:space:]][[:space:]]*/ /g')"

# --------------------------------------------------------------------------
# 1. openspec archive
# --------------------------------------------------------------------------
if printf '%s' "$FLAT" | grep -qE '(^|[;&|] *)openspec +archive\b'; then
  change="$(active_change)"
  # Allow an explicit change name on a non-change branch only if it matches.
  named="$(printf '%s' "$FLAT" | sed -nE 's/.*openspec +archive +([a-z0-9][a-z0-9-]*).*/\1/p' | head -1)"
  [ -z "$change" ] && change="$named"

  if [ -z "$change" ]; then
    block <<EOF
BLOCKED: 'openspec archive' with no identifiable change.

Archiving is what promotes a change's spec delta into openspec/specs/, so it must
name exactly one change and run from that change's branch (change/<id>).
EOF
  fi

  tasks="openspec/changes/$change/tasks.md"
  if [ -f "$tasks" ]; then
    open_n="$(grep -cE '^\s*- \[ \]' "$tasks" 2>/dev/null || true)"
    if [ "${open_n:-0}" -gt 0 ]; then
      block <<EOF
BLOCKED: cannot archive '$change' -- $open_n task(s) in $tasks are still unchecked:

$(grep -nE '^\s*- \[ \]' "$tasks" | head -10)

Archiving publishes the spec delta as settled requirements. Doing that with open
tasks records behaviour the project does not actually have.
EOF
    fi
  else
    block <<EOF
BLOCKED: cannot archive '$change' -- $tasks does not exist.

A change with no task list has no record of what was meant to be done, so there
is nothing to verify as complete.
EOF
  fi

  report="gates/$change/report.json"
  head_sha="$(git rev-parse HEAD 2>/dev/null)"
  if [ ! -f "$report" ]; then
    block <<EOF
BLOCKED: cannot archive '$change' -- no gate report at $report.

Run the gatekeeper subagent. It runs every gate (fmt, clippy, tests, scenario
coverage, the HL7 oracle suite, the differential harness and the performance
budget) and writes the verdict.
EOF
  fi

  rj="$(cat "$report")"
  rgreen="$(json_get "$rj" green false)"
  rsha="$(json_get "$rj" head_sha)"

  if [ "$rsha" != "$head_sha" ]; then
    block <<EOF
BLOCKED: cannot archive '$change' -- the gate report is stale.

  report head_sha : ${rsha:-<missing>}
  actual HEAD     : $head_sha

The code changed after the gates ran, so the verdict does not describe what you
are about to archive. Re-run gatekeeper.
EOF
  fi

  if [ "$rgreen" != "true" ]; then
    block <<EOF
BLOCKED: cannot archive '$change' -- the gate report is not green.

$(printf '%s' "$rj" | head -40)

Every gate must pass. There is no partial credit: fix the failures, re-run
gatekeeper, then archive.
EOF
  fi
fi

# --------------------------------------------------------------------------
# 2. git push protections
# --------------------------------------------------------------------------
if printf '%s' "$FLAT" | grep -qE '(^|[;&|] *)git +(-[^ ]+ +)*push\b'; then
  if printf '%s' "$FLAT" | grep -qE 'push.*(--force\b|--force-with-lease=[^ ]*|[[:space:]]-f([[:space:]]|$))'; then
    block <<EOF
BLOCKED: force push.

A force push rewrites history other agents, worktrees and CI runs are built on,
and it can erase a gate result that referenced a commit by SHA.

Add a new commit instead. If history genuinely must be rewritten, that is an
owner decision.
EOF
  fi
  if printf '%s' "$FLAT" | grep -qE 'push[^;&|]*(\bmain\b|HEAD:main|:main\b)'; then
    block <<EOF
BLOCKED: push to main.

main only ever moves through a reviewed pull request whose CI is green -- that is
what makes the openspec-guard job a real backstop rather than advisory.

  git push -u origin HEAD
  gh pr create --fill
EOF
  fi
  # A bare 'git push' while standing on main is the same thing.
  if [ "$(current_branch)" = "main" ] && printf '%s' "$FLAT" | grep -qE 'git +(-[^ ]+ +)*push( +--?[a-z-]+)* *($|[;&|])'; then
    block <<EOF
BLOCKED: 'git push' while on main pushes main.

Move the work to a branch and open a pull request:

  git switch -c change/<id>     # or chore/<slug>
  git push -u origin HEAD
  gh pr create --fill
EOF
  fi
fi

# --------------------------------------------------------------------------
# 3. --no-verify
# --------------------------------------------------------------------------
if printf '%s' "$FLAT" | grep -qE 'git +(-[^ ]+ +)*(commit|push)[^;&|]*--no-verify'; then
  block <<EOF
BLOCKED: --no-verify.

That flag exists to skip the checks this project relies on. If a hook is wrong,
fix the hook on a chore/governance-* branch -- do not route around it.
EOF
fi

# --------------------------------------------------------------------------
# 4. Narrowed test runs inside gate scripts
# --------------------------------------------------------------------------
# A filtered run is fine interactively; it is not fine when it is what a gate
# script executes, because then the gate silently stops covering most cases.
if printf '%s' "$FLAT" | grep -qE 'cargo +(nextest +run|test)\b' \
   && printf '%s' "$FLAT" | grep -qE '(>|>>|tee) *[^ ]*(gates/|report\.json)'; then
  if printf '%s' "$FLAT" | grep -qE 'cargo +(nextest +run|test)[^;&|]*(-E |--filter|--skip|--exact|--test-threads=1 +[a-z])' \
     || printf '%s' "$FLAT" | grep -qE 'cargo +(nextest +run|test) +[a-z_][a-z_0-9:]*( |$)'; then
    block <<EOF
BLOCKED: a filtered test run is being written into a gate report.

The tests gate means the whole workspace suite passed. A filtered run that is
recorded as the gate result reports a pass for cases that never executed.

  cargo nextest run --workspace
  cargo test --workspace --doc

Filter freely while debugging -- just never as the thing a gate records.
EOF
  fi
fi

# --------------------------------------------------------------------------
# 5. Deleting tests or oracle data
# --------------------------------------------------------------------------
if printf '%s' "$FLAT" | grep -qE '(^|[;&|] *)(rm|git +rm|del|Remove-Item)\b'; then
  if printf '%s' "$FLAT" | grep -qE '(oracles/|/tests/|tests/|_test\.rs|test_[a-z0-9_]*\.rs|harness/corpus)'; then
    block <<EOF
BLOCKED: this command deletes tests or oracle data.

  $CMD

Tests and oracles are the definition of correct here; removing them is how a
project convinces itself it is finished. If something is genuinely obsolete,
specify its removal, have spec-reviewer approve it, and remove it in that change
with the reasoning recorded.
EOF
  fi
fi

allow
