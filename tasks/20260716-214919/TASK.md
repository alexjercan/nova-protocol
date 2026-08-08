# Pause the game while the Victory/Defeat outcome screen is displayed (like the menu)

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.7.0, feature, ui, scenario

## Steps

- [x] `sync_outcome_pause` (nova_menu, next to `sync_outcome_overlay`, gated
      `in_state(Playing)` + `resource_exists::<CurrentOutcome>`): mirror
      `CurrentOutcome` -> `PauseStates`. Outcome change to `Some` -> set
      `Paused` (the existing `OnEnter(Paused)` -> `pause_clocks` freezes
      `Time<Virtual>`+`Time<Physics>`, and the `Unpaused` set-gates stop
      input/weapons - the SAME mechanism the menu uses). Change to `None`
      while currently `Paused` -> set `Unpaused`.
- [x] Guard `setup_pause_ui` (OnEnter Paused): skip spawning the pause-menu
      overlay when `CurrentOutcome` is active - the outcome overlay is the
      modal; do not stack two panels (`does-the-old-element-survive`).
- [x] Guard `toggle_pause`: ESC/Start is inert while an outcome is up - never
      unpause into a live sim behind the overlay, never open the pause menu
      over the outcome. (ESC over the outcome does nothing; it has buttons.)
- [x] `decide_advance` (nova_scenario/loader.rs): allow the scenario-advance
      input while paused when an outcome is present (`paused && !has_outcome
      -> Ignore`). Keeps Enter/Continue/Retry live under the outcome pause;
      the plain pause menu still swallows Enter.
- [x] Old-element-survives decision: on Continue/Retry the outcome clears at
      teardown -> `sync_outcome_pause` unpauses; on Main Menu / Enter-to-menu,
      `force_unpause` (OnExit Playing) covers it. Single source of truth is
      `CurrentOutcome`; no explicit unpause in the button handlers.
- [x] Tests: `decide_advance` table (paused+outcome+queued -> ReleaseQueued,
      paused+outcome+unqueued -> ExitToMenu, paused+no-outcome -> Ignore);
      nova_menu integration - outcome -> Paused + clocks frozen, clear ->
      Unpaused + clocks resume (delivery-guarded both edges), pause overlay
      NOT spawned under the outcome, ESC inert under the outcome. Enumerate
      Victory(queued)/Defeat(queued)/Victory(unqueued) per
      `probe-the-adversarial-variant`.
- [x] Docs: CHANGELOG [Unreleased] line; scenario-system.md +
      guide-author-scenario.md note the overlay now FREEZES the sim.
- [x] Verify: `cargo check --all-targets` + fmt + the new tests (workspace or
      with a unifying sibling per `crate-solo-tests-miss-unified-features`).

## Goal

When the scenario outcome frame (VICTORY/DEFEAT overlay, task 20260716-125856)
is shown, the simulation should PAUSE - freeze physics/AI/gameplay ticking the
same way the menu screens do - instead of the world continuing to run behind
the overlay. Today the overlay draws on top of a still-live scene.

## Direction

- Find how the menu/pause states already freeze the sim (the Esc pause menu and
  the main menu). There is very likely an existing "gameplay is running" run
  condition / state gate (a Pause state, a `TimeState`, or a
  `run_if(in_state(Playing))` guard on the gameplay schedules).
- Apply the SAME mechanism when the outcome frame is active, so physics, AI,
  weapons and timers stop while VICTORY/DEFEAT is up - matching the menu's
  behavior exactly rather than inventing a second pause path.
- Keep the outcome frame's own input alive (Continue/Retry/Main Menu buttons
  must still work while the sim is paused), same as the pause menu overlay.
- Decide what happens to the pause on Continue/Retry: unpause on resume/retry,
  stay paused into the menu, etc. (`does-the-old-element-survive`).

## Notes

- Reported/requested by user 2026-07-16.
- Related: scenario outcome frame (20260716-125856), pause menu Retry
  (20260716-210125).
