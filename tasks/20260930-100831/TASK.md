# Design moddable item identities and open-world item integration

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,items,mods,design

## User facts
- Item identity for mod-added items and related open-world content is a separate task split from `tasks/20260926-174806/TASK.md`. M5 trade/mining comes first with its existing closed `ItemType` unless a separately approved interface requires otherwise.

## Design gate
- Inventory, canister cargo, jettison, ammo refill, loot, mining, markets, UI, commands and authored stock must agree on one stable typed item ID. Map current consumers and mod content loaders before changing the owning interface.
- Define mod-pack ownership, canonical ID namespace/order, validation and duplicate/conflict failure, per-item mass/quantity/category/display, eligible acquisition and consumption, effective-catalog fingerprint, missing-mod behavior, and save identity policy with `tasks/20260925-190156/TASK.md`. Never replace an unknown ID with another good or silently drop stored stock.
- Decide shipped-format migration only if needed; remove obsolete unshipped paths rather than maintaining aliases. Author builders then generate/lint content.

## Verification
- The same effective pack set produces stable IDs and stock independently of load order. Unknown/duplicate IDs and missing required attributes fail at lint then load; a mod item moves through inventory, canister, market and sector lifetime without quantity or mass loss. Incompatible save/catalog changes refuse with a named reason.

## Done when
- A separately approved item-registry contract and proof cover base and mod items through real open-world consumers; no claim of persistence without its own reviewed tier.
