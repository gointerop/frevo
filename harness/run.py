#!/usr/bin/env python3
"""Differential test runner.

Sends identical request sequences to two FHIR servers, normalises both
responses, and diffs them. HAPI FHIR 8.12.0 is the baseline; anything with a
base URL can be the candidate.

    python harness/run.py --baseline http://localhost:8090/fhir \
                          --candidate http://localhost:8080/fhir

Self-test -- point both at the same server. Expected result: zero diffs. This
is what proves the runner and the normaliser are not themselves a source of
false differences:

    python harness/run.py --self-test --baseline http://localhost:8090/fhir

Isolation: each run gets its own run id, substituted into cases as `{{run}}`.
Cases tag the resources they create with it and scope their searches to it, so
the baseline run and the candidate run never see each other's data -- and so a
self-test against one server is still two independent runs rather than the
second run reading the first one's rows.

Exit status is 0 only when every case matched. A diff is a bug or a recorded
deviation in docs/deviations.md; it is never something to normalise away.
"""
from __future__ import annotations

import argparse
import difflib
import glob
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import uuid
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))

from normalize import RULES, Normalizer  # noqa: E402

try:
    import yaml
except ImportError:
    print("FATAL: PyYAML is required.\n  python -m pip install pyyaml", file=sys.stderr)
    sys.exit(2)

FHIR_JSON = "application/fhir+json"


# --------------------------------------------------------------------------
# HTTP
# --------------------------------------------------------------------------
@dataclass
class Response:
    status: int
    headers: dict[str, str]
    body: Any
    raw: str
    elapsed_ms: float
    error: str | None = None


def send(base: str, method: str, path: str, headers: dict[str, str] | None,
         body: Any, timeout: float = 60.0) -> Response:
    url = base.rstrip("/") + "/" + path.lstrip("/")
    hdrs = {"Accept": FHIR_JSON}
    if headers:
        hdrs.update(headers)
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        hdrs.setdefault("Content-Type", FHIR_JSON)

    req = urllib.request.Request(url, data=data, headers=hdrs, method=method.upper())
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            raw = r.read().decode("utf-8", "replace")
            status, rh = r.status, dict(r.headers)
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", "replace")
        status, rh = e.code, dict(e.headers)
    except Exception as e:  # noqa: BLE001  (network/DNS/timeout all reported the same way)
        return Response(0, {}, None, "", (time.perf_counter() - t0) * 1000, str(e))

    elapsed = (time.perf_counter() - t0) * 1000
    parsed: Any = None
    if raw.strip():
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            parsed = {"<unparseable>": raw[:2000]}
    return Response(status, rh, parsed, raw, elapsed)


# --------------------------------------------------------------------------
# Corpus
# --------------------------------------------------------------------------
@dataclass
class Case:
    id: str
    suite: str
    scenarios: list[str]
    request: dict[str, Any]
    expect_status_class: str | None
    normalize: list[str]
    setup: list[dict[str, Any]] = field(default_factory=list)
    description: str = ""
    compare_headers: bool = False


def load_corpus(patterns: list[str]) -> list[Case]:
    cases: list[Case] = []
    seen: set[str] = set()
    files: list[str] = []
    for p in patterns:
        files.extend(sorted(glob.glob(p, recursive=True)))
    if not files:
        print(f"FATAL: no corpus files matched {patterns}", file=sys.stderr)
        sys.exit(2)

    for f in files:
        doc = yaml.safe_load(Path(f).read_text(encoding="utf-8")) or {}
        suite = doc.get("suite") or Path(f).stem
        for raw in doc.get("cases") or []:
            cid = raw.get("id")
            if not cid:
                print(f"FATAL: a case in {f} has no id", file=sys.stderr)
                sys.exit(2)
            if cid in seen:
                print(f"FATAL: duplicate case id {cid} (in {f})", file=sys.stderr)
                sys.exit(2)
            seen.add(cid)
            rules = raw.get("normalize") or []
            bad = [r for r in rules if r not in RULES]
            if bad:
                print(f"FATAL: case {cid} uses unknown normalisation rule(s) {bad}.\n"
                      f"Known: {sorted(RULES)}", file=sys.stderr)
                sys.exit(2)
            cases.append(Case(
                id=cid,
                suite=suite,
                scenarios=raw.get("scenarios") or [],
                request=raw.get("request") or {},
                expect_status_class=raw.get("expect_status_class"),
                normalize=rules,
                setup=raw.get("setup") or [],
                description=raw.get("description", ""),
                compare_headers=bool(raw.get("compare_headers", False)),
            ))
    return cases


VAR_RE = re.compile(r"\{\{([a-zA-Z0-9_.]+)\}\}")


def substitute(obj: Any, vars_: dict[str, str]) -> Any:
    if isinstance(obj, str):
        def repl(m: re.Match[str]) -> str:
            key = m.group(1)
            if key not in vars_:
                raise KeyError(
                    f"case references {{{{{key}}}}} but nothing defined it. "
                    "Variables come from the run id ('run') or from a setup "
                    "step's 'capture'."
                )
            return vars_[key]
        return VAR_RE.sub(repl, obj)
    if isinstance(obj, dict):
        return {k: substitute(v, vars_) for k, v in obj.items()}
    if isinstance(obj, list):
        return [substitute(v, vars_) for v in obj]
    return obj


def jsonpath_get(obj: Any, path: str) -> str | None:
    """Minimal dotted/bracket path lookup, enough for capturing ids."""
    cur = obj
    for part in re.findall(r"[^.\[\]]+", path):
        if isinstance(cur, list):
            try:
                cur = cur[int(part)]
            except (ValueError, IndexError):
                return None
        elif isinstance(cur, dict):
            if part not in cur:
                return None
            cur = cur[part]
        else:
            return None
    return cur if isinstance(cur, str) else (json.dumps(cur) if cur is not None else None)


# --------------------------------------------------------------------------
# Execution
# --------------------------------------------------------------------------
@dataclass
class Outcome:
    case: Case
    matched: bool
    reason: str = ""
    diff: str = ""
    baseline_status: int = 0
    candidate_status: int = 0
    baseline_ms: float = 0.0
    candidate_ms: float = 0.0


def status_class_ok(status: int, expected: str | int | None) -> bool:
    if expected is None or expected == "":
        return True
    # YAML parses `expect_status_class: 404` as an int and `2xx` as a string,
    # so both forms have to be accepted.
    e = str(expected).strip().lower()
    if e.endswith("xx"):
        return status // 100 == int(e[0])
    return status == int(e)


def run_side(base: str, case: Case, run_id: str, timeout: float) -> tuple[Response, dict[str, str]]:
    vars_: dict[str, str] = {"run": run_id}
    for step in case.setup:
        s = substitute(step, vars_)
        r = send(base, s.get("method", "POST"), s["path"], s.get("headers"),
                 s.get("body"), timeout)
        if r.error:
            raise RuntimeError(f"setup step failed against {base}: {r.error}")
        if r.status // 100 not in (2,):
            raise RuntimeError(
                f"setup step {s.get('method')} {s['path']} returned {r.status} "
                f"against {base}: {r.raw[:400]}"
            )
        for name, path in (step.get("capture") or {}).items():
            val = jsonpath_get(r.body, path)
            if val is None:
                # Fall back to the Location header, which is where a create
                # reports the assigned id when the body is not returned.
                loc = r.headers.get("Location") or r.headers.get("Content-Location") or ""
                m = re.search(r"/([A-Za-z0-9._-]+)(?:/_history/.+)?$", loc)
                val = m.group(1) if m else None
            if val is None:
                raise RuntimeError(
                    f"setup capture '{name}' found nothing at '{path}' "
                    f"(and no Location header) against {base}"
                )
            vars_[name] = val

    req = substitute(case.request, vars_)
    resp = send(base, req.get("method", "GET"), req["path"], req.get("headers"),
                req.get("body"), timeout)
    return resp, vars_


def _tokens(vars_: dict[str, str]) -> dict[str, str]:
    """Values that differ between the two runs by construction.

    The run id and every setup-captured value (resource ids, mostly) are
    generated independently on each side, so they are substituted for a stable
    placeholder before comparing. Keeping the variable name in the placeholder
    means a value appearing in the wrong place is still caught.
    """
    return {v: f"<{k}>" for k, v in vars_.items() if isinstance(v, str) and len(v) >= 4}


def compare(case: Case, base_url: str, cand_url: str,
            b: Response, c: Response,
            b_vars: dict[str, str] | None = None,
            c_vars: dict[str, str] | None = None) -> Outcome:
    out = Outcome(case=case, matched=False,
                  baseline_status=b.status, candidate_status=c.status,
                  baseline_ms=b.elapsed_ms, candidate_ms=c.elapsed_ms)

    if b.error or c.error:
        out.reason = f"transport error (baseline={b.error!r}, candidate={c.error!r})"
        return out

    if b.status != c.status:
        out.reason = f"status differs: baseline {b.status}, candidate {c.status}"
        out.diff = _text_diff(
            f"baseline {b.status}\n" + b.raw[:4000],
            f"candidate {c.status}\n" + c.raw[:4000],
        )
        return out

    if not status_class_ok(b.status, case.expect_status_class):
        out.reason = (f"both returned {b.status}, which is outside the case's "
                      f"expected class {case.expect_status_class}. The oracle "
                      f"itself did not behave as the case assumes, so the case "
                      f"is wrong or the oracle is misconfigured.")
        out.diff = b.raw[:2000]
        return out

    nb = Normalizer(case.normalize, [base_url], _tokens(b_vars or {}))
    nc = Normalizer(case.normalize, [cand_url], _tokens(c_vars or {}))
    bb = json.dumps(nb.body(b.body), indent=2, sort_keys=True, ensure_ascii=False)
    cc = json.dumps(nc.body(c.body), indent=2, sort_keys=True, ensure_ascii=False)

    if bb != cc:
        out.reason = "body differs after normalisation"
        out.diff = _text_diff(bb, cc)
        return out

    if case.compare_headers:
        hb = nb.headers(b.headers)
        hc = nc.headers(c.headers)
        if hb != hc:
            out.reason = "headers differ after normalisation"
            out.diff = _text_diff(
                json.dumps(hb, indent=2, sort_keys=True),
                json.dumps(hc, indent=2, sort_keys=True),
            )
            return out

    out.matched = True
    return out


def _text_diff(a: str, b: str, limit: int = 120) -> str:
    lines = list(difflib.unified_diff(
        a.splitlines(), b.splitlines(),
        fromfile="baseline (HAPI 8.12.0)", tofile="candidate", lineterm="", n=3,
    ))
    if len(lines) > limit:
        lines = lines[:limit] + [f"... {len(lines) - limit} more diff lines truncated"]
    return "\n".join(lines)


def wait_ready(base: str, timeout: float) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        r = send(base, "GET", "/metadata", None, None, timeout=10)
        if r.status == 200 and isinstance(r.body, dict) \
                and r.body.get("resourceType") == "CapabilityStatement":
            return True
        time.sleep(3)
    return False


# --------------------------------------------------------------------------
# Reporting
# --------------------------------------------------------------------------
def markdown_report(outcomes: list[Outcome], base: str, cand: str,
                    self_test: bool) -> str:
    matched = [o for o in outcomes if o.matched]
    failed = [o for o in outcomes if not o.matched]
    L: list[str] = []
    L.append("# Differential report")
    L.append("")
    L.append(f"- baseline : `{base}` (HAPI FHIR 8.12.0)")
    L.append(f"- candidate: `{cand}`" + ("  **(self-test: same server)**" if self_test else ""))
    L.append(f"- cases    : {len(outcomes)}")
    L.append(f"- matched  : {len(matched)}")
    L.append(f"- **diffs**: **{len(failed)}**")
    L.append("")
    if self_test:
        L.append("A self-test runs the corpus twice against the same server, with")
        L.append("different run ids. Any diff here is a defect in the runner or in the")
        L.append("normalisation rules, not a parity problem.")
        L.append("")
    if failed:
        L.append("## Diffs")
        L.append("")
        for o in failed:
            L.append(f"### `{o.case.id}` -- {o.reason}")
            L.append("")
            if o.case.scenarios:
                L.append("scenarios: " + ", ".join(f"`{s}`" for s in o.case.scenarios))
                L.append("")
            if o.case.normalize:
                L.append("normalisation applied: " + ", ".join(f"`{r}`" for r in o.case.normalize))
                L.append("")
            if o.diff:
                L.append("```diff")
                L.append(o.diff)
                L.append("```")
                L.append("")
    else:
        L.append("No differences. Every case matched after normalisation.")
        L.append("")
    L.append("## Timing")
    L.append("")
    L.append("| Case | baseline ms | candidate ms |")
    L.append("|---|---|---|")
    for o in outcomes:
        L.append(f"| `{o.case.id}` | {o.baseline_ms:.0f} | {o.candidate_ms:.0f} |")
    L.append("")
    return "\n".join(L)


def junit_report(outcomes: list[Outcome]) -> str:
    suites: dict[str, list[Outcome]] = {}
    for o in outcomes:
        suites.setdefault(o.case.suite, []).append(o)
    root = ET.Element("testsuites",
                      tests=str(len(outcomes)),
                      failures=str(sum(1 for o in outcomes if not o.matched)))
    for name, os_ in suites.items():
        ts = ET.SubElement(root, "testsuite", name=name, tests=str(len(os_)),
                           failures=str(sum(1 for o in os_ if not o.matched)))
        for o in os_:
            tc = ET.SubElement(ts, "testcase", classname=name, name=o.case.id,
                               time=f"{o.baseline_ms / 1000:.3f}")
            if not o.matched:
                f = ET.SubElement(tc, "failure", message=o.reason)
                f.text = o.diff or o.reason
    return ET.tostring(root, encoding="unicode")


# --------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--baseline", default=os.environ.get("FREVO_BASELINE",
                                                         "http://localhost:8090/fhir"))
    ap.add_argument("--candidate", default=os.environ.get("FREVO_CANDIDATE", ""))
    ap.add_argument("--self-test", action="store_true",
                    help="run the corpus twice against the baseline; expect zero diffs")
    ap.add_argument("--corpus", action="append", default=[],
                    help="glob(s) of corpus files (default: harness/corpus/**/*.yaml)")
    ap.add_argument("--filter", default="", help="only cases whose id contains this")
    ap.add_argument("--report", default="", help="write a markdown report here")
    ap.add_argument("--junit", default="", help="write a JUnit XML report here")
    ap.add_argument("--timeout", type=float, default=60.0)
    ap.add_argument("--wait", type=float, default=0.0,
                    help="seconds to wait for both servers to serve /metadata")
    ap.add_argument("--list-rules", action="store_true")
    args = ap.parse_args()

    if args.list_rules:
        from normalize import describe_rules
        print(describe_rules())
        return 0

    candidate = args.baseline if args.self_test else args.candidate
    if not candidate:
        print("FATAL: --candidate is required (or pass --self-test).", file=sys.stderr)
        return 2

    patterns = args.corpus or [str(HERE / "corpus" / "**" / "*.yaml")]
    cases = load_corpus(patterns)
    if args.filter:
        cases = [c for c in cases if args.filter in c.id]
    if not cases:
        print("FATAL: no cases selected.", file=sys.stderr)
        return 2

    if args.wait:
        for label, url in (("baseline", args.baseline), ("candidate", candidate)):
            if not wait_ready(url, args.wait):
                print(f"FATAL: {label} at {url} did not serve a CapabilityStatement "
                      f"within {args.wait:.0f}s.", file=sys.stderr)
                return 2

    print(f"baseline : {args.baseline}")
    print(f"candidate: {candidate}" + ("  (self-test)" if args.self_test else ""))
    print(f"cases    : {len(cases)}")
    print()

    outcomes: list[Outcome] = []
    for case in cases:
        # Distinct run ids keep the two runs' data disjoint, which is what makes
        # a self-test against a single server meaningful.
        b_run = uuid.uuid4().hex[:12]
        c_run = uuid.uuid4().hex[:12]
        try:
            b, bv = run_side(args.baseline, case, b_run, args.timeout)
            c, cv = run_side(candidate, case, c_run, args.timeout)
        except (RuntimeError, KeyError) as e:
            o = Outcome(case=case, matched=False, reason=f"setup failed: {e}")
            outcomes.append(o)
            print(f"  SETUP-FAIL {case.id}: {e}")
            continue
        o = compare(case, args.baseline, candidate, b, c, bv, cv)
        outcomes.append(o)
        print(f"  {'ok  ' if o.matched else 'DIFF'} {case.id}"
              + ("" if o.matched else f"  -- {o.reason}"))

    failed = [o for o in outcomes if not o.matched]
    print()
    print(f"matched {len(outcomes) - len(failed)}/{len(outcomes)}, diffs {len(failed)}")

    if args.report:
        Path(args.report).parent.mkdir(parents=True, exist_ok=True)
        Path(args.report).write_text(
            markdown_report(outcomes, args.baseline, candidate, args.self_test),
            encoding="utf-8", newline="\n")
        print(f"markdown report: {args.report}")
    if args.junit:
        Path(args.junit).parent.mkdir(parents=True, exist_ok=True)
        Path(args.junit).write_text(junit_report(outcomes), encoding="utf-8", newline="\n")
        print(f"junit report: {args.junit}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
