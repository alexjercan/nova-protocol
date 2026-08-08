# GOTO a gravity-well body parks into ORBIT on arrival

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0, autopilot, gravity, ux

## Goal

User request (2026-07-10): flying GOTO at a big object that carries a
gravity well should end in a parked orbit, not a dead stop that
immediately starts falling. When the GOTO leg completes (today:
autopilot_system's `done` path removes the Autopilot at the standoff) and
the destination entity carries a GravityWell, hand off to
`AutopilotAction::Orbit { well: target }` instead of disengaging - the
one-key parking flow becomes zero-key when you already told the computer
where to go.

## Steps

- [x] In `autopilot_system`'s done branch (flight.rs ~1560, `done &&
  hottest_input <= 0.05`): when the action is `Goto { target }` and
  `q_wells.contains(target)`, replace the autopilot in place with
  `Autopilot::engage(AutopilotAction::Orbit { well: target, plan: None })`
  (engage resets the phase to Align; the ORBIT plan block fills the plan
  from the arrival radius next tick) instead of removing the component.
  Everything else - GotoPos, unsized/well-less targets, STOP - releases
  exactly as before.
- [x] Verify the telemetry handoff needs no code: the Orbit arm publishes
  no ManeuverTelemetry, so the existing `None if has_telemetry` branch
  clears the GOTO numbers on the first orbit tick.
- [x] Rework the well integration test
  (goto_into_a_well_stops_at_the_standoff_instead_of_crashing): the leg no
  longer disengages at a well body - run until the action becomes Orbit,
  keep the never-below-surface floor over the whole run, then run on and
  assert the ship stays engaged (ORBIT never completes) and above the
  surface. The sized-target test (BodyRadius, no well) keeps asserting
  release - the contrast case.
- [x] Run flight, input::ai, hud tests and `cargo check --workspace
  --examples`.
- [x] Docs: tasks/20260710-195954/NOTES.md (handoff rule,
  breakout semantics unchanged, no-toggle default per the user request);
  close TASK.md.

## Notes

- /plan owns the steps. The seam is small: the `done` branch in
  autopilot_system knows the action; for `Goto { target }` where target
  has a GravityWell (and ideally is the ship's dominant well), replace the
  component with Orbit instead of removing it. Breakout semantics (any
  flight input, Z) unchanged; the ORBIT plan block then picks the ring
  from the arrival radius - the standoff (50u) sits inside the Gravity
  Rock's stable band (31.5..122.4), which is why this works unmodified.
- HUD follows for free (AP ORBIT states, ring, cues retire).
- Interacts with 20260710-193500 (gravity-blind arrival, crashes into
  well bodies): that task fixes reaching the standoff alive near big
  wells; this one decides what happens after. Same code region -
  whichever is picked up second should re-read the first.
- Consider a settings toggle only if playtests dislike the automatism;
  default on per the user request.

## Resolution

Implemented per the Steps, no deviations: one in-place engage() in the
done branch, gated on the action being Goto and the target being in
q_wells. The telemetry handoff indeed needed no code (verified: the Orbit
arm publishes nothing and the None-with-has_telemetry branch clears). The
well integration test now covers the whole arc - arrival, handoff at the
surface-relative park point, 1200 frames of engaged station-keeping above
the surface with a filled OrbitPlan; the sized-target test remains the
release contrast case.

Difficulties: none - the seam was exactly where the task notes said, and
the two prior arrival tasks (gravity budget, surface-relative standoff)
had already put the park point inside the stable band.

Checks: flight 56, input::ai 73, hud 55, cargo check --workspace
--examples clean. Full suite and clippy left to CI per policy.
