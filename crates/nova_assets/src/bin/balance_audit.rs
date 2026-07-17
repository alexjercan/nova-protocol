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

use nova_assets::balance::{audit_content_tree, BalanceSeverity};

fn main() -> std::process::ExitCode {
    let audits = audit_content_tree();
    let mut errors = 0;
    let mut warnings = 0;
    for (bundle, audit) in &audits {
        print!("[{bundle}] {}", audit.report());
        for finding in audit.findings() {
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
    }
    println!(
        "balance_audit: {} combat scenario(s), {errors} error(s), {warnings} warning(s)",
        audits.len()
    );
    if errors > 0 {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}
