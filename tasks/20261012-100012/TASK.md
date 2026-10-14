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

## Sub-choices for owner review (2026-10-14)
The family rule is implemented as stated: same `SectionKind::class` and same `SectionFootprint` from the authored collider (`section_swap_family`, `crates/nova_ship/src/sections/design_sections.rs`). Choices made inside it:
- Offers come from the section CATALOGUE, not a per-station part list: every non-`hide_in_editor` part that passes the family rule (`swap_offers`). "From the station's stock" is met through the recipe: the parts the hold lacks are bought from the station stock at ask, so a station with no turret bodies cannot sell a turret swap. Consequence: no station-specific offer list exists yet; a later rung can add one as a filter over `swap_offers`.
- The gate is `StationServiceType::Refit` on a living station (`station_offers`), the same gate shape as Rebuild. `StationSite` in `crates/nova_interface/src/ship/rebuild.rs` now takes the service.
- The part the section already is (`SamePart`) is refused, so it is never offered.
- The price is the NEW part's full build recipe, paid by the shared `plan_recipe_payment` (`crates/nova_gameplay/src/inventory.rs`): hold parts first, the rest bought. The old part is removed with no strip refund.
- Post-build checks: `swap_build_fault` reuses the generator's socket graph (`derive_link_point_graph`), exit lanes (`blocked_exits`) and floors (`ShipMeasure`, moved from `ship_layout.rs` to `crates/nova_ship/src/sections/ship_measure.rs` so both callers share one copy). Only a fault the swap ADDS refuses. Role, advancement, size ceiling, mirror symmetry and ruin floor are not checked: a swap answers no generation request, and swapping one of a mirrored pair breaks symmetry by design.
- The build check runs at click, not per frame in the list; the list shows price or payment refusal only.
- After the first swap the ship's design is inline (`owned_ship_design` in `crates/nova_scenario/src/objects/spaceship.rs`): a catalogue design is copied, patched sections become inline configs, unpatched sections stay prototype references, and the swapped section names the new prototype with no patch (a patch on the old part is dropped).
- The root's pinned health max moves by the difference between the two parts' pools, so the collapse share is a share of the ship as built now.
- Event `OnSwapped` (`section` code, `from`, `to`, `parts`, `price_cr`) and credit cause `Swap`, following `OnRebuilt`/`Rebuild`.
- The section panel clips at small windows: a damaged section's repair form pushes the swap list below the panel at 1024x768. The proof runs at 1920x1080. A scrolling side panel is not part of this task.

## Verification
- Asserted example: swap one turret, freeze and thaw, assert the new prototype id, fresh health and the old binding; a cross-family swap refuses.

## Done when
- Swap works in a live docked run and the refusal is a permanent proof.
