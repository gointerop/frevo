#!/usr/bin/env bash
# Activate the enforcement hooks by merging settings.hooks.json into
# .claude/settings.json.
#
# Refuses to do anything unless the hook self-tests pass, because enabling hooks
# that do not actually block is worse than having none: the project would then
# be relying on a control that silently permits everything.
#
# Usage:  bash .claude/hooks/enable.sh [--check]
#           --check   report whether hooks are active; change nothing

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

FRAGMENT=".claude/hooks/settings.hooks.json"
SETTINGS=".claude/settings.json"

PY=""
for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
[ -z "$PY" ] && { echo "FATAL: python3 is required to merge JSON settings." >&2; exit 1; }

if [ "${1:-}" = "--check" ]; then
  if [ -f "$SETTINGS" ] && "$PY" -I -c '
import json,sys
try: s=json.load(open(".claude/settings.json"))
except Exception: sys.exit(1)
sys.exit(0 if s.get("hooks") else 1)
'; then
    echo "hooks: ACTIVE"
    # Backslash-escaped quotes inside an f-string expression are a syntax error
    # in this Python, and nesting them through a single-quoted shell string made
    # it worse. Build the string without any quoting inside the braces.
    "$PY" - <<'PYEOF'
import json
s = json.load(open(".claude/settings.json", encoding="utf-8"))
for ev, entries in sorted(s.get("hooks", {}).items()):
    for e in entries:
        m = e.get("matcher", "*")
        for h in e.get("hooks", []):
            script = h.get("command", "").replace("\\", "/").rstrip('"').split("/")[-1]
            print("  {:<14} {:<30} {}".format(ev, m, script))
PYEOF
    exit 0
  fi
  echo "hooks: NOT active"
  exit 1
fi

[ -f "$FRAGMENT" ] || { echo "FATAL: $FRAGMENT not found." >&2; exit 1; }

echo "Running hook self-tests before enabling..."
if ! bash harness/hook-tests/run.sh >/tmp/frevo-hook-selftest.log 2>&1; then
  echo >&2
  echo "REFUSING TO ENABLE: the hook self-tests fail." >&2
  echo >&2
  tail -30 /tmp/frevo-hook-selftest.log >&2
  echo >&2
  echo "Enabling hooks that do not block would give the project a control it" >&2
  echo "cannot rely on. Fix the hooks first (chore/governance-* branch)." >&2
  exit 1
fi
tail -3 /tmp/frevo-hook-selftest.log

"$PY" -I - "$FRAGMENT" "$SETTINGS" <<'PYEOF'
import json, os, sys

fragment_path, settings_path = sys.argv[1], sys.argv[2]

with open(fragment_path, encoding="utf-8") as f:
    fragment = json.load(f)
hooks = fragment.get("hooks")
if not hooks:
    print("FATAL: fragment has no 'hooks' object", file=sys.stderr)
    sys.exit(1)

settings = {}
if os.path.exists(settings_path):
    with open(settings_path, encoding="utf-8") as f:
        try:
            settings = json.load(f)
        except json.JSONDecodeError as e:
            print(f"FATAL: {settings_path} is not valid JSON: {e}", file=sys.stderr)
            sys.exit(1)

# Replace the hooks block wholesale rather than merging entry by entry: a
# partial merge could leave a stale command pointing at a renamed script, which
# fails open.
settings["hooks"] = hooks

with open(settings_path, "w", encoding="utf-8", newline="\n") as f:
    json.dump(settings, f, indent=2)
    f.write("\n")

n = sum(len(e.get("hooks", [])) for entries in hooks.values() for e in entries)
print(f"wrote {settings_path}: {len(hooks)} event(s), {n} hook command(s)")
PYEOF

echo
echo "Hooks are now ACTIVE. From here on:"
echo "  - crates/ cannot be edited without an APPROVED spec on a change/<id> branch"
echo "  - tests cannot be ignored, deleted or weakened"
echo "  - changes cannot be archived without green gates for the current HEAD"
echo "  - .claude/ itself requires a chore/governance-* branch"
