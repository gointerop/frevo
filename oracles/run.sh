#!/usr/bin/env bash
# Oracle suite runner.
#
#   bash oracles/run.sh <suite> [--candidate <base-url>] [--json <out>]
#   bash oracles/run.sh --all
#
# Phase 0 status: every suite is a STUB. A stub verifies the suite is present
# and pinned, counts its cases, and then exits **2 = not implemented**.
#
# Exiting non-zero is the whole point. A runner that returned success before
# anything was implemented would be the cheapest possible way to fake progress,
# and the gatekeeper treats a non-green gate as red -- which is correct, because
# a suite that did not run has told us nothing.
#
# Each runner becomes real as part of the change that implements the capability
# it grades: oracle-builder wires it up before any implementation exists.
#
# Exit codes:
#   0  the suite ran and Frevo's pass rate is at or above the reference's
#   1  the suite ran and Frevo is below the reference rate
#   2  the suite did not run (not implemented, missing content, missing runtime)

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SUITES="fhirpath cql-tests fhir-test-cases r4-example-roundtrip inferno-us-core inferno-bulk-data inferno-smart"

usage() { sed -n '2,25p' "$0"; exit 2; }

note()  { printf '  %s\n' "$*"; }
head2() { printf '\n== %s ==\n' "$*"; }

# Count <test> elements by PARSING the XML, never by matching text.
# tests-fhir-r4.xml wraps the whole testDollarResource group in an XML comment
# whose bodies still contain <test ...> text, so grep reports 104 groups and
# 1193 cases while the real suite is 103 and 1191. A harness that asserts the
# textual figure fails on its first run against the very suite it grades.
count_cases() {
  local f="$1" p
  for p in python3 python py; do
    if command -v "$p" >/dev/null 2>&1; then
      "$p" - "$f" <<'PYEOF'
import sys, xml.etree.ElementTree as ET
try:
    r = ET.parse(sys.argv[1]).getroot()
    print(f"{len(r.findall('.//test'))} in {len(r.findall('.//group'))} groups")
except Exception:
    print("unparseable")
PYEOF
      return
    fi
  done
  echo "unknown (no python available)"
}

run_one() { # suite
  local suite="$1" dir="oracles/$1" rc=2
  head2 "$suite"

  case "$suite" in
    # ------------------------------------------------------------------ vendored
    fhirpath)
      local f="$dir/tests-fhir-r4.xml"
      if [ ! -f "$f" ]; then
        note "MISSING: $f -- run: bash oracles/fetch.sh"
        return 2
      fi
      note "source      : FHIR/fhir-test-cases r4/fhirpath (Apache-2.0), pinned"
      note "cases       : $(count_cases "$f")"
      note "graded crate: frevo-path"
      note "reference   : HAPI 8.12.0 via org.hl7.fhir.core FHIRPathEngine"
      note ""
      note "NOT IMPLEMENTED: frevo-path has no engine yet, so there is nothing"
      note "to grade. This runner is wired up by the change"
      note "add-fhirpath-and-codegen-r4, before the engine is written."
      rc=2
      ;;

    cql-tests)
      if [ ! -d "$dir/tests" ]; then
        note "MISSING: $dir/tests -- run: bash oracles/fetch.sh"
        return 2
      fi
      note "source      : cqframework/cql-tests (Apache-2.0), pinned"
      note "cases       : $(find "$dir/tests" -name '*.xml' 2>/dev/null | wc -l | tr -d ' ') test files"
      note "graded crate: frevo-cql"
      note "reference   : the cqframework reference engine, which does NOT pass"
      note "              every case itself -- the gate is parity with it, not"
      note "              an absolute rate."
      note ""
      note "NOT IMPLEMENTED: no CQL engine yet (phase 7)."
      rc=2
      ;;

    # ------------------------------------------------------------------- fetched
    fhir-test-cases)
      if [ ! -d "$dir/fetched/.git" ]; then
        note "NOT FETCHED (286 MB, not committed) -- run: bash oracles/fetch.sh"
        return 2
      fi
      note "source      : FHIR/fhir-test-cases (Apache-2.0), pinned, fetched"
      note "graded crate: frevo-validator"
      note "reference   : HAPI 8.12.0 via org.hl7.fhir.validation"
      note ""
      note "NOT IMPLEMENTED: no validator yet (phase 4)."
      note "Note: cases requiring SNOMED CT or LOINC content will report"
      note "skipped-for-licence, which is NOT a pass."
      rc=2
      ;;

    r4-example-roundtrip)
      if [ ! -f "$dir/fetched/.pin" ]; then
        note "NOT FETCHED (19.6 MB package, never committed)"
        note "run: bash oracles/fetch.sh"
        return 2
      fi
      local pkg="$dir/fetched/package"
      note "source      : hl7.fhir.r4.examples#4.0.1 from packages.fhir.org,"
      note "              pinned by SHA-256, fetched, never vendored"
      note "licence     : CC0-1.0, declared in the package manifest. Fetched"
      note "              rather than vendored on size alone; see oracles/PINS.md"
      note "resources   : $(find "$pkg" -name '*.json' 2>/dev/null | wc -l | tr -d ' ') JSON files"
      note "graded crate: frevo-model"
      note "reference   : a parse and re-serialise round trip must return the"
      note "              input bytes unchanged. This is the gate on performance"
      note "              principle 1 -- a read that returns a resource"
      note "              unchanged must not parse or re-serialize it -- so it"
      note "              grades correctness and the storage design together."
      note ""
      note "NOT IMPLEMENTED: frevo-model has no codec yet. This runner is wired"
      note "up by the change add-fhirpath-and-codegen-r4."
      note "Note: examples embedding SNOMED CT or LOINC codes round-trip as"
      note "bytes without any terminology lookup, so none is skipped for"
      note "licence here. Cases needing an expansion live in fhir-test-cases."
      rc=2
      ;;

    inferno-us-core|inferno-bulk-data|inferno-smart)
      if [ ! -d "$dir/fetched/.git" ]; then
        note "NOT FETCHED -- run: bash oracles/fetch.sh"
        return 2
      fi
      note "source      : inferno-framework (Apache-2.0), pinned, fetched"
      note "needs       : Ruby, the Inferno runtime, and a REACHABLE candidate"
      note "              server with a published base URL"
      note ""
      note "NOT IMPLEMENTED. These kits drive a live server over HTTP, so they"
      note "run as a scheduled job against a deployed instance rather than as a"
      note "per-pull-request gate -- a kit run takes far longer than a CI job"
      note "should, and needs a server that outlives the job."
      rc=2
      ;;

    *)
      note "unknown suite '$suite'. Known: $SUITES"
      return 2
      ;;
  esac

  return "$rc"
}

if [ "$#" -eq 0 ]; then usage; fi

if [ "${1:-}" = "--all" ]; then
  overall=0
  notrun=0
  for s in $SUITES; do
    run_one "$s"; r=$?
    [ "$r" -eq 1 ] && overall=1
    [ "$r" -eq 2 ] && notrun=$((notrun + 1))
  done
  printf '\n'
  echo "suites not run: $notrun of $(echo $SUITES | wc -w | tr -d ' ')"
  if [ "$notrun" -gt 0 ]; then
    echo "A suite that did not run is NOT a pass. The oracle gate is red."
    exit 2
  fi
  exit "$overall"
fi

run_one "$1"
exit $?
