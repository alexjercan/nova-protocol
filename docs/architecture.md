# Architecture

Nova Protocol is a 3D space shooter built on **Bevy 0.19** with **avian3d** physics.
It is a Cargo workspace: the root `nova-protocol` crate is a thin shell and all the
real code lives under `crates/`.

## Crate map

| Crate           | Responsibility |
|-----------------|----------------|
| `nova-protocol` (root) | `src/main.rs` = clap CLI + entrypoint. `src/lib.rs` re-exports `nova_core`. Examples live in `examples/`. |
| `nova_core`     | Thin wiring: `AppBuilder` assembles every plugin (window/log/asset setup, status UI). No gameplay logic. Adds `NovaEditorPlugin` when no custom game plugins are supplied. |
| `nova_editor`   | The spaceship **editor scene** (`NovaEditorPlugin`): section-picker UI, ship building/placement, transition to the scenario simulation. Sits above `nova_gameplay` + `nova_scenario`. |
| `nova_gameplay` | Nova-specific gameplay. Submodules: `sections/`, `integrity/`, `input/` (player + ai), `hud/`, `camera_controller`, `plugin` (`NovaGameplayPlugin`). Also owns `GameStates` (top-level Loading/Playing lifecycle). |
| `nova_scenario` | Scenario/modding engine: `events`, `filters`, `actions`, `variables`, `world`, `loader`, and scenario `objects/`. See [scenario-system.md](scenario-system.md). |
| `nova_events`   | Game event kinds and entity identity components (shared vocabulary between gameplay and scenario). |
| `nova_assets`   | `bevy_asset_loader` setup. Loads glb/textures/shaders, then registers the built-in sections and scenarios. |
| `nova_debug`    | Debug-only plugin (inspector, wireframe, section overlays). Compiled only under the `debug` feature. |
| `nova_info`     | Exposes `APP_VERSION`, injected by `build.rs` via the `APP_VERSION` env var. |

Every crate exposes a `pub mod prelude`. **Import from the prelude**
(`use nova_gameplay::prelude::*`), not from inner modules. `nova_core::prelude`
re-exports all the sub-crate preludes, so top-level code and examples usually just do
`use nova_protocol::prelude::*`.

### External shared crate: `bevy-common-systems`

Generic, non-Nova Bevy helpers (WASD/chase cameras, skybox, post-processing, mesh
builders + explode, transform orbits, PD controller, health, status bar, the generic
game-event queue `GameEventsPlugin`/`EventWorld`) live in a **separate repo**,
`bevy-common-systems`, pinned as a git dependency (rev in
`crates/nova_gameplay/Cargo.toml`). It is re-exported through
`nova_gameplay::prelude`, so most call sites do not name it directly.

Historically this crate was vendored under `crates/bevy_common_systems/`. That copy
is being removed on the `feature/cleanup-v0.3.0` branch. If a helper feels "generic",
it almost certainly lives in the external crate, not in this repo.

### Crate boundary policy

Three tiers, from most game-agnostic to most game-specific:

1. **`bevy_common_systems` (external)** - fully game-agnostic Bevy primitives that
   any game could reuse: cameras, skybox, post-processing, mesh builders + explode,
   transform orbits, PD controller, generic `Health`, status bar, the event queue.
   Code only earns a place here once it is genuinely reusable and stable.
2. **`nova_gameplay`** - the umbrella for gameplay plugins: spaceship sections,
   integrity/health-of-sections, weapons, hud, input, camera controller. It also
   holds generic-*leaning* helpers that are **not yet ready** to be promoted to
   `bevy_common_systems` - promotion is deliberate, not automatic.
3. **`nova_core`** - thin wiring only; no gameplay logic. The editor scene it used to
   hold now lives in its own `nova_editor` crate (`GameStates` moved down to
   `nova_gameplay`), so `nova_core` is purely the `AppBuilder` assembler.

Audit finding (task 20260525-132936): the `nova_gameplay` boundary is clean. Every
spaceship/section/weapon/input/camera module is correctly placed, and nothing
gameplay-specific is stranded in another crate. A few modules are game-agnostic
enough to *eventually* promote to `bevy_common_systems`, but promotion is now a
cross-repo change (that crate is a separate repository), so they legitimately stay in
`nova_gameplay` under tier 2 until promoted. Tracked promotion candidates:

- `hud/health.rs` - a text HUD over the generic `Health` component (no Nova coupling).
- `hud/objectives.rs` - a generic id+message objectives text list.
- `hud/velocity.rs` - the `DirectionMagnitudeMaterial` / `DirectionSphereMaterial`
  shader materials (would also move `shaders/directional_*.wgsl`).
- `integrity/blast.rs` + `calculate_blast_damage` / `on_impact_collision_deal_damage`
  in `integrity/mod.rs` - radial-falloff blast volume and impulse/energy collision
  damage that only touch Avian physics and the generic `Health`.

These are captured as a follow-up task rather than actioned here, since they belong to
the external crate's backlog and require a coordinated cross-repo change.

## App assembly

`AppBuilder` (in `crates/nova_core/src/lib.rs`) is the single place the app is wired:

```rust
AppBuilder::new()                 // Bevy DefaultPlugins + window/log setup
    .with_game_plugins(my_plugin) // optional: your own systems/observers
    .with_rendering(true)         // debug-only toggle for headless runs
    .build()                      // adds the plugin stack, returns App
```

`build()` adds, in order: enhanced input, `GameAssetsPlugin`, `NovaGameplayPlugin`,
`NovaScenarioPlugin`, the editor `NovaEditorPlugin` (only when no custom game plugins
were supplied), and (only under `debug`) `DebugPlugin`. (Bevy `DefaultPlugins`,
including UI widgets, are added by `AppBuilder::new()`; avian3d `PhysicsPlugins` comes
in via `NovaGameplayPlugin`, see below.)

`NovaGameplayPlugin` in turn pulls in physics, `bevy_hanabi` particles, `bevy_rand`,
all the `bevy_common_systems` helper plugins, and the Nova sub-plugins (input,
sections, hud, camera controller, integrity).

## States

Two independent state machines:

- `GameStates { Loading, Playing }` - top-level app state (`nova_gameplay`).
- `GameAssetsStates { Loading, Processing, Loaded }` - asset pipeline (`nova_assets`).
  Assets load in `Loading`, get post-processed in `Processing`, and gameplay/scenarios
  begin at `Loaded`. **Scenario setup hooks `OnEnter(GameAssetsStates::Loaded)`** -
  see `examples/03_scenario.rs`.

## Frame flow / system sets

Gameplay systems are grouped into ordered `SystemSet`s so ordering is explicit:

- `SpaceshipSystems { First, Last }` (`nova_gameplay::plugin`) - brackets the
  spaceship update work each frame.
- `SpaceshipSectionSystems`, `IntegritySystems`, `DebugSystems` - per-subsystem sets.

Cross-system communication is done through **events and observers** (Bevy `On<...>`
observers are used heavily, e.g. the whole integrity/destruction chain) rather than
direct calls. Prefer adding an event/observer over coupling two systems directly.

## Assets

`assets/` holds `blender/` sources, exported `gltf/` (`.glb`) models, `textures/`,
`shaders/` (`.wgsl`: thruster exhaust, directional sphere/magnitude), and `icons/`.
The built-in sections and scenarios are currently defined **in Rust code**
(`crates/nova_assets/src/sections.rs` and `scenario.rs`) with a `// This should be
loaded from a JSON file` note - moving them to data files is a known future direction.
