# RCS: refill while Shift is held, mouse cancels Shift+X

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.19.0, flight, input

## User facts
- Owner 2026-11-22: "very big annoyance : if I hold Shift and not move
  mouse (aka not using RCS) the RCS does not 'reload' it should because
  otherwise it's very very annoying and frustrating to use; also when in
  'Shift+X' stop mode we should allow the play to cancel it by moving the
  mouse AKA apply manual RCS, so basically if I press Shift+X and then move
  the mouse it overrides it back to manual RCS".

## Agent findings
- Correction: Shift held with NO mouse event already refills. `RcsActive`
  writes no intent; `decay_player_rcs_intent` snaps it to zero in ~6 ticks
  (`flight/manual.rs:439`), then `rcs_burn_system` calls `recover`.
- Cause (a): the RCS mouse binding had no floor. One mouse count at the
  owner's 500% gain (settings.ron `mouse_rcs_sensitivity: 0.15`) is a 15%
  push, and every push sets `budget.idle = 0.0` (`flight/manual.rs:307`),
  so a resting hand's count every < 2 s holds the refill off and drains it.
- Cause (b): `on_rcs_aim` returned when `Has<Autopilot>`; the wheel had
  `WheelRoleType::Rcs if engaged => {}`; cycle keys wrote an intent the
  autopilot overwrote. Only MATCH can run with Shift held: a fresh Shift
  press already drops any maneuver (`on_rcs_modifier_start`).
- No autopilot or assist depends on the old rule: MATCH, STOP, the
  neutralized brake and docked drivers write `RcsIntent` directly, not
  through the mouse binding. `RcsBudget` and `prediction.rs` are unchanged.

## Delivery
- RCS refills while Shift is held and no RCS input is applied.
- Mouse input (manual RCS) cancels the Shift+X stop and returns manual RCS.
## Build (2026-11-22)
- `MouseRestFloor` modifier (`input/player/flight_rig.rs:96-125`) on the
  RCS mouse binding, ahead of the gain: a frame of at most
  `MOUSE_REST_COUNTS = 1.0` raw count per axis is no input. Registered in
  `SpaceshipPlayerInputPlugin` (`input/player/mod.rs:112`). New rule: no
  dead-zone rule existed for the mouse; bevy's 0.05 stick dead zone is in
  intent units and a 500% count is 0.15. Cost: motion slower than one count
  a frame no longer pushes.
- `command_manual_rcs` (`input/player/control.rs:40`): manual RCS input with
  a maneuver engaged removes `Autopilot` (the manual breakout path, as the
  burn press) and inserts the new intent after the removal, so
  `on_autopilot_removed_cool_engines` cannot zero it. Used by `on_rcs_aim`,
  `on_wheel_step`, and the cycle keys (`targeting/component_lock.rs:227`).
- Tests: `flight/tests/piloted_rcs.rs` (real input plugin on the flight
  harness): the_rcs_magazine_refills_while_shift_is_held_with_no_input,
  a_held_manual_burn_still_holds_the_refill_off,
  manual_rcs_input_ends_the_shift_x_match_and_flies_the_manual_command.
  Before the fix the MATCH test and a one-count jitter repro failed
  (spent 4.72 -> 4.75 over 3 s with Shift held); zero-event hold passed.
- Affected run: `cargo test -p nova_ship --lib -- piloted_rcs
  flight::tests::rcs flight::tests::match_target input::`: 355 passed.
- Live check NOT run: the shared tree did not build (other lanes mid-edit in
  `nova_world_base/src/save/session.rs:586` and `nova_interface/src/jobs.rs:1782`).
  The patched walk (Shift sweep, resting-hand ticks at 500%, Shift+X, mouse)
  is saved at scratchpad `rcs-refill/docking_approach.rcscheck.rs`; the example
  is restored. Profile md5 unchanged.
- Docs: `web/src/wiki/flight-autopilot.md` (refill rule, MATCH breakout),
  `web/src/wiki/keybinds.md`; CHANGELOG Gameplay & Flight + Fixes lines.

## Verification
- Unit or ECS asserts for both rules; a playable example walk.

## Done when
- Both rules hold in play and the owner has played them.
