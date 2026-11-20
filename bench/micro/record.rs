//! Shared recording for the `bench/micro/` workloads.
//!
//! Three bench binaries write rows into the same two files --
//! `crates/frevo-model/benches/model_micro.rs` (the `model-*` rows),
//! `crates/frevo-model/benches/path_bytes.rs` (`path-eval-compiled`,
//! `path-extract-patient`) and `crates/frevo-path/benches/path_micro.rs`
//! (`path-compile`, `path-suite-wallclock`). Each
//! includes this file with `#[path]`, so there is one writer implementation
//! and no dependency.
//!
//! A writer **replaces only the rows it owns** and keeps every other row
//! byte-for-byte, so running one bench never drops another's measurement.
//! Every row names the bench, the commit and the time it was measured at,
//! because the rows of one file can come from different runs; the top-level
//! `commit` is the commit of the last write.
//!
//! The file is written one row per line, and that layout is what the merge
//! reads back: a file in any other layout is refused rather than guessed at.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// The prefix every row line carries.
const ROW_PREFIX: &str = "    { \"name\": ";

pub fn git(args: &[&str], repo: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the system clock, without a date dependency.
pub fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Howard Hinnant's civil_from_days.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One recorded row: its name and the rest of its members, each already a
/// JSON literal (`("median_us", "12.345")`, `("scenario", "\"x\"")`).
pub struct Row {
    pub name: String,
    pub members: Vec<(&'static str, String)>,
}

impl Row {
    pub fn new(name: &str) -> Self {
        Row {
            name: name.to_string(),
            members: Vec::new(),
        }
    }

    pub fn str(mut self, key: &'static str, value: &str) -> Self {
        self.members.push((key, json_string(value)));
        self
    }

    pub fn num(mut self, key: &'static str, value: f64, decimals: usize) -> Self {
        self.members.push((key, format!("{value:.decimals$}")));
        self
    }

    pub fn int(mut self, key: &'static str, value: u64) -> Self {
        self.members.push((key, value.to_string()));
        self
    }

    fn line(&self, bench: &str, commit: &str, dirty: bool, at: &str) -> String {
        let mut s = format!("{ROW_PREFIX}{}", json_string(&self.name));
        for (key, value) in &self.members {
            let _ = write!(s, ", \"{key}\": {value}");
        }
        let _ = write!(
            s,
            ", \"bench\": {}, \"commit\": {}, \"worktree_dirty\": {dirty}, \"measured_at\": {} }}",
            json_string(bench),
            json_string(commit),
            json_string(at)
        );
        s
    }
}

/// The row name a line written by [`Row::line`] carries.
fn row_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(ROW_PREFIX)?.strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// The row lines of an existing file, refusing a file this module did not
/// write.
fn existing_rows(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    let mut in_workloads = false;
    for line in text.lines() {
        if line.trim_start().starts_with("\"workloads\": [") {
            in_workloads = true;
            continue;
        }
        if !in_workloads {
            continue;
        }
        if line.trim() == "]" {
            break;
        }
        let row = line.trim_end().trim_end_matches(',');
        assert!(
            row_name(row).is_some(),
            "{} is not in the one-row-per-line layout bench/micro/record.rs \
             writes; refusing to merge into it: {line:?}",
            path.display()
        );
        rows.push(row.to_string());
    }
    rows
}

/// Write `rows` into `path`, replacing the rows whose names `owns` claims and
/// keeping every other row as it was.
pub fn write_merged(
    path: &Path,
    repo: &Path,
    bench: &str,
    owns: &dyn Fn(&str) -> bool,
    rows: &[Row],
) {
    let commit = git(&["rev-parse", "HEAD"], repo).unwrap_or_else(|| "unknown".to_string());
    let dirty = git(
        &[
            "status",
            "--porcelain",
            "--",
            "crates",
            "bench/micro",
            ":(exclude)bench/micro/results.json",
            ":(exclude)bench/micro/baseline.json",
        ],
        repo,
    )
    .map(|s| !s.is_empty())
    .unwrap_or(true);
    let at = utc_now();
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);

    let mut lines: Vec<String> = existing_rows(path)
        .into_iter()
        .filter(|line| row_name(line).is_some_and(|name| !owns(name)))
        .collect();
    for row in rows {
        assert!(
            owns(&row.name),
            "{bench} writes a row it does not own: {}",
            row.name
        );
        lines.push(row.line(bench, &commit, dirty, &at));
    }

    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"label\": \"frevo bench/micro (crates/frevo-model/benches/model_micro.rs, crates/frevo-model/benches/path_bytes.rs, crates/frevo-path/benches/path_micro.rs)\",\n");
    s.push_str("  \"profile\": \"release\",\n");
    let _ = writeln!(s, "  \"commit\": {},", json_string(&commit));
    let _ = writeln!(s, "  \"worktree_dirty\": {dirty},");
    let _ = writeln!(s, "  \"generated_at\": {},", json_string(&at));
    let _ = writeln!(
        s,
        "  \"environment\": {{ \"os\": {}, \"arch\": {}, \"cpus\": {cpus} }},",
        json_string(std::env::consts::OS),
        json_string(std::env::consts::ARCH)
    );
    s.push_str("  \"method\": \"see bench/micro/README.md; each row names the bench that measured it and its own method\",\n");
    s.push_str("  \"workloads\": [\n");
    for (i, line) in lines.iter().enumerate() {
        s.push_str(line);
        s.push_str(if i + 1 < lines.len() { ",\n" } else { "\n" });
    }
    s.push_str("  ]\n}\n");
    std::fs::write(path, s).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}
