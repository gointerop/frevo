#!/usr/bin/env bash
# Entry point for the three oracles this change adds, with the exit-code
# convention oracles/run.sh already uses:
#
#   0  the oracle ran and Frevo is at or above the reference
#   1  the oracle ran and Frevo is below the reference
#   2  the oracle did not run (not implemented, missing content, missing runtime)
#
# Exit 2 is NOT a pass. That distinction is the whole point: a suite that did
# not run has told us nothing, and the gatekeeper must see the difference.
#
#   bash harness/scripts/run-oracle.sh fhirpath
#   bash harness/scripts/run-oracle.sh path-check-warnings
#   bash harness/scripts/run-oracle.sh model-parser-errors
#   bash harness/scripts/run-oracle.sh --all
#
# Why this script lives in harness/ rather than in oracles/:
# `oracles/**` is editable only on a chore/oracles-* branch, so the
# `fhirpath)` branch of oracles/run.sh cannot be repointed at the Rust harness
# from a change/* branch. The remaining step is a one-line `exec` in
# oracles/fhirpath/runner.sh, on a chore/oracles-* branch; see
# tasks/blockers.md.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

RUNNER="cargo nextest run"
if ! cargo nextest --version >/dev/null 2>&1; then
  # nextest is the gate's runner; cargo test covers the same targets when it
  # is unavailable locally.
  RUNNER="cargo test --no-fail-fast"
fi

note()  { printf '  %s\n' "$*"; }
head2() { printf '\n== %s ==\n' "$*"; }

run_fhirpath() {
  head2 "fhirpath (oracles/fhirpath/tests-fhir-r4.xml)"
  local suite="oracles/fhirpath/tests-fhir-r4.xml"
  if [ ! -f "$suite" ]; then
    note "MISSING: $suite -- it is vendored, so the working tree is damaged."
    note "Check it against its pin: bash oracles/fetch.sh --verify"
    return 2
  fi
  if [ ! -d "oracles/fhir-test-cases/fetched/r4" ]; then
    note "NOT FETCHED: oracles/fhir-test-cases/fetched/r4 -- the suite's 15 input"
    note "files live there and are fetched, not vendored (286 MB)."
    note "Run: bash oracles/fetch.sh"
    note "Until then the 1,101 cases with an inputfile report did-not-run,"
    note "which is NOT a pass."
    return 2
  fi
  note "source      : FHIR/fhir-test-cases r4/fhirpath (Apache-2.0), pinned"
  note "cases       : 1,191 <test> elements in 103 <group>s, counted by XML parse"
  note "graded crate: frevo-path"
  note "reference   : HAPI 8.12.0 via org.hl7.fhir.r4.test.FHIRPathTests"
  note ""
  $RUNNER -p frevo-path --test fhirpath_oracle
  local rc=$?
  if [ "$rc" -ne 0 ]; then
    note ""
    note "The harness is wired up; no engine is bound to it yet, so every case"
    note "reports did-not-run. The gate against HAPI's own measured rate is"
    note "task 3 of add-fhirpath-and-codegen-r4 and is not wired here."
    return 2
  fi
  return 0
}

run_in_process() { # suite-name crate test-target description
  head2 "$1 (in-process oracle over $4)"
  note "graded crate: $2"
  note ""
  $RUNNER -p "$2" --test "$3"
  local rc=$?
  [ "$rc" -eq 0 ] && return 0
  note ""
  note "Nothing is bound to this oracle yet, so it did not grade anything."
  return 2
}

run_one() {
  case "$1" in
    fhirpath) run_fhirpath ;;
    path-check-warnings)
      run_in_process "path-check-warnings" frevo-path path_check_warnings \
        "the nine check-pass warning message ids"
      ;;
    model-parser-errors)
      run_in_process "model-parser-errors" frevo-model model_parser_errors \
        "hand-built documents, one per parser error condition"
      ;;
    *)
      note "unknown oracle '$1'. Known: fhirpath path-check-warnings model-parser-errors"
      return 2
      ;;
  esac
}

if [ "$#" -eq 0 ]; then
  sed -n '2,25p' "$0"
  exit 2
fi

if [ "$1" = "--all" ]; then
  notrun=0
  below=0
  for o in fhirpath path-check-warnings model-parser-errors; do
    run_one "$o"; r=$?
    [ "$r" -eq 1 ] && below=1
    [ "$r" -eq 2 ] && notrun=$((notrun + 1))
  done
  printf '\n'
  if [ "$notrun" -gt 0 ]; then
    echo "oracles not run: $notrun of 3"
    echo "An oracle that did not run is NOT a pass. The oracle gate is red."
    exit 2
  fi
  exit "$below"
fi

run_one "$1"
exit $?
