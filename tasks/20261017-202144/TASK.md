# Economy: a looted neutralized hull or station restocks on every thaw

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.16.0,economy,bug

## Evidence
- `restock_thawed_traders` (`crates/nova_world_base/src/economy.rs:36-47`) queries `(&ThawedAfter, &TraderMarket, &mut ShipInventory)` with `Added<ShipInventory>`. It has no `Without<NeutralizedMarker>` or `Without<DerelictShipMarker>` filter.
- `on_station_neutralized_turn_derelict` (`crates/nova_scenario/src/objects/station.rs:333-359`) keeps `TraderMarket`. Nothing removes it on a ship neutralize.
- `crates/nova_scenario/src/objects/spaceship.rs:910` freezes `TraderMarket` for any ship and `spaceship.rs:1072` reinserts it on thaw.
- Scenario: neutralize a hull, loot it through the Inventory pane, leave the cube, return after 30 min of play. The hold refills to target. Unlimited loot.
- Lint and runtime disagree: `crates/nova_world/src/generation.rs:924-927` requires no market on a body that does not trade.

## Fix direction
- Option A: filter the restock query with `Without<NeutralizedMarker>` and `Without<DerelictShipMarker>`.
- Option B: remove `TraderMarket` on neutralize by a deferred command. `record_neutralized_civilization` (`economy.rs:84`) reads it on the same Add event, so the removal must run after it.
- The stance-graph lane edits `economy.rs` now. Land this after it.

## Done when
- A looted neutralized hull or station stays empty across a freeze and thaw.
- Permanent proof: a unit test or asserted example that neutralizes a trader, empties its hold, freezes and thaws it after a full recovery, and asserts the hold stays empty.

Found by the 2026-10-17 review of epic 20261012-100025.
