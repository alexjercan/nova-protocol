# Architecture

> New to the codebase? Start with the [Project tour](project-tour.md) for a
> faster orientation, then come back here for the detail.

Nova Protocol is a 3D space shooter built on **Bevy 0.19** with **avian3d** physics.
It is a Cargo workspace: the root `nova-protocol` crate is a thin shell and all the
real code lives under `crates/`.

## Crate map

| Crate           | Responsibility |
|-----------------|----------------|
| `nova-protocol` (root) | `src/main.rs` = clap CLI + entrypoint. `src/lib.rs` re-exports `nova_core`. Runnable examples in `examples/`. |
| `nova_core`     | Thin wiring only: `AppBuilder` assembles every plugin (window/log/asset setup, status UI). No gameplay logic. |
| `nova_menu`     | Main menu (owns the `MainMenu` state UI: New Game / Sandbox / Settings / Exit) and the ESC pause overlay. Buttons write `GameMode` and hand off to `Playing`. The Settings modal (audio volume, graphics preset, read-only keybind reference) is shared by both entry points and persisted cross-platform in `settings_store` (RON file / localStorage). |
| `nova_editor`   | The ship editor scene (`NovaEditorPlugin`). Comes up on entering `Playing`, only in `GameMode::Sandbox`. |
| `nova_gameplay` | The shared gameplay layer under the ship: `integrity/`, `damage`, `gravity` (gravity wells), `markers` (the entity markers the ship tags with and this layer reads), `math`, `audio` (the generic SFX engine `nova_menu` and `nova_os_ui` also use), `juice`, `shake`, `settings` (`MasterVolume`/`GraphicsQuality` + apply systems), `mesh`, `transform`, `relations`, `beacon`, `objectives` (the `GameObjectives` list, its panel and the conveyance tags), `lifetime` (`TempEntity`/`DespawnEntity`), `cooldown`, `plugin`. Also owns `GameStates`, `PauseStates`, and the `GameMode` resource. Knows nothing about a ship. |
| `nova_ship`     | The ship and how it is flown: `sections/` (the modular hull and its ammo/damage tint), `input/` (player rigs, the AI pilot and gunner, radar targeting with deliberate lock-on, the `reference` keybind table), `flight/` (the diegetic controller and its autopilot verbs), `camera/` (the chase-camera controller and the chase/skybox/post/WASD rigs under it), `physics/` (the PD attitude controller) and `ship_audio/` (the soundtrack those five produce). Depends on `nova_gameplay` and never the reverse; `NovaShipPlugin` owns the `SpaceshipSystems` brackets and `nova_core` adds it after `NovaGameplayPlugin`. |
| `nova_hud`      | The flight HUD: one module per widget (crosshairs, target inset, ammo readout, flight status, objective markers, the comms panel, the keybind dock, the screen-indicator projection they all share). Reads gameplay state and never drives it, so the dependency runs `nova_hud -> nova_gameplay`. `nova_core` adds `NovaHudPlugin` render-gated, and the crate places `NovaHudSystems` between the section and camera sets itself. |
| `nova_os`       | NOVA OS logic with no UI in it: the terminal model (`terminal`), the shell command language and typo suggestions (`shell`), and the app runtime seam (`app`). |
| `nova_os_ui`    | The NOVA OS cockpit monitor the player opens with Tab: the CRT casing and shader, the terminal nodes and keyboard/pointer systems (`terminal`), and the two apps that run on it - `map` (schematic local space) and `ship` (schematic player ship). A PEER of the flight HUD, not one of its widgets: `nova_core` adds it, and nothing in `nova_hud` reaches into it (it reads `NovaHudAssets` and `NovaHudSystems`, so it sits ABOVE `nova_hud`). |
| `nova_scenario` | Scenario/modding engine: `events`, `filters`, `actions`, `variables`, `world`, `loader`, `objects/`, `lint/` (the scenario half of the `content -- lint` checks), `render_scale` (the Low-preset resolution lever: scenario view into a reduced offscreen target, upscaled to the window). See [Scenario engine](scenario-system.md). |
| `nova_events`   | Game event kinds and entity identity components, shared between gameplay and scenario. |
| `nova_events_macros` | Procedural macros behind `nova_events`' derives. |
| `nova_assets`   | `bevy_asset_loader` setup. Loads glb/textures/shaders/sounds, and loads the base game's own generated content (`assets/base/`) through the same bundle machinery as mods. Owns the mod merge (`register_bundles`, `EnabledMods`, `ModCatalog`), the portal client and downloads (`portal/`), and prefs persistence. |
| `nova_modding`  | Bundle/content/catalog ASSET LOADERS and the `Content` routing enum. See [Mod files](https://alexjercan.github.io/nova-protocol/create/mod-files/). |
| `nova_mod_format` | Pure serde types for the mod formats (bundle manifests, catalog declarations, the portal wire schema). Engine-free; re-exported by `nova_modding`. The static mod portal is built by `scripts/gen-portal.py`, not a crate. See [Publish a mod](https://alexjercan.github.io/nova-protocol/create/publish-a-mod/). |
| `nova_ui`       | Shared UI, a leaf crate everything that renders UI draws from: the theme palette/metrics (`theme::*`), the `UiSkin` visual-language switch (`skin`), the themed widgets (`widget`: button, slider, segmented control, list rows, panel chrome), screen-level composition (`screen`: scrollable viewports and the list-beside-details layout the menu screens and the NOVA OS drawer share), the flight-HUD chip language (`hud`), player-facing unit formatting (`units`), the shared typeface (`font`) and the generic `status_bar`. Consumed by `nova_gameplay`, `nova_hud`, `nova_os_ui`, `nova_menu`, `nova_editor` and `nova_assets`. |
| `nova_debug`    | Debug-only plugin (inspector, overlays). Compiled only under the `debug` feature. |
| `nova_info`     | Exposes `APP_VERSION`, injected by `build.rs`. |
| `nova_autopilot` | Scripted automation drivers and the run-completion protocol the harness examples share. Engine-facing but game-agnostic; `nova_debug`, `nova_probe` and `nova_probe_cli` all build on it. See [Automation harness](automation-harness.md). |
| `nova_probe`    | Dev tooling (not in the shipped game): the IN-GAME half of the run-harness - the capability plugins an example wires to collect evidence about its own run (`capabilities::frametime`, `capabilities::timeline`, `capabilities::invariants`, bundled by `NovaProbePlugin`), the `contract` an example declares, and the wire format the host reads. See [Development](development.md). |
| `nova_probe_cli` | Dev tooling: the HOST half of the run-harness - spawns autopilot runs as child processes, grades their artifacts (`evaluation`) and renders the reports (`report`). Owns the `cargo run --features debug probe run/report` CLI. The two halves meet at the filesystem: nothing in `nova_probe` reads a run's output back. |
| `nova_perf_web` | The wasm app `probe run --platform web` boots and measures: the real game started into a scenario with the frame-time capture armed. Dev tooling, never shipped. |
| `nova_authoring` | The OFFLINE half of the content pipeline (never shipped): the Rust builders that define every built-in scenario and section, the `content -- gen` serializer that writes them to the committed `assets/base/**/*.content.ron`, and the `content -- lint` walk that validates a content tree. |
| `nova_meta_gen` | Binary under `tools/` (web-build tooling, not a game crate): writes default `.meta` sidecars for web assets that lack one (a Trunk `post_build` hook for `AssetMetaCheck::Always`). Boots a headless Bevy app, so it stays Rust. |

The dependency layering the table describes, from top-level shell down to leaf crates:

```mermaid
graph TD
    root["nova-protocol (root)"] --> core["nova_core"]
    core --> menu["nova_menu"]
    core --> editor["nova_editor"]
    core --> gameplay["nova_gameplay"]
    core --> ship["nova_ship"]
    core --> hud["nova_hud"]
    core --> osui["nova_os_ui"]
    core --> scenario["nova_scenario"]
    core --> assets["nova_assets"]
    ship --> gameplay
    ship --> events["nova_events"]
    hud --> ship
    hud --> gameplay
    hud --> ui["nova_ui"]
    osui --> hud
    osui --> ship
    osui --> gameplay
    osui --> os["nova_os"]
    osui --> ui
    menu --> osui
    menu --> ui
    editor --> ship
    editor --> ui
    gameplay --> events
    gameplay --> ui
    scenario --> events
    scenario --> gameplay
    scenario --> ship
    scenario --> hud
    assets --> modding["nova_modding"]
    assets --> scenario
    modding --> modfmt["nova_mod_format"]
    core --> debug["nova_debug"]
    core --> info["nova_info"]
```

The graph is curated for readability, not exhaustive: `nova_core` and `nova_menu`
depend on nearly every crate below them, and edges implied by the layering (for
example `nova_menu -> nova_ship`) are pruned. The authoritative dependency list
for any crate is its `Cargo.toml`. Two edges people guess wrong:

- **`nova_ui` is not menu-only.** Every crate that renders UI draws on it -
  `nova_gameplay` (which adds `NovaUiPlugin` render-gated), `nova_hud` (the
  chip language and screen-indicator styling), `nova_os_ui`, `nova_menu`,
  `nova_editor` and `nova_assets`.
- **`nova_scenario` reaches up into `nova_ship` and `nova_hud`.** It spawns
  ships and drives HUD-facing surfaces (comms dwell limits, target-inset render
  targets), so it sits beside them, not below them.

The dev-tool crates hang off this graph without joining it: `nova_debug` builds
on `nova_autopilot` plus the game crates it inspects; `nova_probe` wires
`nova_core` + `nova_autopilot` into a measurable app; `nova_probe_cli` depends
only on `nova_probe`, `nova_autopilot` and `nova_assets` (it is a host process,
not a game plugin); `nova_perf_web` is `nova_core` + `nova_probe`; and
`nova_authoring` reads half the workspace to build and lint content offline.

Every crate exposes a `pub mod prelude`. Import from the prelude
(`use nova_gameplay::prelude::*`), not from inner modules. `nova_core::prelude`
re-exports all sub-crate preludes, so top-level code and examples usually just do
`use nova_protocol::prelude::*`.

### Generic helpers live here too

The generic, non-Nova Bevy helpers (WASD/chase cameras, skybox, post-processing,
mesh explode, PD controller, health, status bar, the generic game-event queue
`GameEventsPlugin`/`EventWorld`) are nova's own: the camera and transform rigs,
the mesh toolkit in `nova_gameplay`, the camera rigs and the PD controller in
`nova_ship`, the status bar and
tween in `nova_ui`, the event engine in `nova_events`, the inspector and
wireframe layers in `nova_debug`. They used to live in a separate pinned repo;
task 20260806-180450 vendored them in, because splitting them out before the
game was done produced generic-looking code shaped by one game's needs. Whether
any of it deserves extracting is a question for after the game ships.

The generic `HealthDisplay` bar stays here (still available for other games and
for non-player entities), but Nova's player-ship health readout is no longer that
bar: it is diegetic, grading each ship section's own mesh material by integrity
(`nova_ship::sections::damage_tint`, task 20260717-003613). Because that
readout keys on Nova's section graph and materials it is game-specific and is NOT
a promotion candidate - the generic bar and the diegetic readout are different
things at different layers.

Boundary policy, from most game-agnostic to most game-specific:

1. The generic-leaning modules named above - reusable Bevy primitives that
   happen to live in a nova crate; keep them free of game-specific types.
2. `nova_gameplay` - the shared gameplay layer, ship-agnostic.
3. `nova_ship` - the ship above that layer.
4. `nova_hud` and `nova_os_ui` - consumers of the ship and of gameplay, above
   both. `nova_os_ui` is above `nova_hud` in turn: it orders itself against
   `NovaHudSystems`.
5. `nova_core` - wiring only.

## App assembly

`AppBuilder` (in `crates/nova_core/src/lib.rs`) is the single place the app is wired:

```rust
AppBuilder::new()                 // Bevy DefaultPlugins + window/log/asset/render setup
    .with_game_plugins(my_plugin) // optional: your own systems/observers
    .with_rendering(true)         // debug-only toggle for headless runs
    .build()                      // adds the plugin stack, returns App
```

`build()` inits `GameStates` + `PauseStates`, then adds, in order:
`EnhancedInputPlugin`, `GameAssetsPlugin`, `LoadingScreenPlugin`,
`NovaGameplayPlugin`, `NovaShipPlugin` (the ship orders its sets inside
gameplay's `SpaceshipSystems` brackets, so it comes after), `NovaScenarioPlugin`,
then - render-gated, a headless harness run draws neither - `NovaHudPlugin` and
`NovaOsUiPlugin` (HUD first: the monitor orders itself against
`NovaHudSystems`), then `NovaEditorPlugin` and `NovaMenuPlugin` (both only when
no custom game plugins were supplied - the menu fronts the default app and
nothing else, so an example that brings its own game plugins goes straight
`Loading -> Playing`), and finally `DebugPlugin` under the `debug` feature. On
`OnEnter(GameAssetsStates::Loaded)` it hands off to `MainMenu` (or straight to
`Playing` when the menu is off) and spawns the status UI.

`NovaGameplayPlugin` pulls in avian3d `PhysicsPlugins` (zero gravity, projectile
collision hooks), `bevy_rand`, `bevy_hanabi` particles (on wasm via the WebGPU
backend), `NovaUiPlugin` (render-gated), the transform/lifetime/mesh rigs, and
the shared gameplay sub-plugins: integrity, damage, gravity, relations, audio,
juice, settings. The ship stack (input, sections, flight, camera, physics) is
`NovaShipPlugin`'s, and the HUD is `NovaHudPlugin`'s - both added by
`nova_core`, not by gameplay.

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
  `examples/systems/scenario_grammar.rs`.

The top-level lifecycle, the pause overlay nested inside `Playing`, and the asset
pipeline that gates entry:

```mermaid
stateDiagram-v2
    state "GameStates" as GS {
        [*] --> Loading
        Loading --> MainMenu: menu app
        Loading --> Playing: custom game plugins
        MainMenu --> Playing: New Game / Sandbox
        state "Playing" as Playing {
            [*] --> Unpaused
            Unpaused --> Paused: ESC
            Paused --> Unpaused: ESC
        }
    }

    state "GameAssetsStates" as AS {
        [*] --> AsLoading: Loading
        AsLoading --> Processing
        Processing --> Loaded
    }

    AS --> GS: OnEnter(Loaded) hands off to MainMenu / Playing
```

Leaving `Playing` resets `PauseStates` back to `Unpaused`.

## Frame flow

Gameplay systems run in an explicit chain, configured identically in `Update` and
`FixedUpdate`. `nova_ship::NovaShipPlugin` declares the brackets and the ship
sets; `nova_hud` slots `NovaHudSystems` into the gap itself:

```
SpaceshipSystems::First -> SpaceshipInputSystems -> SpaceshipSectionSystems
    -> NovaHudSystems -> NovaCameraSystems -> SpaceshipSystems::Last
```

- Physics (avian3d) runs in `FixedPostUpdate` on a fixed timestep. Rigid bodies get
  `TransformInterpolation` so rendering stays smooth between physics ticks.
- `PostUpdate` hosts the chase camera's final move and the HUD's world-to-screen
  projection, ordered after it.
- While `Paused`, the input and section sets are gated off and the clocks freeze.

The render-rate chain (run in both `Update` and `FixedUpdate`) versus the
fixed-timestep physics step and the interpolation that smooths rendering between
ticks:

```mermaid
flowchart LR
    subgraph render["Update / FixedUpdate chain"]
        first["Spaceship First"] --> input["Spaceship Input"]
        input --> section["Spaceship Section"]
        section --> hud["Nova HUD"]
        hud --> cam["Nova Camera"]
        cam --> last["Spaceship Last"]
    end

    subgraph fixed["FixedPostUpdate"]
        phys["avian3d physics (fixed step)"]
        phys --> interp["TransformInterpolation"]
    end

    subgraph post["PostUpdate"]
        chase["chase camera final move"] --> worldscreen["HUD world-to-screen"]
    end

    render --> fixed --> post
```

### Update vs FixedUpdate - which schedule does my system go in?

The chain above is configured IDENTICALLY in `Update` and `FixedUpdate`
(`nova_ship::NovaShipPlugin`, two `configure_sets` calls with the same set order),
so a gameplay set can host systems in either schedule. The split is not
cosmetic: since every dynamic body opted into avian's `TransformInterpolation`,
the game carries two pose representations on two clocks (see the two-clocks
record, `tasks/20260711-103527/SPIKE.md`):

- **Raw physics pose** -- avian `Position`/`Rotation`, advanced on the 64 Hz
  `FixedUpdate` tick. This is the truth the simulation integrates.
- **Render pose** -- `Transform`, eased between the previous and current physics
  states, with `GlobalTransform` propagated from it in `PostUpdate`.

Which schedule:

- Put a system in **FixedUpdate** when it feeds the physics sim -- forces and
  impulses, spawns whose motion physics integrates (projectiles), guidance. It
  MUST read the raw `Position`/`Rotation` (or compose the root's raw pose with a
  local mount offset). During `FixedUpdate` of frame N, `GlobalTransform` still
  holds the eased pose propagated in frame N-1's `PostUpdate`, so it is stale
  render state here; the avian child-collider pose is one tick stale too.
- Put a system in **Update** (or `PostUpdate`) when it consumes the rendered
  frame -- camera, HUD world-to-screen projection, effects. It reads the eased
  `Transform`/`GlobalTransform`, and every pose in one on-screen computation must
  come from the same frame. A consumer of `PostUpdate`-written state must be
  ordered after its producer.

Why gameplay is split across both: the chain runs in `Update` for
render-rate work and in `FixedUpdate` for sim-rate work; the same set order in
both keeps ordering consistent wherever a system lands.

What breaks if a system lands in the wrong schedule -- worked example: a
`FixedUpdate` system reading `GlobalTransform`. `thruster_impulse_system` used to
apply its impulse at the thruster child's `GlobalTransform`, i.e. the previous
frame's eased pose, up to ~2 ticks of ship motion behind the raw physics it was
pushing, while taking thrust DIRECTION from the raw `Rotation` -- mixing both
clocks in one impulse. The application-point error is proportional to velocity;
at speed a COM-centered engine developed an uncompensated lever arm the throttle
balancer could not see, and the measured failure was a zero-true-torque lateral
engine spinning the hull to 7.1 rad/s in 15 frames (0 rad/s after reading the raw
pose). The fix (`thruster_section.rs`, see the comment above `apply_linear_impulse_at_point`)
composes both application point and thrust direction from the root's raw
`Position`/`Rotation`. The same footgun produced the bullet-spew, HUD-jitter and
crosshair-twitch bugs in that family; all were error proportional to velocity,
which is why they only showed at high speed.

Cross-system communication goes through events and observers (Bevy `On<...>`
observers, e.g. the integrity/destruction chain) rather than direct calls. Prefer
adding an event/observer over coupling two systems.

## Assets

`assets/` is **runtime-only** - everything the game actually loads: `shaders/`
(`.wgsl`), `icons/`, `sounds/` (UI chrome: menu clicks + objective chimes),
and the `base/`/`mods/` data (`.ron`). The base game's own art and world audio
live UNDER `assets/base/` (exported `gltf/` models `.glb`, `textures/`,
`sounds/` `.wav` world cues, `banner.png`), referenced by base content with
`self://` and by mods with `dep://base/<path>`. It is the whole directory the
web (Trunk `copy-dir`) and native (`release.yaml`) builds ship, so non-runtime
files must not live here. The Blender SOURCES the `gltf/` models are exported
from live OUT of the shipped tree, in top-level `art/blender/` (they are
2.7M that was never loaded at runtime). The built-in sections and scenarios ARE
data now: the Rust builders in `crates/nova_authoring` (`sections.rs`,
`scenario.rs`, `scenario/`) are the single source, and
`cargo run content gen` serializes them to the
committed `assets/base/**/*.content.ron` the game loads like any other bundle.
Never hand-edit the generated files; edit the builders and re-run `gen`.

## Find it in the code

- App assembly, plugin order: `AppBuilder` - `crates/nova_core/src/lib.rs`;
  game binary and CLI flags - `src/main.rs`.
- States: `GameStates`, `PauseStates`, `GameMode` -
  `crates/nova_gameplay/src/lib.rs`; ESC overlay and clock freeze -
  `crates/nova_menu/src/pause.rs`.
- Frame-flow sets: `SpaceshipSystems` - `crates/nova_gameplay/src/plugin.rs`;
  chained in `Update` + `FixedUpdate` by `NovaShipPlugin` -
  `crates/nova_ship/src/lib.rs`.
- Asset gate and mod merge: `GameAssetsPlugin` -
  `crates/nova_assets/src/plugin.rs`; `register_bundles` -
  `crates/nova_assets/src/merge.rs`.
- API detail: `cargo doc --open -p nova_core` (any crate from the map works).
