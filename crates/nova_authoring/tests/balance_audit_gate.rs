//! CI gate for the balance audit: the same walk the `content` CLI's `lint` runs
//! (the balance audit was folded into `lint`) must produce ZERO error-grade
//! findings over the whole content tree - a scenario that opens with an armed
//! hostile inside its own effective range of the player spawn fails the build,
//! not a playtest. Warn-grade findings do not gate unless
//! ACKNOWLEDGED-and-stale: an ack a bundle declares in its own
//! `balance_acks.ron` that matches no live finding is dead weight and fails
//! here so every exception list stays pruned. The finding rules' own fail-first
//! lives in nova_authoring::balance's unit tests (a synthetic spawned-dead
//! scenario must grade ERROR, and an ack must never suppress one).

use nova_authoring::{
    balance::{audit_content_tree, partition_findings, BalanceSeverity},
    lint_walk::tree_acks,
};

#[test]
fn shipped_content_carries_no_balance_errors_and_no_stale_acks() {
    let audits = audit_content_tree();
    assert!(
        !audits.is_empty(),
        "the walk found no combat scenarios - the audit itself is broken"
    );

    let acks = tree_acks();
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
    // moved on and the recorded exception is dead weight. It is the OWNING
    // bundle's file to prune, which is why the ack lives there.
    assert!(
        stale.is_empty(),
        "stale balance acks (prune the owning bundle's balance_acks.ron): {:?}",
        stale
            .iter()
            .map(|(bundle, ack)| {
                format!(
                    "[{bundle}] {} '{}' {} (task {})",
                    ack.scenario, ack.hostile, ack.kind, ack.task
                )
            })
            .collect::<Vec<_>>()
    );
}

/// Every declared ack names a finding kind the audit can actually raise. A
/// typo'd kind would silently never match and surface as stale much later.
#[test]
fn every_declared_ack_names_a_real_finding_kind() {
    for (bundle, ack) in tree_acks() {
        assert!(
            ["spawned-dead", "close-spawn"].contains(&ack.kind.as_str()),
            "[{bundle}] unknown finding kind '{}' in balance_acks.ron",
            ack.kind
        );
    }
}
