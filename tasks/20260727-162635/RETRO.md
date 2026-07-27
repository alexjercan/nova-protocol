# RETRO: NOVA OS screen polish (20260727-162635)

- DATE: 2026-07-27
- OUTCOME: 4/4 playtest-feedback items fixed; review APPROVE round 1 (nits only).

## What went well

- Traced every item to its exact cause BEFORE planning (Explore agent + reading
  the shader/spawn code), so the plan named the real mechanism, not a guess. Two
  early hypotheses were corrected by reading the code:
  - The caret-at-0 bug was NOT the blink or a missing spawn; it was the input
    wrap collapsing to 0 height when all three text pieces are empty (absolute
    caret with top:0/bottom:0 had nothing to stretch to).
  - The black gaps were NOT the 18px screen padding (the MaterialNode is
    `position:absolute; inset:0` and already fills under it) - purely the
    barrel-warp margin where sample UVs exceed [0,1]. This saved a pointless
    padding change.
- Kept item 2 a shader `const` (NOVA_OS_OVERSCAN) instead of a new uniform
  field, dodging the encase field-order/alignment corruption trap
  ([[shadertype-encase-field-order-alignment]]).
- Validated the shader+asset the ONLY way that works: RUNNING screenshot_nova_os
  ([[wgsl-not-covered-by-cargo-check]]), then eyeballing the captured PNGs
  (crop+upscale) rather than trusting cargo check.
- The out-of-context reviewer independently re-derived the overscan corner math
  and confirmed 19.2px == Bevy's default line box, converting two of my
  "by-construction" claims into verified ones.

## What went wrong / friction

- A heredoc mis-attached to the wrong `cat` in a `||` fallback hung for the full
  2-minute timeout. Lesson: don't build multi-branch `cat > file <<EOF` shell;
  use the Write tool for file bodies.
- First asset recolour attempt (`-fill white -colorize 100`) silently collapsed
  the PNG to gray+alpha with a broken channel split. Fixed by the explicit
  alpha-extract + CopyOpacity-onto-white-canvas route. Lesson below.
- The empty-input caret could not be PROVEN from the screenshot: the welcome
  shot caught a blink-off (caret alpha toggles at 1.25 Hz, 50% duty), and this
  repo's tests deliberately run no UI layout (they stamp ComputedNode), so there
  was no in-grain way to assert the post-layout caret height. Left to the manual
  DoD; the active shot proves the caret renders and the min_height removes the
  sole collapse cause, so confidence is high but not screenshot-proven for the
  empty case. A blink-phase-frozen capture mode in screenshot_nova_os would have
  closed this deterministically.

## Lessons for the ledger

- imagemagick recolour that must PRESERVE ALPHA and set opaque pixels to a flat
  colour: `-fill white -colorize 100%` is unreliable (can collapse to gray+alpha
  and split channels). Use: extract alpha, then composite a solid-colour canvas
  through it -
  `magick in.png -alpha extract a.png; magick -size WxH xc:white a.png -alpha off
  -compose CopyOpacity -composite PNG32:out.png`. Verify with
  `-background black -alpha remove -alpha off -format '%[fx:mean.r] ...'` (equal
  R=G=B = white where opaque).
- Time-based blink/animation makes single-frame screenshots a coin-flip for
  proving a static-visibility fix. For those, either add a capture mode that
  freezes the animation phase, or accept the manual DoD and say so.

## Action items

- [ ] (optional, filed as thought) Consider a blink-freeze env flag for
  screenshot_nova_os so empty-caret / animation-phase states can be captured
  deterministically. Not filed as a task yet - low priority; raise if the caret
  regresses.
