# Review: Main menu: MainMenu state, bottom-right panel UI, mode wiring

- TASK: 20260711-180426
- BRANCH: feature/main-menu

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (BLOCKER) crates/nova_editor/src/lib.rs:111-122 - New Game is
  unflyable in the default app. The editor plugin gates
  SpaceshipInputSystems and SpaceshipSectionSystems on
  run_if(in_state(ExampleStates::Scenario)); in NewGame mode ExampleStates
  stays Loading, so every player/AI input and section system
  (input/player.rs:65, input/targeting.rs:104, input/ai.rs:54 all live in
  those sets) is disabled while the menu-loaded scenario runs. Verified by
  re-derivation from the configure_sets calls, not by the diff summary.
  Suggested fix: on OnEnter(GameStates::Playing) the editor should map
  GameMode::Sandbox -> ExampleStates::Editor and GameMode::NewGame ->
  ExampleStates::Scenario, and its OnEnter(ExampleStates::Scenario)
  setup_scenario (which triggers LoadScenario(test_scenario)) must be gated
  to Sandbox mode so it does not collide with the menu's asteroid_field
  load. Add a delivery-guarded test or harness run proving a New Game
  scenario accepts input (at minimum: ExampleStates == Scenario and no
  second LoadScenario fired).
  - Response: fixed in 1bcf861. Editor maps Sandbox -> Editor, NewGame ->
    Scenario; setup_scenario gated run_if(resource_equals(GameMode::
    Sandbox)). Regression test new_game_enters_scenario_state_without_
    loading_the_editor_scenario pins the state routing plus a zero-load
    guard; a throwaway harness (real button click + held thrust, not
    committed) measured max speed 51.40 post-fix vs 2.88 pre-fix (pre-fix
    residual drift only - thrust dead), recorded in TASK.md. F1 in a New
    Game now drops to the editor, which reads as an escape hatch, not a
    bug.
- [x] R1.2 (MAJOR) crates/nova_core/src/lib.rs:131-146 - the
  OnEnter(GameAssetsStates::Loaded) hook unconditionally sets MainMenu,
  which yanks the state backwards for the BCS_SHOT screenshot harness:
  nova_screenshot() force-advances Loading -> Playing on frame 1, then the
  Loaded hook fires seconds later and transitions Playing -> MainMenu
  mid-settle (harness gives up with AppExit::error(), or captures a mixed
  editor+menu frame; the editor already entered its Editor state on the
  forced Playing). Guard the hook on the current state: only set
  MainMenu/Playing when still in GameStates::Loading.
  - Response: fixed in 1bcf861 exactly as suggested (early return unless
    still Loading).
- [x] R1.3 (MINOR) crates/nova_menu/src/lib.rs (tests) - the unit tests
  trigger Activate on bare entities spawned with observe(on_new_game) /
  observe(on_sandbox), so they cover the handler fns, not the real button
  wiring; dropping observe(on_new_game) from setup_menu_ui would not fail
  any automated check (Sandbox is covered e2e by the 09_editor smoke, New
  Game is not). Add a test that runs setup_menu_ui headless, finds the
  "New Game Button" by Name, triggers Activate on it, and asserts
  mode/state/LoadScenario.
  - Response: fixed in 1bcf861 - real_new_game_button_is_wired runs
    setup_menu_ui headless via run_system_once, clicks the real button by
    Name, asserts mode + state + scenario id. The (b) editor-entered
    assertion lives in nova_editor's sandbox_heads_to_editor_state.
- [x] R1.4 (MINOR) crates/nova_editor/src/lib.rs:189-191 - test_scenario's
  other ship changed from AI to passive for everyone, including menu-less
  editor use, where the spike had contemplated a separate sandbox_field
  variant. If build-and-fly is the intended sandbox scope (the spike and
  the parent task's wording support this), keep it and say so in the
  Response; otherwise restore AI and register a variant.
  - Response: keeping it, deliberately. The parent task 20260711-174915
    says sandbox should have "idealy no enemies, or passive enemies", and
    the user's 2026-07-11 follow-up direction for the menu background
    doubles down on passive/cinematic scenes. Documented in CHANGELOG and
    the spike fix record; combat testing remains available via New Game.
- [x] R1.5 (NIT) crates/nova_gameplay/src/lib.rs:67 - GameMode derives
  Reflect but is never registered (crate convention is register_type +
  #[reflect(Resource)], see relations.rs/gravity.rs). Register it next to
  init_resource::<GameMode>() or drop the derive.
  - Response: fixed in 1bcf861 - register_type + #[reflect(Resource)].
- [x] R1.6 (NIT) crates/nova_menu/src/lib.rs:96-110 - TASK.md specified an
  absolute-positioned node with right/bottom insets; the implementation
  uses two full-screen flex wrappers that then need Pickable opt-outs.
  position_type: Absolute with insets would match the spec and drop the
  wrappers. Take it or leave it; if left, update the TASK.md step wording
  to match reality.
  - Response: fixed in 1bcf861 for the menu panel (Absolute + right/bottom
    insets, wrapper dropped). The settings overlay keeps its centered
    full-screen wrapper - centering via Absolute needs percent-translate
    tricks that read worse than the wrapper; its Pickable opt-out stays.
- [x] R1.7 (NIT) MainMenu -> Playing (Sandbox) has roughly one camera-less
  frame (menu camera despawns on the outer transition; the editor camera
  spawns only after the inner ExampleStates transition applies). Cosmetic;
  fix only if visible.
  - Response: leaving as-is per the finding's own call; not visible in the
    harness runs.

Round 1 notes: checked clean - observer write-then-read ordering for
GameMode, resource existence at the OnEnter hooks, editor/menu button
observer cross-talk (EditorButton vs MenuButton filters), wasm gating of
Exit, the other 11 examples' lifecycles, docs claims vs code. Full check
suite: cargo check --workspace --all-targets clean, cargo test -p nova_menu
2/2, BCS_AUTOPILOT 09_editor run green (Sandbox path only - see R1.1/R1.3).

## Round 2

- VERDICT: APPROVE

Verified against the new diff (1bcf861 + responses commit):
- R1.1: confirmed the editor's OnEnter(Playing) match routes both modes and
  setup_scenario carries run_if(resource_equals(GameMode::Sandbox)); both
  nova_editor regression tests pass; the thrust A/B numbers (2.88 pre-fix
  drift vs 51.40 post-fix) demonstrate real input delivery, and the strict
  regression is the state-routing assertion, not the speed threshold.
- R1.2: guard present, early return unless Loading.
- R1.3: real_new_game_button_is_wired clicks the button spawned by the real
  setup_menu_ui; it would fail if the observe() wiring were dropped.
- R1.4: pushback accepted - the parent task's own wording and the user's
  2026-07-11 menu-background direction both call for a passive sandbox.
- R1.5, R1.6: confirmed in the diff. R1.7 left by agreement.
- Re-ran: cargo test -p nova_menu -p nova_editor (5/5), BCS_AUTOPILOT
  09_editor smoke green.

No new findings; the fixes are localized to the reviewed paths. APPROVE.
