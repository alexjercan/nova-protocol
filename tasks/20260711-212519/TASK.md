# Re-scope spaceship system set gating to scenario-liveness

- STATUS: CLOSED
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

- [x] Add a public run condition `scenario_is_live` (a fn returning bool from
      `Res<CurrentScenario>`) in crates/nova_scenario/src/loader.rs, exported
      through the crate prelude.
- [x] In ScenarioLoaderPlugin::build (same file), configure the three set
      gates the editor holds today: SpaceshipInputSystems (Update) and
      SpaceshipSectionSystems (Update + FixedUpdate), each
      `.run_if(scenario_is_live)` - factored into
      `configure_scenario_gating(app)` (pause-gating precedent) so the tests
      exercise the production wiring.
- [x] Delete the three configure_sets calls in
      crates/nova_editor/src/lib.rs (~line 137) and update the editor's
      routing comment (~lines 47-53) and test doc comments that cite the
      old gate as the reason MainMenu ships cannot fly.
- [x] Update the stale comment in crates/nova_gameplay/src/audio.rs (~265)
      that names the editor's ExampleStates gate as the inherited condition.
- [x] Tests in nova_scenario (loader): a probe system in each gated set (a)
      does not run with CurrentScenario None, (b) runs after LoadScenario,
      (c) stops again after UnloadScenario. Mirror the harness style of
      nova_gameplay's pause_gating_tests (plugin.rs). Two tests: one flips
      CurrentScenario directly, one drives the real
      on_load_scenario/unload_scenario observers end to end.
- [x] Editor-side regression: pin the invariant the gate's inertness claim
      rests on - the Editor state never has a live scenario. New test
      `editor_state_never_keeps_a_scenario_live`: initial entry fires no
      LoadScenario, and F1 (the one route entering Editor FROM a live
      scenario) triggers UnloadScenario on the same press, delivery-guarded
      by the queued Editor state. (Step rephrased from "assert the sets stay
      inert in Editor state": the sets are gated in nova_scenario, which the
      editor's headless test app deliberately does not include; the
      cross-crate composition is covered by the loader tests plus this
      invariant.)
- [x] Verify: cargo check + fmt, run the newly written tests, and confirm
      the 09_editor and 10_gameplay examples still behave (build only; CI
      runs the suite).

## Notes
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- First of three seeded tasks; blocks the menu payoff task 20260711-212504.
- Pause gating (configure_pause_gating) composes by AND; do not disturb it.
- Watch-out: ambience scenarios must not contain Player-controlled ships
  (player input systems come alive wherever a scenario is live).

## Close record (2026-07-11)

What changed: the three spaceship set gates moved from nova_editor (private
ExampleStates::Scenario) to nova_scenario's ScenarioLoaderPlugin, gated on
the new public `scenario_is_live` run condition (CurrentScenario.is_some()),
factored as `configure_scenario_gating` for production-faithful tests.
Editor and audio comments updated to name the new owner.

Why this shape: scenario-liveness is what the old gate actually meant. The
editor preview stays inert because Editor state never has a scenario loaded
(initial entry loads nothing; F1 unloads on the way in - now pinned by
`editor_state_never_keeps_a_scenario_live`). Alternatives (widening the
editor gate with menu states; per-entity opt-in markers; an abstract
SimulationActive resource) are weighed in the spike doc.

Verification: cargo check --workspace green; cargo fmt applied;
nova_scenario loader tests 5/5 (2 new), nova_editor tests 5/5 (1 new);
examples 09_editor and 10_gameplay build. Per repo policy the full suite
runs in CI, not locally.

Difficulties: none material. The one design wrinkle was where the
editor-side regression could live, since the gating is wired one crate above
the editor's test app; resolved by testing the liveness invariant in
nova_editor and the gate mechanics (including the real observer chain) in
nova_scenario.

Known behavioral delta (review R1.5): the menu orbiter's
basic_controller_section PD attitude hold now runs in MainMenu (FixedUpdate
sections set), so the backdrop ship holds its spawn attitude instead of
tumbling freely. Torque only - the ballistic orbit is unaffected. Interim
state; task 20260711-212504 replaces the orbiter's flight model and carries
the run-and-watch verification.

Self-reflection: the spike had already read all the relevant code, so
implementation was mechanical - evidence that spending the spike on WHY the
gate existed (preview sections carry live bindings) was the right call.
Observers remain ungated by design (set-gates-miss-observers lesson); the
menu ambience scene has no player ship, so nothing observer-driven acts in
MainMenu today - the follow-up tasks keep it that way.
