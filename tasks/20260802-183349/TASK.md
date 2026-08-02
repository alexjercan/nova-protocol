# Move the screenshot reel driver into nova_autopilot behind caller hooks

- PRIORITY: 95
- TAGS: v0.10.0, tooling, autopilot, screenshot
- KIND: TASK
- ACTIVITY: WORKING
- GATES: PLAN
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

- [ ] Add `pub const REEL: &str = "reel";` to
      `crates/nova_autopilot/src/completion.rs` next to `AUTOPILOT` and
      `SCREENSHOT`, with the same one-line doc shape.
- [ ] Port the driver into `crates/nova_autopilot/src/reel.rs`, replacing the
      module's placeholder `//!` line with real crate-shaped docs (a
      `nova_autopilot::reel` import path, `NOVA_REEL`, no `nova_scenario` /
      `ScenarioCameraMarker` / BCS references, and the two-hook contract). Port
      as-is: `REEL_CAPTURE_RESOLUTION = (1920.0, 1080.0)`, the private
      `ReelWindowSize` / `ReelState` / `ReelCaptureDone` resources,
      `reel_resize_window`, `reel_drive`, and the empty-beats warn-and-return in
      `build`. Rename `SCREENSHOT_REEL_ENV` to
      `pub const REEL_ENV: &str = "NOVA_REEL";` (matching
      `autopilot::AUTOPILOT_ENV` / `screenshot::SCREENSHOT_ENV`). Delete the
      outer `/// The screenshot reel driver...` line above `pub mod reel;` in
      `lib.rs` - the module now carries `//!` docs and the outer doc would
      concatenate ahead of them (`20260802-183340` REVIEW.md R1.3).
- [ ] Port the shot-dir path resolution: `pub const SHOT_DIR_ENV: &str =
      "NOVA_SHOT_DIR";` and a private `capture_path(&str) -> PathBuf` that joins
      a RELATIVE path under it and passes an absolute path through unchanged.
- [ ] Port `pub fn capture_window(world: &mut World, path: &str)` verbatim
      (create the parent dir, warn-and-continue on failure, spawn
      `Screenshot::primary_window()` + `save_to_disk(capture_path(path))`). It
      stays public: the UI/combat/juice capture examples drive their own beats
      from an autopilot closure and call it directly.
- [ ] Replace the three reach-ins with hooks on `ScreenshotReelPlugin`, all
      stored as `Option<Arc<...>>` and cloned into systems at `build` (the
      `ScreenshotPlugin::hide_overlay` shape):
      - `ready(impl Fn(&World) -> bool + Send + Sync + 'static)` replaces the
        `With<ScenarioCameraMarker>` probe in `reel_drive`. Unset means
        immediately ready. See DECISION.md D3 for why it gets no wait backstop.
      - `ReelBeat::apply(impl Fn(&mut World) + Send + Sync + 'static)` replaces
        the `reel_pose_camera` call on beat entry, keeping the existing
        "apply, then give the change a frame before settling" ordering. Drop
        `ReelCamera` and `ReelBeat::new(camera, path)` entirely; the constructor
        becomes `ReelBeat::new(path)` with `settle_frames` defaulting to 30
        (today's `NOVA_SCREENSHOT_SETTLE_FRAMES`, which stays behind in
        `nova_debug`). See DECISION.md D1.
      - `hide_overlay(impl Fn(&mut World) + Send + Sync + 'static)` replaces the
        `hide_dev_overlays` + `reel_hide_hud` startup pair, same name and
        signature as `ScreenshotPlugin::hide_overlay`.
      Do NOT port `reel_freeze_bodies` in any form - DECISION.md D2.
- [ ] Fold the exit into the completion protocol: `completion::register(app,
      completion::REEL)` in `build` after the armed and non-empty-beats checks,
      and on the LAST beat's capture landing call
      `completion.done(completion::REEL)` in place of
      `world.write_message(AppExit::Success)`. Keep `state.done = true` so the
      driver goes inert. `reel_drive` is exclusive, so reach the resource with
      `world.resource_mut::<completion::HarnessCompletion>()`.
- [ ] Port the two pure `capture_path` unit tests from `harness.rs`
      (`reel_capture_path_leaves_bare_and_absolute_paths_alone`) into the
      module's `#[cfg(test)]` block. Nothing else stays in the lib-test binary
      (DECISION.md D5).
- [ ] Add `crates/nova_autopilot/tests/reel.rs`, reusing `tests/screenshot.rs`'s
      rig (`MinimalPlugins + StatesPlugin` is not needed - the reel is not
      state-generic, so `MinimalPlugins` alone), `TimeUpdateStrategy::
      ManualDuration(FRAME)`, a `Once`-guarded `arm()`, a per-frame `AppExit`
      drain, and the 1x1 `Rgba8UnormSrgb` `tiny_image()` helper. `arm()` sets
      BOTH `NOVA_REEL=1` and `NOVA_SHOT_DIR` to a per-binary temp dir, once,
      never with a second value.
      - `reel_beats_are_serialized_on_capture`: three beats with distinct paths
        and an `apply` hook pushing its beat index onto a process-static `Mutex<
        Vec<usize>>`. Run frames; assert exactly one `Screenshot` entity exists
        at a time and that no second capture spawns until the first beat's
        `ScreenshotCaptured` is triggered. Drive all three to completion and
        assert the recorded apply order is `[0, 1, 2]` and the three PNGs exist
        under `NOVA_SHOT_DIR` - which also proves the relative-path join.
      - `reel_waits_for_the_scene_to_be_ready`: a `ready` predicate reading a
        test-owned resource that stays `false`. Run well past the beat's
        settle-frame count, assert no `Screenshot` entity and no apply-hook
        call; flip the resource, run again, assert the first beat now runs.
      - `reel_negotiates_completion`: one beat. Before the capture lands assert
        `HarnessCompletion::is_pending(completion::REEL)`; trigger
        `ScreenshotCaptured`, then assert it is no longer pending AND that no
        `AppExit` was written BY THE REEL on that frame - the watcher in `Last`
        owns the exit. Assert the observed exit is `AppExit::Success` and that
        it came from the watcher by also registering a second collector that is
        still pending at capture time and checking no exit appears until it too
        reports done.

## Definition of Done

- Beats are serialized and run in order: no capture starts before the previous
  one lands, and relative paths land under `NOVA_SHOT_DIR`.
  (test: `reel_beats_are_serialized_on_capture`)
  (cmd: `nix develop --command cargo test -p nova_autopilot --test reel reel_beats_are_serialized_on_capture`)
- The reel waits for the caller's ready predicate before the first beat.
  (test: `reel_waits_for_the_scene_to_be_ready`)
  (cmd: `nix develop --command cargo test -p nova_autopilot --test reel reel_waits_for_the_scene_to_be_ready`)
- The reel reports done to the completion protocol instead of exiting itself,
  and a second pending collector holds the exit open.
  (test: `reel_negotiates_completion`)
  (cmd: `nix develop --command cargo test -p nova_autopilot --test reel reel_negotiates_completion`)
- The whole crate still passes, module unit tests included.
  (cmd: `nix develop --command cargo test -p nova_autopilot`)
- The crate still names no Nova or game-physics dependency. Anchored so the
  crate's own `name = "nova_autopilot"` line does not match, and `test -f`
  keeps a missing manifest from passing vacuously.
  (cmd: `test -f crates/nova_autopilot/Cargo.toml && ! rg -n '^(nova_|bevy_common_systems|avian3d)' crates/nova_autopilot/Cargo.toml`)
- The reel module reaches for nothing Nova-shaped.
  (cmd: `! rg -n 'nova_scenario|nova_gameplay|avian3d|bevy_common_systems|ScenarioCamera|HudVisibility|RigidBody' crates/nova_autopilot/src/reel.rs`)

## Notes

- Parent: `20260802-120019`. Depends on the completion and screenshot ports.
- Today's reel writes `AppExit::Success` directly; folding it into the
  completion protocol is the behavior change this port makes deliberately.
- Source: `crates/nova_debug/src/harness.rs:216-561` (`ScreenshotReelPlugin`
  and friends).
- Decisions in `DECISION.md`: D1 apply-hook replaces `ReelCamera`, D2 no
  body-freeze hook at all, D3 no wait backstop on the ready predicate, D4 no
  stand-down under `NOVA_AUTOPILOT`, D5 tests in their own binary, D6 the
  completion change.
- The old DoD guard (`! rg -n "nova_|..." Cargo.toml`) was UNRUNNABLE - `nova_`
  matches the crate's own `name = "nova_autopilot"` line, so it was red on base
  for the wrong reason and could never go green. Replaced with the epic's
  anchored form (verified: exit 0 on base).
- Proof state on base (verified): the three `--test reel` cmds are RED (no
  `tests/reel.rs`). The two `rg` cmds are GREEN - they are regression guards on
  a boundary this task must not breach, not red-to-green proofs. The
  `cargo test -p nova_autopilot` cmd is green today and stays green.
- This task DELETES nothing from `nova_debug`. `harness.rs` keeps compiling
  against its own copy until `20260802-183403` migrates the callers; that is
  what keeps this landable on its own.
- The old `NOVA_SCREENSHOT_SETTLE_FRAMES = 30` and the `reel_pose_camera` /
  `hide_dev_overlays` / `reel_freeze_bodies` bodies stay in `nova_debug` and
  become the hook closures in `20260802-183403`.
- ASSUMPTION: `reel_drive`'s exclusive-system shape survives the port. It needs
  `&mut World` for the apply hook and the capture spawn, so
  `world.resource_scope::<ReelState, _>` stays. Confirm the hooks can be called
  inside `resource_scope` (they take `&mut World` / `&World`, and `ReelState` is
  out of the world for the duration, so they must not touch it - they cannot,
  it is private).
