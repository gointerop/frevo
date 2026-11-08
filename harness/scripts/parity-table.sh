#!/usr/bin/env bash
# Regenerate the parity table in README.md from docs/inventory.md.
#
# The parity percentage is computed from the inventory's Status column rather
# than maintained by hand, so it cannot drift optimistic. A row counts as done
# only when its status names scenario ids, which the coverage checker resolves
# to passing tests.
#
# Usage:  bash harness/scripts/parity-table.sh [--check]
#           --check  exit 1 if README.md is out of date; change nothing
#
# Note: the Python below reads and writes the files itself. An earlier version
# captured Python's stdout in a shell variable and redirected it into
# README.md; on Windows the default console encoding is cp1252, the README
# contains non-Latin-1 characters, and the resulting UnicodeEncodeError left
# the shell writing a truncated capture over the file -- emptying it. Never
# round-trip file content through a pipe when the tool can write it directly.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PY=""
for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
[ -z "$PY" ] && { echo "FATAL: python3 required" >&2; exit 1; }

MODE="${1:-write}"
exec "$PY" -I - "$MODE" <<'PYEOF'
import re
import sys
from pathlib import Path

MODE = sys.argv[1] if len(sys.argv) > 1 else "write"
INVENTORY = Path("docs/inventory.md")
README = Path("README.md")
BEGIN = "<!-- PARITY-TABLE:BEGIN -->"
END = "<!-- PARITY-TABLE:END -->"

if not INVENTORY.exists():
    print(f"FATAL: {INVENTORY} not found. Run harness/scripts/gen-inventory.py first.",
          file=sys.stderr)
    sys.exit(1)

text = INVENTORY.read_text(encoding="utf-8")

SECTIONS = [
    "Configuration keys",
    "Operations",
    "REST interactions",
    "Search features",
    "Interceptor pointcuts",
]

by_name = {}
for part in re.split(r"^## ", text, flags=re.M):
    by_name[part.split("\n", 1)[0].strip()] = part

SCENARIO = re.compile(r"\b[a-z0-9_]+\.[a-z0-9_]+\.[0-9]+\b")


def tally(body: str) -> tuple[int, int]:
    done = total = 0
    for line in body.splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 2:
            continue
        status = cells[-1]
        if status == "Status" or set(status) <= {"-", ":"} or not status:
            continue
        total += 1
        if SCENARIO.search(status):
            done += 1
    return done, total


def bar(pct: int) -> str:
    filled = pct // 10
    return "#" * filled + "." * (10 - filled)


rows, gd, gt = [], 0, 0
for label in SECTIONS:
    d, t = tally(by_name.get(label, ""))
    gd += d
    gt += t
    pct = (100 * d // t) if t else 0
    rows.append(f"| {label} | {d} / {t} | `{bar(pct)}` {pct}% |")

pct = (100 * gd // gt) if gt else 0
table = "\n".join([
    "| Area | Done | Parity |",
    "|---|---|---|",
    *rows,
    f"| **Total** | **{gd} / {gt}** | `{bar(pct)}` **{pct}%** |",
    "",
    "_Generated from [`docs/inventory.md`](docs/inventory.md) by"
    " `harness/scripts/parity-table.sh`. A row counts as done only when its status"
    " names scenario ids that resolve to passing tests._",
])

readme = README.read_text(encoding="utf-8")
i, j = readme.find(BEGIN), readme.find(END)
if i < 0 or j < 0:
    print(f"FATAL: {README} has no {BEGIN} / {END} markers.", file=sys.stderr)
    sys.exit(1)

updated = readme[: i + len(BEGIN)] + "\n" + table + "\n" + readme[j:]

if MODE == "--check":
    if updated == readme:
        print("README parity table is up to date.")
        sys.exit(0)
    print("README parity table is STALE. Run: bash harness/scripts/parity-table.sh",
          file=sys.stderr)
    sys.exit(1)

# Refuse to write something obviously wrong: the table replacement must not
# shrink the file, and the trademark notice must survive.
if len(updated) < len(readme) * 0.5 or "registered trademarks" not in updated:
    print("FATAL: refusing to write a README that lost content.", file=sys.stderr)
    sys.exit(1)

README.write_text(updated, encoding="utf-8", newline="\n")
print(f"updated {README} ({gd}/{gt} rows at parity, {pct}%)")
print(table)
PYEOF
