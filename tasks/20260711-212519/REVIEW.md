# Review: Re-scope spaceship system set gating to scenario-liveness

- TASK: 20260711-212519
- BRANCH: feat/scenario-live-gating

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; the MINORs below were addressed on the
  branch before landing, see Responses)
- Method: fresh-context agent review of the full diff (out-of-context pass,
  per the review skill's blind-spot rule) + in-session re-derivation of the
  tuple run_if semantics and the back-to-menu teardown path. The agent
  additionally traced every route into ExampleStates::Editor, the observer
  timing of Load/UnloadScenario vs run-condition evaluation, all
  in_set(SpaceshipInput/SectionSystems) consumers for MainMenu enablement,
  and all examples; all clean.

- [x] R1.1 (MINOR) crates/nova_menu/src/lib.rs:344-351 - stale doc on
  seed_orbiter_velocity still claims the editor gates the sets and MainMenu
  cannot fly thrusters; after this diff the sets ARE live in MainMenu and
  only `SpaceshipController::None` keeps the orbiter passive. Reword.
  - Response: fixed in the round-1 fixup commit; comment now states the sets
    are live and controller None is what keeps the ship ballistic.
- [x] R1.2 (MINOR) crates/nova_assets/src/scenario.rs:88-94 - same stale,
  now-backwards claim on the menu_orbiter config; load-bearing for the next
  editor of the ambience scene. Replace with a live-sets warning + keep
  controller None note.
  - Response: fixed in the round-1 fixup commit.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/plugin.rs:132 -
  configure_pause_gating doc still says it stacks with "the editor's
  Scenario-state gate". Point it at nova_scenario's scenario_is_live gate.
  - Response: fixed in the round-1 fixup commit.
- [x] R1.4 (MINOR) crates/nova_editor/src/lib.rs (test
  editor_state_never_keeps_a_scenario_live) - doc comment promises a
  "loads nothing on initial entry" half the test does not assert (it enters
  via NewGame and never reads EditorScenarioLoads). Assert the load counter
  stays 0 through the F1 route and fix the comment.
  - Response: fixed in the round-1 fixup commit; the test now asserts
    EditorScenarioLoads == 0 after the press and the comment describes the
    actually-exercised route.
- [x] R1.5 (MINOR) unacknowledged behavioral delta: the menu orbiter's
  basic_controller_section PD controller (sync_controller_section_forces,
  FixedUpdate) now applies attitude-holding torque in MainMenu; previously
  rotation was fully free. Ballistic orbit unaffected (torque only), but
  the backdrop's look changes and nothing verifies it. Record the delta and
  eyeball the menu.
  - Response: recorded in the TASK.md close record and in the ambience
    comment (R1.2's rewrite); the visual eyeball lands with task
    20260711-212504, which replaces the orbiter's flight model entirely and
    has an explicit run-and-watch step. Accepted as a known, benign interim
    delta.
- [x] R1.6 (NIT) crates/nova_scenario/src/loader.rs (configure_scenario_gating)
  - the turret aim chain also sits in SpaceshipSectionSystems in
  PostUpdate, which neither the old nor the new gate covers (parity with
  master). Add a comment so the asymmetry is deliberate, or gate it.
  - Response: comment added in the round-1 fixup commit; left ungated
    deliberately (parity with master; the aim chain is read-only pose math
    and gating it is out of this task's scope).
