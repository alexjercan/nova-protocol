# Decision: Move the screenshot reel driver into nova_autopilot behind caller hooks

- DATE: 20260802-183349
- STATUS: ACCEPTED
- TASK: 20260802-183349
- TAGS: autopilot, screenshot, reel, testing

## Context

`nova_debug::harness`'s reel driver reaches into three Nova-shaped things the
crate may not depend on (epic `20260802-120019` design constraints: `bevy` and
nothing else):

1. Camera posing - `reel_pose_camera` pins `ScriptedCameraPose` on the
   `ScenarioCameraMarker` entity and strips `WASDCameraController`, three types
   from `nova_scenario` and `bevy_common_systems`. It is also the readiness
   probe: `reel_drive` waits for a `With<ScenarioCameraMarker>` entity before the
   first beat.
2. Body freezing - `reel_freeze_bodies` rewrites every `avian3d`
   `RigidBody::Dynamic` to `Static` every frame.
3. Chrome hiding - `hide_dev_overlays` (three `DebugEnabled` resources) plus
   `reel_hide_hud` (`nova_gameplay::HudVisibility`).

The reel also writes `AppExit::Success` itself, which the completion protocol
landed in `20260802-183340` forbids for registered collectors.

## Decision

**D1 - the per-beat hook REPLACES `ReelCamera`, it does not wrap it.**
`ReelBeat` carries `apply: Option<Arc<dyn Fn(&mut World) + Send + Sync>>` and no
camera field; `ReelCamera` and `ReelBeat::new(camera, path)` are dropped.
`ReelBeat::new(path)` is the constructor.

**D2 - body freezing does not become a hook at all.** `reel_freeze_bodies` never
read `ReelState`; it is an ordinary `Update` system with no beat-sequencing
dependency, so `nova_debug` adds it to its own `Update` in `20260802-183403`.

**D3 - the ready predicate is `Fn(&World) -> bool`, defaulting to always-ready,
with no wait backstop of its own.** `&World`, not `&mut World`: a predicate that
mutates is a beat, not a gate.

**D4 - the reel does NOT stand down under `NOVA_AUTOPILOT`,** unlike
`ScreenshotPlugin`.

**D5 - the App-driven tests live in `crates/nova_autopilot/tests/reel.rs`,** not
the lib-test binary. Only the pure `capture_path` tests stay in the module.

**D6 - the last beat reports `completion::REEL` done instead of writing
`AppExit::Success`.** The watcher decides the exit.

## Alternatives considered

**D1: keep `ReelCamera { position, look_at }` in the crate and hand it to the
hook.** Rejected - the crate cannot act on a pose (acting means the three
`nova_scenario`/BCS types), so the struct would be pure ceremony wrapped around
the hook that does the real work. Cost accepted: `screenshot_reel` and
`screenshot_sections` beat lists get wordier. `20260802-183403` absorbs that with
a `nova_debug` helper (`reel_camera_beat(eye, look, path)`) closing over
`reel_pose_camera` - which is exactly the Nova-shaped adapter this crate is
refusing to hold.

**D2: add a per-frame `each_frame(impl Fn(&mut World))` hook to carry the
freeze.** Rejected - a new plugin concept with no requirement behind it, since
the caller can already add the system directly. YAGNI.

**D3: port a `MAX_WAIT_FRAMES` twin for the ready predicate** (the BCS source
hangs forever when the scenario camera never spawns). Rejected - the reel now
registers with the completion protocol, whose deadline
(`NOVA_AUTOPILOT_DEADLINE`, default 120s) already error-exits NAMING `reel` as
the laggard. A second timeout is a second knob to tune and a second failure
message for one failure.

**D4: mirror `ScreenshotPlugin`'s stand-down.** Rejected - the screenshot driver
stands down because both it and the autopilot write `NextState`. The reel writes
no state, and the capture examples deliberately run `NOVA_AUTOPILOT=1
NOVA_REEL=1` together (the autopilot scripts the UI beats while the reel is
armed) - see `scripts/gen-web-screenshots.py`. The old exit fight is precisely
what D6 resolves, so there is nothing left to stand down from.

**D5: keep the tests in the lib-test binary.** Rejected - the reel must arm
`NOVA_REEL` and pin `NOVA_SHOT_DIR` process-wide for the whole binary, and
`NOVA_SHOT_DIR` would then leak into the `screenshot` module's unit tests.
`20260802-183346` already paid for this lesson the expensive way.

## Consequences

- Positive: `crates/nova_autopilot/src/reel.rs` names nothing Nova-shaped, so the
  epic's standalone constraint holds for the last driver.
- Positive: a reel can run alongside the autopilot without either truncating the
  other - the bug class `20260802-183340` cites (an 11-frames-short capture
  losing 229 samples downstream).
- Positive: a never-ready scene now fails with a named laggard instead of hanging
  until an outer supervisor SIGKILLs it.
- Negative: `20260802-183403` inherits more adapter work than the earlier ports -
  the beat helper (D1), the freeze system re-add (D2), and the combined
  overlay+HUD hide closure. Recorded so that task is not sized as a rename.
- Neutral: `nova_debug::harness` keeps its own copy compiling until
  `20260802-183403`, which is what keeps this task landable on its own.
