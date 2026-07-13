# Review: Wire BCS autopilot + screenshot harness into nova examples

- TASK: 20260707-100002
- BRANCH: feature/v0.4.0-harness-wiring

## Round 1

- VERDICT: APPROVE

Diff delivers the Goal: a one-line, env-gated harness reachable from the example
preludes, proven end to end. Verified independently in the worktree:

- `cargo build --examples` (no `debug`): harness cfg's out, green.
- `cargo clippy --features debug --lib --examples`: clean (only the pre-existing
  `proc-macro-error2` dep future-incompat warning).
- `cargo test --doc -p nova_debug`: the `harness.rs` example compiles (1 passed).
- `BCS_AUTOPILOT=1 ... --example 03_scenario --features debug` (Xvfb): logs
  `nova harness: reached Playing` and `autopilot: cycle complete, no panic
  (t=6.0s)`, exit 0, no panic signatures.
- `BCS_SHOT=800x600 ...`: wrote an 800x600 `screenshot.png`, exit 0.

Every ticked Step is genuinely done and the TASK.md Resolution matches the code.
The asset-gating design call (hold `Loading`, never force `Playing`) is correct
and is the right way to reconcile the generic driver with nova's loader-driven
transition; the `reached Playing` assertion closes the silent-stall gap.

No BLOCKER/MAJOR findings. NITs below are take-it-or-leave-it.

- [x] R1.1 (NIT) crates/nova_debug/src/lib.rs:11 - the prelude re-exports the raw
  `AutopilotPlugin` / `ScreenshotPlugin` types alongside the `nova_autopilot` /
  `nova_screenshot` presets. Only the presets are the intended entry point, and
  `ScreenshotPlugin` shares its name with Bevy's internal
  `bevy::render::view::screenshot::ScreenshotPlugin`, so glob-importing the nova
  prelude next to `bevy::prelude::*` invites a future name clash for anyone who
  names the type directly. Suggest exporting only the two preset fns from the
  prelude and leaving the plugin types reachable via `nova_debug::harness::` for
  the rare bespoke-timeline case.
  - Response: Fixed - prelude now exports only `nova_autopilot` / `nova_screenshot`;
    the plugin types stay reachable via `nova_debug::harness::`. Rebuilt with
    `--features debug`, green.
- [x] R1.2 (NIT) docs/retros/20260707-example-harness-wiring.md:44 - the input-closure
  snippet uses bare `GameStates` without showing the `use nova_gameplay::GameStates;`
  import that the equivalent rustdoc in `harness.rs` includes. Cosmetic; add the
  import line for copy-paste fidelity.
  - Response: Fixed - added the `use nova_gameplay::GameStates;` line to the snippet.

Both NITs resolved; verdict stands at APPROVE.
