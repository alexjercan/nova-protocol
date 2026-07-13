# Spaceship rendering is twitchy at high velocity

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): the spaceship itself renders twitchy at
high velocity. Investigated 2026-07-11 as part of the twitching family:
tasks/20260711-103527/SPIKE.md. The camera and
interpolation wiring checked out sound (interpolation opt-ins present,
exp-decay lerp frame-rate independent, anchor ordering correct), so the
leading explanation is that the hull twitch is REAL attitude jitter caused
by the thruster application-point bug (20260711-103527): at high speed
under thrust, spurious torque jiggles the hull every tick.

## Steps

- [x] Add a high-speed attitude-jitter regression. NOTE: the originally
      planned "straight-line burn" is torque-blind to the old bug (the
      20260711-103527 diagnostic proved a stale offset PARALLEL to the
      thrust produces no torque; coasting produces none at all). The
      faithful scenario is a CROSS-VELOCITY burn - thrust perpendicular to
      travel, i.e. a decel path with drift correction: full production
      stack (PD at the shipped 40 torque budget, TransformInterpolation on
      the hull), centered drive, high cross velocity, zero rotation
      command; assert the maximum per-frame angular velocity stays ~0 over
      a couple hundred ticks. DONE:
      `cross_velocity_burn_keeps_the_hull_steady_at_high_speed`
      (flight.rs), proven against the pre-103527 impulse code via a
      temporary A/B revert in this worktree: max spin 4.26 rad/s unfixed
      (PD overwhelmed) vs ~0 fixed (bound 0.05).
- [x] Re-test the visual symptom at high velocity. Headless verdict: the
      hull is physically steady in the perpendicular-burn regime that
      produced the jitter, so there is no motion left for the (verified
      sound) camera/interpolation chain to alias. Visual feel
      confirmation is deliberately DEFERRED to the umbrella task
      20260711-094915's user playtest step - a headless run cannot judge
      feel, and pretending otherwise would be dishonest.
- [x] No residual render twitch mechanism found in code; closed as
      resolved-by-20260711-103527. If the user playtest still sees ship
      twitch at speed, reopen AT THE UMBRELLA with the new observation
      (the camera anchor chain notes below are the starting point).
- [x] cargo check + fmt clean; flight:: module 57/57 pass; outcome
      recorded in the spike doc's fix record.

## Notes

- The camera chases per-frame (bcs ChaseCamera in PostUpdate, smoothing
  0.15) while the hull's Transform is eased between ticks - that path was
  verified correct on 2026-07-11; do not re-litigate it unless the user
  playtest demands it.
- Depends on: 20260711-103527 (thruster application point fix).

## Resolution

What changed: one regression test
(`cross_velocity_burn_keeps_the_hull_steady_at_high_speed`) plus the spike
doc fix-record entry. No production code changes - the symptom's mechanism
was fixed by 20260711-103527; this task's job was to verify that claim
against the production configuration and pin it.

Difficulties: none. The A/B (temporarily restoring the old impulse body,
running the new test, reverting) took one compile cycle and turned
"probably fixed" into a measured 4.26 -> ~0 rad/s delta.

Self-reflection: the planned step ("straight-line burn") encoded the
spike's pre-diagnostic mental model and would have produced a vacuously
green test; updating the Steps to match the corrected mechanism before
implementing (work skill rule) is what kept the regression honest.
