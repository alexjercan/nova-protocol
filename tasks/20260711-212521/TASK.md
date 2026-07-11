# AI orbit directive: config, passive behavior state, autopilot wiring

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,ai,scenario,spike

## Goal

An AI ship can be directed to orbit a gravity well. Config surface:
AIControllerConfig grows an orbit directive (well EntityId string) next to
`patrol`; it maps to a per-entity AIOrbitDirective component mirroring
AIPatrolRoute; a new passive AIBehaviorState::Orbit keeps
AutopilotAction::Orbit engaged the way Patrol keeps a GOTO engaged. Combat
states (Engage/Evade) override and the orbit resumes when calm returns.

## Steps

- [ ] Add `AIOrbitDirective { well: EntityId }` component in
      crates/nova_gameplay/src/input/ai.rs next to AIPatrolRoute (register
      type, reflect). EntityId is nova_events' string id newtype.
- [ ] Add `Orbit` variant to AIBehaviorState (ai.rs ~490): passive, no fire,
      `engages()` stays false for it.
- [ ] Thread the directive through next_behavior_state (ai.rs ~585): the
      passive fallback becomes orbit > patrol > idle (has_orbit param or a
      small passive-kind enum; keep the function pure). Extend its unit
      tests for the new precedence and the Engage pull-in from Orbit.
- [ ] update_behavior_state (ai.rs ~639): read Has<AIOrbitDirective> and
      pass it through.
- [ ] update_passive_flight (ai.rs ~742): new Orbit arm - resolve the
      directive's EntityId to the well entity (Query<(Entity, &EntityId),
      With<GravityWell>>) and, when no Autopilot is engaged, insert
      Autopilot::engage(AutopilotAction::Orbit { well, plan: None }).
      The autopilot self-plans on first tick (flight.rs ~1219) and never
      auto-completes (flight.rs ~1608), and it already disengages itself if
      the well disappears, so re-engage simply retries next frame.
      Unresolvable id: behave like Idle (no panic), log once at debug.
- [ ] Config surface: add the orbit field to AIControllerConfig
      (crates/nova_scenario/src/objects/spaceship.rs) and insert
      AIOrbitDirective in insert_spaceship_sections alongside the
      AIPatrolRoute mapping.
- [ ] Integration-style test (headless app, the ai.rs test harness at
      ~2232/3372 shows the pattern): an AI ship with an orbit directive and
      a well present ends up with an engaged AutopilotAction::Orbit; with a
      hostile in engage range it drops to combat and the autopilot is
      removed.
- [ ] Verify: cargo check + fmt, run the newly written tests.

## Notes
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- Second of three seeded tasks; the menu payoff task 20260711-212504 needs
  this and 20260711-212519.
- Open question for /plan: two Option fields (orbit + patrol) vs a passive
  behavior enum in config; precedence orbit > patrol is the tie-break either
  way.
- Scenario event actions (runtime "go into orbit") and autonomous orbiting
  are deliberate non-goals here; both layer later as writers of the same
  AIOrbitDirective / Orbit state (see spike Options 2 and 3).
