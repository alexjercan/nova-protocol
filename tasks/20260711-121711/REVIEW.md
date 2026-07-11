# Review: Cap the chase camera zoom-out and pivot distance at high speed

- TASK: 20260711-121711
- BRANCH: fix/camera-lag-lead

## Round 1

- VERDICT: APPROVE

Verified with fresh eyes:

- The discrete-time lead formula is correct: reviewer re-derived the
  steady state of `new = lerp(cur, desired, 1 - r)` against an anchor
  advancing v*dt/frame and confirms e = v * dt * r / (1 - r); the
  degenerate guards (smoothing 0 and 1, dt 0, r ~ 1) are right. The
  continuous-tau overshoot the test caught (2.4 u at 60 fps) is exactly
  tau - dt*r/(1-r) at these numbers - the fix history in TASK.md is
  honest and instructive.
- The frame-of-reference handling is right: the lead is rotated into the
  anchor frame and the bcs offset z-sign convention is applied
  (world = rot * (x, y, -z)); focus_offset untouched preserves framing -
  the regression asserts framing equality, which pins both the magnitude
  AND the direction handling in one bound.
- Chaining input -> mode -> rig removes a pre-existing ordering ambiguity
  (the rig previously raced the input write) as a side benefit; the
  chain's rationale is documented at the registration.
- The regression uses the REAL update_camera_rig with physics and
  interpolation, tight 0.5 u bound, delivery guard, and a decisive A/B
  (20 u drift with the lead zeroed). Full lib suite 358/358 re-run by the
  reviewer; ASCII clean.
- Scope: the task's literal ask (a cap) is delivered structurally and the
  reasoning for not adding a knob is documented; the decel-slew note from
  the wobble investigation is resolved by the same mechanism. Both
  follow-ons (feel spike) keep their evidence trail.

No findings.
