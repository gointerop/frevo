//! Replays a recorded HAPI result as if it were `frevo-config effective`.
//!
//! The test harness runs this instead of the real binary when `FREVO_CONFIG_TEST_BIN`
//! names it, and tells it the case through `FREVO_CONFIG_REPLAY_CASE` and the record through
//! `FREVO_CONFIG_REPLAY_RECORD`. It prints the recorded document (exit 0), or the recorded
//! failure output with Java stack frames and non-config log lines removed (exit 1), and the
//! recorded JVM-option notices on standard error. Every test that then fails is a test that
//! disagrees with HAPI, or a scenario with no HAPI counterpart.
#![forbid(unsafe_code)]

use std::io::Write;

fn main() {
    let case = std::env::var("FREVO_CONFIG_REPLAY_CASE").expect("FREVO_CONFIG_REPLAY_CASE");
    let record = std::env::var("FREVO_CONFIG_REPLAY_RECORD").expect("FREVO_CONFIG_REPLAY_RECORD");
    let rec: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(record).expect("record")).expect("JSON");
    let c = &rec["cases"][&case];
    let mut err = std::io::stderr();
    for l in c["stderr"].as_array().unwrap() {
        let l = l.as_str().unwrap();
        if l.starts_with("NOTE: Picked up JDK_JAVA_OPTIONS: ") || l.starts_with("Picked up JAVA_TOOL_OPTIONS: ") {
            writeln!(err, "{l}").unwrap();
        }
    }
    let exit = c["exit"].as_i64().unwrap();
    let mut out = std::io::stdout();
    if exit == 0 {
        write!(out, "{}", serde_json::to_string_pretty(&c["document"]).unwrap()).unwrap();
        std::process::exit(0);
    }
    let lines: Vec<&str> = c["stdout"].as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect();
    let is_log = |l: &str| l.len() > 24 && l.as_bytes()[4] == b'-' && l.contains(" --- [");
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if is_log(l) && l.contains(" ERROR ") && (l.contains("LoggingFailureAnalysisReporter") || l.contains("o.s.boot.SpringApplication")) {
            for r in &lines[i..] {
                let t = r.trim_start();
                let frame = r.starts_with(char::is_whitespace) && (t.starts_with("at ") || (t.starts_with("... ") && (t.ends_with(" more") || t.ends_with("omitted"))));
                if !frame {
                    writeln!(out, "{r}").unwrap();
                }
            }
            break;
        }
        if is_log(l) && l.contains(" WARN ") && l.contains("cancelling refresh attempt") {
            writeln!(out, "{l}").unwrap();
        } else if is_log(l) && l.contains(" INFO ") && l.contains("ConditionEvaluationReportLogger") {
            writeln!(out, "{l}").unwrap();
            i += 1;
            while i < lines.len() && !is_log(lines[i]) {
                writeln!(out, "{}", lines[i]).unwrap();
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    std::process::exit(exit as i32);
}
