# Prototype 05 - PD controller + point velocity -> `nova_gameplay`

Small, and the one step where a partial copy is the right answer.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/physics/pd_controller.rs` | 575 | `crates/nova_gameplay/src/physics/pd_controller.rs` |
| `src/physics/rigid_body.rs` | 101 | `crates/nova_gameplay/src/physics/rigid_body.rs` (partial) |

676 L, matching NOTES.md. Add a `physics/mod.rs`.

## `rigid_body.rs` is a partial copy

The file has exactly two exports:

- `rigid_body_point_velocity` - **copy this**, with its doctest. It is the
  standard `v_point = v_linear + omega x (p - com)` relation, used by the
  torpedo bay and the turret to give a shot its muzzle velocity.
- `destructible_body` - **do not copy**. Nova already owns it; commit
  `5f67c75a` moved it into `crates/nova_gameplay/src/integrity/health.rs`
  alongside nova's `Health`.

Dropping `destructible_body` also drops `use crate::health::prelude::*;`
(`rigid_body.rs:6`), which is the BCS `Health` nova has replaced. **That import
must not come across** - it would reintroduce the exact type-shadowing bug that
`261c7e71` just fixed in `nova_probe`.

`rigid_body.rs:48-50` has rustdoc intra-doc links into
`crate::integrity::components::{ConnectedTo, IntegrityRoot}` - those belong to
`destructible_body` and go with it.

Do not copy `src/physics/doom_controller.rs` (arena-shooter FPS controller,
zero references). Do not copy the `physics/mod.rs` doc header - it is 60+ lines
of radial-gravity recipe written for BCS's `examples/08_dropzone.rs`. Nova's
radial gravity lives in `crates/nova_gameplay/src/gravity.rs`, whose own doc at
line 32 already calls itself a "bevy_common_systems promotion candidate" -
update that line, and consider whether the recipe's content is worth folding
into `gravity.rs`'s docstring. If it is not obviously worth it, drop it; do not
carry a doc about a crate that is being deleted.

## Exports that must survive

`PDController`, `PDControllerInput`, `PDControllerPlugin`, `PDControllerSystems`,
`PDControllerTarget`, `rigid_body_point_velocity`.

All six except `rigid_body_point_velocity` are re-exported by name from
`nova_gameplay/src/lib.rs:77`, so they must keep resolving from
`nova_gameplay::prelude`.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/plugin.rs` | 102 | `PDControllerPlugin` |
| `nova_gameplay/src/sections/torpedo_section/mod.rs` | 16 | `rigid_body_point_velocity`, `TempEntity` |
| `nova_gameplay/src/sections/turret_section/firing.rs` | 8 | `rigid_body_point_velocity`, `TempEntity` |
| `nova_gameplay/src/input/player/flight_rig.rs` | 6 | glob (PD controller input) |
| `nova_gameplay/src/sections/controller_section.rs` | 5 | glob |

The two `rigid_body_point_velocity` sites also import `TempEntity`, which does
not land until prototype 08. Either sequence 08 before 05, or leave those two
`use` lines split across the two paths for one commit. Splitting is fine and
keeps each step small.

`nova_gameplay/src/sections/base_section.rs:324` has a comment about BCS's
`destructible_body` - reword it now that the function is nova's, or delete it.

## Compile hazards

- `pd_controller.rs` needs `avian3d` - `nova_gameplay` already has it
  (`Cargo.toml:10`). No manifest change.
- `pd_controller.rs` has zero `bevy_common_systems` strings.
- `rigid_body.rs` has one - the doctest `use bevy_common_systems::prelude::*;`
  above `rigid_body_point_velocity`. Rewrite to `use nova_gameplay::prelude::*;`
  and confirm the doctest still passes (it is a pure-math assert, so it will).
- `physics/mod.rs` is new: write a short nova docstring, declare the two
  submodules, and a `prelude` re-exporting the six names.
- `#![warn(missing_docs)]`.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib physics
nix develop --command cargo test -p nova_gameplay --lib torpedo_section
nix develop --command cargo test -p nova_gameplay --lib turret_section
nix develop --command cargo fmt --check
```

`PDControllerPlugin` moves out of `plugin.rs:102`, so **run** the examples
under Xvfb `:99`. The PD controller is the ship's attitude authority - a
`sections/controller_section.rs` run plus a `probe` before/after is the
right check. Any drift in torque response shows up in the probe report and
nowhere else.

## Done when

- `nova_gameplay/src/physics/` holds the PD controller and
  `rigid_body_point_velocity` only.
- No copy of `destructible_body`, no `crate::health` import, no
  `doom_controller`.
- `PDControllerPlugin` registered exactly once.
- Probe comparison shows no attitude-control drift.
