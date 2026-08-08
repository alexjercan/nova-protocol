# Bug related to twitching

- STATUS: CLOSED
- PRIORITY: 99
- TAGS: v0.5.0, bug, physics

## Goal

Umbrella for the twitching family from the 2026-07-10 playtest: ship
twitchy at high velocity (20260710-231931), bullets spew non-linearly
(20260710-231930), HUD text jitters (20260710-231928), turret crosshair
jitters (20260710-231929), and the ship twitches/flips when holding
retrograde from high speed (fixed via 20260711-103527). Investigated
2026-07-11; all five share one root-cause family: mixed sampling of the two
pose clocks (raw fixed-tick `Position`/`Rotation` vs render-eased
`Transform`/`GlobalTransform`) introduced by the 2026-07-09 interpolation
change. Full analysis, falsified hypotheses and fix plan:
tasks/20260711-103527/SPIKE.md.

This task closes LAST, as the family-level verification.

## Steps

- [x] Investigate the family and document the shared mechanism
      (tasks/20260711-103527/SPIKE.md).
- [x] Member tasks CLOSED: 20260711-103527 (thruster application point),
      20260710-231931 (ship twitch re-test), 20260710-231930 (bullets),
      20260710-231928 (HUD text), 20260710-231929 (crosshair). Plus the
      mid-family additions: 20260711-121701 (decel wobble - falsified
      physically, pinned by regression).
- [x] Combined high-speed verification on master: the family's regressions
      all run in the 357/357 green lib suite -
      cross_velocity_burn_keeps_the_hull_steady_at_high_speed and
      hold_reverse_decel_from_300_keeps_the_hull_steady (attitude at
      speed), high_speed_lateral_burn_through_the_com_adds_no_spin
      (application point), bullet_stream_stays_linear_at_high_ship_velocity
      + fire_rate cadence (bullets),
      indicator_projects_with_the_frames_final_camera_pose (HUD),
      pip_anchor_carries_the_same_frame_intercept (crosshair).
      cargo check + fmt clean; full suite in CI.
- [x] User playtest: ran CONTINUOUSLY through the family (better than the
      planned one-shot re-run). Verdicts: speed/flight-computer chips
      confirmed twitch-free; bullets confirmed good in flight; stopping
      with X confirmed sharper. Residuals captured as queued tasks:
      camera jumps at speed (20260711-125225), camera zoom cap + decel
      zoom slew (20260711-121711), bullet one-frame spawn pop
      (20260711-121839), redundant caption speed (20260711-125226),
      feel-filtering spike (20260711-125227, queued last per user).
      INTERIM (user, 2026-07-11, mid-family): bullets confirmed good in
      flight (one-frame spawn pop filed as 20260711-121839); stopping
      with X confirmed sharper; residual decel wobble filed as
      20260711-121701; camera zoom-out cap request filed as
      20260711-121711. Final verdict still pending the full family.

## Notes

- Original playtest text preserved: "at high velocities/distances the
  physics seems a bit janky: the camera following the spaceship feels a bit
  unstable, at high speeds the bullets that are spawned from the PDC turret
  do not have perfect position, they spew out and twitch, so their position
  is not linear as expected, at high speeds the thrusters sometimes create
  torque on the ship which is understandable, but annoying, so for example
  if you try to stop the spaceship from high speed, and try to hold the
  reverse direction, the spaceship cannot hold it's deceleration path
  perrfectly, sometimes it twitches and flips."
- Falsified during investigation (do not re-chase): missing interpolation
  components, frame-rate-dependent camera lerp, f32 precision at documented
  scales, iterative aim-solver oscillation. Numbers in the spike doc.
- The "thrusters create torque, which is understandable" instinct was too
  generous: the torque is numerical (stale application point), not
  physical. The balancer exists precisely to cancel the physical part.
- Depends on: all five member tasks above.

## Resolution

The family's shared mechanism (two pose clocks - raw fixed-tick physics
vs eased render transforms - mixed across schedule boundaries) is
documented in the spike doc with a per-symptom fix record; all five code
fixes landed as one squash commit each with A/B-proven regressions. The
remaining playtest observations are camera-rig and cosmetic items, each
with its own queued task; nothing left in this umbrella's scope.
