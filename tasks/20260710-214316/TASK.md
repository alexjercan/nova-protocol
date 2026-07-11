# Holo ribbon should terminate at the arrival park point, not the target center

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: v0.5.0, hud, polish

## Goal

Review finding R1.4 of 20260710-202408 (2026-07-10): the trajectory
ribbon (hud/holo_instruments.rs, sync_trajectory_ribbon) draws its last
segment to the target CENTER, so on a big body it visually plunges
radius + standoff past the actual park point. Terminate the ribbon at the
arrival park point instead: goal minus closing_dir * (arrival_standoff +
resolved target radius) - the same surface-relative geometry the arrival
now flies (docs/2026-07-10-surface-relative-standoff.md).

## Notes

- Pre-existing behavior (the ribbon always overshot by the standoff);
  only worth doing as HUD polish. The park point is derivable from
  ManeuverTelemetry (goal, distance is surface-relative) without new
  physics plumbing - check whether telemetry needs to publish the
  resolved radius or effective standoff explicitly.
- Plan decision (2026-07-11): publish the park point explicitly as a new
  `ManeuverTelemetry.park_point: Vec3` instead of deriving it in the HUD.
  Deriving the radius from `distance` breaks when the surface clamp
  engages (distance floors at 0 in flight.rs `arrival_desired`), and
  mixing the raw-clock scalar with the render-clock ship pose violates
  the two-clocks rule. The telemetry seam doctrine is "the HUD computes
  nothing" (hud/maneuver_instruments.rs module doc).

## Steps

- [x] Add `park_point: Vec3` to `ManeuverTelemetry` (flight.rs): the
  point the leg comes to rest at. In `arrival_desired` (both branches):
  `goal - closing_dir * standoff.min(distance)` where
  `standoff = arrival_standoff + target_radius` (the standoff already
  computed at the top of `arrival_desired`) and
  `closing_dir = to_target.normalize_or_zero()` - at/inside the envelope
  this degenerates to the ship position, so the instrument never draws a
  leg the computer will not fly. In the STOP arm: `park_point: goal`
  (the predicted rest point is the park point).
- [x] Terminate the ribbon at the park point: in
  hud/holo_instruments.rs `sync_trajectory_ribbon`, push
  `telemetry.park_point` instead of `telemetry.goal` as the final path
  point. The flip gate keeps facing along `goal` (well-defined even when
  the park point degenerates to the ship position).
- [x] Update the `ManeuverTelemetry` literals in the holo_instruments
  and maneuver_instruments test helpers.
- [x] Tests, fail-first where the behavior changes: (a) holo ribbon test
  asserting the last segment ends at `park_point`, not `goal` - run it
  against the unmodified ribbon to record the failure; (b) flight.rs:
  GotoPos telemetry publishes `park_point` standoff-short of the goal
  along the closing line; sized-target GOTO publishes it
  `standoff + radius` short (extend
  `goto_standoff_is_surface_relative_for_sized_targets`); STOP publishes
  `park_point == goal`.
- [x] cargo check + cargo fmt + run the new/updated tests
  (one substring filter per cargo test run).
- [x] Document the change in docs/.
