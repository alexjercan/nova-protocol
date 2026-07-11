# Pause menu: ESC overlay in game and editor with Back to Main Menu

- STATUS: CLOSED
- PRIORITY: 43
- TAGS: v0.5.0,ui,menu,spike

## Goal

ESC toggles a pause overlay from anywhere in Playing (scenario play or the
editor): a centered panel with Resume / Back to Main Menu / Exit. Pausing
truly freezes the simulation (virtual + physics time); Back to Main Menu
re-enters GameStates::MainMenu cleanly (scenario unloaded, editor scene
torn down).

## Steps

- [x] Add `PauseStates { #[default] Unpaused, Paused }` as a bevy States
      enum in `crates/nova_gameplay/src/lib.rs` next to GameStates/GameMode
      (shared vocabulary: nova_menu owns the overlay, nova_editor and the
      gameplay plugin gate on it), init_state'd by AppBuilder alongside
      GameStates, exported via the prelude.
- [x] ESC toggle in nova_menu: plain ButtonInput just_pressed
      (KeyCode::Escape - repo-wide grep found no existing Escape binding),
      run_if in_state(GameStates::Playing); toggles Paused <-> Unpaused.
- [x] Freeze the sim: OnEnter(PauseStates::Paused) pause `Time<Virtual>`
      (stops Update deltas and FixedUpdate accumulation) AND
      `Time<Physics>` (avian's documented pause API,
      avian3d/src/schedule/time.rs); OnExit unpauses both. Verify whether
      pausing Virtual alone already halts avian (Physics follows Virtual in
      variable schedules) - pause both regardless, unpause must mirror.
- [x] Cursor: on pause, release the grab (grab_mode None, visible true) so
      the overlay is clickable - scenario play locks the cursor
      (setup_grab_cursor_scenario, nova_editor). On resume, re-grab ONLY
      when scenario play is active; verify in code what distinguishes
      scenario play from editor build mode headlessly (candidate proxy: a
      PlayerSpaceshipMarker entity exists - confirm the editor's build-mode
      preview ship does not carry it), and respect the existing
      cfg!(not(feature = "debug")) grab rule.
- [x] Gate gameplay input while paused: in nova_gameplay's plugin,
      configure SpaceshipInputSystems and SpaceshipSectionSystems with
      run_if(in_state(PauseStates::Unpaused)) - run conditions from
      different configure_sets calls compose (AND) with the editor's
      Scenario-state gate. Verify bevy_enhanced_input actions cannot
      side-effect outside those sets while paused.
- [x] Pause overlay UI in nova_menu: OnEnter(Paused) spawn a full-screen
      dim layer + centered panel ("Paused" title, Resume / Back to Main
      Menu / Exit buttons, Exit cfg-gated off wasm), tagged
      DespawnOnExit(PauseStates::Paused), reusing the menu's button
      bundle/palette. Extend update_button_colors to run for the pause
      panel too (currently gated in_state(MainMenu)).
- [x] Back to Main Menu wiring: button sets NextState<GameStates>::MainMenu
      and PauseStates::Unpaused. Entering MainMenu already loads the
      ambience scenario (LoadScenario tears down the gameplay scenario) and
      drives HudVisibility::None. Add the missing editor cleanup: in
      nova_editor, OnExit(GameStates::Playing) resets its private
      ExampleStates to Loading so DespawnOnExit(ExampleStates::Editor/
      Scenario) entities despawn and a later Sandbox entry starts fresh.
      Audit for other Playing-scoped leftovers (grab state, GameMode -
      GameMode may persist, the menu buttons rewrite it).
- [x] Resume + Exit wiring: Resume sets Unpaused; Exit sends AppExit
      (mirror the main menu's Exit).
- [x] Tests (nova_menu + nova_editor headless): (a) ESC toggles the state
      both ways with per-press delivery guards; (b) OnEnter(Paused) pauses
      both clocks, OnExit unpauses (delivery guard: assert paused BEFORE
      asserting unpaused); (c) Back-to-menu path: from Playing+Paused,
      button handler lands in MainMenu + Unpaused, editor ExampleStates
      reset to Loading (nova_editor test), and UnloadScenario/LoadScenario
      teardown fires (observer flag); (d) input sets gated: a system in
      SpaceshipInputSystems does not run while Paused.
- [x] e2e: throwaway harness - boot default app, New Game, ESC, assert
      clocks paused + overlay entities exist, click Resume, assert running;
      ESC again, click Back to Main Menu, assert MainMenu + ambience
      loaded + gameplay scenario gone. 09_editor smoke must stay green.
- [x] Run check/fmt + new tests; Xvfb screenshot of the pause overlay over
      a running scenario.
- [x] Docs: CHANGELOG; architecture.md States section gains PauseStates;
      Fix record line in docs/spikes/20260711-180500-main-menu.md.

## Notes

- Spike: docs/spikes/20260711-180500-main-menu.md (menu family)
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (main menu, CLOSED - landed 8504948)
- Verified this session: no Escape binding exists anywhere (repo grep);
  avian pause is Time<Physics>::pause()/unpause(); the cursor lock lives in
  nova_editor's setup_grab_cursor_scenario with a debug-feature carve-out.
- F1 (Scenario -> Editor) stays as-is; the pause menu is the way OUT of
  Playing, F1 is movement within the editor's own states.
- A Settings button joins the pause panel when the Settings content task
  (20260711-180511, v0.6.0) lands - do not add a dead button now.

## Close record (2026-07-11)

What changed:
- PauseStates { Unpaused, Paused } in nova_gameplay (shared vocabulary),
  init'd by AppBuilder. nova_gameplay gates SpaceshipInputSystems +
  SpaceshipSectionSystems (Update and FixedUpdate) on Unpaused - composes
  with the editor's Scenario gate, and matters because paused clocks do
  not stop action side effects like projectile spawns.
- nova_menu owns the rest: ESC toggle (KeyCode::Escape, no prior binding,
  Playing-only), OnEnter(Paused) pauses Time<Virtual> + Time<Physics>
  (avian's documented API), releases the cursor, and spawns the dimmed
  overlay (DespawnOnExit, GlobalZIndex above the HUD) with Resume / Back
  to Main Menu / Exit (wasm-gated); OnExit unpauses and re-grabs the
  cursor only during scenario play (PlayerSpaceshipMarker proxy - only the
  scenario spawn path inserts it - honoring the debug-build no-grab rule).
  force_unpause on OnExit(Playing) covers the Back path.
- nova_editor resets its private ExampleStates to Loading on
  OnExit(GameStates::Playing), so DespawnOnExit scene entities tear down
  and the next Sandbox entry starts fresh.
- Button color feedback ungated (query matches only MenuButton).

Verification (corrected per review R1.3):
- 5 new tests: ESC toggles state + both clocks (per-press guards), overlay
  spawn/despawn via the real Resume button, back-to-menu lands
  MainMenu+Unpaused+ambience-loaded, editor inner-state reset
  (nova_editor), and spaceship_sets_freeze_while_paused (nova_gameplay,
  against the production configure_pause_gating wiring). Post-change
  totals: nova_menu 10, nova_editor 3, nova_gameplay 388, all green.
- Throwaway e2e harness (not committed), all ten assertions OK in the real
  app: clocks frozen, ship frozen for 60 frames under HELD thrust (the
  input-gate delivery proof), resume, back to menu with the gameplay
  scenario torn down and clocks running.
- Xvfb capture of the overlay over a frozen New Game scene (dim layer,
  panel, all three buttons).
- 09_editor smoke green; check/fmt clean; full suite/clippy per repo
  policy left to CI.

Playtest fallout filed separately: F1 editor escape must be Sandbox-only
(20260711-203805) - noticed while wiring the New Game path here.

## Review round 1 addendum (2026-07-11)

Review caught three MAJORs the original implementation missed:
1. The input layer is OBSERVERS, which system-set gating does not touch -
   while paused, G engaged the autopilot (ship flies off on resume), Enter
   advanced the scenario script, mouse-look accumulated. Fixed by
   pause-guarding 14 observers across nova_gameplay (flight verbs,
   thruster/turret/torpedo intents, target/component cycles, camera
   rotation) and nova_scenario (on_next_input); releases stay ungated so
   held keys clear during pause. E2E harness re-verified: G while paused
   engages nothing, before and after resume.
2. The overlay copied the main menu's non-blocking Pickable; under it the
   editor's buttons and section picking stayed clickable and right-click
   re-locked the cursor. Fixed: should_block_lower on the dim layer,
   lock_on_left_click additionally gated on Unpaused.
3. The claimed input-set gating test did not exist (and the e2e frozen-ship
   check could not prove the gate - paused virtual time stops FixedUpdate
   regardless). Added spaceship_sets_freeze_while_paused against the
   extracted production wiring.
Also from review: on_back_to_menu now unpauses in the same transition batch
(the OnExit(Playing) force_unpause alone applied one frame late, leaking a
Paused frame into the menu), restore_cursor bails outside Playing, and the
thruster loop sink pauses with the game (audio ignores virtual time).
