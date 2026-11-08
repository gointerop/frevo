#!/usr/bin/env python3
"""Load Synthea transaction bundles into a FHIR server.

Used both to seed the differential harness and as the bulk-ingest benchmark
workload, so it reports throughput as well as errors.

    python harness/scripts/load_bundles.py --dir harness/data/synthea-small/fhir \
                                           --base http://localhost:8090/fhir

Ordering matters: Synthea writes `hospitalInformation*.json` and
`practitionerInformation*.json` containing the Organizations and Practitioners
that every patient bundle references. Those are loaded first, serially. Loading
them alongside the patient bundles produces referential-integrity failures that
look like server bugs but are an artefact of the load order.
"""
from __future__ import annotations

import argparse
import json
import queue
import sys
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path

FHIR_JSON = "application/fhir+json"


def post_bundle(base: str, payload: bytes, timeout: float) -> tuple[int, str]:
    req = urllib.request.Request(
        base.rstrip("/") + "/",
        data=payload,
        headers={"Content-Type": FHIR_JSON, "Accept": FHIR_JSON},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            r.read()
            return r.status, ""
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", "replace")
        return e.code, body[:600]
    except Exception as e:  # noqa: BLE001
        return 0, str(e)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", required=True)
    ap.add_argument("--base", required=True)
    ap.add_argument("--concurrency", type=int, default=8)
    ap.add_argument("--timeout", type=float, default=300.0)
    ap.add_argument("--limit", type=int, default=0, help="load at most N patient bundles")
    ap.add_argument("--report", default="", help="write a JSON summary here")
    args = ap.parse_args()

    d = Path(args.dir)
    if not d.is_dir():
        print(f"FATAL: {d} is not a directory", file=sys.stderr)
        return 2

    all_files = sorted(d.glob("*.json"))
    infra = [f for f in all_files
             if f.name.startswith(("hospitalInformation", "practitionerInformation"))]
    patients = [f for f in all_files if f not in set(infra)]
    if args.limit:
        patients = patients[: args.limit]

    if not all_files:
        print(f"FATAL: no .json bundles in {d}", file=sys.stderr)
        return 2

    print(f"base        : {args.base}")
    print(f"infra       : {len(infra)} bundle(s) (loaded first, serially)")
    print(f"patients    : {len(patients)} bundle(s)")
    print(f"concurrency : {args.concurrency}")
    print()

    failures: list[tuple[str, int, str]] = []
    lock = threading.Lock()
    done = [0]
    t_start = time.perf_counter()

    # --- infrastructure, serially -----------------------------------------
    for f in infra:
        payload = f.read_bytes()
        status, err = post_bundle(args.base, payload, args.timeout)
        if status // 100 != 2:
            failures.append((f.name, status, err))
            print(f"  FAIL {f.name}: {status} {err[:200]}")
        else:
            print(f"  ok   {f.name}")
    if failures:
        print("\nFATAL: infrastructure bundles failed to load. Patient bundles "
              "reference the Organizations and Practitioners they contain, so "
              "continuing would produce misleading referential-integrity errors.",
              file=sys.stderr)
        return 1

    # --- patients, concurrently -------------------------------------------
    q: queue.Queue[Path] = queue.Queue()
    for f in patients:
        q.put(f)

    total = len(patients)

    def worker() -> None:
        while True:
            try:
                f = q.get_nowait()
            except queue.Empty:
                return
            try:
                payload = f.read_bytes()
                status, err = post_bundle(args.base, payload, args.timeout)
                with lock:
                    done[0] += 1
                    n = done[0]
                    if status // 100 != 2:
                        failures.append((f.name, status, err))
                    if n % 25 == 0 or n == total:
                        el = time.perf_counter() - t_start
                        rate = n / el if el > 0 else 0
                        print(f"  {n}/{total}  {rate:.1f} bundles/s  "
                              f"failures={len(failures)}")
            finally:
                q.task_done()

    threads = [threading.Thread(target=worker, daemon=True)
               for _ in range(max(1, args.concurrency))]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    elapsed = time.perf_counter() - t_start
    print()
    print(f"loaded {total - len(failures)}/{total} patient bundles in {elapsed:.1f}s "
          f"({total / elapsed:.1f} bundles/s)")

    if failures:
        print(f"\n{len(failures)} failure(s); first 5:")
        for name, status, err in failures[:5]:
            print(f"  {name}: {status} {err[:300]}")

    if args.report:
        Path(args.report).parent.mkdir(parents=True, exist_ok=True)
        Path(args.report).write_text(json.dumps({
            "base": args.base,
            "dir": str(d),
            "infra_bundles": len(infra),
            "patient_bundles": total,
            "failures": len(failures),
            "elapsed_s": round(elapsed, 2),
            "bundles_per_s": round(total / elapsed, 2) if elapsed else None,
        }, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"summary: {args.report}")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
