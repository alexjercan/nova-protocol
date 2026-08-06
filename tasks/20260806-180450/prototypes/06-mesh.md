# Prototype 06 - the mesh toolkit -> `nova_gameplay`

The one step with a real dependency change (`noise`) and the one with a hidden
private module.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/mesh/builder.rs` | 495 | `crates/nova_gameplay/src/mesh/builder.rs` |
| `src/mesh/explode.rs` | 342 | `.../mesh/explode.rs` |
| `src/mesh/slice.rs` | 133 | `.../mesh/slice.rs` (**private**) |
| `src/mesh/mod.rs` | 17 | `.../mesh/mod.rs` |

970 L, matching NOTES.md.

## `slice.rs` is the trap

`src/mesh/mod.rs:9` declares it as `mod slice;` - **private, no prelude block**.
`builder.rs:29` imports `triangle_slice` and `TriangleSliceResult` from it. It
does not appear in any prelude, any export list, or any grep for BCS names in
nova. If you copy `builder.rs` without it, the mesh module does not compile.

It is new since the rev nova has locked (`30d1befa` / `v0.19.5`), which is why
it is easy to miss. Keep it `mod slice;` (private) in nova too.

## Exports that must survive

`TriangleMeshBuilder` (from `builder.rs`), `ExplodeMesh`, `ExplodeFragments`,
`ExplodeMeshPlugin` (from `explode.rs`'s prelude).

`nova_scenario/src/objects/asteroid.rs:3` names `TriangleMeshBuilder` directly.
`nova_gameplay/src/lib.rs:77` does **not** re-export the mesh names - they are
reached through `nova_scenario`'s own glob and through
`plugin.rs:100`. Confirm before deleting anything from the prelude list.

## Dependency change: `noise`

`builder.rs:27` does `use noise::NoiseFn;`. This is a **new direct dep for
`nova_gameplay`**:

```diff
 # crates/nova_gameplay/Cargo.toml
+noise = { version = "0.9" }
```

`nova_scenario` already pins `noise = "0.9"` (`Cargo.toml:14`), so the version
is settled and cargo unifies it. No lock churn beyond the new edge.

## The `rand` story (corrects NOTES.md)

NOTES.md names four files touching `rand`. It is **three**, and `mesh/builder.rs`
is not one of them - `builder.rs` uses `noise` only.

The three, and exactly what rand 0.10.2 requires:

| File | Line | Change |
|---|---|---|
| `mesh/explode.rs` | 10 | `use rand::Rng;` -> `use rand::RngExt;` |
| `mesh/explode.rs` | 159 | `fn random_unit_vector(rng: &mut impl Rng)` -> `impl RngExt` |
| `camera/shake.rs` (prototype 03) | 62 | `use rand::Rng;` -> `use rand::RngExt;` |
| `transform/random_sphere_orbit.rs` (prototype 04) | 7 | `use rand::prelude::*;` - **no change**, the 0.10 prelude re-exports `RngExt` |

Why: rand 0.10.0 renamed the method trait `Rng` -> `RngExt`, because upstream
`rand_core` renamed `RngCore` -> `Rng` (rand CHANGELOG, PR #1717). Everything
else these files use is unchanged in 0.10 - `rand::rng()` still exists
(`rngs/thread.rs:201`), `random_range` still lives on `RngExt`
(`rng.rs:163`), and the free `rand::random_range` is still there
(`lib.rs:235`).

**Do not rewire these onto `bevy_rand`.** The owner's ruling: use the nova
workspace's rand *version* (0.10.2), nothing more. `integrity/explode.rs`
already uses `bevy_rand` + `rand::RngExt`, and matching that pattern would be a
determinism redesign inside a mechanical lift - a separate task if it is ever
wanted.

`nova_gameplay/Cargo.toml:19` already has `rand = "0.10.2"`.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/plugin.rs` | 100 | `ExplodeMeshPlugin` |
| `nova_scenario/src/objects/asteroid.rs` | 3 | `TriangleMeshBuilder` (alongside `CommandsGameEventExt`, from prototype 01) |
| `nova_gameplay/src/integrity/explode.rs` | 13 | already imports BCS names - check which survive here vs. which are already nova's |

`integrity/explode.rs` is nova's own destruction reaction from commit
`5f67c75a`; it imports from the BCS prelude at line 13. Read that import list
carefully: some of it is mesh (`ExplodeMesh`), some is health (already nova's).
Only the mesh half is this prototype's business.

`nova_scenario/src/objects/asteroid.rs:350` has a comment about
`destructible_body` being BCS's - that belongs to prototype 05's cleanup.

## Module wiring

`crates/nova_gameplay/src/lib.rs`: `pub mod mesh;` and `mesh::prelude::*` in
the crate prelude's `super::` block.

`mesh/mod.rs` keeps its shape: `pub mod builder; pub mod explode; mod slice;`
plus a prelude re-exporting `builder::TriangleMeshBuilder` and
`explode::prelude::*`. Rewrite the two-paragraph header as a nova docstring.

## Compile hazards

- `builder.rs:30` `use crate::meth::prelude::*;` -> `use crate::math::slerp;`
  (the `math` module lands in prototype 03). Used at `builder.rs:326-328`.
- `slice.rs` has zero external imports and zero `bevy_common_systems` strings.
- `explode.rs` has zero `bevy_common_systems` strings; `builder.rs` has one
  (a doctest `use`).
- `#![warn(missing_docs)]`.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib mesh
nix develop --command cargo test -p nova_scenario --lib objects::asteroid
nix develop --command cargo fmt --check
```

`ExplodeMeshPlugin` moves out of `plugin.rs:100`, so **run** under Xvfb `:99`.
Asteroids are the visible consumer of `TriangleMeshBuilder` (procedural
noise-displaced icosphere) and of `ExplodeMesh` (fragments on destruction) -
run a `systems/` example that spawns and destroys asteroids, and take a
before/after `probe` report. A silent `noise` or `slerp` regression shows up
as changed geometry, not as a compile error.

## Done when

- `nova_gameplay/src/mesh/` holds builder, explode and the private `slice`.
- `noise = "0.9"` is a `nova_gameplay` dep and resolves to the same version
  `nova_scenario` already pins.
- No `bevy_rand` rewiring; the three rand edits are exactly the two
  `RngExt` imports and the one generic bound.
- `ExplodeMeshPlugin` registered exactly once.
- Asteroid geometry and debris are visually unchanged (probe report).
