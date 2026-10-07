# Research RON-authored items and open-world identity migration

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.16.0, items, mods, spike

## User facts
- Item identity for mod-added items and related open-world content is separate from shipped M5 (`tasks/20260926-174806/TASK.md`). The owner wants base and mod items authored as RON content, but wants this **last** in the v0.16.0 sequence and needs more time to choose the model. Do research and small examples first; do not implement a registry yet.

## Research and future design gate
- First inventory existing `ItemType` consumers and inspect RON-authored registries and mod-merge examples (`ShipDesignId`, `AsteroidKindId`) as possible patterns, not accepted item contracts. Prepare focused examples of base and mod item definitions, conflict/unknown-ID cases, save incompatibility, and a complete player inventory -> canister -> market flow. Compare alternatives and consequences for owner review.
- Later, if separately approved, inventory, canister cargo, jettison, ammo refill, loot, mining, markets, UI, commands and authored stock must agree on one stable typed item ID. Map current consumers and mod content loaders before changing the owning interface; avoid a parallel registry with no actual consumers.
- Define mod-pack ownership, canonical ID namespace/order, validation and duplicate/conflict failure, per-item mass/quantity/category/display, eligible acquisition and consumption, effective-catalog fingerprint, missing-mod behavior, and save identity policy with `tasks/20260925-190156/TASK.md`. Never replace an unknown ID with another good or silently drop stored stock.
- Decide shipped-format migration only if needed; remove obsolete unshipped paths rather than maintaining aliases. Author builders then generate/lint content.

## Verification
- The same effective pack set produces stable IDs and stock independently of load order. Unknown/duplicate IDs and missing required attributes fail at lint then load; a mod item moves through inventory, canister, market and sector lifetime without quantity or mass loss. Incompatible save/catalog changes refuse with a named reason.

## Done when
- Research yields code-backed RON examples, migration alternatives and explicit owner choices; implementation remains deferred until a separate approval after the persistence and player-experience priorities are settled. Any later registry delivery must prove base and mod items through real open-world consumers; no claim of persistence without its reviewed tier.
