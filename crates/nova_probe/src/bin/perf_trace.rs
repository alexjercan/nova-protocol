//! `perf_trace` - render the top-N costliest-systems table from a
//! chrome-trace JSON (a `--features trace` run with `TRACE_CHROME=<path>`).
//! Thin CLI over [`nova_probe::profile`]; `scripts/perf-profile.sh` drives
//! the whole profiled pass.
//!
//! ```text
//! cargo run -p nova_probe --bin perf_trace -- <trace.json> [--top N] [-o table.md]
//! ```

use std::{path::PathBuf, process::ExitCode};

use nova_probe::profile::{aggregate_system_costs, render_top_table};

const USAGE: &str = "usage: perf_trace <trace.json> [--top N] [-o <table.md>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut trace_path: Option<PathBuf> = None;
    let mut top: usize = 20;
    let mut output: Option<PathBuf> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
            "--top" => {
                let Some(value) = iter.next().and_then(|v| v.parse().ok()) else {
                    eprintln!("perf_trace: --top needs a number\n\n{USAGE}");
                    return ExitCode::FAILURE;
                };
                top = value;
            }
            "-o" | "--output" => {
                let Some(value) = iter.next() else {
                    eprintln!("perf_trace: -o needs a file path\n\n{USAGE}");
                    return ExitCode::FAILURE;
                };
                output = Some(PathBuf::from(value));
            }
            other if other.starts_with('-') => {
                eprintln!("perf_trace: unknown flag {other}\n\n{USAGE}");
                return ExitCode::FAILURE;
            }
            other => {
                if trace_path.replace(PathBuf::from(other)).is_some() {
                    eprintln!("perf_trace: only one trace file may be given\n\n{USAGE}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let Some(trace_path) = trace_path else {
        eprintln!("perf_trace: a trace file is required\n\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let contents = match std::fs::read_to_string(&trace_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!(
                "perf_trace: could not read {}: {error}",
                trace_path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let costs = match aggregate_system_costs(&contents) {
        Ok(costs) => costs,
        Err(message) => {
            eprintln!("perf_trace: {}: {message}", trace_path.display());
            return ExitCode::FAILURE;
        }
    };
    let table = render_top_table(&costs, top);
    match &output {
        Some(path) => {
            if let Err(error) = std::fs::write(path, &table) {
                eprintln!("perf_trace: could not write {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
            println!(
                "perf_trace: wrote {} ({} systems aggregated)",
                path.display(),
                costs.len()
            );
        }
        None => print!("{table}"),
    }
    ExitCode::SUCCESS
}
