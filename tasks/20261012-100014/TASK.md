# Upgrades: buy a whole ship from a tiered station catalogue

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 6, third rung: buy an entire ship at a station; the catalogue is tiered by the station's advancement; the old hull is sold or kept in station storage is NOT in scope (sections are never items; a hull is not an item either). Decide the fate of the old hull with the owner.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Generated hull sizes follow advancement (`crates/nova_world_base/src/ship_layout.rs:89-95`); `plan_ship` lays out hulls by role (`crates/nova_world_base/src/sector_ships.rs:1-35`).

## Delivery gate
- The catalogue is generated from the station's civilization and advancement, deterministic from the seed; buying swaps the player's design and respawns the hull docked. Stop for the owner on the old hull (trade-in credit vs. left as a derelict).

## Verification
- Asserted example: buy, undock, save, load, the new hull is the player's; the catalogue of a ring-1 station holds only small hulls.

## Done when
- Buy works in a live run and the catalogue tiers are a permanent test.
