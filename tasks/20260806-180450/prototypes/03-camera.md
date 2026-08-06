# Prototype 03 - the camera rigs (+ `meth`) -> `nova_gameplay`

The largest single step and the one most likely to change behavior if rushed.
It also carries `meth`, which NOTES.md wrongly told you to drop.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/camera/chase.rs` | 241 | `crates/nova_gameplay/src/camera_controller/chase.rs` |
| `src/camera/shake.rs` | 578 | `.../camera_controller/shake.rs` |
| `src/camera/skybox.rs` | 138 | `.../camera_controller/skybox.rs` |
| `src/camera/post.rs` | 73 | `.../camera_controller/post.rs` |
| `src/camera/wasd.rs` | 219 | `.../camera_controller/wasd.rs` |
| `src/helpers/wasd.rs` | 231 | `.../camera_controller/wasd_controller.rs` |
| `src/meth/{lerp,sphere}.rs` + `mod.rs` | 143 | `crates/nova_gameplay/src/math.rs` |

1249 L of camera (matches NOTES.md) + 231 L of controller + 143 L of math.

**Two files are named `wasd.rs` upstream** - `camera/wasd.rs` is the free-fly
camera (`WASDCamera`, `WASDCameraInput`, `WASDCameraPlugin`,
`WASDCameraSystems`) and `helpers/wasd.rs` is the input controller over it
(`WASDCameraController`, `WASDCameraControllerPlugin`). They cannot share a
basename. Split as above.

## `meth` is not optional

NOTES.md lists `src/meth/` under "copy nothing - verified zero references".
That check was run against **nova's** code and is correct there. It is wrong
about the **copied** code. Three copied files import it:

| Copied file | Line | Needs |
|---|---|---|
| `src/camera/chase.rs` | 52 | `crate::prelude::LerpSnap` |
| `src/transform/random_sphere_orbit.rs` | 9, 105, 173 | `spherical_to_cartesian` |
| `src/mesh/builder.rs` | 30, 326-328 | `slerp` |

So `meth` must land before or with the camera. Put it at
`crates/nova_gameplay/src/math.rs` - one flat file, 143 L, holding `LerpSnap`
(f32 + Vec3 impls), `spherical_to_cartesian`, `direction_to_spherical`,
`slerp`. Do not name it `meth` in nova; that spelling was a BCS in-joke, and
the module is only reachable inside `nova_gameplay`.

Keep the `LerpSnap::lerp_and_snap` NOTE comment about `powi(7)` and
frame-rate independence - it guards a value.

`direction_to_spherical` arrives unused. Leave it (dead-code sweep is a
follow-up); it is `pub` in a `pub mod`, so no `dead_code` warning fires.

`meth`'s `mod.rs` doc is 70 lines of "difficulty ramp" recipes for BCS's
example games. Those recipes are about a crate nova is deleting - drop them
and write a two-line nova docstring instead. This is the one place where
dropping copied prose is right.

## Destination layout

Land the six camera files inside the existing
`crates/nova_gameplay/src/camera_controller/`, beside nova's `authority.rs`,
`framing.rs`, `handback.rs`, `mode.rs`, `rig.rs`.

Then, **in the same commit**, `git mv camera_controller camera` and let the
compiler chase the path. The dir is named `camera_controller` because it used
to hold only nova's controller *over* BCS's cameras; once the rigs live here
too, `camera/` is the honest name. This is a rename, not a redesign - it is the
kind of layout change the owner explicitly allowed.

Names that change with the rename:
- `crate::camera_controller::SpaceshipCameraControllerPlugin` at
  `plugin.rs:113`
- `camera_controller::prelude::*` in `nova_gameplay/src/lib.rs:97`
- the `mod.rs:44` prelude doc line

Do **not** rename `SpaceshipCameraControllerPlugin` itself, or any of the
`Spaceship*` marker types. Only the module path moves.

## Wrapper collapse - what is and is not in scope

`camera_controller/framing.rs`, `handback.rs`, `mode.rs`, `rig.rs` all do
`use bevy_common_systems::prelude::*;`. After the copy those become
`use super::{chase::*, shake::*, ...}` or `use crate::prelude::*`. **That import
rewrite is the whole of the "collapse".** Do not merge nova's `framing.rs` into
BCS's `chase.rs`, do not delete a nova system because BCS has a similar one, do
not change what writes `Transform`. If a merge looks tempting, it is a
follow-up task.

`authority.rs:20` is the one site that matters:

```rust
use bevy_common_systems::prelude::{CameraShakeSystems, ChaseCameraSystems, WASDCameraSystems};
```

Those three `SystemSet`s are the ordering contract described at
`camera_controller/mod.rs:108-114` ("Every camera-Transform writer in the app -
bcs's three and nova's scripted pose - is ordered by this one chain"). After
the move they are crate-local. **The ordering must come out byte-identical** -
this branch already has a fix for it (`cd1bff21`, "run shake before the
scripted pose") and its record is `tasks/20260805-185103/`. Re-read that record
before touching `authority.rs`. Also update the mod.rs comment: "bcs's three"
becomes nova's three.

`authority.rs:86` has a test-local import of the same shape.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/plugin.rs` | 81, 82 | `WASDCameraPlugin`, `WASDCameraControllerPlugin` |
| `nova_gameplay/src/plugin.rs` | 84 | `ChaseCameraPlugin` |
| `nova_gameplay/src/plugin.rs` | 86, 87 | `SkyboxPlugin`, `PostProcessingDefaultPlugin` |
| `nova_gameplay/src/camera_controller/{framing,handback,mode,rig}.rs` | 8, 7, 6, 6 | glob imports |
| `nova_gameplay/src/camera_controller/authority.rs` | 20, 86 | the three `*Systems` sets |
| `nova_gameplay/src/hud/screen_indicator.rs` | 22, 1365 | `ChaseCameraSystems`; test uses `ChaseCamera`, `ChaseCameraInput`, `ChaseCameraPlugin` |
| `nova_gameplay/src/hud/velocity.rs` | 16 | glob |
| `nova_gameplay/src/input/player/{flight_rig,hints,intent,test_support}.rs` | 6, 5, 6, 5 | globs |
| `nova_gameplay/src/juice.rs` | 11 | doc comment naming the BCS trauma model |
| `nova_gameplay/src/sections/{controller_section,thruster_section}.rs` | 5, 10 | globs |
| `nova_gameplay/src/sections/turret_section/{aim,setup}.rs` | 6, 7 | globs |
| `nova_debug/src/harness.rs` | 78 | `WASDCameraController` |
| `nova_scenario/src/actions/view.rs` | 10, 146, 531 | glob; doc about skybox image loading; test uses `WASDCameraController` |
| `nova_scenario/tests/skybox_swap_e2e.rs` | 7, 29 | `SkyboxPlugin` and friends - the doc at :7 calls it "the LAST bridge" |

`nova_gameplay/src/lib.rs:77` re-exports `CameraShake`, `CameraShakeInput`,
`CameraShakePlugin`, `ChaseCamera`, `ChaseCameraInput`, `PostProcessingCamera`,
`SkyboxConfig`, `WASDCameraController`. Those names must keep resolving from
`nova_gameplay::prelude` - examples glob it. Move them from the BCS block to
the `super::` block in the same edit.

## Compile hazards

- `shake.rs:64` imports `crate::camera::chase::ChaseCameraSystems` - after the
  move, `super::chase::ChaseCameraSystems`.
- `shake.rs:62` `use rand::Rng;` -> **`use rand::RngExt;`** (rand 0.10 renamed
  the method trait; see prototype 06 for the full rand story). `rand::rng()`
  and `random_range` are unchanged.
- `chase.rs:52` `use crate::prelude::LerpSnap;` -> `use crate::math::LerpSnap;`
- `helpers/wasd.rs:16` `use crate::prelude::{WASDCamera, WASDCameraInput};` ->
  `use super::wasd::{WASDCamera, WASDCameraInput};`
- `helpers/wasd.rs` needs `bevy_enhanced_input` - `nova_gameplay` already has
  it (`Cargo.toml:18`). No new dep.
- `nova_gameplay` already has `rand = "0.10.2"` (`Cargo.toml:19`). No manifest
  change for this step.
- 4 `bevy_common_systems` doc strings across `chase.rs`, `shake.rs`,
  `skybox.rs`, `post.rs`, `wasd.rs`, `helpers/wasd.rs` (one each) - all
  doctest `use` lines.
- `#![warn(missing_docs)]` on `nova_gameplay`.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib camera
nix develop --command cargo test -p nova_gameplay --lib juice
nix develop --command cargo test -p nova_scenario --lib actions::view
nix develop --command cargo fmt --check
```

This step moves five plugin registrations out of `plugin.rs:81-87`, so
**examples must be RUN under Xvfb `:99`**, not checked. Run at least one
`sections/` example (chase camera + shake), `examples/screenshots/` (skybox +
post-processing + `pose_camera`), and `nova_scenario`'s
`tests/skybox_swap_e2e.rs`.

Run the `probe` skill before/after on a `sections/` example: camera framing is
exactly the kind of change a check cannot judge.

## Done when

- `nova_gameplay/src/camera/` holds nova's controller and the six rig files.
- `crate::math` exists and `chase.rs` / `random_sphere_orbit.rs` /
  `mesh/builder.rs` resolve through it.
- `authority.rs`'s three-way ordering chain is unchanged in effect; the
  `cd1bff21` fix still holds.
- Every camera plugin is registered exactly once, from `plugin.rs`.
- Examples RUN clean, not just check clean.
