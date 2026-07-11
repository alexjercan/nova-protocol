# Re-scope spaceship system set gating to scenario-liveness

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.5.0,scenario,input,spike

Goal: move the SpaceshipInputSystems / SpaceshipSectionSystems gating off the
editor's private ExampleStates::Scenario and onto "a scenario is live"
(CurrentScenario.is_some()), owned by nova_scenario's ScenarioLoaderPlugin as
a public named run condition. Delete the three configure_sets calls in
crates/nova_editor/src/lib.rs. The editor's build-mode preview stays inert
because the Editor state never has a loaded scenario (F1 unloads); the
MainMenu ambience scenario comes alive (thrusters, guns, sounds) because it
IS a loaded scenario. Regression tests: editor preview inert, sets live in
MainMenu with ambience loaded, sets stop again after unload.

Notes:
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- First of three seeded tasks; blocks the menu payoff task 20260711-212504.
- Pause gating (configure_pause_gating) composes by AND; do not disturb it.
- Watch-out: ambience scenarios must not contain Player-controlled ships
  (player input systems come alive wherever a scenario is live).
