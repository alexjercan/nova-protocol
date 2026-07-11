# F1 to editor must be Sandbox-only (disable in New Game scenarios)

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,bug,editor,input

## Goal

User report (2026-07-11): in New Game (scenario mode) pressing F1 drops you
into the ship editor. That path is demo/sandbox furniture and must be
disabled outside GameMode::Sandbox - "campaigns" should have no editor
escape. (Integrating the editor into campaigns properly is future design,
not this task.)

## Steps

- [ ] Gate `switch_scene_editor` (crates/nova_editor/src/lib.rs, the F1
      handler running in ExampleStates::Scenario) with
      `run_if(resource_equals(GameMode::Sandbox))`, mirroring the
      setup_scenario gating from the menu task.
- [ ] Regression test in nova_editor's test module: in GameMode::NewGame +
      ExampleStates::Scenario, press F1 (ButtonInput), assert the state
      stays Scenario and no UnloadScenario fired (delivery guard: same
      press in Sandbox mode must flip to Editor - prove the stimulus
      works).
- [ ] check/fmt + new test; note in CHANGELOG (Fixed).

## Notes

- Reported during the pause-menu cycle (20260711-185156); kept separate to
  avoid widening that branch (both touch nova_editor).
- The pause menu is the sanctioned way out of a New Game scenario.

