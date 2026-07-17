//! The content authoring/validation CLI (task 20260717-212219): one tool
//! over the repo's content tree, with a subcommand per task. Replaces the
//! former separate `gen_content`, `content_lint` and `balance_audit` bins.
//!
//! ```text
//! cargo run -p nova_assets --bin content -- gen
//! cargo run -p nova_assets --bin content -- lint [--target <mod-dir-or-id>]
//! cargo run -p nova_assets --bin content -- audit
//! ```
//!
//! - `gen` writes the builder-backed base content files (task
//!   20260716-155823): the scenario/section builders in
//!   `nova_assets::scenario_generation` are the single definition of each
//!   built-in; this serializes them into the committed
//!   `assets/base/**/*.content.ron` the game loads. Run it (and commit the
//!   result) after any builder change - the `content_ron_parity` test
//!   asserts the files match and names this command when they drift.
//! - `lint` runs the identifier + geometry checks the load/publish gates
//!   cannot make (task 20260716-191543): unknown section prototypes,
//!   dangling NextScenario targets, unspawnable filter targets, duplicate
//!   ids, mount-base adjacency; warns on unset variables and unmatched
//!   ObjectiveComplete. `--target` lints one mod (task 20260716-204618):
//!   a mod directory anywhere on disk (the dir name is the mod id,
//!   portal-style) or an in-repo id (`webmods/<id>`, `assets/mods/<id>`,
//!   or `base`). Exits non-zero on any Error; CI runs the same walk via
//!   the `content_lint_gate` test.
//! - `audit` prints every combat scenario's derived balance sheet and
//!   grades the two static fairness findings (task 20260717-112656; ERROR
//!   spawned-dead, WARN close-spawn - see `nova_assets::balance`). Exits
//!   non-zero on any Error or stale ack; CI runs the same walk via the
//!   `balance_audit_gate` test.

use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};
use nova_assets::balance::{audit_content_tree, partition_findings, shipped_acks, BalanceSeverity};
use nova_scenario::prelude::LintSeverity;

#[derive(Parser)]
#[command(
    name = "content",
    about = "Author and validate Nova Protocol content (gen/lint/audit)."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serialize the code-built base content to the committed *.content.ron.
    Gen,
    /// Lint the content tree (or one mod with --target) for the checks the
    /// load/publish gates cannot make.
    Lint {
        /// A mod directory (anywhere on disk; the dir name is the mod id) or
        /// an in-repo id (`webmods/<id>`, `assets/mods/<id>`, or `base`).
        /// Omit to lint the whole repo tree.
        #[arg(long)]
        target: Option<String>,
    },
    /// Print every combat scenario's balance sheet and grade the static
    /// fairness findings.
    Audit,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Gen => run_gen(),
        Command::Lint { target } => run_lint(target.as_deref()),
        Command::Audit => run_audit(),
    }
}

/// Write the builder-backed base content files. CARGO_MANIFEST_DIR is
/// compiled in, so the paths resolve regardless of the invocation directory.
fn run_gen() -> ExitCode {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (rel, contents) in nova_assets::scenario_generation::content_files() {
        let path = assets.join(&rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
        }
        std::fs::write(&path, contents)
            .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

fn run_lint(target: Option<&str>) -> ExitCode {
    let issues = match target {
        None => nova_assets::lint_walk::lint_content_tree(),
        Some(target) => {
            let Some(dir) = nova_assets::lint_walk::resolve_target(target) else {
                eprintln!(
                    "content lint: no mod named '{target}' (not a directory, not under webmods/ or assets/mods/, not 'base')"
                );
                return ExitCode::FAILURE;
            };
            println!("content lint: target {}", dir.display());
            nova_assets::lint_walk::lint_target(&dir)
        }
    };
    let mut errors = 0;
    for (bundle, issue) in &issues {
        let tag = match issue.severity {
            LintSeverity::Error => {
                errors += 1;
                "ERROR"
            }
            LintSeverity::Warn => "WARN ",
        };
        println!(
            "{tag} [{bundle}] scenario '{}': {}",
            issue.scenario, issue.message
        );
    }
    if errors > 0 {
        println!(
            "content lint: {errors} error(s), {} finding(s) total",
            issues.len()
        );
        ExitCode::FAILURE
    } else {
        println!("content lint: clean ({} warning(s))", issues.len());
        ExitCode::SUCCESS
    }
}

fn run_audit() -> ExitCode {
    let audits = audit_content_tree();
    let acks = shipped_acks();
    let mut findings = Vec::new();
    for (bundle, audit) in &audits {
        print!("[{bundle}] {}", audit.report());
        for finding in audit.findings() {
            findings.push((bundle.clone(), finding));
        }
    }
    let (active, acked, stale) = partition_findings(findings, &acks);

    let mut errors = 0;
    let mut warnings = 0;
    for (bundle, finding) in &active {
        let tag = match finding.severity {
            BalanceSeverity::Error => {
                errors += 1;
                "ERROR"
            }
            BalanceSeverity::Warn => {
                warnings += 1;
                "WARN "
            }
        };
        println!("{tag} [{bundle}] {}: {}", finding.scenario, finding.message);
    }
    for (bundle, finding, ack) in &acked {
        println!(
            "ACK   [{bundle}] {}: {} | acked by {}: {}",
            finding.scenario, finding.message, ack.task, ack.reason
        );
    }
    // A stale ack is repo hygiene gone bad: the content moved on and the
    // exception it justified no longer exists. Counts as a warning here
    // and FAILS the CI gate, so acks stay pruned.
    for ack in &stale {
        warnings += 1;
        println!(
            "WARN  stale ack: [{}] {} '{}' {} (task {}) matches no live finding - prune it",
            ack.bundle, ack.scenario, ack.hostile, ack.kind, ack.task
        );
    }
    println!(
        "content audit: {} combat scenario(s), {errors} error(s), {warnings} warning(s), {} acked",
        audits.len(),
        acked.len()
    );
    if errors > 0 || !stale.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
