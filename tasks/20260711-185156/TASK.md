# Pause menu: ESC overlay in game and editor with Back to Main Menu

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.5.0,ui,menu,spike

## Goal

ESC toggles a pause overlay from anywhere in Playing (scenario play or the
editor): a centered panel with Resume / Back to Main Menu / Exit. Pausing
truly freezes the simulation (virtual + physics time); Back to Main Menu
re-enters GameStates::MainMenu cleanly (scenario unloaded, editor scene
torn down).

## Steps

- [ ] Add `PauseStates { #[default] Unpaused, Paused }` as a bevy States
      enum in `crates/nova_gameplay/src/lib.rs` next to GameStates/GameMode
      (shared vocabulary: nova_menu owns the overlay, nova_editor and the
      gameplay plugin gate on it), init_state'd by AppBuilder alongside
      GameStates, exported via the prelude.
- [ ] ESC toggle in nova_menu: plain ButtonInput just_pressed
      (KeyCode::Escape - repo-wide grep found no existing Escape binding),
      run_if in_state(GameStates::Playing); toggles Paused <-> Unpaused.
- [ ] Freeze the sim: OnEnter(PauseStates::Paused) pause `Time<Virtual>`
      (stops Update deltas and FixedUpdate accumulation) AND
      `Time<Physics>` (avian's documented pause API,
      avian3d/src/schedule/time.rs); OnExit unpauses both. Verify whether
      pausing Virtual alone already halts avian (Physics follows Virtual in
      variable schedules) - pause both regardless, unpause must mirror.
- [ ] Cursor: on pause, release the grab (grab_mode None, visible true) so
      the overlay is clickable - scenario play locks the cursor
      (setup_grab_cursor_scenario, nova_editor). On resume, re-grab ONLY
      when scenario play is active; verify in code what distinguishes
      scenario play from editor build mode headlessly (candidate proxy: a
      PlayerSpaceshipMarker entity exists - confirm the editor's build-mode
      preview ship does not carry it), and respect the existing
      cfg!(not(feature = "debug")) grab rule.
- [ ] Gate gameplay input while paused: in nova_gameplay's plugin,
      configure SpaceshipInputSystems and SpaceshipSectionSystems with
      run_if(in_state(PauseStates::Unpaused)) - run conditions from
      different configure_sets calls compose (AND) with the editor's
      Scenario-state gate. Verify bevy_enhanced_input actions cannot
      side-effect outside those sets while paused.
- [ ] Pause overlay UI in nova_menu: OnEnter(Paused) spawn a full-screen
      dim layer + centered panel ("Paused" title, Resume / Back to Main
      Menu / Exit buttons, Exit cfg-gated off wasm), tagged
      DespawnOnExit(PauseStates::Paused), reusing the menu's button
      bundle/palette. Extend update_button_colors to run for the pause
      panel too (currently gated in_state(MainMenu)).
- [ ] Back to Main Menu wiring: button sets NextState<GameStates>::MainMenu
      and PauseStates::Unpaused. Entering MainMenu already loads the
      ambience scenario (LoadScenario tears down the gameplay scenario) and
      drives HudVisibility::None. Add the missing editor cleanup: in
      nova_editor, OnExit(GameStates::Playing) resets its private
      ExampleStates to Loading so DespawnOnExit(ExampleStates::Editor/
      Scenario) entities despawn and a later Sandbox entry starts fresh.
      Audit for other Playing-scoped leftovers (grab state, GameMode -
      GameMode may persist, the menu buttons rewrite it).
- [ ] Resume + Exit wiring: Resume sets Unpaused; Exit sends AppExit
      (mirror the main menu's Exit).
- [ ] Tests (nova_menu + nova_editor headless): (a) ESC toggles the state
      both ways with per-press delivery guards; (b) OnEnter(Paused) pauses
      both clocks, OnExit unpauses (delivery guard: assert paused BEFORE
      asserting unpaused); (c) Back-to-menu path: from Playing+Paused,
      button handler lands in MainMenu + Unpaused, editor ExampleStates
      reset to Loading (nova_editor test), and UnloadScenario/LoadScenario
      teardown fires (observer flag); (d) input sets gated: a system in
      SpaceshipInputSystems does not run while Paused.
- [ ] e2e: throwaway harness - boot default app, New Game, ESC, assert
      clocks paused + overlay entities exist, click Resume, assert running;
      ESC again, click Back to Main Menu, assert MainMenu + ambience
      loaded + gameplay scenario gone. 09_editor smoke must stay green.
- [ ] Run check/fmt + new tests; Xvfb screenshot of the pause overlay over
      a running scenario.
- [ ] Docs: CHANGELOG; architecture.md States section gains PauseStates;
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
