# Wire BCS autopilot + screenshot harness into nova examples

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, test, harness

Infrastructure task that unblocks the "examples as tests" goal. bevy-common-systems
ships env-gated developer plugins (`AutopilotPlugin<S>` behind `BCS_AUTOPILOT`,
`ScreenshotPlugin<S>` behind `BCS_SHOT`) that drive a game's state machine headlessly,
log transitions, and exit cleanly with no panic. See `docs/dev-harness.md` in
`~/personal/bevy-common-systems`. Nova already depends on bevy_common_systems, so we
should reuse this harness instead of hand-rolling one per example.

Goal: any nova example can be run headless as a smoke test that asserts "reaches
Playing, runs N seconds, no panic", and can emit a screenshot for docs.

## Steps

- [x] Confirm the autopilot/screenshot plugins are exported from the bevy_common_systems
      prelude at the pinned rev; if not, coordinate exposing them there first.
      RESULT: reachable at pinned rev `47548cd` under `bevy_common_systems::debug::harness`
      (and its `prelude`), behind the `debug` feature. Not in the top-level prelude, but
      that path is public - no cross-repo change needed.
- [x] Add a small nova-side helper (`crates/nova_debug/src/harness.rs`) exposing
      `nova_autopilot() -> AutopilotPlugin<GameStates>` and
      `nova_screenshot() -> ScreenshotPlugin<GameStates>`, re-exported through the
      nova_debug/nova_core/nova-protocol preludes.
      NOTE: nova's `Loading -> Playing` is asset-gated, not input-gated, so the autopilot
      must NOT force `Playing` (would panic pre-`GameAssets` or double-run `OnEnter(Playing)`).
      It holds `Loading` on a single generous step and lets the loader reach `Playing`;
      `DebugPlugin` logs `nova harness: reached Playing` (under the autopilot) so a stalled
      run fails instead of falsely passing.
- [x] Gate it behind the existing `debug` feature so normal runs pay nothing. The bcs
      plugins already env-gate in `Plugin::build` and exit via `AppExit::Success`.
- [x] Add the per-example input closure hook - `nova_autopilot().input(|world, t| ...)`
      passes straight through to the bcs builder (documented, gate input to Playing).
- [x] Document the invocation in `docs/2026-07-07-example-harness-wiring.md`.
- [x] Prove it on 03_scenario: `BCS_AUTOPILOT=1 cargo run --example 03_scenario --features debug`
      reaches Playing and exits with no panic (verified under Xvfb).

## Resolution

Wired the bevy_common_systems env-gated harness into nova via a new
`crates/nova_debug/src/harness.rs` (`nova_autopilot()` / `nova_screenshot()`),
re-exported through the nova_debug/nova_core/nova-protocol `debug`-gated preludes.
Example `03_scenario` adds both under `#[cfg(feature = "debug")]`. `DebugPlugin`
emits `nova harness: reached Playing` under the autopilot so a stalled run fails
instead of falsely passing. Documented in `docs/2026-07-07-example-harness-wiring.md`.

Key design call: nova's `Loading -> Playing` is asset-gated, so `nova_autopilot`
holds `Loading` on a single step and lets the loader reach `Playing` rather than
force-setting it (which would panic pre-`GameAssets` or double-run `OnEnter(Playing)`).

Verified (Xvfb + real GPU render node):
- `BCS_AUTOPILOT=1 ... --example 03_scenario --features debug` -> `nova harness: reached
  Playing`, `autopilot: cycle complete, no panic (t=6.0s)`, exit 0, no panic signatures.
- `BCS_SHOT=800x600 ...` -> `screenshot.png` (800x600 PNG), exit 0.
- Normal `cargo build --examples` (no debug): harness cfg's out, green.
- `cargo clippy --features debug --lib --examples`: clean.

No cross-repo change to bevy_common_systems was needed.

## Notes

This is a prerequisite for tasks 20260707-095008 (turret range), 20260707-100001
(torpedo range), and 20260525-133005 (convert examples into integration tests) - the
same env-gated driver is what lets an example run as a `#[test]`.
