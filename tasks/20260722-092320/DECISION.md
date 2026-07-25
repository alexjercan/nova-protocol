# DECISION: critical-damage / neutralized-ship model

- STATUS: ACCEPTED (2026-07-25, owner via flow plan gate)

## Context

A ship today only dies when EVERY section's health reaches zero (aggregate in
`crates/nova_gameplay/src/integrity/glue.rs:130-186`), then it despawns and
fires `OnDestroyedEvent` (`.../integrity/explode.rs:159-179`). This is the
"grind every section to zero / a mostly-wrecked ship that will not die"
annoyance (absorbed sibling 20260722-092326). We add a "combat-dead" notion:
a ship with no working weapons AND no working thrusters is out of the fight
even with hull intact.

Sections are child entities of the ship root, tagged Turret / Torpedo /
Thruster / Controller / Hull; a section is "working" when its entity exists
without `SectionInactiveMarker` (`.../sections/base_section.rs:174`). Scenarios
have NO central "ship beaten" registry - each scenario's `OnDestroyed` handler
filters `OnDestroyedEvent` by ship id (`.../nova_scenario/src/events.rs:21,45`,
handlers spawned in `loader.rs:954`).

## The predicate (decided)

A ship becomes NEUTRALIZED when ALL of:

1. it had at least one turret OR torpedo section at spawn (it was an armed
   combatant), AND
2. it now has zero working weapon sections (no live Turret/Torpedo child
   without `SectionInactiveMarker`), AND
3. it now has zero working thruster sections (no live Thruster child without
   `SectionInactiveMarker`).

The "armed at spawn" guard (1) is a correctness gate, not a user fork: without
it, an unarmed hauler/derelict would be "neutralized" the instant its engines
die and its "destroy the hauler" objective would complete without the hull ever
being destroyed. With the guard, unarmed ships are NEVER neutralized - they can
only be destroyed - so their objectives keep meaning "destroy it" and need no
scenario change. Controller is deliberately NOT part of the predicate (the
story names weapons + thrusters; no thrusters already means no maneuver).

## Fork 1 - what "neutralized" IS (owner: DISTINCT INERT-WRECK STATE)

Not "reuse destruction / despawn". Instead:

- new `NeutralizedMarker` on the ship root, inserted once when the predicate
  first holds;
- the ship is NOT despawned - it lingers as a powerless drifting wreck (avian
  RigidBody keeps it drifting; all its weapons/thrusters are already dead by
  the predicate, so it physically cannot act);
- the ship's combat AI is switched off (`AINonCombatant`,
  `.../input/ai.rs:129-142`) so it is taken out of the fight and stops being
  re-acquired / chased.

## Fork 2 - player consequence (owner: IMMEDIATE DEFEAT)

When the PLAYER ship is neutralized, the Defeat outcome fires immediately
(same terminal-act Defeat path scenarios already use for player death). No
intermediate "critical" warning state.

## Fork 3 - scenario-facing signal (owner: PURE DISTINCT `OnNeutralizedEvent`)

Neutralization fires a NEW `OnNeutralizedEvent` (nova_events) with the same
`{id, type_name}` payload as `OnDestroyedEvent`, exposed to scenarios as a new
`EventConfig::OnNeutralized` handler. It does NOT auto-fire `OnDestroyedEvent`.
Scenarios stay able to distinguish "blown up" from "disarmed".

Consequence accepted by the owner: every existing "destroy X" objective that
should ALSO count a neutralize must get an explicit `OnNeutralized` sibling
handler. Audited surface (armed combatants only; unarmed haulers/derelicts are
never neutralized so are left untouched):

- broadside: corvette_a, corvette_b, player_spaceship
- broadside_gunship: gunship (both hauler-fate variants), player_spaceship
- final_tally: picket_a, picket_b, flagship, player_spaceship
- lifeline: raider_1a/1b/2a/2b/2c/3a/3b, player_spaceship
- shakedown_run: pirate, player_spaceship
- asteroid_field: player_spaceship only (the other handler is an asteroid)

Implemented: 15 enemy siblings + 6 player siblings = 21 `OnNeutralized`
handlers. Each mirrors its `OnDestroyed` counterpart's OBJECTIVE / terminal
actions (the idempotent `VariableSet(=1)` + `ObjectiveMarkerDetach`, and the
guarded act-advancing Victory/Defeat blocks) so the mission registers a beaten
ship. Pure-flavour "first-one-down" comms handlers (the two broadside corvette
StoryMessage-only lines) were deliberately NOT mirrored: a neutralize completes
the objective but may skip that one voice beat, which is cosmetic and avoids a
double-spoken line if the wreck is later destroyed.

Unarmed targets left destroy-only by the arming guard: broadside `hauler`,
broadside_gunship `hauler`, lifeline `hauler_queen` / `hauler_meridian`,
shakedown_run `derelict`. (Confirm each of these is actually unarmed at /work
time; if any carries a turret, it gets a sibling too.)

Once-semantics: an armed ship emits its beaten-signal once. If it is neutralized
first, the eventual real hull-destruction must not re-drive the objective; the
mirrored objective actions are idempotent (`VariableSet(=1)` +
`ObjectiveMarkerDetach`), and narrative handlers keep their existing act/flag
guards, so a late destruction after neutralize is benign. /work verifies no
objective double-completes and none softlocks.

## Rejected alternatives

- "Neutralized = destroyed" (despawn / reuse `OnDestroyedEvent`): rejected at
  Fork 1 - owner wants the hull to linger as a wreck and the two states kept
  distinct.
- "Central bridge: neutralize also fires `OnDestroyedEvent`": rejected at Fork
  3 - zero scenario churn but scenarios could not tell disarmed from destroyed.
- Player "critical warning then defeat": rejected at Fork 2 - immediate Defeat.
