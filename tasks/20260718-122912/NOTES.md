# RCS player input - design / fix record

Task: 20260718-122912. Spike: tasks/20260718-122508/SPIKE.md. Depends on the RCS
core primitive (20260718-122906, CLOSED).

## What shipped

Holding SHIFT enters an RCS fine-adjust translation mode: the mouse (XZ plane)
and scroll (Y) drive the ship-local `RcsIntent`, the heading and camera view
freeze, and release restores normal flight.

- `RcsActive` marker component on the ship root (flight.rs), reflected +
  registered. It is the single modal gate the helm, camera and scroll all read -
  the RCS analogue of `Autopilot` presence.
- Input (input/player.rs): two new actions in `flight_input_rig` -
  `RcsModifierInput` (SHIFT, plain Down) and `RcsAimInput` (mouse motion, `Vec2`,
  shares the mouse_motion source with the camera rig, `consume_input: false`).
- Modal observers (player.rs):
  - `on_rcs_modifier_start` (Start): if `ship_grants_verb(Rcs)`, insert
    `RcsActive` and `remove::<Autopilot>()` (entering RCS is a flight input).
  - `on_rcs_modifier_released` (Complete): remove `RcsActive`, zero `RcsIntent`.
  - `on_rcs_aim` (Fire): while `RcsActive`, accumulate the mouse delta into
    `RcsIntent.x` (strafe) and `.z` (forward/back) via the shared
    `accumulate_rcs_axis` helper.
- Scroll -> RCS vertical: `on_component_cycle_next/prev` (targeting.rs) branch on
  `RcsActive` - nudge `RcsIntent.y` instead of stepping the component lock.
- Helm freeze: `Without<RcsActive>` added to the ship `Single` in
  `update_controller_target_rotation_torque` (player.rs) - the mouse stops
  driving the heading, exactly as `Without<Autopilot>` already does.
- View freeze: `on_rotation_input` (camera_controller.rs) ZEROES the rig rate
  (`PointRotationInput`) when the player ship is `RcsActive`, so the mouse stops
  orbiting the camera. It zeroes rather than merely skips because the bcs
  `point_rotation_update_system` INTEGRATES the rate every frame - a stale
  nonzero rate (mouse moving at the instant SHIFT was pressed) would otherwise
  keep drifting the view/heading. Held at zero, the rig quat stays put, so the
  helm resumes on exit with no snap (no re-seed).
- `accumulate_rcs_axis(current, delta)` pure helper in flight.rs: integrate +
  clamp to `[-1,1]`, shared by the mouse and scroll paths.

## Design decisions (control feel)

- **Held-direction virtual joystick (spike Q1).** Mouse/scroll INTEGRATE into a
  persistent `RcsIntent` offset that stays put when the input stops; the pilot
  pushes to build it and pulls back to null it. Release zeroes it. The offset is
  invisible this task - the diegetic sphere (task 20260718-122923) will show it.
- **Freeze the VIEW, not just the heading (spike Q4).** Because the rig is frozen
  (not free-looking), its quat stays at the held heading, so the helm resumes on
  exit with NO snap - so, unlike the autopilot, RCS needs no exit re-seed
  (contrast camera_controller.rs `on_autopilot_disengaged`).
- **Deflection = acceleration, terminal = cap; no primitive change.** At the 2
  u/s cap the review-R1.1 "partial deflection reaches full cap" distinction is
  immaterial, so `rcs_burn_system` is untouched.

## Difficulties / surprises

- The scroll repurpose was cleaner as an in-place branch inside the existing
  `on_component_cycle_next/prev` observers (one scroll event, one decision) than
  as separate vertical actions + gating the cycle - amended the plan step to
  match (`half-ticked-compound-steps`).
- Test event API: this Bevy (0.19) uses the MESSAGE API - `World::write_message`
  (not `write_event`) and `MessageWriter` (not `EventWriter`); mouse motion is a
  `MouseMotion` message that flows to `AccumulatedMouseMotion`, which
  bevy_enhanced_input reads. The aim test injects motion that way and it lands in
  one `update`.
- Input tests need `app.finish()` + `app.cleanup()` before spawning the rig
  (`bei-app-finish-in-tests`), and assert after each gesture step
  (`assert-each-gesture-step`).

## Tests (all green)

- flight.rs: `accumulate_rcs_axis_integrates_and_clamps_to_the_unit_range`.
- input/player.rs (real rig + EnhancedInputPlugin):
  `rcs_shift_gesture_enters_exits_and_disengages_autopilot` (enter -> RcsActive +
  autopilot gone + ship excluded from the helm authority query; exit -> gone +
  RcsIntent zeroed), `rcs_shift_is_gated_by_the_controller_verb`,
  `rcs_mouse_motion_accumulates_intent_only_while_active`.
- Added in review round 1 (both fail if the guarded code is reverted):
  `rcs_zeroes_the_rig_rate_so_the_view_does_not_drift` (camera - pins the
  drift-freeze) and `rcs_scroll_drives_the_vertical_axis_only_while_active`
  (player - pins the scroll-Y branch). Also made `on_rcs_modifier_released` take
  `Option<&mut RcsIntent>` so `RcsActive` always clears on exit (review R1.3).
- No regressions: input:: (164) and camera_controller:: (13) suites stayed
  green after the shared-observer signature changes.

Per repo policy the full suite / clippy run in CI; ran check, fmt, the new tests,
and the two touched-module suites locally.
