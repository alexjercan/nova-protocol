# RCS player input: SHIFT-held mouse/scroll translation with rotation-authority freeze

- STATUS: CLOSED
- PRIORITY: 4
- TAGS: v0.7.0, feature, input, spike

## Goal

Wire player input to the RCS primitive (task 20260718-122906) so holding SHIFT
enters a fine-adjust translation mode:

- While SHIFT is held on an RCS-capable ship (gate on `ship_grants_verb(Rcs)`),
  enter RCS mode. Entering disengages any engaged autopilot (consistent with
  "any flight input removes Autopilot").
- Take rotation authority the way the autopilot does: freeze the helm at its
  current heading and repurpose the mouse from aiming to translation. Extend the
  `Without<Autopilot>` gate on `update_controller_target_rotation_torque`
  (input/player.rs:311) so RCS-active also stops feeding the mouse to the helm.
- Held-direction (virtual joystick) mapping into `RcsIntent`: mouse Y ->
  ship-local +/-Z (forward/back), mouse X -> +/-X (strafe), scroll -> +/-Y
  (up/down). Push-and-hold builds toward the cap; release clears `RcsIntent` and
  restores mouse-to-helm. Settle the origin/deadzone/curve during implementation.

## Design decisions (from planning)

- **Control model:** persistent virtual-joystick accumulator. While RCS is
  active, mouse motion (and scroll) INTEGRATE into `RcsIntent` (ship-local),
  clamped per-axis to `[-1, 1]`, and the offset PERSISTS when the mouse stops
  (held-direction, spike Q1). SHIFT release zeroes `RcsIntent`. The offset is
  invisible this task; the diegetic sphere (task 20260718-122923) visualizes it.
- **Deflection = acceleration, terminal = cap (no primitive change).** Keep the
  landed `rcs_burn_system` as-is: a bigger push accelerates faster to the same
  `cap` (2 u/s). At that small cap the "partial deflection reaches full cap"
  distinction (review R1.1) is immaterial - full-cap IS fine-adjust speed - so
  scaling the cap by `|cmd|` is deliberately NOT done. Revisit only if the cap
  is ever raised.
- **Freeze the VIEW, not just the heading (spike Q4: "mouse fully repurposed to
  translation").** During RCS the mouse must not orbit the camera either, so the
  camera rig freezes too. Because the rig stays put at the (also frozen) heading,
  NO re-seed is needed on exit (unlike autopilot disengage,
  camera_controller.rs:361 `on_autopilot_disengaged`).

## Steps

- [x] Add an `RcsActive` marker component on the ship root in
  `crates/nova_gameplay/src/flight.rs` (next to `RcsIntent`), reflected +
  registered. Present = the player is holding RCS mode; it is the gate other
  systems read (mirrors how `Autopilot` presence gates rotation authority).
- [x] Add the input actions in `crates/nova_gameplay/src/input/player.rs`
  alongside the existing flight actions (~line 520) and bind them in
  `flight_input_rig` (~line 574), `consume_input: false` like the others:
  - `RcsModifierInput` (`bool`, plain Down): `KeyCode::ShiftLeft` +
    `ShiftRight` (+ a gamepad button, e.g. `LeftTrigger2`/a free one). SHIFT is
    otherwise unused (only CTRL is taken, for radar). Read as a held modifier via
    the existing `action_held`/`TriggerState` pattern where needed.
  - `RcsAimInput` (`Vec2`, `Binding::mouse_motion()` with a `Scale`): the mouse
    delta for the XZ plane. A second binding of mouse_motion (the camera rig also
    binds it) is fine with `consume_input: false`.
  - (ADAPTED: no separate vertical actions needed - the existing
    `ComponentCycleNext/Prev` scroll actions are reused, with the Y nudge done
    inside their observers when RCS is active; see the repurpose step below.)
- [x] Modal state observers (player.rs, near the autopilot observers ~781),
  all pause-gated like the existing ones:
  - `On<Start<RcsModifierInput>>`: if `ship_grants_verb(ship, FlightVerb::Rcs,
    &q_verbs)` (player.rs:775), insert `RcsActive` on the player ship and
    `remove::<Autopilot>()` (entering RCS is a flight input - consistent with
    `on_flight_burn_input`, player.rs:724). No-op if the verb is withheld.
  - `On<Complete<RcsModifierInput>>` and `On<ActionCancel<RcsModifierInput>>`:
    remove `RcsActive` and set the ship's `RcsIntent` to `Vec3::ZERO`.
- [x] Accumulate mouse/scroll into `RcsIntent` while active (player.rs
    observers), each gated on the player ship having `RcsActive`:
  - `On<Fire<RcsAimInput>>`: `RcsIntent.x += dx * sens` (strafe, ship +X),
    `RcsIntent.z += dy * sens` with the sign chosen so pushing the mouse
    forward/up moves the ship forward (ship -Z). Clamp each to `[-1, 1]`.
  - `On<Fire<RcsVerticalUpInput>>` / `Down`: `RcsIntent.y += step` / `-= step`,
    clamped. (scroll notch = a discrete Y nudge to the offset.)
  - Factor the integrate-and-clamp into a small pure helper
    (`accumulate_rcs_axis(current, delta) -> f32`) so it can be unit-tested
    without the input stack.
- [x] Repurpose scroll while RCS is active (ADAPTED to a cleaner in-place
  branch): inside `on_component_cycle_next/prev` (targeting.rs:1366/1387), when
  the player ship has `RcsActive`, nudge `RcsIntent.y` by +/-`RCS_SCROLL_STEP`
  (via `accumulate_rcs_axis`) and `continue` instead of stepping the component
  lock - one scroll event, one observer decides, exactly the "modifier decides"
  precedent (CTRL layer, player.rs:566-573). No separate vertical actions.
- [x] Freeze the helm: add `Without<RcsActive>` to the `spaceship` `Single`
  filter in `update_controller_target_rotation_torque` (player.rs:311), so while
  RCS is active the mouse stops driving the hull heading (the exact mechanism the
  `Without<Autopilot>` gate already uses).
- [x] Freeze the camera view: in `on_rotation_input`
  (camera_controller.rs:727, the `On<Fire<CameraInputRotate>>` observer), add a
  query for the player ship carrying `RcsActive` and early-return when present,
  so the mouse stops orbiting the camera during RCS. Because the rig quat then
  stays equal to the frozen heading, no exit re-seed is needed (contrast
  `on_autopilot_disengaged`). Verify by test that the helm target does not jump
  on release.
- [x] Register the new `RcsActive` type and add any new observers to the input
  plugin build (wherever `on_autopilot_stop_input` etc. are added as observers).
- [x] Tests (headless, `bevy_enhanced_input`; follow the input-test harness at
  player.rs:1637 - `app.finish()` + `app.cleanup()` before spawning
  `flight_input_rig`, per `bei-app-finish-in-tests`; assert after EACH gesture
  step per `assert-each-gesture-step`):
  - Gesture: press SHIFT on an RCS-granting ship -> `RcsActive` inserted and any
    `Autopilot` removed; release -> `RcsActive` gone and `RcsIntent == ZERO`.
  - Verb gate: press SHIFT on a ship whose controller withholds `Rcs` -> no
    `RcsActive`, no `RcsIntent` change.
  - Accumulation: with `RcsActive` present, drive the aim/scroll path (inject
    motion, or call the observer/helper directly) and assert `RcsIntent`
    accumulates on the right axes and clamps at `[-1, 1]`.
  - Helm freeze: with `RcsActive` present, a mouse delta does not change the
    controller's `ControllerSectionRotationInput`; on release it resumes.
  - Pure helper: `accumulate_rcs_axis` integrates and clamps correctly.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Q1 held-direction, Q4 freeze-heading).
Depends on the RCS core primitive (task 20260718-122906, CLOSED): `RcsIntent`,
`RcsSpeedCap`, `rcs_burn_system`, `FlightVerb::Rcs`, `ship_grants_verb`
(player.rs:775). Input via bevy_enhanced_input.

Reference points verified during planning:
- Flight input rig + action/binding pattern: player.rs:520 (action structs),
  574-708 (`flight_input_rig`), mouse_wheel bindings 679-703.
- Autopilot activation observer + `ship_grants_verb` gate + "any input
  disengages": player.rs:781 (`on_autopilot_stop_input`), 724
  (`on_flight_burn_input`), 775 (`ship_grants_verb`).
- Rotation-authority freeze (the `Without<Autopilot>` gate): player.rs:311
  (`update_controller_target_rotation_torque`).
- Held-modifier read pattern (`action_held`/`TriggerState`): camera_controller.rs:762.
- Camera mouse->rig observer to freeze: camera_controller.rs:727
  (`on_rotation_input`); re-seed precedent (not needed here): 361.
- Input test harness (finish/cleanup, spawn rig, press keys): player.rs:1637.

Lessons applied: `modal-input-observer-dispatch` (modifier as a plain action
read in observers, not a binding Chord), `assert-each-gesture-step`,
`bei-app-finish-in-tests`, `two-clocks` (RcsIntent written at render rate is
consumed in FixedUpdate - a one-frame lag is fine for a held control).

Feel decision inherited from the core primitive (review R1.1 on -122906):
resolved above - deflection sets acceleration, terminal is the cap; no primitive
change (justified by the small 2 u/s cap).

Feel decision inherited from the core primitive (review R1.1 on -122906): in
`rcs_burn_system`, the `RcsIntent` magnitude sets the ACCELERATION, so ANY held
deflection asymptotes to the full per-axis `cap` - deflection controls how fast
you reach the cap, not the terminal speed. Decide here whether a partial mouse
deflection should instead target a proportionally lower speed (which would mean
scaling the per-axis cap by `|cmd|` in the primitive), or whether full-cap-on-
any-hold is the intended docking feel. Choose deliberately.
