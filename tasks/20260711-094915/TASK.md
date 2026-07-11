# Bug related to twitching

- STATUS: OPEN
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
docs/spikes/20260711-103527-twitching-family-two-clocks.md.

This task closes LAST, as the family-level verification.

## Steps

- [x] Investigate the family and document the shared mechanism
      (docs/spikes/20260711-103527-twitching-family-two-clocks.md).
- [ ] Member tasks CLOSED: 20260711-103527 (thruster application point),
      20260710-231931 (ship twitch re-test), 20260710-231930 (bullets),
      20260710-231928 (HUD text), 20260710-231929 (crosshair).
- [ ] Combined high-speed verification on master: hard decel from high
      speed holds attitude (no flip), bullet streams linear, HUD text and
      crosshair stable; full check suite green.
- [ ] Ask the user to re-run the playtest checklist at high velocity and
      capture any residual feel notes as new tasks.

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
