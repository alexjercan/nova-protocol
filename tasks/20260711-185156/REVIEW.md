# Review: Pause menu: ESC overlay in game and editor with Back to Main Menu

- TASK: 20260711-185156
- BRANCH: feature/pause-menu

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/input/player.rs:27-49,654-770;
  crates/nova_scenario/src/loader.rs:261;
  crates/nova_gameplay/src/camera_controller.rs:37 - the input layer is
  observers, which do not belong to system sets, so the Unpaused gate does
  not cover them: while paused, G/O/X engage/disengage the Autopilot (ship
  flies off on resume), burn input silently removes an engaged Autopilot,
  Enter advances the scenario script (on_next_input), and camera mouse-look
  accumulates while the cursor roams the overlay (view snaps on resume).
  The task step's "verify observers cannot side-effect" was checked with
  the wrong conclusion. Gate the gameplay observers on
  PauseStates::Unpaused (early return), including on_next_input and the
  camera rotation observer.
  - Response: fixed in dd25e93 - 14 observers pause-guarded (flight verbs, thruster/turret/torpedo intents, target/component cycles, camera rotation, scenario on_next_input); releases stay ungated so held keys clear during pause. E2E re-verified: G while paused engages no autopilot, before and after resume. One test fixture (targeting) gained the PauseStates it now needs.
- [x] R1.2 (MAJOR) crates/nova_menu/src/lib.rs setup_pause_ui - the dim
  root copies the main menu's Pickable { should_block_lower: false } where
  nothing sits beneath; under the pause overlay the editor's buttons and
  section-picking observers still receive clicks, and lock_on_left_click
  re-locks and hides the cursor UNDER the pause menu. Set
  should_block_lower: true on the dim layer and gate lock_on_left_click on
  Unpaused.
  - Response: fixed in dd25e93 - should_block_lower: true on the dim layer (with a comment on why it differs from the main menu root); lock_on_left_click additionally gated on Unpaused.
- [x] R1.3 (MAJOR) - the close record checks off test step (d) (a system
  in SpaceshipInputSystems does not run while Paused) but no such test
  exists, and the e2e's frozen-ship assertion does not prove the gate
  (paused Time<Virtual> stops FixedUpdate regardless); the gate could be
  deleted with everything green. Also the close record says "5 new tests
  ... existing 10/3" - reality is 4 new, 10/3 are post-change totals. Add
  the gating test against the production wiring and correct the record.
  - Response: fixed in dd25e93 - configure_pause_gating extracted and spaceship_sets_freeze_while_paused exercises the production wiring (runs, freezes same-frame, resumes); close record corrected (5 new tests now true, totals labeled as totals).
- [x] R1.4 (MINOR) - force_unpause on OnExit(Playing) applies one frame
  late: OnEnter(MainMenu) runs still-Paused (overlay lingers a frame) and
  restore_cursor executes in MainMenu, harmless only because the ambience
  has no player ship. Set Unpaused directly in on_back_to_menu (removes
  the lag) and make restore_cursor bail unless GameStates is Playing.
  - Response: fixed in dd25e93 - on_back_to_menu sets Unpaused in the same transition batch (force_unpause stays as the safety net for other exits); restore_cursor bails unless GameStates is Playing. E2E asserts the overlay is gone and the state Unpaused on the first menu frame.
- [x] R1.5 (MINOR) crates/nova_gameplay/src/audio.rs:268 - the thruster
  loop keeps roaring at its last volume while paused (volume updater is in
  the gated set; audio sinks ignore Time<Virtual>). Pause/resume the
  thruster sinks on the pause transitions.
  - Response: fixed in dd25e93 - thruster loop sinks pause on OnEnter(Paused) and resume on exit (audio sinks ignore virtual time).
- [x] R1.6 (NIT) - unpause_clocks/force_unpause unpause unconditionally;
  fine while this branch owns the only pause callers, but leave a comment
  so a future cutscene/debug freeze knows the pause menu will stomp it.
  - Response: fixed in dd25e93 - comment on unpause_clocks records the single-pauser assumption.

Round 1 notes (verified clean): Paused unreachable outside Playing;
double-ESC impossible; force_unpause vs editor reset have no ordering
hazard (different NextState resources); pausing Time<Virtual> stops
FixedUpdate in bevy 0.19 and nothing reads Time<Real>; the cursor cfg
carve-out is feature-symmetric across crates; Single params skip headless;
run_if conditions AND-compose; docs match the diff apart from R1.3. cargo
check clean; nova_menu 10 + nova_editor 3 green.

## Round 2

- VERDICT: APPROVE

Verified against dd25e93:
- R1.1: all 14 guards present with the release-path exemption; the e2e
  harness proves the headline case (G under the overlay) both while paused
  and after resume; targeting fixture updated rather than weakened.
- R1.2: dim layer blocks lower picking; lock_on_left_click gate confirmed.
- R1.3: the new test runs against configure_pause_gating (the exact fn the
  plugin calls) and asserts run/freeze/resume; close record now truthful.
- R1.4: same-batch unpause confirmed; e2e asserts no Paused frame leaks
  into the menu; restore_cursor guard present.
- R1.5/R1.6 confirmed in the diff.
- Re-ran: nova_gameplay 388 + nova_menu 10 + nova_editor 3 green, cargo
  check --workspace clean, fmt clean, 09_editor smoke green.

No new findings. APPROVE.
