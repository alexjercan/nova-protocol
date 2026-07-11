# Review: Camera jumps at high speed give the controls a twitchy feel

- TASK: 20260711-125225
- BRANCH: fix/camera-jump-hunt

## Round 1

- VERDICT: APPROVE

Reviewed as a diagnosis cycle (the 20260711-121701 precedent):

- The trace method is sound: measuring the ship in CAMERA space is the
  on-screen motion, the only metric the user's report is about. The rig
  is production-faithful (real avian + interpolation + bcs chase +
  ordering pin, production smoothing).
- The mechanism is quantitatively coherent: reviewer re-derived the lerp
  time constant (tau = -1/(7 ln 0.15) = 75 ms) and the steady lag
  (22.4 u at 300 u/s) - both match the traced 40.5 u camera distance
  against the 20.6 u rig; the hitch jump magnitude matches
  (1 - t^dt) * v * dt within rounding. The claim that velocity-lead
  cannot fix the transient (only the steady lag) is correct - checked by
  hand on the update equation.
- The fix routing respects the user's OWN queue: lag -> zoom cap task
  (next), smoothing architecture -> the feel spike (queued last, and the
  fork genuinely needs a feel decision that is not the implementer's to
  make). Both follow-up tasks carry the numbers.
- No production diff; trace deleted with numbers preserved in TASK.md;
  camera_controller tests 9/9 green; ASCII clean.

No findings.
