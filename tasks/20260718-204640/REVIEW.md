# Review: Only trim ORBIT with RCS where it has authority over gravity

- TASK: 20260718-204640
- BRANCH: fix/orbit-rcs-gravity-authority

## Round 1

- VERDICT: APPROVE

Reviewed the diff (commit 4c92f00a) against master. Full `flight::` suite 75
passed. The fix adds one gate to `use_rcs_orbit`: the RCS trim takes an orbit
only when `orbit_gravity_accel (mu/r^2) < rcs_accel * 0.5`.

Independently verified the load-bearing arithmetic and the A/B:

- Gate separates the two regimes cleanly. Menu planetoid: `mu` from the ~85u
  geometric radius is ~43000, so `g` at the r=140 orbit is ~2.2 u/s^2, far
  above the `rcs_accel * 0.5 = 0.75` threshold -> RCS gated OFF, main drive
  holds (the pre-151102 path the user confirmed worked). Weak test well:
  `mu=1200` at r=50 gives `g=0.48 < 0.75` -> RCS still engages, so the trim
  feature is preserved where it is valid. Both are covered by tests that pass.
- `orbit_gravity_accel = well_data.mu / r_vec.length_squared()` is the correct
  local gravity magnitude (`r_vec` is the ship->well vector, |r_vec| = orbit
  radius), computed in the Orbit match arm and 0.0 elsewhere; `use_rcs_orbit`
  also gates on `is_orbit`, so non-orbit maneuvers are untouched, and
  `use_rcs_settle` (STOP/GOTO) is not on this path at all.
- A/B: `strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs` FAILED
  pre-fix (REPROEXIT=101 on the `saw_rcs` assertion - RCS engaged in the strong
  well) and passes post-fix. That assertion IS discriminating: it fails with the
  fix deleted.

Honesty about the test's scope (self-review caveat):

- The `saw_rcs` assertion is the real regression guard (RCS must not engage
  where it lacks authority). The paired `r_min > 0.6*plan` assertion is a
  coarse sanity bound, NOT discriminating - it passed pre-fix too, because a
  clean headless orbit is perturbation-stable (nothing there for the weak RCS
  to fail to correct). The actual in-game spiral needs the irregular asteroid
  gravity + two interacting ships + the insertion, which the headless rig does
  not model. So this branch proves the MECHANISM-level fix (the code path
  reverts to the known-good main drive for strong wells) but not the pixel-level
  "ships hold orbit," which is the outstanding by-eye check recorded in TASK.md.
- The fix is conservative by design: for the affected wells it restores the
  exact autopilot path that existed before 20260718-151102, which the user
  reported held orbit. So even absent a headless crash reproduction, the change
  cannot make strong-well orbits worse than the known-good baseline.

Design: the authority gate encodes the physical validity condition
(`rcs_accel` must have headroom over the pull) rather than blanket-disabling
the ORBIT trim, so the feature keeps working in weak wells. The 0.5 margin (2x
authority) is documented on the constant and sits with clear separation between
the two live cases (0.48 vs 2.2).

Non-blocking:

- OBSERVATION: a well whose `g` sits right at `rcs_accel*0.5` could flip the
  gate as the radius drifts (same no-hysteresis note as the parent task). Not a
  concern for the menu (far above) or the test wells (far below); revisit only
  if a scenario lands a ship near the boundary.
- FOLLOW-UP (already in TASK.md): eyeball the running menu to confirm the two
  haulers hold their orbit.

No BLOCKER/MAJOR/MINOR findings. Ship it.
