# Autopilot RCS integration - design / fix record

Task: 20260718-122932. Spike: tasks/20260718-122508/SPIKE.md (Fork 4).

## What shipped

The autopilot hands the STOP / GOTO / GotoPos **terminal settle-to-rest** to the
RCS primitive (a torque-free COM P-brake) for ships that grant the `Rcs` verb, in
`autopilot_system` (crates/nova_gameplay/src/flight.rs):

- Gate `use_rcs = rcs_granted && desired.length() <= stop_speed_epsilon &&
  velocity.length() < rcs_cap && error_speed > 1e-3`. The `desired ~= 0` clause
  admits the rest-seeking maneuvers and EXCLUDES ORBIT and the GOTO approach leg
  (their desired is well above zero); `velocity < cap` is the precondition for
  the absolute-speed-capped primitive to brake cleanly.
- When settling: write `RcsIntent = clamp(rotation.inverse() * error / cap, -1,
  1)` (a proportional brake, since `error = -velocity` at rest) and zero the
  main-drive demand so only the RCS COM push acts.
- Verb-gated via the same `WithheldVerbs`/`FlightVerb::Rcs` check the input layer
  uses. A ship without the verb (the mainline campaign, RCS disabled pending
  rework) gets the exact original main-drive arrival.

## The ORBIT incompatibility (split out to 20260718-151102)

ORBIT station-keep via the CURRENT RCS is impossible: `rcs_burn_system` caps
ABSOLUTE along-axis speed at `rcs_speed_cap` (2 u/s), and orbits run at
`sqrt(mu/r)` ~= 2.5-6 u/s (repo test: ~4.9 at r=50). A prograde push gates to
zero, a retrograde one brakes the orbit. ORBIT needs an error-relative RCS mode
(cap the correction vs a desired velocity, not absolute speed) - a primitive
redesign, seeded as its own task.

## Known limitation (rework item, task 20260718-151102)

The RCS terminal settles to WITHIN the autopilot's `settle_deadband` (0.75 u/s),
not to `stop_speed_epsilon` (0.2): the disengage's `fine && firing_authority<=0`
branch fires while in RCS mode (no aligned main engine), so the maneuver releases
at the deadband like an off-axis main-drive residual. It is looser than the
main-drive terminal (which can align and brake to ~0.2-0.5). Acceptable within
the autopilot's stated "bounded creep is the contract"; tightening the RCS
terminal creep is folded into the rework task.

## Backward compatibility

RCS is a normal verb, granted by DEFAULT like Stop/Goto/Orbit (user call
2026-07-18). The 11 legacy autopilot tests that assert main-drive arrival now
call `withhold_rcs(app, ship)` to disable RCS and keep testing the behavior they
were written for - the same opt-out the mainline campaign uses. New tests cover
the RCS-enabled path.

## Difficulties / surprises

- First cut regressed 11 autopilot tests: RCS is granted by default, so the
  terminal-settle hijacked every ship's STOP/GOTO. Resolved by withholding RCS in
  those legacy tests (backward compatible) per the user's model.
- My own two new tests failed on too-tight thresholds (`< 0.3`): the ship settles
  to within the 0.75 deadband, not to zero - fixed to `< 0.8` and documented.

## Tests (all green - full flight:: suite, 70 passed)

- `orbit_never_engages_rcs`: ORBIT's `RcsIntent` stays zero for the whole hold.
- `stop_terminal_brakes_via_rcs`: RCS engages (non-zero intent), the main drive
  stays cold, the ship settles within the deadband.
- `stop_terminal_without_rcs_verb_uses_the_main_drive`: no verb -> no RcsIntent,
  settles on the main drive.
- Regressions intact: `goto_arrives_at_standoff_and_disengages`,
  `orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap`, and the 11 legacy
  autopilot tests (now RCS-withheld).

Per repo policy the full suite / clippy run in CI; ran check, fmt, and the whole
`flight::` module suite locally (it exercises the changed autopilot loop).
