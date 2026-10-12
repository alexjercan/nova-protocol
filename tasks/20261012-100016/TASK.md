# Fleet: hire an escort at a station

- STATUS: OPEN
- PRIORITY: 73
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 13 (2026-10-12): the fleet half of D13 lands right after Stage 2: escorts for sale at any living station until the stance graph lands, then at Ally rosters; the fleet list lives in the player save.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- AI follow and patrol behaviours exist (`crates/nova_ship/src/input/ai/`); `FrozenAI` keeps behavior, patrol, leash (`crates/nova_ship/src/input/ai/frozen.rs:27-34`). Allies are `Allegiance::Player` (`crates/nova_gameplay/src/relations.rs:37-44`).

## Delivery gate
- A `fleet` field in `SavedPlayer`; an escort is a generated hull with `Allegiance::Player` and a follow order; it crosses sectors with the player (not frozen in the old cell). Stop for the owner on the escort's death rule (gone for good vs. rebuyable).

## Verification
- Asserted example: hire, cross a sector, save, load, the escort is beside the player and still follows.

## Done when
- Hire works in a live run and the escort survives a crossing and a reload.
