# Stations: buy stock, store and withdraw parts, sell goods while docked

- STATUS: OPEN
- PRIORITY: 83
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decisions 2b and 4 (2026-10-12): transactions only while docked; station storage holds parts and goods, capped, never sections; sell what you brought back at the station's bid.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Docked trade exists between ships (`crates/nova_gameplay/src/inventory.rs`, the docked-partner logic knows only trade and loot); `ShipInventory { capacity_g, stacks }` on the ship root. Generated stock and credits are planned by advancement (`crates/nova_world_base/src/sector_ships.rs:92-108`).
- Landed: `StationStorage` moved to `nova_gameplay::inventory`, one gate `station_offers` for Rebuild, Trade and Storage, `plan_item_storage` (Store/Withdraw, no credits), Inventory pane Storage column, `OnStored`/`OnWithdrawn`. Generated living-station stock and credits scale by advancement (`STATION_STOCK_SHARE` 0.5..1.0); a dead station keeps its authored stock as loot. World save format 5 -> 6. Proof: `examples/systems/system_station_storage.rs` (retire, thaw, save, load). The `TODO(20261012-100005)` at `station_design_id` in `sector_stations.rs` is untouched.

## Delivery gate
- A station inventory on the station body with a cap from its design; the Inventory pane gains Store and Withdraw while docked to a station; Buy and Sell reuse the docked trade path. Reference prices only (the economy strand computes actual prices later).

## Verification
- Asserted example: store parts, undock, leave the sector, return, withdraw the same stacks; the cap refuses an overflow with a visible line.

## Done when
- Store, withdraw, buy and sell work in a live docked run and survive a save round trip.
