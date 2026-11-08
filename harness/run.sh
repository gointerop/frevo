#!/usr/bin/env bash
# Thin wrapper around harness/run.py so gate scripts have one stable entry point.
#
#   bash harness/run.sh --candidate http://localhost:8080/fhir
#   bash harness/run.sh --self-test
#
# Defaults: baseline is the harness compose stack's HAPI on :8090, reports land
# in harness/out/ (git-ignored).
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PY=""
for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
[ -z "$PY" ] && { echo "FATAL: python3 required" >&2; exit 2; }

mkdir -p harness/out
exec "$PY" harness/run.py \
  --baseline "${FREVO_BASELINE:-http://localhost:8090/fhir}" \
  --report harness/out/differential.md \
  --junit  harness/out/differential.xml \
  "$@"
