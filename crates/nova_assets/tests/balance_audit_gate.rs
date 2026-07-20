//! CI gate for the balance audit (task 20260717-112656): the same walk the
//! `content` CLI's `lint` runs (the balance audit was folded into `lint`,
//! task 20260718-152240) must produce ZERO error-grade findings over the
//! shipped content tree - a scenario that opens with an armed hostile
//! inside its own effective range of the player spawn (the pre-rework
//! ledger_ch2 shape) fails the build, not a playtest. Warn-grade findings
//! do not gate unless ACKNOWLEDGED-and-stale (task 20260717-143806): an
//! ack in crates/nova_assets/balance_acks.ron that matches no live finding
//! is dead weight and fails here so the exception list stays pruned. The
//! finding rules' own fail-first lives in nova_assets::balance's unit
//! tests (a synthetic spawned-dead scenario must grade ERROR, and an ack
//! must never suppress one).

use nova_assets::balance::{audit_content_tree, partition_findings, shipped_acks, BalanceSeverity};

#[test]
fn shipped_content_carries_no_balance_errors_and_no_stale_acks() {
    let audits = audit_content_tree();
    assert!(
        !audits.is_empty(),
        "the walk found no combat scenarios - the audit itself is broken"
    );

    let acks = shipped_acks();
    let findings = audits
        .iter()
        .flat_map(|(bundle, audit)| {
            audit
                .findings()
                .into_iter()
                .map(move |finding| (bundle.clone(), finding))
        })
        .collect();
    let (active, _acked, stale) = partition_findings(findings, &acks);

    let errors: Vec<String> = active
        .iter()
        .filter(|(_, finding)| finding.severity == BalanceSeverity::Error)
        .map(|(bundle, finding)| format!("[{bundle}] {}: {}", finding.scenario, finding.message))
        .collect();
    assert!(
        errors.is_empty(),
        "balance errors in shipped content (never ackable):\n{}\nfull report:\n{}",
        errors.join("\n"),
        audits
            .iter()
            .map(|(b, a)| format!("[{b}] {}", a.report()))
            .collect::<Vec<_>>()
            .join("")
    );

    // Acks must stay pruned: one matching no live finding means the content
    // moved on and the recorded exception is dead weight.
    assert!(
        stale.is_empty(),
        "stale balance acks (prune crates/nova_assets/balance_acks.ron): {:?}",
        stale
            .iter()
            .map(|ack| {
                format!(
                    "[{}] {} '{}' {} (task {})",
                    ack.bundle, ack.scenario, ack.hostile, ack.kind, ack.task
                )
            })
            .collect::<Vec<_>>()
    );
}
