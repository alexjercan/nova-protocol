# Review: CI fmt gate (20260721-163942)

- VERDICT: APPROVE

(No code change - task already satisfied; premise falsified. See below.)

## Summary

This task asked to add a `cargo fmt --check` step to CI. Investigation shows
the step already exists and has since before the task was filed. No code change
is warranted; the correct outcome is to close on evidence and route the real
underlying gap to a new task (done: 20260722-183022).

## Evidence

- **DoD 1** - `grep -n "fmt --check" .github/workflows/ci.yaml` matches the
  "Formatting" step at ci.yaml:60-61 (`cargo fmt --check`).
- **DoD 2** - `cargo fmt --check` exits 0 on the current tree
  (rustfmt 1.9.0-nightly, c397dae808 2026-07-02).
- The step was introduced 2026-07-09 in commit f1720672
  ("ci: run fmt, clippy and tests on PRs and pushes to master"), 12 days
  before this task (20260721-163942) was created.

## Why drift still reached master (the real, separate gap)

The four files healed in 8c7be318 (2026-07-21) landed unformatted despite the
CI step because the CI fmt gate is advisory for this project's workflow:

- Landing is local (`sprout land` squash-merge) + a direct push to master, not
  a PR merge. CI on push-to-master runs *after* the commit is already on
  master and does not block or revert it.
- No local guard exists: `core.hooksPath` is unset, `.git/hooks` holds only
  samples, and neither `/work` verify nor `sprout land` runs `cargo fmt
  --check`.

That gap is out of scope for "add a CI step" and is filed as 20260722-183022
(pre-land/pre-commit fmt guard), aligned with LESSONS.md
`lint-gate-is-the-last-step` (x3, Pending promotion).

## Process note

The spawning retro (tasks/20260721-160842/RETRO.md) claimed "CI does not gate
cargo fmt --check" without opening ci.yaml - the same class of unverified
history claim its own lesson `pickaxe-hit-is-not-a-mechanism` was written to
prevent. Recorded in RETRO.md as the takeaway.
