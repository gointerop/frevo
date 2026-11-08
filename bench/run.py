#!/usr/bin/env python3
"""Benchmark a FHIR server on the workloads in bench/workloads.yaml.

    python bench/run.py --base http://localhost:8090/fhir \
                        --label "HAPI 8.12.0" --container frevo-harness-hapi-1 \
                        --out bench/baseline-hapi-8.12.md

Reports p50/p95/p99 latency, throughput, and peak CPU and RSS of the server
container, per workload.

Three things this tries hard to get right, because a benchmark that gets them
wrong is worse than no benchmark -- it produces a number people act on:

1. **Warmup is measured and discarded.** HAPI runs on the JVM, so the first
   requests are interpreted and the JIT has not run. Comparing a warm Frevo
   against a cold HAPI would flatter Frevo by a large factor and teach us
   nothing.
2. **Ids are sampled from the server under test**, so no workload depends on
   ids that happen to exist only in one deployment.
3. **The dataset is stated in the output.** A latency figure without the row
   count it was measured against is not comparable to anything.
"""
from __future__ import annotations

import argparse
import json
import queue
import random
import statistics
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:
    print("FATAL: PyYAML required.\n  python -m pip install pyyaml", file=sys.stderr)
    sys.exit(2)

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
FHIR_JSON = "application/fhir+json"


# --------------------------------------------------------------------------
# Body templates
# --------------------------------------------------------------------------
def body_for(template: str, run: str) -> Any:
    if template == "patient-small":
        return {
            "resourceType": "Patient",
            "identifier": [{"system": "http://frevo.test/bench", "value": f"{run}-{uuid.uuid4().hex[:8]}"}],
            "name": [{"family": "Benchmark", "given": ["Case"]}],
            "gender": "unknown",
            "birthDate": "1980-01-01",
        }
    if template == "transaction-small":
        u = f"urn:uuid:{uuid.uuid4()}"
        return {
            "resourceType": "Bundle",
            "type": "transaction",
            "entry": [
                {
                    "fullUrl": u,
                    "resource": {
                        "resourceType": "Patient",
                        "identifier": [{"system": "http://frevo.test/bench", "value": f"{run}-{uuid.uuid4().hex[:8]}"}],
                        "name": [{"family": "TxBench"}],
                    },
                    "request": {"method": "POST", "url": "Patient"},
                },
                {
                    "resource": {
                        "resourceType": "Observation",
                        "status": "final",
                        "code": {"coding": [{"system": "http://loinc.org", "code": "8867-4"}]},
                        "subject": {"reference": u},
                        "valueQuantity": {"value": 70, "unit": "/min",
                                          "system": "http://unitsofmeasure.org", "code": "/min"},
                    },
                    "request": {"method": "POST", "url": "Observation"},
                },
            ],
        }
    raise ValueError(f"unknown body_template '{template}'")


# --------------------------------------------------------------------------
# HTTP
# --------------------------------------------------------------------------
def request_once(base: str, method: str, path: str, body: Any, timeout: float) -> tuple[int, float, int]:
    url = base.rstrip("/") + "/" + path.lstrip("/")
    data = json.dumps(body).encode() if body is not None else None
    hdrs = {"Accept": FHIR_JSON}
    if data:
        hdrs["Content-Type"] = FHIR_JSON
    req = urllib.request.Request(url, data=data, headers=hdrs, method=method)
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            n = len(r.read())
            return r.status, (time.perf_counter() - t0) * 1000, n
    except urllib.error.HTTPError as e:
        n = len(e.read())
        return e.code, (time.perf_counter() - t0) * 1000, n
    except Exception:  # noqa: BLE001
        return 0, (time.perf_counter() - t0) * 1000, 0


# --------------------------------------------------------------------------
# Container resource sampling
# --------------------------------------------------------------------------
class StatSampler(threading.Thread):
    """Polls `docker stats` for one container while a workload runs."""

    def __init__(self, container: str, interval: float = 1.0):
        super().__init__(daemon=True)
        self.container = container
        self.interval = interval
        self.cpu: list[float] = []
        self.mem: list[float] = []
        self._stop = threading.Event()

    def run(self) -> None:
        while not self._stop.is_set():
            try:
                out = subprocess.run(
                    ["docker", "stats", "--no-stream", "--format",
                     "{{.CPUPerc}}\t{{.MemUsage}}", self.container],
                    capture_output=True, text=True, timeout=20,
                )
                line = (out.stdout or "").strip()
                if line:
                    cpu_s, mem_s = line.split("\t")[:2]
                    self.cpu.append(float(cpu_s.strip().rstrip("%")))
                    used = mem_s.split("/")[0].strip()
                    mult = {"B": 1 / 2**20, "KiB": 1 / 1024, "MiB": 1.0,
                            "GiB": 1024.0, "KB": 1 / 1024, "MB": 1.0, "GB": 1024.0}
                    for suffix, m in sorted(mult.items(), key=lambda kv: -len(kv[0])):
                        if used.endswith(suffix):
                            self.mem.append(float(used[: -len(suffix)]) * m)
                            break
            except Exception:  # noqa: BLE001  (sampling must never fail a run)
                pass
            self._stop.wait(self.interval)

    def stop(self) -> None:
        self._stop.set()


# --------------------------------------------------------------------------
@dataclass
class Result:
    name: str
    stresses: str
    method: str
    path: str
    iterations: int
    concurrency: int
    ok: int = 0
    errors: int = 0
    status_counts: dict[int, int] = field(default_factory=dict)
    latencies: list[float] = field(default_factory=list)
    bytes_total: int = 0
    elapsed_s: float = 0.0
    cpu_peak: float | None = None
    cpu_mean: float | None = None
    rss_peak_mib: float | None = None

    def pct(self, p: float) -> float:
        if not self.latencies:
            return float("nan")
        s = sorted(self.latencies)
        k = max(0, min(len(s) - 1, int(round(p / 100 * (len(s) - 1)))))
        return s[k]

    @property
    def rps(self) -> float:
        return (self.ok / self.elapsed_s) if self.elapsed_s else 0.0


def sample_ids(base: str, timeout: float) -> dict[str, list[str]]:
    """Collect real ids and a family name from the server under test."""
    out: dict[str, list[str]] = {"patient": [], "encounter": [], "family": []}

    def get(path: str) -> Any:
        url = base.rstrip("/") + "/" + path.lstrip("/")
        req = urllib.request.Request(url, headers={"Accept": FHIR_JSON})
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return json.loads(r.read().decode("utf-8", "replace"))
        except Exception:  # noqa: BLE001
            return None

    b = get("/Patient?_count=200&_elements=id,name")
    for e in (b or {}).get("entry") or []:
        res = e.get("resource") or {}
        if res.get("id"):
            out["patient"].append(res["id"])
        for nm in res.get("name") or []:
            if nm.get("family"):
                out["family"].append(nm["family"])

    b = get("/Encounter?_count=100&_elements=id")
    for e in (b or {}).get("entry") or []:
        if (e.get("resource") or {}).get("id"):
            out["encounter"].append(e["resource"]["id"])

    return out


def dataset_profile(base: str, timeout: float) -> dict[str, int]:
    prof: dict[str, int] = {}
    for t in ("Patient", "Observation", "Encounter", "Condition", "Procedure",
              "MedicationRequest", "Organization", "Practitioner", "Immunization",
              "DiagnosticReport", "Claim"):
        url = base.rstrip("/") + f"/{t}?_summary=count&_total=accurate"
        req = urllib.request.Request(url, headers={"Accept": FHIR_JSON})
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                prof[t] = int(json.loads(r.read().decode()).get("total", -1))
        except Exception:  # noqa: BLE001
            prof[t] = -1
    return prof


def run_workload(base: str, wl: dict[str, Any], ids: dict[str, list[str]],
                 container: str | None, run_token: str) -> Result:
    method = wl.get("method", "GET")
    path_t = wl["path"]
    iters = int(wl["iterations"])
    conc = int(wl["concurrency"])
    warm = int(wl["warmup"])
    timeout = float(wl["timeout"])
    tmpl = wl.get("body_template")

    def make() -> tuple[str, Any]:
        p = path_t
        if "{patient}" in p:
            p = p.replace("{patient}", random.choice(ids["patient"]) if ids["patient"] else "0")
        if "{encounter}" in p:
            p = p.replace("{encounter}", random.choice(ids["encounter"]) if ids["encounter"] else "0")
        if "{family}" in p:
            p = p.replace("{family}", random.choice(ids["family"]) if ids["family"] else "Smith")
        p = p.replace("{run}", run_token)
        return p, (body_for(tmpl, run_token) if tmpl else None)

    res = Result(name=wl["name"], stresses=wl.get("stresses", "").strip(),
                 method=method, path=path_t, iterations=iters, concurrency=conc)

    # --- warmup: measured, then discarded -------------------------------
    for _ in range(warm):
        p, b = make()
        request_once(base, method, p, b, timeout)

    sampler = None
    if container:
        sampler = StatSampler(container)
        sampler.start()

    q: queue.Queue[int] = queue.Queue()
    for i in range(iters):
        q.put(i)
    lock = threading.Lock()
    t0 = time.perf_counter()

    def worker() -> None:
        while True:
            try:
                q.get_nowait()
            except queue.Empty:
                return
            p, b = make()
            status, ms, n = request_once(base, method, p, b, timeout)
            with lock:
                res.latencies.append(ms)
                res.bytes_total += n
                res.status_counts[status] = res.status_counts.get(status, 0) + 1
                if 200 <= status < 300:
                    res.ok += 1
                else:
                    res.errors += 1
            q.task_done()

    ts = [threading.Thread(target=worker, daemon=True) for _ in range(max(1, conc))]
    for t in ts:
        t.start()
    for t in ts:
        t.join()

    res.elapsed_s = time.perf_counter() - t0
    if sampler:
        sampler.stop()
        sampler.join(timeout=5)
        if sampler.cpu:
            res.cpu_peak = max(sampler.cpu)
            res.cpu_mean = statistics.fmean(sampler.cpu)
        if sampler.mem:
            res.rss_peak_mib = max(sampler.mem)
    return res


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True)
    ap.add_argument("--label", default="server under test")
    ap.add_argument("--container", default="", help="docker container name, for CPU/RSS sampling")
    ap.add_argument("--workloads", default=str(HERE / "workloads.yaml"))
    ap.add_argument("--only", default="", help="comma-separated workload names")
    ap.add_argument("--out", default="", help="write a markdown report here")
    ap.add_argument("--json", dest="json_out", default="")
    ap.add_argument("--dataset", default="synthea-small", help="dataset label for the report")
    args = ap.parse_args()

    doc = yaml.safe_load(Path(args.workloads).read_text(encoding="utf-8"))
    defaults = doc.get("defaults") or {}
    wls = doc["workloads"]
    if args.only:
        want = {s.strip() for s in args.only.split(",")}
        wls = [w for w in wls if w["name"] in want]
    for w in wls:
        for k, v in defaults.items():
            w.setdefault(k, v)

    print(f"target : {args.label} at {args.base}")
    print("sampling ids and dataset profile...")
    ids = sample_ids(args.base, 60)
    prof = dataset_profile(args.base, 120)
    print(f"  patients sampled: {len(ids['patient'])}, families: {len(set(ids['family']))}")
    print(f"  dataset: " + ", ".join(f"{k}={v}" for k, v in prof.items() if v > 0))
    if not ids["patient"]:
        print("FATAL: no Patient ids on the server. Load a dataset first:\n"
              "  bash harness/scripts/synthea.sh load small <base>", file=sys.stderr)
        return 2
    print()

    run_token = uuid.uuid4().hex[:10]
    results: list[Result] = []
    for w in wls:
        print(f"  {w['name']:<30} ", end="", flush=True)
        r = run_workload(args.base, w, ids, args.container or None, run_token)
        results.append(r)
        print(f"p50={r.pct(50):8.1f}ms  p95={r.pct(95):8.1f}ms  "
              f"p99={r.pct(99):8.1f}ms  {r.rps:7.1f} rps  err={r.errors}")

    payload = {
        "label": args.label,
        "base": args.base,
        "dataset": args.dataset,
        "dataset_profile": prof,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "workloads": [
            {
                "name": r.name, "method": r.method, "path": r.path,
                "iterations": r.iterations, "concurrency": r.concurrency,
                "ok": r.ok, "errors": r.errors, "status_counts": r.status_counts,
                "p50_ms": round(r.pct(50), 2), "p95_ms": round(r.pct(95), 2),
                "p99_ms": round(r.pct(99), 2),
                "mean_ms": round(statistics.fmean(r.latencies), 2) if r.latencies else None,
                "rps": round(r.rps, 2),
                "bytes_mean": round(r.bytes_total / r.ok) if r.ok else None,
                "cpu_peak_pct": round(r.cpu_peak, 1) if r.cpu_peak is not None else None,
                "cpu_mean_pct": round(r.cpu_mean, 1) if r.cpu_mean is not None else None,
                "rss_peak_mib": round(r.rss_peak_mib) if r.rss_peak_mib is not None else None,
            }
            for r in results
        ],
    }

    if args.json_out:
        Path(args.json_out).parent.mkdir(parents=True, exist_ok=True)
        Path(args.json_out).write_text(json.dumps(payload, indent=2) + "\n",
                                       encoding="utf-8", newline="\n")
        print(f"\njson: {args.json_out}")

    if args.out:
        Path(args.out).parent.mkdir(parents=True, exist_ok=True)
        Path(args.out).write_text(render_markdown(payload, results),
                                  encoding="utf-8", newline="\n")
        print(f"markdown: {args.out}")

    bad = [r for r in results if r.errors > r.iterations * 0.01]
    if bad:
        print("\nWARNING: workloads with >1% errors (their numbers are not usable):")
        for r in bad:
            print(f"  {r.name}: {r.errors}/{r.iterations} errors, statuses {r.status_counts}")
        return 1
    return 0


def render_markdown(p: dict[str, Any], results: list[Result]) -> str:
    L: list[str] = []
    A = L.append
    A(f"# Baseline: {p['label']}")
    A("")
    A(f"- measured: {p['generated_at']}")
    A(f"- target: `{p['base']}`")
    A(f"- dataset: `{p['dataset']}`")
    A("")
    A("## Dataset")
    A("")
    A("A latency figure without the data volume it was measured against is not")
    A("comparable to anything, so the row counts are recorded here.")
    A("")
    A("| Resource | Count |")
    A("|---|---|")
    for k, v in p["dataset_profile"].items():
        if v and v > 0:
            A(f"| {k} | {v:,} |")
    A("")
    A("## Results")
    A("")
    A("| Workload | p50 ms | p95 ms | p99 ms | rps | conc | errors | CPU peak % | RSS peak MiB |")
    A("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for w in p["workloads"]:
        A(f"| `{w['name']}` | {w['p50_ms']} | {w['p95_ms']} | {w['p99_ms']} | "
          f"{w['rps']} | {w['concurrency']} | {w['errors']} | "
          f"{w['cpu_peak_pct'] if w['cpu_peak_pct'] is not None else '-'} | "
          f"{w['rss_peak_mib'] if w['rss_peak_mib'] is not None else '-'} |")
    A("")
    A("## What each workload stresses")
    A("")
    for r in results:
        if r.stresses:
            A(f"**`{r.name}`** — {' '.join(r.stresses.split())}")
            A("")
    A("## Method")
    A("")
    A("- Warmup requests are issued and **discarded** before measurement. HAPI")
    A("  runs on the JVM, so a cold measurement would be meaningless and would")
    A("  flatter any native implementation compared against it.")
    A("- Ids are sampled from the server under test, so no workload depends on")
    A("  ids that exist only in one deployment.")
    A("- CPU and RSS are sampled from `docker stats` once a second during each")
    A("  workload, so they are the server process's usage, not the host's.")
    A("- Percentiles are computed from every measured request in the workload.")
    A("")
    A("Reproduce with:")
    A("")
    A("```bash")
    A("docker compose -f harness/docker-compose.yml up -d")
    A("bash harness/scripts/synthea.sh generate small")
    A("bash harness/scripts/synthea.sh load small http://localhost:8090/fhir")
    A(f"python bench/run.py --base {p['base']} --label \"{p['label']}\" \\")
    A("    --container frevo-harness-hapi-1 --out bench/baseline-hapi-8.12.md")
    A("```")
    A("")
    return "\n".join(L)


if __name__ == "__main__":
    sys.exit(main())
