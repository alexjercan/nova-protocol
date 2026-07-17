//! The balance audit over the repo's content tree (task 20260717-112656):
//!
//! ```text
//! cargo run -p nova_assets --bin balance_audit
//! ```
//!
//! Prints every combat scenario's derived balance sheet (player pool,
//! per-spawn-group hostile dps / ranges / distances / TTK, cover tiers)
//! and grades the two static fairness findings (ERROR spawned-dead, WARN
//! close-spawn - see `nova_assets::balance`). Exits non-zero on any
//! ERROR; CI runs the same walk via the `balance_audit_gate` test.

use nova_assets::balance::{audit_content_tree, partition_findings, shipped_acks, BalanceSeverity};

fn main() -> std::process::ExitCode {
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
        "balance_audit: {} combat scenario(s), {errors} error(s), {warnings} warning(s), {} acked",
        audits.len(),
        acked.len()
    );
    if errors > 0 || !stale.is_empty() {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}
