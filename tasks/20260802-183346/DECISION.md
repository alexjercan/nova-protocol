# Decision: Port the single-shot screenshot driver into nova_autopilot

- DATE: 20260802-203718
- STATUS: ACCEPTED
- TASK: 20260802-183346
- TAGS: autopilot, screenshot, testing

## Context

Three forks in this port are load-bearing and would otherwise be re-litigated.

1. The BCS source hides the debug overlay by reaching into
   `debug::inspector::DebugEnabled`. `nova_autopilot` may depend on `bevy` and
   nothing else (epic `20260802-120019` design constraints), so the reach-in
   cannot survive the move. Nova's own hider is three separate `DebugEnabled`
   resources (`nova_debug::DebugEnabled` plus the BCS inspector and wireframe
   ones), already collected in `nova_debug::harness::hide_dev_overlays`.
2. The stand-down path ("`NOVA_SHOT` and `NOVA_AUTOPILOT` both set, defer to
   the autopilot") is decided in `Plugin::build` from process-global env vars.
   The sibling port's test rig (`autopilot.rs`) arms its env once per test
   BINARY with a `Once`, precisely because parallel test threads share one
   environment. Setting `NOVA_AUTOPILOT` from inside the screenshot lib-test
   binary would make every OTHER screenshot test stand down and fail.
3. A real capture needs a render app: without `RenderPlugin` nothing ever
   triggers `ScreenshotCaptured`, so a headless test cannot observe the
   done-report by running the app alone. `ScreenshotCaptured` is a public
   struct and `save_to_disk` operates on a plain `Image` with no GPU
   (`bevy_render-0.19.0/src/view/window/screenshot.rs:134`).

## Decision

1. The overlay hide becomes a caller-supplied hook,
   `ScreenshotPlugin::hide_overlay(impl Fn(&mut World) + Send + Sync + 'static)`,
   stored as an `Arc` and run as an exclusive `Startup` system. `&mut World`
   rather than a Bevy system, because that is the shape `AutopilotPlugin::input`
   already established in this crate and it stores in a plain field without a
   marker generic on the plugin type. `nova_debug` passes a closure that calls
   its existing `hide_dev_overlays`.
2. The stand-down test lives in its own integration binary,
   `crates/nova_autopilot/tests/screenshot_stand_down.rs`, which sets BOTH env
   vars at the top of its single test. One test per process means no race, and
   it asserts only the public surface: no `HarnessCompletion` resource exists,
   so nothing registered.
3. The settle test drives the app headlessly to the capture spawn, then
   synthesizes the render side by triggering `ScreenshotCaptured` on the spawned
   entity with a 1x1 `Image`. Both observers fire for real: `save_to_disk`
   writes a PNG under `std::env::temp_dir()` and the crate's observer reports
   done. This proves the ordering the module claims - the PNG is on disk before
   the run reports done - which a spawn-timing-only assertion cannot.

## Alternatives considered

- **Keep an overlay-hide system in the crate, gated on a resource the caller
  inserts.** Would mean `nova_autopilot` owns a `DebugEnabled`-shaped type that
  only `nova_debug` populates - a Nova concept living in the standalone crate,
  which is the exact reach-in the epic is undoing.
- **Take the hook as `impl IntoScheduleConfigs<...>`** so callers pass
  `hide_dev_overlays` directly. Cleaner at the call site, but the marker
  generic has to be erased into a field, adding a wrapper type for one caller.
  Deferred under KISS; the `&mut World` closure already matches `input`.
- **Test the stand-down through a pure predicate**
  (`fn stands_down(shot, autopilot) -> bool` called from `build`). Tests trivia
  rather than the plugin: a predicate can be right while `build` ignores it.
  The extra process is cheaper than the extra seam.
- **Assert only that the `Screenshot` entity spawns on the settle frame** and
  leave the done-report untested. Cheapest, but the module's whole reason for
  the two-observer arrangement is that done must not fire before the PNG
  lands - the DoD criterion would go unproven. Kept as the fallback if the
  synthesized `Image` cannot round-trip through `try_into_dynamic`; that
  fallback costs the ordering guarantee and would be recorded in the retro.
- **Do nothing / leave the driver in BCS.** Blocks the rest of the epic: the
  example, the migration, and the BCS retirement all need this module.

## Consequences

- `nova_debug` must now wire the overlay hide explicitly in its preset
  (`nova_screenshot()`), which is one more line but makes the Nova-specific
  behavior visible at the seam instead of hidden in a distant `build`.
- A caller that forgets `hide_overlay` gets a capture with overlays in frame
  and no error. That is the price of the hook; the preset is the one place
  that has to remember, and the example fleet goes through the preset.
- The crate grows a `tests/` directory for the first time, so the module's
  proof command is `cargo test -p nova_autopilot screenshot`, not `--lib`.
  Anything filtering on `--lib` silently skips the stand-down test.
- The settle test writes a real file to the temp dir on every run. It is
  small, uniquely named per test, and never read back except for existence.

## Amendment 1: every App-driven screenshot test leaves the lib binary

- DATE: 20260802-214500
- STATUS: ACCEPTED

Decision 2 scoped the env hazard too narrowly. It ruled that the screenshot
tests must not SET `NOVA_AUTOPILOT`, and it was right, but the hazard runs the
other way too: `autopilot.rs`'s own `arm()` sets `NOVA_AUTOPILOT=1` for the
whole lib-test binary. So when the unfiltered suite runs, `ScreenshotPlugin`
stands down inside every screenshot lib test, and each one exercises a plugin
that added nothing. Filtering on `screenshot` hid it - the four tests passed
under the DoD command and failed the first time the full suite ran.

The fix is the same shape as decision 2, one level up: ALL App-driven
screenshot tests move to `crates/nova_autopilot/tests/screenshot.rs`, whose
`arm()` sets only `NOVA_SHOT`. Two follow-on adjustments, both small:

- `MAX_WAIT_FRAMES` becomes `pub` - the give-up test lives outside the crate
  now, and hardcoding 1800 in the test would let the bound drift silently.
- `unreached_target_state_error_exits` cannot order its reset system
  `.after(screenshot_drive)` (private), so it writes `NextState` from
  `PostUpdate` instead. Same guarantee, and arguably clearer: the write lands
  after the driver's within the frame, so the next `StateTransition` applies
  it and the target is provably never reached.

Rejected: making `screenshot_drive` public just for the ordering. It is
internal, and `PostUpdate` gets the determinism without widening the surface.

Rejected: dropping `autopilot`'s `arm()` in favour of per-test env
manipulation. That reintroduces exactly the parallel-thread race decision 2
exists to avoid, in a module this task does not own.
