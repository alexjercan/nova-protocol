# Switch web build to bevy/webgpu and un-gate hanabi on wasm

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,wasm,polish

Spike: tasks/20260714-085955/SPIKE.md (Option A chosen)

Goal: run the same `bevy_hanabi` particle effects on the web build that native
already runs, by moving the web build from WebGL2 to the WebGPU backend and
removing the wasm gates that currently `#[cfg]` hanabi off.

Hanabi needs compute shaders, which on wasm exist only under WebGPU - see the
spike. The web build currently ships WebGL2 solely because that is bevy's default
feature (there is no explicit `WgpuSettings`/`RenderPlugin` override in
`nova_core`). So the work is: enable `bevy/webgpu` for the web build, then delete
the three FIXME(20260706-162908) gates.

Direction (leave the Steps for /plan):
- Add a `webgpu` cargo feature that turns on `bevy/webgpu`, and wire the trunk
  build to enable it (bevy prefers webgpu when both webgl2+webgpu are on).
- Remove the `#[cfg(not(target_family = "wasm"))]` hanabi gates:
  - `crates/nova_gameplay/src/plugin.rs:51` (`HanabiPlugin`)
  - `crates/nova_gameplay/src/sections/turret_section.rs:323,327` (muzzle + projectile)
  - `crates/nova_gameplay/src/sections/torpedo_section/mod.rs:320-328` (launch + detonation)
- Verify effects (thruster plume, muzzle flash, torpedo launch/detonation) render
  in a WebGPU browser via `scripts/preview-web.sh`.
- Decide whether the WebGL2-only workarounds still belong: std140 padding fields
  (`hud/velocity.rs`, `sections/thruster_section.rs`) and the `target_inset.rs`
  `view_formats` guard become unnecessary under WebGPU but are harmless; removing
  is optional cleanup, not required.

Depends on / pairs with 20260714-233443 (the WebGPU-detection gate), which must
ship together so non-WebGPU browsers get a friendly message instead of a dead
canvas (a webgpu build fails to init the renderer on a browser without WebGPU).

Note: hanabi's `serde` feature is wasm-incompatible (typetag), but nova does not
serialize effects, so this is unaffected.
