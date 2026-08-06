# Handoff: absorb `bevy_common_systems` into nova, delete the dependency

## Goal

Copy the still-used `bevy_common_systems` (BCS) modules **verbatim** into nova
crates at the locations below, then do compiler-assisted refactoring until the
workspace builds. End state: no `bevy_common_systems` entry in any `Cargo.toml`,
none in `Cargo.lock`.

This is a mechanical lift-and-shift. Do not redesign the copied code. Rename
only where the target module path demands it, and only where the compiler or
the destination crate's existing naming forces the change.

## Where BCS lives

Source of truth for every copy:

```
/home/alex/personal/bevy-common-systems/
```

Use the **working copy**, not the cargo checkout. It sits at `e5da687`
(version 0.19.6), which is ~50 commits ahead of the rev nova has locked
(`30d1befa`, tag `v0.19.5`). The owner wants the newer source; the lock is
irrelevant because the dependency is being deleted.

Every path, line count and export in this document was verified against
`e5da687`, not against the locked rev. Two structural changes matter:

- `src/mesh/slice.rs` (133 L) is **new** and is a private `mod slice;`
  (`src/mesh/mod.rs:9`) that `builder.rs:29` imports (`triangle_slice`,
  `TriangleSliceResult`). It has no prelude block, so it is easy to miss.
  Copy it with `builder.rs` or the mesh module will not compile.
- `src/integrity/` was restructured upstream (`damage.rs` extracted from
  `plugin.rs`). **Ignore it.** Nova already migrated integrity from the older
  shape in commit `5f67c75a`; do not re-import or reconcile.

Also skip `src/debug/harness/` -- nova has `nova_autopilot` for that already.

## Prior art

Two chunks already came across. Follow their shape.

| Chunk | Landed in | Task |
|---|---|---|
| The autopilot/screenshot harness | `crates/nova_autopilot` | `20260802-183403`, `20260802-183406` |
| Health + the destruction pipeline | `crates/nova_gameplay/src/integrity/` | `20260805-185103` (commit `5f67c75a`) |

Read `crates/nova_gameplay/src/integrity/{mod,health,core,components}.rs`
first. It is the reference for what "absorbed" looks like here: BCS's
`health` + `integrity/{blast,components,plugin}` became nova modules with nova
docstrings explaining *why nova owns them*, the glue layer that used to wrap
BCS collapsed into them, and `destructible_body` moved with them.

## Current BCS dependency

Five crates declare it:

| Crate | Line |
|---|---|
| `nova_gameplay` | `Cargo.toml:22`, feature fwd at `:46` |
| `nova_scenario` | `Cargo.toml:15`, feature fwd at `:42` |
| `nova_events` | `Cargo.toml:12`, feature fwd at `:18` |
| `nova_assets` | `Cargo.toml:72` (tests only) |
| `nova_debug` | `Cargo.toml:19`, `features = ["debug"]` |

`nova_gameplay/src/lib.rs:32` does `pub use bevy_common_systems;` and its
prelude re-exports ~30 BCS names by hand (`lib.rs:77`). `nova_probe` reaches
BCS only through that re-export. Both go away.

Note the comment at `nova_gameplay/src/lib.rs:69-76`: the prelude list is
explicit *on purpose* because a glob used to drag in the retired BCS harness
twins and boot every example inert. Keep that lesson -- after the migration the
same names come from nova modules, so the hazard is gone, but do not
opportunistically switch that block back to a glob.

## The move map

Every destination crate **already depends on** its source's consumers. This
migration adds **zero new edges** to the workspace graph. Verify that stays true.

### 1. `nova_events` <- the event engine

`nova_events` is a leaf (`bevy` + `serde` only) and is already nova's event
vocabulary. `nova_gameplay`, `nova_scenario` and `nova_assets` all already
depend on it.

| Copy from | To | Exports that must survive |
|---|---|---|
| `src/modding/events.rs` (560 L) | `crates/nova_events/src/engine.rs` (or `engine/`) | `GameEvent`, `GameEventInfo`, `GameEventQueue`, `EventHandler`, `EventAction`, `EventFilter`, `EventWorld`, `GameEventsPlugin`, `CommandsGameEventExt` |
| `bevy_common_systems_macros/src/lib.rs` (44 L) | new workspace crate `crates/nova_events_macros` | the `EventKind` derive |

- `nova_events` is the derive's only user. Add the new crate to
  `Cargo.toml`'s `[workspace] members`.
- `nova_scenario/src/filters.rs` imports `bevy_common_systems::modding::prelude::*`
  and `nova_scenario/benches/scenario_dispatch.rs` imports
  `bevy_common_systems::modding::events::GameEventQueue` -- both need a nova
  path that exposes the same names.
- **Drop** `src/modding/registry.rs` (494 L). `EventHandlerRegistry`,
  `HandlerSpec`, `parse_specs`, `RegistryError`: zero references anywhere in
  `crates/` or `examples/`.

### 2. `nova_gameplay` <- the simulation

| Copy from | To | Notes |
|---|---|---|
| `src/camera/{chase,shake,skybox,post,wasd}.rs` (1249 L) | `crates/nova_gameplay/src/camera_controller/` | merge beside the existing `authority.rs`, `framing.rs`, `handback.rs`, `mode.rs`, `rig.rs`; consider renaming the dir to `camera/` once the wrappers collapse |
| `src/helpers/wasd.rs` (231 L) | same dir | `WASDCameraController` belongs with the camera code, not in a helpers bag |
| `src/transform/*.rs` (823 L) | `crates/nova_gameplay/src/transform/` | `PointRotation`, `SmoothLookRotation`, `SphereOrbit`, `DirectionalSphereOrbit`, `RandomSphereOrbit` |
| `src/physics/{pd_controller,rigid_body}.rs` (676 L) | `crates/nova_gameplay/src/physics/` | from `rigid_body.rs` only `rigid_body_point_velocity` is still needed -- `destructible_body` is already nova's, in `integrity/health.rs` |
| `src/mesh/{builder,explode,slice}.rs` (970 L) | `crates/nova_gameplay/src/mesh/` | `TriangleMeshBuilder`, `ExplodeMesh`, `ExplodeFragments`. **`slice.rs` is a private module with no prelude -- copy it or `builder.rs` will not compile** |
| `src/audio/{mod,registry}.rs` (347 L) | merge into `crates/nova_gameplay/src/audio/` | `SfxPlugin`, `PlaySfx`, `SfxCommandsExt`, `SfxMasterVolume`, `SoundBank`; lands beside `combat.rs`, `cues.rs`, `loops.rs`, `mixing.rs` |
| `src/helpers/{temp,despawn}.rs` (171 L) | `crates/nova_gameplay/src/lifetime.rs` | kill the "helpers" name -- this is `TempEntity` + `DespawnEntity`, a lifetime concern |
| `src/time/cooldown.rs` (177 L) | `crates/nova_gameplay/src/cooldown.rs` | `Cooldown` only; used by the torpedo bay and AI threat memory |
| `src/ui/objectives.rs` (171 L) | `crates/nova_gameplay/src/objectives.rs` | mission state, not a widget; merge with the existing `objective_marker.rs`. Drop this file's panel half (`ObjectivesPanelConfig`, `ObjectivesPanelMarker`, `objectives_panel`) -- unused, nova draws objectives in `hud/objective_stack.rs` |

### 3. `nova_ui` <- the chrome

Leaf toolkit (`bevy` only). `nova_gameplay` and `nova_core` already depend on it.

| Copy from | To | Notes |
|---|---|---|
| `src/ui/status.rs` (328 L) | `crates/nova_ui/src/status_bar.rs` | `nova_core/src/lib.rs` is the other caller |
| `src/tween/mod.rs` (419 L) | `crates/nova_ui/src/tween.rs` | only `nova_gameplay/src/hud/` uses it; it is a UI animation primitive |

### 4. `nova_debug` <- the inspector

Sole consumer, already feature-gated.

| Copy from | To | Notes |
|---|---|---|
| `src/debug/inspector.rs` (313 L) | `crates/nova_debug/src/inspector.rs` | `InspectorDebugPlugin`, `DebugEnabled` |
| `src/debug/wireframe.rs` (66 L) | `crates/nova_debug/src/wireframe.rs` | `WireframeDebugPlugin`, `DebugEnabled` |

`nova_debug/src/lib.rs:19` currently aliases the two `DebugEnabled` types as
`InspectorEnabled` / `WireframeEnabled`, and `harness.rs` reaches them by full
path. Keep both resources distinct; nova_debug already has its own third
`DebugEnabled` at `lib.rs:82`.

## Copy nothing from these -- verified zero references

Checked name-by-name against every `.rs` under `crates/` and `examples/`.

| BCS path | Note |
|---|---|
| `src/completion.rs` | nova's is `nova_autopilot::completion` |
| `src/debug/harness/` | nova's is `nova_autopilot` |
| `src/feedback/` | `Flash` hits are `nova_gameplay/src/juice.rs`'s own struct |
| `src/health/`, `src/integrity/` | already nova's, commit `5f67c75a` |
| `src/material.rs` | `glowing_material` |
| `src/persist/` | also drops the `dirs` + `web-sys` deps |
| `src/scoring/` | `HighScore`, `Streak` |
| `src/input/` | `release_cursor` hits are `nova_menu/src/pause.rs`'s own fn |
| `src/meth/` | **all** `slerp` call sites are Bevy's `Quat`/`Dir3` methods, not BCS's `Vec3` fn; `LerpSnap`, `spherical_to_cartesian`, `direction_to_spherical` unreferenced |
| `src/modding/registry.rs` | `EventHandlerRegistry`, `HandlerSpec`, `parse_specs`, `RegistryError` |
| `src/physics/doom_controller.rs` | |
| `src/camera/project.rs` | `world_to_screen`, `pointer_on_plane` |
| `src/helpers/pointer.rs` | `EnhancedInputPointerPlugin` |
| `src/ui/{animate,health_display,menu,popup,touchpad}.rs` | |

Roughly **4.9k LOC copied**; the rest of the crate is dropped.

Within the copied files there is more dead surface (`CameraShakeOutput`,
`WASDCamera`, `WASDCameraInput`, `EventHandlerIndex`, `BlastDamageConfig`,
`*Systems` sets nobody orders against, `RandomSphereOrbit`'s components -- only
its plugin is registered). Copy those verbatim in this pass; a dead-code sweep
is a separate follow-up, not this session's job.

## Dependency fallout

- **`rand`**: BCS pins `0.9.2`, the nova workspace is on `0.10.2`
  (`Cargo.toml:191`). Copied code that uses `rand` hits the 0.10 API change.
  Exactly four files touch `rand`/`noise`, all bound for `nova_gameplay`:
  `mesh/builder.rs`, `mesh/explode.rs`, `camera/shake.rs`,
  `transform/random_sphere_orbit.rs`. Note
  `nova_gameplay/src/integrity/explode.rs` already uses `bevy_rand` +
  `rand::RngExt`; prefer matching that.
- **`noise` 0.9**: new direct dep for `nova_gameplay` (`mesh/builder.rs`).
- **`bevy-inspector-egui` 0.37**: becomes a direct `nova_debug` dep.
  `nova_debug` already has `avian3d` with `diagnostic_ui`.
- **Dropped**: `dirs`, `web-sys`, and probably `serde_json` (persist + registry
  only). Confirm before removing from any manifest.
- **Feature plumbing**: `nova_gameplay`, `nova_scenario` and `nova_events` each
  forward their `debug` feature to `bevy_common_systems/debug`. Rewire to
  `nova_debug`'s own gating. `nova_events` may not need a `debug` feature at
  all afterwards -- check.
- **cargo-about**: BCS is `publish = false` / first-party, so it is already
  excluded from the third-party license manifest. Removing it should be a
  no-op there, but re-run the license check.

## Known straggler to fix on the way

`crates/nova_probe/src/invariants.rs:44` and `crates/nova_probe/src/capture.rs:25`
still import `bevy_common_systems::health::Health` via the `nova_gameplay`
re-export. Nova moved to its own `Health` in commit `5f67c75a`, so these two
now query a type **no entity carries**:

- `invariants.rs` "Health bounds" check silently passes on an empty query.
- `capture.rs` "keep every combatant alive" top-up silently does nothing.

Point both at `nova_gameplay::prelude::Health` and confirm the invariant
actually sees entities.

## Suggested order

Each step should leave the workspace compiling.

1. `nova_events_macros` + `nova_events/src/engine.rs`; drop the BCS dep from
   `nova_events`, then `nova_scenario`, then `nova_assets`.
2. `nova_ui`: `status_bar.rs` + `tween.rs`; repoint `nova_core` and
   `nova_gameplay/src/hud/`.
3. `nova_gameplay`: camera, then transform, then physics, then mesh, then the
   small ones (`lifetime`, `cooldown`, `audio`, `objectives`). Collapse the
   `camera_controller` wrappers as you go.
4. `nova_debug`: inspector + wireframe.
5. Delete `pub use bevy_common_systems;` and the hand-written prelude
   re-export block in `nova_gameplay/src/lib.rs`; fix `nova_probe`'s two
   `Health` imports and the `GameEvent`/`GameEventInfo` paths in
   `recorder.rs`.
6. Remove the dep from all five manifests. `grep -rn bevy_common_systems .`
   must come back empty (including `Cargo.lock`).

## Done when

- `grep -rn bevy_common_systems . --include='*.rs' --include='*.toml'` is empty,
  and `bevy_common_systems` no longer appears in `Cargo.lock`.
- `nix develop --command cargo check --workspace --all-targets` is clean.
- `nix develop --command cargo fmt --check` is clean.
- The workspace crate graph gained no new edges.
- `nova_probe`'s health invariant is checked against a non-empty query.

## Project conventions that apply

- `cargo` only runs via `nix develop --command cargo`.
- Do not run the full test suite -- it OOMs the box. Use `--lib` with a filter,
  and `cargo check --examples`. CI (`.github/workflows/ci.yaml`) runs the suite.
- Bevy examples must be **run** (Xvfb `:99`), not just checked: `cargo check`
  misses duplicate-component panics. Relevant here because plugin registration
  moves wholesale out of `nova_gameplay/src/plugin.rs:81-106`.
- Comment *why*, not *what*. Keep the guard comments in the copied code.
- Commits: user authorship only, no AI attribution or co-author trailers.
- Tasks live in `tasks/<id>/TASK.md`, driven by the `tatr` CLI. This work is
  large enough to want a parent epic with per-crate members rather than one
  task -- confirm with the owner before creating records.
