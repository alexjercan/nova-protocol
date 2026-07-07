# Wire BCS autopilot + screenshot harness into nova examples

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0,test,harness

Infrastructure task that unblocks the "examples as tests" goal. bevy-common-systems
ships env-gated developer plugins (`AutopilotPlugin<S>` behind `BCS_AUTOPILOT`,
`ScreenshotPlugin<S>` behind `BCS_SHOT`) that drive a game's state machine headlessly,
log transitions, and exit cleanly with no panic. See `docs/dev-harness.md` in
`~/personal/bevy-common-systems`. Nova already depends on bevy_common_systems, so we
should reuse this harness instead of hand-rolling one per example.

Goal: any nova example can be run headless as a smoke test that asserts "reaches
Playing, runs N seconds, no panic", and can emit a screenshot for docs.

## Steps

- [ ] Confirm the autopilot/screenshot plugins are exported from the bevy_common_systems
      prelude at the pinned rev; if not, coordinate exposing them there first.
- [ ] Add a small nova-side helper (e.g. in `nova_debug` or an examples `common`
      module) that wires `AutopilotPlugin<GameStates>` / `ScreenshotPlugin<GameStates>`
      with a nova timeline (Loading -> Playing) so each example adds it in one line.
      Nova's states are `GameStates` / `GameAssetsStates` (no Menu/GameOver), so the
      timeline is simpler than the bevy-common-systems games.
- [ ] Gate it behind the existing `debug` feature so normal runs pay nothing, matching
      the bevy-common-systems approach (env-gate in `Plugin::build`, early-return when
      unset, `AppExit::Success` not `std::process::exit`).
- [ ] Add the per-example input closure hook where a scene needs to press fire/thrust
      (turret + torpedo ranges need this).
- [ ] Document the invocation (`BCS_AUTOPILOT=1 cargo run --example ... --features debug`)
      in `docs/`.
- [ ] Prove it on at least one existing example (e.g. 03_scenario) before closing.

## Notes

This is a prerequisite for tasks 20260707-095008 (turret range), 20260707-100001
(torpedo range), and 20260525-133005 (convert examples into integration tests) - the
same env-gated driver is what lets an example run as a `#[test]`.
