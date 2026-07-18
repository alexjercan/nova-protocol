# Review: autopilot RCS terminal-settle

- TASK: 20260718-122932
- BRANCH: feat/rcs-autopilot

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commit 130295c1 vs master (c65f0432). The gate is right and the ORBIT
exclusion is sound, but there is a real post-disengage drift bug.

Independently verified:
- Gate `desired.length() <= stop_speed_epsilon (0.2)` correctly excludes ORBIT
  (desired = orbital speed ~2.5-6 u/s) and the GOTO approach leg (desired >=
  min_approach_speed 1.5); admits STOP and GOTO/GotoPos inside the standoff
  (desired == 0). `velocity < rcs_cap` is the right precondition. Confirmed by
  `orbit_never_engages_rcs` and the still-green ORBIT ring-hold test.
- The verb check mirrors `ship_grants_verb` (live controller with a PD, not
  withholding `Rcs`).
- The 11 legacy tests genuinely still test the main drive: `withhold_rcs` turns
  RCS off so `use_rcs` is always false and the main-drive path runs unchanged -
  they are not vacuous (they'd fail if the main-drive path broke).
- `stop_terminal_brakes_via_rcs` would fail if the RCS branch were reverted
  (RcsIntent would stay zero -> `saw_rcs` false).

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/flight.rs (`autopilot_system` +
  `on_autopilot_removed_cool_engines`:1957) - the residual `RcsIntent` is NOT
  cleared when the autopilot disengages. On the completing tick `use_rcs` is
  still true, so `RcsIntent` is left at the brake value (~-velocity direction);
  the disengage observer zeros the THRUSTERS but not `RcsIntent`. Because
  `rcs_burn_system` acts on ANY nonzero `RcsIntent` + verb (it is not
  autopilot-gated), it keeps pushing after arrival: velocity crosses zero and
  accelerates to the RCS cap (~2 u/s) in the reverse direction - the ship drifts
  away instead of resting. Affects the player too (a completed GOTO leaves the
  ship drifting). Fix: zero `RcsIntent` on `On<Remove, Autopilot>` - add a
  `Query<&mut RcsIntent>` to `on_autopilot_removed_cool_engines` (it already owns
  the disengage cleanup) and set it to `Vec3::ZERO` for the removed ship. Add a
  test that runs PAST disengage and asserts the ship stays at rest (it fails
  today).
  - Response: FIXED - `on_autopilot_removed_cool_engines` now zeros `RcsIntent`
    for the removed ship. Added `rcs_settled_autopilot_leaves_the_ship_at_rest_after_disengage`,
    which runs 400 ticks past release and asserts no drift; it fails without the
    clear (the ship would accelerate toward the cap).
- [x] R1.2 (MINOR) crates/nova_gameplay/src/flight.rs (`orbit_never_engages_rcs`,
  `stop_terminal_without_rcs_verb_uses_the_main_drive`) - both assert `RcsIntent`
  stays zero, which is ALSO true if the RCS branch is reverted, so they do not
  fail on a deleted feature (regression guards, not delivery proofs). Acceptable
  as guards, but strengthen `stop_terminal_without_rcs_verb` with a paired
  positive: assert that the SAME ship WITH the verb granted DOES write RcsIntent,
  so the no-verb zero is proven to be the gate, not a dead path.
  - Response: The paired positive already exists: `stop_terminal_brakes_via_rcs`
    is the SAME STOP-from-1.5 setup WITH the verb granted and asserts RcsIntent
    goes non-zero - so the no-verb zero is proven to be the gate, not a dead path.
    Left as the pair (no code change).

### Round 1 resolution

- VERDICT: APPROVE

R1.1 (MAJOR) fixed with a test that fails without the clear; R1.2 covered by the
existing positive/negative pair. Full `flight::` suite green (71 passed).
