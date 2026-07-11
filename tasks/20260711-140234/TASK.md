# Quiet the GOTO arrival hunt: widen the settle deadband in the desired-velocity-zero regime

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.5.0, bug, physics, flight, feel

## Steps

- [x] Regression first (fail-first A/B): `goto_arrival_settles_without_hunting`
      on the shipped 5-section rig flying GotoPos (300,0,-600) to
      completion; terminal-phase (remaining < 15 u) max spin < 0.15 rad/s
      and release spin < 0.1, with delivery guards (leg completes,
      standoff actually crossed, a real flip happened). Pre-fix failure
      measured at 0.5949 rad/s (bitwise match to the spike's PROD row).
      RIG CORRECTION along the way: the regression is PRODUCTION-wired
      (command copy in Update via diag_app(true)), not flight_app - see
      the wiring-dependence finding below.
- [x] `FlightSettings::settle_deadband` (default 0.75) added - but the
      planned scoping was FALSIFIED twice and corrected:
      1. Scoped to `desired == Vec3::ZERO`: terminal max spin unchanged
         BIT-FOR-BIT (0.6727556 pre and post) - the hunt's onset is in
         the brake tail where desired is still nonzero. Re-scoped BY LEG:
         STOP/GOTO/GotoPos get the settle band, ORBIT keeps
         attitude_deadband.
      2. Band alone (urgency left on attitude_deadband): terminal 0.643 -
         still failing. The spike's global-raise A/B was CONFOUNDED: it
         moved the crumb band AND the urgency denominator together. With
         BOTH keyed to the leg-scoped crumb_band the spike numbers
         reproduce exactly (terminal 0.0973, release spin 0.047).
- [x] Guard suite: two guards encoded the old arrival contract and were
      updated to the settings-derived one (release residual bounded by
      the settle band, not 0.5): autopilot_burn_recruits_a_lateral (the
      recruit's LATERAL residual is now released, not hunted - the
      shipped single-centered-drive ship keeps exact rest via the
      aligned-authority argument) and goto_into_a_well_stops (ORBIT
      handoff residual 0.705 <= band). Everything else unchanged: flight
      module 59 green, ai module 86 green (fmt+check clean; full suite
      deferred to CI per user instruction).
- [x] Diagnostic kept `#[ignore]`d for 20260711-140241; spike Fix record
      appended (including the falsifications and the NEW wiring-dependence
      finding); a WARNING section added to 140241's task file - the
      same-tick handoff it proposes REINTRODUCES the hunt (terminal 0.63
      under same-tick vs 0.097 under shipped wiring on this very build),
      so that task must fix the limit cycle first and is gated by this
      regression.

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

## Resolution

What changed: `FlightSettings::settle_deadband` (0.75, documented against
the measured 0.45-0.6 u/s doorstep residual) applied BY LEG in
autopilot_system - STOP/GOTO/GotoPos use it for both the crumb band and
the urgency denominator, ORBIT keeps the tight attitude_deadband - plus
the production-wired regression and two guard updates to the
settings-derived arrival contract.

Evidence rig (record-the-rig rule): diag_app(true) - the spike's
harness with the command copy in the Update schedule, matching
ControllerSectionPlugin's shipped wiring - flying the shipped 5-section
geometry (PD 4/4/40, single rear drive) on GotoPos (300,0,-600) from
rest; terminal window = remaining-to-standoff < 15 u; release state read
at autopilot disengage.

Falsified along the way (both recorded in Steps): the desired==0 scoping
and the band-without-urgency variant. New finding: arrival dynamics are
wiring-dependent (same-tick command handoff phase-locks the hunt);
handed to 20260711-140241 as a blocking warning.

Self-reflection: the spike's deadband experiment moved two coupled knobs
with one setting and attributed the effect to one of them; the
implementation caught it only because the regression was written
fail-first and the scoped version left the number bit-for-bit unchanged
- a strong argument for exact-number A/B oracles over "did it improve"
assertions. The confound should have been caught in the spike by
grepping every reader of the knob before concluding.
