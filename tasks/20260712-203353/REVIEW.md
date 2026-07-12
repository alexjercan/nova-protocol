# Review: Sticky-from-acquisition lock (B5)

- TASK: 20260712-203353
- BRANCH: feature/sticky-focused-lock

## Round 1

- VERDICT: REQUEST_CHANGES

Independent verification (shared-session blind-spot guard): traced how the lock
is consumed beyond combat. The lock (`SpaceshipPlayerTargetLock`) is ALSO the
GOTO / torpedo NAV DESIGNATOR - `input/player.rs:848` engages
`AutopilotAction::Goto { target }` from it - and you switch a nav designation by
AIMING at a different beacon/asteroid (the lock is aim-driven). Confirmed the
CTRL+scroll cycle set (`rank_ship_candidates`) is HOSTILE SHIPS ONLY, so nav
bodies are not in it.

- [x] R1.1 (MAJOR) input/targeting.rs `update_spaceship_target_input` - the new
  `held` gate is UNIVERSAL: it holds ANY current lock, including an asteroid or
  beacon. That breaks nav designation - once you aim-lock asteroid X for a GOTO,
  aiming at asteroid Y no longer re-designates (held on X), and CTRL+scroll
  cannot help (it cycles only hostile ships). The player is stuck on the first
  nav target until it leaves range. The reported problem was a TORPEDO stealing
  a COMBAT (ship) lock; make the stickiness ship-only: `held` should require the
  current lock be a ship (`is_ship` in the candidate tuple), so ships stick
  (torpedo can't steal) while asteroids/beacons stay aim-driven (nav
  designation unchanged). Add a test that a non-ship lock is NOT sticky (aiming
  re-designates).
  - Response: Fixed. `held` now requires the candidate's `is_ship` flag, so only
    ship locks are sticky; asteroids/beacons stay aim-driven and nav
    re-designation is unchanged. Added
    `a_non_ship_lock_is_not_sticky_so_nav_re_designates`. 44 targeting tests pass.

## Round 2

- VERDICT: APPROVE

R1.1 verified fixed on the new diff: `held` gates on `is_ship`; the new nav test
proves a non-ship lock re-designates by aiming, while
`a_held_lock_is_not_stolen_by_a_closer_body` still proves a ship lock sticks.
The residual "switch from a held ship lock to a nav target by aiming" gap is
recorded as a playtest item (aim-away-release remedy), not built here. No new
findings. Branch ready to land.

Notes: the ship-only fix still leaves one narrow gap - switching FROM a held
ship lock TO a nav target by aiming (you would finish/leave the fight, or the
ship leaves range, first). That is the spike's open "feels stuck?" question and
its aim-away-release remedy; keep it a playtest item, do not build it here. The
core reported fix (torpedo not stealing a ship lock) and all nav-to-nav
designation are delivered by the ship-only gate.

Everything else is sound: first-acquisition / death-reacquire / CTRL+scroll all
fall out correctly, the pin-expiry test was correctly rewritten for sticky
semantics, and the new held-not-stolen test is delivery-guarded. 43 targeting
tests + both autopilots pass - but the suite did not catch R1.1 because no test
exercises an asteroid/beacon nav re-designation while locked.
