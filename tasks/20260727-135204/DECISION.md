# DECISION: how to wrap the NOVA OS screen border to the curved CRT

- TASK: 20260727-135204
- STATUS: ACCEPTED
- DATE: 2026-07-27

## Context

The screen content is barrel-warped in the CRT shader (`nova_os_crt.wgsl`,
`warp = 0.12`), so the picture bows like a tube. The screen edge treatment,
though, was flat straight-edged Bevy UI border nodes: the screen node's 1px
bright-phosphor border and a `spawn_nova_os_phosphor_rim` overlay (a glow + line
pair), both rounded rectangles at a fixed 12px radius. Against the bowed content
those straight rings read as a flat rectangle sitting in front of a curved
screen (owner playtest, feedback item 2).

A Bevy UI border node is ALWAYS a straight-edged rounded rectangle; it cannot
bow with the barrel warp. So the three plan options were mutually exclusive:
(a) bake the edge into the warped CRT render; (b) overlay a pre-shaped curved
frame graphic; (c) drop the hard border for a soft rim/vignette.

## Decision

Chosen: **(a), refined by (c)** - move the crisp screen edge into the CRT
shader as a phosphor rim that follows the BARREL-WARPED panel bounds, so the lip
bows with the tube exactly (the shader is the only place that knows the curve).

Concretely:
- The shader computes the distance to the panel edge in `warped` (barrel) uv
  space, not screen space, so the rim's iso-contour is the same bowed curve as
  the content edge. A phosphor lip glows there, fading inward over a small band.
- The flat straight UI rings are demoted so they no longer fight the curve: the
  screen node's bright-phosphor border becomes a dark recess line (the glass
  now sits recessed in the bezel), and the `phosphor_rim` overlay drops to a
  faint outer halo (kept as nodes so the headless fallback and the existing
  rim test still have their glow+line pair, but no longer a hard bright frame).
- The plastic bezel frame stays a rounded rect: a moulded plastic bezel IS
  flat, so it is not the offender; only the glass edge needed to curve.

## Rejected

- (b) pre-shaped overlay graphic: needs a new asset shaped to the exact warp,
  and it would go stale if `warp` is retuned. The shader already has the warp
  as a live uniform, so (a) stays correct automatically.
- Pure (c) in UI nodes only: a soft rounded-rect glow still cannot bow with the
  barrel, so at the edge midpoints it would still not hug the bowed content.

## Consequences

- The curved edge only exists on the shader (render-capable) path; the headless
  fallback keeps the faint UI rim. That is acceptable - the curve is a visual,
  owner-confirmed at the manual acceptance check; headless is test-only.
- Grain green-tint (feedback item 3) rides the same shader pass.
