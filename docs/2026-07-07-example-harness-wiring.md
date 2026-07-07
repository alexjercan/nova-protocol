# Example smoke-test harness wiring (autopilot + screenshot)

- DATE: 2026-07-07
- TASK: `tasks/20260707-100002`
- BRANCH: `feature/v0.4.0-harness-wiring`

## What this is

`nova_debug::harness` wires the `bevy_common_systems` env-gated developer
plugins into nova so any example can be run as a headless smoke test ("reaches
gameplay, runs a few seconds, exits with no panic") or made to emit a
screenshot, without hand-rolling a driver per example. It reuses the harness
documented in `~/personal/bevy-common-systems/docs/dev-harness.md`
(`AutopilotPlugin<S>` behind `BCS_AUTOPILOT`, `ScreenshotPlugin<S>` behind
`BCS_SHOT`) rather than reinventing it here.

- `nova_autopilot() -> AutopilotPlugin<GameStates>` - drives the run and exits.
- `nova_screenshot() -> ScreenshotPlugin<GameStates>` - captures a PNG.
- Both are re-exported through the `nova_debug` / `nova_core` / `nova-protocol`
  preludes under the `debug` feature, so an example gets them from
  `use nova_protocol::prelude::*;`.

## How to use it

Add the presets once, gated behind `debug` (the harness lives there); they are
inert unless their env var is set, so leaving them in costs nothing in a normal
run:

```rust
let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

#[cfg(feature = "debug")]
{
    app.add_plugins(nova_autopilot());
    app.add_plugins(nova_screenshot());
}

app.run();
```

If the scene needs input (fire, thrust), chain an input closure onto the
autopilot and gate it to the gameplay state:

```rust
use nova_gameplay::GameStates;

app.add_plugins(nova_autopilot().input(|world, _elapsed| {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
}));
```

### Running it

```text
# Headless cycle, check for no panic:
BCS_AUTOPILOT=1 cargo run --example 03_scenario --features debug
#   -> `nova harness: reached Playing`
#   -> `autopilot: cycle complete, no panic`

# Screenshot at a given resolution:
BCS_SHOT=1024x768 cargo run --example 03_scenario --features debug
#   -> writes screenshot.png
```

The app still opens a real window (it uses `DefaultPlugins` via `AppBuilder`),
so a truly headless box needs a virtual display, e.g. `Xvfb :99 & DISPLAY=:99`.

## Why this shape (design decisions)

### The autopilot must not force `Playing`

The central wrinkle is that nova's `Loading -> Playing` transition is
**asset-gated**, not input-gated: the loader flips `GameStates` to `Playing` in
`OnEnter(GameAssetsStates::Loaded)`. The generic `AutopilotPlugin` drives a
state machine by force-setting `NextState` on a timeline, which fights that:

- Force `Playing` too early and it fires before the `GameAssets` resource
  exists, panicking any `OnEnter(Playing)` scene setup that reads it.
- Force `Playing` after the loader already did and it re-enters `Playing`,
  double-running the setup (Bevy re-runs `OnExit`/`OnEnter` even for a
  same-state `NextState::set`, which is why the plugin guards the very first
  transition).

So `nova_autopilot` uses a **single-step timeline**: hold `Loading` for
`NOVA_AUTOPILOT_SECS` (6s) and nothing else. The loader reaches `Playing` on its
own inside that window, the run exercises real gameplay there, and the autopilot
exits when the step ends -- it never touches `NextState`, so it can neither race
nor double-fire. `NOVA_AUTOPILOT_SECS` just has to outlast asset loading; bump
it if a heavier scene needs more settling time.

To keep that from silently passing when the loader never arrives (no panic, but
stuck in `Loading`), `DebugPlugin` logs `nova harness: reached Playing` on
`OnEnter(GameStates::Playing)` when `BCS_AUTOPILOT` is set. A smoke test asserts
on both that line and `autopilot: cycle complete, no panic`.

### The screenshot preset does force `Playing`

`ScreenshotPlugin` has no non-forcing mode -- it sets `NextState` to the target
once on the first frame, then waits for it to apply. For nova that means it can
advance to `Playing` before the loader is done. It is therefore best paired with
examples that build their scene in `OnEnter(GameAssetsStates::Loaded)` (the
nova scenario convention, as `03_scenario` does) rather than
`OnEnter(GameStates::Playing)`. The 30 settle frames give the loader time to
finish and the scene to render before the capture. Screenshots are a
run-by-hand docs aid, so this caveat is acceptable; the autopilot is the
CI-facing path.

### Placement

The helper lives in `nova_debug` because that crate already depends on
`bevy_common_systems` with the `debug` feature (where the harness plugins live)
and on `nova_gameplay` (for `GameStates`). It flows up through the existing
`#[cfg(feature = "debug")]` prelude re-exports, so no new dependency edges were
added. Examples reference it under the same `debug` cfg they would already use
for other debug tooling.

## No changes needed in bevy_common_systems

Step one of the task was to confirm the plugins are reachable at the pinned rev
(`47548cd`). They are: `AutopilotPlugin` / `ScreenshotPlugin` are public under
`bevy_common_systems::debug::harness` (and its `prelude`) behind the `debug`
feature. They are not in the crate's top-level prelude, but that is fine -- we
import them by their `debug::harness` path. No cross-repo change was required.

## Follow-ups

- `20260707-095008` / `20260707-100001` (turret / torpedo test ranges) use
  `nova_autopilot().input(...)` to fire at gates headlessly.
- `20260525-133005` (convert examples into integration tests) can wrap this same
  env-gated run in a `#[test]` that shells out with `BCS_AUTOPILOT=1` and asserts
  on the two log lines.
