#!/usr/bin/env bash
# Fetch the pinned upstream reference sources listed in PINS.md.
#
# These are read-only: never modified, never committed, never compiled into the
# product. The script is idempotent and verifies the commit SHA of every tree,
# so a silently re-pinned or half-finished clone fails loudly instead of quietly
# changing what "correct" means.
set -uo pipefail
cd "$(dirname "$0")"

# name <TAB> url <TAB> ref <TAB> expected commit SHA  (see PINS.md)
PINS=$(cat <<'TBL'
hapi-fhir-jpaserver-starter	https://github.com/hapifhir/hapi-fhir-jpaserver-starter.git	image/v8.12.0-2	377e6eba316920082e6a90551b488e4599a71eb8
hapi-fhir	https://github.com/hapifhir/hapi-fhir.git	v8.12.0	7ab4c7c7c49281496faa1b3c2498a652457669ae
org.hl7.fhir.core	https://github.com/hapifhir/org.hl7.fhir.core.git	6.9.12	73c432b5f43ddbce979978a760ec4aa81e929094
clinical_quality_language	https://github.com/cqframework/clinical_quality_language.git	v5.4.0	c41c21f5616a0da82d49e32a9cc3a5b3ca299bc2
clinical-reasoning	https://github.com/cqframework/clinical-reasoning.git	v4.13.0	ccba474a2fba73329353b41d270f9d0001d6cb52
TBL
)

# core.longpaths is required on Windows: several upstream test-resource paths
# exceed MAX_PATH. Set per-invocation so no global git config is touched.
GIT="git -c core.longpaths=true -c advice.detachedHead=false"

rc=0
while IFS=$'\t' read -r name url ref want; do
  [ -z "${name:-}" ] && continue

  if [ ! -d "$name/.git" ]; then
    echo "[clone] $name @ $ref"
    rm -rf "$name"
    $GIT clone --depth 1 --branch "$ref" --single-branch "$url" "$name" >/dev/null 2>&1
  fi

  if [ ! -d "$name/.git" ]; then
    echo "[FAIL] $name: clone produced no repository"; rc=1; continue
  fi

  got=$($GIT -C "$name" rev-parse HEAD 2>/dev/null)
  if [ "$got" != "$want" ]; then
    echo "[FAIL] $name: HEAD $got != pinned $want (PINS.md and the tree disagree)"
    rc=1; continue
  fi

  # A clone can leave an unpopulated or partial working tree without failing.
  # Force the checkout, then require a clean tree and a non-empty index.
  dirty=$($GIT -C "$name" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
  if [ "$dirty" != "0" ]; then
    echo "[fix]   $name: working tree incomplete ($dirty paths), re-checking out"
    $GIT -C "$name" checkout -f HEAD >/dev/null 2>&1
    dirty=$($GIT -C "$name" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
  fi

  files=$($GIT -C "$name" ls-files 2>/dev/null | wc -l | tr -d ' ')
  if [ "$dirty" != "0" ] || [ "${files:-0}" -lt 100 ]; then
    echo "[FAIL] $name: tree not usable (dirty=$dirty files=$files)"; rc=1; continue
  fi

  echo "[ok]    $name $got  ($files files)"
done <<< "$PINS"

if [ "$rc" -ne 0 ]; then
  echo "---CLONE-RC:$rc--- reference tree is NOT trustworthy; do not mine it for specs"
else
  echo "---CLONE-RC:0--- all pins verified against PINS.md"
fi
exit "$rc"
