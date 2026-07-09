# AI threat-scored target selection over the relation model

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.4.0,ai,spike,targeting


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 1)

Goal: replace the hardcoded player Single in input/ai.rs with a per-ship
AITarget(Option<Entity>) picked over hostiles via the relation model
(20260708-203708): score by distance, recent-damage-to-me and target type
(ships over torpedoes over asteroids), with switch hysteresis so the pick
does not flip-flop between frames. All four AI systems (rotation, thrust,
turret target, fire) consume AITarget instead of querying the player.

Blocked on: 20260708-203708 (faction/relation model).
Depends on: 20260709-225726 (state skeleton).
