# Enable thruster-driven ships outside editor Scenario state (menu AI orbit)

- STATUS: CLOSED
- PRIORITY: 41
- TAGS: v0.5.0,ai,scenario,input,spike

Outcome (2026-07-11): DELIVERED across the three spike-seeded tasks.
20260711-212519 re-scoped the spaceship set gating to scenario-liveness
(scenario_is_live, nova_scenario), 20260711-212521 built the AI orbit
directive (config -> AIOrbitDirective -> AIBehaviorState::Orbit ->
AutopilotAction::Orbit), and 20260711-212504 flipped the menu orbiter to a
thruster-flown AI orbit and deleted the ballistic seeding - verified live
(flame visible, ship circling, camera steady). Options 2 (event actions)
and 3 (autonomous orbiting) remain deliberate non-goals, layerable on the
same directive component later. Full record: the spike doc's fix record
and each task's close record.

Goal (sharpened per user direction 2026-07-11, moved into v0.5.0): the
menu ambience ship should fly its orbit with real thrusters instead of the
current ballistic seeding. That is blocked by the editor gating ALL
spaceship input/section system sets on its private ExampleStates::Scenario
(crates/nova_editor/src/lib.rs configure_sets) - in MainMenu nothing can
fire a thruster. The gating presumably exists so the editor's build-mode
preview ship does not fly itself; a spike must first answer WHY it is
there (git history, editor preview behavior) and how to scope it correctly
(e.g. gate on "a live scenario is loaded" instead of the editor's private
state, or opt ships in per-entity), then un-gate MainMenu safely without
letting editor preview ships fly.

Second half, the original AI-behavior direction - three candidate shapes
to weigh in the same spike:

1. Expand AIControllerConfig (today it only has `patrol: Vec<Vec3>`,
   crates/nova_scenario/src/objects/spaceship.rs) with behavior options,
   e.g. an orbit-this-well directive.
2. Scenario event actions that command an AI at runtime ("go into orbit"),
   fitting the existing EventActionConfig vocabulary - would make AI
   direction moddable per scenario.
3. Orbit as an autonomous AI behavior: if the ship can do it, a well is in
   range, and the AI is in a passive state (patrol/idle, not in combat),
   it enters orbit on its own. The AI state machine
   (crates/nova_gameplay/src/input/ai.rs, next_behavior_state) already has
   the passive/engaged split to hang this on, and the ORBIT autopilot verb
   (AutopilotAction::Orbit { well, plan }) is the flying substrate.

These compose: 3 gives ambient life everywhere, 2 gives scenario authors
control, 1 is the config surface for both. Spike should pick the seam.

Notes:
- Spike DONE (2026-07-11): docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
  answered both halves. Direction: gate the spaceship sets on
  scenario-liveness (CurrentScenario.is_some(), owned by nova_scenario);
  AI orbit via config directive -> AIOrbitDirective -> passive
  AIBehaviorState::Orbit -> AutopilotAction::Orbit. Seeded tasks
  20260711-212519, 20260711-212521, 20260711-212504; this task closes when
  they land.
- Origin: brainstormed while building the menu ambience scene
  (20260711-180455), which could NOT use AI/autopilot orbiting because the
  editor gates all spaceship input/section system sets on its private
  Scenario state - in MainMenu they do not run. That gating is itself
  worth revisiting in this spike (it also blocks any future in-menu
  thruster visuals).
- When this lands, the menu ambience scene's ballistic orbit seeding can
  be replaced by the real AI behavior (thruster visuals included) if the
  gating is resolved.
- Related: AutopilotAction::Orbit (crates/nova_gameplay/src/flight.rs),
  gravity wells (crates/nova_gameplay/src/gravity.rs), AI behavior states
  (crates/nova_gameplay/src/input/ai.rs).

