#!/usr/bin/env bash
# H2 -- PreToolUse on Edit|Write|MultiEdit
#
# Product code may only be written in service of an approved specification.
# For any path under crates/, this requires:
#   1. the branch is change/<id>
#   2. openspec/changes/<id>/ exists
#   3. openspec validate <id> --strict passes
#   4. review.md starts with APPROVED
#
# Exit 2 blocks and returns stderr to the agent.

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
require_repo_root

# Only guard product code. Everything else is handled by protect-paths.sh.
guarded=0
while IFS= read -r p; do
  [ -z "$p" ] && continue
  case "$p" in crates/*) guarded=1 ;; esac
done <<< "$(target_paths "$INPUT")"
[ "$guarded" -eq 0 ] && allow

branch="$(current_branch)"
change="$(active_change)"

if [ -z "$change" ]; then
  block <<EOF
BLOCKED: cannot edit crates/ on branch '${branch:-<detached>}'.

Product code is written against a reviewed specification, so it needs an active
OpenSpec change. Propose first, then review, then implement:

  git switch -c change/<id>
  /opsx:propose "<what this change does>"
  # then spec-reviewer writes openspec/changes/<id>/review.md

If this edit is infrastructure rather than product behaviour (CI, harness, docs,
tooling), do it on a chore/<slug> branch instead -- those may not change
behaviour under crates/**/src.
EOF
fi

dir="openspec/changes/$change"
if [ ! -d "$dir" ]; then
  block <<EOF
BLOCKED: branch is change/$change but $dir does not exist.

The branch name is the single source of truth for which change is active, so a
change/ branch without a proposal cannot be implemented.

  /opsx:propose "<what this change does>"

Or switch to the branch whose change you meant to work on.
EOF
fi

# Strict validation. If the CLI is missing we say so rather than waving the
# edit through: the whole point of this hook is that the spec was checked.
os="$(openspec_bin)"
if [ -z "$os" ]; then
  block <<EOF
BLOCKED: the openspec CLI is not on PATH, so the specification for '$change'
cannot be validated before editing product code.

  npm i -g @fission-ai/openspec@latest

Then retry.
EOF
fi

if ! vout="$("$os" validate "$change" --strict --no-interactive 2>&1)"; then
  block <<EOF
BLOCKED: 'openspec validate $change --strict' fails, so the specification is not
yet a valid basis for implementation. Fix the spec first.

$vout
EOF
fi

# The review gate.
review="$dir/review.md"
if [ ! -f "$review" ]; then
  block <<EOF
BLOCKED: $review does not exist.

A proposal must be independently reviewed against the upstream Java source
before it is implemented, because the failure mode that costs this project most
is a missed behaviour. Run the spec-reviewer subagent; it writes review.md with
APPROVED or CHANGES_REQUESTED on the first line.
EOF
fi

verdict="$(head -1 "$review" | tr -d '\r' | tr -d '[:space:]')"
if [ "$verdict" != "APPROVED" ]; then
  block <<EOF
BLOCKED: $review says '$verdict', not APPROVED.

crates/ stays closed until the specification is approved. Address the reviewer's
numbered findings in the spec delta, then have spec-reviewer re-review.

Findings:
$(sed -n '2,25p' "$review")
EOF
fi

allow
