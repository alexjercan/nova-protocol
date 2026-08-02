# Port the single-shot screenshot driver into nova_autopilot

- STATUS: OPEN
- PRIORITY: 96
- TAGS: v0.10.0, tooling, autopilot, screenshot
- KIND: TASK
- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183340

## Story

Port the single-shot capture driver into `nova_autopilot::screenshot`:
`ScreenshotPlugin<S>` forces a window resolution, advances to a target state,
waits N settled frames, writes a PNG, and reports done. Armed by `NOVA_SHOT`; a
`WxH` value overrides the resolution. Stands down when the autopilot is also
armed, since both drive `NextState`.

## Steps

- [ ] Port `bevy-common-systems/src/debug/harness/screenshot.rs` into
      `crates/nova_autopilot/src/screenshot.rs`: `MAX_WAIT_FRAMES`,
      `ScreenshotPlugin<S>` with `new`/`settle_frames`/`path`/`resolution`, the
      private `ScreenshotConfig<S>` resource, `resize_window`,
      `screenshot_drive`, and `parse_resolution`. Declare
      `pub const SCREENSHOT_ENV: &str = "NOVA_SHOT";` in the module (matching
      `autopilot::AUTOPILOT_ENV`) and import `AUTOPILOT_ENV` from
      `crate::autopilot` for the stand-down check. Register with
      `completion::register(app, completion::SCREENSHOT)`. Rewrite the module
      `//!` docs for the crate: `NOVA_SHOT`, a `nova_autopilot::screenshot`
      import path, and no reference to the BCS harness module.
- [ ] Replace the `hide_debug_overlay` reach-in with a caller hook:
      `ScreenshotPlugin::hide_overlay(impl Fn(&mut World) + Send + Sync + 'static)`
      stored as `Option<Arc<dyn Fn(&mut World) + Send + Sync>>` and, when set,
      added as an exclusive `Startup` system. Drop the
      `super::super::inspector::DebugEnabled` import entirely. See
      `DECISION.md` for why a closure and not a Bevy system.
- [ ] Delete the outer `/// The settled-frame screenshot driver.` line above
      `pub mod screenshot;` in `crates/nova_autopilot/src/lib.rs`. The module
      now carries `//!` docs with intra-doc links, and the outer doc would
      concatenate ahead of them and re-resolve those links in `lib.rs` scope -
      the defect `20260802-183340` REVIEW.md R1.3 already fixed for
      `autopilot`/`completion`.
- [ ] Port the three `parse_resolution` unit tests verbatim
      (`parses_valid_wxh`, `rejects_non_resolution_values`,
      `rejects_non_positive_dimensions`).
- [ ] Add the App-driven lib tests, reusing `autopilot.rs`'s rig shape
      (`MinimalPlugins + StatesPlugin`, `TimeUpdateStrategy::ManualDuration`,
      a `Once`-guarded `arm()`, a per-frame `AppExit` drain). Arm this binary
      with `NOVA_SHOT=390x844` so the env's `WxH` override is exercised for
      real; tests that do not spawn a window are unaffected because
      `resize_window` no-ops when `windows.single_mut()` is `Err`.
      - `screenshot_env_resolution_pins_the_primary_window`: spawn
        `(Window::default(), PrimaryWindow)` before adding the plugin, run one
        frame, assert the resolution is 390x844 and `resizable` is `false`.
      - `screenshot_reports_done_after_settling`: `.settle_frames(4)` and a
        `.path()` under `std::env::temp_dir()`. Assert no entity carries
        `Screenshot` before the settle frame; once one does, trigger
        `ScreenshotCaptured { image, entity }` on it with a 1x1
        `Rgba8UnormSrgb` `Image`, then assert the PNG exists on disk AND
        `HarnessCompletion::is_pending(completion::SCREENSHOT)` is false.
        Decide: if the synthesized `Image` cannot round-trip through
        `try_into_dynamic`, drop to asserting the spawn frame and that
        completion is still pending, and record the lost ordering guarantee in
        the retro.
      - `unreached_target_state_error_exits`: add a system that force-sets
        `NextState` back to the default every frame, run `MAX_WAIT_FRAMES + 2`
        frames, assert exactly one exit and that it is not `AppExit::Success`.
      - `hide_overlay_hook_runs_before_the_capture`: a hook incrementing a
        counter resource; assert it ran exactly once and before the capture
        spawn.
- [ ] Add `crates/nova_autopilot/tests/screenshot_stand_down.rs` holding the
      single test `screenshot_stands_down_when_the_autopilot_is_armed`: set
      both `NOVA_SHOT` and `NOVA_AUTOPILOT` at the top of the test (own
      process, so no cross-test env race), build an app with
      `ScreenshotPlugin`, run a few frames, and assert no `HarnessCompletion`
      resource exists, no `Screenshot` entity spawned, and the state never
      left the default.

## Definition of Done

- A `WxH` value sets the resolution; a bare toggle or a nonsense value does not.
  (test: `rejects_non_resolution_values`)
- The armed `WxH` value pins the primary window, unresizable, before capture.
  (test: `screenshot_env_resolution_pins_the_primary_window`)
- The capture waits the configured settled frames, and the PNG is on disk
  before the run reports done.
  (test: `screenshot_reports_done_after_settling`)
- An unreachable target state error-exits instead of hanging.
  (test: `unreached_target_state_error_exits`)
- The caller-supplied overlay-hide hook runs, so the crate needs no game
  dependency to clear the frame.
  (test: `hide_overlay_hook_runs_before_the_capture`)
- The driver stands down when the autopilot is also armed, registering nothing.
  (test: `screenshot_stands_down_when_the_autopilot_is_armed`)
- The module is actually ported under the renamed env and its whole suite -
  lib tests plus the stand-down binary - is green. The `rg` guard keeps a
  vacuous zero-test run from passing; `--lib` is deliberately absent because
  it would skip the stand-down test.
  (cmd: `rg -q '^pub const SCREENSHOT_ENV: &str = "NOVA_SHOT";' crates/nova_autopilot/src/screenshot.rs && nix develop --command cargo test -p nova_autopilot screenshot`)

## Notes

- Parent: `20260802-120019`. Depends on the completion port.
- Overlay hiding is Nova-specific (nova, inspector, wireframe `DebugEnabled`);
  it becomes a hook the `nova_debug` preset supplies. `nova_debug` already has
  the public system `harness::hide_dev_overlays` for the closure to call; the
  actual wiring is `20260802-183403`'s migration, not this task.
- Source: `bevy-common-systems/src/debug/harness/screenshot.rs`.
- `Screenshot` is a plain component with no add-hook
  (`bevy_render-0.19.0/src/view/window/screenshot.rs:80`) and its capture
  systems live in the render app, so a headless test can spawn and observe it
  without a GPU and nothing fires `ScreenshotCaptured` on its own.
- No new dependencies: `Image`, `Screenshot`, `ScreenshotCaptured`, and
  `save_to_disk` all come from the existing `bevy` dep, and the temp path from
  `std::env::temp_dir`.
- The DoD `cmd` is red on base: `screenshot.rs` is still the two-line stub, so
  the `rg` guard exits 1 and short-circuits.
- The prelude stays empty this task; `20260802-183355` populates it.

## Decisions

- `DECISION.md` - the overlay hook shape, the separate stand-down test binary,
  and the synthesized `ScreenshotCaptured` trigger.
