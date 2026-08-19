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
/// `run` takes autopilot examples through the harness passes and hands back
/// per-example run reports plus an aggregated status index; a single example is
/// the same aggregate shape with one row. `scenario` takes the same passes to a
/// scenario, with no example between the tool and the data:
///
/// ```text
/// cargo run --features debug probe run <example>         # clean + declared frame time + trace + report
/// cargo run --features debug probe run <example>,<example>  # comma list -> aggregate
/// cargo run --features debug probe run <category>        # a category dir's examples
/// cargo run --features debug probe run --all             # the whole catalog
/// cargo run --features debug probe scenario <id>         # a scenario from the merged registry
/// cargo run --features debug probe scenario <file.ron>   # a loose content file, catalog or not
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
///
/// `scenario` runs the SAME passes against the game binary, which resolves the
/// id against its own merged registry (or registers the loose file first). It
/// consults no catalog and writes one run dir - a scenario is not a spec, so
/// there is nothing to expand and no aggregate to build.
pub fn main(args: &[String]) -> ExitCode {
    match parse(args) {
        Err(message) => {
            eprintln!("probe: {message}\n\n{USAGE}");
            ExitCode::FAILURE
        }
        Ok(Cmd::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Cmd::RunSpec { tokens, all, base }) => match sweep::run_spec(&tokens, all, base) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("probe: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(Cmd::Scenario { base }) => match run::run(&base) {
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
