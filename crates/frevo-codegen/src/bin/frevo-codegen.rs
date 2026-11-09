//! `frevo-codegen` — the generator's command line.
//!
//! ```text
//! frevo-codegen generate   --version r4 [--package <dir>] [--out <dir>] [--sha256 <hex>]
//! frevo-codegen cross-check --version r4 [--package <dir>] --hapi-profiles <dir>
//! frevo-codegen drift-check --version r4 [--package <dir>] --checked-in <dir>
//! frevo-codegen verify    [--package <dir>]
//! frevo-codegen pin
//! ```
//!
//! Argument parsing is hand-written. A command with four flags does not
//! justify a dependency, and the constitution's dependency rule means the
//! alternative is an ADR.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use frevo_codegen::crosscheck::HAPI_PIN_SHA;
use frevo_codegen::error::CodegenError;
use frevo_codegen::pin::{self, GENERATOR_NAME, PACKAGE_DIR, VERSION_SELECTOR};
use frevo_codegen::run::{self, GenerateRequest};

const USAGE: &str = "\
frevo-codegen — generate the Rust FHIR model from a pinned definition package

Usage:
  frevo-codegen generate     --version r4 [--package <dir>] [--out <dir>] [--sha256 <hex>]
  frevo-codegen cross-check  --version r4 [--package <dir>] --hapi-profiles <dir>
  frevo-codegen drift-check  --version r4 [--package <dir>] --checked-in <dir>
  frevo-codegen verify       [--package <dir>]
  frevo-codegen pin

Options:
  --version <sel>        the FHIR version to generate; r4 is the only one
  --package <dir>        the definition package; defaults to the recorded pin's directory
  --out <dir>            where to write; defaults to crates/frevo-model/src/r4
  --sha256 <hex>         the digest to verify against; defaults to the recorded one
  --hapi-profiles <dir>  HAPI's profile/ directory, for the cross-check
  --checked-in <dir>     the committed output tree, for the drift check
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&arguments) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{GENERATOR_NAME}: {error}");
            let code = match error {
                CodegenError::Failed { exit_code, .. } => exit_code,
                CodegenError::NotImplemented => 70,
            };
            ExitCode::from(u8::try_from(code).unwrap_or(1))
        }
    }
}

fn dispatch(arguments: &[String]) -> Result<String, CodegenError> {
    let Some((command, rest)) = arguments.split_first() else {
        return Ok(USAGE.to_string());
    };
    let flags = Flags::parse(rest)?;
    match command.as_str() {
        "generate" => {
            let request = flags.request()?;
            let report = run::generate(&request)?;
            Ok(format!(
                "{}\nWrote {} file(s) to {}.",
                report.summary,
                report.files.len(),
                request.out_dir.display()
            ))
        }
        "cross-check" => {
            let hapi = flags.required("--hapi-profiles")?;
            let report = run::cross_check(flags.package.as_deref(), &hapi)?;
            Ok(format!(
                "Scope: {}\nPackage fhirVersion {}, HAPI fhirVersion {} at {HAPI_PIN_SHA}.\n\
                 {} definitions in HAPI's profile bundles; {} only in the package, {} only in \
                 HAPI, {} element disagreements.",
                report.scope,
                report.package_fhir_version,
                report.hapi_fhir_version,
                report.hapi_definition_count,
                report.only_in_package.len(),
                report.only_in_hapi.len(),
                report.element_disagreements.len(),
            ))
        }
        "drift-check" => {
            let checked_in = flags.required("--checked-in")?;
            let request = flags.request()?;
            let report = run::drift_check(&request, &checked_in)?;
            if report.in_sync {
                Ok(format!(
                    "{} is in sync with a fresh run.",
                    checked_in.display()
                ))
            } else {
                Err(CodegenError::other(format!(
                    "{} differs from a fresh run in {:?}. {}",
                    checked_in.display(),
                    report.differing_files,
                    report.remediation
                )))
            }
        }
        "verify" => {
            let dir = flags
                .package
                .clone()
                .unwrap_or_else(pin::default_package_dir);
            let package = frevo_codegen::package::load(&dir)?;
            frevo_codegen::package::verify_digest(&package, pin::PACKAGE_CONTENT_SHA256)?;
            Ok(format!(
                "{} matches the recorded content digest {} over {} files ({} bytes).",
                package.dir.display(),
                package.digest,
                package.digest_file_count,
                package.digest_byte_count
            ))
        }
        "pin" => {
            let pin = pin::recorded_pin();
            Ok(format!(
                "{} {}\ncontent SHA-256 {}\narchive SHA-256 {}\ndirectory {}",
                pin.name,
                pin.version,
                pin.sha256,
                pin::PACKAGE_ARCHIVE_SHA256,
                PACKAGE_DIR
            ))
        }
        "help" | "--help" | "-h" => Ok(USAGE.to_string()),
        other => Err(CodegenError::other(format!(
            "unknown command {other:?}\n\n{USAGE}"
        ))),
    }
}

#[derive(Debug, Default)]
struct Flags {
    fhir_version: Option<String>,
    package: Option<PathBuf>,
    out: Option<PathBuf>,
    sha256: Option<String>,
    named: Vec<(String, PathBuf)>,
}

impl Flags {
    fn parse(arguments: &[String]) -> Result<Flags, CodegenError> {
        let mut flags = Flags::default();
        let mut index = 0usize;
        while index < arguments.len() {
            let flag = arguments[index].as_str();
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| CodegenError::other(format!("{flag} needs a value\n\n{USAGE}")))?;
            match flag {
                "--version" => flags.fhir_version = Some(value.clone()),
                "--package" => flags.package = Some(PathBuf::from(value)),
                "--out" => flags.out = Some(PathBuf::from(value)),
                "--sha256" => flags.sha256 = Some(value.clone()),
                "--hapi-profiles" | "--checked-in" => {
                    flags.named.push((flag.to_string(), PathBuf::from(value)));
                }
                other => {
                    return Err(CodegenError::other(format!(
                        "unknown option {other:?}\n\n{USAGE}"
                    )));
                }
            }
            index += 2;
        }
        Ok(flags)
    }

    fn required(&self, flag: &str) -> Result<PathBuf, CodegenError> {
        self.named
            .iter()
            .find(|(name, _)| name == flag)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| CodegenError::other(format!("{flag} is required\n\n{USAGE}")))
    }

    fn request(&self) -> Result<GenerateRequest, CodegenError> {
        Ok(GenerateRequest {
            fhir_version: self
                .fhir_version
                .clone()
                .unwrap_or_else(|| VERSION_SELECTOR.to_string()),
            package_dir: self.package.clone(),
            expected_sha256: self.sha256.clone(),
            out_dir: self
                .out
                .clone()
                .unwrap_or_else(|| pin::repo_path("crates/frevo-model/src/r4")),
            reference_dir: None,
        })
    }
}
