# Re-scope spaceship system set gating to scenario-liveness

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.5.0,scenario,input,spike

## Goal

Move the SpaceshipInputSystems / SpaceshipSectionSystems gating off the
editor's private ExampleStates::Scenario and onto "a scenario is live"
(CurrentScenario.is_some()), owned by nova_scenario's ScenarioLoaderPlugin as
a public named run condition. The editor's build-mode preview stays inert
because the Editor state never has a loaded scenario (F1 unloads); the
MainMenu ambience scenario comes alive (thrusters, guns, sounds) because it
IS a loaded scenario.

## Steps

- [ ] Add a public run condition `scenario_is_live` (a fn returning bool from
      `Res<CurrentScenario>`) in crates/nova_scenario/src/loader.rs, exported
      through the crate prelude.
- [ ] In ScenarioLoaderPlugin::build (same file), configure the three set
      gates the editor holds today: SpaceshipInputSystems (Update) and
      SpaceshipSectionSystems (Update + FixedUpdate), each
      `.run_if(scenario_is_live)`.
- [ ] Delete the three configure_sets calls in
      crates/nova_editor/src/lib.rs (~line 137) and update the editor's
      routing comment (~lines 47-53) and test doc comments that cite the
      old gate as the reason MainMenu ships cannot fly.
- [ ] Update the stale comment in crates/nova_gameplay/src/audio.rs (~265)
      that names the editor's ExampleStates gate as the inherited condition.
- [ ] Tests in nova_scenario (loader): a probe system in each gated set (a)
      does not run with CurrentScenario None, (b) runs after LoadScenario,
      (c) stops again after UnloadScenario. Mirror the harness style of
      nova_gameplay's pause_gating_tests (plugin.rs).
- [ ] Editor-side regression: adapt/extend the nova_editor tests to assert
      the sets stay inert in the Editor state (no scenario loaded) - the
      preview-with-live-bindings scenario the old gate protected.
- [ ] Verify: cargo check + fmt, run the newly written tests, and confirm
      the 09_editor and 10_gameplay examples still behave (build only; CI
      runs the suite).

## Notes
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- First of three seeded tasks; blocks the menu payoff task 20260711-212504.
- Pause gating (configure_pause_gating) composes by AND; do not disturb it.
- Watch-out: ambience scenarios must not contain Player-controlled ships
  (player input systems come alive wherever a scenario is live).
