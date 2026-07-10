# Retro: Orbit survey zoom

- TASK: 20260710-222518
- BRANCH: feature/orbit-survey-zoom (squashed to master as eb7bcce)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR + 2 NIT, fixed in-round)

## What went well

- The interpretation decision ("allow zooming out" vs no zoom control
  existing) was made explicitly at plan time, recorded in the Resolution,
  and verified by the reviewer against the actual wheel bindings - no
  silent scope guess.
- Scaling the dolly to the planned ring radius instead of a fixed step
  came from asking what "the area" IS; the review confirmed it frames
  both small and giant orbits.
- Riding the existing chase smoothing meant zero new smoothing code and
  made the "blend like a mode switch" claim literally true (the reviewer
  traced it to the same lerp target in bcs).

## What went wrong

- R1.1: I wrote `f32::clamp(base_len, MAX)` two retros after writing the
  recompute-both-sides convention - the bounds are fine with today's
  constants, but both are advertised playtest knobs, and clamp PANICS
  when a knob turn crosses them. The convention covered the arithmetic
  but not the API's failure mode. Refinement: when the two sides of a
  bound are independently tunable, prefer min/max composition over
  f32::clamp - degrade, don't panic.
- Left STATUS: IN_PROGRESS while ticking the "close TASK.md" step - a
  pure oversight the reviewer caught.

## What to improve next time

- Treat `f32::clamp` as a code smell wherever both bounds are tunables
  or derived values; min-then-max is the same line and cannot panic.

## Action items

- [x] Fixed in-round; no follow-ups. The ship-vs-body framing NIT is
  playtest territory (recorded in REVIEW.md R1.3).
