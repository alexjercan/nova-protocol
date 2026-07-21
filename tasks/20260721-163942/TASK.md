# CI: gate cargo fmt --check so master cannot accumulate format drift

- STATUS: OPEN
- PRIORITY: 20
- TAGS: v0.8.0,tooling,ci

## Goal

Master accumulates rustfmt drift because CI gates tests and clippy but not
formatting: four files (hud/readout.rs, probe/bin/probe.rs, probe/catalog.rs,
scenario/actions.rs) had drifted by 2026-07-21 and got swept into an
unrelated task's diff when it ran the documented `cargo fmt` ritual
(tasks/20260721-160842/RETRO.md). Prose says "run fmt before committing";
a CI gate makes the drift impossible instead (tool > prose).

## Steps

- [ ] Add a `cargo fmt --check` step to .github/workflows/ci.yaml (same
      toolchain the fmt ritual uses; fail the job on diff).
- [ ] Run the gate's command locally once to confirm the tree is currently
      clean (it is, post-8c7be318).

## Definition of Done

- CI fails on unformatted code (cmd: `grep -n "fmt --check" .github/workflows/ci.yaml`).
- Tree is fmt-clean at the gate's introduction (cmd: `cargo fmt --check`).
