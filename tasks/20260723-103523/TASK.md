# Fix stale content_lint_gate test: ledger ch4 'mutually exclusive' warn gone after ch4 rework

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.8.0, bug, testing, content

## Story

As a developer, I want `cargo test -p nova_assets` green on master, so the
content lint gate is a trustworthy guard. The integration test
`content_lint_gate::target_mode_lints_one_mod_in_repo_or_external` currently
FAILS on master (confirmed 20260723): it asserts the-ledger lint surfaces a
ch4 warn whose message contains "mutually exclusive", but `lint_target("the-
ledger")` now returns `[]` for that expectation, panicking at
`crates/nova_assets/tests/content_lint_gate.rs:48`.

Root cause (to confirm): the ch4 ending rework on 2026-07-22 (commit 33dc293f,
"diverging ch4 endings - burn avoids the Auditor", task 20260722-214110)
restructured `webmods/the-ledger/ledger_ch4.content.ron` and removed whatever
produced the "mutually exclusive" warn; the test's expectation was not updated
in the same change. This is INHERITED, unrelated to the campaign-metadata run
(umbrella 20260723-093914) that discovered it - task B's own guards
(content_ron_parity, content lint) pass.

## Steps

- [x] Reproduce: `cargo test -p nova_assets --test content_lint_gate
      target_mode_lints_one_mod_in_repo_or_external` fails at line 48.
- [x] Determine whether the ch4 "mutually exclusive" warn SHOULD still exist:
      LEGITIMATELY REMOVED. Empirically `content -- lint --target the-ledger`
      now reports 0 errors, 0 warns, 1 ACKED (the auditor close-spawn). The ack
      reason itself documents it: "The Auditor is spawned ONLY by the SELL
      branch now - the burn branch no longer fights (its spawn was removed...)"
      (task 20260722-214110). The cross-handler duplicate-spawn warn
      (lint.rs:189 "spawned by more than one handler ... mutually exclusive")
      no longer fires because there is no longer a dual spawn. Test is STALE.
- [x] Re-pin the test on a DURABLE the-ledger signal instead of the incidental
      warn: switch the in-repo half to `collect_target` (ContentReport) and
      assert (a) error_count == 0, (b) all findings + acks scoped to
      "the-ledger", (c) the ACKED auditor close-spawn exception surfaces
      (message contains "auditor", ack_task 20260717-143806). The acked finale
      entrance is a deliberate, playtested design decision in
      `balance_acks.ron` - stable, unlike a warn that content cleanup removes.
      Docstring updated to match.
- [x] Green `cargo test -p nova_assets --test content_lint_gate` (2/2). Full
      `cargo test -p nova_assets` still has 2 OTHER inherited failures in
      final_tally_claim.rs (survey->picket) - filed SEPARATELY as
      20260723-115419 (also fails identically on master; different subsystem).

## Definition of Done

- `cargo test -p nova_assets --test content_lint_gate` passes. (cmd: that test)
- The test asserts a signal the-ledger actually emits today (the acked auditor
  exception), not a stale warn. (test: content_lint_gate)
- Full `cargo test -p nova_assets` green EXCEPT the separately-filed
  final_tally_claim failures (20260723-115419) - this task fixes the
  content_lint_gate failure only. (cmd: `cargo test -p nova_assets --test content_lint_gate`)

## Notes

- Discovered during umbrella 20260723-093914 task B (095909) verify step; that
  branch does not touch the-ledger and is not the cause.
- Fails identically on master (checked out master, same panic at line 48).
- SCOPE: this task fixes ONLY the content_lint_gate stale test. Its verify step
  also surfaced 2 unrelated inherited failures in final_tally_claim.rs (survey
  posts no picket objective), filed as 20260723-115419 - not widened into this
  branch (one task per branch).

## Close-out (20260723)

Diagnosis: the failing assertion pinned a "mutually exclusive" cross-handler
duplicate-spawn WARN (nova_scenario lint.rs:189) that the-ledger no longer
emits. Confirmed empirically with `content -- lint --target the-ledger`: 0
errors, 0 warns, 1 acked. The ch4 diverging-endings rework (task
20260722-214110) removed the Auditor's second spawn branch, so the duplicate
spawn - and its warn - are gone by design; the ack reason documents exactly
this. So the test was stale, not the content broken.

Fix: re-pinned the in-repo half on the-ledger's DURABLE signal instead of an
incidental warn (the `would-it-fail-without-it` / pin-durable-intents lesson).
Switched from `lint_target` (unacked issues only, now empty for the-ledger) to
`collect_target` (the full ContentReport, which exposes acked exceptions) and
assert: error_count 0, findings+acks all scoped to "the-ledger", and the acked
auditor close-spawn exception surfaces (message contains "auditor", ack_task
20260717-143806). The acked finale entrance is a deliberate, playtested,
recorded design decision - far more stable than a warn that content cleanup
removes. The external half (self-authored bogus-prototype mod) is unchanged and
still robustly pins target-mode surfaces+attributes+error.

Verify: `cargo test -p nova_assets --test content_lint_gate` 2/2 green;
`cargo fmt --check` clean. The full `cargo test -p nova_assets` still shows 2
OTHER inherited failures in final_tally_claim.rs (survey->picket), which fail
identically on master and are a different subsystem - filed as 20260723-115419,
not fixed here.

Self-reflection: the merge-red / check-source-first discipline paid off twice
this session - both content_lint_gate AND final_tally_claim looked like "my
change broke the suite" but a `git branch --show-current`=master re-run proved
both inherited. Cheap check, correct attribution. Re-pinning on the acked
finding (not just deleting the stale assertion) keeps the test meaningful:
it would fail if target-mode attribution broke, if the-ledger regressed an
error, or if the auditor ack vanished.
