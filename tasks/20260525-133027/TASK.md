# Implement HUD for objectives

- PRIORITY: 0
- TAGS: wontdo, objectives
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Display current objective state to the player. Legacy #90.

CLOSED (wontdo, 20260714): shipped differently in v0.5.0. Objective conveyance
landed as a gold marker chip with live distance, salvage glow/brackets, keybind
emphasis pulses, and a completion chime (`crates/nova_gameplay/src/hud/objective_feedback.rs`,
`objective_marker.rs`, `hud/item_highlights.rs`). The legacy "objectives HUD" idea
is satisfied.
