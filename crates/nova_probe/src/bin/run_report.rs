//! `run_report` - assemble a run directory's artifacts into the unified run
//! report (`report.html` + `checks.json`). Thin CLI over
//! [`nova_probe::run_report`]; see that module for the artifact contract.
//!
//! ```text
//! cargo run -p nova_probe --bin run_report -- <run-dir> [--baseline <dir>]
//! ```
//!
//! Exit code: 0 on OK/WARN (soft gates never fail a run), 1 on FAIL or on
//! unreadable artifacts. The final verdict belongs to the reviewer either
//! way - see the checklist at the bottom of the report.

// The report reads the recorder's timeline - native-only, like the module
// it wraps; the wasm build gets a stub main so `cargo check --target wasm32`
// over the whole package stays green.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use std::{path::PathBuf, process::ExitCode};

#[cfg(not(target_arch = "wasm32"))]
use nova_probe::run_report::{
    checks_json, evaluate_checks, overall_verdict, print_checks, render_run_report, RunArtifacts,
};

#[cfg(not(target_arch = "wasm32"))]
const USAGE: &str = "usage: run_report <run-dir> [--baseline <dir>]";

#[cfg(not(target_arch = "wasm32"))]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut run_dir: Option<PathBuf> = None;
    let mut baseline: Option<PathBuf> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
            "--baseline" => {
                let Some(value) = iter.next() else {
                    eprintln!("run_report: --baseline needs a directory\n\n{USAGE}");
                    return ExitCode::FAILURE;
                };
                baseline = Some(PathBuf::from(value));
            }
            other if other.starts_with('-') => {
                eprintln!("run_report: unknown flag {other}\n\n{USAGE}");
                return ExitCode::FAILURE;
            }
            other => {
                if run_dir.replace(PathBuf::from(other)).is_some() {
                    eprintln!("run_report: only one run dir may be given\n\n{USAGE}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let Some(run_dir) = run_dir else {
        eprintln!("run_report: a run dir is required\n\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let artifacts = match RunArtifacts::load(&run_dir, baseline.as_deref()) {
        Ok(artifacts) => artifacts,
        Err(message) => {
            eprintln!("run_report: {message}");
            return ExitCode::FAILURE;
        }
    };
    let checks = evaluate_checks(&artifacts);
    let verdict = overall_verdict(&checks);

    let html = render_run_report(&run_dir, &artifacts, &checks);
    let html_path = run_dir.join("report.html");
    if let Err(error) = std::fs::write(&html_path, &html) {
        eprintln!(
            "run_report: could not write {}: {error}",
            html_path.display()
        );
        return ExitCode::FAILURE;
    }
    let json_path = run_dir.join("checks.json");
    let json = format!("{:#}\n", checks_json(&checks, artifacts.manifest.as_ref()));
    if let Err(error) = std::fs::write(&json_path, json) {
        eprintln!(
            "run_report: could not write {}: {error}",
            json_path.display()
        );
        return ExitCode::FAILURE;
    }

    println!(
        "run_report: {verdict} - wrote {} and {}",
        html_path.display(),
        json_path.display()
    );
    print_checks(&checks);
    // NO_DATA exits non-zero too: an agent must never mistake an
    // evidence-free dir for a passing run.
    if verdict == "FAIL" || verdict == "NO_DATA" {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
