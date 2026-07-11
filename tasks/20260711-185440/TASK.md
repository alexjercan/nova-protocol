# AI orbit behavior: config-driven AI behaviors / scenario AI commands

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,ai,scenario,spike

Goal: let designers make AI ships DO things beyond patrol/combat. User
direction (2026-07-11), three candidate shapes to weigh in a spike:

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

