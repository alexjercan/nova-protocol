# Notes: NOVA OS CRT scanline + grain realism pass

- TASK: 20260726-193155

## What changed

Shader-only pass on `assets/shaders/nova_os_crt.wgsl` plus the small uniform +
feed wiring in `crates/nova_gameplay/src/hud/drawer.rs`. No architecture change.

- **Resolution-aware soft scanlines.** Added a `resolution: vec2` uniform to
  `NovaOsCrtMaterial`/`NovaOsCrtUniform`, fed each frame from the CRT overlay
  node's `ComputedNode.size` in `animate_nova_os_crt` (which already runs while
  the computer is open and stamps `time`). The scanlines are now a SOFT cosine
  trough every ~3 device pixels (`uv.y * res_y / 3 * 2pi`), replacing the hard
  `fract(uv.y * 240.0) < 0.5` step. Soft profile + real-pixel spacing means the
  line count tracks the panel size and never aliases/moires on resize. Falls back
  to a fixed 720/1280 density before the first layout pass feeds `resolution`.
- **Vertical slot-mask.** A whisper of aperture-grille (`SLOT_STRENGTH = 0.03`),
  a cosine stripe on `uv.x * res_x / 3`, for phosphor-stripe texture.
- **Analog grain shimmer.** The fine grain layer is now INTERPOLATED between its
  ~9/s reseeds (`mix(fine_lo, fine_hi, fract(time*9))`) instead of the hard
  `floor(time*9)` step, so the movement is smooth, not steppy. The coarse layer
  stays static as a stable structure underneath.
- **Edge-weighted grain.** Grain (and sparks) are scaled by
  `mix(0.55, 1.3, smoothstep(0.1, 0.9, dist))` so they are quiet in the bright
  centre where the text sits and stronger toward the vignetted edges. Since a UI
  material cannot sample the glyphs behind it, this weights by screen POSITION,
  not text luminance (recorded as the task Assumption).

## Uniform layout

`NovaOsCrtUniform` field order is `tint (vec4), resolution (vec2), scanline,
vignette, glow, grain, time`. `resolution` sits right after the vec4 so encase's
8-byte vec2 alignment matches the WGSL struct with no manual padding; the WGSL
`NovaOsCrtMaterial` mirrors the same order.

## Tests

- `nova_os_crt_material_receives_resolution_and_time`: spawns an overlay with a
  `MaterialNode` + a `ComputedNode { size: 800x600 }`, runs `animate_nova_os_crt`,
  and asserts `material.data.resolution == 800x600` and `time` is stamped.
- `drawer_uses_crt_material_overlay_when_assets_are_available`: extended to assert
  `resolution` starts `Vec2::ZERO` before layout.

## Verification

- `nix develop --command cargo test -p nova_gameplay drawer` (47 passed)
- `nix develop --command cargo fmt --check`
- `nix develop --command cargo check`
- DoD greps: no `fract(uv.y * 240.0)`, no `floor(material.time * 9.0)`, shader
  references `resolution`.
- Captured `shots/nova-os-{welcome,active}.png` with `screenshot_nova_os`: soft
  scanlines, fine green grain, text still crisp. (Shimmer + resolution-scaling
  are motion/size effects a still cannot show; the shader validated on the GPU.)

## Difficulties

- `init_asset::<NovaOsCrtMaterial>()` panics under `MinimalPlugins` alone; the
  wiring test needs `AssetPlugin` added too (matches the sibling material test).
- `Assets::get_mut` returns a guard that must be bound `mut` to write through it.

## Self-reflection

Cheap, self-contained, screenshot-verified. The resolution feed is the one piece
of real wiring; everything else is shader math, so the automated proofs are the
wiring test + greps and the real judge is the capture - appropriate for a
shader-visual pass, and honestly scoped in the DoD rather than pretending a
headless test proves the look.
