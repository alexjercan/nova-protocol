# Residual wobble while decelerating from high speed

- STATUS: CLOSED
- PRIORITY: 88
- TAGS: v0.5.0,bug,physics,flight

## Goal

Playtest (user, 2026-07-11, AFTER the raw-pose impulse fix 20260711-103527
and the corkscrew fix f43e550 landed): "the spaceship is a bit unstable
and wobbles a bit when decelerating". The gross flip is gone and stopping
with X reads sharper (user-confirmed), but a residual decel wobble
remains. This is a NEW observation, not a reopen of 20260710-231931 (whose
cross-velocity regression pins zero spurious torque from the application
point).

## Steps

- [x] Diagnostic first (family convention). SCOPE CORRECTION discovered
      while reading the shipped scenario: the player ship has exactly ONE
      thruster (single centered rear drive), so recruit/balancer chatter
      is impossible on it - the planned per-engine throttle trace was
      dropped and the rig became the exact playtest scenario instead: the
      shipped 5-section geometry (sections on the z axis, unit masses, PD
      4/4/40), 300 u/s, command flipped to retrograde (mouse-still =
      constant command, verified: PointRotationOutput is mouse-delta
      accumulated, camera-pose INDEPENDENT - no camera feedback loop into
      the command), then full reverse burn to rest.
      VERDICT: the hull is DEAD STEADY - flip settles cleanly, and across
      the entire 22 s burn (1411 frames) max spin is 0.0023 rad/s with
      nose alignment 1.0000. There is NO physical wobble in this regime.
- [x] Staleness check: the rotation command is written in Update
      (one-frame control staleness) but is CONSTANT while holding a
      direction - contributes nothing. The impulse application point is
      raw-consistent since 20260711-103527 (its regression stays green).
- [x] Fix per diagnosis: no physics fix to make - the mechanism is not
      physical. Two camera-side candidates identified for what the user
      SAW: (a) the bcs chase-move vs transform-propagation ordering was
      an unordered coin flip until 5ba0e3c pinned it - and that pin
      landed AFTER the user's test session, so their build may have
      rendered every frame one camera-step late; (b) the speed-based
      zoom contracts continuously while decelerating - noted on the
      camera cap task 20260711-121711 to slew/cap the zoom target.
- [x] Regression `hold_reverse_decel_from_300_keeps_the_hull_steady`
      (flight.rs): the trace converted to a pinned bound (max spin < 0.05
      rad/s across the whole burn) with delivery guards (flip completed,
      burn reached rest); the print-trace diagnostic was deleted in the
      same branch per convention.

## Notes

- Context: tasks/20260711-103527/SPIKE.md
  (fix record), tasks/20260709-125640/RETRO.md
  (trace-first discipline, falsified-theory bookkeeping).
- Related guards that must stay green:
  high_speed_stop_settles_without_tumbling (settle bound),
  cross_velocity_burn_keeps_the_hull_steady_at_high_speed (application
  point), off-axis counter-torque tests (balancer).
- Filed from user feedback mid-flow (2026-07-11); not part of the
  20260711-094915 umbrella's original four, but the umbrella's combined
  verification should mention it.

## Resolution

What changed: one regression test plus a scope note on the camera cap
task. No physics changes - the investigation falsified the premise that
the wobble is physical, with a production-faithful trace of the exact
playtest scenario.

Evidence rig (per the residual-roll retro's record-the-rig rule):
flight_app harness (60 fps manual time, 64 Hz physics), shipped 5-section
geometry on the z axis with unit masses and a single rear drive
(magnitude 1.0) at z = +2, PD 4/4/40 on the controller at the origin,
TransformInterpolation on the root, LinearVelocity -Z * 300,
ControllerSectionRotationInput = 180 deg yaw (constant), FlightIntent
burn = 1.0 after a 240-frame flip phase.

What to tell the user: re-test the decel feel on a build that includes
5ba0e3c (the camera ordering pin) - it landed after the reported test
session and fixes a real one-frame camera lag coin flip; if the wobble
persists, it should improve again once 20260711-121711 caps and slews the
speed-zoom. If it STILL persists after both, reopen with the new
observation - the physical side is now pinned by a regression, so the
next hunt starts camera-side.

Self-reflection: the task plan encoded the balancer-chatter hypothesis
before checking whether the shipped ship even has multiple engines;
reading the scenario config first inverted the whole investigation in
five minutes. Also the third "plan encoded a wrong mechanism" instance in
this family - the promotion threshold from the bullet retro is met; the
plan skill lesson goes in this cycle's retro.
