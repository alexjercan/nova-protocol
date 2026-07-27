# Review: Fix catalog_matches_disk (smoke-list nova_os examples)

- TASK: 20260727-143752
- BRANCH: fix/catalog-smoke-nova-os

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (trivial diff - test-list only: two example names added to
  the `SCREENSHOTS` / `NOT_SMOKED` lists in `tests/examples_smoke.rs`, with the
  failing->passing `catalog_matches_disk` test as the direct proof)

DoD proof re-run: `cargo test --test examples_smoke catalog_matches_disk` PASS
(1 passed) - it was FAILED on master (its BTreeSet diff named `screenshot_nova_os`
and `nova_os_rtt_poc` as the two unaccounted examples).

Verified: `screenshot_nova_os` is a harnessed screenshot producer (autopilot,
reaches Playing) so `SCREENSHOTS` is correct; `nova_os_rtt_poc` runs its own
`App` with `DefaultPlugins` (not the game's AppBuilder/GameStates), prints a
`POC PICKING native: OK/FAIL` verdict and auto-exits, so it never reaches Playing
and `NOT_SMOKED` (with the recorded reason) is correct, not a smoke list. No
production code touched; the only risk (mis-classifying the poc into a smoke list
where reach-Playing would fail) was checked against the example's own main().

No BLOCKER / MAJOR / MINOR / NIT findings.
