//! The native probe driver, one module per concern: the command line, spec
//! resolution, paths, child-run environments, the profile sandbox those
//! environments point at, child-run supervision, the single-example run, the
//! web pass, the multi-example sweep, and `report`.

use std::process::ExitCode;

mod cli;
mod env;
#[cfg(test)]
mod fixtures;
mod paths;
pub mod profile_sandbox;
mod report;
mod run;
mod spec;
mod supervise;
mod sweep;
mod web;

use cli::{parse, Cmd, USAGE};

/// Parse the forwarded command line (everything after `probe` on the game
/// binary's command line) and dispatch it; the process exit code is the
/// harness verdict.
///
/// One command runs autopilot examples through the harness passes and hands
/// back per-example run reports plus an aggregated status index. A single
/// example is the same aggregate shape with one row:
///
/// ```text
/// cargo run --features debug probe run player_path       # clean + declared frame time + trace + report
/// cargo run --features debug probe run player_path,scenario_grammar  # comma list -> aggregate
/// cargo run --features debug probe run ui                # a category dir's examples
/// cargo run --features debug probe run --all             # the whole catalog
/// cargo run --features debug probe report <run-dir>      # re-render (manifest-gated)
/// ```
///
/// `run` orchestrates natively: pass 1 CLEAN (timeline + invariants + log), a
/// frame-time pass when the program declares it, pass 2 PROFILED (a separate
/// trace build whose overhead never touches frame-time numbers), an optional
/// `--samply` flamegraph run, then the run report in-process. Specs resolve
/// against the Cargo.toml `[[example]]` catalog (the single source of truth -
/// autoexamples is off), run sequentially with continue-on-failure, and write
/// `index.html` + `index.json` + `probe-all.json` above the per-example run
/// dirs.
pub fn main(args: &[String]) -> ExitCode {
    match parse(args) {
        Err(message) => {
            eprintln!("probe: {message}\n\n{USAGE}");
            ExitCode::FAILURE
        }
        Ok(Cmd::RunSpec { tokens, all, base }) => match sweep::run_spec(&tokens, all, base) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("probe: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(Cmd::Report { dirs, baseline }) => {
            match report::report_many(&dirs, baseline.as_deref()) {
                Ok(code) => code,
                Err(message) => {
                    eprintln!("probe: {message}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
