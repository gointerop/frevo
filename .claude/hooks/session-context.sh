#!/usr/bin/env bash
# H1 -- SessionStart
#
# Prints where the work stands. The constitution assumes agent context is reset
# at any time, so this is the recovery mechanism: branch, active change, its
# unchecked tasks, the last gate verdict, and what is unblocked next.
#
# This hook never blocks. It always exits 0: failing to print context must not
# prevent a session from starting.

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

cd "$REPO_ROOT" 2>/dev/null || exit 0

branch="$(current_branch)"
change="$(active_change)"

echo "=============================================================="
echo " Frevo session context"
echo "=============================================================="
echo "branch : ${branch:-<detached>}"

# --- branch kind, and what it permits -------------------------------------
case "$branch" in
  main)
    echo "kind   : main -- product work does NOT belong here."
    echo "         Start a change:     git switch -c change/<id>"
    echo "         Or infrastructure:  git switch -c chore/<slug>"
    ;;
  change/*)
    echo "kind   : product change"
    echo "change : $change"
    ;;
  chore/governance-*)
    echo "kind   : governance (may modify .claude/**, gates/**, openspec-guard.yml)"
    ;;
  chore/oracles-*)
    echo "kind   : oracles (may modify oracles/**)"
    ;;
  chore/*)
    echo "kind   : infrastructure (no behavioural edits under crates/**/src)"
    ;;
  *)
    echo "kind   : UNRECOGNISED. Use change/<id> or chore/<slug>."
    ;;
esac

# --- uncommitted work ------------------------------------------------------
dirty="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
echo "dirty  : $dirty uncommitted path(s)"

# --- the active change -----------------------------------------------------
if [ -n "$change" ]; then
  dir="openspec/changes/$change"
  echo
  echo "--- change: $change -------------------------------------------"
  if [ ! -d "$dir" ]; then
    echo "  NO PROPOSAL at $dir"
    echo "  Next: run /opsx:propose to create it, or switch branches."
  else
    # Review verdict decides whether crates/** may be touched at all.
    if [ -f "$dir/review.md" ]; then
      verdict="$(head -1 "$dir/review.md" | tr -d '\r' | tr -d ' ')"
      echo "  review : $verdict"
      [ "$verdict" != "APPROVED" ] && \
        echo "           crates/** is BLOCKED until this reads APPROVED."
    else
      echo "  review : (none) -- crates/** is BLOCKED until spec-reviewer writes review.md"
    fi

    os="$(openspec_bin)"
    if [ -n "$os" ]; then
      echo "  --- openspec show ---"
      "$os" show "$change" --no-interactive 2>/dev/null | head -25 | sed 's/^/  /' \
        || echo "  (openspec show produced no output)"
    fi

    if [ -f "$dir/tasks.md" ]; then
      total="$(grep -cE '^\s*- \[[ xX]\]' "$dir/tasks.md" 2>/dev/null || true)"
      done_n="$(grep -cE '^\s*- \[[xX]\]' "$dir/tasks.md" 2>/dev/null || true)"
      echo "  tasks  : $done_n/$total complete"
      echo "  --- next unchecked tasks ---"
      grep -nE '^\s*- \[ \]' "$dir/tasks.md" 2>/dev/null | head -5 | sed 's/^/    /' \
        || echo "    (none -- all tasks checked)"
    else
      echo "  tasks  : no tasks.md yet"
    fi
  fi

  # --- gate report ---------------------------------------------------------
  echo
  report="gates/$change/report.json"
  head_sha="$(git rev-parse HEAD 2>/dev/null)"
  if [ -f "$report" ]; then
    rj="$(cat "$report" 2>/dev/null)"
    g="$(json_get "$rj" green false)"
    rs="$(json_get "$rj" head_sha)"
    if [ "$rs" != "$head_sha" ]; then
      echo "gate   : STALE -- report is for ${rs:0:8}, HEAD is ${head_sha:0:8}. Re-run gatekeeper."
    else
      echo "gate   : green=$g for HEAD ${head_sha:0:8}"
    fi
  else
    echo "gate   : no report at $report -- gatekeeper has not run for this change."
  fi
fi

# --- next unblocked work from the task graph -------------------------------
echo
if [ -f tasks/graph.yaml ]; then
  echo "--- next unblocked work ---"
  if [ -x tasks/next.sh ]; then
    tasks/next.sh --limit 3 2>/dev/null | sed 's/^/  /' || echo "  (tasks/next.sh failed)"
  else
    echo "  (tasks/next.sh not executable; see tasks/graph.yaml)"
  fi
else
  echo "--- task graph not yet built (tasks/graph.yaml absent) ---"
fi

# --- blockers awaiting the owner ------------------------------------------
if [ -f tasks/blockers.md ] && grep -q '^## BLOCKED' tasks/blockers.md 2>/dev/null; then
  n="$(grep -c '^## BLOCKED' tasks/blockers.md)"
  echo
  echo "!! $n blocker(s) in tasks/blockers.md awaiting the owner."
fi

echo "=============================================================="
exit 0
