# Review: Radial lock-acquisition ring HUD (UiMaterial shader)

- TASK: 20260717-004302
- BRANCH: feat/lock-dwell-ring-hud

## Round 1

- VERDICT: APPROVE

Reviewed the diff against master plus an independent out-of-context pass
(shared implementer/reviewer session). Load-bearing claims re-derived
independently and confirmed:

- **WGSL <-> Rust uniform layout**: `LockDwellRingUniform { color: LinearRgba,
  progress, inner, softness }` matches the WGSL `struct { vec4<f32>, f32, f32,
  f32 }` at `@group(1) @binding(0)` field-for-field. `LinearRgba` is a
  ShaderType of size/align 16 = `vec4<f32>`; the trailing three f32s pack into
  the slot after the vec4 (offsets 16/20/24), encase pads to 32. No std140 risk.
- **Shader math**: `atan2(p.x, -p.y)` with y-down UV yields top=0, right=0.25,
  bottom=0.5, left=0.75 - genuinely clockwise from 12 o'clock. Empty at
  progress=0, full at progress=1 (frac<1 always). Straight-alpha output matches
  bevy UI's non-premultiplied blending. Annulus band correct.
- **Driver / `is_dwelling` after-commit**: holding the same candidate after the
  dwell completes keeps `dwell_secs >= dwell_needed`, so `is_dwelling()` is false
  and the ring hides the instant the lock snaps. Sweeping to empty space resets
  `dwell_needed` to 0 (not left stale). Empty `q_player` clears the anchor
  without panic.
- **Example injection**: safe - the CTRL hold is not fired in the injection
  window, so `update_radar_search` hits `if !hold_fired { continue; }` and never
  clobbers the injected dwell fields; the `RadarState` is removed before the
  downstream kill/goto stages. The ~0.5 progress tolerance is generous (driver
  writes exactly 0.5).
- **Tests**: all 3 hud tests are non-vacuous (each fails if the driver were
  broken); full `nova_gameplay` lib suite 518 passed / 0 failed.

The example could not be run to completion locally (lavapipe software rendering
too slow); no shader-compile error at load and CI's `examples_smoke` covers the
e2e run - an honest, policy-consistent skip (recorded in TASK.md).

No BLOCKER/MAJOR/MINOR findings. NITs only, left to discretion:

- [ ] R1.1 (NIT) assets/shaders/lock_dwell_ring.wgsl - at `progress == 0` a
  measure-zero lit sliver can survive exactly at the top seam. Visually
  negligible; leave as-is.
  - Response: acknowledged, left (imperceptible; the ring is never parked at
    exactly 0 in practice - the dwell is charging whenever it is shown).
- [ ] R1.2 (NIT) examples/11_hud_range.rs - the injected `RadarState` leaves
  `acquired: false` via `..default()`; cosmetic, nothing consumes it in the
  non-fired window.
  - Response: acknowledged, left (removed promptly, no consumer).
- [ ] R1.3 (NIT) hud/lock_dwell_ring.rs tests - no driver-level test for a
  candidate-drop-to-None mid-gesture; the same `is_dwelling()==false` path is
  covered by the "no dwell" test and the example.
  - Response: acknowledged, left (redundant with existing coverage).
