#!/usr/bin/env bash
# H3 -- PreToolUse on Edit|Write|MultiEdit
#
# Some paths are never hand-edited, because editing them breaks the mechanism
# that makes the rest of the rules mean anything:
#
#   openspec/specs/**     settled requirements; change only via openspec archive
#   crates/frevo-model/src/** generated; fix the generator instead (tests/ is not)
#   any @generated file   same
#   reference/**          read-only pinned upstream source
#   oracles/**            vendored suites; only on chore/oracles-*
#   .claude/**            the enforcement machinery; only on chore/governance-*
#   gates/**              gate verdicts; only on chore/governance-* (gatekeeper
#                         writes report.json through its own path, see below)
#   openspec-guard.yml    the CI backstop; only on chore/governance-*

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
require_repo_root

change="$(active_change)"

while IFS= read -r p; do
  [ -z "$p" ] && continue

  case "$p" in
    openspec/specs/*)
      block <<EOF
BLOCKED: $p is a settled capability spec and is never hand-edited.

Settled specs change in exactly one way: the commit that archives a change.
That is what keeps the spec and the implementation from drifting apart.

  - to change a requirement: edit openspec/changes/<id>/ and archive it
  - to record the outcome of this change: openspec archive $change
EOF
      ;;

    crates/frevo-model/src/*|crates/frevo-convert/src/*)
      block <<EOF
BLOCKED: $p is generated code.

crates/frevo-model/src and crates/frevo-convert/src are produced by
frevo-codegen from FHIR StructureDefinitions. FHIR models are regenerated from
the specification artifacts, never translated or hand-corrected -- a hand edit
is silently lost on the next regeneration.

Fix the generator in crates/frevo-codegen, then regenerate.

This rule covers src/ only. crates/frevo-model/tests/** is ordinary test code
and oracle-builder writes it; the old rule matched the whole crate and refused
those files as generated. The @generated marker check still catches the real
outputs file by file, which is the check that does not depend on guessing which
directories a generator writes to.
EOF
      ;;

    reference/*)
      # PINS.md, clone.sh and README.md are ours; the clones are not.
      case "$p" in
        reference/PINS.md|reference/clone.sh|reference/README.md) ;;
        *)
          block <<EOF
BLOCKED: $p is inside the read-only upstream reference tree.

reference/ holds pinned upstream sources that define what "correct" means.
Modifying them would change the reference behaviour this project is measured
against, silently and locally.

  - to read it: use Read or Grep, which are not restricted
  - to move a pin: edit reference/PINS.md and reference/clone.sh together, and
    treat it as a reviewed decision, not a dependency bump
EOF
          ;;
      esac
      ;;

    oracles/*)
      if ! is_oracles_branch; then
        block <<EOF
BLOCKED: $p is a vendored oracle suite, editable only on a chore/oracles-* branch.

Oracles define correctness. Re-pinning or editing one changes every gate in the
project, so it is isolated to its own branch and its own review.

  git switch -c chore/oracles-<slug>

If you are trying to make a failing oracle case pass by changing the case: that
is never the remedy. Record it in docs/known-gaps.md and stop.
EOF
      fi
      ;;

    .claude/*)
      if ! is_governance_branch; then
        block <<EOF
BLOCKED: $p is part of the enforcement machinery, editable only on a
chore/governance-* branch.

Hooks, agent definitions and settings are what make the other rules binding. An
agent that can edit them while doing product work can remove its own
constraints, so changing them is a separate, reviewed act.

  git switch -c chore/governance-<slug>

Changes there must also pass the hook self-tests (harness/hook-tests/run.sh).
EOF
      fi
      ;;

    gates/*)
      # gatekeeper writes gates/<id>/report.json; that is its whole purpose and
      # is allowed. Anything else under gates/ is governance.
      if [ -n "$change" ] && [ "$p" = "gates/$change/report.json" ]; then
        : # allowed: the verdict for the active change
      elif ! is_governance_branch; then
        block <<EOF
BLOCKED: $p is a gate verdict.

Only gatekeeper writes gate reports, and only for the active change
(gates/$change/report.json). A verdict written by the agent whose work is being
judged is not a verdict.

If you are gatekeeper, write gates/$change/report.json.
Otherwise: fix the failure the report describes.
EOF
      fi
      ;;

    .github/workflows/openspec-guard.yml)
      if ! is_governance_branch; then
        block <<EOF
BLOCKED: $p is the CI backstop for the OpenSpec methodology.

It enforces, for anything that reaches a pull request, the same rules the local
hooks enforce. Weakening it from a product branch would remove the only check
that survives a developer disabling their hooks.

  git switch -c chore/governance-<slug>
EOF
      fi
      ;;
  esac

  # @generated marker anywhere: respect it regardless of location.
  if [ -f "$p" ] && head -20 "$p" 2>/dev/null | grep -q '@generated'; then
    block <<EOF
BLOCKED: $p is marked @generated.

Its content is produced by a generator, so a hand edit is lost the next time the
generator runs and leaves the committed output inconsistent with its source.

Find the generator that writes this file (look for the input it names in its
header) and change that instead.
EOF
  fi
done <<< "$(target_paths "$INPUT")"

allow
