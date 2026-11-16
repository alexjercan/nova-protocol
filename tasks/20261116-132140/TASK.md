# Crafting rework: station and on-ship recipes, more items, consumables

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.19.0, economy, crafting, content

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- Wants more items, more crafting and refine methods, and consumables for the
  ship.
- "the market+inventory+crafting system can be re-worked a bit, how we add
  sections to a ship, how we lose them and get them back".
- Some recipes need a station; others can be made on the ship, "just like
  minecraft has 2x2 vs 3x3 crafting".
- Owner call 2026-11-16: this reverses "No crafting tree"
  (`tasks/20261012-095526/DESIGN.html:219`), "crafting is not a goal"
  (`tasks/20261012-095526/RESEARCH-CODEBASE.md:267`) and the no-consumables
  scope (`tasks/20260926-174806/TASK.md:10,16`).
- Still on record, not reversed: no blueprints or known-design set
  (`DESIGN.html:499`).

## Decisions
- Owner 2026-11-16: more recipes mean the Inventory pane's Refine verb is
  not enough. Crafting gets its own UI, probably modelled on the Ship pane
  section catalogue (`crates/nova_interface/src/ship/catalogue.rs`, gallery
  in `crates/nova_ship/src/gallery/`). Share any catalogue UX fix with task
  20261116-133436.

## Agent findings
- 23 items in six categories (`assets/base/items/base.content.ron:3-236`,
  `crates/nova_gameplay/src/inventory.rs:391-406`); builders in
  `crates/nova_authoring/src/base_content/items.rs`.
- One refine recipe: AlloyPlate = 2 IronOre + 1 CarbonOre + 30 cr
  (`items/base.content.ron:213-233`). 26 section build recipes
  (`inventory.rs:1829-1838`). Lint refuses a refine that loses money
  (`crates/nova_assets/src/items.rs:179-214`).
- Refine is station-only, from advancement 0.25
  (`inventory.rs:1193-1195`, `crates/nova_world_base/src/station_layout.rs:143,568`).
- No consumables: thrust uses no stock (`inventory.rs:387-388`); Medkit and
  Rations are job and trade goods only.
- Inventory pane: one form for Give, Sell, Store, Buy, Take, Withdraw and
  Refine, about 8.3k lines (`crates/nova_interface/src/inventory/mod.rs`).
- Market rules are keyed by category, never by item
  (`crates/nova_gameplay/src/market.rs:23-25`).
- Item, recipe and beam prices are provisional
  (`tasks/20261111-122537/TASK.md:20-23`).

## Open decisions (owner)
- On-ship tier: built into every hull, or needs a fabricator section.
- Recipe list for each tier, and which items are new.
- Consumable kinds (agent ideas, not approved: field repair kit, ammo packs,
  thrust boost, sensor flare, decoy).
- Whether sections become items, or stay station-built only.
- Scope of the Inventory and Market pane rework.

## Delivery
1. Design page in this task folder (`DESIGN.html`): item list, recipe tiers,
   consumables, UI flow, how it links to field repair and ammo. Owner
   approves before code.
2. Content format and lint first (explicit fields, known IDs), then
   builders, `content -- gen`, then runtime and UI.
3. Changelog marks format breaks **(breaking)**; migrate the example mod.

## Verification
- Unit tests for recipe validation and lint refusals.
- ECS asserted example: an on-ship recipe crafts in flight; a station recipe
  refuses undocked.
- A playable example for the crafting loop.

## Done when
- The approved design is implemented: on-ship and station recipes work,
  consumables have an effect in flight, and the wiki explains the loop.
