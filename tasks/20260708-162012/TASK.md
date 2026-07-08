# Smarter enemy AI (target selection, evasion, patrol)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0, ai

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md (roadmap)

Today the AI brain (`crates/nova_gameplay/src/input/ai.rs`) only targets and
shoots the nearest asteroid; `input/player.rs` carries TODOs about needing a
better target-selection heuristic and a "scuffed" targeting path to clean up.
Direction: give AI ships real behaviours - target selection (threat/priority, not
just nearest), evasion under fire, patrol/idle states, and firing discipline
(bursts/lead). This is the enabler for enemies as a gameplay loop rather than
inert asteroids, and pairs with the mission/objective work (133026-133029).
</content>
