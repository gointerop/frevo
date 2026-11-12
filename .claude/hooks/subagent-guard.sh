#!/usr/bin/env bash
# H8 -- SubagentStop
#
# Each role has one deliverable. A subagent that finishes without producing it
# has not done its job, and the orchestrator would otherwise proceed as though
# it had.
#
#   spec-reviewer  -> openspec/changes/<id>/review.md, with a verdict on line 1
#                     and, for CHANGES_REQUESTED, a numbered list of findings
#   gatekeeper     -> gates/<id>/report.json, for the current HEAD
#   oracle-builder -> tests tagged with scenario ids from the change
#
# Identifying the role: the installed Claude Code version may or may not include
# the subagent identity in the hook payload. We read every field it might use
# and, when none is present, fall back to inferring the role from what changed
# in the working tree. Where even that is ambiguous we allow the stop rather than
# blocking the wrong agent -- the role boundaries themselves are enforced
# up-front by protect-paths.sh and by each agent's tool allowlist, so this hook
# is a completeness check, not the primary control.

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
use_input_root "$INPUT"
cd "$REPO_ROOT" 2>/dev/null || exit 0

stop_hook_active="$(json_get "$INPUT" stop_hook_active false)"
[ "$stop_hook_active" = "true" ] && allow   # already re-prompted once; do not loop

change="$(active_change)"
[ -z "$change" ] && allow

# --- identify the role ----------------------------------------------------
role=""
for key in agent_type subagent_type subagent_name agent_name tool_input.subagent_type; do
  v="$(json_get "$INPUT" "$key")"
  if [ -n "$v" ]; then role="$v"; break; fi
done

if [ -z "$role" ]; then
  # Infer from what this subagent left behind.
  changed="$(git status --porcelain 2>/dev/null | awk '{print $2}')"
  if printf '%s' "$changed" | grep -q "openspec/changes/$change/review.md"; then
    role="spec-reviewer"
  elif printf '%s' "$changed" | grep -q "gates/$change/"; then
    role="gatekeeper"
  else
    allow   # cannot tell; do not block on a guess
  fi
fi

dir="openspec/changes/$change"

case "$role" in
  *spec-reviewer*)
    review="$dir/review.md"
    if [ ! -f "$review" ]; then
      block <<EOF
spec-reviewer finished without writing $review.

That file is the gate on all product code: no file under crates/ can be edited
until its first line reads APPROVED. Without it the change cannot proceed at all.

Write $review with:
  line 1: APPROVED   or   CHANGES_REQUESTED
  for CHANGES_REQUESTED: a numbered list of findings, each naming the upstream
  evidence (reference/ path and line, or the HAPI test method) for the behaviour
  that is missing, wrong or untestable.
EOF
    fi

    verdict="$(head -1 "$review" | tr -d '\r' | tr -d '[:space:]')"
    if [ "$verdict" != "APPROVED" ] && [ "$verdict" != "CHANGES_REQUESTED" ]; then
      block <<EOF
$review starts with '$verdict', which is not a verdict.

The first line is parsed by the hooks and by openspec-guard. It must be exactly
one of:

  APPROVED
  CHANGES_REQUESTED
EOF
    fi

    if [ "$verdict" = "CHANGES_REQUESTED" ]; then
      n="$(grep -cE '^\s*[0-9]+\.' "$review" 2>/dev/null || true)"
      if [ "${n:-0}" -eq 0 ]; then
        block <<EOF
$review requests changes but lists no numbered findings.

A verdict without specifics cannot be acted on, so the proposer would have to
guess what to fix. Add a numbered list; each item names the behaviour that is
missing or wrong and the upstream evidence for it.
EOF
      fi
    fi
    ;;

  *gatekeeper*)
    report="gates/$change/report.json"
    if [ ! -f "$report" ]; then
      block <<EOF
gatekeeper finished without writing $report.

The report is the only record of whether the gates passed, and archiving reads
it. Write it with the schema in .claude/agents/gatekeeper.md: a result entry per
gate (fmt, clippy, tests, coverage, oracle, differential, performance), the
head_sha, and a top-level "green" that is the conjunction of them all.
EOF
    fi

    rj="$(cat "$report" 2>/dev/null)"
    rsha="$(json_get "$rj" head_sha)"
    head_sha="$(git rev-parse HEAD 2>/dev/null)"
    if [ "$rsha" != "$head_sha" ]; then
      block <<EOF
$report records head_sha '${rsha:-<missing>}' but HEAD is '$head_sha'.

A report for a different commit says nothing about this one. Re-run the gates
against the current HEAD and rewrite the report.
EOF
    fi

    g="$(json_get "$rj" green)"
    if [ "$g" != "true" ] && [ "$g" != "false" ]; then
      block <<EOF
$report has no boolean "green" field (found: '${g:-<missing>}').

"green" is read by the archive guard and by CI. It must be true or false, and it
must be the conjunction of every gate result -- there is no partial credit.
EOF
    fi
    ;;

  *oracle-builder*)
    # The deliverable is tests that reference this change's scenario ids.
    ids="$(grep -rhoE '#### Scenario: \[[a-z0-9_]+\.[a-z0-9_]+\.[0-9]+\]' "$dir" 2>/dev/null \
            | sed -E 's/.*\[([^]]+)\].*/\1/' | sort -u)"
    if [ -z "$ids" ]; then
      allow   # no scenarios specified yet; nothing to require tests for
    fi

    missing=""
    # Search the worktree the subagent worked in (see use_input_root), not
    # whichever tree the hook process happened to start in.
    for id in $ids; do
      if ! grep -rqF "covers: $id" "$REPO_ROOT/crates" "$REPO_ROOT/harness" 2>/dev/null \
         && ! grep -rqE "covers:.*\b${id//./\\.}\b" "$REPO_ROOT/crates" "$REPO_ROOT/harness" 2>/dev/null; then
        missing="$missing $id"
      fi
    done

    if [ -n "$missing" ]; then
      block <<EOF
oracle-builder finished, but these scenario ids from '$change' have no test
tagged for them:$missing

Tests come before the implementation here, so every scenario needs its failing
test now -- otherwise the implementer has nothing to work against and the
coverage gate will fail later.

  // covers: <scenario-id>
  #[test]
  fn <name>() { ... }

Add a test per scenario (or a differential corpus entry under harness/corpus/
listing the id), then confirm each fails for the right reason.
EOF
    fi
    ;;
esac

allow
