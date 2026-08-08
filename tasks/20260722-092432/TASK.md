# Ally convoy haulers loiter/orbit the belt instead of drifting into the planetoid (lifeline)

- STATUS: CLOSED
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

- [x] Verify-first: harness check that spawns the lifeline convoy and asserts,
      after N seconds, the haulers are still in the belt region (near their
      loiter anchor / within a radius band of the body) and have NOT reached
      the planetoid surface. Fails today (they drift in).
- [x] Give the convoy haulers a passive AI controller instead of None: an
      Orbit directive around the belt body, or a Patrol route through
      belt waypoints, with a leash so they stay in-region. Confirm they have
      (or are given) enough thruster section to actually fly the plan.
- [x] Confirm the unarmed haulers never enter Engage (no weapons => no target
      acquisition); they should remain non-combatants that just move.
- [x] Keep them targetable by the raider waves (allegiance stays Player) so
      the "keep the convoy alive" objective still works; verify the defense
      scenario still plays.
- [x] Regen content if generated from these builders
      (`content -- gen`), lint clean; never hand-edit generated RON.
- [x] Docs sweep: scenario-authoring note on non-combatant loiter/orbit
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

## Fix (2026-07-22)

Verify-first sharpened the premise (as in the sibling gravity task 092427):
lifeline has NO gravity well/planetoid, and the haulers spawn AT REST, so the
owner's "crash into the planetoid" is KNOCKBACK DRIFT - a raider collision or
explosion shoves a thrust-less `controller: None` hauler and it drifts forever
with nothing to arrest it. So "orbit the body" is not available (no well); the
fix is active loitering that also recovers from a shove.

Three pieces:
1. NEW non-combatant AI (nova_gameplay): an `AINonCombatant` marker; while set,
   `update_ai_target` skips the ship and keeps its `AITarget` clear, so
   `update_behavior_state` always reads "nothing hostile" and holds the passive
   routine. A weaponless AI ship would otherwise chase raiders (the FSM does not
   gate Engage on having weapons). It stays TARGETABLE (allegiance unchanged),
   so a Player convoy is still hunted.
2. AUTO-DETECT (nova_scenario): `insert_spaceship_sections` tracks whether any
   turret/torpedo section was spawned; an AI ship with none gets `AINonCombatant`
   at spawn. No AIControllerConfig field (would have churned ~13 sites) - it is
   derived from the loadout, and dovetails with the future critical-damage
   backlog (weapons-destroyed => non-combatant, the dynamic version).
3. CONTENT (lifeline): `convoy_hauler` is now `controller: AI` with a `patrol`
   loiter loop per hauler (legs > the ~75u arrival radius so they FLY the loop,
   centred on the holding station, staying in the belt). The cargoa hull already
   has two thrusters and a controller, so no craft change was needed.

Alternatives weighed: (a) station-keep only (empty patrol => Idle STOP-on-drift)
- fixes the crash but not "fly around"; chose patrol for the owner's "fly around,
stay in belt". (b) a `non_combatant` config flag vs auto-detect - chose
auto-detect (less churn, more correct). (c) leash-to-zero to force passivity -
rejected (a raider inside the leash would still trip Engage).

Coverage:
- nova_gameplay `a_non_combatant_never_targets_or_engages` (gate skips it;
  armed control still engages).
- nova_scenario `an_unarmed_ai_ship_is_flagged_non_combatant` (auto-detect:
  unarmed AI tagged, armed AI not, player never).
- lifeline_convoy integration: the haulers are AI with a loiter patrol.
- lifeline PROBE walk (real physics): both haulers still within 200u of their
  loiter centres after two waves of fire (in-region, no drift-off), the
  screen objective is live, invariants hold, log_clean PASS. Fails-first
  against the old drifting None haulers.
- CHANGELOG (Gameplay & Flight: non-combatant rule + convoy loiters); authoring
  guide + lifeline module doc updated.

Depended on the gravity-hold task 092427 (a hauler that gains an AI pilot opts
back into gravity coherently) and was BLOCKED mid-task by a task-1 pacing
regression (OnStart gate stamps read undefined scenario_elapsed) that the
lifeline probe surfaced - fixed separately as 20260722-114541, then this branch
rebased onto it (the probe is clean only with both).
