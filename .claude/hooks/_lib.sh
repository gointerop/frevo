#!/usr/bin/env bash
# Shared helpers for the Frevo enforcement hooks.
#
# Contract with Claude Code:
#   - hook input arrives as a single JSON object on stdin;
#   - exit 0 allows the action;
#   - exit 2 BLOCKS it, and stderr is returned to the agent as the reason.
#
# Every block message must therefore say exactly what to do instead. A hook that
# blocks without a remedy just makes an agent retry the same thing.
#
# These hooks are also run by harness/hook-tests/run.sh with crafted stdin, and
# by the hook-selftest CI job on Linux. They must not depend on anything outside
# the repository, on the host OS, or on an interactive terminal.

set -uo pipefail

# The hook scripts live in exactly one place, but a session may be working in a
# linked git worktree. Deriving the root from the script's own location
# therefore resolved to the MAIN worktree for every worktree session:
# current_branch() reported main's branch, active_change() came back empty, and
# every path rule compared the edited file against a tree nobody was editing.
# The hooks were enforcing, carefully, against the wrong repository.
#
# The self-tests could not see it. They copy .claude into a throwaway repo and
# run the hooks from its root, so there the script location IS the repo root and
# the two derivations coincide. That is the second time these hooks have passed
# their own tests while enforcing nothing -- the first was the Windows path
# normalisation in ADR 0002 section 3 -- and both times the cause was a fixture
# that could not reproduce the real layout. The worktree cases at the end of
# harness/hook-tests/run.sh exist so this one cannot come back invisibly.
#
# Derive it from the working directory, which Claude Code sets to the session's
# project directory, and fall back to the script location only when the working
# directory is not inside a work tree at all.
_repo_root() {
  local r
  r="$(git rev-parse --show-toplevel 2>/dev/null)"
  if [ -n "${r:-}" ] && [ -d "$r" ]; then
    ( cd "$r" && pwd )
    return 0
  fi
  ( cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd )
}
REPO_ROOT="$(_repo_root)"
export REPO_ROOT

# Every hook begins by entering REPO_ROOT, and every one of them used to do it
# as `cd "$REPO_ROOT" 2>/dev/null || exit 0`. Exit 0 ALLOWS the action, so a
# guard that could not reach the repository waved the edit through -- the same
# fail-open the jq resolution below was written to avoid.
#
# require_repo_root is for the four PreToolUse guards and the Stop guard. The
# informational hooks (SessionStart, PostToolUse, SubagentStop) keep the lenient
# behaviour on purpose: blocking a session start or a post-edit report buys
# nothing, and H8 is a completeness check that is documented to fail open.
require_repo_root() {
  cd "$REPO_ROOT" 2>/dev/null && return 0
  cat >&2 <<EOF
BLOCKED: the enforcement hooks cannot locate the repository root.

REPO_ROOT resolved to: ${REPO_ROOT:-<empty>}

That directory is not reachable, so no path rule, branch rule or coverage check
can be evaluated. The action is refused rather than allowed, because a guard
that cannot check anything must not pretend that it did.

  - run this from inside the repository or one of its worktrees
  - in a worktree, check that \`git rev-parse --show-toplevel\` works there
EOF
  exit 2
}

# --------------------------------------------------------------------------
# JSON
# --------------------------------------------------------------------------
# jq is the documented dependency, but it is not always on PATH (notably on a
# Windows host where winget installed it into a shim directory that an already
# running shell never picked up). Resolve it properly, and fall back to python3
# rather than failing open: a hook that cannot parse its input must never
# silently allow the action it was meant to check.
_find_jq() {
  if command -v jq >/dev/null 2>&1; then echo "jq"; return 0; fi
  local c
  for c in \
    "$LOCALAPPDATA/Microsoft/WinGet/Links/jq.exe" \
    "$HOME/AppData/Local/Microsoft/WinGet/Links/jq.exe" \
    "$HOME/AppData/Local/Microsoft/WinGet/Packages/jqlang.jq_Microsoft.Winget.Source_8wekyb3d8bbwe/jq.exe" \
    "/usr/bin/jq" "/usr/local/bin/jq" "/opt/homebrew/bin/jq"
  do
    [ -n "${c:-}" ] && [ -x "$c" ] && { echo "$c"; return 0; }
  done
  return 1
}
JQ="$(_find_jq || true)"

_find_python() {
  local p
  for p in python3 python py; do
    command -v "$p" >/dev/null 2>&1 && { echo "$p"; return 0; }
  done
  return 1
}
PY="$(_find_python || true)"

# json_get <json> <dotted.path> [default]
# Returns the string value at the path, or the default (empty) if absent/null.
json_get() {
  local json="$1" path="$2" default="${3:-}" out=""
  if [ -n "$JQ" ]; then
    out="$(printf '%s' "$json" | "$JQ" -r --arg d "$default" "(.${path} // \$d) | if type==\"string\" then . else tostring end" 2>/dev/null)"
  elif [ -n "$PY" ]; then
    out="$("$PY" -I -c '
import json,sys
raw,path,default=sys.stdin.read(),sys.argv[1],sys.argv[2]
try: cur=json.loads(raw)
except Exception: print(default); sys.exit(0)
for part in path.split("."):
    if isinstance(cur,dict) and part in cur: cur=cur[part]
    else: print(default); sys.exit(0)
if cur is None: print(default)
elif isinstance(cur,str): print(cur)
elif isinstance(cur,bool): print("true" if cur else "false")
else: print(json.dumps(cur))
' "$path" "$default" <<<"$json" 2>/dev/null)"
  else
    # Neither parser available: fail closed, loudly.
    printf 'FREVO HOOK ERROR: neither jq nor python3 is available, so hook input cannot be parsed.\n' >&2
    printf 'Install jq (https://jqlang.org) and re-run. Refusing to allow an unchecked action.\n' >&2
    exit 2
  fi
  printf '%s' "${out:-$default}"
}

# --------------------------------------------------------------------------
# Outcomes
# --------------------------------------------------------------------------
# block <<'EOF' ... EOF  -- or -- block "message"
# Writes the reason to stderr and exits 2 so Claude Code blocks the action.
block() {
  if [ "$#" -gt 0 ]; then printf '%s\n' "$*" >&2; else cat >&2; fi
  exit 2
}
allow() { exit 0; }

# --------------------------------------------------------------------------
# Repository state
# --------------------------------------------------------------------------
current_branch() { git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "" ; }

# The active change id, derived from the branch name only. Deriving it from
# anything else (a marker file, the most recent directory) would let two
# branches disagree about which change is in flight.
active_change() {
  local b; b="$(current_branch)"
  case "$b" in
    change/*) printf '%s' "${b#change/}" ;;
    *) printf '' ;;
  esac
}

is_governance_branch() { case "$(current_branch)" in chore/governance-*) return 0;; *) return 1;; esac; }
is_oracles_branch()    { case "$(current_branch)" in chore/oracles-*)    return 0;; *) return 1;; esac; }
is_chore_branch()      { case "$(current_branch)" in chore/*)            return 0;; *) return 1;; esac; }

# Path of the edited/written file, normalised to a repo-relative POSIX path.
#
# This is load-bearing: every path rule in protect-paths.sh and
# require-active-change.sh matches on the result, so if normalisation fails the
# hooks silently ALLOW everything. That is exactly what happened before this
# was written properly -- on Windows the hook payload carries a native path
# (C:\repo\crates\x.rs) while bash's pwd reports an MSYS path (/c/repo), and a
# plain string prefix strip never matches. Git Bash also aliases mount points
# (/tmp is not literally C:/.../Temp), so comparing spellings is not enough.
#
# The only reliable comparison is between *physically resolved* paths, so both
# sides are resolved by cd-ing into the deepest existing directory and asking
# the shell where it landed.
_phys_dir() {
  local d="$1"
  [ -d "$d" ] || { printf '%s' "$d"; return; }
  ( cd "$d" 2>/dev/null && { pwd -W 2>/dev/null || pwd; } ) || printf '%s' "$d"
}

# Resolve a possibly-nonexistent path: find its deepest existing ancestor,
# resolve that physically, and re-attach the remainder.
_resolve() {
  local p="${1//\\//}" tail=""
  [ -z "$p" ] && { printf ''; return; }
  while [ -n "$p" ] && [ ! -d "$p" ]; do
    local base="${p##*/}" parent="${p%/*}"
    [ "$parent" = "$p" ] && break          # no slash left
    tail="${base}${tail:+/}$tail"
    p="${parent:-/}"
  done
  local resolved; resolved="$(_phys_dir "$p")"
  resolved="${resolved%/}"
  printf '%s' "${resolved}${tail:+/}$tail"
}

_lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }

rel_path() {
  local raw="${1:-}"
  [ -z "$raw" ] && { printf ''; return; }

  local p root lp lr
  p="$(_resolve "$raw")"
  root="$(_resolve "$REPO_ROOT")"
  root="${root%/}"
  lp="$(_lower "$p")"
  lr="$(_lower "$root")"

  case "$lp" in
    "$lr"/*) p="${p:$(( ${#root} + 1 ))}" ;;
    "$lr")   p="" ;;
    *)
      # Not under the repository at all. Return the normalised absolute path so
      # callers can still reason about it, but it will match no repo-relative
      # rule -- which is correct: the rules are about this repository.
      ;;
  esac
  p="${p#./}"
  printf '%s' "$p"
}

# The file path(s) a tool call targets. Edit/Write use file_path; MultiEdit in
# some versions uses file_path too, and NotebookEdit uses notebook_path.
target_paths() {
  local json="$1" p
  for key in file_path notebook_path path; do
    p="$(json_get "$json" "tool_input.$key")"
    [ -n "$p" ] && { rel_path "$p"; printf '\n'; }
  done
}

# openspec CLI, if installed. Hooks degrade to a clear message rather than
# blocking work when the CLI is absent, except where validation is the point.
openspec_bin() { command -v openspec >/dev/null 2>&1 && echo openspec || echo ""; }

# --------------------------------------------------------------------------
# Which worktree is this call about?
# --------------------------------------------------------------------------
# _repo_root above answers "which worktree is the hook PROCESS in". That is the
# wrong question whenever a session sits in one worktree and edits a file in
# another by absolute path -- the orchestrator on main in the primary tree,
# say, while an agent writes into a sibling worktree on change/<id>. Every rule
# then ran against the session's tree:
#
#   - test-integrity.sh saw no active change, searched only openspec/specs,
#     and BLOCKED a covers: id that sat in the edited worktree's delta (an
#     agent then wrote the file through the shell to get past it);
#   - require-active-change.sh and protect-paths.sh saw an absolute path
#     outside "the" repository, matched no rule, and ALLOWED it -- a settled
#     spec or an unapproved crates/ edit in the sibling went through silently.
#
# use_input_root re-points REPO_ROOT at the worktree the call is actually
# about: the worktree containing the target file, else the worktree of the
# payload's cwd (the session's current directory, which can differ from the
# directory the hook process was started in). Call it BEFORE require_repo_root
# and before anything that reads the branch or a repo-relative path.
#
# It only ever moves to another worktree of the SAME repository, compared by
# git common dir. A path in an unrelated repository, or in none, leaves
# REPO_ROOT alone and so matches no repo-relative rule, exactly as before. It
# never makes a hook fail open: when nothing qualifies, REPO_ROOT is what it
# was, and require_repo_root still refuses an unreachable root.

# Physical path of the git common dir for a directory, or nothing.
_git_common_of() {
  local d="$1" c
  c="$(git -C "$d" rev-parse --git-common-dir 2>/dev/null)" || return 1
  [ -n "$c" ] || return 1
  ( cd "$d" 2>/dev/null && cd "$c" 2>/dev/null && { pwd -W 2>/dev/null || pwd; } )
}

# Physical toplevel of the worktree containing a (possibly nonexistent) path.
_toplevel_for() {
  local p="${1//\\//}" parent t
  [ -n "$p" ] || return 1
  while [ ! -d "$p" ]; do
    parent="${p%/*}"
    [ "$parent" = "$p" ] && return 1
    [ -n "$parent" ] || parent="/"
    p="$parent"
  done
  t="$(git -C "$p" rev-parse --show-toplevel 2>/dev/null)" || return 1
  [ -n "$t" ] && [ -d "$t" ] || return 1
  _phys_dir "$t"
}

use_input_root() {
  local json="$1" base key raw top cand
  base="$(_git_common_of "$REPO_ROOT")"
  [ -n "$base" ] || return 0
  base="$(_lower "$base")"
  for key in tool_input.file_path tool_input.notebook_path tool_input.path cwd; do
    raw="$(json_get "$json" "$key")"
    [ -n "$raw" ] || continue
    top="$(_toplevel_for "$raw")" || continue
    cand="$(_git_common_of "$top")" || continue
    if [ "$(_lower "$cand")" = "$base" ]; then
      REPO_ROOT="$top"
      export REPO_ROOT
      return 0
    fi
  done
  return 0
}
