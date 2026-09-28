# Add enemy, neutral, and allied ship encounters to the open world

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.15.0,world,encounters,design

## Goal

Let the streamed world place small, seeded groups of enemy, neutral, and allied ships as encounters, including combat zones the player can fly into. Start with a playable example, then judge whether this makes the world better. A full faction, standing, reputation, or station-job system is a later decision, not part of this task.

## Starting point

- `crates/nova_gameplay/src/relations.rs` has Player, Enemy, and Neutral combat allegiances; there is no distinct Allied allegiance. Decide the simplest way to express allies without silently changing how the existing sides target one another.
- `crates/nova_world_base/src/clusters.rs` places unpiloted derelicts; `crates/nova_world/src/streaming.rs` spawns them with no controller and neutral allegiance. It does not yet make live ship encounters.
- `20260923-110307` owns regional ship designs and looks. This task owns whether, where, and how encounter ships appear and behave. `20260925-190156` owns what happens to an encounter after the sector retires.

## Decisions before implementation

Choose a first enemy/neutral/allied encounter, its seed and location rule, ship/AI source, combat relations, entry and outcome cues, and retirement/revisit policy. Test it in flight before generalizing. Do not add a generic faction/reputation system or a station job board just to spawn the first encounter.

## Done when

The owner has reviewed a code-backed first encounter and its lifetime/AI rules; after separate implementation approval, a seeded player flow shows all three ship roles and verifies combat, neutral, and allied behavior without changing authored scenarios unexpectedly.
