#!/usr/bin/env python3
"""The HAPI side of the config-roundtrip oracle: build, inspect, run, record, self-test.

    python harness/config-roundtrip/run.py inspect      # check the pinned image (versions, application files)
    python harness/config-roundtrip/run.py build        # docker build the probe image
    python harness/config-roundtrip/run.py record       # run every case, write hapi-8.12.0-results.json
    python harness/config-roundtrip/run.py self-test    # run every case twice, compare (config.oracle.002)
    python harness/config-roundtrip/run.py schema       # print the bean-property schema of the starter's classes
    python harness/config-roundtrip/run.py case <id>    # run one case and print what was recorded

`record` and `self-test` run `inspect` and `build` first and stop if `inspect` fails
(tasks.md task 1: the build fails if spring-boot, spring-core or snakeyaml in
WEB-INF/lib is not the version the spec delta was measured with). `--diagnostic`
runs anyway and writes target/diagnostic-results.json instead of the committed record;
a diagnostic file is evidence for an owner decision, never the oracle.

Requires Docker with the pinned image available. Every case runs in a fresh container
of the probe image, through the image's own ENTRYPOINT (PropertiesLauncher), with the
working directory /app (the image's WORKDIR), the case's files copied into /app, its
root/ files copied to /, the image's environment plus the case's variables, and the
case's arguments. Standard output and standard error are captured separately.

Two runners (review.md finding 1, owner decision 2026-11-22). The probe runner
(loader.main=dev.frevo.harness.config.ConfigRoundtrip) is what a case's document, reads,
gates and properties come from; every case runs it. A case the probe fails is run a
second time by the application runner: the unchanged pinned image itself, whose
PropertiesLauncher starts the war's own Start-Class, ca.uhn.fhir.jpa.starter.Application,
with the same files, environment and arguments. Its failure output is what HAPI prints,
so it is the case's recorded result ("runner": "application"); the probe's own failure is
kept beside it under "probe". A case the application runner does not fail (it starts the
server) is listed under "runner_disagreements" and keeps the probe's result, flagged.

Every case the probe passes is run by the application runner too (review.md, fifth review,
finding 1): the probe's run stays its record, and the real Application's outcome is kept
under "application" (started, or exit status and config lines). The cases where that run
fails are listed under "application_failures", the reverse of "runner_disagreements". A
real Application has started the server when it prints Spring Boot's start line and is
still running APPLICATION_SETTLE seconds later, or is still running after
APPLICATION_TIMEOUT seconds (the 600 s safeguard); it is then killed. Every container
carries the label dev.frevo.config-roundtrip=1.
"""

from __future__ import annotations

import concurrent.futures
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import threading
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
CASES = HERE / "cases"
PACKAGED = HERE / "packaged"
TARGET = HERE / "target"
RECORD = HERE / "hapi-8.12.0-results.json"
DIAGNOSTIC = TARGET / "diagnostic-results.json"

HAPI_IMAGE = "hapiproject/hapi:v8.12.0-2@sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e"
PROBE_TAG = "frevo-config-roundtrip:local"

# proposal.md Risk 1: the versions the spec delta's THEN clauses were measured with.
# Re-pinned in reference/PINS.md (#38, #40): Spring Framework 6.2.19 is what the image ships.
EXPECTED_JARS = {
    "spring-boot": "3.5.14",
    "spring-core": "6.2.19",
    "spring-beans": "6.2.19",
    "spring-context": "6.2.19",
    "spring-aop": "6.2.19",
    "spring-expression": "6.2.19",
    "snakeyaml": "2.4",
    "slf4j-api": "2.0.16",
}
# Must be absent from WEB-INF/lib (the image ships no spring-jcl jar).
ABSENT_JARS = ["spring-jcl"]
# Reported, not enforced: the rest of the probe's class path (design.md section 2).
REPORTED_JARS = [
    "spring-boot-autoconfigure", "logback-classic", "logback-core", "jul-to-slf4j", "jackson-core",
    "hapi-fhir-base", "hapi-fhir-storage", "hapi-fhir-jpaserver-model", "org.hl7.fhir.r4",
    "org.hl7.fhir.utilities", "cqf-fhir-cql", "cqf-fhir-utility", "cql-to-elm-jvm",
]
PACKAGED_FILES = ["application.yaml", "application-cds.yaml", "application-elastic.yaml"]
APPLICATION_FILE = re.compile(r"^(config/)?application[^/]*\.(properties|xml|yml|yaml)$|^config/application")

STATUS_LINE = re.compile(r"^\d\d:\d\d:\d\d,\d\d\d \|-(INFO|WARN|ERROR) in ")


def docker(*args: str, check: bool = True, capture: bool = True, **kw) -> subprocess.CompletedProcess:
    return subprocess.run(["docker", *args], check=check, capture_output=capture, **kw)


# ----------------------------------------------------------------------------------------
# inspect
# ----------------------------------------------------------------------------------------

def extract_war() -> Path:
    war = TARGET / "main.war"
    if not war.exists():
        TARGET.mkdir(parents=True, exist_ok=True)
        cid = docker("create", HAPI_IMAGE, text=True).stdout.strip()
        try:
            docker("cp", f"{cid}:/app/main.war", str(war))
        finally:
            docker("rm", cid)
    return war


def inspect() -> dict:
    """Read the pinned image's war: jar versions, application files, packaged YAML."""
    war = extract_war()
    report: dict = {"image": HAPI_IMAGE, "jars": {}, "problems": [], "application_files_in_lib": []}
    with zipfile.ZipFile(war) as z:
        libs = [n for n in z.namelist() if n.startswith("WEB-INF/lib/") and n.endswith(".jar")]
        versions: dict[str, list[str]] = {}
        for n in libs:
            base = n.rsplit("/", 1)[1][:-4]
            m = re.match(r"^(.*?)-(\d[\w.\-]*)$", base)
            if m:
                versions.setdefault(m.group(1), []).append(m.group(2))
            with z.open(n) as raw:
                inner = zipfile.ZipFile(io.BytesIO(raw.read()))
                for e in inner.namelist():
                    if APPLICATION_FILE.match(e):
                        report["application_files_in_lib"].append(f"{n}!/{e}")
        for name, want in EXPECTED_JARS.items():
            got = versions.get(name, [])
            report["jars"][name] = got
            if got != [want]:
                report["problems"].append(
                    f"WEB-INF/lib/{name}: expected {want} (proposal Risk 1), found {', '.join(got) or 'none'}")
        for name in ABSENT_JARS:
            got = versions.get(name, [])
            report["jars"][name] = got
            if got:
                report["problems"].append(f"WEB-INF/lib/{name}: expected absent, found {', '.join(got)}")
        for name in REPORTED_JARS:
            report["jars"][name] = versions.get(name, [])
        if report["application_files_in_lib"]:
            report["problems"].append(
                "WEB-INF/lib jars carry application files (config.oracle.001): "
                + ", ".join(report["application_files_in_lib"]))
        report["packaged"] = {}
        for f in PACKAGED_FILES:
            data = z.read(f"WEB-INF/classes/{f}")
            report["packaged"][f] = hashlib.sha256(data).hexdigest()
            committed = PACKAGED / f
            if not committed.exists() or committed.read_bytes() != data:
                PACKAGED.mkdir(exist_ok=True)
                committed.write_bytes(data)
                report["problems"].append(
                    f"packaged/{f} differed from the war's WEB-INF/classes/{f} and was rewritten; commit it")
        report["logback.xml"] = hashlib.sha256(z.read("WEB-INF/classes/logback.xml")).hexdigest()
    return report


# ----------------------------------------------------------------------------------------
# build
# ----------------------------------------------------------------------------------------

def build() -> str:
    docker("build", "-q", "-t", PROBE_TAG, str(HERE), capture=True, text=True)
    return docker("image", "inspect", PROBE_TAG, "--format", "{{.Id}}", text=True).stdout.strip()


# ----------------------------------------------------------------------------------------
# cases
# ----------------------------------------------------------------------------------------

def read_case(d: Path) -> dict:
    meta: dict[str, str] = {}
    for line in (d / "case.yaml").read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or ":" not in line:
            continue
        k, v = line.split(":", 1)
        meta[k.strip()] = v.strip()
    env: list[tuple[str, str]] = []
    if (d / "env").exists():
        for line in (d / "env").read_bytes().decode("utf-8").split("\n"):
            if line == "":
                continue
            name, value = line.split("=", 1) if not line.startswith("=") else ("", line[1:])
            env.append((name, value))
    args: list[str] = []
    if (d / "args").exists():
        args = [a for a in (d / "args").read_bytes().decode("utf-8").split("\n") if a != ""]
    return {"id": d.name, "meta": meta, "env": env, "args": args, "dir": d}


APPLICATION_MAIN = "ca.uhn.fhir.jpa.starter.Application"
# The safeguard: a real Application that has not exited after this many seconds has started
# the server, whether or not its start line was seen.
APPLICATION_TIMEOUT = int(os.environ.get("CONFIG_ROUNDTRIP_APP_TIMEOUT", "600"))
# A real Application that printed Spring Boot's start line (StartupInfoLogger, before the
# runners are called) and is still running this many seconds later has started the server.
APPLICATION_SETTLE = int(os.environ.get("CONFIG_ROUNDTRIP_APP_SETTLE", "15"))
STARTED_LINE = b"ca.uhn.fhir.jpa.starter.Application      : Started Application in "
# The real Application boots H2, JPA and Quartz: fewer of them at once than probe runs.
APPLICATION_SLOTS = threading.Semaphore(int(os.environ.get("CONFIG_ROUNDTRIP_APP_JOBS", "3")))
# Every container this script creates carries this label, so its own can be found and removed.
LABEL = "dev.frevo.config-roundtrip=1"


def wait_application(cid: str, timeout: int) -> tuple[int | None, str | None]:
    """Waits for a real Application: (exit status, None) if it exits, (None, how) if it starts."""
    deadline = time.monotonic() + timeout
    seen_at = None
    while True:
        running = docker("inspect", "-f", "{{.State.Running}}", cid, text=True).stdout.strip()
        if running != "true":
            return int(docker("wait", cid, text=True).stdout.strip()), None
        now = time.monotonic()
        if seen_at is None and STARTED_LINE in docker("logs", cid).stdout:
            seen_at = now
        if seen_at is not None and now - seen_at >= APPLICATION_SETTLE:
            return None, "start line"
        if now >= deadline:
            return None, "timeout"
        time.sleep(3)


def run_container(case: dict, image: str, timeout: int | None = None) -> dict:
    """One run of a case in a fresh container of `image`: exit, both streams, probe files.

    With `timeout` (the application runner), a run that starts the server is killed and
    reported as started, with `exit` None.
    """
    create = ["create", "--label", LABEL]
    for name, value in case["env"]:
        create += ["-e", f"{name}={value}"]
    create += [image, *case["args"]]
    cid = docker(*create, text=True).stdout.strip()
    started = None
    try:
        d: Path = case["dir"]
        if (d / "files").is_dir():
            docker("cp", f"{d / 'files'}{os.sep}.", f"{cid}:/app/")
        if (d / "root").is_dir():
            docker("cp", f"{d / 'root'}{os.sep}.", f"{cid}:/")
        docker("start", cid)
        if timeout is None:
            exit_code = int(docker("wait", cid, text=True).stdout.strip())
        else:
            exit_code, started = wait_application(cid, timeout)
            if started:
                docker("kill", cid, check=False)
                docker("wait", cid, text=True, check=False)
        logs = docker("logs", cid)
        stdout = logs.stdout.decode("utf-8", "replace")
        stderr = logs.stderr.decode("utf-8", "replace")
        document = None
        system_properties = None
        with tempfile.TemporaryDirectory() as tmp:
            r = docker("cp", f"{cid}:/tmp/config-roundtrip/.", tmp, check=False)
            if r.returncode == 0:
                p = Path(tmp) / "document.json"
                if p.exists():
                    document = json.loads(p.read_text(encoding="utf-8"))
                s = Path(tmp) / "system-properties.json"
                if s.exists():
                    system_properties = json.loads(s.read_text(encoding="utf-8"))
    finally:
        docker("rm", "-f", cid, check=False)
    out_lines = [l for l in stdout.split("\n") if not STATUS_LINE.match(l)]
    err_lines = [l for l in stderr.split("\n") if not STATUS_LINE.match(l)]
    while out_lines and out_lines[-1] == "":
        out_lines.pop()
    while err_lines and err_lines[-1] == "":
        err_lines.pop()
    # Java stack frames are never compared (config.oracle.003): counted, not kept.
    frames = sum(1 for l in out_lines if FRAME.match(l))
    out_lines = [l for l in out_lines if not FRAME.match(l)]
    return {
        "exit": exit_code,
        "started": started is not None,
        "start_signal": started,
        "document": document,
        "stdout": out_lines,
        "stderr": err_lines,
        "stack_frame_lines": frames,
        "logback_status_lines": {
            "stdout": sum(1 for l in stdout.split("\n") if STATUS_LINE.match(l)),
            "stderr": sum(1 for l in stderr.split("\n") if STATUS_LINE.match(l)),
        },
        "config_lines": config_lines(out_lines) if exit_code not in (0, None) else [],
        "_system_properties": system_properties,
    }


def run_case(case: dict) -> dict:
    """The probe runner, and for a case the probe fails, the application runner too."""
    covers = [c.strip() for c in case["meta"].get("covers", "").split(",") if c.strip()]
    probe = run_container(case, PROBE_TAG)
    probe.pop("started")
    probe.pop("start_signal")
    with APPLICATION_SLOTS:
        app = run_container(case, HAPI_IMAGE, timeout=APPLICATION_TIMEOUT)
    app.pop("_system_properties")
    started = app.pop("started")
    if probe["exit"] == 0:
        # The probe's run stays the case's record (document, reads, gates, properties). The
        # real Application's outcome is kept beside it: a probe exit 0 says the configuration
        # layer accepts the case, not that HAPI starts (review.md, fifth review, finding 1).
        outcome = {"main": APPLICATION_MAIN, "started": started, "start_signal": app["start_signal"],
                   "exit": app["exit"], "config_lines": app["config_lines"],
                   "stack_frame_lines": app["stack_frame_lines"], "stderr": app["stderr"]}
        if not started:
            outcome["stdout"] = app["stdout"]
        return {"covers": covers, "runner": "probe", **probe, "application": outcome}
    app.pop("start_signal")
    if started:
        # The real Application did not fail: the probe's failure is not HAPI's. Kept, flagged.
        return {"covers": covers, "runner": "probe", "application_started": True, **probe}
    probe_part = {k: probe[k] for k in ("exit", "stdout", "stderr", "stack_frame_lines", "config_lines")}
    return {"covers": covers, "runner": "application", "application_main": APPLICATION_MAIN,
            **app, "probe": probe_part, "_system_properties": probe["_system_properties"]}


# ----------------------------------------------------------------------------------------
# config.oracle.003: the config-related lines of a failure
# ----------------------------------------------------------------------------------------

LOG_LINE = re.compile(
    r"^(?P<ts>\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}(?:Z|[+-]\d\d:\d\d))\s+(?P<level>[A-Z]+) (?P<pid>\d*) --- "
    r"(?P<app>\[[^\]]*\] )?\[(?P<thread>[^\]]*)\] (?P<logger>.{40}) : (?P<msg>.*)$")
# A Java stack frame is tab-led only (review.md finding 2): `\tat ...`, `\t... N more`,
# `\t... N common frames omitted` (one more tab per level of a suppressed exception).
# Jackson's ` at [Source: REDACTED ...]` location line is led by a space: a message line, kept.
FRAME = re.compile(r"^\t+at |^\t+\.\.\. \d+ (more|common frames omitted)$")


def config_lines(lines: list[str]) -> list[str]:
    """The lines config.oracle.003 compares, with timestamp and PID masked.

    The WARN 'cancelling refresh attempt' line, the INFO ConditionEvaluationReportLogger line
    (with its message continuation), the ERROR line and everything after it, minus Java stack
    frames. Every line keeps its logger field: config.oracle.003 compares it (HAPI's real
    Application prints ConfigServletWebServerApplicationContext and
    .s.b.a.l.ConditionEvaluationReportLogger).
    """
    out: list[str] = []
    i = 0
    error_at = None
    while i < len(lines):
        m = LOG_LINE.match(lines[i])
        if m and m.group("level") == "ERROR" and (
                "LoggingFailureAnalysisReporter" in m.group("logger") or "o.s.boot.SpringApplication" in m.group("logger")):
            error_at = i
            break
        if m and m.group("level") == "WARN" and "cancelling refresh attempt" in m.group("msg"):
            out.append(mask(m))
        elif m and m.group("level") == "INFO" and "ConditionEvaluationReportLogger" in m.group("logger"):
            out.append(mask(m))
            j = i + 1
            while j < len(lines) and not LOG_LINE.match(lines[j]):
                out.append(lines[j])
                j += 1
            i = j
            continue
        i += 1
    if error_at is not None:
        out.append(mask(LOG_LINE.match(lines[error_at])))
        out += [l for l in lines[error_at + 1:] if not FRAME.match(l)]
    return out


def mask(m: re.Match) -> str:
    """A log line with its timestamp and PID masked; the logger field is kept."""
    app = m.group("app") or ""
    pid = "<pid>" if m.group("pid") else ""
    return f"<ts> {m.group('level'):>5} {pid} --- {app}[{m.group('thread')}] {m.group('logger')} : {m.group('msg')}"


# ----------------------------------------------------------------------------------------
# commands
# ----------------------------------------------------------------------------------------

def all_cases() -> list[dict]:
    return [read_case(d) for d in sorted(CASES.iterdir()) if (d / "case.yaml").exists()]


def run_all(cases: list[dict], jobs: int) -> dict[str, dict]:
    results: dict[str, dict] = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        futs = {pool.submit(run_case, c): c["id"] for c in cases}
        for n, f in enumerate(concurrent.futures.as_completed(futs), 1):
            cid = futs[f]
            results[cid] = f.result()
            r = results[cid]
            app = r.get("application")
            extra = "" if app is None else (
                f", application {'started' if app['started'] else 'exit ' + str(app['exit'])}")
            print(f"[{n}/{len(cases)}] {cid}: exit {r['exit']} ({r['runner']}){extra}", file=sys.stderr)
    return dict(sorted(results.items()))


def strip_random(doc, paths: list[str]):
    if doc is None:
        return None
    doc = json.loads(json.dumps(doc))
    for p in paths:
        prefix, _, rest = p.partition(":")
        if prefix == "reads":
            doc["reads"][rest] = "<random>"
            continue
        node = doc["properties"][prefix]
        keys = rest.split(".")
        for k in keys[:-1]:
            node = node[k]
        node[keys[-1]] = "<random>"
    return doc


def dump_record(record: dict) -> str:
    """Compact JSON, one case per line, so a re-record diffs case by case."""
    record = dict(record)
    cases = record.pop("cases")
    head = json.dumps(record, ensure_ascii=False, separators=(",", ":"))[:-1]
    lines = [head + ',"cases":{']
    items = list(cases.items())
    for i, (k, v) in enumerate(items):
        sep = "," if i < len(items) - 1 else ""
        lines.append(json.dumps(k) + ":" + json.dumps(v, ensure_ascii=False, separators=(",", ":")) + sep)
    lines.append("}}")
    return "\n".join(lines) + "\n"


def application_outcome(r: dict):
    """What the self-test compares of the real Application's run of a probe-passing case."""
    app = r.get("application")
    return None if app is None else (app["started"], app["exit"], app["config_lines"])


def cmd_record(diagnostic: bool, jobs: int, only: list[str] | None = None) -> int:
    report = inspect()
    if report["problems"] and not diagnostic:
        for p in report["problems"]:
            print(f"inspect: {p}", file=sys.stderr)
        print("inspect failed: not recording (rerun with --diagnostic for target/ evidence only)", file=sys.stderr)
        return 1
    probe = build()
    cases = all_cases()
    if only:
        cases = [c for c in cases if c["id"] in only]
    results = run_all(cases, jobs)
    # config.oracle.002: a second run of the whole corpus must give the same record.
    second = run_all(cases, jobs)
    differing = []
    for c in cases:
        rnd = [p.strip() for p in c["meta"].get("random", "").split(",") if p.strip()]
        a, b = results[c["id"]], second[c["id"]]
        if not (a["exit"] == b["exit"] and strip_random(a["document"], rnd) == strip_random(b["document"], rnd)
                and a["config_lines"] == b["config_lines"] and a["stderr"] == b["stderr"]
                and application_outcome(a) == application_outcome(b)):
            differing.append(c["id"])
    sysprops = None
    for cid, r in results.items():
        if cid == "config.oracle.001":
            sysprops = r["_system_properties"]
        r.pop("_system_properties")
    disagreements = [cid for cid, r in results.items() if r.get("application_started")]
    for cid in disagreements:
        print(f"record: {cid}: the probe fails but the real Application starts", file=sys.stderr)
    # The reverse direction: the probe starts the case, the real Application does not.
    application_failures = [cid for cid, r in results.items()
                            if r.get("application") and not r["application"]["started"]]
    for cid in application_failures:
        print(f"record: {cid}: the probe starts but the real Application fails", file=sys.stderr)
    record = {
        "comment": "Recorded by harness/config-roundtrip/run.py from the pinned HAPI image. Do not edit.",
        "image": HAPI_IMAGE,
        "probe_image": probe,
        "runners": {
            "probe": "the probe image, loader.main=dev.frevo.harness.config.ConfigRoundtrip: documents, "
                     "reads, gates and properties; every case runs it. A probe exit 0 means the "
                     "configuration layer accepts the case, not that HAPI starts",
            "application": f"the unchanged pinned image, its own Start-Class {APPLICATION_MAIN}: every "
                           "case runs it too. For a case the probe fails it is the recorded result (the "
                           "probe's run is kept under 'probe'); for a case the probe passes its outcome "
                           "is kept under 'application' (started, or exit and config lines). Started "
                           f"means Spring Boot's start line and still running {APPLICATION_SETTLE} s "
                           f"later, or still running after {APPLICATION_TIMEOUT} s; the run is then killed",
        },
        "runner_disagreements": disagreements,
        "application_failures": application_failures,
        "diagnostic": diagnostic,
        "inspection": report,
        "system_properties": sysprops,
        "self_test": {"runs": 2, "cases": len(cases), "differing": differing},
        "cases": results,
    }
    out = DIAGNOSTIC if diagnostic else RECORD
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(dump_record(record).encode("utf-8"))
    print(f"wrote {out}", file=sys.stderr)
    return 0


def cmd_self_test(diagnostic: bool, jobs: int) -> int:
    report = inspect()
    if report["problems"] and not diagnostic:
        for p in report["problems"]:
            print(f"inspect: {p}", file=sys.stderr)
        return 1
    build()
    cases = all_cases()
    a = run_all(cases, jobs)
    b = run_all(cases, jobs)
    bad = 0
    for c in cases:
        cid = c["id"]
        rnd = [p.strip() for p in c["meta"].get("random", "").split(",") if p.strip()]
        ra, rb = a[cid], b[cid]
        same = (ra["exit"] == rb["exit"]
                and strip_random(ra["document"], rnd) == strip_random(rb["document"], rnd)
                and ra["config_lines"] == rb["config_lines"]
                and application_outcome(ra) == application_outcome(rb))
        if not same:
            bad += 1
            print(f"self-test: {cid} differs between two runs", file=sys.stderr)
    print(f"self-test: {len(cases) - bad}/{len(cases)} cases identical", file=sys.stderr)
    return 1 if bad else 0


def main(argv: list[str]) -> int:
    diagnostic = "--diagnostic" in argv
    argv = [a for a in argv if a != "--diagnostic"]
    jobs = int(os.environ.get("CONFIG_ROUNDTRIP_JOBS", "6"))
    if not argv:
        print(__doc__)
        return 2
    cmd = argv[0]
    if cmd == "inspect":
        r = inspect()
        print(json.dumps(r, indent=1))
        return 1 if r["problems"] else 0
    if cmd == "build":
        print(build())
        return 0
    if cmd == "record":
        return cmd_record(diagnostic, jobs, argv[1:] or None)
    if cmd == "self-test":
        return cmd_self_test(diagnostic, jobs)
    if cmd == "schema":
        build()
        r = docker("run", "--rm", "--entrypoint", "java", PROBE_TAG, "--class-path", "/app/main.war",
                   "-Dloader.path=main.war!/WEB-INF/classes/,main.war!/WEB-INF/,/app/extra-classes",
                   "-Dloader.main=dev.frevo.harness.config.Schema",
                   "org.springframework.boot.loader.PropertiesLauncher", text=True)
        print(r.stdout, end="")
        return 0
    if cmd == "case":
        build()
        for cid in argv[1:]:
            r = run_case(read_case(CASES / cid))
            r.pop("_system_properties")
            print(json.dumps(r, indent=1, ensure_ascii=False))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
