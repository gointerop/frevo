#!/usr/bin/env bash
# Check that the deviation register and the technical-debt register agree.
#
#   bash harness/scripts/check-registries.sh
#
# Three registers carry the project's deliberate exceptions: docs/deviations.md
# (HAPI disagrees with the specification), docs/tech-debt.md (work deferred with
# a trigger) and docs/known-gaps.md (a case that cannot pass). They are the only
# memory this project has between context resets, so a half-written entry or a
# dangling cross-reference is not untidiness -- it is a decision that has
# silently stopped existing.
#
# What is checked:
#
#   1. Every DEBT id is well formed, unique, and allocated in order from 0001.
#   2. Every DEBT entry has all nine fields, each non-empty. In particular a
#      Trigger, because debt with no trigger is a wish, and a Cost if unpaid,
#      because debt with no cost is not debt.
#   3. Every DEV entry has a Debt field, naming either `none` or DEBT ids that
#      exist.
#   4. Reciprocity: if a DEBT entry's Origin cites DEV-NNNN and that deviation
#      is present in this checkout, the deviation must cite the DEBT id back.
#      "If present" is deliberate -- a deviation and the debt it creates can
#      arrive on different branches, and this check must not couple their merge
#      order. Once both are on main it is exact.
#   5. No dangling DEBT reference anywhere under docs/ or tasks/.
#
# Entry-format examples live inside fenced code blocks, which are stripped
# before parsing: the format sample is documentation, not an entry.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PY=""
for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
[ -z "$PY" ] && { echo "FATAL: python3 required to check the registers" >&2; exit 2; }

exec "$PY" - <<'PYEOF'
import io
import re
import sys
from pathlib import Path

DEVIATIONS = Path("docs/deviations.md")
TECH_DEBT = Path("docs/tech-debt.md")

REQUIRED = [
    "Area", "Origin", "Owed", "Why deferred", "Cost if unpaid",
    "Trigger", "Size", "Status", "Reviewed",
]
STATUSES = {"open", "blocking", "paid", "dropped"}

errors = []
notes = []


def err(msg):
    errors.append(msg)


def read(path):
    if not path.exists():
        err("%s is missing" % path)
        return ""
    return io.open(str(path), encoding="utf-8").read()


def strip_fences(text):
    """Blank out fenced code blocks, keeping line numbers stable."""
    out = []
    inside = False
    for line in text.split("\n"):
        if line.lstrip().startswith("```"):
            inside = not inside
            out.append("")
            continue
        out.append("" if inside else line)
    return "\n".join(out)


def entries(text, prefix):
    """Split into (id, body) pairs for '### PREFIX-NNNN' headings."""
    pattern = re.compile(r"^### (%s-\d{4})\b" % prefix, re.M)
    marks = list(pattern.finditer(text))
    found = []
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(text)
        found.append((m.group(1), text[m.end():end]))
    return found


def fields(body):
    """Map field name -> value for '- **Name:** value' lists, honouring
    continuation lines so a wrapped value is not read as empty."""
    out = {}
    name = None
    for line in body.split("\n"):
        m = re.match(r"\s*- \*\*([^*]+?):\*\*\s*(.*)$", line)
        if m:
            name = m.group(1).strip()
            out[name] = m.group(2).strip()
        elif name and re.match(r"\s{2,}\S", line) and not line.lstrip().startswith("- "):
            out[name] = (out[name] + " " + line.strip()).strip()
        elif not line.strip():
            name = None
    return out


# ---- technical debt ------------------------------------------------------
debt_text = strip_fences(read(TECH_DEBT))
debt = entries(debt_text, "DEBT")
debt_ids = [d for d, _ in debt]

seen = set()
for i, (did, body) in enumerate(debt, start=1):
    if did in seen:
        err("%s: duplicate entry" % did)
    seen.add(did)
    expected = "DEBT-%04d" % i
    if did != expected:
        err("%s: ids must be allocated in order with no gaps; expected %s here"
            % (did, expected))

    f = fields(body)
    for name in REQUIRED:
        if name not in f:
            err("%s: no '%s' field" % (did, name))
        elif not f[name]:
            err("%s: '%s' is empty" % (did, name))

    status = f.get("Status", "").strip().strip("`").split()[0] if f.get("Status") else ""
    if status and status not in STATUSES:
        err("%s: Status '%s' is not one of %s"
            % (did, status, ", ".join(sorted(STATUSES))))

debt_set = set(debt_ids)

# ---- deviations ----------------------------------------------------------
dev_text = strip_fences(read(DEVIATIONS))
dev = entries(dev_text, "DEV")
dev_fields = {}

for did, body in dev:
    f = fields(body)
    dev_fields[did] = f
    if "Debt" not in f:
        err("%s: no 'Debt' field. Every deviation states what it leaves owed, "
            "or 'none'." % did)
        continue
    value = f["Debt"]
    cited = set(re.findall(r"DEBT-\d{4}", value))
    if not cited and value.strip().lower().rstrip(".") != "none":
        err("%s: Debt field is neither 'none' nor a DEBT id: %r" % (did, value))
    for c in sorted(cited):
        if c not in debt_set:
            err("%s: cites %s, which is not an entry in %s" % (did, c, TECH_DEBT))

# ---- reciprocity --------------------------------------------------------
for did, body in debt:
    origin = fields(body).get("Origin", "")
    for dev_id in sorted(set(re.findall(r"DEV-\d{4}", origin))):
        if dev_id not in dev_fields:
            notes.append("%s cites %s, which is not in this checkout of %s "
                         "(expected while they are on different branches)"
                         % (did, dev_id, DEVIATIONS))
            continue
        back = dev_fields[dev_id].get("Debt", "")
        if did not in back:
            err("%s names %s as its origin, but %s does not cite %s back"
                % (did, dev_id, dev_id, did))

# ---- dangling references ------------------------------------------------
for path in sorted(list(Path("docs").rglob("*.md")) + list(Path("tasks").rglob("*.md"))):
    if path == TECH_DEBT:
        continue
    text = io.open(str(path), encoding="utf-8").read()
    for ref in sorted(set(re.findall(r"DEBT-\d{4}", text))):
        if ref not in debt_set:
            err("%s references %s, which is not an entry in %s"
                % (path, ref, TECH_DEBT))

# ---- report -------------------------------------------------------------
print("%s: %d entry(ies)" % (TECH_DEBT, len(debt)))
print("%s: %d entry(ies)" % (DEVIATIONS, len(dev)))
blocking = [d for d, b in debt if fields(b).get("Status", "").startswith("blocking")]
if blocking:
    print("blocking: %s" % ", ".join(blocking))

for n in notes:
    print("note: %s" % n)

if errors:
    print()
    for e in errors:
        print("::error::%s" % e)
    print("\n%d problem(s)." % len(errors))
    sys.exit(1)

print("registers agree.")
PYEOF
