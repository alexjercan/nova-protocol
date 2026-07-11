# AI orbit directive: config, passive behavior state, autopilot wiring

- STATUS: CLOSED
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

- [x] Add `AIOrbitDirective { well: EntityId }` component in
      crates/nova_gameplay/src/input/ai.rs next to AIPatrolRoute (register
      type, reflect). EntityId is nova_events' string id newtype.
- [x] Add `Orbit` variant to AIBehaviorState (ai.rs ~490): passive, no fire,
      `engages()` stays false for it. Added an `is_passive()` helper so the
      transition match names the passive family once.
- [x] Thread the directive through next_behavior_state (ai.rs ~585): the
      passive fallback becomes orbit > patrol > idle (has_orbit bool param,
      function stays pure). Extended the unit tests for the precedence, the
      far-hostile hold, the Engage pull-in (range and shot-from-afar), and
      the calm return to Orbit.
- [x] update_behavior_state (ai.rs ~639): read Has<AIOrbitDirective> and
      pass it through.
- [x] update_passive_flight (ai.rs ~742): new Orbit arm - resolve the
      directive's EntityId to the well entity (Query<(Entity, &EntityId),
      With<GravityWell>>) and, when no Autopilot is engaged, insert
      Autopilot::engage(AutopilotAction::Orbit { well, plan: None }).
      The autopilot self-plans on first tick (flight.rs ~1219) and never
      auto-completes (flight.rs ~1608), and it already disengages itself if
      the well disappears, so re-engage simply retries next frame.
      Unresolvable id: state still Orbit, ship drifts, debug_once log.
- [x] Config surface: add the orbit field to AIControllerConfig
      (crates/nova_scenario/src/objects/spaceship.rs) and insert
      AIOrbitDirective in insert_spaceship_sections alongside the
      AIPatrolRoute mapping. Plus a mapping test (orbit -> directive,
      patrol -> route, absent -> nothing).
- [x] Integration-style test (orbit_directive_tests, mirroring
      patrol_idle_tests' run_system_once pipeline): engage-on-well,
      orbit-beats-patrol, unresolvable-id drift with late-well delivery
      guard, mid-flight no-churn, and combat-interrupts/calm-resumes.
- [x] Verify: cargo check + fmt, run the newly written tests.

## Notes
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- Second of three seeded tasks; the menu payoff task 20260711-212504 needs
  this and 20260711-212519.
- Open question resolved: kept two config fields (orbit: Option<String>
  next to patrol: Vec<Vec3>) over a passive-behavior enum - both set is
  legal and documented (orbit wins; the patrol route is simply shadowed),
  and the enum would have broken every existing patrol config for no
  behavioral gain today. Revisit if a third passive routine appears.
- Scenario event actions (runtime "go into orbit") and autonomous orbiting
  are deliberate non-goals here; both layer later as writers of the same
  AIOrbitDirective / Orbit state (see spike Options 2 and 3).

## Close record (2026-07-11)

What changed: AIBehaviorState::Orbit (passive, engages() false so every
combat consumer ignores it), AIOrbitDirective { well: EntityId } mirroring
AIPatrolRoute end to end (config -> component -> passive fallback ->
autopilot verb), passive precedence orbit > patrol > idle in the pure
next_behavior_state (new has_orbit param; 18 test call sites updated),
an Orbit arm in update_passive_flight that resolves the well by scenario id
and keeps AutopilotAction::Orbit engaged, and the orbit field in
AIControllerConfig with its insert_spaceship_sections mapping.

Design notes: the Orbit arm (re)engages when nothing is engaged OR the
directive was retargeted to a different well (review R1.2, the ORBIT
analogue of the patrol arm's leg_changed); an autopilot already circling
the right well is left alone, and a stale non-ORBIT maneuver (e.g. a
patrol GOTO when a directive is hot-inserted) flies out before Orbit takes
over.
Unresolvable well ids are spawn-order tolerant: the ship sits in Orbit
state drifting and the engage retries every calm frame, so a well spawned
later is picked up (tested). ORBIT never self-completes and self-disengages
if the well dies, so the single-engage design holds the ring indefinitely.

Verification: cargo check --workspace green, fmt applied; new tests all
green - behavior_state_tests::an_orbit_directive_wins_the_passive_fallback,
5 orbit_directive_tests, spaceship::tests::ai_config_maps_to_directive_
components; existing behavior/patrol suites (9 + 10 tests) still green.
Per repo policy the full suite runs in CI.

Difficulties: one compile error - EntityId does not derive PartialEq, so
AIOrbitDirective dropped PartialEq from its derive (AIPatrolRoute has it;
the directive does not need it). Otherwise mechanical; the spike and plan
had pre-verified the seams (self-planning ORBIT, never-done semantics,
engages() coverage) so no surprises surfaced during implementation.

Self-reflection: updating 18 pure-function call sites for the new bool
param was a sed-able mechanical change and worth it to keep the function
pure and signature-honest; a passive-kind enum would have been prettier in
the signature but noisier at every call site. The one thing to watch in
review: Orbit ships hold OUT of combat only until AI_ENGAGE_RANGE - for
menu use this is irrelevant (no hostiles), but scenario authors combining
orbit directives with nearby hostiles get a fight, not a scenic orbit.
