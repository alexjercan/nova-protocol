# Retro: extract editor into nova_editor (task 20260525-132937)

## What was asked
Finalize nova_core as a thin wiring layer: verify no gameplay logic, move anything
substantive into nova_gameplay or a dedicated crate.

## What happened
core.rs (~1100 lines) was the whole spaceship editor scene - substantive logic that
did not belong in the wiring crate. Extracted it into a new `nova_editor` crate
(`NovaEditorPlugin`), moved `GameStates` down to `nova_gameplay`, and slimmed
`nova_core` to just `bevy` + `bevy_enhanced_input` + the nova crates. build
--all-targets, clippy, and fmt all green; behavior unchanged.

## What went well
- Checking the dependency direction *first* turned an open question ("nova_gameplay or
  a dedicated crate?") into a forced answer: the editor uses nova_scenario, which
  already depends on nova_gameplay, so folding it into nova_gameplay would be a cycle.
  A dedicated crate was the only clean home.
- Relocating `GameStates` into a crate both sides already depend on (nova_gameplay),
  and keeping it re-exported through every prelude, meant zero churn for the binary and
  examples that reference it via `nova_protocol::prelude`.
- Sharing the main checkout's `target/` via `CARGO_TARGET_DIR` made the worktree build
  incremental (seconds) instead of a full from-scratch Bevy compile (>10 min, which
  timed out the first attempt).

## What to do differently
- The first worktree build timed out because a sprouted worktree has its own `target/`.
  For future tasks, set `CARGO_TARGET_DIR` to the main checkout's target from the start,
  and run long builds in the background rather than inline (2-min tool timeout).

## Lessons for future tasks
- When deciding where code belongs, map the dependency graph before proposing a home;
  cycles rule out options fast.
- Types shared across the wiring boundary (like `GameStates`) belong in the lowest
  crate that all consumers already depend on, and should stay in the preludes so moving
  them is invisible to callers.
- Worktree builds: always `CARGO_TARGET_DIR=<main>/target` + run in background.
