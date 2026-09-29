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

## Decisions and open gates

- For the first live encounter, represent allies as `Allegiance::Player` (the player's existing combat side). Keep civilization provenance separate; no new `Allied` variant, standing or reputation is implied. This applies to live encounter ships only: the intact static ships from `20260923-110307` remain Neutral regardless of civilization or role.
- The first playable encounter example waits for the shared generated-ship prototype from `20260923-110307` and uses its generated hulls. Do not place authored encounter hulls in the open world as a temporary exception; live world integration remains a separate gate after the prototype.
- A player attack on a neutral live encounter ship makes only that attacked ship retaliate against the player for this encounter. Do not turn its whole civilization hostile or grant lasting reputation. The target-specific AI/relation interface and reset/lifetime are code-backed gates; `Allegiance::Neutral` alone cannot express the retaliation under today's `relation()` resolver.
- Shooting a Player-aligned allied encounter ship does not change its side or make it retaliate; it remains allied. Damage may still occur under the existing damage rules. There is no group alert or reputation change.
- First playable example: one local mixed scene using generated hulls, with one hostile ship attacking, one neutral ship staying out unless the player hits it, and one Player-aligned ally defending against the hostile ship. This proves all three roles together without yet selecting open-world group density or layout.
- First streamed groups use deterministic candidate slots near rock/world clusters, not empty-cell transit scatter. Draw live ships only from reachable living civilizations' generated hulls. Keep their civilization machine identity separate from combat side; seeded manifest, counts and clearance must not depend on visit order or player position. Exact density and clearance are later code-backed gates.
- Prototype lifetime: after sector retirement, defeated live encounters may regenerate at their same seeded slots on revisit. This is temporary, not persistent defeat or claim-once loot; durable lifetime is owned by `20260925-190156`.
- Still gate the AI spawn/target interface (especially target-specific neutral self-defense), encounter density and clearance, entry/outcome cues, and any loot source before implementation. Reuse current AI behavior where it fits rather than promising new group tactics. Test it in flight before generalizing. Do not add a generic faction/reputation system or a station job board just to spawn the first encounter.

## Delivery sequence (proposed, not implementation approval)

1. [ ] After the shared generated-ship prototype exists, map AI spawn markers, scenario ship construction, relation/target acquisition, world cluster candidates, streaming and retirement. Show exact paths, interfaces, call graph, failure policy and affected docs before code edits. Do not change `20260923-110307`'s static Neutral allegiance.
2. [ ] Gate a playable mixed example with three generated ships: Enemy attacker, initially non-aggressive Neutral, and Player-aligned defender. Prove the neutral retaliates only after the player's hit, the ally stays allied when hit, and the ally engages the enemy. Inspect actual flight/combat results, not spawn counts alone. Keep example-only fixtures out of world selection.
3. [ ] Gate a deterministic streamed encounter manifest at anchored clusters, drawing eligible generated ships of reachable living civilizations, with stable candidate IDs, clearance and density. Define who pilots each role, entry/outcome cues, what happens on neutralization, and exact failure/skip diagnostics; then seek runtime implementation approval. Civilization name must not determine hostility or allies.
4. [ ] Verify the same seed/content makes the same encounters independent of visit order or player position; inspect rendered travel into a combat zone, all three sides' actions, stand-down on neutralization, and repeatable respawn after sector retirement. Coordinate any drop/claim-once or durable absence with `20260925-190156` and the item-loop stack. Keep faction reputation and station jobs out of this delivery.

## Done when

The owner has reviewed a code-backed first encounter and its lifetime/AI rules; after separate implementation approval, a seeded player flow shows all three ship roles and verifies combat, neutral, and allied behavior without changing authored scenarios unexpectedly.
