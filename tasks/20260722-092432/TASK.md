# Ally convoy haulers loiter/orbit the belt instead of drifting into the planetoid (lifeline)

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.8.0, content, ai, scenario

## Story

Playtest verdict (owner, 2026-07-22): in Lifeline the ally convoy of unarmed
haulers just crashes into the planetoid. They have no weapons so they should
not fight, but they should feel alive: fly around, orbit the body, try to
stay in the asteroid-belt area / keep out of danger rather than sitting still
and drifting into the planetoid.

Root cause: the convoy haulers spawn with `SpaceshipController::None`
(convoy_hauler in lifeline.rs) - no AI, no thrust - so they neither hold
station nor move. This is the "active loiter" half of the non-combatant ship
problem; the "float in place" half is sibling task 20260722-092427.

The engine already has the primitives: AIBehaviorState::{Orbit, Patrol, Idle},
AIOrbitDirective, AIPatrolRoute, AILeash, and ORBIT/GOTO autopilots. A ship
with no weapons never acquires a target, so a passive AI controller stays in
Patrol/Orbit/Idle and never engages. This task composes those, it does not
build new AI.

## Depends on

- 20260722-092427 (non-combatant gravity hold) - land that first so the
  gravity behaviour the loiter relies on is settled.

## Steps

- [ ] Verify-first: harness check that spawns the lifeline convoy and asserts,
      after N seconds, the haulers are still in the belt region (near their
      loiter anchor / within a radius band of the body) and have NOT reached
      the planetoid surface. Fails today (they drift in).
- [ ] Give the convoy haulers a passive AI controller instead of None: an
      Orbit directive around the belt body, or a Patrol route through
      belt waypoints, with a leash so they stay in-region. Confirm they have
      (or are given) enough thruster section to actually fly the plan.
- [ ] Confirm the unarmed haulers never enter Engage (no weapons => no target
      acquisition); they should remain non-combatants that just move.
- [ ] Keep them targetable by the raider waves (allegiance stays Player) so
      the "keep the convoy alive" objective still works; verify the defense
      scenario still plays.
- [ ] Regen content if generated from these builders
      (`content -- gen`), lint clean; never hand-edit generated RON.
- [ ] Docs sweep: scenario-authoring note on non-combatant loiter/orbit
      pattern for haulers. CHANGELOG under Scenarios & Objectives (or AI).

## Definition of Done

- The lifeline convoy haulers loiter/orbit within the belt region and do not
  drift into the planetoid; they never fight
  (test: lifeline walk asserts in-region + non-engagement after N seconds;
  manual: owner replays Lifeline, haulers fly around and stay in the belt).
- The "keep the convoy alive" objective and raider waves still function
  (test: existing lifeline walk stays green).
- CHANGELOG entry (cmd: `grep -ni "convoy\|loiter\|hauler" CHANGELOG.md`).

## Notes

- Key symbols: input/ai.rs AIBehaviorState / AIPatrolRoute / AIOrbitDirective /
  AILeash / update_passive_flight; flight.rs AutopilotAction::{Orbit,GotoPos};
  scenario AIControllerConfig (patrol, orbit, leash, grace); lifeline.rs
  convoy_hauler. menu.rs backdrop_orbiter is an existing orbiting-ship example.
- Lesson rename-id-sweep-in-file: lint does NOT validate AI orbit/patrol
  targets, so if any id is referenced by an orbit/patrol directive, grep the
  whole file by hand.
