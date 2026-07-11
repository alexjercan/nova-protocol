# Review: Residual wobble while decelerating from high speed

- TASK: 20260711-121701
- BRANCH: fix/decel-wobble

## Round 1

- VERDICT: APPROVE

Verified with fresh eyes:

- The falsification is sound. The rig reproduces the reported scenario
  faithfully: shipped 5-section geometry (verified against
  nova_assets/scenario.rs - the player ship really has a single centered
  drive, so the plan's balancer-chatter hypothesis was structurally
  impossible and the scope correction is right), constant retrograde
  command (verified against bcs point_rotation.rs that the manual command
  is mouse-delta accumulated and cannot feed camera motion back into the
  PD), full burn to rest. A hull that holds 0.0023 rad/s max spin over a
  22 s / 1400-frame burn is not wobbling.
- The regression is a real guard, not a trophy: it pins the whole
  103527-family invariant (no speed-coupled torque) in the exact regime
  the user plays, with both delivery guards (flip completed, burn reached
  rest) so neither a dead command path nor a dead engine can green it.
  Complementary to high_speed_stop_settles_without_tumbling (autopilot
  STOP, settle + release) rather than duplicating it.
- The redirect is evidence-based and actionable: camera ordering pin
  (5ba0e3c) postdates the user's test session, and the zoom-slew note on
  20260711-121711 gives the camera task a concrete second objective. The
  user-facing re-test guidance is written in the task file.
- Reviewer ran flight:: 58/58; diff is ASCII-clean; print-trace
  diagnostic deleted in-branch per convention; TASK.md records the rig
  exactly (residual-roll retro rule followed).

No findings.
