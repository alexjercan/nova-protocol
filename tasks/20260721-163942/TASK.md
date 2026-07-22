# CI: gate cargo fmt --check so master cannot accumulate format drift

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.8.0, tooling, ci

## Goal

Master accumulates rustfmt drift because CI gates tests and clippy but not
formatting: four files (hud/readout.rs, probe/bin/probe.rs, probe/catalog.rs,
scenario/actions.rs) had drifted by 2026-07-21 and got swept into an
unrelated task's diff when it ran the documented `cargo fmt` ritual
(tasks/20260721-160842/RETRO.md). Prose says "run fmt before committing";
a CI gate makes the drift impossible instead (tool > prose).

## Steps

- [x] Add a `cargo fmt --check` step to .github/workflows/ci.yaml (same
      toolchain the fmt ritual uses; fail the job on diff).
      ALREADY PRESENT: ci.yaml:60-61 "Formatting" step, added 2026-07-09 in
      commit f1720672 - 12 days before this task was filed. No change needed.
- [x] Run the gate's command locally once to confirm the tree is currently
      clean (it is, post-8c7be318).
      `cargo fmt --check` exits 0 on the current tree.

## Definition of Done

- CI fails on unformatted code (cmd: `grep -n "fmt --check" .github/workflows/ci.yaml`).
- Tree is fmt-clean at the gate's introduction (cmd: `cargo fmt --check`).

## Resolution: already satisfied (premise falsified)

Closed with NO code change - both DoD checks already pass:

- `grep -n "fmt --check" .github/workflows/ci.yaml` -> matches ci.yaml:61.
- `cargo fmt --check` -> exit 0 (clean tree).

The task's premise ("CI gates tests and clippy but not formatting") is false.
CI has run `cargo fmt --check` since 2026-07-09 (f1720672). The spawning retro
(tasks/20260721-160842/RETRO.md) asserted "CI does not gate cargo fmt --check"
without opening ci.yaml - the exact failure its own lesson
`pickaxe-hit-is-not-a-mechanism` warns against.

The genuine gap it was groping at is different and is now filed separately as
**20260722-183022**: the existing CI step is only advisory for this project's
land flow (local `sprout land` squash-merge + direct push to master; a red
push-to-master run does not block/revert), and there is no local
pre-commit/pre-land fmt guard. That new task carries the real "tool > prose"
fix, matching LESSONS.md `lint-gate-is-the-last-step` (Pending promotion).

See REVIEW.md and RETRO.md for the evidence and the falsification record.
