# Trajectory ribbon terminates at the arrival park point

Task: tasks/20260710-214316 - review finding R1.4 of 20260710-202408: the
trajectory ribbon drew its last segment to the target CENTER, so on a big
body it visually plunged radius + standoff past where the arrival
actually stops (the surface-relative standoff geometry of
docs/retros/20260710-surface-relative-standoff.md).

## What changed

- New `ManeuverTelemetry.park_point: Vec3`
  (crates/nova_gameplay/src/flight.rs): where the leg comes to rest.
  - `arrival_desired` (GOTO/GotoPos) publishes
    `goal - closing_dir * standoff.min(distance)` with
    `standoff = arrival_standoff + resolved target radius`. The `min`
    matters: at or inside the park envelope the point degenerates to the
    ship's own position - the computer stops where it is and never plans
    a leg back out to the boundary, so the instrument must not draw one
    (a plain boundary point would put a backward-pointing stub behind
    the ship during terminal creep, up to ~11u in a strong well per the
    known limit in the standoff doc).
  - The STOP arm publishes `park_point == goal` (its goal already IS the
    predicted rest point; no standoff).
- `sync_trajectory_ribbon` (crates/nova_gameplay/src/hud/
  holo_instruments.rs) pushes `telemetry.park_point` instead of
  `telemetry.goal` as the final path point. The flip gate keeps facing
  along `goal`: the direction to the center is well-defined even when
  the park point degenerates to the ship position.

## Why publish the point instead of deriving it in the HUD

- The telemetry seam doctrine is "the HUD computes nothing"
  (hud/maneuver_instruments.rs module doc); the arrival geometry lives
  in the autopilot next to the plan it describes, so instrument and
  autopilot cannot drift apart.
- Deriving the radius HUD-side from the published fields
  (`center_distance - telemetry.distance`) breaks when the surface clamp
  engages (`distance` floors at zero), and mixes the raw-clock scalar
  with the render-clock ship pose (two-clocks rule,
  docs/spikes/20260711-103527-twitching-family-two-clocks.md).
- Publishing the effective standoff as an f32 and computing the point in
  the HUD was the middle option; rejected for the same seam reason - the
  `min(standoff, distance)` degeneracy handling would then live in the
  HUD, away from the envelope test it mirrors.

## Evidence

- Fail-first: `ribbon_terminates_at_the_park_point_not_the_goal` run
  against the unmodified ribbon fails with "the ribbon ends at the park
  point [0, 0, -250], got [0, 0, -300]" - the full 50u standoff
  overshoot; passes after the one-line ribbon change.
- `goto_publishes_telemetry_and_disengaging_clears_it` now also asserts
  the GotoPos park point sits exactly one standoff short of the goal on
  the closing line, and that a STOP publishes `park_point == goal`.
- `goto_standoff_is_surface_relative_for_sized_targets` now also asserts
  a sized target (BodyRadius 30) pushes the park point to
  standoff + radius from the center, and samples the leg inside the park
  envelope to pin the degenerate park point to the ship (< 2u).

## Difficulties

- None to speak of; the seam was already shaped for this (flip_point set
  the precedent for publishing plan geometry as world points). The main
  design work was the inside-envelope case, settled by the "instruments
  must not out-promise the autopilot" rule already written on the ribbon
  module doc.

## Self-reflection

- The task note's "check whether telemetry needs to publish the resolved
  radius or effective standoff explicitly" was answered with a third
  option (publish the point itself); writing the three options down in
  the plan note first made the choice quick to defend in review.
