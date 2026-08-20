# Running without a GPU: what it cost, and what it bought

DONE 2026-08-20. This file was a scoping note; it is now the record. Where the
scoping was wrong, the wrong version is kept and marked, because two of the
three surprises were things the note asserted were safe.

## What `--norender` is now

Four things, all under one flag:

- `wgpu.backends = None`, so `RenderPlugin` never asks for an adapter. It builds
  no device and no render sub-app - no extract, no prepare, no queue
  (`bevy_render-0.19.0/src/lib.rs:357`).
- `primary_window: None` plus `ExitCondition::DontExit`. The exit condition is
  load-bearing: bevy's default `OnAllClosed` is SATISFIED by zero windows, so
  without it the app exits before the first frame finishes.
- `WinitPlugin` disabled, `ScheduleRunnerPlugin` in its place. `WinitPlugin::build`
  constructs the event loop, which wants a display server whether or not a window
  is ever opened - asking winit for zero windows is not enough for a box with no
  `DISPLAY`. And without a replacement runner `App::run` ticks ONCE and returns,
  because bevy's fallback runner is `run_once`.
- The visual game plugins stay off, as before.

Before this, `--norender` dropped the HUD, NOVA OS, hanabi and the ship/scenario
render bits, and then opened a window and ran the full render sub-app on a real
device. It rendered an emptier scene. It did not stop rendering.

## The design call: ONE switch, and the note had this backwards

The scoping note said **do not overload `with_rendering(false)`** - "it means no
visual game plugins today and callers depend on that". That was checked and it is
false. Every caller:

- `with_rendering(false)`: NOBODY. One call site exists, `editor_app`, and its
  argument comes from `--norender` alone.
- `with_rendering(true)`: also only `editor_app`.
- `editor_app(true, ..)`: six examples (`system_ship_editor`, `system_menu_boot`,
  `bug_menu_picker`, `bug_sandbox_soak`, `screenshot_editor`, `screenshot_menu`,
  `screenshot_scenario_picker`). All want rendering.
- 48 `AppBuilder::new()` call sites across `examples/` and `nova_perf_web`. None
  touches `with_rendering` at all.

So there was no caller whose meaning could change, and nothing anywhere wants "no
visual plugins but still a window". Two flags would have been accretion with no
second reader - `AGENTS.md`'s NEVER BACKWARD COMPATIBLE, exactly.

The halves also cannot be separated even in principle: `bevy_hanabi` panics
outright without a render sub-app (`bevy_hanabi-0.19.0/src/plugin.rs:361`,
`app.sub_app_mut(RenderApp)`), and hanabi is gated on this same flag. Dropping
the device FORCES dropping the plugins that need it.

`with_rendering` is therefore GONE. Rendering is fixed at construction:
`AppBuilder::new()` or `AppBuilder::headless()`. It had to move to the
constructor regardless of the fold - `AppBuilder::new` bakes `DefaultPlugins`
into the app immediately (the `mods://` source must be registered before
`AssetPlugin` lands), so a later setter could not reach the wgpu and window
settings. A setter that silently could not do its job is worse than no setter.

## What actually broke, which was not the renderer

The note's risk assessment - "no game crate uses `RenderDevice`, `RenderQueue`,
`RenderApp` or `ExtractSchedule`, so no first-party system can panic for want of
a GPU" - is now STALE. `nova_probe` names three of them
(`capabilities/framecost.rs:28,146`, `capabilities/frametime.rs:28,783`). It is
stale in the harmless direction: every one is already guarded
(`get_sub_app_mut(...) else return`, `Option<Res<RenderAdapterInfo>>`), and
`frametime.rs:388-392` documents a `--norender` build degrading the adapter to
`unknown` by name. The probe was written for this before it existed.

Nothing render-side panicked. Two other things did, both only findable by
running:

1. **`nova_assets::collections::update_nova_hud_assets` took the whole run down**
   at the end of asset loading. It requires `ResMut<NovaHudAssets>`, a resource
   `nova_hud::NovaHudPlugin` owns - and that plugin is render-gated, so under the
   OLD `--norender` the resource was already absent. This was a live defect in
   the existing flag that nobody had ever booted far enough to hit. Now
   `Option<ResMut<..>>` with an early return, matching what
   `nova_os_ui/src/terminal/spawn.rs:35` already did with the same resource and
   what `nova_menu/src/lib.rs:92` already did for `HudVisibility`.

2. **A genuine bevy 0.19 headless hole.**
   `ExtractComponentPlugin::build` adds `SyncComponentPlugin` UNCONDITIONALLY
   (`bevy_render-0.19.0/src/extract_component.rs:85`), and that plugin's
   `on_remove` hook does an unguarded `world.resource_mut::<PendingSyncEntity>()`
   (`sync_component.rs:55`). The resource ships with `ExtractPlugin`, which a
   backendless `RenderPlugin` never adds. So despawning ANY extracted component
   panics - hit on the loading screen's UI nodes, the first despawn after assets
   finish. Bevy's own headless examples never spawn and despawn anything, which
   is why this has not been reported upstream.

   Worked around by adding the public `SyncWorldPlugin` in the headless branch,
   which supplies the resource on its own. See the limitation below.

## What a user gets from `--norender` today

A silent process. Log lines on stderr and, when armed, the probe's files -
`<label>.json`, `frametime.csv`, `census.json`. No TUI, no window, nothing to
look at, and NO input channel of any kind: winit is gone, so there is not even a
keyboard. The only way to affect a run is the command line and the environment
before it starts.

That makes it a batch job, not a "CLI mode". Anything interactive on top of it -
a structured input channel, a command stream - is new construction, not a
setting: it needs a driver that is not the autopilot and a transport that is not
winit. Nothing in this change blocks that, and nothing in it provides it.

## A bare `--norender` HANGS, and that is correct

There is no window to close and no input, so nothing ever raises `AppExit`. The
app sits in the main menu and ticks forever. This is documented on
`AppBuilder::headless` and in `--help`, and it is NOT a bug - do not go looking
for the deadlock.

The working invocation is a scenario plus a driver:

```
NOVA_AUTOPILOT=1 cargo run --features debug -- --norender --scenario shakedown_run
```

Verified: exit 0, `autopilot: cycle complete, no panic (t=6.0s)`, then
`harness completion: all collectors done, exiting`. With `NOVA_PERF` armed the
same run wrote 900 counted frames, a full main-world schedule breakdown, and
`render_world=0.000ms gpu=0.000ms` - the render columns are zero because there is
no render world, not because they were not measured. `backend` and `adapter`
both read `unknown`, as `frametime.rs:388` says they should.

An unknown `--scenario` id still refuses the launch and exits 1 from a headless
app, so the in-app refusal path survives with no window to report it through.

The RENDERED path was re-run afterwards under Xvfb and is unchanged: real Vulkan
adapter, hanabi initialised, window created, exit 0. That was the regression that
mattered and a headless test could not have caught it.

## Known limitations

- **The sync queue is never drained.** `SyncWorldPlugin` gives us
  `PendingSyncEntity` but the thing that empties it, `entity_sync_system`, lives
  in the render sub-app's extract. One ~24-byte record leaks per synced spawn and
  per synced component removal - about 2.4 MB across 100k of them, linear in run
  length. Fine for a probe-length run, NOT for an indefinite soak. Adding
  `ExtractPlugin` would drain it and would also rebuild the mirror render world
  this flag exists to avoid, so it is the wrong trade. The type is `pub(crate)`,
  so there is no way to clear it from outside bevy.
- **Three noisy bevy lines on every headless boot**, none fatal: an ERROR-level
  `Render app did not exist when trying to add extract_resource for <ClearColor>`,
  a `bevy_gizmos_render` warning, and two `CompressedImageFormatSupport` warnings.
  If a probe gate ever grades on a clean log, these are what it will trip on.
- **The probe reports `resolution: 1280x720`** for a run that has no window. It
  is `DEFAULT_RESOLUTION` falling through. Harmless but misleading in a CSV row.

## CI: could `systems/` ranges run headless

Reported, not acted on.

They could, and the win is real: CI runs them under lavapipe today, where
`stress_torpedoes` takes 48 s against a 180 s timeout, and the software rasteriser
is most of that. A headless run needs no GPU, no ICD, and no X server at all.

Two things stand in the way, neither hard:

1. **Transport.** `--norender` is an argument to the game BINARY. Ranges are
   examples that call `AppBuilder::new()` directly and take no such argument, and
   the probe's `--render gpu|sw` switch is env-only (`native/env.rs:150`). Wiring
   this means an env var that selects `headless()` inside `AppBuilder`. Note that
   is a TRANSPORT question, not a second flag: the meaning stays the one switch
   decided above.
2. **Ranges that assert on render state.** Most `systems/` ranges assert game
   state, but not all - anything reading a camera, a material, or a capture would
   need auditing one by one.

**What would be lost, and it is the whole point of the exercise:** a headless run
stops catching render-side panics. The failures that only appear with a device
are exactly the ones this repo keeps hitting - duplicate-component panics,
material and pipeline breakage, the async-compile SIGSEGV that
`synchronous_pipeline_compilation` exists for. A fully headless `systems/` suite
would be blind to all of it.

So the answer is not "move them", it is "split them". Every range headless for
speed, plus a SMALL fixed set kept rendered as the render-side canary - the
`screenshots/` producers already have to run rendered (they produce pixels), and
`system_ship_editor`, `system_menu_boot` and `system_nova_os` between them cover
UI, menu and the CRT material path. That is roughly a dozen rendered runs
carrying the panic detection for the whole suite, with the rest running headless
in a fraction of the time. Sizing that set properly is its own task.

## Deferred

**No comparative timing was taken.** The box was in use by another measurement
lane for the whole of this work. The headless-vs-rendered frame cost - the "floor"
number this was originally scoped for - is UNMEASURED and was not attempted. The
frame numbers quoted above are evidence the capture pipeline works headless, not
a measurement of anything.

## Docs invalidated, not fixed here (out of lane)

- `docs/development.md:115` describes `--norender` as "build the app with
  rendering off (`editor_app(false)`)". Understated now, and the reader is not
  told the run needs a driver to terminate.
- `docs/architecture.md:146` shows `.with_rendering(true)` in the `AppBuilder`
  chain. That method no longer exists.
- `docs/concept-index.md:78` lists `--norender` under debug tooling. Still true.

## Note that survives

`synchronous_pipeline_compilation: true` is moot without a device and was NOT
deleted. `lib.rs` records that bevy's async default SIGSEGVs one run in five, and
that reason survives this change untouched.
