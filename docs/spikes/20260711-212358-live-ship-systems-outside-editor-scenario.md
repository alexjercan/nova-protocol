# Spike: How do thruster-driven ships come alive outside the editor's Scenario state?

- DATE: 20260711-212358
- STATUS: RECOMMENDED
- TAGS: spike, v0.5.0, ai, scenario, input, menu

## Question

Two coupled uncertainties, from task 20260711-185440:

1. The editor gates ALL spaceship input/section system sets on its private
   `ExampleStates::Scenario`, so in MainMenu (and any future state) nothing
   can fire a thruster, gun, or thruster hum. Why is that gate there, and
   what is the correct scope for it so menu ambience scenes can "play the
   game without HUD" (Factorio-style) without letting the editor's build-mode
   preview ship fly itself?
2. What is the right seam for making the menu ambience ship fly its orbit
   with real thrusters: expand `AIControllerConfig`, add scenario event
   actions, or make orbiting an autonomous passive AI behavior?

A good answer names the run condition that replaces the editor's state gate
(with proof the editor preview stays inert), and picks one of the three AI
shapes with a concrete component/config design the planner can expand.

## Context

- `crates/nova_editor/src/lib.rs` (configure_sets, ~line 137) gates
  `SpaceshipInputSystems` (Update) and `SpaceshipSectionSystems` (Update +
  FixedUpdate) on `in_state(ExampleStates::Scenario)`. The states are the
  editor plugin's private enum: Loading -> Editor | Scenario.
- WHY the gate exists (verified by reading the editor, not just the comment):
  the editor's preview ship uses REAL section components. Placing a thruster
  in build mode spawns `thruster_section(...)` plus a live
  `SpaceshipThrusterInputBinding` (same for turrets/torpedoes,
  on_click_spaceship_section). Only the root marker differs
  (`SpaceshipPreviewMarker` vs `SpaceshipRootMarker`). Input observers fire
  regardless of system sets (they are observers), but everything that ACTS -
  thruster impulses, projectile spawning, shader flames, the thruster hum -
  lives in the two gated sets. Ungated, pressing a bound key while building
  would fly/fire the preview. Git history adds nothing beyond this: the gate
  arrived with the original example app (pre-`nova_editor`) with no recorded
  rationale; the code is the rationale.
- The scenario loader already exposes the exact signal we need:
  `CurrentScenario(Option<ScenarioConfig>)` (crates/nova_scenario/src/loader.rs).
  `LoadScenario` sets it, `UnloadScenario` clears it. In the Editor state a
  scenario is NEVER live: initial entry loads nothing, and F1-back-to-editor
  triggers `UnloadScenario`. The menu ambience backdrop IS a loaded scenario.
- Pause gating (`configure_pause_gating`, nova_gameplay/src/plugin.rs)
  composes by AND across separate configure_sets calls, so it stacks with
  whatever replaces the editor gate.
- Audio: the thruster hum joins `SpaceshipSectionSystems` explicitly to
  inherit this gate (nova_gameplay/src/audio.rs ~265); one-shot cues fire on
  events that only occur inside the gated sets. Sound in the menu comes for
  free once the sets run.
- All examples (01..12) that spawn ships do so via `LoadScenario`, so a
  scenario-liveness gate changes nothing for them. Examples that skip the
  scenario machinery (02_thruster_shader, 05_directional) register their own
  systems, not the gated sets.
- The menu ambience ship (nova_assets/src/scenario.rs, `menu_orbiter`)
  already has controller + thruster sections but
  `SpaceshipController::None`; nova_menu seeds a ballistic orbit velocity as
  a workaround and documents the gate as the reason.
- AI substrate: `AIPatrolRoute` is the existing per-entity passive directive
  (config -> component -> passive behavior state). `next_behavior_state`
  (nova_gameplay/src/input/ai.rs) picks the passive fallback: `Patrol` with
  a route, else `Idle`. `AutopilotAction::Orbit { well, plan }`
  (nova_gameplay/src/flight.rs) is the flying substrate; AI steering already
  drives Patrol/Idle through engaged autopilot verbs (GOTO/STOP).

## Options considered

### Part 1: the gate

- **A. Gate on scenario-liveness, owned by nova_scenario (chosen).** Move the
  three configure_sets calls out of nova_editor into `ScenarioLoaderPlugin`,
  with `run_if(|s: Res<CurrentScenario>| s.is_some())` (as a named condition,
  e.g. `scenario_is_live`). Editor preview stays inert because Editor state
  never has a live scenario. MainMenu ambience runs live because it IS a
  loaded scenario. Pros: matches the gate's true meaning ("a simulation is
  running"), one owner, menu/editor/examples all fall out correctly, layering
  is fine (nova_scenario already depends on nova_gameplay). Cons: the sets'
  run condition now lives one crate above where the sets are defined; apps
  that skip ScenarioLoaderPlugin get ungated sets (same as every example
  today, so no regression).
- **B. Keep the editor gate, widen it (Scenario OR MainMenu).** Rejected:
  nova_editor would have to know about the menu's states (layering
  violation), and every future live-scene state repeats the bug.
- **C. Per-entity opt-in (a LiveSimulationMarker on ship roots).** Most
  precise (could even let a future editor test-fire one thruster), but
  invasive: every system in both sets would need query changes, and the
  existing idiom for "is the sim running" is set-level run conditions (pause
  does the same). Rejected for now; note that A does not preclude adding
  this later if the editor ever wants selectively-live previews.
- **D. Abstract `SimulationActive` resource in nova_gameplay, toggled by
  nova_scenario.** Same behavior as A with one more moving part; only worth
  it if a second writer beside the scenario loader appears. Rejected as
  premature.

### Part 2: the AI orbit seam

- **1. Config directive in `AIControllerConfig` (chosen as the seam).** Add
  an orbit directive next to `patrol`, e.g. `orbit: Option<EntityId>` (the
  well entity's scenario id). `insert_spaceship_sections` maps it to a new
  per-entity component `AIOrbitDirective { well: EntityId }` exactly as
  `patrol` maps to `AIPatrolRoute`. A new passive state `Orbit` in
  `AIBehaviorState`: passive fallback precedence orbit > patrol > idle in
  `next_behavior_state`. Steering resolves the well `EntityId` to its Entity
  and keeps `AutopilotAction::Orbit { well, plan: None }` engaged, the same
  shape as Patrol keeping a GOTO engaged; combat transitions
  (Engage/Evade) override it for free and it resumes when calm returns.
  Pros: deterministic, per-ship, scenario-authorable, smallest new surface,
  and it reuses the AIPatrolRoute pattern end to end. Cons: not runtime
  commandable and not autonomous, but see below - both compose on top.
- **2. Scenario event action ("go into orbit") in `EventActionConfig`.**
  Good moddability, but it needs the same component and behavior state as 1
  to have something to command; the action is just a runtime writer of
  `AIOrbitDirective`. Deferred: layer it on later when a scenario actually
  needs mid-run AI direction.
- **3. Autonomous orbiting (passive AI near a well enters orbit on its
  own).** Gives ambient life everywhere, but silently changes every existing
  AI ship's passive behavior in every scenario (patrollers drifting into
  orbits, station-keepers wandering off), which is a tuning/QA cost the menu
  does not need. Deferred: if wanted later it is a small autonomous writer
  of the same Orbit passive state (enter when idle + well in range).
- **Do nothing (keep ballistic seeding).** The menu already orbits, but with
  dead thrusters (no flame, no hum) and bespoke staging math in nova_menu;
  the whole point of the task is the alive-ness. Rejected.

The three AI options compose exactly as the task suspected: 1 is the config
surface and the seam, 2 and 3 are alternative writers of the same directive
component/behavior state later.

## Recommendation

Three steps, in order:

1. **Re-scope the gate to scenario-liveness.** Delete the three
   configure_sets calls from nova_editor; add them to
   `ScenarioLoaderPlugin` gated on `CurrentScenario.is_some()` via a public
   named run condition. Regression tests: (a) editor-state preview stays
   inert (a probe system in the set does not run while a preview thruster
   binding exists and no scenario is loaded), (b) the sets run in MainMenu
   with the ambience scenario live, (c) F1 back-to-editor stops them again.
2. **AI orbit directive.** `AIControllerConfig.orbit: Option<EntityId>` ->
   `AIOrbitDirective` component -> `AIBehaviorState::Orbit` passive state ->
   steering keeps `AutopilotAction::Orbit` engaged on the resolved well.
   Passive precedence: orbit > patrol > idle. Combat states override and
   return naturally.
3. **Menu payoff.** Flip `menu_orbiter` to
   `SpaceshipController::AI(AIControllerConfig { orbit: Some("menu_planetoid"), .. })`
   and delete the menu's ballistic seeding + restaging math (the autopilot's
   own orbit insertion plan replaces it); keep the camera staging. Thruster
   flame + hum in the menu come free via the gated sets and the audio
   plugin's set membership.

Watch-outs for the planner:
- Player input systems also come alive wherever a scenario is live; today
  that is harmless (no player ship in ambience scenes, and the flight input
  rig only spawns with `PlayerSpaceshipMarker`), but ambience scenarios must
  not include Player-controlled ships. Worth a doc note on `menu_ambience`.
- The scenario loader spawns its own "Scenario Camera" on load; the menu
  already handles staging its camera over it (stage_menu_camera). Unchanged,
  just do not break it.
- `AISpaceshipMarker` currently implies `Allegiance::Enemy`; irrelevant in a
  menu with no player, but verify no HUD/targeting system panics without a
  player present when the AI sets run in MainMenu (HUD chrome is hidden, not
  absent).

## Open questions

- Should `AutopilotAction::Orbit`'s insertion plan handle a ship starting
  from rest at arbitrary distance gracefully enough for the menu (no wild
  swing across the camera)? If not, a small staging position in the scenario
  config (spawn near the target orbit) hides it - measure during step 3.
- Whether `orbit` in config should be an enum (`behavior: Passive::{Idle,
  Patrol(..), Orbit(..)}`) instead of a second Option field; two Options can
  both be set. Planner's call; precedence orbit > patrol is the documented
  tie-break either way.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260711-212519: re-scope spaceship system set gating to
  scenario-liveness (nova_editor -> nova_scenario, CurrentScenario.is_some())
- tatr 20260711-212521: AI orbit directive (config -> AIOrbitDirective ->
  AIBehaviorState::Orbit -> AutopilotAction::Orbit)
- tatr 20260711-212504: menu ambience flies the orbit on thrusters, ballistic
  seeding deleted (depends on the other two)

The originating task 20260711-185440 is the spike itself; it closes when
these land (the last task's notes carry the reminder).

## Fix record

(appended by implementing tasks as they land)
