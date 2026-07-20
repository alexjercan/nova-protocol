# AGENTS.md

Orientation for agents working on **Nova Protocol**, a 3D space shooter built
with [Bevy](https://bevyengine.org) 0.19. Read this first, then
`LESSONS.md` (see below), then dive into the crate you need.

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
| `nova_gameplay` | Sections, integrity, input, HUD, targeting, flight/autopilot, AI, camera. Owns `GameStates`. |
| `nova_scenario` | Scenario/modding engine: actions, events, filters, variables, objects, the content lint. |
| `nova_assets` | Asset loading; content builders and the `content` CLI (gen/lint; balance audit + input-overlap folded into lint). |
| `nova_modding` | Mod loading/merging: bundles, installed catalog, portal client, downloads. |
| `nova_mod_format` | Engine-free serde types for the mod formats (portal wire schema). |
| `nova_menu` | Main/pause menus, settings, mods UI, scenarios picker. |
| `nova_editor` | Ship editor scene: build UI, keybind chips, play-test transition. |
| `nova_ui` | Shared UI theme + widgets (menu, editor, HUD chrome). |
| `nova_events` | Game event kinds and entity id/type-name components. |
| `nova_info` | `APP_VERSION`, set by `build.rs`. |
| `nova_debug` | Debug tooling (inspector, wireframe, overlays); `debug` feature only. |
| `nova_probe` | Run-harness: frame-time capture + perf reporting over autopilot runs. |
| `nova_meta_gen` | `.meta` sidecar generator for the web build (Trunk post_build hook). |
| `nova_portal_gen` | Static mod-portal generator (`webmods/` -> `site/mods`). |

Import through each crate's `prelude` (`use nova_gameplay::prelude::*`), not
deep module paths; new public items go through the prelude too.

Generic Bevy helpers (camera, mesh builders, math, the event queue) live in
**`bevy-common-systems`**: our own repo, pinned as a git dependency in
`crates/nova_gameplay/Cargo.toml`, local checkout at
`~/personal/bevy-common-systems`. Need a change there? Make it there (same
task flow), then bump the pin here.

## Build, run, test

Nightly toolchain (`rust-toolchain.toml`). On NixOS: `nix develop`.

```sh
cargo run                        # the game (boots into the main menu)
cargo run --features dev         # + debug tooling (inspector, wireframe)
cargo run --example scenario     # examples are the fastest way to test a subsystem
trunk serve                      # web build on :8080
cargo check && cargo fmt         # do this before committing
cargo run -p nova_assets --bin content -- gen   # regen base content (also: lint - refs + balance + input overlaps)
cargo run -p nova_probe -- run playable      # run-harness check: correctness+perf report
```

Do NOT run the full `cargo test` or `cargo clippy` locally unless asked: CI
(`.github/workflows/ci.yaml`) runs both on every PR and push to master, and
the local suite is slow. When you skip them, say so plainly. DO run the tests
you write or touch - with CI's feature set (workspace-wide, a unifying
sibling, or `--features serde`; `cargo test -p nova_scenario` alone does not
compile, see `crate-solo-tests-miss-unified-features` in LESSONS).

Features: `debug` gates all debug tooling; `dev` = `debug`. `--norender` and
`--debugdump` exist only under `debug`.

## Testing: harness-first

This is a game, so the strongest evidence is a harness that PLAYS it - an
App-driven test or a `BCS_AUTOPILOT`-scripted example - not a unit test of a
suspected mechanism.

- **Bugs: reproduce first.** The first artifact of a bug task is a failing
  harness test that replays the scenario where the bug happens. Then trace
  the mechanism with real numbers, then fix; the same rig becomes the
  regression pin (fail-first proof, numbers recorded in TASK.md).
- **Features ship with harness coverage** that exercises them the way a
  player does - `tests/gauntlet_course.rs` and `tests/ledger_ch2_encounter.rs`
  are the reference style, the examples the curriculum. They live in purpose
  dirs (`examples/sections|gameplay|ui|screenshots|perf/`, reading order =
  the `[[example]]` catalog in the root Cargo.toml), and each category smokes
  alone: `cargo test --test examples_smoke sections` (or `gameplay`, `ui`,
  `screenshots`). Unit tests prove the pieces; the harness proves the
  feature.
- **Rigs mirror production**: scheduling, spawn defaults, shipped
  configuration. Prefer extending an existing rig over writing a bespoke one.
- **Probe gameplay-touching changes.** The post-feature check is one command:
  `cargo run -p nova_probe -- run <example>` runs the autopilot example
  headless and produces `probe-runs/<example>/report.html` + `checks.json`
  (run timeline, continuous invariants, log scan, optional profiled pass and
  FPS deltas) with a provisional OK/WARN/FAIL the reviewer confirms. Use it
  in /work's verify step, for before/after evidence on bug and perf tasks,
  and read SKIPPED as "not measured", never "held". Full usage + SDLC
  wiring: the `probe` skill (`.claude/skills/probe/SKILL.md`); docs in the
  wiki's Performance section.

## How the app is assembled

`AppBuilder::new().with_game_plugins(...).build()` in
`crates/nova_core/src/lib.rs`. Order: Bevy defaults, enhanced input, assets,
gameplay, scenario, then (when no custom game plugins) the editor, the main
menu (`with_main_menu(bool)` overrides), and `debug`-gated tooling. avian3d
physics comes in via `NovaGameplayPlugin`. States:
`GameStates::{Loading, MainMenu, Playing}` and
`GameAssetsStates::{Loading, Processing, Loaded}`; scenario setup usually
hooks `OnEnter(GameAssetsStates::Loaded)`.

## Conventions

- Global rules from `~/AGENTS.md` apply: plain ASCII punctuation, plain commit
  messages with no AI attribution, no time-based technical arguments, and the
  shell/verification rules (no pipes eating exit codes, kill by PID).
- Bevy idioms: one plugin per subsystem, systems grouped in `SystemSet`s,
  subsystems talk via events (`nova_events`), not direct coupling.
- Generated content follows its builder: the base
  `assets/base/**/*.content.ron` are GENERATED from Rust builders via
  `content -- gen` and guarded by parity tests. Edit the BUILDER and
  regenerate in the same commit; never hand-edit the generated RON (not even
  comments).
- Worktrees come only from the sprout skill (used by /work and /flow). Never
  create one by hand. Otherwise work in the main checkout.
- Rustdoc (the code-level docs; the wiki owns concepts, task 20260525-133033):
  every crate opens with a crate-level `//!` paragraph - what it owns, its main
  plugin(s), how it relates to its neighbors - distilled from the architecture
  wiki, not duplicating it. Public items get a `///` line saying what and why,
  not how (the code shows how). Prefer intra-doc links (`` [`Type`] ``) for
  types in reach; link to the relevant wiki page for a concept that needs more
  than a paragraph rather than restating it. Runnable `# Examples` are NOT
  expected on every item - the `examples/` dir and the wiki are the worked
  examples. `#![warn(missing_docs)]` is enabled per crate AS IT COMES CLEAN
  (not workspace-wide): `nova_info` is the exemplar; a crate turns it on only
  once its whole public surface is documented (the per-crate push is
  20260525-133032). Keep `cargo doc --workspace --no-deps` warning-free.

## Shared-checkout discipline

Parallel sessions share the main checkout. These rules keep them from eating
each other's work:

- `git branch --show-current` before EVERY commit in the main checkout - an
  external checkout/reset can move HEAD under you.
- Stage explicit paths there, never `git add -A` (another session's or the
  user's files may be sitting in the tree); glance at `git status` for
  related generated files (Cargo.lock) so they are not dropped. Inside an
  isolated sprout worktree, `git add -A` is fine.
- Never leave the index staged-but-uncommitted across tool calls - a parallel
  `git add -A`/`git commit -a` sweeps it into its own commit. A squash-land
  is ONE atomic command:
  `pwd && git branch --show-current && git merge --squash <b> && git commit`.
- When parallel jobs may hold the tree, read repo facts via
  `git show HEAD:<path>`, not the working files.
- Background jobs: the Write/Edit guard blocks the main checkout but not a
  sprout worktree - author master-side artifacts (task stubs, ledger lines)
  via Bash heredoc, and do all code edits in the worktree.

## Development flow

/flow drives development here: work is planned into tatr tasks, implemented
in sprout worktrees, reviewed out-of-context in round 1, and retro'd via
/compound - the full plan/work/review/compound cycle. Definition of Done
items carry their proof notation (`test:`, `cmd:`, `manual:`). `LESSONS.md`
at the repo root is the lessons ledger - read it before starting any task.
`tatr check` (plus `tatr check --ledger LESSONS.md`) is the conformance gate
for tasks and the ledger.

## LESSONS.md: the repo's paid-for mistakes

`LESSONS.md` (repo root) is the compressed memory of every mistake this repo
has already paid for. Read its header and Pending promotions before starting
work; grep it mid-task for your area's crate/subsystem names - "has this
bitten before?" is answered there, one line per lesson. What is inside:

- process traps: sweeps (`sweep-then-delete`), landing discipline, docs sync;
- testing rules: `production-faithful-rigs`, `fail-first-regression-ab`,
  `would-it-fail-without-it`, pins at boundaries;
- Bevy/avian facts: `two-clocks`, observer semantics, message rigs,
  `verify-engine-guarantees-in-source`;
- content/modding facts: generated RON, stemmed extensions, feature
  unification, overlay semantics;
- measurement discipline: quiet hosts, isolated levers, screenshots.

## Where records go (/plan, /spike, /work, /review, /compound, /flow)

Everything tied to one task lives in that task's folder - never as loose
`.md` files under `docs/`:

- `tasks/<id>/TASK.md` - the task (tatr; body shape: Story / Steps /
  Definition of Done / Notes).
- `tasks/<id>/SPIKE.md` - the spike/research doc (/spike).
- `tasks/<id>/REVIEW.md` - review rounds and verdict (/review).
- `tasks/<id>/RETRO.md` - the retrospective (/compound).
- `tasks/<id>/NOTES.md` - design/fix record for the shipped change.

`docs/` is EPHEMERAL scratch (task 20260718-175424): write whatever notes you
like during a cycle, but at every release tag it is wiped to only
`docs/README.md` (which describes the model); the ledger /compound appends to
is `LESSONS.md` at the repo root. Distil anything durable out of scratch
FIRST - lessons into `LESSONS.md`, reference detail into the wiki - then run
`scripts/wipe-docs.sh`; the release-flow guard (`scripts/check-docs-clean.sh`)
fails a tag if scratch remains. Plans are tatr tasks, NOT `docs/plans` files
(retired). If a skill's default output path says `docs/retros/`, `docs/spikes/`
or `docs/plans/`, use the task folder (or a tatr task for a plan) instead.

## Docs, tasks, versioning

- Durable reference docs live in the wiki source under `web/src/wiki/dev/`
  (architecture, scenario-system, sections, development = build/web/release,
  modding-ron, mod-portal, and keeping-docs-in-sync = the map of what to
  update when). Edit them there; keep them accurate in the SAME task as the
  code change that invalidates them.
- Tasks: `tatr` CLI, markdown under `tasks/`. Check the backlog before
  starting, close tasks when done. Skills: /plan, /work, /review, /compound,
  /flow.
- Task tags encode scheduling - EVERY new tatr task carries exactly one of:
  `backlog` with priority 0 (not scheduled), or the current release tag (the
  active `vX.Y.Z`, e.g. `v0.8.0` - its `release`/`meta` tracker task holds the
  strand map) with a priority slotted RELATIVE to that release's open tasks
  (`tatr ls -f ':tags contains vX.Y.Z' --sort priority` first). Topical tags
  (`bug`, `scenario`, `ui`, ...) come on top. Pulling a backlog task into a
  release = swap the tag, re-slot the priority.
- Version lives in root `Cargo.toml` (`workspace.package.version`). Notable
  changes go to `CHANGELOG.md` (Keep a Changelog). Release steps:
  `web/src/wiki/dev/development.md`.

## The website (`web/`)

`web/` is the public site (landing, news, tutorial, wiki), deployed to GitHub
Pages with the game under `/play/`. It is hand-maintained content: when a code
change makes something on it wrong or missing, fix it in the SAME task. The
full map is the dev wiki page **Keeping docs in sync**
(`web/src/wiki/dev/keeping-docs-in-sync.md`). The short version:

- **Player-facing behavior changed** (controls, verbs, HUD, menus, a section,
  a weapon, a scenario): fix the affected `web/src/wiki/*.md` pages, plus
  `web/src/tutorial.html` if the first-flight flow moved.
- **Internals or a data format changed**: fix `web/src/wiki/dev/*.md` (a
  RON/bundle/catalog break lands in modding-ron/mod-portal the same task).
- **A release went out**: update `CHANGELOG.md` (terse, grouped by subsystem)
  and `web/src/news/` (one post per FEATURE release, h2 sections + h3
  subsections for the TOC; patch releases fold into the parent post's
  `## Point releases`).
- **New feature worth showing**: land a `.figure` placeholder slot + caption
  now, drop the capture in later.

Adding or renaming a page/post edits `web/webpack.config.js` (page list +
`historyApiFallback`), plus `web/src/wiki-pages.ts` for a wiki page or
`NEWS_POSTS` + a card in `web/src/news.html` for a news post. Verify with
`cd web && npm run ci`.
