# Architecture

Nova Protocol is a 3D space shooter built on **Bevy 0.19** with **avian3d** physics.
It is a Cargo workspace: the root `nova-protocol` crate is a thin shell and all the
real code lives under `crates/`.

## Crate map

| Crate           | Responsibility |
|-----------------|----------------|
| `nova-protocol` (root) | `src/main.rs` = clap CLI + entrypoint. `src/lib.rs` re-exports `nova_core`. Runnable examples in `examples/`. |
| `nova_core`     | Thin wiring only: `AppBuilder` assembles every plugin (window/log/asset setup, status UI). No gameplay logic. |
| `nova_menu`     | Main menu (owns the `MainMenu` state UI: New Game / Sandbox / Settings / Exit) and the ESC pause overlay. Buttons write `GameMode` and hand off to `Playing`. |
| `nova_editor`   | The ship editor scene (`NovaEditorPlugin`). Comes up on entering `Playing`, only in `GameMode::Sandbox`. |
| `nova_gameplay` | Gameplay umbrella: `sections/`, `integrity/`, `damage`, `flight`, `gravity` (gravity wells), `input/` (player, ai, radar targeting with deliberate lock-on), `hud/` (many widgets: crosshairs, target inset, ammo, flight status, markers, ...), `camera_controller`, `audio`, `juice`, `relations`, `beacon`, `objective_marker`, `plugin`. Also owns `GameStates`, `PauseStates`, and the `GameMode` resource. |
| `nova_scenario` | Scenario/modding engine: `events`, `filters`, `actions`, `variables`, `world`, `loader`, `objects/`. See [scenario-system.md](scenario-system.md). |
| `nova_events`   | Game event kinds and entity identity components, shared between gameplay and scenario. |
| `nova_assets`   | `bevy_asset_loader` setup. Loads glb/textures/shaders/sounds, then registers the built-in sections and scenarios. |
| `nova_debug`    | Debug-only plugin (inspector, overlays). Compiled only under the `debug` feature. |
| `nova_info`     | Exposes `APP_VERSION`, injected by `build.rs`. |

Every crate exposes a `pub mod prelude`. Import from the prelude
(`use nova_gameplay::prelude::*`), not from inner modules. `nova_core::prelude`
re-exports all sub-crate preludes, so top-level code and examples usually just do
`use nova_protocol::prelude::*`.

### External shared crate: `bevy-common-systems`

Generic, non-Nova Bevy helpers (WASD/chase cameras, skybox, post-processing, mesh
explode, PD controller, health, status bar, the generic game-event queue
`GameEventsPlugin`/`EventWorld`) live in a separate repo, `bevy-common-systems`,
pinned as a git dependency (rev in `crates/nova_gameplay/Cargo.toml`) and re-exported
through `nova_gameplay::prelude`. If a helper feels "generic", it lives there.

Boundary policy, from most game-agnostic to most game-specific:

1. `bevy_common_systems` (external) - fully reusable Bevy primitives.
2. `nova_gameplay` - Nova gameplay, plus generic-leaning helpers not yet ready for
   promotion (promotion is a deliberate cross-repo change, not automatic).
3. `nova_core` - wiring only.

## App assembly

`AppBuilder` (in `crates/nova_core/src/lib.rs`) is the single place the app is wired:

```rust
AppBuilder::new()                 // Bevy DefaultPlugins + window/log/asset setup
    .with_game_plugins(my_plugin) // optional: your own systems/observers
    .with_rendering(true)         // debug-only toggle for headless runs
    .build()                      // adds the plugin stack, returns App
```

`build()` inits `GameStates` + `PauseStates`, then adds, in order:
`EnhancedInputPlugin`, `GameAssetsPlugin`, `NovaGameplayPlugin`,
`NovaScenarioPlugin`, then `NovaEditorPlugin` and `NovaMenuPlugin` (both only when
no custom game plugins were supplied; `with_main_menu(bool)` overrides the menu
default), and finally `DebugPlugin` under the `debug` feature. On
`OnEnter(GameAssetsStates::Loaded)` it hands off to `MainMenu` (or straight to
`Playing` when the menu is off) and spawns the status UI.

`NovaGameplayPlugin` pulls in avian3d `PhysicsPlugins` (zero gravity, projectile
collision hooks), `bevy_rand`, `bevy_hanabi` particles (not on wasm), the
`bevy_common_systems` helper plugins, and the Nova sub-plugins: input, sections,
hud, camera controller, integrity, damage, flight, gravity, relations, audio, juice.

## States

- `GameStates { Loading, MainMenu, Playing }` (`nova_gameplay`) - top-level
  lifecycle. `MainMenu` only occurs when `NovaMenuPlugin` fronts the app (the
  default editor app); examples with custom game plugins go straight
  `Loading -> Playing`. The `GameMode` resource (`Sandbox` default | `NewGame`)
  records what the menu handed off to.
- `PauseStates { Unpaused, Paused }` - the ESC pause overlay. `nova_gameplay` owns
  the enum and gates the spaceship sets; `nova_menu` owns the toggle, the overlay
  UI, and the clock freeze (`Time<Virtual>` + `Time<Physics>`). Only meaningful
  inside `Playing`; leaving `Playing` resets it.
- `GameAssetsStates { Loading, Processing, Loaded }` (`nova_assets`) - asset
  pipeline. Scenario setup hooks `OnEnter(GameAssetsStates::Loaded)` - see
  `examples/08_scenario.rs`.

## Frame flow

Gameplay systems run in an explicit chain, configured identically in `Update` and
`FixedUpdate` (`nova_gameplay::plugin`):

```
SpaceshipSystems::First -> SpaceshipInputSystems -> SpaceshipSectionSystems
    -> NovaHudSystems -> NovaCameraSystems -> SpaceshipSystems::Last
```

- Physics (avian3d) runs in `FixedPostUpdate` on a fixed timestep. Rigid bodies get
  `TransformInterpolation` so rendering stays smooth between physics ticks.
- `PostUpdate` hosts the chase camera's final move and the HUD's world-to-screen
  projection, ordered after it.
- While `Paused`, the input and section sets are gated off and the clocks freeze.

Cross-system communication goes through events and observers (Bevy `On<...>`
observers, e.g. the integrity/destruction chain) rather than direct calls. Prefer
adding an event/observer over coupling two systems.

## Assets

`assets/` holds `blender/` sources, exported `gltf/` models, `textures/`,
`shaders/` (`.wgsl`), `sounds/`, and `icons/`. The built-in sections and scenarios
are defined in Rust (`crates/nova_assets/src/sections.rs`, `scenario.rs`,
`scenario/`); moving them to data files is a known future direction.
