# Prototype 04 - the transform rigs -> `nova_gameplay`

Small and mechanical. Depends on prototype 03 only for `crate::math`
(`spherical_to_cartesian`).

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/transform/point_rotation.rs` | 228 | `crates/nova_gameplay/src/transform/point_rotation.rs` |
| `src/transform/smooth_look_rotation.rs` | 140 | `.../transform/smooth_look_rotation.rs` |
| `src/transform/sphere_orbit.rs` | 130 | `.../transform/sphere_orbit.rs` |
| `src/transform/directional_sphere_orbit.rs` | 135 | `.../transform/directional_sphere_orbit.rs` |
| `src/transform/random_sphere_orbit.rs` | 176 | `.../transform/random_sphere_orbit.rs` |
| `src/transform/mod.rs` | 14 | `.../transform/mod.rs` (rewrite the doc) |

823 L of code (matches NOTES.md) plus the 14 L `mod.rs`.

Keep the directory shape - five one-rig files under `transform/` is already how
nova's own subsystems are laid out (`integrity/`, `sections/`, `hud/`).

## Exports that must survive

`PointRotation`, `PointRotationOutput`, `PointRotationPlugin`,
`SmoothLookRotation`, `SmoothLookRotationPlugin`, `SphereOrbit`,
`SphereOrbitPlugin`, `DirectionalSphereOrbit`, `DirectionalSphereOrbitOutput`,
`DirectionalSphereOrbitPlugin`, `RandomSphereOrbit` + `SphereRandomOrbitPlugin`.

`nova_gameplay/src/lib.rs:77` re-exports `PointRotation`,
`PointRotationOutput`, `DirectionalSphereOrbitOutput` by name - examples glob
that prelude, so those three must keep resolving.

## Module wiring

`crates/nova_gameplay/src/lib.rs`: `pub mod transform;` and
`transform::prelude::*` in the `super::` block of the crate prelude.

`transform/mod.rs`'s BCS doc is one line; write a nova docstring saying what
these rigs are for (they drive `Transform` as *outputs*, the same idiom as
`tween`) and why nova owns them.

Note the naming inconsistency you are inheriting: the plugin for
`random_sphere_orbit` is `SphereRandomOrbitPlugin`, not
`RandomSphereOrbitPlugin`. **Copy it as-is.** Renaming is a follow-up.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/plugin.rs` | 89 | `PointRotationPlugin` |
| `nova_gameplay/src/plugin.rs` | 91 | `SphereRandomOrbitPlugin` |
| `nova_gameplay/src/plugin.rs` | 93 | `SmoothLookRotationPlugin` |
| `nova_gameplay/src/plugin.rs` | 95, 96 | `SphereOrbitPlugin`, `DirectionalSphereOrbitPlugin` |
| `nova_gameplay/src/sections/turret_section/aim.rs` | 6, 488 | glob; test adds `SmoothLookRotationPlugin` |
| `nova_gameplay/src/camera_controller/*` | - | already handled by prototype 03's glob rewrite |

The `plugin.rs:91` comment says the random orbit is "for debug to have a random
orbiting object". Keep it - it is the only record of why a dead-ish plugin is
registered.

## Compile hazards

- `random_sphere_orbit.rs:9` `use crate::meth::prelude::*;` ->
  `use crate::math::spherical_to_cartesian;` (from prototype 03).
- `random_sphere_orbit.rs:7` `use rand::prelude::*;` **compiles unchanged** on
  rand 0.10 - the 0.10 prelude re-exports `RngExt` (`rand-0.10.2/src/prelude.rs`),
  so `rng.random_range(..)` still resolves. `rand::rng()` at line 114 is also
  unchanged. This file is the one rand user that needs no edit; verify rather
  than assume.
- Zero `bevy_common_systems` strings in any of the five files - no rustdoc to
  rewrite.
- No new deps.
- `#![warn(missing_docs)]`: `transform/mod.rs`'s `pub mod prelude` has a doc,
  the five files' preludes may not.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib transform
nix develop --command cargo test -p nova_gameplay --lib turret_section::aim
nix develop --command cargo fmt --check
```

Five plugin registrations move out of `plugin.rs:89-96`, so **run** the
examples under Xvfb `:99`. The turret example is the sharpest test
(`SmoothLookRotation` drives turret facing); `sections/turret_section.rs`.

## Done when

- `nova_gameplay/src/transform/` holds the five rigs.
- All five plugins registered exactly once, from `plugin.rs`.
- `PointRotation`, `PointRotationOutput`, `DirectionalSphereOrbitOutput` still
  resolve from `nova_gameplay::prelude`.
- No new deps, no new graph edge.
