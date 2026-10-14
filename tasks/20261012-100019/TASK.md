# Salvage: strip a docked lootable hull into recipe parts

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.16.0,salvage

## User facts
- Epic `20261012-100018`. Strip is a timed action on a docked lootable partner: the section detaches through the existing detach path and breaks down into its recipe parts in the player's hold, more of them the healthier it was; refuses on a non-lootable hull; a stripped wreck stays stripped.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Generated derelicts spawn with every system disabled, `lootable: true`, Neutral, no controller; detached pieces exist (`FrozenTransient::DetachedPiece`, `crates/nova_world_base/src/save/transients.rs:61-81`). `SalvagedParts` is a required item role (`tasks/20260930-100831/TASK.md`).

## Delivery gate
- Strip uses the section recipe times `strip_yield` times health fraction; the lint rule (strip yield at bid never exceeds the recipe at ask) lives with the recipe. The detached piece despawns into parts, not a floating item.

## Verification
- Asserted example: strip once, retire and return, the wreck stays stripped, a second strip on the same section refuses; a strip on a non-lootable hull refuses.

## Done when
- Strip works in a live docked run and the lint rule is permanent.

## Decisions (worker, 2026-10-14; owner said work without stopping, so each records its options)
- Strip yield: `BuildRecipe::strip_parts_at(share)` gives `floor(count * strip_yield * share)` per part, one floor; `strip_parts()` is `strip_parts_at(1.0)`, so the wreck drop is unchanged (`crates/nova_gameplay/src/inventory.rs`). Options: (a) one floor per part over yield times health (chosen: matches "rounded down" once, never pays more than a full strip); (b) floor the full strip, then floor again by health (double floor, loses more parts).
- Trade-in stays apart from strip (`hull_trade_in_cr`, `crates/nova_ship/src/sections/hull_buy.rs`). Options: (a) keep the trade-in a credit price scaled smoothly by health (chosen: a price need not be whole parts; a damaged light hull still pays a credit); (b) trade-in = whole stripped parts at bid (shared code, but drops most damaged sections to 0 cr and changes a landed rule).
- Lint: the rule "strip at bid stays below the recipe at ask" already landed with the recipe in 26a9d3660 (`lint_section_recipe`, `crates/nova_scenario/src/lint/ship.rs`), with a clean test and the negative `a_recipe_that_strips_for_its_price_or_more_is_an_error`. A health share is at most one, so a strip at any health is at most the full strip the lint bounds; a unit test holds `strip_parts_at` at or under `strip_parts`.
- Who may be stripped: the partner the player is docked with that a Take may loot, neutralized or `LootableShipMarker`, the one rule the Inventory pane uses. Options: (a) lootable or neutralized (chosen: decision 4 says "neutralized or derelict", and what you may Take you may strip); (b) `LootableShipMarker` only (a beaten raider could be looted but not stripped).
- Structure: a strip refuses the partner's port that holds the dock (`Berth`) and a section whose loss splits the partner's section graph (`SplitsHull`). Options: (a) refuse both (chosen: the dock and the hull stay one body, no sever, no fragment ids); (b) let the sever path split the wreck (the docked half and a new fragment; sever ids and mass motion reopen). The berth is always left, so a hull is never stripped to nothing.
- Removal: the section is not destroyed. `SectionStripRequest` (`crates/nova_ship/src/sections/section_strip.rs`) prunes it from its neighbours' `ConnectedTo`, lowers the root's pinned health max by its pool (as a swap does), and despawns it with its art. Options: (a) direct removal (chosen); (b) the destroy path (`IntegrityDestroyMarker`) with skips: it fires the destruction burst, `OnDestroyed`, a canister drop of the strip share and a 30 s floating wreck piece, all four wrong for a teardown. The detached piece "despawns into parts": no body is spawned.
- Persistence: a stripped section is a design section with no live entity, as a destroyed one is; `FrozenShip` keeps only live sections, so a retire, a thaw, a save and a Load keep it stripped with no save change (format 8 unchanged). A second strip of it refuses `SectionGone`.
- Hold full: refuse up front with the mass needed and free (`HoldFull`). Options: (a) refuse (chosen, as the task prefers; no part is lost); (b) drop the overflow as a canister (a hold-full strip becomes a free jettison). The plan runs again when the timer ends; a hold filled meanwhile cancels the strip with its reason and nothing moves.
- Timed action: `max(3 s, 1 s per recipe part)` of game time (`strip_duration_secs`): a light hull 3 s, a PDC 5 s, a capital drive 40 s. Options: (a) by recipe part count (chosen: the recipe is what is taken apart, and modded sections get a time with no new field); (b) by footprint cells (a hull and a turret of one cell take the same). The timer runs on the virtual clock, so it holds while the TAB interface pauses the world. One strip at a time (`Busy`). It cancels when the player undocks or the partner or section is gone; the job is not saved, so a save mid-strip loses only the timer.
- Event: `OnStripped` (`id` = the ship that stripped, `hull` = the stripped hull's scenario id, `section` = its pane code, `parts`), beside `OnRebuilt`, `OnSwapped` and `OnAdded`.
- UI: no surface shows a docked partner's sections today (the Ship pane shows only the player ship; the Inventory pane shows the partner's hold). Options: (a) a "Strip the docked wreck" list in the Ship pane side panel, one row per live partner section with its code, yield and a Strip button, and a progress line (chosen: the station services live there and the panel scrolls); (b) a partner schematic in the Ship pane (a second 3D scene); (c) the Inventory pane partner column (no section concept there). The list shows only while docked with a lootable partner; a non-lootable partner is refused by the rule if a command reaches it. In flight the mode chip reads `STRIP <code> <percent>` while a strip runs.
- List rows: a refused row shows its reason in the danger tone with no Strip button; while a strip runs the rows hide and the progress line stands alone; rows sort by code (`crates/nova_interface/src/ship/strip.rs`). Options: (a) this (chosen: a 30-section wreck listed every Busy refusal and a dead button per row); (b) a button on every row with the refusal on click (long list, dead buttons).
- Proof seed: 115, the seed `system_open_world_identity` pins for its derelict; seed 95 holds no derelict in its window. The not-lootable check docks at the nearest living Player or Neutral station.
