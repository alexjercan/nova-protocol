//! `perf_report` - turn a `perf-baseline.sh` results directory into one
//! self-contained HTML report. Thin CLI over [`nova_probe::report`], which
//! owns the rendering (and grows into the unified run report, task
//! 20260719-112304).
//!
//! Input is a results dir holding the aggregated `frametime.csv` the capture
//! harness writes (schema v1 or v2 - see [`nova_probe::stats`]). Output is a
//! single `.html` file with per-run percentiles, a bar chart, renderer
//! metadata, and - when a `--baseline` dir is given - the percentage delta of
//! every run against the baseline row of the same label.
//!
//! ```text
//! cargo run -p nova_probe --bin perf_report -- <results-dir> \
//!   [--baseline <baseline-dir>] [-o <output.html>]
//! ```
//!
//! Defaults: the report is written to `<results-dir>/report.html`.

use std::{path::PathBuf, process::ExitCode};

use nova_probe::report::{read_runs, render_report};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match Options::parse(&args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("perf_report: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let runs = match read_runs(&opts.results_dir) {
        Ok(runs) => runs,
        Err(message) => {
            eprintln!("perf_report: {message}");
            return ExitCode::FAILURE;
        }
    };
    if runs.is_empty() {
        eprintln!(
            "perf_report: no runs found in {} (frametime.csv had only a header)",
            opts.results_dir.display()
        );
        return ExitCode::FAILURE;
    }

    let baseline = match &opts.baseline_dir {
        None => None,
        Some(dir) => match read_runs(dir) {
            Ok(base) => Some((dir.clone(), base)),
            Err(message) => {
                eprintln!("perf_report: baseline: {message}");
                return ExitCode::FAILURE;
            }
        },
    };

    let output = opts
        .output
        .clone()
        .unwrap_or_else(|| opts.results_dir.join("report.html"));

    let html = render_report(&opts.results_dir, &runs, baseline.as_ref());
    if let Err(error) = std::fs::write(&output, &html) {
        eprintln!("perf_report: could not write {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    println!(
        "perf_report: wrote {} ({} runs, {} bytes)",
        output.display(),
        runs.len(),
        html.len()
    );
    ExitCode::SUCCESS
}

const USAGE: &str = "usage: perf_report <results-dir> [--baseline <dir>] [-o <output.html>]";

/// Parsed command line.
struct Options {
    results_dir: PathBuf,
    baseline_dir: Option<PathBuf>,
    output: Option<PathBuf>,
}

impl Options {
    /// Parse `<results-dir> [--baseline <dir>] [-o|--output <file>]`. Exactly
    /// one positional (the results dir) is required.
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut results_dir: Option<PathBuf> = None;
        let mut baseline_dir: Option<PathBuf> = None;
        let mut output: Option<PathBuf> = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => return Err("help".to_string()),
                "--baseline" => {
                    let value = iter.next().ok_or("--baseline needs a directory")?;
                    baseline_dir = Some(PathBuf::from(value));
                }
                "-o" | "--output" => {
                    let value = iter.next().ok_or("-o needs a file path")?;
                    output = Some(PathBuf::from(value));
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown flag {other}"));
                }
                other => {
                    if results_dir.replace(PathBuf::from(other)).is_some() {
                        return Err("only one results dir may be given".to_string());
                    }
                }
            }
        }
        Ok(Self {
            results_dir: results_dir.ok_or("a results dir is required")?,
            baseline_dir,
            output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_positional_and_flags() {
        let opts = Options::parse(&[
            "results/".to_string(),
            "--baseline".to_string(),
            "base/".to_string(),
            "-o".to_string(),
            "out.html".to_string(),
        ])
        .expect("valid args");
        assert_eq!(opts.results_dir, PathBuf::from("results/"));
        assert_eq!(opts.baseline_dir, Some(PathBuf::from("base/")));
        assert_eq!(opts.output, Some(PathBuf::from("out.html")));

        assert!(Options::parse(&[]).is_err()); // results dir required
        assert!(Options::parse(&["a".to_string(), "b".to_string()]).is_err()); // two positionals
        assert!(Options::parse(&["--nope".to_string()]).is_err()); // unknown flag
    }
}
