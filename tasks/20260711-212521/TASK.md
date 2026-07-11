# AI orbit directive: config, passive behavior state, autopilot wiring

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,ai,scenario,spike

Goal: an AI ship can be directed to orbit a gravity well. Config surface:
AIControllerConfig grows an orbit directive (well EntityId) next to `patrol`
(crates/nova_scenario/src/objects/spaceship.rs); insert_spaceship_sections
maps it to a new per-entity AIOrbitDirective component, mirroring
AIPatrolRoute. Behavior: a new passive AIBehaviorState::Orbit
(crates/nova_gameplay/src/input/ai.rs, next_behavior_state) with passive
precedence orbit > patrol > idle; steering resolves the well EntityId and
keeps AutopilotAction::Orbit { well, plan: None } engaged, the same shape as
Patrol keeping a GOTO engaged. Combat states (Engage/Evade) override and the
orbit resumes when calm returns.

Notes:
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- Second of three seeded tasks; the menu payoff task 20260711-212504 needs
  this and 20260711-212519.
- Open question for /plan: two Option fields (orbit + patrol) vs a passive
  behavior enum in config; precedence orbit > patrol is the tie-break either
  way.
- Scenario event actions (runtime "go into orbit") and autonomous orbiting
  are deliberate non-goals here; both layer later as writers of the same
  AIOrbitDirective / Orbit state (see spike Options 2 and 3).
