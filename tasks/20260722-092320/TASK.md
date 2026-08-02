# Backlog: critical-damage state - a ship is combat-dead when weapons+thrusters are destroyed

- PRIORITY: 40
- TAGS: v0.9.0, gameplay, feature
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

Playtest verdict (owner, 2026-07-22): killing a ship today means grinding
every section's health to zero, which is tedious and sometimes leaves you
stuck (see the sibling kill-condition backlog item 20260722-092326). The
owner wants a real notion of a ship being "out of the fight": once its
weapons AND thrusters are gone it can neither shoot nor maneuver, so it
should count as combat-dead / mission-neutralized even with hull sections
still intact. This applies to the PLAYER too (a new lose/critical condition
when you can no longer fight), not just AI ships.

This is a NEW FEATURE, deliberately deferred to the backlog. Filed now so it
is not lost; not to be implemented in the current pacing/ship-behavior goal
(umbrella 20260722-092316).

## Design

Decided design + forks live in `DECISION.md` (ACCEPTED). Summary: a ship that
was armed at spawn and now has zero working weapon sections AND zero working
thruster sections becomes NEUTRALIZED - a distinct inert-wreck state (new
`NeutralizedMarker`, ship lingers, combat AI off, NOT despawned). It fires a
new distinct `OnNeutralizedEvent`; the player ship neutralizing declares
immediate Defeat. Unarmed ships (no turret/torpedo at spawn) are never
neutralized, only destroyed.

## Steps

- [x] Add `OnNeutralizedEvent` + `OnNeutralizedEventInfo { id, type_name }` in
      `crates/nova_events/src/lib.rs` (mirror `OnDestroyedEvent` at
      `nova_events/src/lib.rs:75-90`, event_name `"onneutralized"`).
- [x] Add `EventConfig::OnNeutralized` -> `EventHandler::new::<OnNeutralizedEvent>()`
      in `crates/nova_scenario/src/events.rs:17-52` (mirror `OnDestroyed` at
      `events.rs:21,45`). Confirm the existing `Entity` filter
      (`nova_scenario/src/filters.rs:38-133`) matches it by id/type_name
      unchanged.
- [x] Add a `NeutralizedMarker` component + neutralization system in
      `nova_gameplay` (new module under `src/integrity/`, registered next to
      the destroy pipeline in `explode.rs`). Each frame, for every
      `SpaceshipRootMarker` root that is armed-at-spawn, not already
      `NeutralizedMarker`, count live weapon sections (Turret/Torpedo children
      without `SectionInactiveMarker`) and live thruster sections (Thruster
      children without `SectionInactiveMarker`) - the "working section" query
      is the same `Without<SectionInactiveMarker>` gate the weapon/thruster
      systems use (`sections/turret_section.rs:945`, `thruster_section.rs:327`,
      `torpedo_section/mod.rs:622`). When both counts are 0: insert
      `NeutralizedMarker`, read `EntityId`/`EntityTypeName` and
      `commands.fire::<OnNeutralizedEvent>(...)` (mirror
      `explode.rs:159-179`).
- [x] Record "armed at spawn": stamp each root with a component/flag when it
      first has >=1 Turret/Torpedo section (or read the spawn loadout), so an
      unarmed hull never satisfies the predicate. Cite the section-spawn site
      used.
- [x] On neutralize, take an AI ship out of the fight: insert `AINonCombatant`
      (`nova_gameplay/src/input/ai.rs:129-142`) on the neutralized root so it
      stops being targeted/chased. Player ship keeps its marker; the Defeat
      comes from the scenario `OnNeutralized` handler (next step).
- [x] Wire scenario handlers (see DECISION.md audit). For each ARMED enemy
      "destroy X" objective, add an `OnNeutralized` sibling mirroring its
      `OnDestroyed` objective actions (idempotent `VariableSet(=1)` +
      `ObjectiveMarkerDetach`, preserving any narrative guard). For each
      `player_spaceship` `OnDestroyed` Defeat handler (all 6 scenarios), add an
      `OnNeutralized` sibling firing the same `Outcome(Defeat)` +
      `NextScenario(retry)`. Confirm at edit time that each `hauler` /
      `derelict` / `hauler_queen` / `hauler_meridian` is actually unarmed
      (leave destroy-only) and mirror any that turn out armed.
- [x] Guard `outcome-is-last-write-wins-close-the-act`: the player-neutralized
      Defeat path sets a terminal act exactly like player-death Defeat; sweep
      by class (every neutralize-driven outcome handler), not just the
      motivating scenario.
- [x] Harness coverage (see Definition of Done). Prefer the integrity
      `test_support` app + a scenario-level test mirroring
      `crates/nova_assets/tests/broadside_assault.rs` (drive real sections /
      real scenario, not a hand-built stand-in - `review-rig-can-false-green`).
- [x] Docs sweep (`keep-docs-in-sync-with-code`): new event kind + new
      `EventConfig` are a doc surface - update the scenario-system / event
      enumerations in the dev wiki and any content-kind list, plus CHANGELOG.
      `grep -rn 'OnDestroyed' web/ README* AGENTS* CHANGELOG*` and add the
      neutralize counterpart where the destroy event is documented.

## Definition of Done

- Predicate: a ship that was armed at spawn, with zero working weapon sections
  and zero working thruster sections, gets `NeutralizedMarker` and fires
  `OnNeutralizedEvent` once, with hull health still > 0 and the ship still
  present in the world (not despawned).
  (test: nova_gameplay integrity test drives an armed ship's weapon+thruster
  sections to `SectionInactiveMarker`/destroyed, asserts `NeutralizedMarker`
  added + `OnNeutralizedEvent` fired once + root entity still alive + hull
  section health > 0.)
- Unarmed guard: a ship with NO turret/torpedo section at spawn, driven to zero
  working thrusters, does NOT get `NeutralizedMarker` and fires no
  `OnNeutralizedEvent`.
  (test: same harness with an unarmed loadout asserts the null result -
  `delivery-guards-on-null-assertions`: the stimulus (thruster kill) fires in
  the same test.)
- AI neutralized enemy is out of the fight: the neutralized AI root carries
  `AINonCombatant` and is not despawned.
  (test: assert the marker present and the entity still exists after neutralize.)
- Scenario integration: an armed enemy "destroy X" objective completes on
  neutralize (not only on destroy), and does not double-complete or softlock if
  the wreck is later destroyed.
  (test: scenario-level test in the nova_assets style - fire `OnNeutralized`
  for the target id, assert the objective variable/outcome; then fire
  `OnDestroyed` for the same id and assert no regression.)
- Player neutralized => Defeat with a terminal act set (no last-write-wins
  overwrite).
  (test: fire `OnNeutralized` for `player_spaceship`, assert
  `CurrentOutcome` == Defeat and the act is terminal.)
- `cargo check` / fmt clean; docs surfaces updated. CI runs the suite.

## Notes

- Related: kill-condition rethink 20260722-092326 (the immediate annoyance),
  integrity/sections system in nova_gameplay.

## Merged in the kill-condition rethink (2026-07-24, v0.9.0 planning)

Absorbed sibling 20260722-092326 (now CLOSED): "destroying a ship should not
require zeroing every section's health." Same question, inseparable, so this
task now owns BOTH angles - the immediate annoyance (a mostly-wrecked ship that
will not die) and the critical-damage model. The critical-damage predicate
(no working weapons + no working thrusters => combat-dead) is the likely
mechanism for both; the beaten-ship threshold gets designed here too. Tagged
v0.9.0 as the Goal-B STRETCH: cut first if the cockpit-HUD work (Goal C) runs
long. Full DoD/steps to be defined in the v0.9.0 planning pass.
