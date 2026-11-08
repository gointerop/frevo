#!/usr/bin/env bash
# Scenario coverage, checked in both directions.
#
#   forward : every scenario id declared in a spec has at least one test tagged
#             "// covers: <id>"
#   reverse : every "covers:" tag resolves to a declared scenario id
#
# Both matter. Without the forward check, specified behaviour can ship untested.
# Without the reverse check, a typo in a tag reports coverage that nobody ever
# specified -- which is worse, because it looks like coverage.
#
# Usage:
#   harness/scripts/scenario-coverage.sh [--change <id>] [--markdown] [--quiet]
#
#   --change <id>  restrict the forward check to one change's spec delta
#                  (the reverse check always resolves against every spec)
#   --markdown     emit a report suitable for a pull-request comment
#
# Exit 0 if coverage holds in both directions, 1 otherwise.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

CHANGE=""; MARKDOWN=0; QUIET=0
while [ $# -gt 0 ]; do
  case "$1" in
    --change)   CHANGE="${2:-}"; shift 2 ;;
    --markdown) MARKDOWN=1; shift ;;
    --quiet)    QUIET=1; shift ;;
    -h|--help)  sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

SCENARIO_RE='[a-z0-9_]+\.[a-z0-9_]+\.[0-9]+'

# --------------------------------------------------------------------------
# Collect declared scenario ids.
#   Declared as:  #### Scenario: [<id>] <description>
# --------------------------------------------------------------------------
declared_roots=()
[ -d openspec/specs ] && declared_roots+=(openspec/specs)
if [ -n "$CHANGE" ]; then
  [ -d "openspec/changes/$CHANGE" ] && declared_roots+=("openspec/changes/$CHANGE")
else
  [ -d openspec/changes ] && declared_roots+=(openspec/changes)
fi

ALL_DECLARED=""
if [ "${#declared_roots[@]}" -gt 0 ]; then
  ALL_DECLARED="$(grep -rhoE "#### Scenario: \[$SCENARIO_RE\]" "${declared_roots[@]}" 2>/dev/null \
                   | sed -E 's/.*\[([^]]+)\].*/\1/' | sort -u)"
fi

# The forward check's scope: one change's delta if given, otherwise everything.
FORWARD_SCOPE="$ALL_DECLARED"
if [ -n "$CHANGE" ] && [ -d "openspec/changes/$CHANGE" ]; then
  FORWARD_SCOPE="$(grep -rhoE "#### Scenario: \[$SCENARIO_RE\]" "openspec/changes/$CHANGE" 2>/dev/null \
                    | sed -E 's/.*\[([^]]+)\].*/\1/' | sort -u)"
fi

# For the reverse check, a tag may legitimately reference an archived scenario.
if [ -d openspec/changes/archive ]; then
  ARCHIVED="$(grep -rhoE "#### Scenario: \[$SCENARIO_RE\]" openspec/changes/archive 2>/dev/null \
               | sed -E 's/.*\[([^]]+)\].*/\1/' | sort -u)"
  ALL_DECLARED="$(printf '%s\n%s\n' "$ALL_DECLARED" "$ARCHIVED" | grep -E "^$SCENARIO_RE$" | sort -u)"
fi

# --------------------------------------------------------------------------
# Collect tagged ids from tests and from the differential corpus.
# --------------------------------------------------------------------------
tag_roots=()
[ -d crates ]  && tag_roots+=(crates)
[ -d harness ] && tag_roots+=(harness)

TAGGED=""
if [ "${#tag_roots[@]}" -gt 0 ]; then
  TAGGED="$(grep -rhoE "covers:[^\"]*" "${tag_roots[@]}" \
              --include='*.rs' --include='*.yaml' --include='*.yml' 2>/dev/null \
            | sed 's/covers://' | tr ',' '\n' | tr -d ' \r' \
            | grep -E "^$SCENARIO_RE$" | sort -u)"
fi

# --------------------------------------------------------------------------
# Compare
# --------------------------------------------------------------------------
UNCOVERED=""   # declared (in scope) but never tagged
UNKNOWN=""     # tagged but not declared anywhere

for id in $FORWARD_SCOPE; do
  printf '%s\n' "$TAGGED" | grep -qxF "$id" || UNCOVERED="$UNCOVERED $id"
done
for id in $TAGGED; do
  printf '%s\n' "$ALL_DECLARED" | grep -qxF "$id" || UNKNOWN="$UNKNOWN $id"
done

n_scope=$(printf '%s\n' $FORWARD_SCOPE | grep -c . || true)
n_tagged=$(printf '%s\n' $TAGGED       | grep -c . || true)
n_unc=$(printf '%s\n' $UNCOVERED       | grep -c . || true)
n_unk=$(printf '%s\n' $UNKNOWN         | grep -c . || true)
covered=$(( ${n_scope:-0} - ${n_unc:-0} ))

rc=0
[ "${n_unc:-0}" -gt 0 ] && rc=1
[ "${n_unk:-0}" -gt 0 ] && rc=1

if [ "$MARKDOWN" -eq 1 ]; then
  echo "### Scenario coverage"
  echo
  if [ -n "$CHANGE" ]; then echo "Change: \`$CHANGE\`"; echo; fi
  echo "| | |"
  echo "|---|---|"
  echo "| scenarios in scope | ${n_scope:-0} |"
  echo "| covered by a test | $covered |"
  echo "| **uncovered** | **${n_unc:-0}** |"
  echo "| tags that resolve to no scenario | ${n_unk:-0} |"
  echo "| distinct \`covers:\` tags found | ${n_tagged:-0} |"
  echo
  if [ "${n_unc:-0}" -gt 0 ]; then
    echo "#### Scenarios with no test"
    echo
    echo "Every specified behaviour needs at least one test tagged \`// covers: <id>\`."
    echo
    for id in $UNCOVERED; do echo "- \`$id\`"; done
    echo
  fi
  if [ "${n_unk:-0}" -gt 0 ]; then
    echo "#### Tags that resolve to no scenario"
    echo
    echo "These claim coverage of behaviour that is not specified anywhere. Fix the"
    echo "tag, or specify the scenario first."
    echo
    for id in $UNKNOWN; do echo "- \`$id\`"; done
    echo
  fi
  [ "$rc" -eq 0 ] && echo "Coverage holds in both directions."
elif [ "$QUIET" -eq 0 ]; then
  echo "scenarios in scope : ${n_scope:-0}"
  echo "covered            : $covered"
  echo "uncovered          : ${n_unc:-0}${UNCOVERED}"
  echo "unresolved tags    : ${n_unk:-0}${UNKNOWN}"
fi

exit $rc
