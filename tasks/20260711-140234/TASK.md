# Quiet the GOTO arrival hunt: widen the settle deadband in the desired-velocity-zero regime

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0,bug,physics,flight,feel

## Goal

Playtest (user, 2026-07-11): "the spaceship still feels a bit clunky on
GOTO ... it wobbles" while STOP "perfectly turns around and stops as it
should regardless of speed". The spike traced the wobble to a terminal
attitude hunt: a GOTO leg crosses the standoff boundary at ~3.5 u/s (the
arrival limit plus the min_approach floor guarantee a hot crossing by
design), the doorstep brake leaves a residual of ~0.45-0.6 u/s, and that
residual sits JUST ABOVE the shipped `attitude_deadband` (0.4) - so the
"crumbs" rule never engages and the computer re-aims the hull after a
sub-visible velocity error at ~0.4-0.5 rad/s (~25 deg/s of visible nose
swing) for 3-5 seconds at the end of EVERY GOTO leg. STOP is immune: its
error decreases monotonically through the deadband once, nose-on-error,
and the done gate releases at 0.2 u/s.

Spike: docs/spikes/20260711-140234-feel-filtering.md (mechanism, traces,
measured variants). Measured on the shipped 5-section rig (PD 4/4/40,
single rear drive, GotoPos 30 deg off-nose over ~670 u):

- shipped (deadband 0.4): terminal spin max/rms 0.59/0.38 rad/s over 175
  frames; RELEASE spin 0.44 rad/s (the post-release guard is 0.5!).
- deadband 0.6: terminal 0.28/0.12, release spin 0.088.
- deadband 0.75: terminal 0.10/0.03, release spin 0.047, release creep
  0.58 u/s, max lateral path deviation UNCHANGED (4.41 u in all variants).

Direction (spike recommendation): scope the wider settle band to the
regime where the leg's desired velocity is zero (inside the GOTO
standoff, STOP at rest) instead of raising `attitude_deadband` globally -
`orbit_hold_enter` (0.8) is documented as 2x the deadband and ORBIT
station-keeping should keep the tight band. STOP precision is unaffected
by construction: with the nose on the error the drive keeps aligned
authority, so the fine-release branch never fires and STOP still brakes
to `stop_speed_epsilon`. The wider band only stops the hull RE-AIMING for
crumbs it can only chase by flipping.

The spike left an `#[ignore]`d diagnostic (`goto_wobble_diagnostic` in
flight.rs) with the full A/B rig; convert its terminal-phase measurement
into the regression and delete the rest per the diagnostic convention.

## Notes

- Regression to add (A/B-proven by the spike): a GotoPos leg on the
  shipped rig asserting terminal-phase (remaining < 15 u) max spin below
  ~0.15 rad/s and release spin below ~0.1, with delivery guards (leg
  completes, standoff actually crossed). The shipped config fails it at
  0.59 max.
- Guards that must stay green: post-release residual spin < 0.5
  (flight.rs), high_speed_stop_settles_without_tumbling,
  hold_reverse_decel_from_300_keeps_the_hull_steady, AI settle roll bound
  (input/ai.rs), orbit hold/station-keeping tests.
- Value tuning: 0.75 measured best on the shipped rig; pick the knob
  (new `arrival_settle_deadband` field vs a multiplier) during /plan.
- Related but separate: the cross-clock command handoff (task
  20260711-140241) - the hunt exists under BOTH wirings, so these are
  independent fixes.
