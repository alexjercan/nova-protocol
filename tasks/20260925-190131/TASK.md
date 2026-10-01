# Add enemy, neutral, and allied ship encounters to the open world

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.15.0,world,encounters,design

## Goal

Let the streamed world place small, seeded groups of enemy, neutral, and allied ships as encounters, including combat zones the player can fly into. For this first release, assign each living civilization a stable player-facing combat allegiance (Player, Enemy, or Neutral), then let that allegiance determine its live encounter ships' starting side. Start with a playable example, then judge whether this makes the world better. Dynamic standing, trade-driven reputation, relations between civilizations, and station jobs belong to a separate backlog task. Do not start implementation yet.

## Starting point

- `crates/nova_gameplay/src/relations.rs` has Player, Enemy, and Neutral combat allegiances; there is no distinct Allied allegiance. Decide the simplest way to express allies without silently changing how the existing sides target one another.
- `crates/nova_world_base/src/clusters.rs` places unpiloted derelicts; `crates/nova_world/src/streaming.rs` spawns them with no controller and neutral allegiance. It does not yet make live ship encounters.
- `20260923-110307` owns regional ship designs and looks. This task owns whether, where, and how encounter ships appear and behave. `20260925-190156` owns what happens to an encounter after the sector retires.

## Decisions and open gates

- For the first live encounter, represent allies as `Allegiance::Player` (the player's existing combat side); use `Allegiance::Enemy` and `Allegiance::Neutral` for the other two starting sides. Seed a stable player-facing allegiance for each living civilization, independent of its role, advancement, appearance, and living/extinct draw. Carry that value into live encounter ship construction; keep the civilization's stable machine identity separate from the combat component. The existing unpiloted static ships from `20260923-110307` remain Neutral until an explicitly reviewed integration decides otherwise; this task must not silently retag them. No new `Allied` variant or dynamic reputation is implied.
- The first playable encounter example waits for the shared generated-ship prototype from `20260923-110307` and uses its generated hulls. Do not place authored encounter hulls in the open world as a temporary exception; live world integration remains a separate gate after the prototype.
- A player attack on a neutral live encounter ship makes only that attacked ship retaliate against the player for this encounter. Its civilization's seeded starting allegiance does not change, other ships of that civilization do not change sides, and no lasting reputation is granted. The target-specific AI/relation interface and reset/lifetime are code-backed gates; `Allegiance::Neutral` alone cannot express the retaliation under today's `relation()` resolver.
- Shooting a Player-aligned allied encounter ship does not change its side or make it retaliate; it remains allied. Damage may still occur under the existing damage rules. There is no group alert or reputation change.
- First playable example: one local mixed scene using generated hulls, with one hostile ship attacking, one neutral ship staying out unless the player hits it, and one Player-aligned ally defending against the hostile ship. This proves all three roles together without yet selecting open-world group density or layout.
- First streamed groups use deterministic candidate slots near rock/world clusters, not empty-cell transit scatter. Draw live ships only from reachable living civilizations' generated hulls. Keep their civilization machine identity separate from combat side; seeded manifest, counts and clearance must not depend on visit order or player position. Exact density and clearance are later code-backed gates.
- Prototype lifetime: after sector retirement, defeated live encounters may regenerate at their same seeded slots on revisit. This is temporary, not persistent defeat or claim-once loot; durable lifetime is owned by `20260925-190156`.
- Before implementation, gate where the civilization allegiance is derived and pinned, how the deterministic seed selects and represents it, what happens if no reachable living civilization has a required side, and how to avoid changing existing Neutral static ships. Gate the AI spawn/target interface (especially target-specific neutral self-defense), encounter density and clearance, entry/outcome cues, and any loot source. Do not reroll civilizations or ships to force a three-side scene in the streamed world; the controlled example may select known civilizations. Reuse current AI behavior where it fits. Test it in flight before generalizing. Do not add dynamic reputation or a station job board just to spawn the first encounter.

## Delivery sequence (proposed, not implementation approval)

1. [ ] After the shared generated-ship prototype exists, map civilization identity/seed ownership, the stable allegiance decision, AI spawn markers, scenario ship construction, relation/target acquisition, world cluster candidates, streaming and retirement. Show exact paths, interfaces, call graph, failure policy and affected docs before code edits. Do not change `20260923-110307`'s static Neutral allegiance without a separate explicit gate.
2. [ ] Gate a playable mixed example with three generated ships from civilizations assigned different starting allegiances: Enemy attacker, initially non-aggressive Neutral, and Player-aligned defender. Prove the neutral retaliates only after the player's hit without changing its civilization's side, the ally stays allied when hit, and the ally engages the enemy. Inspect actual flight/combat results, not spawn counts alone. Keep example-only fixtures out of world selection.
3. [ ] Gate a deterministic streamed encounter manifest at anchored clusters, drawing eligible generated ships of reachable living civilizations and deriving each live ship's starting combat allegiance from its civilization identity, with stable candidate IDs, clearance and density. Define who pilots each side, entry/outcome cues, what happens on neutralization, and exact failure/skip diagnostics; then seek runtime implementation approval. Civilization display name, ship role, and appearance must not determine allegiance.
4. [ ] Verify the same seed/content gives each civilization the same starting allegiance and makes encounters independent of visit order or player position; inspect rendered travel into a combat zone, all three sides' actions, target-only neutral retaliation, stand-down on neutralization, and repeatable respawn after sector retirement. Coordinate any drop/claim-once or durable absence with `20260925-190156` and the item-loop stack. Keep dynamic reputation and station jobs out of this delivery.

## Done when

The owner has reviewed the civilization-to-starting-allegiance, first-encounter, and lifetime/AI interfaces; after separate implementation approval, a seeded player flow shows Enemy, Neutral, and Player-aligned encounter ships, including ship-only neutral retaliation, without changing authored scenarios or static generated ships unexpectedly. Dynamic faction reputation remains open in the separate backlog task.
