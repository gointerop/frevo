//! Shared harness for the `config-roundtrip` tests of `add-config-application-yaml`.
//!
//! Every config case lives in `harness/config-roundtrip/cases/<id>/` (written by
//! `harness/config-roundtrip/make_cases.py`). A test runs `frevo-config effective` on a
//! case, the way the HAPI side ran it inside the pinned image (`run.py`):
//!
//! - a fresh temporary file-system root `<tmp>/root/`, with the case's `root/` copied
//!   into it, and the working directory `<tmp>/root/app/` (the image's `WORKDIR /app`)
//!   with the case's `files/` copied into it;
//! - a cleared environment plus the case's `env`, plus `FREVO_CONFIG_TEST_ROOT=<tmp>/root`
//!   (see below);
//! - the case's `args` after `effective`.
//!
//! It then compares the result with the recorded HAPI result in
//! `harness/config-roundtrip/hapi-8.12.0-results.json` (config.oracle.003, .004), and
//! the individual test asserts the scenario's THEN clauses on top.
//!
//! `FREVO_CONFIG_TEST_ROOT`: absolute paths (`/app/extra-classes/`, `/app/config/…`,
//! `/configs/…`) cannot be created by a test on a developer machine or a CI runner. The
//! tests therefore ask `frevo-config` to resolve every absolute path beneath this
//! directory. It is a test seam, not a configuration key: it is the implementer's to
//! accept or replace (tasks/blockers.md, add-config-application-yaml).
//!
//! The binary is found through `CARGO_BIN_EXE_frevo-config`, which Cargo defines for
//! integration tests once the crate has a `frevo-config` binary target
//! (config.dump.001). Until then every test fails with that message.

#![allow(dead_code)]

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The repository root.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

pub fn harness() -> PathBuf {
    repo().join("harness/config-roundtrip")
}

/// The nine bean prefixes of config.dump.002.
pub const PREFIXES: [&str; 9] = [
    "hapi.fhir",
    "hapi.fhir.cdshooks",
    "hapi.fhir.cr",
    "hapi.fhir.cr.caregaps",
    "hapi.fhir.cr.cql",
    "hapi.fhir.cr.cql.compiler",
    "hapi.fhir.cr.cql.runtime",
    "hapi.fhir.cr.cql.terminology",
    "hapi.fhir.cr.cql.data",
];

/// The thirteen reads of config.read.001.
pub const READS: [&str; 13] = [
    "hapi.fhir.fhir_version",
    "hapi.fhir.cr.enabled",
    "hapi.fhir.cdshooks.enabled",
    "hapi.fhir.mdm_enabled",
    "hapi.fhir.ips_enabled",
    "hapi.fhir.ig_runtime_upload_enabled",
    "hapi.fhir.binary_storage_mode",
    "hapi.fhir.custom_content_path",
    "hapi.fhir.app_content_path",
    "hapi.fhir.enable_repository_validating_interceptor",
    "hapi.fhir.custom-bean-packages",
    "spring.ai.mcp.server.enabled",
    "scheduling_disabled",
];

/// The twenty-five gates of config.gate.001.
pub const GATES: [&str; 25] = [
    "fhir-dstu2",
    "fhir-dstu3",
    "fhir-r4",
    "fhir-r4b",
    "fhir-r5",
    "fhir-any",
    "cr",
    "cdshooks",
    "mdm",
    "ips",
    "ig-runtime-upload",
    "repository-validating-interceptor",
    "binary-storage-filesystem",
    "binary-storage-database",
    "custom-content",
    "app-content",
    "mcp",
    "mcp-cdshooks",
    "cors",
    "implementation-guides",
    "partitioning",
    "remote-terminology",
    "tester",
    "elasticsearch",
    "scheduling-disabled",
];

// ---------------------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Case {
    pub id: String,
    pub dir: PathBuf,
    pub covers: Vec<String>,
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub random: Vec<String>,
    pub process: Vec<String>,
    pub deviation: Option<String>,
}

pub fn case(id: &str) -> Case {
    let dir = harness().join("cases").join(id);
    let meta = std::fs::read_to_string(dir.join("case.yaml")).unwrap_or_else(|e| {
        panic!(
            "case {id}: cannot read {}: {e}",
            dir.join("case.yaml").display()
        )
    });
    let mut fields = BTreeMap::new();
    for line in meta.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            fields.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    let list = |k: &str| -> Vec<String> {
        fields
            .get(k)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut env = Vec::new();
    if let Ok(bytes) = std::fs::read(dir.join("env")) {
        for line in String::from_utf8(bytes).expect("env is UTF-8").split('\n') {
            if line.is_empty() {
                continue;
            }
            let (n, v) = line.split_once('=').expect("NAME=value");
            env.push((n.to_string(), v.to_string()));
        }
    }
    let mut args = Vec::new();
    if let Ok(bytes) = std::fs::read(dir.join("args")) {
        for line in String::from_utf8(bytes)
            .expect("args are UTF-8")
            .split('\n')
        {
            if !line.is_empty() {
                args.push(line.to_string());
            }
        }
    }
    Case {
        id: id.to_string(),
        covers: list("covers"),
        random: list("random"),
        process: list("process"),
        deviation: fields.get("deviation").cloned(),
        env,
        args,
        dir,
    }
}

/// Every case directory name.
pub fn all_case_ids() -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(harness().join("cases"))
        .expect("harness/config-roundtrip/cases")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("case.yaml").exists())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

// ---------------------------------------------------------------------------------------
// Running frevo-config effective
// ---------------------------------------------------------------------------------------

/// The `frevo-config` binary, or a failure naming the missing capability.
pub fn frevo_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("FREVO_CONFIG_TEST_BIN") {
        return PathBuf::from(p); // the oracle self-check (harness/config-roundtrip/README.md)
    }
    match option_env!("CARGO_BIN_EXE_frevo-config") {
        Some(p) => PathBuf::from(p),
        None => panic!(
            "frevo-config has no `frevo-config` binary target: `frevo-config effective` \
             (config.dump.001) is not implemented"
        ),
    }
}

#[derive(Debug)]
pub struct Run {
    pub case: Case,
    pub exit: i32,
    pub stdout: String,
    pub stderr: String,
    /// The parsed document when the run exited 0 and printed one.
    pub doc: Option<Value>,
    /// The working directory the run had (`<tmp>/root/app`).
    pub workdir: PathBuf,
    pub pid: u32,
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn copy_tree(from: &Path, to: &Path) {
    if !from.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(from).expect("read case directory") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            std::fs::create_dir_all(&target).expect("mkdir");
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::create_dir_all(to).expect("mkdir");
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// Runs `frevo-config effective` on a case.
pub fn run(id: &str) -> Run {
    run_case(case(id))
}

pub fn run_case(case: Case) -> Run {
    let bin = frevo_bin();
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let base = std::env::temp_dir()
        .join("frevo-config-tests")
        .join(format!("{}-{}-{}", case.id, std::process::id(), n));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("root");
    let workdir = root.join("app");
    std::fs::create_dir_all(&workdir).expect("create working directory");
    copy_tree(&case.dir.join("root"), &root);
    copy_tree(&case.dir.join("files"), &workdir);

    let mut cmd = Command::new(&bin);
    cmd.arg("effective")
        .args(&case.args)
        .current_dir(&workdir)
        .env_clear()
        .env("FREVO_CONFIG_TEST_ROOT", &root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &case.env {
        cmd.env(k, v);
    }
    if std::env::var_os("FREVO_CONFIG_TEST_BIN").is_some() {
        // the replay stand-in of the oracle self-check needs to know what to replay
        cmd.env("FREVO_CONFIG_REPLAY_CASE", &case.id)
            .env("FREVO_CONFIG_REPLAY_RECORD", record_path());
    }
    let child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {}: {e}", bin.display()));
    let pid = child.id();
    let out = child.wait_with_output().expect("wait for frevo-config");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    let exit = out.status.code().unwrap_or(-1);
    let doc = if exit == 0 {
        Some(serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "case {}: exit 0 but standard output is not one JSON document ({e}):\n{stdout}",
                case.id
            )
        }))
    } else {
        None
    };
    Run {
        case,
        exit,
        stdout,
        stderr,
        doc,
        workdir,
        pid,
    }
}

// ---------------------------------------------------------------------------------------
// The recorded HAPI side
// ---------------------------------------------------------------------------------------

/// The record the tests compare against: `hapi-8.12.0-results.json`, or the file named by
/// `FREVO_CONFIG_TEST_RECORD` for the oracle self-check against a diagnostic record.
pub fn record_path() -> PathBuf {
    std::env::var_os("FREVO_CONFIG_TEST_RECORD")
        .map(PathBuf::from)
        .unwrap_or_else(|| harness().join("hapi-8.12.0-results.json"))
}

pub fn record() -> &'static Value {
    static RECORD: OnceLock<Value> = OnceLock::new();
    RECORD.get_or_init(|| {
        let p = record_path();
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
            panic!(
                "no recorded HAPI result: {} cannot be read ({e}); recording is blocked, see \
                 tasks/blockers.md (add-config-application-yaml)",
                p.display()
            )
        });
        serde_json::from_str(&text).expect("hapi-8.12.0-results.json is JSON")
    })
}

pub fn hapi(id: &str) -> &'static Value {
    record()["cases"]
        .get(id)
        .unwrap_or_else(|| panic!("hapi-8.12.0-results.json has no case {id}; rerun run.py record"))
}

// ---------------------------------------------------------------------------------------
// config.oracle.003 / .004: the comparison
// ---------------------------------------------------------------------------------------

/// Paths of `Set` properties (sorted before comparing, config.oracle.004). `*` is any map key.
pub const SET_PATHS: [&str; 4] = [
    "hapi.fhir:auto-version-reference-at-paths",
    "hapi.fhir:local-base-urls",
    "hapi.fhir:logical-urls",
    "hapi.fhir:implementation-guides.*.additional-resource-folders",
];

fn sort_sets(doc: &mut Value) {
    for p in SET_PATHS {
        let (prefix, rest) = p.split_once(':').unwrap();
        let parts: Vec<&str> = rest.split('.').collect();
        if let Some(node) = doc.get_mut("properties").and_then(|v| v.get_mut(prefix)) {
            sort_at(node, &parts);
        }
    }
}

fn sort_at(node: &mut Value, parts: &[&str]) {
    if parts.is_empty() {
        if let Value::Array(a) = node {
            a.sort_by(|x, y| x.as_str().unwrap_or("").cmp(y.as_str().unwrap_or("")));
        }
        return;
    }
    if parts[0] == "*" {
        if let Value::Object(m) = node {
            for v in m.values_mut() {
                sort_at(v, &parts[1..]);
            }
        }
    } else if let Some(v) = node.get_mut(parts[0]) {
        sort_at(v, &parts[1..]);
    }
}

fn mask_path(doc: &mut Value, p: &str, with: &str) {
    let (prefix, rest) = p.split_once(':').expect("prefix:path");
    if prefix == "reads" {
        if let Some(v) = doc.get_mut("reads").and_then(|r| r.get_mut(rest)) {
            assert!(v.is_string(), "{p}: a masked read must be text, got {v}");
            *v = Value::String(with.to_string());
        }
        return;
    }
    let mut node = match doc.get_mut("properties").and_then(|v| v.get_mut(prefix)) {
        Some(n) => n,
        None => return,
    };
    let parts: Vec<&str> = rest.split('.').collect();
    for k in &parts[..parts.len() - 1] {
        node = match node.get_mut(*k) {
            Some(n) => n,
            None => return,
        };
    }
    if let Value::Object(m) = node {
        if let Some(v) = m.get_mut(parts[parts.len() - 1]) {
            assert!(
                v.is_string(),
                "{p}: a masked value must be a string, got {v}"
            );
            *v = Value::String(with.to_string());
        }
    }
}

/// One log line of the console pattern: timestamp, level, pid, `---`, app, thread, logger, message.
pub struct LogLine<'a> {
    pub level: &'a str,
    pub pid: &'a str,
    pub app: &'a str,
    pub thread: &'a str,
    pub logger: &'a str,
    pub message: &'a str,
}

pub fn parse_log_line(line: &str) -> Option<LogLine<'_>> {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(
            r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}(?:Z|[+-]\d\d:\d\d)\s+([A-Z]+) (\d*) --- (\[[^\]]*\] )?\[([^\]]*)\] (.{40}) : (.*)$",
        )
        .unwrap()
    });
    let c = re.captures(line)?;
    Some(LogLine {
        level: c.get(1).unwrap().as_str(),
        pid: c.get(2).unwrap().as_str(),
        app: c.get(3).map(|m| m.as_str()).unwrap_or(""),
        thread: c.get(4).unwrap().as_str(),
        logger: c.get(5).unwrap().as_str(),
        message: c.get(6).unwrap().as_str(),
    })
}

fn is_frame(line: &str) -> bool {
    let t = line.trim_start();
    (line.starts_with(char::is_whitespace) && t.starts_with("at "))
        || (line.starts_with(char::is_whitespace)
            && t.starts_with("... ")
            && (t.ends_with(" more") || t.ends_with("omitted")))
}

fn masked(l: &LogLine<'_>, keep_logger: bool) -> String {
    format!(
        "<ts> {:>5} {} --- {}[{}] {} : {}",
        l.level,
        if l.pid.is_empty() { "" } else { "<pid>" },
        l.app,
        l.thread,
        if keep_logger { l.logger } else { "<logger>" },
        l.message
    )
}

/// The config-related lines of a failing start (config.oracle.003), masked, and the lines
/// that were not config-related.
pub fn config_lines(stdout: &str) -> (Vec<String>, Vec<String>) {
    let lines: Vec<&str> = stdout.split('\n').collect();
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(l) = parse_log_line(line) {
            if l.level == "ERROR"
                && (l.logger.contains("LoggingFailureAnalysisReporter")
                    || l.logger.contains("o.s.boot.SpringApplication"))
            {
                kept.push(masked(&l, true));
                for rest in &lines[i + 1..] {
                    if is_frame(rest) {
                        dropped.push(rest.to_string());
                    } else {
                        kept.push(rest.to_string());
                    }
                }
                while kept.last().map(|s| s.is_empty()).unwrap_or(false) {
                    kept.pop();
                }
                return (kept, dropped);
            }
            if l.level == "WARN" && l.message.contains("cancelling refresh attempt") {
                kept.push(masked(&l, false));
                i += 1;
                continue;
            }
            if l.level == "INFO" && l.logger.contains("ConditionEvaluationReportLogger") {
                kept.push(masked(&l, false));
                i += 1;
                while i < lines.len() && parse_log_line(lines[i]).is_none() {
                    kept.push(lines[i].to_string());
                    i += 1;
                }
                continue;
            }
        }
        if !line.is_empty() {
            dropped.push(line.to_string());
        }
        i += 1;
    }
    (kept, dropped)
}

/// The JVM-option notices of config.source.023, in order.
pub fn jvm_notices(stderr_lines: impl IntoIterator<Item = String>) -> Vec<String> {
    stderr_lines
        .into_iter()
        .filter(|l| {
            l.starts_with("NOTE: Picked up JDK_JAVA_OPTIONS: ")
                || l.starts_with("Picked up JAVA_TOOL_OPTIONS: ")
        })
        .collect()
}

fn lines_of(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("array of lines")
        .iter()
        .map(|l| l.as_str().expect("line").to_string())
        .collect()
}

/// Runs the case and asserts it matches the recorded HAPI result: config.oracle.003 for a
/// failing start, config.oracle.004 for a document. Returns the run for further assertions.
pub fn parity(id: &str) -> Run {
    let r = run(id);
    assert_parity(&r);
    r
}

pub fn assert_parity(r: &Run) {
    assert_parity_masked(r, &[]);
}

/// [`assert_parity`] with extra text masks applied to both sides' failure lines. Only for
/// text the HAPI side itself does not reproduce between runs (a Java identity hash); every
/// use names its finding in tasks/blockers.md.
pub fn parity_masked(id: &str, masks: &[(&str, &str)]) -> Run {
    let r = run(id);
    assert_parity_masked(&r, masks);
    r
}

pub fn assert_parity_masked(r: &Run, masks: &[(&str, &str)]) {
    let id = &r.case.id;
    let h = hapi(id);
    let hapi_exit = h["exit"].as_i64().expect("exit");
    assert_eq!(
        i64::from(r.exit),
        hapi_exit,
        "case {id}: exit status differs from HAPI's\nfrevo stdout:\n{}\nfrevo stderr:\n{}",
        r.stdout,
        r.stderr
    );
    // Standard error: the JVM-option notices HAPI's JVM printed, and nothing else.
    let hapi_notices = jvm_notices(lines_of(&h["stderr"]));
    let frevo_err: Vec<String> = r.stderr.lines().map(str::to_string).collect();
    assert_eq!(
        frevo_err, hapi_notices,
        "case {id}: standard error differs (config.source.023, config.error.007)"
    );
    if hapi_exit == 0 {
        let mut ours = r.doc.clone().expect("document");
        let mut theirs = h["document"].clone();
        sort_sets(&mut ours);
        sort_sets(&mut theirs);
        for p in &r.case.random {
            mask_path(&mut ours, p, "<random>");
            mask_path(&mut theirs, p, "<random>");
        }
        for p in &r.case.process {
            mask_path(&mut ours, p, "<process>");
            mask_path(&mut theirs, p, "<process>");
        }
        if ours != theirs {
            panic!(
                "case {id}: document differs from HAPI's\n{}",
                diff(&theirs, &ours)
            );
        }
    } else {
        let hapi_stdout = lines_of(&h["stdout"]).join("\n");
        let (theirs, _) = config_lines(&hapi_stdout);
        let (ours, extra) = config_lines(&r.stdout);
        let apply = |lines: Vec<String>| -> Vec<String> {
            lines
                .into_iter()
                .map(|l| {
                    masks.iter().fold(l, |acc, (pattern, with)| {
                        regex::Regex::new(pattern)
                            .expect("mask pattern")
                            .replace_all(&acc, *with)
                            .into_owned()
                    })
                })
                .collect()
        };
        let (theirs, ours) = (apply(theirs), apply(ours));
        assert!(
            !theirs.is_empty(),
            "case {id}: HAPI's record has no config-related failure line"
        );
        assert_eq!(
            ours, theirs,
            "case {id}: config-related failure lines differ (config.oracle.003)"
        );
        assert!(
            extra.is_empty(),
            "case {id}: frevo wrote lines outside the compared set (config.error.009): {extra:#?}"
        );
    }
}

/// A readable list of the paths where two documents differ.
pub fn diff(expected: &Value, actual: &Value) -> String {
    let mut out = Vec::new();
    diff_at("", expected, actual, &mut out);
    out.join("\n")
}

fn diff_at(path: &str, e: &Value, a: &Value, out: &mut Vec<String>) {
    match (e, a) {
        (Value::Object(me), Value::Object(ma)) => {
            for (k, ve) in me {
                match ma.get(k) {
                    Some(va) => diff_at(&format!("{path}/{k}"), ve, va, out),
                    None => out.push(format!("{path}/{k}: HAPI {ve}, frevo absent")),
                }
            }
            for k in ma.keys() {
                if !me.contains_key(k) {
                    out.push(format!("{path}/{k}: HAPI absent, frevo {}", ma[k]));
                }
            }
        }
        _ if e != a => out.push(format!("{path}: HAPI {e}, frevo {a}")),
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------
// Assertions on one run
// ---------------------------------------------------------------------------------------

/// Splits a spec path (`hapi.fhir.cr.cql.namespaces[hl7.fhir.uv.cql]`) into the longest
/// bean prefix and the member path within it.
pub fn split_path(path: &str) -> (&'static str, Vec<String>) {
    let prefix = PREFIXES
        .iter()
        .filter(|p| {
            path == **p || path.starts_with(&format!("{p}.")) || path.starts_with(&format!("{p}["))
        })
        .max_by_key(|p| p.len())
        .unwrap_or_else(|| panic!("{path} is under none of the nine bean prefixes"));
    let rest = &path[prefix.len()..];
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut in_bracket = false;
    for ch in rest.chars() {
        match ch {
            '[' if !in_bracket => {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
                in_bracket = true;
            }
            ']' if in_bracket => {
                parts.push(std::mem::take(&mut cur));
                in_bracket = false;
            }
            '.' if !in_bracket => {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    (prefix, parts)
}

impl Run {
    pub fn doc(&self) -> &Value {
        self.doc.as_ref().unwrap_or_else(|| {
            panic!(
                "case {}: expected a document, frevo-config exited {}\nstdout:\n{}\nstderr:\n{}",
                self.case.id, self.exit, self.stdout, self.stderr
            )
        })
    }

    /// The value at a spec path, `Value::Null` if absent along the way.
    pub fn get(&self, path: &str) -> Value {
        let (prefix, parts) = split_path(path);
        let mut node = &self.doc()["properties"][prefix];
        for p in &parts {
            node = match node.get(p.as_str()) {
                Some(n) => n,
                None => return Value::Null,
            };
        }
        node.clone()
    }

    pub fn has(&self, path: &str) -> bool {
        let (prefix, parts) = split_path(path);
        let mut node = &self.doc()["properties"][prefix];
        for p in &parts {
            node = match node.get(p.as_str()) {
                Some(n) => n,
                None => return false,
            };
        }
        true
    }

    /// Asserts "the effective configuration reports P = V".
    pub fn expect(&self, path: &str, value: Value) -> &Self {
        assert!(
            self.has(path),
            "case {}: {path} is not in the document",
            self.case.id
        );
        assert_eq!(self.get(path), value, "case {}: {path}", self.case.id);
        self
    }

    pub fn read(&self, name: &str) -> Value {
        self.doc()["reads"]
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no read {name}"))
    }

    pub fn expect_read(&self, name: &str, value: Value) -> &Self {
        assert_eq!(self.read(name), value, "case {}: read {name}", self.case.id);
        self
    }

    pub fn gate(&self, name: &str) -> bool {
        self.doc()["gates"]
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or_else(|| panic!("no gate {name}"))
    }

    pub fn expect_gate(&self, name: &str, on: bool) -> &Self {
        assert_eq!(self.gate(name), on, "case {}: gate {name}", self.case.id);
        self
    }

    pub fn gates_on(&self) -> Vec<String> {
        let g = self.doc()["gates"].as_object().expect("gates");
        g.iter()
            .filter(|(_, v)| v.as_bool() == Some(true))
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn active(&self) -> Vec<String> {
        strings(&self.doc()["profiles"]["active"])
    }

    pub fn default_profiles(&self) -> Vec<String> {
        strings(&self.doc()["profiles"]["default"])
    }

    /// Asserts "fails to start": exit 1, no document on standard output.
    pub fn expect_failure(&self) -> &Self {
        assert_eq!(
            self.exit, 1,
            "case {}: expected a failed start\nstdout:\n{}",
            self.case.id, self.stdout
        );
        assert!(
            serde_json::from_str::<Value>(&self.stdout).is_err(),
            "case {}: a failed start printed a JSON document",
            self.case.id
        );
        self
    }

    /// Asserts that standard output carries this exact line.
    pub fn expect_line(&self, line: &str) -> &Self {
        assert!(
            self.stdout.lines().any(|l| l == line),
            "case {}: standard output has no line {line:?}\nstdout:\n{}",
            self.case.id,
            self.stdout
        );
        self
    }

    /// Asserts that some standard-output line contains the text.
    pub fn expect_text(&self, text: &str) -> &Self {
        assert!(
            self.stdout.contains(text),
            "case {}: standard output does not contain {text:?}\nstdout:\n{}",
            self.case.id,
            self.stdout
        );
        self
    }

    pub fn expect_no_text(&self, text: &str) -> &Self {
        assert!(
            !self.stdout.contains(text),
            "case {}: standard output contains {text:?}",
            self.case.id
        );
        self
    }

    /// The config-related lines of this run's failure output (config.oracle.003).
    pub fn failure_lines(&self) -> Vec<String> {
        config_lines(&self.stdout).0
    }

    /// Asserts the failure-analysis form of config.error.002 with these report lines.
    pub fn expect_analysis(&self, description: &str, report: &[&str]) -> &Self {
        self.expect_failure();
        let l = self.failure_lines();
        let at = l
            .iter()
            .position(|x| {
                x.contains("o.s.b.d.LoggingFailureAnalysisReporter") && x.contains(" ERROR ")
            })
            .unwrap_or_else(|| {
                panic!(
                    "case {}: no failure-analysis ERROR line in {l:#?}",
                    self.case.id
                )
            });
        let block = &l[at + 1..];
        let head = [
            "",
            "***************************",
            "APPLICATION FAILED TO START",
            "***************************",
            "",
            "Description:",
            "",
        ];
        assert_eq!(
            &block[..head.len()],
            &head.map(String::from)[..],
            "case {}: analysis header",
            self.case.id
        );
        assert_eq!(
            block[head.len()],
            description,
            "case {}: description",
            self.case.id
        );
        for line in report {
            assert!(
                block.iter().any(|b| b == line),
                "case {}: analysis lacks {line:?}\n{block:#?}",
                self.case.id
            );
        }
        self
    }

    /// Asserts the run-failed form of config.error.007 whose exception chain has these lines.
    pub fn expect_run_failed(&self, exception_lines: &[&str]) -> &Self {
        self.expect_failure();
        let l = self.failure_lines();
        let at = l
            .iter()
            .position(|x| {
                x.contains("o.s.boot.SpringApplication") && x.ends_with(": Application run failed")
            })
            .unwrap_or_else(|| {
                panic!(
                    "case {}: no `Application run failed` line in {l:#?}",
                    self.case.id
                )
            });
        let chain = &l[at + 1..];
        for line in exception_lines {
            assert!(
                chain.iter().any(|c| c == line),
                "case {}: exception chain lacks {line:?}\n{chain:#?}",
                self.case.id
            );
        }
        self
    }
}

pub fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|s| s.as_str().expect("string").to_string())
        .collect()
}

/// The document's `properties` member for one prefix.
pub fn bean<'a>(doc: &'a Value, prefix: &str) -> &'a Map<String, Value> {
    doc["properties"][prefix]
        .as_object()
        .unwrap_or_else(|| panic!("properties.{prefix} is not an object"))
}
