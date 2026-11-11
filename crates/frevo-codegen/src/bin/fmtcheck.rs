//! Diff each generated file against `rustfmt`'s own output.
//!
//! A development aid for `codegen.output.004`, which asserts that every
//! emitted file is already in `rustfmt`'s output form. The oracle asserts it;
//! this prints *where* the two disagree, which is what makes a layout rule in
//! [`frevo_codegen::rustsrc`] findable rather than guessable.
//!
//! ```text
//! cargo run -p frevo-codegen --bin fmtcheck -- <out-dir>
//! ```

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use frevo_codegen::run::{GenerateRequest, generate};

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("frevo-fmtcheck"));
    let _ = std::fs::remove_dir_all(&out);

    let report = match generate(&GenerateRequest::pinned(&out)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("generate failed: {error}");
            std::process::exit(1);
        }
    };

    let mut disagreements = 0usize;
    for file in &report.files {
        let source = file.text();
        let formatted = rustfmt(source);
        if formatted == source {
            println!("{}: clean", file.path);
            continue;
        }
        disagreements += 1;
        println!("{}: DIFFERS", file.path);
        let ours: Vec<&str> = source.lines().collect();
        let theirs: Vec<&str> = formatted.lines().collect();
        let mut shown = 0usize;
        let mut index = 0usize;
        while index < ours.len().max(theirs.len()) && shown < 12 {
            let a = ours.get(index).copied().unwrap_or("<eof>");
            let b = theirs.get(index).copied().unwrap_or("<eof>");
            if a != b {
                println!("  line {}:\n    ours   {a:?}\n    rustfmt {b:?}", index + 1);
                shown += 1;
                // Skip ahead so one shifted block does not fill the output.
                index += 1;
                continue;
            }
            index += 1;
        }
    }
    if disagreements > 0 {
        std::process::exit(1);
    }
}

fn rustfmt(source: &str) -> String {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--quiet", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rustfmt on PATH");
    child
        .stdin
        .as_mut()
        .expect("rustfmt stdin")
        .write_all(source.as_bytes())
        .expect("writing to rustfmt");
    let out = child.wait_with_output().expect("rustfmt output");
    assert!(
        out.status.success(),
        "rustfmt rejected the input: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("rustfmt output is UTF-8")
}
