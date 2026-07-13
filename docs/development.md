# Development

## Toolchain

- **Rust nightly** is required (`rust-toolchain.toml` pins `channel = "nightly"` with
  `rustfmt` + `clippy`). Bevy 0.19 and some features rely on nightly.
- **NixOS**: run `nix develop` to enter the dev shell (`flake.nix`). It provides the
  nightly toolchain with the `wasm32-unknown-unknown` target plus all the system libs
  Bevy needs (udev, alsa, vulkan-loader, X11, wayland, libxkbcommon) and `trunk` /
  `wasm-pack` for web builds. Without Nix, install those libraries yourself.

## Everyday commands

```sh
cargo run                         # run the game (spaceship editor scene)
cargo run --features dev          # dev build: turns on all debug tooling
cargo run --example 03_scenario   # run one of the examples/ end-to-end demos
cargo build --release             # release profile: opt="s", lto, codegen-units=1, strip
cargo clippy --all-targets        # lint (workspace clippy config in root Cargo.toml)
cargo fmt                         # format (rustfmt.toml at repo root)
cargo test                        # tests
```

The dev profile turns on `opt-level = 1` for our code and `opt-level = 3` for all
dependencies (including Bevy), so the first build is slow but iteration is fast.

Two habits that keep the check suite honest and fast in this repo:

- Run **`cargo test --workspace`**, not bare `cargo test`. This is a
  root-package-plus-members workspace and the unit tests live in the member
  crates; bare `cargo test` only runs the root package and gives false comfort.
- For a headless example run, **build cold first, then time only the run**
  (`cargo build --example X --features debug`, then `BCS_AUTOPILOT=1 timeout N
  cargo run --example X ...`). Wrapping a cold `cargo run` in a run-sized timeout
  burns the whole window on the build and never executes.

### Building from a sprout worktree

A fresh `sprout` worktree has its own empty `target/`, so a build there is a full
cold Bevy compile (minutes, and tens of GB of disk). The tempting shortcut of
pointing `CARGO_TARGET_DIR` at the main checkout's warm cache is **not safe for
the workspace's own crates**: both checkouts are the same packages at the same
versions, so when the sources diverge, builds from the two checkouts overwrite
each other's artifacts and a worktree binary can silently link the main
checkout's version of `nova_gameplay` (observed in task 20260709-131502: the
worktree's torpedo-range smoke ran master's gameplay code, faking a bug in the
branch, and later failed to resolve a symbol that existed only in the worktree).

If you want to reuse the warm cache, it is only trustworthy while the two
checkouts' workspace crates are identical (a freshly cut worktree), and never
after building from the other checkout in between. When in doubt, build in the
worktree's own `target/` and accept the one-time cold compile; the third-party
dependency artifacts are the bulk of it and sccache-style sharing of those is a
separate problem.

The dev profile also sets `split-debuginfo = "unpacked"` and `debug = "line-tables-only"`.
This is a **memory** knob, not a speed one: `cargo test` / `cargo build --all-targets`
links one Bevy+avian binary per target (the lib unittest, every example, and doctests).
With embedded DWARF each binary is ~1.5 GB, and cargo links several in parallel, so peak
toolchain RAM approached ~40 GB and swap-thrashed a 32 GB machine. Leaving DWARF in the
`.o` files and keeping only line tables drops the peak to ~20 GB while preserving panic
backtraces (so test failures still point at source lines). If you need full
local-variable debugging under a debugger, temporarily set `debug = true`. See
`../bevy-common-systems/docs/2026-07-03-test-memory.md` for the original diagnosis.

## Features

Defined in the root `Cargo.toml`:

- `debug` - enables the whole `nova_debug` plugin (egui inspector, wireframe, section
  overlays) and `bevy/track_location`. Propagates to the sub-crates'
  `debug` features.
- `dev` - alias that enables `debug`. Use `--features dev` while developing.

Debug-only CLI flags (only compiled under `debug`, see `src/main.rs`):

- `--norender` - build the app without rendering (headless-ish; toggles
  `with_rendering(false)`).
- `--debugdump` - dump the Bevy system schedule as a graph and exit. There is a
  checked-in `dump.dot` / `dump.svg` produced this way.

## Examples

`examples/` are the fastest way to exercise a single subsystem end to end (this repo
prefers runnable examples over isolated unit tests):

| Example | Exercises |
|---------|-----------|
| `01_scene`            | basic scene setup |
| `02_thruster_shader`  | thruster exhaust shader |
| `03_scenario`         | loading a scenario / the modding pipeline |
| `04_asteroids`        | procedural asteroids + destruction |
| `05_directional`      | directional sphere shader |
| `07b_slicer`          | mesh slicing / fragmentation |

When adding a substantial feature, consider adding an example that drives it.

## Web build

The game targets WASM via **Trunk** (`Trunk.toml`, `index.html`):

```sh
trunk serve        # build for wasm32 and serve on http://localhost:8080
trunk build --release
```

`.cargo/config.toml` sets `--cfg=web_sys_unstable_apis` for the wasm target, and
`bevy_rand` picks up the `wasm_js` feature under wasm. Trunk uses the `release`
profile (it does not support custom cargo profiles). The GitHub Pages deploy is in
`.github/workflows/deploy-page.yaml`.

## Versioning and release

- Workspace version lives in root `Cargo.toml` under `workspace.package.version`;
  crates inherit it via `version = { workspace = true }`.
- `nova_info::APP_VERSION` is set at build time from the `APP_VERSION` env var
  (wired through `build.rs`).
- Native packaging assets (Windows icon/installer, macOS `.app`/iconset) are under
  `build/`; the release workflow is `.github/workflows/release.yaml`.

### Cutting a release

Releases are driven entirely by git tags. Pushing a tag matching `v[0-9]+.[0-9]+.[0-9]+*`
triggers the `release-flow` workflow (`.github/workflows/release.yaml`), which builds and
publishes the platform artifacts. Steps, done on `master`:

1. Bump `workspace.package.version` in root `Cargo.toml` (e.g. `0.4.1` -> `0.5.0`).
2. Refresh `Cargo.lock` so the workspace crates pick up the new version:
   `cargo metadata --format-version 1 >/dev/null` (or any build).
3. Update `CHANGELOG.md` (Keep a Changelog format, entries attributed like
   `@alexjercan ...`):
   - promote `[Unreleased]` to `[<version>] - <YYYY-MM-DD>` and leave a fresh empty
     `## [Unreleased]` heading on top;
   - merge any duplicate section headings that accumulated under `[Unreleased]`
     during the cycle (a long cycle tends to grow a second `### Changed`);
   - update the compare links at the bottom of the file: repoint `[unreleased]` to
     `compare/v<version>...HEAD` and add a `[<version>]` compare line below it.
4. Commit exactly those three files as `chore(release): vX.Y.Z`:
   `git add Cargo.toml Cargo.lock CHANGELOG.md && git commit -m "chore(release): vX.Y.Z"`.
5. Tag it: `git tag vX.Y.Z` (the tag name is what CI reads for the release name).
6. Push both: `git push origin master && git push origin vX.Y.Z`.
7. Watch the run until the assets land: `gh run watch` (or
   `gh run list --workflow=release.yaml`), then check the GitHub release page.

CI then runs four jobs off the tagged commit and uploads the assets to a GitHub release
named after the tag:

- `build-macOS` - universal (`aarch64` + `x86_64`) `.dmg`.
- `build-linux` - `.tar.gz` (binary + `assets/` + `credits/`).
- `build-windows` - `.zip` (`.exe` + `assets/` + `credits/`).
- `build-web` - wasm build via `trunk`, wasm-opt'd, zipped.

The workflow can also be run manually via `workflow_dispatch` with a `version` input
(form `v1.2.3`) if you need to re-run packaging without pushing a new tag.

## Task tracking

Work items are tracked with the `tatr` CLI as markdown files under `tasks/`. Check the
backlog before starting and close tasks when done. The plan-work-review-retro loop is
available via the `/plan`, `/work`, `/review`, and `/compound` skills. Retros land in
`docs/retros/`, spikes in `docs/spikes/`.
