# AGENTS.md

Orientation for agents working on **Nova Protocol**, a 3D space shooter built with
[Bevy](https://bevyengine.org) 0.19. Read this first, then dive into the relevant crate.

## What this project is

A spaceship editor + simulation game. You build ships out of modular sections
(hull, controller, thruster, turret, torpedo bay) and run them through scenarios
(asteroid fields, zones, objectives). Runs natively and on the web (WASM via Trunk).

## Workspace layout

Cargo workspace. The root crate `nova-protocol` is thin: `src/main.rs` is the CLI
entrypoint, `src/lib.rs` just re-exports `nova_core`. Real code lives in `crates/`:

| Crate           | Responsibility |
|-----------------|----------------|
| `nova_core`     | `AppBuilder` (assembles all plugins). Thin wiring only. Start here to see how the app is wired. |
| `nova_editor`   | The spaceship editor scene (`NovaEditorPlugin`, `src/lib.rs`): section-picker UI, ship building, transition into the scenario sim. Added by default when no custom game plugins are supplied. |
| `nova_gameplay` | Nova-specific gameplay: ship `sections/`, `integrity/` (health/blast/explode), `input/` (player + ai), `hud/`, `camera_controller`. Also owns `GameStates`. |
| `nova_scenario` | Scenario/modding engine: `actions`, `events`, `filters`, `variables`, `world`, `loader`, and scenario `objects/` (area, asteroid, spaceship). |
| `nova_events`   | Game event kinds (`OnStart`, `OnUpdate`, `OnDestroyed`, `OnEnter`, `OnExit`) and entity id/type-name components. |
| `nova_assets`   | `bevy_asset_loader` setup; loads glb/textures/shaders and registers sections + scenarios. |
| `nova_debug`    | Debug-only plugin (inspector, wireframe, section overlays). Compiled only under the `debug` feature. |
| `nova_info`     | Exposes `APP_VERSION` (set by `build.rs` via the `APP_VERSION` env var). |

Each crate exposes a `pub mod prelude`. Import from the prelude
(`use nova_gameplay::prelude::*`) rather than reaching into modules directly.

### External shared crate

Generic, non-Nova Bevy helpers (camera, mesh builders, transforms, math, the game
event queue) live in **`bevy-common-systems`**, a separate repo pinned as a git
dependency (`crates/nova_gameplay/Cargo.toml`). It used to be vendored under
`crates/bevy_common_systems/`; that copy is being deleted on the current cleanup
branch. If you need a generic helper, it is probably in that external crate,
re-exported through `nova_gameplay::prelude`.

## Build, run, test

Toolchain is **nightly** (`rust-toolchain.toml`). On NixOS use `nix develop` to get
the dev shell with all the system libs (udev, alsa, vulkan, X11/wayland, trunk).

```sh
cargo run                      # run the game (editor scene)
cargo run --features dev       # dev build: enables debug (inspector, wireframe, etc.)
cargo run --example 03_scenario   # run an example (see examples/)
cargo build --release          # release profile (opt=s, lto, stripped)
trunk serve                    # web build, serves on :8080 (see Trunk.toml)
cargo clippy --all-targets     # lint (workspace lints in root Cargo.toml)
cargo fmt                      # rustfmt.toml at root
cargo test                     # tests
```

Features: `debug` gates all debug tooling; `dev` = `debug`. The `--norender` and
`--debugdump` CLI flags exist only under the `debug` feature (`--debugdump` writes a
system schedule graph and exits).

**Skip `cargo test` and `cargo clippy` locally unless explicitly asked to run
them** - both run in CI on every PR, so local runs mostly burn time (the workspace
test suite alone takes ~1-2 minutes because of the examples smoke test). A quick
`cargo check` (and `cargo fmt`) before committing is still fine - it is cheap and
catches breakage early. When tests were not run locally, say so plainly instead of
claiming the suite is green; CI is the source of truth.

Examples in `examples/` (`01_scene`, `02_thruster_shader`, `03_scenario`,
`04_asteroids`, `05_directional`, `07b_slicer`) are the fastest way to exercise a
subsystem end to end. When adding a substantial feature, prefer wiring up an example
over unit tests (see repo conventions below).

## How the app is assembled

`AppBuilder::new().with_game_plugins(...).with_rendering(bool).build()` in
`crates/nova_core/src/lib.rs`. It adds, in order: Bevy default plugins (UI widgets are
part of `DefaultPlugins` on 0.19, so they are not re-added), enhanced input,
`GameAssetsPlugin`, `NovaGameplayPlugin`, `NovaScenarioPlugin`, and (only when no custom
game plugins were supplied) the editor `NovaEditorPlugin` from `nova_editor`, plus
(under `debug`) `DebugPlugin`. avian3d `PhysicsPlugins` is pulled in by
`NovaGameplayPlugin`, not here. State machine: `GameStates::{Loading, Playing}` and
`GameAssetsStates::{Loading, Processing, Loaded}`. Scenario setup typically hooks
`OnEnter(GameAssetsStates::Loaded)`.

## Conventions

- Follow the global rules in `~/AGENTS.md`: plain ASCII punctuation (no em dashes,
  smart quotes, arrows), plain commit messages with no AI attribution, no
  time-based technical arguments.
- Bevy idioms: plugin-per-subsystem, systems grouped into `SystemSet`s, communicate
  via events (`nova_events`) rather than direct coupling.
- Keep the module/prelude pattern: new public items go through the crate's `prelude`.
- Document meaningful changes in `docs/` per the global reflection guideline: what
  changed and why, difficulties, and lessons.

## Deeper docs

The `docs/` folder has the detail this file only summarizes:

- `docs/architecture.md` - crate map, plugin wiring, states, frame flow.
- `docs/scenario-system.md` - the scenario/modding engine (events/filters/actions).
- `docs/sections.md` - spaceship sections + the integrity/destruction system.
- `docs/development.md` - toolchain, build/run/test, features, web build, release.

## Task tracking

This repo uses the `tatr` CLI (tasks stored as markdown under `tasks/`). Check the
backlog before starting and close tasks when done. Related skills: `/plan`, `/work`,
`/review`, `/compound`, `/flow`.

## Versioning

Workspace version in root `Cargo.toml` (`workspace.package.version`), currently
`0.3.1`. Update `CHANGELOG.md` (Keep a Changelog format) for notable changes. See
`docs/development.md` for the full release process.
