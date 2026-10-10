# Research RON-authored items and open-world identity migration

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.16.0, items, mods, spike

## User facts
- Item identity for mod-added items and related open-world content is separate from shipped M5 (`tasks/20260926-174806/TASK.md`). The owner first requested research, then approved Plan D implementation in the isolated `open-item-catalog` Sprout. Do not stage, commit or push that work without separate authorization.

## Research and future design gate
- First inventory existing `ItemType` consumers and inspect RON-authored registries and mod-merge examples (`ShipDesignId`, `AsteroidKindId`) as possible patterns, not accepted item contracts. Prepare focused examples of base and mod item definitions, conflict/unknown-ID cases, save incompatibility, and a complete player inventory -> canister -> market flow. Compare alternatives and consequences for owner review.
- Later, if separately approved, inventory, canister cargo, jettison, ammo refill, loot, mining, markets, UI, commands and authored stock must agree on one stable typed item ID. Map current consumers and mod content loaders before changing the owning interface; avoid a parallel registry with no actual consumers.
- Define mod-pack ownership, canonical ID namespace/order, validation and duplicate/conflict failure, per-item mass/quantity/category/display, eligible acquisition and consumption, effective-catalog fingerprint, missing-mod behavior, and save identity policy with `tasks/20260925-190156/TASK.md`. Never replace an unknown ID with another good or silently drop stored stock.
- Decide shipped-format migration only if needed; remove obsolete unshipped paths rather than maintaining aliases. Author builders then generate/lint content.

## Prior research (2026-10-10)
- [RESEARCH.md](RESEARCH.md) inventories `ItemType` consumers, mod merge and save identity on master `467ae965b`, compares four plans and lists owner decisions. The owner favors Plan D (open authored item id, unrelated-pack duplicates refused) for further research. A separate preliminary attribute-only step is not approved. Any open-id plan breaks shipped bare-key stock spelling, including save inventories. Independent papers compare [salvage/refit](PROGRESSION-BUILDER.md) and [charters/economy](PROGRESSION-WORLD.md); [the synthesis](PROGRESSION-SYNTHESIS.md) recommends researching charter goals first, subject to observed play and owner choice. It corrects save/section-thaw and open-ID validation assumptions. The approved implementation contract is below.

## Approved design (Plan D, 2026-10-10)
- Interface: `ItemDesignId(Arc<str>)` (transparent serde; stock map keys are quoted, `{"HullPlate": 12}`), `ItemDesign { id, name, about, category, mass_g, ask_cr, bid_cr }` as `Content::Item`, and the merged `GameItems` resource. `ItemType` is deleted. Inventory, canister, transfer, trade and jettison take `&GameItems`; the command shell completes `live::ITEM` from it.
- Roles: `ITEM_ROLES` names the ten ids the game itself uses (hull plate Repair; PDC round, rail slug, torpedo Ammo; four ores Raw; `Rations` Food and `SalvagedParts` Parts, the trade goods generated ship holds draw). Base must define each in RON under its category. A dependent mod may replace a role item whole: its category is fixed, its name, about, mass and prices are not. Mod items are generic stock only.
- Rule (`nova_assets::items::item_pack_faults`, shared by the merge and `content lint`): non-empty id, name and about; `mass_g` > 0; `bid_cr` <= `ask_cr`; an ore role fits one canister; one definition per pack. Two packs may define one id only when exactly one depends on the other, and the dependent replaces the whole item. Unrelated packs and dependency cycles refuse both packs in any load order; dependents of a refused mod are refused; a base fault is fatal.
- Lint applies the rule to every walked repo pack at once, so it is stricter than the merge, which sees only enabled packs: repo packs never enabled together still fail, and `--target` checks against every repo pack. Revisit only with a concrete false-positive repro.
- Lint resolves stock ids against the visible catalog; an unknown id is a `ContentIssues` error at lint then load.
- Generation: `NovaLayeredWorld` pins the catalog it armed with; generated stock reads masses from it.
- Saves: `CONTENT_CATALOG_DIGEST_VERSION` 2 and `WORLD_SAVE_FORMAT` 2. Frozen holders expose `item_ids()`. `check_world` (no lock, no cleanup) and `open_world` run one state check, unknown items included; the Load list runs `check_world` per header-good row on the IO pool, shows "Checking saved state" with Load greyed, and replaces a refused row's header with the named reason. Load checks again. A save whose state holds an unknown item fails before writing and keeps the last good save.
- Decided (owner, 2026-10-11): `Rations` and `SalvagedParts` are required roles, `ITEM_RATIONS` and `ITEM_SALVAGED_PARTS`, so a catalog without them fails at lint then load instead of panicking in a sector hold draw.

## Verification
- The same effective pack set produces stable IDs and stock independently of load order. Unknown/duplicate IDs and missing required attributes fail at lint then load; a mod item moves through inventory, canister, market and sector lifetime without quantity or mass loss. Incompatible save/catalog changes refuse with a named reason.

## Done when
- The approved registry replaces `ItemType` throughout base and mod content and live consumers; content lint and load reject invalid catalog identities and stock, and incompatible saves remain unavailable in Load without losing the last valid generation. Prove mod items through real inventory, canister and market paths with focused checks. Distinguish source-level and fixture proof from a full flown open-world journey; do not claim delivery before the Sprout's final review and validation.
