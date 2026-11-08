#!/usr/bin/env bash
# Fetch and verify the oracle suites pinned in oracles/PINS.md.
#
#   bash oracles/fetch.sh             # fetch what is not vendored
#   bash oracles/fetch.sh --verify    # verify vendored content against its pin
#   bash oracles/fetch.sh --list      # show the pins and dispositions
#
# Two dispositions:
#
#   vendored  the content is committed, because it is small AND its licence
#             permits redistribution. `--verify` re-downloads it and compares,
#             so a silently edited oracle case is caught.
#   fetched   the content is not committed -- too large, or it needs a runtime
#             we do not ship -- and is cloned at the pinned commit on demand
#             into oracles/<suite>/fetched/ (git-ignored).
#
# Nothing here is ever modified in place. Making a failing oracle case pass by
# editing the case is the one thing this project must never do; the remedy is
# docs/known-gaps.md.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# suite | disposition | repo | commit | path-within-repo (vendored only)
PINS=$(cat <<'TBL'
fhirpath|vendored|https://github.com/FHIR/fhir-test-cases.git|c95adbee099c875de97d25ed7616aae7d66dd85b|r4/fhirpath/tests-fhir-r4.xml
cql-tests|vendored|https://github.com/cqframework/cql-tests.git|5747d2b72f29d733fbc114383c4a15a30999ffaf|tests
fhir-test-cases|fetched|https://github.com/FHIR/fhir-test-cases.git|c95adbee099c875de97d25ed7616aae7d66dd85b|
inferno-us-core|fetched|https://github.com/inferno-framework/us-core-test-kit.git|87ddb5358b73a851fc64b765963dbccab41e50a2|
inferno-bulk-data|fetched|https://github.com/inferno-framework/bulk-data-test-kit.git|7428430c05fa399ae3e84d411e6b7e270f14fb5e|
inferno-smart|fetched|https://github.com/inferno-framework/smart-app-launch-test-kit.git|980e54e4ed632b28267d797013399a8588772174|
TBL
)

GIT="git -c advice.detachedHead=false -c core.longpaths=true"
MODE="${1:-fetch}"

if [ "$MODE" = "--list" ]; then
  printf '%-20s %-10s %-12s %s\n' SUITE DISPOSITION COMMIT PATH
  while IFS='|' read -r suite disp repo sha path; do
    [ -z "${suite:-}" ] && continue
    printf '%-20s %-10s %-12s %s\n' "$suite" "$disp" "${sha:0:10}" "${path:-<whole repo>}"
  done <<< "$PINS"
  exit 0
fi

rc=0

# Clone one repo at a pinned commit into a scratch directory.
clone_at() { # repo sha dest
  local repo="$1" sha="$2" dest="$3"
  rm -rf "$dest"
  mkdir -p "$dest"
  (
    cd "$dest" || exit 1
    $GIT init -q
    $GIT remote add origin "$repo"
    # Fetch just the one commit where the server allows it; fall back to a
    # full fetch otherwise.
    if ! $GIT fetch -q --depth 1 origin "$sha" 2>/dev/null; then
      $GIT fetch -q origin 2>/dev/null || exit 1
    fi
    $GIT checkout -q "$sha" 2>/dev/null || exit 1
  ) || return 1
  [ -d "$dest/.git" ] || return 1
  local got
  got="$($GIT -C "$dest" rev-parse HEAD 2>/dev/null)"
  [ "$got" = "$sha" ] || { echo "[FAIL] $dest: HEAD $got != pinned $sha"; return 1; }
  return 0
}

while IFS='|' read -r suite disp repo sha path; do
  [ -z "${suite:-}" ] && continue
  dir="oracles/$suite"
  mkdir -p "$dir"

  case "$disp" in
    vendored)
      target="$dir/$(basename "${path:-content}")"
      if [ "$MODE" = "--verify" ] || [ ! -e "$target" ]; then
        tmp="$(mktemp -d)"
        if ! clone_at "$repo" "$sha" "$tmp/src"; then
          echo "[FAIL] $suite: could not clone $repo at ${sha:0:10}"
          rm -rf "$tmp"; rc=1; continue
        fi
        src="$tmp/src/$path"
        if [ ! -e "$src" ]; then
          echo "[FAIL] $suite: $path not found at ${sha:0:10}"
          rm -rf "$tmp"; rc=1; continue
        fi
        if [ "$MODE" = "--verify" ] && [ -e "$target" ]; then
          if diff -r -q "$src" "$target" >/dev/null 2>&1; then
            echo "[ok]    $suite vendored content matches ${sha:0:10}"
          else
            echo "[FAIL]  $suite vendored content DIFFERS from the pinned source."
            echo "        An oracle case must never be edited locally. If the"
            echo "        pin should move, do it on a chore/oracles-* branch."
            diff -r -q "$src" "$target" 2>&1 | head -10
            rc=1
          fi
        else
          cp -r "$src" "$target"
          echo "[vendor] $suite <- $path @ ${sha:0:10}"
        fi
        rm -rf "$tmp"
      else
        echo "[skip]  $suite already vendored (use --verify to check it)"
      fi
      ;;

    fetched)
      dest="$dir/fetched"
      if [ "$MODE" = "--verify" ]; then
        if [ -d "$dest/.git" ]; then
          got="$($GIT -C "$dest" rev-parse HEAD 2>/dev/null)"
          if [ "$got" = "$sha" ]; then
            echo "[ok]    $suite fetched at ${sha:0:10}"
          else
            echo "[FAIL]  $suite fetched tree is at $got, pinned ${sha:0:10}"
            rc=1
          fi
        else
          echo "[miss]  $suite not fetched (run without --verify)"
        fi
        continue
      fi
      if [ -d "$dest/.git" ] && [ "$($GIT -C "$dest" rev-parse HEAD 2>/dev/null)" = "$sha" ]; then
        echo "[skip]  $suite already fetched at ${sha:0:10}"
        continue
      fi
      echo "[fetch] $suite <- $repo @ ${sha:0:10}"
      if clone_at "$repo" "$sha" "$dest"; then
        echo "[ok]    $suite fetched"
      else
        echo "[FAIL]  $suite fetch failed"
        rc=1
      fi
      ;;
  esac
done <<< "$PINS"

echo
if [ "$rc" -ne 0 ]; then
  echo "---ORACLES-RC:$rc--- oracle content is NOT trustworthy; gates using it are not valid"
else
  echo "---ORACLES-RC:0--- all oracle pins verified"
fi
exit "$rc"
