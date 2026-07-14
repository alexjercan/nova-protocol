# NOTES: Switch web build to bevy/webgpu and un-gate hanabi on wasm

## What shipped

The web build moved from the default WebGL2 render backend to WebGPU, and the
three `#[cfg(not(target_family = "wasm"))]` gates that disabled `bevy_hanabi`
particle effects on wasm were removed. Native was already running these effects;
now the same code runs on the web build too.

## The backend switch mechanism

Render backend is selected entirely by bevy cargo features - `nova_core` adds a
plain `DefaultPlugins` with no `WgpuSettings`/`RenderPlugin` override, and bevy's
default features include `webgl2`, which is why the web build shipped WebGL2.

`bevy_hanabi` particles run on compute shaders, which on wasm exist only under
WebGPU (never WebGL2). The switch is a single additive, wasm-only feature in
`crates/nova_core/Cargo.toml`:

```toml
[target.'cfg(all(target_family = "wasm", any(target_os = "unknown", target_os = "none")))'.dependencies]
bevy = { version = "0.19.0", features = ["webgpu"] }
```

bevy's `webgpu` feature overrides `webgl2` when both are enabled (verified: bevy
examples README + bevy.org/news/bevy-webgpu, and `tasks/20260714-085955/SPIKE.md`),
so this needs no change to the trunk invocation (`trunk build` / `--release`, no
`--features`) and no disabling of default features. Feature unification carries it
to the whole wasm build. Confirmed with `cargo tree`:

- wasm target: bevy has both `webgl2` and `webgpu` -> WebGPU wins.
- native target: bevy has `webgl2` (a default, inert on native) but NOT `webgpu`.
  The block is wasm-scoped, so native is untouched.

## What is actually a hanabi effect (correction to the spike)

The spike listed the "thruster plume" as one of the gated effects. It is not: the
thruster exhaust is a custom shader (`ThrusterExhaustConfig`, "exhaust shader" in
`thruster_section.rs`), not a hanabi particle system, and already rendered on the
WebGL2 web build. The hanabi (compute) effects that were gated and are now enabled
on wasm are:

- turret muzzle flash (`insert_turret_barrel_muzzle_effect`)
- projectile trail (`on_projectile_marker_effect`)
- torpedo detonation burst (`insert_particle_effect`)
- torpedo launch burst (`insert_torpedo_spawner_effect` / `on_torpedo_launch_effect`)

The torpedo blast-radius sphere (`BlastRadiusVisual`) stays a plain mesh - it shows
the AoE radius, a different thing from the spray, and stays visible regardless.

## WebGL2 shims kept, not removed

The `#[cfg(target_arch = "wasm32")]` std140 padding fields (`hud/velocity.rs`,
`sections/thruster_section.rs`) and the empty-`view_formats` guard in
`hud/target_inset.rs` were WebGL2 workarounds. They are kept: both stay valid under
WebGPU (extra uniform padding is harmless; empty view_formats is legal on WebGPU
too), and removing them is unrelated churn that would only matter if the web build
ever went back to WebGL2 (the Option C door the paired gate task leaves open).

## Verification

- `cargo check --workspace --all-targets` (native): clean (1m 46s).
- `trunk build` (wasm, debug): clean, `bevy_hanabi` and the un-gated `nova_gameplay`
  observers compile for wasm32, trunk `✅ success` (5m 06s). This is the ONLY
  compile gate for the un-gated wasm code: CI (`ci.yaml`) builds native only, and
  `deploy-page.yaml` is `workflow_dispatch`, so neither compiles this path.
- `cargo fmt --check`: clean.

NOT done here (honest gap): the runtime visual confirmation that the effects render
in a live WebGPU browser. This environment is headless (no WebGPU browser), so it
was not eyeballed. Risk is low - these are the exact effects already verified
rendering on native, and they now compile for wasm under the backend hanabi
requires - but "compiles + correct backend" is not "seen rendering". The paired
gate task (20260714-233443) runs `scripts/preview-web.sh` and is the natural place
to eyeball particles at the real `/play/` path in a WebGPU browser.

## Lessons applied

- `verify-ci-triggers-before-claiming-coverage`: did the real `trunk build` instead
  of trusting a green native CI that never compiles wasm.
- `stage-lock-with-manifest`: the Cargo.toml dep change bumped `Cargo.lock`; staged
  it too.
- `sweep-then-delete`: grepped the repo for the FIXME id and "wasm-blocked"/"not on
  wasm" prose and updated the stale comments in `render.rs`, `juice.rs`, and
  `docs/architecture.md`, not just the gate lines.
