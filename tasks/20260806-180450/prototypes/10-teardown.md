# Prototype 10 - delete the dependency

Nothing is copied here. This step removes the re-export, the last stragglers,
and the manifest entries, and proves the dep is gone.

**Do not start until 01-09 have all landed.** Every name must already have a
nova home.

## 10a. `nova_probe` - the last direct reach

`crates/nova_probe/src/recorder.rs` still names BCS through the `nova_gameplay`
re-export:

| Line | Current |
|---|---|
| 49 | `use nova_gameplay::{bevy_common_systems::modding::events::GameEvent, GameStates, PauseStates};` |
| 478 | `use nova_gameplay::bevy_common_systems::modding::events::GameEventInfo;` (test-local) |

Both become `nova_events::engine::{GameEvent, GameEventInfo}` (prototype 01).

**Add `nova_events` to `crates/nova_probe/Cargo.toml` as a direct dependency.**
Owner's ruling: the recorder's job is to record game events, so the event
vocabulary is a first-class dep of `nova_probe`, not something to reach through
`nova_gameplay`'s re-export. This is one of the two intended new graph edges
for the whole task (the other is `nova_events -> nova_events_macros`).

Rewrite the manifest comments at `Cargo.toml:37-44` while you are there - they
currently explain the re-export routing ("re-exported through nova_gameplay, so
the bcs version stays unified") and that reasoning is retired.

`recorder.rs:26` has a doc line about accessors that "landed in
bevy_common_systems" - reword.

### Already done, do not redo

NOTES.md lists `invariants.rs:44` and `capture.rs:25` as a straggler to fix.
**They were fixed on this branch in commit `261c7e71`.** `capture.rs:20-26` now
carries a comment pinning the `nova_gameplay::prelude::Health` path and saying
why naming any other path is how the query silently stopped matching.
`invariants.rs:44` reads `nova_gameplay::{flight::FlightSpeedCap, prelude::Health}`.
Confirm both still hold; do not re-fix.

## 10b. `nova_gameplay/src/lib.rs` - delete the re-export

Two things go:

```rust
// lib.rs:32
pub use bevy_common_systems;
```

and the whole hand-written re-export block at `lib.rs:69-83`. Every name in it
now comes from a nova module and belongs in the `super::` block below:

`status_bar`, `status_bar_item`, `status_fps_color_fn`, `status_fps_value_fn`,
`status_version_color_fn`, `status_version_value_fn` -> `nova_ui` (prototype 02,
and `nova_core` should already read them from `nova_ui` directly)
`CameraShake`, `CameraShakeInput`, `CameraShakePlugin`, `ChaseCamera`,
`ChaseCameraInput`, `PostProcessingCamera`, `SkyboxConfig`,
`WASDCameraController` -> `camera::prelude` (03)
`PointRotation`, `PointRotationOutput`, `DirectionalSphereOrbitOutput` ->
`transform::prelude` (04)
`PDController`, `PDControllerInput`, `PDControllerPlugin`, `PDControllerSystems`,
`PDControllerTarget` -> `physics::prelude` (05)
`PlaySfx`, `SfxCommandsExt`, `SfxPlugin`, `SoundBank` -> `audio` (07)
`Cooldown` -> `cooldown` (08)
`GameObjectives`, `Objective`, `ObjectivesPlugin` -> `objectives` (08)
`StatusBarItemConfig`, `StatusBarRootConfig` -> `nova_ui` (02)

**Keep the comment at `lib.rs:69-76`.** It records why this block is an
explicit list rather than a glob: a glob used to drag in BCS's retired harness
twins (`AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
`ScreenshotReelPlugin`, `HarnessCompletion`) and boot every example inert
(task `20260802-183403`). The hazard is gone once BCS is gone - but the lesson
("adding a name to the prelude is a decision") is not, and the `super::` block
below is already an explicit list. Rewrite the comment to carry the lesson
forward; do **not** opportunistically switch anything to a glob.

Also update the crate docstring at `lib.rs:10`, which says
"`bevy_common_systems` supplies the engine-level layers around them".

## 10c. `plugin.rs`

`plugin.rs:20` (`use crate::{bevy_common_systems, prelude::*};`) becomes
`use crate::prelude::*;`. Lines 6 and 49 are doc/comment references. By now
lines 81-105 should all be local paths already - if any BCS path survives here,
a prototype was left incomplete.

## 10d. `nova_core`

`crates/nova_core/src/lib.rs:231` and `:233` are the log-filter strings:

```
"...,bevy_common_systems=trace,nova_assets=trace,..."
"...,bevy_common_systems=debug,nova_assets=debug,..."
```

Delete the `bevy_common_systems=` terms. Consider adding `nova_ui=`,
`nova_events_macros=` if the crate list is meant to be exhaustive - check the
existing list against the workspace members before changing more than the
deletion.

## 10e. Remaining prose

Comments and docs that name BCS but do not import it. Reword, do not delete
the surrounding explanation:

| File | Line |
|---|---|
| `nova_assets/src/persist.rs` | 16 (modelled-on note - this is a **historical citation**, and BCS's `persist` was never copied; keep the citation, or point it at the tasks record) |
| `nova_autopilot/src/lib.rs` | 3 ("no `bevy_common_systems`") - now trivially true, reword or drop |
| `nova_gameplay/src/gravity.rs` | 32 ("bevy_common_systems promotion candidate") |
| `nova_gameplay/src/juice.rs` | 11 |
| `nova_gameplay/src/sections/base_section.rs` | 324 |
| `nova_gameplay/src/objective_marker.rs` | 19 (see prototype 08) |
| `nova_scenario/src/actions/mission.rs` | 13 |
| `nova_scenario/src/actions/view.rs` | 146 |
| `nova_scenario/src/loader/lifecycle.rs` | 701 |
| `nova_scenario/src/objects/asteroid.rs` | 350 |
| `nova_scenario/tests/skybox_swap_e2e.rs` | 7 |
| `nova_probe/src/recorder.rs` | 26 |

Several of these say "the generic bevy_common_systems X" as a way of saying
"this is the reusable half". The owner's plan is to rebuild a
`bevy-common-systems` **out of** nova once the game is done, so that framing is
still worth keeping - just stop naming a crate the workspace no longer has.

## 10f. Manifests

Delete the `bevy_common_systems` line from all five, and fix the feature
forwards:

| Crate | Line | Feature forward |
|---|---|---|
| `nova_gameplay` | 22 | `:46` `debug = ["bevy/track_location", "bevy_common_systems/debug"]` |
| `nova_scenario` | 15 | `:42` same shape |
| `nova_events` | 12 | `:18` same shape (deleted back in prototype 01) |
| `nova_assets` | 72 | dev-dep, no feature (deleted in prototype 01) |
| `nova_debug` | 19 | `features = ["debug"]`, no forward |

Each `debug` feature reduces to `["bevy/track_location"]`. Do not rewire them
to `nova_debug/debug` - `nova_debug` has no features and is itself the gate
(see prototype 09). If a reduced feature has no remaining caller, delete it and
its forwards rather than leaving a no-op knob.

Then check for deps that only BCS pulled in and that nova never adopted:
`dirs` and `web-sys` were BCS's `persist` (never copied) - but **both are
already direct nova deps for other reasons** (`nova_debug/Cargo.toml:12`,
`nova_assets/Cargo.toml:43,46`). Do not remove them. `serde_json` moved to
`nova_events` (prototype 01) and is already a direct dep of `nova_assets` and a
dev-dep of `nova_scenario`. Net removals from the workspace: none beyond BCS
itself and `bevy_common_systems_macros`.

## 10g. Licenses

`cargo-about`: BCS is `publish = false` / first-party, so it was already
excluded from the third-party manifest. `nova_events_macros` must be
`publish = false` for the same reason (prototype 01). Removing BCS should be a
no-op, but re-run the license check and confirm the manifest is unchanged.

`syn` / `quote` / `proc-macro2` are already in the tree (BCS's macro crate
pulled them); the edge just moves to `nova_events_macros`. `bevy-inspector-egui`
likewise moves from BCS to `nova_debug`. Diff `Cargo.lock` and confirm the only
removals are `bevy_common_systems` and `bevy_common_systems_macros`.

## Verification - the real gate

```
grep -rn bevy_common_systems . --include='*.rs' --include='*.toml'   # must be empty
grep -n bevy_common_systems Cargo.lock                              # must be empty

nix develop --command cargo check --workspace --all-targets
nix develop --command cargo check --workspace --all-targets --features debug
nix develop --command cargo clippy --workspace --all-targets --features debug
nix develop --command cargo fmt --check
```

Crate-graph check: `cargo tree -p nova_gameplay` (etc.) before and after the
whole task. Exactly two new edges are permitted, both intended:
`nova_events -> nova_events_macros` and `nova_probe -> nova_events`. Anything
else means a module landed in the wrong crate.

**Run** the full example catalog under Xvfb `:99`, not just check it. This is
the step where a plugin registered twice or a component inserted twice finally
shows, and `cargo check` cannot see either. Then run the `probe` skill across
`sections/` (5), `systems/`, `stress/` and `ui/` and compare against the
baseline in `tasks/20260805-185103/` - that record has the last full pass
(sections 5, systems 3, stress 4, ui 5, all OK) and is the honest before.

## Done when

- Both greps come back empty.
- Workspace check + debug check + clippy + fmt all clean.
- Crate graph gained exactly the two permitted edges.
- Every example RUNS.
- Probe verdicts match the `20260805-185103` baseline.
- `Cargo.lock` lost exactly two packages.
