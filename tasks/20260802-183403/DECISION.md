# Decision: nova_debug is the Nova adapter layer that fills nova_autopilot's hooks

- DATE: 20260803-085556
- STATUS: ACCEPTED
- TASK: 20260802-183403
- TAGS: autopilot, crates, examples

## Context

The four driver ports (`20260802-183340` .. `20260802-183349`) deliberately cut
every Nova reach-in out of the drivers and replaced it with a caller hook:
`ScreenshotPlugin::hide_overlay`, `ScreenshotReelPlugin::hide_overlay`,
`ScreenshotReelPlugin::ready`, and `ReelBeat::apply`. `ReelCamera` was dropped
from the crate outright, because a camera pose expressed as position + look-at
means nothing without `ScenarioCameraMarker` and `ScriptedCameraPose`, which are
`nova_scenario` types. Body freezing (`RigidBody::Dynamic -> Static`) never
reached driver state at all and stayed behind as an ordinary system.

Somebody has to fill those hooks. The consumer side is ~25 examples plus
`tests/examples_smoke.rs`, all reaching the harness through
`nova_protocol::prelude::*` -> `nova_core::prelude` -> `nova_debug::prelude`.
The activation envs rename in the same commit with no compatibility aliases, so
whatever shape this takes has to land atomically.

## Decision

`nova_debug::harness` becomes the Nova adapter layer over `nova_autopilot`, and
the example fleet keeps talking to `nova_debug`, not to the crate.

Concretely: `nova_debug` re-exports the crate's plugin types and
`capture_window`; keeps `ReelCamera`, `reel_pose_camera`, `hide_dev_overlays`,
`reel_freeze_bodies` and the `ScenarioLoaded` smoke assertion; and adds two
adapters that pre-fill the hooks - `reel_beat(camera, path)` (pose into
`ReelBeat::apply`) and `nova_reel(beats)` (a plugin adding
`ScreenshotReelPlugin` with `ready` + `hide_overlay` wired, plus the freeze
system). `nova_screenshot()` gains `.hide_overlay(hide_dev_overlays)`.
`hide_dev_overlays` is rewritten as an exclusive `fn(&mut World)` so the single
function serves both the `Startup` registration four screenshot examples already
use and the crate's `Fn(&mut World)` hook signature.

This is what we would build from scratch today: `nova_debug` already existed as
the Nova-shaped presets layer over a generic harness (that was its whole job
under BCS), and the hooks are precisely the seam the ports carved. The adapter
functions have real callers in this task - not speculative extension points.

Cost accepted: two example call sites change vocabulary
(`ScreenshotReelPlugin::new(beats)` -> `nova_reel(beats)`,
`ReelBeat::new(ReelCamera::new(..), path)` -> `reel_beat(ReelCamera::new(..),
path)`) in `screenshot_reel.rs` and `screenshot_sections.rs`. Everything else in
the fleet changes only env-var strings and doc recipes.

A second, smaller choice rides along: `nova_gameplay::settings::HarnessMute`
keeps its env list as string literals (`"NOVA_AUTOPILOT"`, `"NOVA_SHOT"`,
`"NOVA_REEL"`) rather than importing the crate's consts. `nova_gameplay` is a
shipping crate; taking a dev-tooling dependency to deduplicate three strings
buys less than it costs, and the DoD's absence grep catches a missed rename.

## Alternatives considered

- **Every example fills its own hooks.** Each reel example would pass its own
  `ready`/`hide_overlay`/`apply` closures. Rejected: it duplicates the same four
  Nova closures across the fleet, moves `nova_scenario` knowledge into example
  files, and turns an env rename into a 25-file rewrite - exactly the atomicity
  risk this task is trying to keep small.
- **Push the hooks back into `nova_autopilot`.** Rejected outright: it re-adds
  the `nova_scenario` / `avian3d` / `nova_gameplay` dependencies the epic's
  "almost standalone" constraint exists to prevent, and it was the whole point
  of the four ports.
- **Keep `ReelBeat::new(camera, path)` by re-declaring a Nova `ReelBeat` that
  converts.** Rejected: two types with one name, and the conversion is a
  closure the crate already accepts. `reel_beat` is one function against one
  type.
- **`nova_gameplay` depends on `nova_autopilot` for the mute env consts.**
  Acyclic and would kill the drift risk, but it puts a dev-tooling crate in the
  shipped dependency graph for three string literals. Deferred; the absence grep
  is the guard.
- **Do nothing / stage the rename behind aliases.** Rejected by the epic: a
  half-renamed tree that still boots hides exactly the misses the hard rename
  surfaces immediately.

## Consequences

Easier: `nova_debug` shrinks to Nova-shaped adapters with no driver internals -
`ReelState`, `reel_drive`, `reel_resize_window` and the local `capture_window`
twin all delete. The example fleet's import surface does not move. `nova_probe`
stops reaching through `nova_gameplay::bevy_common_systems` for the completion
protocol and names its dependency directly.

Harder: there is now one more indirection between an example and the driver it
runs - reading `nova_reel` means reading `nova_debug` and then
`nova_autopilot::reel`. Anyone adding a reel hook has to decide which layer owns
it, and the honest answer ("Nova types -> `nova_debug`, driver state ->
`nova_autopilot`") is a judgement call, not a rule the compiler enforces. The
duplicated env strings in `nova_gameplay` can drift if a future rename skips the
absence grep.
