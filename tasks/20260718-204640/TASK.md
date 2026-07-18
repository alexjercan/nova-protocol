# Menu-scene ships crash the asteroid and cannot hold orbit (RCS-trim regression?)

- STATUS: OPEN
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

- [ ] Reproduce: identify the menu ambience scenario + which builder spawns the
  two orbiting ships (likely a craft-built scenario; grep the menu scene setup).
  Confirm the ships grant `FlightVerb::Rcs` (they must, for the trim to engage).
- [ ] Instrument headlessly: extend the orbit test (or add one) with a well that
  has an asteroid-like radius and a ship starting with a small inward
  perturbation, and assert it does NOT lose the ring over many laps. Make it
  FAIL on the current code first (prove the regression) before fixing.
- [ ] Decide the fix. Candidates, in rough order:
  - add hysteresis / only trim when the residual is well inside the cap, so a
    large perturbation stays on the main drive (which has authority) instead of
    a too-weak RCS;
  - or keep the main drive available for the radial (ring-error) component and
    let RCS only trim the tangential residual;
  - or gate ORBIT-RCS off when the radial ring error exceeds a small bound (RCS
    only for a near-perfect orbit), falling back to the main drive.
  - Escape hatch: if error-relative ORBIT trim is not worth the complexity for
    the ambience, disable the ORBIT branch (keep the STOP/GOTO settle) and
    reopen when it can be tuned with a real playtest.
- [ ] Verify the menu scene holds orbit again (by-eye) and the flight suite
  stays green.

## Notes

Suspect commit: 4455da4b (task 20260718-151102). Design record:
tasks/20260718-151102/NOTES.md (the `RcsReference` mechanism + the documented
no-hysteresis limitation at the trim/main-drive handoff). Relevant code:
`autopilot_system` ORBIT branch + `use_rcs_orbit` gate, `rcs_burn_system`
(flight.rs). If confirmed, this is a `diagnostic-first` + `render-output-eyeball`
bug: reproduce the exact scenario before theorizing, and the final proof is
by-eye in the running menu.
