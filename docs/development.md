# Development

## Toolchain

- **Rust nightly**, pinned by `rust-toolchain.toml` (with rustfmt + clippy).
- **NixOS**: `nix develop` gives the toolchain, the `wasm32-unknown-unknown`
  target, all system libs Bevy needs (udev, alsa, vulkan, X11/wayland), and
  `trunk`. Without Nix, install those yourself.

## Everyday commands

```sh
cargo run                         # the game (boots into the main menu)
cargo run --features dev          # + debug tooling (inspector, wireframe)
cargo run --example 03_scenario   # run an example
cargo build --release             # release profile: opt=s, lto, stripped
cargo check && cargo fmt          # before committing
cargo test --workspace            # full suite (CI runs this; skip locally unless asked)
```

Notes that keep the suite honest and fast:

- Use `cargo test --workspace`, never bare `cargo test`: unit tests live in the
  member crates, so the bare form runs almost nothing and gives false comfort.
- `cargo test` takes ONE filter and one `-p` per invocation; separate runs for
  separate filters or packages.
- For a timed headless example run, build first, then time only the run
  (`cargo build --example X --features debug`, then `BCS_AUTOPILOT=1 timeout N
  cargo run --example X ...`). A cold build inside the timeout burns the window.
- Struct-field changes: `cargo check --workspace --all-targets`, or examples and
  tests stay silently broken.

The dev profile uses `opt-level = 1` for our code, `3` for dependencies: slow
first build, fast iteration. `split-debuginfo = "unpacked"` +
`debug = "line-tables-only"` keep link-time RAM around 20 GB instead of 40
(one Bevy-sized binary per test/example target); set `debug = true` temporarily
if you need a debugger. Diagnosis:
`../bevy-common-systems/docs/2026-07-03-test-memory.md`.

**Worktree builds**: a fresh sprout worktree has an empty `target/`, so the
first build is a cold Bevy compile. Do NOT point `CARGO_TARGET_DIR` at the main
checkout's cache: both checkouts hold the same crates at the same versions, so
their artifacts overwrite each other and a worktree binary can silently link
the main checkout's code (observed in task 20260709-131502). Accept the cold
build.

## Features

- `debug` - the whole `nova_debug` plugin (inspector, wireframe, overlays) plus
  `bevy/track_location`.
- `dev` - alias for `debug`.

Debug-only CLI flags (`src/main.rs`): `--norender` (no rendering) and
`--debugdump` (write the system schedule graph and exit).

## Examples

`examples/` exercises one subsystem each, end to end; this repo prefers
runnable examples over isolated unit tests. Current set: `01_scene`,
`02_thruster_shader`, `03_scenario`, `04_asteroids`, `05_directional`,
`06_torpedo_range`, `07_torpedo_guidance`, `07b_slicer`, `08_turret_range`,
`09_editor`, `10_gameplay`, `11_com_range`, `12_hud_range`,
`13_menu_newgame`. When adding a substantial feature, consider adding an
example that drives it.

## Web build

WASM via **Trunk** (`Trunk.toml`, `index.html`):

```sh
trunk serve            # serve on http://localhost:8080
trunk build --release
```

`.cargo/config.toml` sets `--cfg=web_sys_unstable_apis` for wasm; `bevy_rand`
uses its `wasm_js` feature there. Trunk only supports the `release` profile.
The GitHub Pages deploy (`.github/workflows/deploy-page.yaml`) builds the
landing site (`web/`) at the root and the game under `/play/`.

## Versioning and release

- Version: `workspace.package.version` in root `Cargo.toml`; crates inherit it.
- `nova_info::APP_VERSION` comes from the `APP_VERSION` env var via `build.rs`.
- Packaging assets (icons, installer, .app) live under `build/`.

### Cutting a release

Pushing a tag `v[0-9]+.[0-9]+.[0-9]+*` triggers `release-flow`
(`.github/workflows/release.yaml`). Steps, on `master`:

1. Bump `workspace.package.version` in root `Cargo.toml`.
2. Refresh `Cargo.lock`: `cargo metadata --format-version 1 >/dev/null`.
3. Update `CHANGELOG.md` (Keep a Changelog, one concise line per entry):
   promote `[Unreleased]` to `[<version>] - <YYYY-MM-DD>`, leave a fresh empty
   `## [Unreleased]` on top, merge any duplicate section headings that grew
   during the cycle, and update the compare links at the bottom (repoint
   `[unreleased]`, add the new `[<version>]` line).
4. Commit exactly those three files:
   `git add Cargo.toml Cargo.lock CHANGELOG.md && git commit -m "chore(release): vX.Y.Z"`.
5. `git tag vX.Y.Z` (CI reads the tag for the release name).
6. `git push origin master && git push origin vX.Y.Z`.
7. Watch the run (`gh run watch`), then check the GitHub release page and
   consider adding summarized release notes (`gh release edit vX.Y.Z --notes-file ...`).
8. Write a devblog for the release cycle (see below) and land it in `web/`.

The workflow uploads four assets to a release named after the tag: macOS
universal `.dmg`, Linux `.tar.gz`, Windows `.zip`, and a wasm-opt'd web zip.
It can also be re-run via `workflow_dispatch` with a `version` input.

### Writing the release devblog

Every release cycle also gets a devblog on the site, in `web/`. The devlogs
are numbered (`#1`..`#N`) and track the minor versions: one devlog per minor
release (`v0.4.0` -> Devlog #4, `v0.5.0` -> Devlog #5), with the cycle's patch
releases folded into that same post as a short closing note rather than getting
their own devlog. Source the content straight from the `CHANGELOG.md` sections
for the versions in the cycle.

Adding a devblog touches four places (mirror an existing post such as
`devlog-3-zones-torpedoes-and-blast-damage`):

1. Write the post at `web/src/posts/<slug>.html`. Copy an existing devlog for
   the structure: the `prose__meta` line carries `<date> // v<X.Y.0>`, the
   `<title>`/`<meta name="description">` summarize the release, and there is a
   `<!-- Devlog video ... -->` placeholder for the recorded footage.
2. Register the page in `web/webpack.config.js`: add a `page("post", ...)`
   entry in `plugins` and a matching `historyApiFallback` rewrite (keep both
   lists newest-first, above the previous devlog).
3. Add a card to `web/src/blog.html` at the top of `.post-list` (newest first),
   with the date/version, title, and a one-line excerpt.
4. Rebuild and check it: `cd web && npm run ci` (format check, lint, build).

## Task tracking

Work items: `tatr` CLI, markdown under `tasks/`. Check the backlog before
starting, close tasks when done. The plan-work-review-retro loop is the
`/plan`, `/work`, `/review`, `/compound` skills (plus `/flow` for the whole
cycle). All task-scoped records live in the task's folder: `SPIKE.md`,
`REVIEW.md`, `RETRO.md`, `NOTES.md` next to its `TASK.md`. Only multi-task
plans (`docs/plans/`) and the lessons ledger (`docs/LESSONS.md`) live
under `docs/`.
