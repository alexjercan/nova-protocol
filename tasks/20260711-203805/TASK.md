# F1 to editor must be Sandbox-only (disable in New Game scenarios)

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.5.0,bug,editor,input

## Goal

User report (2026-07-11): in New Game (scenario mode) pressing F1 drops you
into the ship editor. That path is demo/sandbox furniture and must be
disabled outside GameMode::Sandbox - "campaigns" should have no editor
escape. (Integrating the editor into campaigns properly is future design,
not this task.)

## Steps

- [x] Gate `switch_scene_editor` (crates/nova_editor/src/lib.rs, the F1
      handler running in ExampleStates::Scenario) with
      `run_if(resource_equals(GameMode::Sandbox))`, mirroring the
      setup_scenario gating from the menu task.
- [x] Regression test in nova_editor's test module: in GameMode::NewGame +
      ExampleStates::Scenario, press F1 (ButtonInput), assert the state
      stays Scenario and no UnloadScenario fired (delivery guard: same
      press in Sandbox mode must flip to Editor - prove the stimulus
      works).
- [x] check/fmt + new test; note in CHANGELOG (Fixed).

## Notes

- Reported during the pause-menu cycle (20260711-185156); kept separate to
  avoid widening that branch (both touch nova_editor).
- The pause menu is the sanctioned way out of a New Game scenario.


## Close record (2026-07-11)

switch_scene_editor gained run_if(resource_equals(GameMode::Sandbox)) on
top of its Scenario-state gate (and_then composition, mirrors the other
mode gates from the menu family). Regression test
f1_returns_to_editor_only_in_sandbox_mode: F1 in NewGame leaves the state
and scenario untouched (with the editor-scenario-load counter as the null
guard), and the delivery guard flips GameMode to Sandbox at press time and
asserts the same press queues the Editor state - would fail without the
fix (the gate reads the resource live) and fails if the F1 path itself
breaks. nova_editor 4/4 green; check/fmt clean.

Test-rig wrinkle worth remembering: entering Playing under Sandbox routes
the inner state to Editor whose scene setup needs GameAssets (panics
headless) - the test enters via NewGame and flips the mode before the
press instead.
