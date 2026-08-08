# Smarter enemy AI (target selection, evasion, patrol)

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.4.0, ai

Spike: tasks/20260708-161726/SPIKE.md (roadmap)

Today the AI brain (`crates/nova_gameplay/src/input/ai.rs`) only targets and
shoots the nearest asteroid; `input/player.rs` carries TODOs about needing a
better target-selection heuristic and a "scuffed" targeting path to clean up.
Direction: give AI ships real behaviours - target selection (threat/priority, not
just nearest), evasion under fire, patrol/idle states, and firing discipline
(bursts/lead). This is the enabler for enemies as a gameplay loop rather than
inert asteroids, and pairs with the mission/objective work (133026-133029).
</content>

## Superseded (20260709)

Split by spike tasks/20260709-225508/SPIKE.md into nine
behavior-level tasks (20260709-225726 .. 20260709-225734) on a shared
AIBehaviorState skeleton, with 20260708-203708 (factions) and
20260709-155921 (AI rotation path) as the unchanged prerequisites. Closed as
superseded, not implemented - the work continues in those tasks.
