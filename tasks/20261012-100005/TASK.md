# Stations: buy stock, store and withdraw parts, sell goods while docked

- STATUS: OPEN
- PRIORITY: 83
- TAGS: v0.16.0, stations

## User facts
- Epic `20261012-095958`. Decisions 2b and 4 (2026-10-12): transactions only while docked; station storage holds parts and goods, capped, never sections; sell what you brought back at the station's bid.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Docked trade exists between ships (`crates/nova_gameplay/src/inventory.rs`, the docked-partner logic knows only trade and loot); `ShipInventory { capacity_g, stacks }` on the ship root. Generated stock and credits are planned by advancement (`crates/nova_world_base/src/sector_ships.rs:92-108`).
- Landed: `StationStorage` moved to `nova_gameplay::inventory`, one gate `station_offers` for Rebuild, Trade and Storage, `plan_item_storage` (Store/Withdraw, no credits), Inventory pane Storage column, `OnStored`/`OnWithdrawn`. Generated living-station stock and credits scale by advancement (`STATION_STOCK_SHARE` 0.5..1.0); a dead station keeps its authored stock as loot. World save format 5 -> 6. Proof: `examples/systems/system_station_storage.rs` (retire, thaw, save, load). The `TODO(20261012-100005)` at `station_design_id` in `sector_stations.rs` is untouched.

- Orchestrator close 2026-10-13: landed in 42d502e91; follow-up 6129789a3 rewrote the `station_design_id` TODO without a task id (every civilization still builds the base waystation until a second station design exists) and trimmed two changelog entries to 200 chars. `content lint` re-run: 0 errors, 0 warnings. Agent-chosen values for owner review: `STATION_STOCK_SHARE` 0.5..1.0, dead stations keep full stock as loot, live proof on seed 10.

- Quick item actions (2026-10-14), current flow. Every item move lives in the TAB Inventory pane (`crates/nova_interface/src/inventory/`); the Station pane (`station/`) sells only hulls, escorts and the station. A row click (`on_inventory_row`, `app.rs:960`) runs `select_inventory_row`, which opens a one-unit draft of `offered_action` (`app.rs:1159`): own row Give (lootable) / Sell (trades) / Store (stores) / Give; partner row Take (lootable) / Buy (trades); storage row Withdraw; own row Jettison while undocked. Give/Sell/Store chips switch the own-row draft (`on_inventory_draft_action_chip`). Wheel, slider, field and All set the quantity; Confirm writes `InventoryActionCommand`, and `apply_inventory_action_commands` runs `plan_item_transfer/trade/storage` and flashes the note (refusals buzz, keep the draft, change nothing). No double click in `nova_interface` or `nova_ui`; `nova_editor` has its own `LastClick` + `DOUBLE_CLICK_SECS = 0.5` on `Activate` and `Time<Real>` (`config.rs:77`).
- Chosen mapping: a double click on a row is "select it, then Confirm the form it opens", with the quantity set to the whole stack (Shift held: one unit). One rule for every row: hold row at a station = the own-row action the form shows (Sell, or Store, or Give at a lootable partner); storage row = Withdraw; station/ship stock row = Buy (Take at a lootable partner). Jettison is excluded (undocked dumping is not a transaction). The own-row chip choice now sticks across rows while the partner takes it, so picking Store once makes every following double click store; a new row no longer resets to Sell. The command goes through the same `InventoryActionCommand` and `apply_inventory_action_commands`, so refusals (credits, capacity, stock, cap) are the button's own, on the note line, with no change. A whole-stack Buy of a large station stock refuses as the button would; Shift+double click buys one.
- Shared helper: `nova_ui::widget::DoubleClick<K>` with `DOUBLE_CLICK_SECS = 0.5` on `Time<Real>`, keyed by the clicked thing; the editor's `LastClick` moves onto it (one detector, one threshold). Three fast clicks are one double and one single.
- Values for owner review: threshold 0.5 s (editor value); whole stack on double click, one unit on Shift (raw Left/Right Shift, not a rebindable action); sticky own-row chip for the session; Jettison not quick.

## Owner feedback (2026-10-14, reopens the task)
- Make item transactions faster: a double click (or a similar single gesture) on an item quick-sells, quick-buys, stores or withdraws it, so storing items is fast.

- Owner decision 2026-10-14 (answers the 100 kg storage question): storage at a station is the player's own. The station's storage view shows the holds of the player's own ships docked at that station, such as a traded-in old ship parked at a port, as storage. The player moves items between the current ship and those holds. A trade-in no longer moves the old ship's cargo into station storage; the cargo stays in the old ship's hold, so the hold-does-not-fit refusal goes away. The station storage capacity is not raised.

## Delivery gate
- A station inventory on the station body with a cap from its design; the Inventory pane gains Store and Withdraw while docked to a station; Buy and Sell reuse the docked trade path. Reference prices only (the economy strand computes actual prices later).

## Verification
- Asserted example: store parts, undock, leave the sector, return, withdraw the same stacks; the cap refuses an overflow with a visible line.

## Done when
- Store, withdraw, buy and sell work in a live docked run and survive a save round trip.
