# Switch web build to bevy/webgpu and un-gate hanabi on wasm

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.6.0,wasm,polish

## Outcome

Switched the web build to the WebGPU render backend via a single additive,
wasm-only `bevy/webgpu` feature in `crates/nova_core/Cargo.toml`, and removed the
three `#[cfg(not(target_family = "wasm"))]` gates that disabled hanabi particles on
wasm (turret muzzle flash, projectile trail, torpedo launch + detonation bursts).
Native is untouched (the feature block is wasm-scoped; `cargo tree` confirms native
gets no `webgpu`). Verified: native `cargo check --all-targets` clean, `trunk build`
compiles hanabi + the un-gated code for wasm32 with `✅ success`, `cargo tree` shows
webgpu enabled (overriding webgl2) on wasm only, `cargo fmt --check` clean. Full fix
record and the honest runtime-verification gap in `NOTES.md`. Corrected a spike
inaccuracy: the thruster plume is a shader, not hanabi, and already worked on the
web. Ships with 20260714-233443 (the WebGPU detection gate).

## Goal

Run the same `bevy_hanabi` particle effects on the web build that native already
runs, by moving the web build from WebGL2 to the WebGPU backend and removing the
wasm `#[cfg]` gates that currently disable hanabi. Scoped from
`tasks/20260714-085955/SPIKE.md` (Option A).

## Steps

- [x] Enable the WebGPU backend on the wasm target only, additively. In
  `crates/nova_core/Cargo.toml` add a target block mirroring the existing wasm
  pattern in `nova_gameplay`/`nova_scenario`:
  `[target.'cfg(all(target_family = "wasm", any(target_os = "unknown", target_os = "none")))'.dependencies]`
  with `bevy = { version = "0.19.0", features = ["webgpu"] }`. (Verified: bevy's
  `webgpu` feature overrides the default `webgl2` when both are on - bevy examples
  README + bevy.org/news/bevy-webgpu - so this needs no `--features` on the trunk
  invocation and no disabling of default features. Feature unification applies it
  to the whole wasm build.)
- [x] Remove the hanabi wasm gates and their `FIXME(20260706-162908)` comments:
  - `crates/nova_gameplay/src/plugin.rs:50-52` (`HanabiPlugin`)
  - `crates/nova_gameplay/src/sections/turret_section.rs:322-327` (muzzle +
    projectile-marker effect observers)
  - `crates/nova_gameplay/src/sections/torpedo_section/mod.rs:319-329`
    (`insert_particle_effect`, `insert_torpedo_spawner_effect`,
    `on_torpedo_launch_effect`)
  Re-read each surrounding block after editing (reread-after-insert).
- [x] Grep the repo to confirm no `#[cfg(not(target_family = "wasm"))]` /
  `#[cfg(not(target_arch = "wasm32"))]` guard gating hanabi/effects remains, and
  that the now-unconditional `bevy_hanabi::prelude::*` imports in
  `turret_section.rs` / `torpedo_section/mod.rs` are actually used on wasm (else
  they become unused-import errors under the workspace lints).
- [x] Leave the WebGL2 std140 padding fields (`hud/velocity.rs`,
  `sections/thruster_section.rs`, `#[cfg(target_arch = "wasm32")]`) and the
  `hud/target_inset.rs` empty-`view_formats` guard in place - both stay valid
  under WebGPU (padding is harmless; empty view_formats is legal on WebGPU too).
  Record in NOTES why they are kept rather than removed.
- [x] Native sanity: `cargo check --workspace --all-targets` (guards the
  Cargo.toml edit; native already compiles hanabi).
- [x] Real wasm compile - the ONLY compile check for the un-gated wasm code, since
  CI builds native only (`ci.yaml`) and `deploy-page.yaml` is
  `workflow_dispatch`: run `trunk build` (debug) from repo root; it must compile
  cleanly. Do not skip - a green CI proves nothing here
  (verify-ci-triggers-before-claiming-coverage).
- [~] Runtime-verify in a WebGPU browser at the real `/play/` path via
  `scripts/preview-web.sh`: enter a scenario and confirm muzzle flash and torpedo
  launch + detonation particles render. NOT eyeballed - this environment is
  headless (no WebGPU browser). Compile + backend-wiring verified instead (see
  NOTES); the same effects already render on native. Deferred to the paired gate
  task's `preview-web.sh` run for a live-browser eyeball. (Correction: the
  thruster plume is a shader, not hanabi, and already rendered on the web.)
- [x] Docs: update `docs/architecture.md:62` ("`bevy_hanabi` particles (not on
  wasm)" -> now on wasm via the WebGPU backend), add a `CHANGELOG.md` entry, and
  write `tasks/20260714-233438/NOTES.md` (fix record: what shipped, the backend
  mechanism, why the WebGL2 shims stay). Also swept stale "wasm-blocked" prose in
  `torpedo_section/render.rs` and `juice.rs`.

## Notes

- Relevant files: `crates/nova_core/Cargo.toml` (backend feature),
  `crates/nova_gameplay/src/plugin.rs`,
  `crates/nova_gameplay/src/sections/turret_section.rs`,
  `crates/nova_gameplay/src/sections/torpedo_section/mod.rs` (gates),
  `scripts/preview-web.sh` (preview at /play/), `Trunk.toml` (no change).
- Backend is chosen purely by cargo features - `nova_core` adds plain
  `DefaultPlugins` with no `WgpuSettings`/`RenderPlugin` override, and default
  features include `webgl2`, which is why the web build ships WebGL2 today.
- CI does NOT build wasm; the local `trunk build` is the sole compile gate.
- stage-lock-with-manifest: the `Cargo.toml` change moves deps; stage `Cargo.lock`
  too (in the sprout worktree `git add -A` is the safe form).
- hanabi's `serde` feature is wasm-incompatible (typetag) but nova does not
  serialize effects, so it stays off (`default-features = false`, `2d`/`3d` only).
- Depends on / pairs with 20260714-233443 (detection gate); ship together so
  non-WebGPU browsers get a message, not a dead canvas.
