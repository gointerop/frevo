#!/usr/bin/env bash
# Generate and load a Synthea R4 dataset for the differential harness and the
# benchmarks.
#
#   bash harness/scripts/synthea.sh generate small          # 100 patients
#   bash harness/scripts/synthea.sh generate medium         # 1000 patients
#   bash harness/scripts/synthea.sh load small http://localhost:8090/fhir
#   bash harness/scripts/synthea.sh stats small
#
# Synthea output is synthetic by construction, so it carries no PHI and is safe
# for a public repository -- but it is large, so harness/data/ is git-ignored
# and datasets are regenerated rather than committed.
#
# Determinism matters here: a benchmark baseline is only comparable if the
# dataset is identical. The Synthea jar is pinned by release AND verified by
# SHA-256, and generation uses a fixed seed. "master-branch-latest" is
# deliberately not used: it moves, which would silently change every recorded
# baseline.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

SYNTHEA_VERSION="v4.0.0"
SYNTHEA_SHA256="ed43c20ad40ba5c3bc724503a5af032715fe3c491620b766148e7c2361e6ecc1"
SYNTHEA_URL="https://github.com/synthetichealth/synthea/releases/download/${SYNTHEA_VERSION}/synthea-with-dependencies.jar"

DATA="harness/data"
JAR="$DATA/synthea-${SYNTHEA_VERSION}.jar"

# Fixed seeds, so a regenerated dataset is byte-identical.
SEED=20261108
CLINICIAN_SEED=20261108

size_to_count() {
  case "$1" in
    tiny)   echo 10 ;;
    small)  echo 100 ;;
    medium) echo 1000 ;;
    large)  echo 10000 ;;
    *)      echo "" ;;
  esac
}

# Sizing note, from measurement rather than guesswork.
#
# "medium" was 10000 and is now 1000. Generating 10000 patients produced 31 GB
# of transaction bundles on its own, and loading them into HAPI's PostgreSQL
# needs several times that again -- more than a typical development machine
# has spare. A baseline that cannot be reproduced is not a baseline.
#
# 1000 patients is roughly 2.5 GB of bundles and about 400k resources, which is
# past the point where an index stops fitting comfortably in cache -- the thing
# the small set cannot show -- while still running on a laptop.
#
# "large" keeps 10000 for a machine or CI runner with the disk to host it.

die() { printf 'FATAL: %s\n' "$*" >&2; exit 1; }

need_java() {
  command -v java >/dev/null 2>&1 || die "java is required to run Synthea."
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else
    for p in python3 python py; do
      command -v "$p" >/dev/null 2>&1 && {
        "$p" -I -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$1"
        return
      }
    done
    die "no sha256 tool available"
  fi
}

fetch_jar() {
  mkdir -p "$DATA"
  if [ -f "$JAR" ]; then
    got="$(sha256_of "$JAR")"
    if [ "$got" = "$SYNTHEA_SHA256" ]; then
      echo "[ok] synthea ${SYNTHEA_VERSION} present and verified"
      return 0
    fi
    echo "[warn] $JAR has unexpected SHA-256 ($got); re-downloading"
    rm -f "$JAR"
  fi
  echo "[fetch] $SYNTHEA_URL (about 200 MB)"
  curl -fsSL --retry 3 -o "$JAR.part" "$SYNTHEA_URL" || die "download failed"
  got="$(sha256_of "$JAR.part")"
  if [ "$got" != "$SYNTHEA_SHA256" ]; then
    rm -f "$JAR.part"
    die "SHA-256 mismatch for the Synthea jar.
  expected $SYNTHEA_SHA256
  got      $got
A dataset generated from an unverified jar would silently invalidate every
recorded benchmark baseline, so this is fatal rather than a warning."
  fi
  mv "$JAR.part" "$JAR"
  echo "[ok] verified ${SYNTHEA_VERSION}"
}

cmd_generate() {
  local size="${1:-}" count out
  count="$(size_to_count "$size")"
  [ -z "$count" ] && die "unknown size '$size' (tiny|small|medium)"
  need_java
  fetch_jar
  out="$DATA/synthea-$size"

  if [ -d "$out/fhir" ] && [ "$(find "$out/fhir" -name '*.json' 2>/dev/null | head -1)" ]; then
    n=$(find "$out/fhir" -name '*.json' | wc -l | tr -d ' ')
    echo "[skip] $out already has $n bundle file(s). Delete it to regenerate."
    return 0
  fi

  mkdir -p "$out"
  echo "[generate] $count patients -> $out (seed $SEED)"
  # -p count, fixed seeds, R4 transaction bundles, no CSV/CCDA output.
  java -jar "$JAR" \
    -p "$count" \
    -s "$SEED" \
    -cs "$CLINICIAN_SEED" \
    --exporter.baseDirectory "$out" \
    --exporter.fhir.export true \
    --exporter.fhir.transaction_bundle true \
    --exporter.fhir.use_us_core_ig false \
    --exporter.hospital.fhir.export true \
    --exporter.practitioner.fhir.export true \
    --exporter.csv.export false \
    --exporter.ccda.export false \
    --exporter.text.export false \
    --generate.append_numbers_to_person_names false \
    Massachusetts 2>&1 | tail -15

  n=$(find "$out/fhir" -name '*.json' 2>/dev/null | wc -l | tr -d ' ')
  [ "${n:-0}" -eq 0 ] && die "Synthea produced no FHIR output in $out"
  echo "[ok] $n bundle file(s) in $out/fhir"
  du -sh "$out" 2>/dev/null
}

cmd_load() {
  local size="${1:-}" base="${2:-}" dir
  [ -z "$size" ] && die "usage: synthea.sh load <size> <base-url> [concurrency]"
  [ -z "$base" ] && die "usage: synthea.sh load <size> <base-url> [concurrency]"
  dir="$DATA/synthea-$size/fhir"
  [ -d "$dir" ] || die "$dir not found. Run: synthea.sh generate $size"

  local conc="${3:-8}"
  PY=""
  for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
  [ -z "$PY" ] && die "python3 required"

  echo "[load] $dir -> $base (concurrency $conc)"
  "$PY" -I harness/scripts/load_bundles.py --dir "$dir" --base "$base" --concurrency "$conc"
}

cmd_stats() {
  local size="${1:-}" dir
  dir="$DATA/synthea-$size/fhir"
  [ -d "$dir" ] || die "$dir not found"
  echo "files : $(find "$dir" -name '*.json' | wc -l | tr -d ' ')"
  echo "size  : $(du -sh "$dir" 2>/dev/null | awk '{print $1}')"
  echo "patients (by filename): $(find "$dir" -name '*.json' ! -name 'hospitalInformation*' ! -name 'practitionerInformation*' | wc -l | tr -d ' ')"
}

case "${1:-}" in
  generate) shift; cmd_generate "$@" ;;
  load)     shift; cmd_load "$@" ;;
  stats)    shift; cmd_stats "$@" ;;
  *)
    sed -n '2,20p' "$0"
    exit 2
    ;;
esac
