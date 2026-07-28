# Review: nova_probe commit-keyed probe-runs folders and baseline discovery

- TASK: 20260729-003352
- BRANCH: tooling/probe-commit-keyed-runs

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) crates/nova_probe/src/bin/probe.rs:52 - CLI usage still labels run `--baseline` as `<run-dir>` and `report` as a single `<run-dir>`, even though the new behavior treats run baselines as a storage base and accepts multiple report dirs. Update usage to reflect `<base>` and `report <run-dir>...`.
  - Response: fixed by changing run usage to `--baseline <base-dir>` and report usage to `report <run-dir>...`.
- [x] R1.2 (MINOR) web/src/wiki/dev/development.md:416 - The dev wiki still shows `probe report <run-dir>` only. Add the multi-dir form so the task's report fix is discoverable.
  - Response: fixed by documenting `probe report <run-dir>... [--baseline <old-run-dir>]`.
- [x] R1.3 (NIT) crates/nova_probe/src/bin/probe.rs:450 - `group_baseline_for` is now only used by tests, so `cargo check -p nova_probe` warns about dead code. Remove the wrapper or gate it to tests.
  - Response: fixed by deleting the wrapper and updating the test to call `baseline_for` directly.

Verification:

- `nix develop --command cargo test -p nova_probe` passed.
- `nix develop --command cargo fmt --check` passed.
- `git diff --check master...HEAD` passed.
- `nix develop --command cargo check -p nova_probe` passed after the dead-code warning was fixed.
- `tatr check` and `tatr check --ledger LESSONS.md` failed before REVIEW.md and RETRO.md existed, as expected at this phase.

Pending manual checks: none.
