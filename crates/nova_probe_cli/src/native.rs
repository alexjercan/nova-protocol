//! The native probe driver, one module per concern: the command line, spec
//! resolution, paths, child-run environments, child-run supervision, the
//! single-example run, the web pass, the multi-example sweep, and `report`.

use std::process::ExitCode;

mod cli;
mod env;
#[cfg(test)]
mod fixtures;
mod paths;
mod report;
mod run;
mod spec;
mod supervise;
mod sweep;
mod web;

use cli::{parse, Cmd, USAGE};

/// Parse the command line and dispatch it; the process exit code is the
/// harness verdict.
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
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
