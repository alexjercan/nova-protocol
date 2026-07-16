# Retro: Debug F12 screenshot to Downloads

- TASK: 20260716-114125
- BRANCH: debug-screenshot
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Reused the exact primitive the codebase already had
  (`Screenshot::primary_window()` + `save_to_disk`, plus the `dirs` workspace
  dep and the `toggle_debug_mode` input pattern), so the feature was ~135 lines
  of new code with no new abstractions. Reading `harness.rs` first paid for
  itself immediately.
- Verified the keybind was free (`grep KeyCode::F12`) and the feature-gate
  isolation (default + `--features debug` both compile) before landing, so the
  review had nothing structural to catch.
- The one genuine judgment call - keeping the capture OUT of the `DebugSystems`
  run-condition set so F12 works with overlays toggled off - was made
  deliberately by reading how the F11 gate is wired, not by reflex.

## What went wrong

- Nothing blocking. The single review NIT (R1.1): `dirs` was added
  unconditionally in `nova_debug/Cargo.toml`, while `nova_assets` gates it
  native-only. Root cause: copied the dep line without checking the neighbor's
  target-gating. Judged acceptable (nova_debug never compiles for wasm), but the
  convention mismatch is a small readability tax a neighbor check would have
  surfaced pre-review.

## What to improve next time

- When adding a system to a plugin that already partitions systems into gated
  sets (a `run_if` SystemSet), decide the target set explicitly and write down
  why - the default of "it's a debug thing, put it in the debug set" would have
  been wrong here.
- When copying a dependency line, glance at how sibling crates declare the same
  dep (target-gating, features) before pasting.

## Action items

- [x] Ledger lesson `pick-the-system-set-seam` recorded.
- No follow-up tasks: the dirs-gating NIT was consciously left as-is (a wasm
  `cfg` branch for a build that does not exist would be needless complexity).
