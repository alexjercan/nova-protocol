# DECISION: how to kill the black gaps in the NOVA OS screen

- TASK: 20260727-162635
- STATUS: ACCEPTED
- DATE: 2026-07-27

## Context

Owner playtest (feedback item 2): the NOVA OS screen "still has black gaps";
the picture should stretch more and run "below the casing lines" so no black
shows between the content and the casing.

The screen content is barrel-warped in the CRT shader (`nova_os_crt.wgsl`,
`warp = 0.12`). `barrel()` pushes UVs OUTWARD from centre by `r^2`, so a screen
corner (offset 0.5 from centre) samples the source at ~1.03 of the panel. That
is outside `[0,1]`, and `rgb * in_bounds` (shader line ~202) zeroes it to
tube-black. So the black is a barrel-shaped margin INSIDE the glass, worst at
the corners. The 18px `NOVA_OS_SCREEN_PAD_PX` does NOT contribute: the CRT
`MaterialNode` is `position:absolute; inset:0`, so it already fills edge-to-edge
under the padding (the padding only insets the headless fallback terminal).

Sibling task 20260727-135204 already established the precedent of recording the
screen-edge treatment as a DECISION.

## Options

- (a) Shader OVERSCAN: scale the sampled UV in toward centre by a constant so the
  bowed corners land back inside `[0,1]`; the picture over-fills the quad and the
  bowed edges fall under the bezel. Exactly a real CRT's overscan.
- (b) Lower the `warp` uniform so the barrel bows less and stays in-bounds. This
  flattens the tube - the opposite of the curved-CRT look tasks 135204/193233
  deliberately built.
- (c) Shrink the screen node / add negative margins so the glass extends under
  the bezel. Does not touch the interior warp-black at all, only the outer frame;
  the corners would still be black.

## Decision

Chosen: **(a) shader overscan**, as a shader `const NOVA_OS_OVERSCAN = 0.93`.

- Applied right after the barrel warp:
  `warped = (warped_raw - 0.5) * NOVA_OS_OVERSCAN + 0.5`. Everything downstream
  (sample, bloom taps, `in_bounds`, the curved rim) reads the overscanned
  `warped`, so the whole treatment follows the enlarged picture and the rim still
  sits at the visible glass edge.
- 0.93 < the 0.943 that just clears the worst corner (offset 0.53 -> 0.5), so
  there is a hair of bleed and no black survives even with the rounded-corner
  mask. It crops ~7% of the source at the edges - acceptable overscan; the
  terminal content has its own inner margin so no glyphs are lost.
- A `const`, NOT a new uniform field: adding a `f32`/`vec2` mid-struct risks the
  encase field-order/alignment corruption lesson
  ([[shadertype-encase-field-order-alignment]]). `warp` is fixed at 0.12, so a
  const overscan tuned to it stays correct; if `warp` is ever retuned, this const
  is retuned with it (both live in the same shader file).

## Rejected

- (b): flattens the tube, undoing the deliberate curved-CRT look.
- (c): leaves the interior warp-black corners untouched; only reshapes the frame.

## Consequences

- The fix lives entirely on the shader (render-capable) path; the headless
  fallback has no warp and no black to begin with, so it is unaffected.
- Validated by RUNNING `screenshot_nova_os` (the WGSL only compiles at runtime -
  [[wgsl-not-covered-by-cargo-check]]), not by `cargo check`.
