# Economy: computed prices from reference, scarcity and demand

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.16.0,economy

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
