# Upgrades: same-family section swap at a station

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 6 (2026-10-12), first rung: while docked at a station, swap a section for a same-family, same-footprint one from the station's stock, paid in credits and parts; the generator's post-build checks validate the result; the deleted editor stays deleted.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- `ShipDesignSource::{Inline, Prototype { id, section_patches }}` (`crates/nova_scenario/src/objects/ship_design.rs:303-321`); the player's saved ship thaws the original design (`crates/nova_scenario/src/objects/spaceship.rs:583-615`). Post-build checks live in `crates/nova_world_base/src/ship_layout.rs`.

## Delivery gate
- The player ship is saved inline after the first swap; the swap is a design edit plus respawn of one section, keeping the old binding on that id. Stop for the owner on the family rule (same kind and footprint).

## Verification
- Asserted example: swap one turret, freeze and thaw, assert the new prototype id, fresh health and the old binding; a cross-family swap refuses.

## Done when
- Swap works in a live docked run and the refusal is a permanent proof.
