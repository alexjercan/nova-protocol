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
cargo run --example 08_scenario  # examples are the fastest way to test a subsystem
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

- The durable reference docs live in the wiki source under `web/src/wiki/dev/`
  (rendered as public pages at `/wiki/dev/`): `architecture.md`,
  `scenario-system.md`, `sections.md`, `development.md` (build/web/release),
  `modding-ron.md`, `mod-portal.md`, and `keeping-docs-in-sync.md` (the map of
  which docs to update when you change code or cut a release). Edit them there
  and keep them accurate when the code they describe changes. `docs/` now holds
  only transient records - the `LESSONS.md` ledger, `plans/`, per-task folders,
  and one-off writeups like the Bevy migration notes; `docs/README.md` indexes it.
- Tasks: `tatr` CLI, markdown files under `tasks/`. Check the backlog before
  starting, close tasks when done. Skills: /plan, /work, /review, /compound, /flow.
- Task tags encode scheduling - EVERY new tatr task carries exactly one of:
  - `backlog` with priority 0: not scheduled for the current release; or
  - the current release tag (e.g. `v0.7.0` - the newest `vX.Y.Z` plan in
    `docs/plans/`): scheduled work, with a priority slotted RELATIVE to the
    other open tasks of that release (`tatr ls -f ':tags contains vX.Y.Z'
    --sort priority` first, then pick where it belongs).
  Topical tags (`bug`, `scenario`, `ui`, ...) come on top. Pulling a backlog
  task into a release means swapping the tag and re-slotting the priority.
- Version lives in root `Cargo.toml` (`workspace.package.version`). Notable
  changes go to `CHANGELOG.md` (Keep a Changelog). Release steps:
  `web/src/wiki/dev/development.md`.

## The website (`web/`)

`web/` is the public site (landing, news, tutorial, wiki): TypeScript + Webpack
+ Tailwind, deployed to GitHub Pages with the game served under `/play/`. It is
content, not generated - it does not update itself, so keep it in sync by hand
whenever a code change makes something on it wrong or missing. The full map of
what to touch when is a dev wiki page, **Keeping docs in sync**
(`web/src/wiki/dev/keeping-docs-in-sync.md`, published at
`/wiki/dev/keeping-docs-in-sync/`). The short version:

- **Player-facing behavior changed** (controls, verbs, HUD, menu flow, a
  section, a weapon, a scenario primitive): fix the affected player wiki pages
  under `web/src/wiki/*.md`, plus `web/src/tutorial.html` if the first-flight
  flow moved. A page drifting behind the game is exactly the failure to avoid -
  change a keybind or a targeting rule, fix its doc in the same task.
- **Internals or a data format changed**: fix the dev wiki pages under
  `web/src/wiki/dev/*.md` (a RON / bundle / catalog break must land in
  `dev/modding-ron.md` / `dev/mod-portal.md` the same task).
- **A release went out**: update `CHANGELOG.md` (terse, grouped by subsystem)
  and the site's **News** (`/news/`, `web/src/news/`). One post per FEATURE
  release; patch releases fold into their parent post's `## Point releases`
  section, never a post of their own. Full steps are in
  `web/src/wiki/dev/development.md` under "Writing the release news post".
- **New feature or mechanic worth showing**: earn it a screenshot or diagram.
  Use the `.figure` component (see `web/src/style.css`); it ships a dashed
  placeholder naming the image file to capture, so land the slot and caption now
  and drop the real capture in later.

Adding or renaming a page/post edits `web/webpack.config.js` (the page list and
`historyApiFallback` rewrites), plus the wiki manifest `web/src/wiki-pages.ts`
for a wiki page, or `NEWS_POSTS` + a card in `web/src/news.html` for a news
post. Verify with `cd web && npm run ci` (format check, lint, build).
