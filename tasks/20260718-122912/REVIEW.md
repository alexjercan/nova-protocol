# Review: RCS player input (SHIFT + mouse/scroll fine-adjust)

- TASK: 20260718-122912
- BRANCH: feat/rcs-input

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commits ee59c14f + 1393b365 vs master (799e75c6). The feature delivers
the Goal: SHIFT enters a verb-gated fine-adjust mode, mouse (XZ) + scroll (Y)
drive `RcsIntent`, the heading and camera view freeze, release restores flight.
The design is a clean reuse of the `Autopilot`-presence gating pattern, and the
implementer already caught a genuine drift bug pre-review (the rate integrator).

Independently verified (shared-session blind-spot guard):
- The `point_rotation_update_system` (bcs) applies `PointRotationInput` as a
  per-frame delta EVERY frame - so the drift fix (zeroing, not skipping) is
  necessary and correct; merely skipping would drift the view on entry.
- A failing `Single` param skips the system/observer (SystemParamValidationError
  is ignored by the executor), so the `Without<RcsActive>` helm gate freezes the
  heading correctly, and the observers safely no-op when there is no player ship.
- `insert_flight_control` (flight.rs:530) inserts `RcsIntent` on every player
  ship, so the release path's `&mut RcsIntent` requirement is satisfied in
  production.
- Ran the 8 RCS tests + input:: (164) + camera_controller:: (13) suites: green,
  no existing test weakened.

The blocker to APPROVE is test coverage of two paths that would NOT fail if the
code were reverted - i.e. they are currently unverified.

- [ ] R1.1 (MAJOR) crates/nova_gameplay/src/camera_controller.rs:750 - the
  drift-freeze fix (zeroing `PointRotationInput` while `RcsActive`) has no test.
  Deleting the fix leaves every camera/RCS test green, so the exact bug it fixes
  (view drift from a stale rate) is unguarded against regression. Add a focused
  test: build the camera input rig + `EnhancedInputPlugin`, spawn a player ship
  with `RcsActive`, seed the rig's `PointRotationInput` non-zero (mouse-moving-
  at-press), inject a `MouseMotion` message, update, and assert
  `PointRotationInput` (or `PointRotationOutput`) is unchanged/zeroed - a test
  that fails with the fix reverted.
  - Response: Added `rcs_zeroes_the_rig_rate_so_the_view_does_not_drift`
    (camera_controller.rs tests): seeds the active rig's `PointRotationInput`
    non-zero, injects a `MouseMotion` message while a player ship is `RcsActive`,
    and asserts the rate is zeroed. Fails with the fix reverted (the stale rate
    or `fire.value` survives).
- [x] R1.2 (MINOR) crates/nova_gameplay/src/input/targeting.rs:1382,1420 - the
  scroll-Y-during-RCS branch (nudge `RcsIntent.y`, skip the component step) is
  untested. The accumulate mechanism is proven by the mouse-aim test, but the
  branch wiring (scroll drives Y AND does not cycle the component lock while
  `RcsActive`) is not. Add a test in the flight-rig harness: with `RcsActive`,
  inject a scroll and assert `RcsIntent.y` moved and `ComponentLock` did NOT
  change; without `RcsActive`, the same scroll steps the component and leaves
  `RcsIntent` alone.
  - Response: Added `rcs_scroll_drives_the_vertical_axis_only_while_active`
    (player.rs tests): scroll outside RCS leaves `RcsIntent.y` at zero (it
    cycles instead); with `RcsActive` the same scroll raises Y.
    `on_component_cycle_next` made `pub(crate)` so the test can observe it.
    Fails with the branch reverted.
- [x] R1.3 (NIT) crates/nova_gameplay/src/input/player.rs:1032
  (`on_rcs_modifier_released`) - the enter observer needs only `Entity` but the
  release observer needs `&mut RcsIntent`, so a player ship missing `RcsIntent`
  could enter RCS but never clear `RcsActive` (helm/view stuck frozen).
  Guaranteed safe today (insert_flight_control always inserts it), but the
  asymmetry is a latent trap; `Option<&mut RcsIntent>` on release (zero it if
  present, always remove `RcsActive`) removes the coupling.
  - Response: Changed the release observer to `Option<&mut RcsIntent>` - it now
    always removes `RcsActive` and zeroes the intent only when present.
- [ ] R1.4 (NIT) crates/nova_gameplay/src/input/player.rs (`on_rcs_aim`) - the
  mouse-Y -> ship-Z sign is convention-dependent (the camera rig negates its
  mouse_motion; RCS does not). It may need a playtest flip so "push forward =
  move forward". Documented as tunable; just calling it out so a playtest checks
  it deliberately.
  - Response: Left as-is (NIT) - a deliberate playtest call, documented in
    NOTES.md and the code comment as tunable. No code change.

### Round 1 resolution

- VERDICT: APPROVE

R1.1 (MAJOR) and R1.2 (MINOR) now have regression-guarding tests that fail if
the code is reverted; R1.3 (NIT) is fixed. R1.4 (NIT) is an intentional
playtest-time decision, left documented. All 10 RCS tests pass (2 new); the
input:: and camera_controller:: suites stayed green. No open BLOCKER/MAJOR.
