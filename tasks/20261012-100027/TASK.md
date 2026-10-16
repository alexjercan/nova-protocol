# Economy: computed prices from reference, scarcity and demand

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.16.0, economy

## User facts
- Epic `20261012-100025`. Actual price = reference x scarcity x demand, computed at dock time from content rules (cluster type, civilization role preference, stock level); no reputation input.

## Agent findings
- Items carry `ask_cr` and `bid_cr`, labelled provisional (`crates/nova_assets/src/items.rs:14-15`, `:115-158`); stock is planned by advancement (`crates/nova_world_base/src/sector_ships.rs:92-108`); no restock, no trade AI.

## Delivery gate
- A pure `price()` function with named factors and clamps as content constants; the Inventory pane shows the factors, not only the number. Lint: a rule that can push bid above ask is an error.

## Verification
- Unit tests over the factor table; an asserted example where ore is cheap in an AsteroidRich cluster and dear in a PlanetHeavy one.

## Done when
- Prices are computed, readable in the UI, and the factor table is permanent content.

## Agent decisions 2026-10-16
- `ShipRoleType` moves from `nova_world` to `nova_gameplay::market`, the lowest crate that prices by role.
- `PriceTable` is a singleton content kind with the fixed id `price_table`. The last bundle that states one wins, whole. The base bundle must state one; a bundle may state at most one.
- Demand = cluster factor x civilization factor. The civilization factor is the role-preference-weighted mean of the role rules for the item's category; a missing rule counts as 1.
- Scarcity curve: `starved` at stock 0, 1 at the target, `glut` at `glut_share` x target and above, linear between; `unkept` for an item the trader spawned without. Target = spawn stock.
- `TraderMarket` sits on living generated stations and intact generated ships. `FrozenShip` carries it. Save format 13 -> 14.
- Authored traders, derelicts and job-spawned ships have no market and deal at reference (`PriceQuote::reference`).
- A trade prices the whole lot at the pre-trade stock.
- A Buy at an owned station pays the computed bid (`TradePriceType::Owned`).
- Refit and section part purchases, job rewards and kit values stay at reference ask.
- Lint soundness: every factor must have bid <= ask and lie in the clamp, and the clamp must hold 1, so the product bid <= product ask on every path.
- Base values (named constants in `crates/nova_authoring/src/base_content/market.rs`): clamp 0.25..3.0; starved 1.5/1.25; glut 0.9/0.6 at 3x target; unkept 1.0/0.9; AsteroidRich Raw 0.6, RockOnly Raw 0.8, PlanetHeavy Raw 1.5, DerelictField Parts 0.7 and Repair 0.85; Industrial Raw 0.8, Civilian Food 1.25, Scavenger Parts 0.8, Armored Ammo 1.25 and Repair 1.15.
- The interface reads `GamePriceTable` as `Option<Res>` and requires it (expect) only when the partner has a market.
- The inspector Civilization row shows the leading role label with the combined civilization factor.
- `market_sandbox` uses two worlds (seed 1 AsteroidRich, seed 21 PlanetHeavy) through the main menu: no searched seed has both station types in one window.
- Added beyond the gate: `PriceQuote::reference`, merge fn `price_table_errors`, the nova_world manifest case "an intact ship that states no market", and a market-aware price sample in `station_own_sandbox`.

## Proof 2026-10-16
- Unit: nova_gameplay `market` (11) and `item_trade_plans`; nova_authoring `base_price_table_has_no_faults` and `base_iron_ore_sells_cheaper_in_an_asteroid_rich_cluster_than_a_planet_heavy_one` (AR 8/5, PH 19/13); nova_assets merge (14); nova_world lib (42); nova_world_base save/clusters/sector_ships (50); nova_interface inventory (31); nova_scenario freeze/thaw (5); nova_authoring content_ron_parity, generated_stations, generated_wrecks.
- `content gen` writes `assets/base/market/base.content.ron`; `content lint` 0 errors.
- Live: `market_sandbox` on Xvfb + lavapipe, autopilot. Asteroid-rich ask 9 / bid 6 (cluster x0.60, civilization x0.97), sold 20 for 120 cr. Planet-heavy 22 / 15 (cluster x1.50, civilization x0.93), sold 20 for 300 cr. Asserted "iron ore bid 6 cr asteroid-rich < 15 cr planet-heavy", cycle complete, no panic. Screenshots of both inspectors show Ask/bid, Reference, Scarcity, Cluster, Civilization and Total rows.

## Open gaps
- A bulk trade does not walk the scarcity curve per unit.
- The Civilization row names the leading role but shows the weighted factor.
- Refit, job reward and kit prices are not computed.
- No restock (`20261012-100029`) or clock (`20261012-100028`).
- PlanetHeavy stations are rare in the generator.
