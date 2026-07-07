# Bevy 0.17 -> 0.19 migration

Notes from migrating the workspace off Bevy 0.17 to 0.19 (the version bump was done
in the `Cargo.toml` files; this documents the source changes needed to compile).

## Toolchain

Bevy 0.19 requires **rustc 1.95+**. The Nix flake (`nightly.latest`) already resolves
to a new enough nightly (1.98 at time of writing). If `cargo build` fails with
`rustc X is not supported by ... bevy@0.19.0 requires rustc 1.95.0`, the shell is
using a stale toolchain: re-enter with `nix develop` (or run
`nix develop --command cargo build ...`) so the current flake toolchain is on `PATH`.

## API changes applied

### Scenes: `Handle<Scene>` -> `Handle<WorldAsset>`, `SceneRoot` -> `WorldAssetRoot`

Bevy 0.19 replaced the old scene types with the world-serialization API. glTF scenes
now load as `Handle<WorldAsset>` and are spawned with the `WorldAssetRoot` component
(both are in `bevy::prelude`). The `#Scene0` asset-path label is unchanged.

- All `render_mesh: Option<Handle<Scene>>` fields (and the `*RenderMesh` wrapper
  components) in `crates/nova_gameplay/src/sections/*.rs` -> `Handle<WorldAsset>`.
- The `#[asset(path = "...glb#Scene0")] pub ...: Handle<Scene>` fields in
  `crates/nova_assets/src/lib.rs` -> `Handle<WorldAsset>`.
- Every `SceneRoot(handle)` spawn -> `WorldAssetRoot(handle)`.

### `BorderRadius` is no longer a component

`BorderRadius` is now a **field of `Node`** (`node.border_radius`), not a standalone
component. Any spawn tuple that listed `BorderRadius::MAX` / `BorderRadius::all(..)`
as its own element fails with "`(...)` is not a `Bundle`". Move it into the `Node`
literal:

```rust
// before
(Node { height: px(6), ..default() }, BackgroundColor(c), BorderRadius::all(px(3)))
// after
(Node { height: px(6), border_radius: BorderRadius::all(px(3)), ..default() }, BackgroundColor(c))
```

Fixed in `crates/nova_editor/src/lib.rs` (`button`) and the slider widgets in
`examples/02_thruster_shader.rs` and `examples/04_asteroids.rs`.

Note: this error blames the *whole tuple*, not the offending element. Bisecting the
tuple (drop members until it compiles) is the fastest way to find which type stopped
being a `Component`.

### `TextFont.font_size` is now `FontSize`

`font_size` takes a `FontSize` enum instead of `f32`: `font_size: 24.0` ->
`font_size: FontSize::Px(24.0)` (`FontSize` is in the prelude). Also `Px`/`Vw`/`Vh`
variants exist.

### `TextLayout::new_with_justify` -> `TextLayout::justify`

Constructor renamed. `TextLayout::new_with_justify(Justify::Center)` ->
`TextLayout::justify(Justify::Center)`.

### Resources now also derive `Component`

In 0.19 `#[derive(Resource)]` emits a `Component` impl too, so deriving both
`Resource` and `Component` on the same type now conflicts (E0119). Drop the explicit
`Component` from the derive list; the type is still usable as a component.

Consequence for generic code: a `fn f<T: Resource + Component>(x: ResMut<T>)` needs
`T`'s mutability pinned, because `ResMut` requires `Resource<Mutability = Mutable>`.
Add the bound: `T: Resource + Component<Mutability = bevy::ecs::component::Mutable>`
(see `button_on_setting` in `nova_editor/src/lib.rs`).

### `rand` 0.10 trait reshuffle

`rand` 0.10 moved methods between traits:

- `random_range(..)` on an rng now comes from `rand::RngExt` (was `rand::Rng`).
- `next_u32()` now comes from `rand::Rng` (was `rand::RngCore`, which is no longer at
  the crate root).

Adjust the `use rand::...;` import in the affected file to bring the right trait into
scope (`explode.rs`, `objects/asteroid.rs`).

### Misc

- `materials.get_mut(..)` returns a change-detection guard that must be bound `mut`
  before mutating fields through it (`let Some(mut material) = ...`). Hit in the
  thruster/velocity shader update systems and example 02.

## Runtime-only regressions (compiled + clippy/fmt clean, only surfaced by running)

These did not produce compiler errors; they only appeared when the game was launched.
They are the reason "builds green" is not the same as "works" for an engine migration.

- **Double-added plugin group.** `UiWidgetsPlugins` is part of `DefaultPlugins` on 0.19
  (it was experimental/manual in 0.17). `AppBuilder` added it a second time, panicking at
  startup ("plugin was already added"). Fix: do not re-add it (`nova_core/src/lib.rs`).
- **Resource used as a per-entity component (B0002 + resource clobbering).** Because
  `#[derive(Resource)]` is now component-backed, a type used *both* as a resource and as
  a per-entity component breaks two ways: a system taking `ResMut<T>` plus a `Query<&T>`
  panics with B0002, and spawning entities carrying `T` warns "Tried inserting the
  resource ... while one already exists" and clobbers the resource. The editor's
  `SectionChoice` (a selection resource *and* a per-button tag) hit both. Fix: split it -
  keep the resource, and give buttons a distinct `ButtonValue<T>` component
  (`nova_editor/src/lib.rs`). Prefer this over the `Without<IsResource>` band-aid.
- **A second window-targeting camera blacks out the 3D camera.** On 0.19, adding a second
  camera that renders to the same window as a 3D camera using HDR + post-processing
  (bloom/tonemapping) makes the 3D camera render nothing (fully black - no skybox, no
  meshes), while the HUD/UI still draws. The torpedo crosshair used a separate `Camera2d`
  and blacked out every scenario. Fix: render such overlays as **UI**, not a second
  camera (`hud/torpedo_target.rs`). For a HUD reticle, an absolutely-positioned
  `ImageNode` moved via `Camera::world_to_viewport` is the right pattern.

## Verification

```sh
nix develop --command cargo build --features dev --all-targets
```

builds the whole workspace (lib, binary, and all examples) cleanly. The only
remaining warning is a future-incompat note from the transitive `proc-macro-error2`
dependency, which is not our code.
