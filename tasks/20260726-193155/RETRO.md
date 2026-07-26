# Retro: NOVA OS CRT scanline + grain realism pass

- TASK: 20260726-193155
- BRANCH: feat/nova-os-crt-scanline-grain
- REVIEW ROUNDS: 1 (APPROVE, out-of-context)

## What went well

- The spike (`20260726-193040`) had already scoped this precisely, so /flow went
  straight from understanding to a clean plan with no fork - the spike-first
  investment paid off exactly as intended.
- The capture rig built back in `20260726-180807` made the shader-visual gate a
  one-command check instead of a guess; verifying the render was cheap.
- Placing the new `resolution: vec2` uniform directly after the `tint: vec4`
  kept the encase/WGSL layout aligned with zero manual padding, which the
  out-of-context reviewer flagged as the one high-risk item and confirmed correct.

## What went wrong

- Two small self-inflicted compile/test stumbles: `Assets::get_mut` needs a `mut`
  binding, and `init_asset` panics under `MinimalPlugins` without `AssetPlugin`.
  Both caught immediately by the test run; cost was one extra iteration each.

## What to improve next time

- When adding a headless test that touches an asset type, add `AssetPlugin`
  alongside `MinimalPlugins` from the start (copy the sibling material test's
  plugin set) rather than reaching for `init_asset` on a bare MinimalPlugins app.

## Action items

- Added a shader-uniform layout lesson to LESSONS.md (x1): put a `vec2`/`vec3`
  right after a `vec4` (or pad) so the Rust `ShaderType`/encase layout matches
  the WGSL struct - a field-order/alignment mismatch silently corrupts the whole
  uniform.
- No follow-up code tasks. Siblings `20260726-193219` (casing/glass) and
  `20260726-193233` (RTT pipeline) remain in the backlog.
