# Review: Spaceship rendering is twitchy at high velocity (re-test)

- TASK: 20260710-231931
- BRANCH: fix/ship-twitch-retest

## Round 1

- VERDICT: REQUEST_CHANGES

Scope verified: verification-only task; one regression test, spike doc fix
record, honest TASK.md close-out (visual feel explicitly deferred to the
umbrella's user playtest instead of being claimed). Step rewrite from the
vacuously-green "straight-line burn" to the cross-velocity regime is
correct per the 20260711-103527 diagnostic, and the A/B evidence (4.26
rad/s unfixed vs ~0 fixed) is recorded with the exact rig. flight:: module
run by the reviewer: 57/57 green.

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/flight.rs:2784-2812
  (cross_velocity_burn_keeps_the_hull_steady_at_high_speed) - the test's
  only assertion is that NOTHING happens (max spin ~0), so it passes
  vacuously if the burn never fires: should the FlightIntent -> manual
  burn -> ThrusterSectionInput seam change (component rename, marker
  requirement, input binding gate), the engine goes silent and the
  regression keeps passing while guarding nothing. Add a delivery
  assertion pinning that thrust actually flowed, e.g.
  `assert!(velocity_of(&app, ship).z < -20.0)` after the run (the -Z burn
  must have accelerated the ship), mirroring how the off_center test's
  `pulled > 0.4` arm self-proves the engine fired.
  - Response: fixed - delivery guard added exactly as suggested
    (velocity_of(...).z < -20.0 with a comment naming R1.1 and the
    vacuous-pass rationale); test re-run green with the guard active.

## Round 2

- VERDICT: APPROVE

R1.1 verified resolved in commit 22bd685: the delivery guard asserts the
-Z burn accelerated the ship before the spin bound is trusted, and the
guard is placed BEFORE the spin assertion so a silent engine fails loudly
with its own message. flight:: 57/57 re-confirmed by the reviewer. No new
findings; the branch delivers the task's verification goal.
