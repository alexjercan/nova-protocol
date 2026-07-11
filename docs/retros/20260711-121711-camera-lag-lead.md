# Retro: Camera velocity lead - the "zoom cap" that was a lag

- TASK: 20260711-121711
- BRANCH: fix/camera-lag-lead (squash-merged as eced738)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **The diagnosis cycle paid for itself immediately**: the 125225 trace
  had already measured the mechanism and magnitude, so this cycle was
  pure implementation - derive, code, pin - with no exploration.
- **The tight regression bound caught a real formula error.** The first
  landed lead used the continuous time constant and overshot by 2.4 u at
  60 fps; the 0.5 u speed-invariance bound rejected it and forced the
  correct discrete-time form. A "did it improve" style assertion would
  have shipped the sloppy version.
- **Framing as the invariant, not distance.** Asserting the ship's
  position in camera space (rather than camera distance) pinned the
  magnitude AND the rotation-frame/sign handling in one number.

## What went wrong

- The continuous-vs-discrete discretization slip itself: the lag of a
  per-frame lerp is dt * r / (1 - r), not the continuous tau. Root
  cause: deriving from the differential limit out of habit when the
  system is explicitly frame-stepped.

## What to improve next time

- When compensating a DISCRETE-time filter, derive the steady state of
  the actual update equation, not its continuous limit - and let the
  regression bound be tight enough to tell the two apart.

## Action items

- [x] The feel spike (20260711-125227) inherits the note that the
      post-hitch tail is now short; its remaining question is the hitch
      transient itself (ship-relative smoothing).
