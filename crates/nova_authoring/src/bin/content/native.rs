//! The authoring CLI itself; see `main.rs` for what it does.

use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand, ValueEnum};
use nova_authoring::content_report::ContentReport;

#[derive(Parser)]
#[command(
    name = "content",
    about = "Author and validate Nova Protocol content (gen/lint)."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serialize the code-built base content to the committed *.content.ron.
    Gen,
    /// Lint the content tree (or one mod with --target) for every check the
    /// load/publish gates cannot make: references/geometry, combat balance,
    /// and flight-rig input overlaps.
    Lint {
        /// A mod directory (anywhere on disk; the dir name is the mod id) or
        /// an in-repo id (`webmods/<id>`, `assets/mods/<id>`, or `base`).
        /// Omit to lint the whole repo tree.
        #[arg(long)]
        target: Option<String>,
        /// Write a per-mod findings report to this path (a document that names
        /// file + element + fix for every finding). Omit for stdout only.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Report format. Defaults to Markdown, or HTML when `--report` ends
        /// in `.html`.
        #[arg(long, value_enum)]
        format: Option<ReportFormat>,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum ReportFormat {
    Md,
    Html,
}

pub fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Gen => run_gen(),
        Command::Lint {
            target,
            report,
            format,
        } => run_lint(target.as_deref(), report, format),
    }
}

/// Write the builder-backed base content files. CARGO_MANIFEST_DIR is
/// compiled in, so the paths resolve regardless of the invocation directory.
fn run_gen() -> ExitCode {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (rel, contents) in nova_authoring::scenario_generation::content_files() {
        let path = assets.join(&rel);
        nova_assets::storage::write_atomic(&path, contents.as_bytes())
            .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

fn run_lint(
    target: Option<&str>,
    report: Option<PathBuf>,
    format: Option<ReportFormat>,
) -> ExitCode {
    let report_data = match target {
        None => nova_authoring::lint_walk::collect_tree(),
        Some(target) => {
            let Some(dir) = nova_authoring::lint_walk::resolve_target(target) else {
                eprintln!(
                    "content lint: no mod named '{target}' (not a directory, not under webmods/ or assets/mods/, not 'base')"
                );
                return ExitCode::FAILURE;
            };
            println!("content lint: target {}", dir.display());
            nova_authoring::lint_walk::collect_target(&dir)
        }
    };

    print_summary(&report_data);

    if let Some(path) = report {
        if let Err(err) = write_report(&report_data, &path, format) {
            eprintln!(
                "content lint: failed to write report to {}: {err}",
                path.display()
            );
            return ExitCode::FAILURE;
        }
        println!("content lint: wrote report to {}", path.display());
    }

    // The exit-code rule (preserved from the merged lint + audit): non-zero on
    // any Error - a broken reference, a spawned-dead hostile, or a stale ack
    // (all Error-grade in the report). Warnings (close-spawn, input-overlap,
    // authoring smells) never gate.
    if report_data.error_count() > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The concise stdout view: one line per finding, then a count line. The full
/// located document is the `--report` file.
fn print_summary(report: &ContentReport) {
    use nova_authoring::content_report::Severity;

    for finding in &report.findings {
        let tag = match finding.severity {
            Severity::Error => "ERROR",
            Severity::Warn => "WARN ",
        };
        let file = finding.file.as_deref().unwrap_or("(unknown file)");
        println!(
            "{tag} [{}] {file} {}: {}",
            finding.bundle, finding.element, finding.message
        );
    }
    for acked in &report.acked {
        println!(
            "ACK   [{}] {}: {} | acked by {}: {}",
            acked.bundle, acked.element, acked.message, acked.ack_task, acked.ack_reason
        );
    }
    println!(
        "content lint: {} error(s), {} warning(s), {} finding(s), {} scenario(s) balance-audited, {} acked",
        report.error_count(),
        report.warn_count(),
        report.findings.len(),
        report.scenarios_audited,
        report.acked.len(),
    );
}

/// Resolve the format (explicit flag wins, else the path extension, else
/// Markdown) and write the report document.
fn write_report(
    report: &ContentReport,
    path: &std::path::Path,
    format: Option<ReportFormat>,
) -> std::io::Result<()> {
    let html = match format {
        Some(ReportFormat::Html) => true,
        Some(ReportFormat::Md) => false,
        None => path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("html")),
    };
    let contents = if html {
        report.to_html()
    } else {
        report.to_markdown()
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)
}
