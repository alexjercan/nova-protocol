# Integrate RCS into autopilot (ORBIT station-keep, GOTO terminal arrival write RcsIntent)

- STATUS: CLOSED
- PRIORITY: 2
- TAGS: v0.7.0, feature, flight, spike

## Goal

Follow-up (the user flagged it as such): once the RCS primitive exists, have the
autopilot drive it instead of coarse main-burn micro-pulses for the maneuvers
that need sub-cap precision:

- ORBIT station-keeping: hold the ring with `RcsIntent` micro-nudges rather than
  main-drive pulses.
- GOTO terminal arrival: the last-few-meters settle at `arrival_standoff` via
  RCS.
- The autopilot writes the same `RcsIntent` the player input writes, so no new
  force path is needed - this is the payoff of building RCS as a shared
  primitive (spike Fork 4A). Requires the RCS verb to be granted for the ship.

## Scope decision (from planning + a user call)

The planning pass found that **ORBIT station-keep via the current RCS is
INCOMPATIBLE** and it is split out (seeded follow-up 20260718-151102):

- `rcs_burn_system` caps ABSOLUTE along-axis speed at `rcs_speed_cap` (2 u/s):
  `along = velocity.dot(world_axis)` is absolute velocity, not the error.
- An orbiting ship moves at `circular_orbit_speed = sqrt(mu/r)` ~= 2.5-6 u/s
  (the repo test pins ~4.9 u/s at r=50), ABOVE the cap. So a prograde RCS push
  gates to zero and a retrograde one brakes the orbit - RCS fights the maneuver.
- ORBIT needs a different primitive (an error-relative / target-relative RCS
  mode) - a redesign, seeded separately. The user chose "GOTO-only now, seed
  ORBIT redesign" (2026-07-18).

**This task delivers the GOTO/STOP terminal settle-to-rest via RCS** (the
compatible case: as the maneuver's goal is rest, velocity drops below the cap and
RCS can P-brake it). Verb-gated, so ships that do not grant `Rcs` (the mainline
campaign, which is disabling RCS pending rework) keep the exact main-drive
arrival unchanged.

## Steps

- [x] Extend `q_computer` in `autopilot_system` (crates/nova_gameplay/src/flight.rs:1183)
  with `Option<&WithheldVerbs>`, and add `Option<&RcsSpeedCap>` +
  `Option<&mut RcsIntent>` to `q_ship` (flight.rs:1145). RcsSpeedCap/RcsIntent are
  in scope (same module); `WithheldVerbs`/`FlightVerb` via `crate::prelude`.
- [x] After the error is computed (flight.rs:1599-1601), compute the RCS-settle
  decision, per ship:
  - `rcs_granted` = any live controller of this ship whose `WithheldVerbs` grants
    `FlightVerb::Rcs` (mirror `ship_grants_verb`, input/player.rs).
  - `rcs_cap` = `RcsSpeedCap` override or `settings.rcs_speed_cap`.
  - `use_rcs = rcs_granted && desired.length() <= settings.stop_speed_epsilon
    && velocity.length() < rcs_cap && error_speed > 1e-3`. The `desired ~= 0`
    gate is what EXCLUDES ORBIT (orbital desired >> cap) and the approach leg of
    GOTO (desired >= min_approach_speed); it admits STOP and GOTO/GotoPos INSIDE
    the standoff (desired == 0). `velocity < cap` is the precondition for the
    capped primitive to P-brake cleanly rather than fight a faster body.
- [x] Write the intent: `local = rotation.inverse() * error` (world error ->
  ship-local), `rcs_intent = (local / rcs_cap).clamp(-1, 1 per axis)` when
  use_rcs else `Vec3::ZERO` (so a stale nudge clears when leaving the regime).
  Since desired ~= 0, `error = -velocity`, so this is a proportional brake that
  fades to zero as velocity does - no overshoot past rest. Write it directly if
  the ship carries `RcsIntent`, else `commands.insert` on first use.
- [x] Zero the main-drive demand when settling by RCS: at the demand line
  (flight.rs:1834) make it `if use_rcs { 0.0 } else { <existing> }`, so the main
  drive spools down and only the torque-free RCS COM push brakes the last meters.
  Leave the rotation/phase logic untouched (the deadbands already stop the nose
  chasing tiny errors; ORBIT is unaffected since use_rcs is always false there).
- [x] Confirm completion still fires: `done`/self-disengage is velocity-based
  (`error_speed <= stop_speed_epsilon`, flight.rs:1697), and the RCS P-brake
  drives velocity -> 0, so GOTO/STOP still self-complete. Verify by test.
- [x] Tests (headless, flight_app/orbit_app + spawn_ship + run, per the existing
  autopilot tests):
  - REGRESSION: existing `goto_arrives_at_standoff_and_disengages` and the ORBIT
    ring-hold test still pass (arrival bounds + rest; orbit still holds).
  - `orbit_never_engages_rcs`: an ORBIT ship's `RcsIntent` stays ~zero for the
    whole hold (the desired-gate excludes it) while it keeps the ring.
  - `stop_or_goto_terminal_brakes_via_rcs`: an Rcs-granting ship settling to rest
    from below the cap gets a nonzero `RcsIntent` during the tail AND reaches rest
    (velocity -> ~0); assert the main-drive `ThrusterSectionInput` stays ~0 during
    that tail (RCS, not the drive, did the braking).
  - `terminal_without_rcs_verb_uses_the_main_drive`: same settle with `Rcs`
    withheld -> `RcsIntent` stays zero and it still reaches rest (fallback).
- [x] Record design/fix notes in `tasks/20260718-122932/NOTES.md` and append to
  the spike Fix record. Seed the ORBIT-redesign follow-up (done in planning:
  tatr 20260718-151102).

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Fork 4: RCS as shared primitive).
Depends on the RCS core primitive (task 20260718-122906, CLOSED).

Reference points verified during planning (via exploration + reading):
- `autopilot_system` signature (has Commands, Rotation, LinearVelocity,
  ComputedMass, q_computer controllers): flight.rs:1140-1209.
- `error = desired - velocity` (world): flight.rs:1599-1601.
- The demand/allocation/spool tail (intercept point): flight.rs:1757-1886;
  `demand` at 1834.
- `done` self-completion (velocity-based): flight.rs:1697-1700.
- `rcs_burn_system` absolute-speed gate (the ORBIT incompatibility):
  flight.rs (gate `along = velocity.dot(world_axis)`, cap `rcs_speed_cap`).
- `circular_orbit_speed = sqrt(mu/r)` ~4.9 u/s at r=50: gravity.rs:292, test 536.

Lessons applied: `two-clocks` (autopilot is FixedUpdate, reads raw
Rotation/LinearVelocity), `verify-first-plan-steps` (the ORBIT incompatibility
was found by reading the gate, not assumed).
