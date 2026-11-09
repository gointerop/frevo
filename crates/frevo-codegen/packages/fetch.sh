#!/usr/bin/env bash
# Fetch and verify the FHIR definition package frevo-codegen generates from.
#
#   bash crates/frevo-codegen/packages/fetch.sh            # fetch what is missing
#   bash crates/frevo-codegen/packages/fetch.sh --verify   # re-verify what is there
#   bash crates/frevo-codegen/packages/fetch.sh --list     # print the pin
#
# The pin lives in PINS.md and in crates/frevo-codegen/src/pin.rs, and the two
# must agree.
#
# Two digests are recorded, and they are checked in two different places.
#
#   archive SHA-256   checked here, before anything is extracted, so a
#                     substituted or truncated download never reaches the disk
#                     as definitions. Reproducible with `curl | sha256sum`.
#   content SHA-256   checked by the generator, on every run, over the
#                     extracted tree -- which is what the generator actually
#                     reads (codegen.inputs.002). `frevo-codegen verify`
#                     exposes the same check on its own.
#
# The content digest is deliberately not recomputed here: duplicating its
# canonical manifest in shell would mean two definitions of the same digest,
# and the one that matters is the one the generator enforces.
#
# Nothing here is ever modified in place, and a failure is loud. A package that
# does not match its pin is not a package to generate from.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PACKAGE="hl7.fhir.r4.core"
VERSION="4.0.1"
PRIMARY="https://packages.fhir.org/${PACKAGE}/${VERSION}"
MIRROR="https://packages.simplifier.net/${PACKAGE}/${VERSION}"
ARCHIVE_SHA256="ebd7731df7d36b5b7d39d5fb6c9d77b44bb7fe5742f1a2e87f164738c3289d44"
CONTENT_SHA256="8af9fe30008c2a02c351728c198759141b05482f91dee63475777aa23f06354c"
EXPECTED_FILES=4583

DEST="fetched/${PACKAGE}-${VERSION}"
ARCHIVE="fetched/${PACKAGE}-${VERSION}.tgz"
DEFINITIONS="${DEST}/package"

MODE="${1:-fetch}"

if [ "$MODE" = "--list" ]; then
  printf '%-18s %s\n' package "${PACKAGE}#${VERSION}"
  printf '%-18s %s\n' registry "$PRIMARY"
  printf '%-18s %s\n' mirror "$MIRROR"
  printf '%-18s %s\n' archive-sha256 "$ARCHIVE_SHA256"
  printf '%-18s %s\n' content-sha256 "$CONTENT_SHA256"
  printf '%-18s %s\n' definitions "crates/frevo-codegen/packages/${DEFINITIONS}"
  exit 0
fi

fail() { echo "[FAIL] $*" >&2; exit 1; }

sha256_of() { # file
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    fail "neither sha256sum nor shasum is available; cannot verify the pin"
  fi
}

check_layout() {
  [ -f "${DEFINITIONS}/package.json" ] \
    || fail "${DEFINITIONS}/package.json is missing; the archive layout is not what the pin expects"
  grep -q "\"version\": *\"${VERSION}\"" "${DEFINITIONS}/package.json" \
    || fail "${DEFINITIONS}/package.json does not declare version ${VERSION}"
  local files
  files="$(find "$DEFINITIONS" -type f | wc -l | tr -d ' ')"
  [ "$files" = "$EXPECTED_FILES" ] \
    || fail "${DEFINITIONS} holds ${files} files, not the pinned ${EXPECTED_FILES}"
}

report_ok() {
  echo "[ok]    ${PACKAGE}#${VERSION} archive digest and layout verified"
  echo "        definitions:    crates/frevo-codegen/packages/${DEFINITIONS}"
  echo "        content digest: ${CONTENT_SHA256}, checked by the generator on every run"
}

if [ "$MODE" = "--verify" ]; then
  [ -f "$ARCHIVE" ] || fail "${ARCHIVE} is not present; run this script with no arguments"
  got="$(sha256_of "$ARCHIVE")"
  [ "$got" = "$ARCHIVE_SHA256" ] \
    || fail "${ARCHIVE}: archive SHA-256 ${got} != pinned ${ARCHIVE_SHA256}"
  check_layout
  report_ok
  exit 0
fi

if [ "$MODE" != "fetch" ]; then
  fail "unknown mode ${MODE}; use --verify or --list"
fi

mkdir -p fetched

if [ ! -f "$ARCHIVE" ] || [ "$(sha256_of "$ARCHIVE")" != "$ARCHIVE_SHA256" ]; then
  rm -f "$ARCHIVE"
  for url in "$PRIMARY" "$MIRROR"; do
    echo "[fetch] $url"
    if curl -fsSL --retry 3 --max-time 600 -o "$ARCHIVE" "$url"; then
      got="$(sha256_of "$ARCHIVE")"
      if [ "$got" = "$ARCHIVE_SHA256" ]; then
        break
      fi
      echo "[warn]  $url: archive SHA-256 ${got} != pinned ${ARCHIVE_SHA256}" >&2
    fi
    rm -f "$ARCHIVE"
  done
fi

[ -f "$ARCHIVE" ] || fail "could not fetch ${PACKAGE}#${VERSION} from either registry"
got="$(sha256_of "$ARCHIVE")"
[ "$got" = "$ARCHIVE_SHA256" ] \
  || fail "${ARCHIVE}: archive SHA-256 ${got} != pinned ${ARCHIVE_SHA256}"

if [ ! -f "${DEFINITIONS}/package.json" ]; then
  rm -rf "$DEST"
  mkdir -p "$DEST"
  tar -xzf "$ARCHIVE" -C "$DEST" || fail "extracting ${ARCHIVE}"
fi

check_layout
report_ok
