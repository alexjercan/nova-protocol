# REVIEW: NOVA OS screen polish (20260727-162635)

- VERDICT: APPROVE
- ROUND: 1 (out-of-context reviewer)
- DATE: 2026-07-27
- COMMIT: c790a227 on feature/nova-os-polish

## Summary

All four intended fixes are correct and well-implemented. Only nits; none block.

## Correctness verification (reviewer, independent)

1. Overscan math CORRECT. `barrel(uv, 0.12)` pushes the corner to -0.03 (the
   tube-black gap); overscan `(x-0.5)*0.93+0.5` pulls all four corners to
   ~0.0071 / 0.9929 - inside [0,1] with margin. The "< 0.943 clears the corner"
   claim is exact (`o <= 0.5/0.53 = 0.9434`). Barrel runs once, overscan once
   (no double-application). `in_bounds`, the 12-tap bloom, `edge`/`rim`, and
   `textureSample` all consume the single overscanned `warped` (intended). Glass
   effects (vignette, scanlines, grain, hum, retrace, corner_mask) correctly read
   `in.uv`, so they are untouched. ~7% symmetric content crop under the bezel =
   correct CRT overscan, fine for a margined terminal.
2. Caret min_height CORRECT. `16 * 1.2 = 19.2px` is exactly Bevy's default line
   box for the 16px prompt text, so empty and typed carets are the SAME height
   (not taller). The absolute caret (top:0/bottom:0, ZIndex 2 over text ZIndex 1)
   can no longer collapse to 0. `position_nova_os_block_caret` only sets `left`;
   blink is orthogonal.
3. GRAIN_TINT safe. Grain is signed and added post-tint; lowering R/B strictly
   reduces excursion vs. the shipped version; all channels stay in [0,1].
4. Asset recolour CORRECT. `-alpha remove` gives R=G=B=0.283 (white where opaque,
   alpha preserved). BOTH call sites (nova_os.rs:3574 plate, objective_hint.rs:117
   topbar) render native on dark chrome, so white-on-dark reads at both. No test
   inspects the icon pixels.

## Tests

- `nova_os_block_caret_is_absolute_and_tracks_measured_text_width` (asserts
  position_type + left only) - untouched.
- prompt-input-wrap assertion (flex_grow + overflow only) - untouched.
- No regressions.

## Findings (all nits, addressed)

- nit nova_os.rs:4064 - note that 1.2 is deliberately Bevy's default line-height
  factor. FIXED (comment clarified).
- nit nova_os_crt.wgsl:73 - "~1.03 of the [0,1] source" phrasing slightly
  imprecise (the overshoot is 0.03 in uv at the corner). FIXED (comment
  clarified).
- nit commit message - second call site is objective_hint.rs, not nova_os.rs.
  Noted; no code change.
