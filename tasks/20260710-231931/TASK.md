# Spaceship rendering is twitchy at high velocity

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): the spaceship itself renders twitchy at
high velocity. Investigated 2026-07-11 as part of the twitching family:
docs/spikes/20260711-103527-twitching-family-two-clocks.md. The camera and
interpolation wiring checked out sound (interpolation opt-ins present,
exp-decay lerp frame-rate independent, anchor ordering correct), so the
leading explanation is that the hull twitch is REAL attitude jitter caused
by the thruster application-point bug (20260711-103527): at high speed
under thrust, spurious torque jiggles the hull every tick.

## Steps

- [ ] After 20260711-103527 lands, add a no-input straight-line regression:
      ship coasting/burning at high V with zero rotation command; assert
      per-tick angular velocity stays ~0 across a few hundred ticks
      (catches spurious thrust torque re-emerging).
- [ ] Re-test the visual symptom at high velocity (headless where
      provable, plus the 10_gameplay example for feel); record the verdict
      here.
- [ ] If a residual render twitch remains with a steady hull: profile the
      camera anchor chain for freshness (anchor written in Update from the
      frame's eased pose, chase moves in PostUpdate) and fix the residual;
      otherwise close as resolved-by-20260711-103527.
- [ ] cargo check + fmt + new tests; note the outcome in the spike doc's
      fix record.

## Notes

- The camera chases per-frame (bcs ChaseCamera in PostUpdate, smoothing
  0.15) while the hull's Transform is eased between ticks - that path was
  verified correct on 2026-07-11; do not re-litigate it unless the
  residual-twitch step above demands it.
- Depends on: 20260711-103527 (thruster application point fix).
