//! CI gate for the balance audit (task 20260717-112656): the same walk the
//! `balance_audit` bin runs must produce ZERO error-grade findings over the
//! shipped content tree - a scenario that opens with an armed hostile
//! inside its own effective range of the player spawn (the pre-rework
//! ledger_ch2 shape) fails the build, not a playtest. Warn-grade findings
//! are surfaced in the failure message but do not gate. The finding rules'
//! own fail-first lives in nova_assets::balance's unit tests (a synthetic
//! spawned-dead scenario must grade ERROR).

use nova_assets::balance::{audit_content_tree, BalanceSeverity};

#[test]
fn shipped_content_carries_no_balance_errors() {
    let audits = audit_content_tree();
    assert!(
        !audits.is_empty(),
        "the walk found no combat scenarios - the audit itself is broken"
    );

    let mut errors = Vec::new();
    for (bundle, audit) in &audits {
        for finding in audit.findings() {
            if finding.severity == BalanceSeverity::Error {
                errors.push(format!(
                    "[{bundle}] {}: {}",
                    finding.scenario, finding.message
                ));
            }
        }
    }
    assert!(
        errors.is_empty(),
        "balance errors in shipped content:\n{}\nfull report:\n{}",
        errors.join("\n"),
        audits
            .iter()
            .map(|(b, a)| format!("[{b}] {}", a.report()))
            .collect::<Vec<_>>()
            .join("")
    );
}
