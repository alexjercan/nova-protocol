# AGENTS.md

Orientation for agents working on **Nova Protocol**, a 3D space shooter built with
[Bevy](https://bevyengine.org) 0.19. Read this first, then dive into the crate you need.

## What this is

A spaceship editor + simulation game. Build ships from modular sections (hull,
controller, thruster, turret, torpedo bay) and fly them through scenarios.
Runs natively and on the web (WASM via Trunk).

## Layout

Cargo workspace. The root crate is thin (`src/main.rs` is the CLI entry,
`src/lib.rs` re-exports `nova_core`). Real code lives in `crates/`:

| Crate | What it does |
|-------|--------------|
| `nova_core` | `AppBuilder`: assembles all plugins. Start here. |
| `nova_menu` | Main menu (with live ambient scene) and pause menu. |
| `nova_editor` | Ship editor scene: build UI, keybind chips, play-test transition. |
| `nova_gameplay` | Gameplay: `sections/`, `integrity/`, `input/`, `hud/`, targeting, camera. Owns `GameStates`. |
| `nova_scenario` | Scenario/modding engine: actions, events, filters, variables, loader, objects. |
| `nova_events` | Game event kinds and entity id/type-name components. |
| `nova_assets` | Asset loading; registers sections and scenarios. |
| `nova_debug` | Debug tooling (inspector, wireframe, overlays). Only under the `debug` feature. |
| `nova_info` | `APP_VERSION`, set by `build.rs`. |

Import through each crate's `prelude` (`use nova_gameplay::prelude::*`), not deep
module paths. New public items go through the prelude too.

Generic Bevy helpers (camera, mesh builders, math, the event queue) live in
**`bevy-common-systems`**: our own repo, pinned as a git dependency in
`crates/nova_gameplay/Cargo.toml`, local checkout at `~/personal/bevy-common-systems`.
Need a change there? Make it there (same task flow), then bump the pinned `rev` here.

## Build, run, test

Nightly toolchain (`rust-toolchain.toml`). On NixOS: `nix develop`.

```sh
cargo run                        # the game (boots into the main menu)
cargo run --features dev         # + debug tooling (inspector, wireframe)
cargo run --example 03_scenario  # examples are the fastest way to test a subsystem
trunk serve                      # web build on :8080
cargo check && cargo fmt         # do this before committing
```

Do NOT run `cargo test` or `cargo clippy` locally unless asked: CI
(`.github/workflows/ci.yaml`) runs both on every PR and push to master, and the
local suite is slow. When you skip them, say so plainly; CI is the source of truth.

Features: `debug` gates all debug tooling; `dev` = `debug`. The `--norender` and
`--debugdump` CLI flags exist only under `debug`.

For substantial features, prefer a runnable example in `examples/` over unit tests.

## How the app is assembled

`AppBuilder::new().with_game_plugins(...).build()` in `crates/nova_core/src/lib.rs`.
Order: Bevy defaults, enhanced input, assets, gameplay, scenario, then (when no
custom game plugins) the editor, the main menu (`with_main_menu(bool)` overrides),
and `debug`-gated debug tooling. avian3d physics comes in via `NovaGameplayPlugin`.

States: `GameStates::{Loading, MainMenu, Playing}` and
`GameAssetsStates::{Loading, Processing, Loaded}`. Scenario setup usually hooks
`OnEnter(GameAssetsStates::Loaded)`.

## Conventions

- Global rules from `~/AGENTS.md` apply: plain ASCII punctuation, plain commit
  messages with no AI attribution, no time-based technical arguments.
- Bevy idioms: one plugin per subsystem, systems grouped in `SystemSet`s,
  subsystems talk via events (`nova_events`), not direct coupling.
- Read `docs/LESSONS.md` before starting work: the short list of
  mistakes this repo has already paid for.
- Worktrees come only from the sprout skill (used by /work and /flow). Never
  create one by hand. Otherwise work in the main checkout.
- Document meaningful changes: a `NOTES.md` or `RETRO.md` in the task's folder
  (see below), or the relevant reference doc in `docs/`.

## Where records go (/plan, /spike, /work, /review, /compound, /flow)

Everything tied to one task lives in that task's folder - never as loose
`.md` files under `docs/`:

- `tasks/<id>/TASK.md` - the task (tatr).
- `tasks/<id>/SPIKE.md` - the spike/research doc (/spike).
- `tasks/<id>/REVIEW.md` - review rounds and verdict (/review); PR-level
  reviews go on the PR's primary task.
- `tasks/<id>/RETRO.md` - the retrospective (/compound).
- `tasks/<id>/NOTES.md` - design/fix record for the shipped change.

`docs/` keeps only: the reference docs, `docs/plans/` (multi-task plans), and
`docs/LESSONS.md` (the ledger /compound appends to). If a skill's default
output path says `docs/retros/` or `docs/spikes/`, use the locations above
instead; the ledger is `docs/LESSONS.md`, never `docs/retros/LESSONS.md`.

## Docs, tasks, versioning

- `docs/README.md` is the index. Reference docs: `architecture.md`,
  `scenario-system.md`, `sections.md`, `development.md` (build/web/release).
- Tasks: `tatr` CLI, markdown files under `tasks/`. Check the backlog before
  starting, close tasks when done. Skills: /plan, /work, /review, /compound, /flow.
- Version lives in root `Cargo.toml` (`workspace.package.version`). Notable
  changes go to `CHANGELOG.md` (Keep a Changelog). Release steps:
  `docs/development.md`.
