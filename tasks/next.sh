#!/usr/bin/env bash
# Query the work queue in tasks/graph.yaml.
#
#   tasks/next.sh                 what is ready now
#   tasks/next.sh --limit 3       the next three ready nodes
#   tasks/next.sh --blocked       what is waiting, and on what
#   tasks/next.sh --all           every node with its status
#   tasks/next.sh --phase 3       restrict to one phase
#   tasks/next.sh --validate      internal consistency; CI runs this
#
# "Ready" means: status is pending, and every dependency is done.
#
# The SessionStart hook calls this, so a reset context recovers its place from
# the repository rather than from the conversation. That makes a bug in here
# misleading to every session that starts, which is why --validate exists and
# runs in CI.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PY=""
for p in python3 python py; do command -v "$p" >/dev/null 2>&1 && { PY="$p"; break; }; done
[ -z "$PY" ] && { echo "FATAL: python3 required to read tasks/graph.yaml" >&2; exit 2; }

exec "$PY" - "$@" <<'PYEOF'
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print("FATAL: PyYAML required.\n  python -m pip install pyyaml", file=sys.stderr)
    sys.exit(2)

GRAPH = Path("tasks/graph.yaml")
if not GRAPH.exists():
    print("FATAL: tasks/graph.yaml not found", file=sys.stderr)
    sys.exit(2)

doc = yaml.safe_load(GRAPH.read_text(encoding="utf-8")) or {}
nodes = doc.get("nodes") or []
phases = doc.get("phases") or {}
by_id = {n["id"]: n for n in nodes}

# ---- args ----------------------------------------------------------------
args = sys.argv[1:]
limit = 0
phase = None
mode = "ready"
i = 0
while i < len(args):
    a = args[i]
    if a == "--limit":
        limit = int(args[i + 1]); i += 2; continue
    if a == "--phase":
        phase = int(args[i + 1]); i += 2; continue
    if a in ("--blocked", "--all", "--validate", "--ready"):
        mode = a.lstrip("-"); i += 1; continue
    if a in ("-h", "--help"):
        print(__doc__ or "see tasks/next.sh"); sys.exit(0)
    print(f"unknown argument: {a}", file=sys.stderr); sys.exit(2)

DONE = "done"


def unmet(n: dict) -> list[str]:
    return [d for d in (n.get("depends_on") or [])
            if by_id.get(d, {}).get("status") != DONE]


# ---- validate ------------------------------------------------------------
if mode == "validate":
    rc = 0
    seen: set[str] = set()
    for n in nodes:
        nid = n.get("id")
        if not nid:
            print("ERROR: a node has no id"); rc = 1; continue
        if nid in seen:
            print(f"ERROR: duplicate node id '{nid}'"); rc = 1
        seen.add(nid)
        if n.get("status") not in ("pending", "in_progress", "done", "blocked"):
            print(f"ERROR: {nid} has invalid status '{n.get('status')}'"); rc = 1
        if n.get("phase") is None:
            print(f"ERROR: {nid} has no phase"); rc = 1
        for d in n.get("depends_on") or []:
            if d not in by_id:
                print(f"ERROR: {nid} depends on unknown node '{d}'"); rc = 1
        # A node cannot be done while something it depends on is not.
        if n.get("status") == DONE:
            u = unmet(n)
            if u:
                print(f"ERROR: {nid} is done but depends on unfinished {u}")
                rc = 1

    # cycles
    WHITE, GREY, BLACK = 0, 1, 2
    colour = {nid: WHITE for nid in by_id}

    def visit(nid: str, stack: list[str]) -> bool:
        if colour[nid] == GREY:
            print(f"ERROR: dependency cycle: {' -> '.join(stack + [nid])}")
            return False
        if colour[nid] == BLACK:
            return True
        colour[nid] = GREY
        ok = True
        for d in by_id[nid].get("depends_on") or []:
            if d in by_id and not visit(d, stack + [nid]):
                ok = False
        colour[nid] = BLACK
        return ok

    for nid in list(by_id):
        if colour[nid] == WHITE and not visit(nid, []):
            rc = 1

    n_done = sum(1 for n in nodes if n.get("status") == DONE)
    print(f"\n{len(nodes)} node(s), {n_done} done, "
          f"{sum(1 for n in nodes if not unmet(n) and n.get('status') == 'pending')} ready")
    print("graph is consistent" if rc == 0 else "graph is INCONSISTENT")
    sys.exit(rc)

# ---- listings ------------------------------------------------------------
sel = [n for n in nodes if phase is None or n.get("phase") == phase]

if mode == "all":
    for n in sorted(sel, key=lambda n: (n.get("phase", 99), n["id"])):
        u = unmet(n)
        flag = {"done": "x", "in_progress": ">", "blocked": "!"}.get(n.get("status"), " ")
        print(f"[{flag}] p{n.get('phase')} {n['id']}"
              + (f"  (waiting on: {', '.join(u)})" if u and n.get("status") != DONE else ""))
    sys.exit(0)

if mode == "blocked":
    any_blocked = False
    for n in sorted(sel, key=lambda n: (n.get("phase", 99), n["id"])):
        if n.get("status") in (DONE, "in_progress"):
            continue
        u = unmet(n)
        if u:
            any_blocked = True
            print(f"p{n.get('phase')} {n['id']}")
            print(f"    waiting on: {', '.join(u)}")
    if not any_blocked:
        print("nothing blocked")
    sys.exit(0)

# ready
ready = [n for n in sel
         if n.get("status") == "pending" and not unmet(n)]
ready.sort(key=lambda n: (n.get("phase", 99), n["id"]))
in_prog = [n for n in sel if n.get("status") == "in_progress"]

if in_prog:
    for n in in_prog:
        print(f"IN PROGRESS  p{n.get('phase')} {n['id']}")
    print()

if not ready:
    print("nothing ready."
          + ("" if not nodes else " Check --blocked, or everything is done."))
    sys.exit(0)

shown = ready[:limit] if limit else ready
for n in shown:
    crates = ", ".join(n.get("crates") or []) or "-"
    oracles = ", ".join(n.get("oracles") or []) or "-"
    print(f"READY  p{n.get('phase')}  {n['id']}")
    print(f"    {' '.join((n.get('summary') or '').split())}")
    print(f"    crates : {crates}")
    print(f"    oracles: {oracles}")
    if n.get("notes"):
        print(f"    note   : {' '.join(n['notes'].split())}")
    print(f"    start  : git switch -c change/{n['id']} && /opsx:propose")
    print()

if limit and len(ready) > limit:
    print(f"... and {len(ready) - limit} more ready (run without --limit)")
PYEOF
