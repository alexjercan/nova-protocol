# NOVA OS clicks: the forwarded pointer vs the CRT picture

Fix record for task 20260730-123039.

## The bug

`forward_nova_os_pointer` places a custom `PointerId` on the NOVA OS's offscreen
image so the terminal stays interactive through the CRT composite. To do that it
has to answer one question: **which image texel is the CRT drawing under the
cursor?**

The shader answers it like this (`assets/shaders/nova_os_crt.wgsl`):

```wgsl
let warped_raw = barrel(shaken_uv, material.warp);   // c * (1 + a*r^2)
let warped     = (warped_raw - 0.5) * OVERSCAN + 0.5; // OVERSCAN = 0.93
base           = textureSample(source_texture, source_sampler, warped);
```

The pointer answered it like this:

```rust
fn nova_os_inverse_barrel(uv: Vec2, amount: f32) -> Vec2 {
    let c = uv - Vec2::splat(0.5);
    Vec2::splat(0.5) + c / (1.0 + amount * c.length_squared())
}
```

Two independent errors, compounding:

1. **Wrong direction.** The shader's chain already maps *screen* uv to *image*
   uv - it is the forward map, and the pointer needed exactly it. The pointer
   applied the barrel's INVERSE instead, so the error is roughly twice the warp
   rather than zero.
2. **The overscan was invisible to it.** `NOVA_OS_OVERSCAN = 0.93` lived as a
   WGSL-local `const`. Nothing on the Rust side could see it, so the pointer
   never applied it. This term is close to LINEAR in distance from centre, which
   is why the miss did not stay confined to the corners.

## The measurement

Grid of screen uv through both maps, on a 1280x720 screen (the math rig,
`nova_os_pointer_mapping_matches_the_crt_shader_across_the_screen`):

| screen uv | miss (px) |
| --- | --- |
| centre (0.5, 0.5) | 0 |
| (0.4, 0.4) - a tenth out | ~8 x |
| corner (0, 0) | **27.1 x, 15.3 y** |

The blips are 12 px. So a corner contact's dot was more than two dot-widths from
where the click landed, and even quite near the middle the miss ate most of the
dot. The live-tree rig measured the same thing end to end: aiming at a corner
contact put the forwarded pointer on image px (79.1, 51.3) while the dot sat at
(65.2, 43.7).

**This is the owner's report exactly.** The `ship` app's blips cluster around a
schematic near screen centre, where the residual is small; the `map` app's
contacts spread over the whole viewport including the corners. Same plumbing,
very different hit rates, because the error is a function of radius.

## The fix

`nova_os_crt_screen_to_image_uv(uv, warp, overscan, power)` now mirrors the
shader's whole sample-UV chain: raster-collapse remap, barrel, overscan. It is
exact, not an approximation, because it is the same map rather than an attempted
inverse of one. It returns `Option`, so the tube-black margin outside a
collapsing raster is un-clickable the way it is un-drawn.

To keep the two from drifting apart again, **the overscan moved out of WGSL into
the material uniform** this crate fills, joining `warp`. Three tests hold that
line:

- the mapping agrees with an independent transcription of the WGSL across a
  17x17 grid, budget 0.5 px;
- the shader source still reads `material.warp` / `material.overscan` and carries
  no `const NOVA_OS_OVERSCAN`, and its power-collapse literals still match the
  Rust constants;
- the default material publishes `NOVA_OS_CRT_WARP` / `NOVA_OS_CRT_OVERSCAN`.

The pointer also takes `power` from `NovaOsOpenness` now - the same value
`animate_nova_os_crt` feeds the shader - so a half-collapsed raster is clicked
where it is actually drawn.

## The second half: the hit targets

Hypothesis 2 in the task said the map's bare `Text` label was not a hit target at
all. **That is false**, and worth recording: bevy 0.19's `ui_picking` hit-tests a
text node's whole box and falls back to the node entity when the cursor is not on
a glyph section (`picking_backend.rs`, the `.or_else` after
`pick_ui_text_section`). Clicking the middle of the bare label always worked.

What was real is the GEOMETRY around it. Measured on the live tree:

- the map label started 6 px clear of its dot - a dead band that selected
  nothing - and its box was tight to the glyph run on every side;
- the ship app, held up as the thing that works, carried the same defect at 4 px.

Both offsets were absolute `left` values that overlooked that an absolutely
positioned child's `left` is measured from its parent's PADDING edge, i.e.
already inside the dot's 2 px border. Both are now
`<BLIP>_PX - <BLIP>_BORDER_PX`, which lands the label exactly on the dot's outer
edge, and the map label gained the ship's padded backing pill (`padding: 4x1`,
its own fill, text nested inside). Net visual change: the map's glyphs move 2 px
left and gain a dark backing; the ship's move 4 px left.

## Clip and overlap

Both checked against the rig rather than argued about:

- a blip straddling the viewport's clip edge is pickable over its visible half
  and dead over the clipped half (`Overflow::clip()` is in the rig's viewport, so
  every other click test runs through clipping too);
- two contacts stacked on one pixel resolve to the TOPMOST by UI stacking order,
  deterministically - the bigger label pill does not change that.

## The rigs

`crates/nova_gameplay/src/hud/nova_os_pointer_rig.rs` stands up the real RTT
composite - offscreen image, its dedicated UI camera, the through-image content
root, the window-space sampling surface - and drives the REAL
`forward_nova_os_pointer` from a window cursor, letting bevy's own `ui_picking`
decide what was hit. Tests say where on the glass they click; the image point the
CRT shows there comes from a transcription of the WGSL, never from the production
helper under test.

Two things the rig needed that a layout-only rig does not:

- `VisibilityPlugin` (plus `Assets<Mesh>` / `Assets<SkinnedMeshInverseBindposes>`
  for its bounds pass): `ui_picking` refuses any node whose `InheritedVisibility`
  is not computed-true, and nothing in a `MinimalPlugins` app propagates it;
- `ButtonPlugin` alone rather than `UiWidgetsPlugins`, whose text-input plugin
  wants IME messages and an `InputFocus` resource a headless rig has no backend
  for.

## Alternatives considered

- **Invert the forward warp properly** (closed form or Newton), as the task's
  step 3 suggested. Rejected once the direction error was found: no inverse is
  needed at all. The shader maps screen -> image, which is the direction the
  pointer wants; solving a cubic would have been machinery in service of a
  mistake.
- **Leave the overscan as a WGSL constant and copy it into Rust.** Rejected -
  that is the same two-definitions shape that caused the bug, just with the
  second copy in a different file.
- **Mirror the degauss shear too.** Rejected: it is a decaying transient peaking
  at 6 px, pulsed only by an app launch/exit/switch, and chasing it would make
  the pointer jitter exactly while the content is being replaced. Documented on
  the helper rather than left implicit.
