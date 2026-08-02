# Move the screenshot reel driver into nova_autopilot behind caller hooks

- PRIORITY: 95
- TAGS: v0.10.0, tooling, autopilot, screenshot
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183346

## Story

Move the multi-shot reel driver out of `nova_debug` into
`nova_autopilot::reel`. The crate owns beat sequencing: wait for the scene to be
ready, apply the beat, settle, capture, wait for the PNG to land, advance, then
report done. Everything Nova-shaped - posing the scenario camera, freezing
rigid bodies, hiding the HUD - becomes a caller-supplied hook, so the reel
carries no `nova_scenario` or `avian3d` dependency. Armed by `NOVA_REEL`.

## Steps

- [ ] Port `ScreenshotReelPlugin`, `ReelBeat`, the shot-dir path resolution
      (`NOVA_SHOT_DIR`), and `capture_window`, replacing the scenario-camera,
      rigid-body, and HUD reach-ins with a ready predicate and per-beat apply
      hook. Rename the arming env to `NOVA_REEL`.
- [ ] Add App-driven tests: beats run in order, a beat never captures before the
      previous PNG lands, the reel waits for the ready predicate, and completion
      is negotiated rather than a direct success exit.

## Definition of Done

- Beats are serialized: no capture starts before the previous one lands.
  (test: `reel_beats_are_serialized_on_capture`)
- The reel waits for the caller's ready predicate before the first beat.
  (test: `reel_waits_for_the_scene_to_be_ready`)
- The reel reports done to the completion protocol instead of exiting itself.
  (test: `reel_negotiates_completion`)
- The crate still names no Nova or game-physics dependency.
  (cmd: `! rg -n "nova_|bevy_common_systems|avian3d" crates/nova_autopilot/Cargo.toml`)

## Notes

- Parent: `20260802-120019`. Depends on the completion and screenshot ports.
- Today's reel writes `AppExit::Success` directly; folding it into the
  completion protocol is the behavior change this port makes deliberately.
- Source: `crates/nova_debug/src/harness.rs` (`ScreenshotReelPlugin`).
