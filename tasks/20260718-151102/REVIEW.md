# Review: RCS error-relative mode for autopilot ORBIT station-keep

- TASK: 20260718-151102
- BRANCH: feature/rcs-error-relative

## Round 1

- VERDICT: APPROVE

Reviewed the diff (commit f2b5b360) against master. The full `flight::` suite
runs green (74 passed, 0 failed), no compiler warnings. The change adds an
optional `RcsReference(Vec3)` and rebases the RCS cap onto `velocity -
reference`; the autopilot writes the orbital velocity as the reference in the
ORBIT branch so RCS trims a fast orbit by a sub-cap delta.

Independently re-verified the two load-bearing claims (same-session review, so
this cross-check is required):

- **Zero reference == old behavior, byte-for-byte.** `along = (velocity -
  reference).dot(world_axis)` with `reference = Vec3::ZERO` is exactly the old
  `velocity.dot(world_axis)`. Confirmed every non-orbit path leaves the
  reference at zero: the player input layer never writes `RcsReference`; the
  autopilot writes `Vec3::ZERO` on every non-orbit tick; a fresh ship has no
  component (`unwrap_or(Vec3::ZERO)`). So the player fine-adjust and the
  STOP/GOTO terminal settle are provably unchanged - and the 60+ pre-existing
  flight tests (player caps, STOP settle, disengage) pass unmodified, which is
  the empirical half of the same claim.
- **The orbit trim is a stable proportional controller, correct sign.**
  Re-derived: command direction is `error = desired - v`; along the error axis
  `along = (v - desired)·error_dir = -|error| < 0`, so the gate is
  `(cap + |error|)/taper` -> clamps to full push toward `desired`, and flips to
  brake on overshoot. Convergent, not divergent. The pre-existing
  `orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap` still holds the
  ring for a full lap - now exercising the RCS trim path - which is the
  integration proof that the trim is stable, not merely present.

Test quality (do they fail if the fix is reverted?):

- `rcs_relative_cap_trims_a_fast_moving_reference`: with the reference term
  reverted, the with-reference case gates to zero (v = 5 > cap), so `v.x`
  stays 5 and the `> 5.1` assertion fails. Meaningful. The control arm (no
  reference -> no push) pins the other side. flight_app has no gravity, so
  `v.x` is driven only by RCS - the assertion isolates the primitive.
- `orbit_engages_rcs_only_to_trim_a_sub_cap_residual`: pins the new contract
  from both ends - no RCS while spinning up from rest (residual > cap), and
  when trimming, `reference > cap` and `|v - reference| <= cap`. The
  `reference > cap` assertion is exactly what the old absolute cap could never
  satisfy, so it would fail on the pre-change code. This correctly REPLACES
  `orbit_never_engages_rcs`, whose asserted non-behavior this task
  intentionally reverses - not a weakened test, an inverted contract.
- `orbit_rcs_reference_clears_on_disengage`: guards the off-ramp
  (`shared-primitive-clear-on-handoff`); fails if the observer clear is
  dropped.

Design: one system with a reference term (rather than a second sibling burn or
a mode enum on RcsIntent) is the right call - it keeps the two modes provably
consistent and makes "player/settle" the natural `reference = 0` case.
Cleanup is handled on both the per-tick path and the disengage observer.

Non-blocking observations (left to discretion / follow-up):

- NIT (flight.rs, orbit trim handoff): the main-drive/RCS handoff at
  `error_speed == cap` has no hysteresis. The orbit-hold test stays in-band so
  there's no gross chatter, but a boundary flip is possible. Documented as a
  known limitation in NOTES.md; a dead-band is a cheap follow-up if a playtest
  shows it. Not blocking.
- OBSERVATION: whether the RCS-trimmed orbit *feels* as tight as the old
  pure-main-drive hold can only be judged in a live playtest - the headless
  test only guarantees the radius/speed band. This is inherent to a flight-feel
  change, same caveat as the rest of the RCS family. Worth a by-eye pass when
  convenient; not a blocker on the primitive being correct.

No BLOCKER/MAJOR/MINOR findings. The reference-defaults-to-zero identity makes
the no-regression guarantee structural rather than incidental. Ship it.
