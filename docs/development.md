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

- Workspace version lives in root `Cargo.toml` under `workspace.package.version`
  (currently `0.3.0`); crates inherit it via `version = { workspace = true }`.
- `nova_info::APP_VERSION` is set at build time from the `APP_VERSION` env var
  (wired through `build.rs`).
- Update `CHANGELOG.md` (Keep a Changelog format, entries attributed like
  `@alexjercan ...`) for notable changes.
- Native packaging assets (Windows icon/installer, macOS `.app`/iconset) are under
  `build/`; the release workflow is `.github/workflows/release.yaml`.

## Task tracking

Work items are tracked with the `tatr` CLI as markdown files under `tasks/`. Check the
backlog before starting and close tasks when done. The plan-work-review-retro loop is
available via the `/plan`, `/work`, `/review`, and `/compound` skills. Retros land in
`docs/retros/`, spikes in `docs/spikes/`.
