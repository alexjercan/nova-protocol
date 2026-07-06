# Architecture

Nova Protocol is a 3D space shooter built on **Bevy 0.19** with **avian3d** physics.
It is a Cargo workspace: the root `nova-protocol` crate is a thin shell and all the
real code lives under `crates/`.

## Crate map

| Crate           | Responsibility |
|-----------------|----------------|
| `nova-protocol` (root) | `src/main.rs` = clap CLI + entrypoint. `src/lib.rs` re-exports `nova_core`. Examples live in `examples/`. |
| `nova_core`     | `AppBuilder` (assembles every plugin) and `GameStates`. The spaceship **editor scene** is `src/core.rs`. |
| `nova_gameplay` | Nova-specific gameplay. Submodules: `sections/`, `integrity/`, `input/` (player + ai), `hud/`, `camera_controller`, `plugin` (`NovaGameplayPlugin`). |
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

## App assembly

`AppBuilder` (in `crates/nova_core/src/lib.rs`) is the single place the app is wired:

```rust
AppBuilder::new()                 // Bevy DefaultPlugins + window/log setup
    .with_game_plugins(my_plugin) // optional: your own systems/observers
    .with_rendering(true)         // debug-only toggle for headless runs
    .build()                      // adds the plugin stack, returns App
```

`build()` adds, in order: UI widgets, avian3d `PhysicsPlugins`, enhanced input,
`GameAssetsPlugin`, `NovaGameplayPlugin`, `NovaScenarioPlugin`, the editor
`core_plugin`, and (only under `debug`) `DebugPlugin`.

`NovaGameplayPlugin` in turn pulls in physics, `bevy_hanabi` particles, `bevy_rand`,
all the `bevy_common_systems` helper plugins, and the Nova sub-plugins (input,
sections, hud, camera controller, integrity).

## States

Two independent state machines:

- `GameStates { Loading, Playing }` - top-level app state (`nova_core`).
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
