# Menu-scene ships crash the asteroid and cannot hold orbit (RCS-trim regression?)

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.7.0, bug, flight

## Report

Playtest 2026-07-18: in the main menu scene (two spaceships orbiting an
asteroid as ambience), the two ships now crash INTO the asteroid and can no
longer hold their orbit. This worked before.

## Prime suspect

The error-relative RCS ORBIT trim (task 20260718-151102, landed 4455da4b). That
change made the autopilot hand ORBIT station-keeping to the RCS primitive once
the residual `|v - v_orbit|` drops below the fine-adjust cap (2 u/s): it writes
`RcsReference = desired` + a proportional `RcsIntent`, and ZEROES the main-drive
demand while trimming. If the RCS trim cannot supply enough correction (its cap
is a gentle 2 u/s and `rcs_accel` is ~1.5 u/s^2), a ship perturbed toward the
ring may be unable to recover and spirals into the asteroid - whereas the old
main-drive hold had full authority.

That task's own review explicitly flagged this: "whether the RCS-trimmed orbit
FEELS as tight as the old pure-main-drive hold can only be judged in a live
playtest - the headless test only guarantees the radius/speed band." This is
that playtest signal.

## Why the headless test missed it

`orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap` (flight.rs) uses a
single ship around a bare `GravityWell` point mass with NO collider, from a
clean near-rest insertion, and only asserts the radius stays in a wide band
(0.8-1.25 x plan) for one lap. The menu scene differs: an ASTEROID with a real
collider (so "drift inward" becomes "crash", not a slow band excursion), TWO
ships (possible interaction / shared-handle effects), and a longer runtime.

## Steps (diagnose first, then fix)

- [x] Reproduce: the two menu ambience ships are `menu_waystation` haulers
  (nova_assets/src/scenario.rs) orbiting `menu_planetoid` at r=140; they grant
  `FlightVerb::Rcs` by default (no DisableVerb on their AI controller). The well
  derives `mu` from the GEOMETRIC asteroid radius (~85u), so `mu ~= 43000` and
  the local gravity at r=140 is `mu/r^2 ~= 2.2 u/s^2`, ABOVE `rcs_accel` (1.5).
- [x] Instrument headlessly:
  `strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs` (flight.rs)
  spawns a menu-strength well (surface gravity 6 at 85u) and orbits a ship at
  r=140. It FAILED on the pre-fix code (`saw_rcs` true - RCS engaged despite
  lacking authority, the regression) and passes after the fix.
- [x] Decide the fix: the "clear authority" gate (candidate 1). `use_rcs_orbit`
  now also requires `orbit_gravity_accel < rcs_accel * RCS_ORBIT_GRAVITY_AUTHORITY`
  (0.5), so the RCS trim only takes an orbit where its push has 2x headroom over
  the inward pull. Strong wells (the menu) stay on the full-authority main
  drive, exactly as before the RCS trim landed; weak wells keep the trim (the
  r=50 mu=1200 tests, g=0.48 < 0.75, still engage RCS).
- [x] Flight suite green (75 passed). By-eye menu confirmation is still
  OUTSTANDING - it needs the running game (I cannot launch it headless); the
  headless regression guard proves the mechanism (RCS no longer engages in the
  menu-strength well), but the visual "ships hold orbit" should be eyeballed.

## Notes

Suspect commit: 4455da4b (task 20260718-151102). Design record:
tasks/20260718-151102/NOTES.md (the `RcsReference` mechanism + the documented
no-hysteresis limitation at the trim/main-drive handoff). Relevant code:
`autopilot_system` ORBIT branch + `use_rcs_orbit` gate, `rcs_burn_system`
(flight.rs). If confirmed, this is a `diagnostic-first` + `render-output-eyeball`
bug: reproduce the exact scenario before theorizing, and the final proof is
by-eye in the running menu.

## Close-out (2026-07-18)

Root cause: the error-relative ORBIT trim (task 20260718-151102) handed
station-keeping to the RCS primitive whenever the orbital-velocity residual
dropped below the fine-adjust cap, and ZEROED the main drive while trimming.
That is safe only where RCS can actually hold the orbit. The menu planetoid is
a STRONG well (mu derived from the ~85u geometric radius, ~43000), so the
inward pull at the r=140 orbit (~2.2 u/s^2) exceeds `rcs_accel` (1.5): once RCS
took over and the main drive spooled down, gravity overwhelmed the trim and the
ships spiralled into the rock. The r=50 mu=1200 headless test never caught it
because that well is weak (g=0.48 < rcs_accel).

Fix: gate `use_rcs_orbit` on RCS having CLEAR authority over local gravity -
`orbit_gravity_accel (mu/r^2) < rcs_accel * RCS_ORBIT_GRAVITY_AUTHORITY (0.5)`.
Chosen over fully disabling the ORBIT trim because the trim is correct and
valuable in weak wells (it keeps working there); the gate encodes the physical
validity condition rather than a blanket off-switch. `orbit_gravity_accel` is
computed in the Orbit match arm from `well_data.mu` and `r_vec`.

A/B proof: `strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs`
FAILED pre-fix (REPROEXIT=101, `saw_rcs` assertion) and passes post-fix; the
weak-well trim tests still pass. Full flight:: suite 75 passed.

Follow-up (not blocking): by-eye confirmation in the running main menu that the
two haulers hold their orbit - the headless guard proves RCS no longer engages
in that well, but the visual is worth a glance. Design note in
tasks/20260718-151102/NOTES.md already flagged this handoff-authority gap as a
known limitation; this closes it with an explicit gate.
