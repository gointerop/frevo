#!/usr/bin/env bash
# H6 -- PostToolUse on Edit|Write|MultiEdit
#
# Returns problems immediately, while the agent still has the change in mind:
#   - openspec/changes/<id>/**  -> strict validation errors
#   - crates/<crate>/**         -> cargo fmt applied, then clippy -D warnings
#
# Exit 2 returns the message to the agent. This hook is advisory in effect --
# the edit has already happened -- but surfacing the failure now is far cheaper
# than discovering it in a gate run.

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
use_input_root "$INPUT"
cd "$REPO_ROOT" 2>/dev/null || exit 0

# Bound the work: these run after every edit, so they must stay fast.
TIMEOUT_BIN="$(command -v timeout || true)"
run_bounded() { # seconds, command...
  local secs="$1"; shift
  if [ -n "$TIMEOUT_BIN" ]; then "$TIMEOUT_BIN" "$secs" "$@"; else "$@"; fi
}

problems=""
seen_crates=""
validated=""

while IFS= read -r p; do
  [ -z "$p" ] && continue

  # --- OpenSpec artifacts ------------------------------------------------
  case "$p" in
    openspec/changes/*)
      id="$(printf '%s' "$p" | sed -nE 's|^openspec/changes/([^/]+)/.*|\1|p')"
      [ -z "$id" ] && continue
      [ "$id" = "archive" ] && continue
      case " $validated " in *" $id "*) continue ;; esac
      validated="$validated $id"

      os="$(openspec_bin)"
      if [ -z "$os" ]; then
        problems="$problems
openspec CLI not on PATH, so '$id' could not be validated. Install it with:
  npm i -g @fission-ai/openspec@latest"
        continue
      fi
      if ! out="$(run_bounded 60 "$os" validate "$id" --strict --no-interactive 2>&1)"; then
        problems="$problems
--- openspec validate $id --strict ---
$out"
      fi
      ;;
  esac

  # --- Rust crates -------------------------------------------------------
  case "$p" in
    crates/*)
      crate="$(printf '%s' "$p" | sed -nE 's|^crates/([^/]+)/.*|\1|p')"
      [ -z "$crate" ] && continue
      case " $seen_crates " in *" $crate "*) continue ;; esac
      seen_crates="$seen_crates $crate"
      [ -f "crates/$crate/Cargo.toml" ] || continue

      # Format first: a formatting-only clippy failure is noise.
      run_bounded 60 cargo fmt -p "$crate" >/dev/null 2>&1

      if ! out="$(run_bounded 180 cargo clippy -p "$crate" --all-targets -- -D warnings 2>&1)"; then
        # Keep the diagnostics, drop the progress chatter.
        detail="$(printf '%s' "$out" | grep -vE '^\s*(Compiling|Checking|Finished|Updating|Downloaded|Blocking)' | tail -40)"
        problems="$problems
--- cargo clippy -p $crate -- -D warnings ---
$detail"
      fi
      ;;
  esac
done <<< "$(target_paths "$INPUT")"

if [ -n "$problems" ]; then
  block <<EOF
The edit was applied, but it does not currently pass the gates. Fix these now --
they are gate conditions (clippy is run with -D warnings, and a change cannot be
archived while strict validation fails), so they will block the change later
anyway.
$problems
EOF
fi

allow
