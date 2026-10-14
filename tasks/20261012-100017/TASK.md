# Own station: buy a station from a living civilization

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.16.0, wizard

## User facts
- Epic `20261012-100008`. Decision 13 (2026-10-12): the buy half of D10 lands right after Stage 2: buy a station from a living civilization; its stock targets and price rules become yours; it refuses nobody.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Decisions (2026-10-14)
Owner told the orchestrator to work the list without a stop at the gate. The agent took the recommended option at each stop and records the options here.

- Ownership: one source of truth. `OwnedStationMarker` (`crates/nova_gameplay/src/markers.rs`) on the live station root, and `FrozenStation.owned` (`crates/nova_scenario/src/objects/station.rs`) in the frozen record a retire or a save writes and a thaw reads back. Options: a `SavedPlayer` id list only (a thaw would have to look the player save up, and a retire within a session has no player save); the field and a `SavedPlayer` list (two copies that can disagree, and nothing reads the list yet); the field only (chosen). The Map and UI read the marker or the ledger.
- Admission: `station_admission(allegiance, derelict, occupied, owned, hostile)` (`crates/nova_ship/src/sections/docking_section/admission.rs`). Occupied refuses first; owned or derelict admits on any side; then `EnemyStation`; then the new `HostileStation` for a living station the ship is hostile to by `ship_relation` (a Player station and an Enemy-flagged ship, or a station answering its fire). Without `HostileStation` an Enemy-flagged ship was admitted at every non-Enemy station, so "refuses nobody" would prove nothing. The owned station keeps its civilization side and AI. Options: give the station the Player side on buy (its guns would then read the civilization's foes, and D13's stance graph owns that); keep its side (chosen). Known conflict: an owned Player station's guns read an Enemy-flagged owner as hostile; the player is never Enemy-flagged in play today.
- Price: `station_price_cr` = (every live section's recipe at ask + hold at ask + credits) x `STATION_OWNERSHIP_FACTOR` 2/1 (`crates/nova_ship/src/sections/station_buy.rs`). 2 puts a station above an escort hire (3/2) and makes buying one to strip it lose money. The player's storage is excluded: it is already the player's. The price goes to the civilization; the station's balance does not move. Options: 3/2 (equal to a hire); 2 (chosen); stock-only (a station with an empty hold would be near free).
- Income: none. `TODO(20261012-100028)` in `thaw_station`, removed when the economy strand adds the play-time clock and the income rule.
- "Price rules become yours", this step: a Buy from an owned station's hold pays `bid_cr` (`TradePriceType::Owned`, `crates/nova_gameplay/src/inventory.rs`); storage withdraw was already free. Not in this step: Take credits from an owned station (its balance stays its trading float, where income lands later); services (rebuild, swap, add, hull, hire) still price at ask; editing stock targets waits for 100028.
- Which stations: every living, generated (`GeneratedBodyId`), non-derelict station, while docked. No new `StationServiceType`. Options: a new authored service (content and lint churn for a rule every generated station meets); reuse `Refit` (unrelated); the rule (chosen). UI: a "This station" block in the Ship pane (`crates/nova_interface/src/ship/station_buy.rs`), following the shipyard list, with a "Buy station" button (`STATION_BUY_BUTTON`).
- Event: `OnStationBought` (`id`, `type_name`, `station`, `price_cr`) and one `OnCreditsChanged` for the player with cause `StationBuy`.

- Agent result 2026-10-14: implemented in 097c0311f (rule, marker, admission, pane, event), 2d9ae5dc8 (proof) and bb4ccc06b (docs). Proof: `examples/systems/system_station_own.rs` on seed 95 (station `sector_0_n1_0_cluster_0_n1_0_station_0`, Player side, 177 sections, 144,366 cr) passed under lavapipe: refused Enemy-flagged before the buy (`HostileStation`), bought through the pane (player -144,366 cr, station balance unchanged, one `StationBuy`, one `OnStationBought`), frozen owned after a jump three cells out, thawed owned, saved owned (generation 5, format 8), and after Load docked Enemy-flagged. 5 roster invariants, `SYSTEMS_INVARIANTS` 540. Seed 10 has no Player-side station in the window. Unit tests: price and plan (`station_buy.rs`), admission (`docking_section/tests.rs`), owned bid price (`inventory.rs`).

- Orchestrator close 2026-10-14: landed in 097c0311f, 2d9ae5dc8, bb4ccc06b and 52e7a0c43. Ownership is `OwnedStationMarker` on the live station and `FrozenStation.owned` on the frozen record; there is no `SavedPlayer` copy. Price is (recipes + hold + credits at ask) x 2, paid to the civilization. An owned station admits any side and sells at bid (`TradePriceType::Owned`). No income: `TODO(20261012-100028)` in `thaw_station`. New `OnStationBought` event and credit cause `StationBuy`. Proof: `examples/systems/system_station_own.rs`, seed 95 (refusal before the buy, then owned through retire, thaw, save and Load, and admits an Enemy-flagged ship). The orchestrator reviewed the owned-block frame and re-ran `content lint`: 0 errors. For the owner: a new admission reason `HostileStation` now refuses a ship at any station it is hostile to; before, only Enemy-side stations refused. Balance: 144,366 cr for a 177-section station.

## Owner feedback (2026-10-14, reopens the task)
- A bought station becomes the player's. It moves to the player's side and drops its old civilization, so it no longer counts as that civilization's station for prices, admission, relations or anything else. It saves that way.

## Agent findings (2026-10-14, reopen)
Survey: every consumer of a station's civilization or side. A live station carries no civilization id. Its civilization shows only through its side (`Allegiance`), its `Name` ("<civilization> station") and the generator's record.

Generator (pure function of the seed; a frozen record replaces the generated body once a cell retires or saves):
- `SectorStation.civilization` `crates/nova_world/src/generation.rs:305`, printed in `canonical` `:538`.
- Side and dead-or-living from the civilization: `crates/nova_world_base/src/clusters.rs:1871`; `station_count` per civilization `crates/nova_world_base/src/sector_stations.rs:113`; the hull catalogue from the civilization `clusters.rs:1879`. No runtime list of a civilization's stations exists, so nothing at runtime counts a bought station for its old civilization.
- Spawn: `spawn_sector_station` names the station from the civilization `crates/nova_world/src/streaming.rs:373-384` (`generated_station_name` `:403`); `dead_station_name` `:421` parses that name back and panics on another; `rename_dead_generated_station` `:435`. A held `PendingSectorStation` `:96` respawns from the manifest (never a docked, bought station).

Side (`Allegiance`), set from the civilization at spawn `crates/nova_scenario/src/objects/station.rs:233-237`:
- Relations: `relation` / `ship_relation` `crates/nova_gameplay/src/relations.rs:64`, `:100`.
- Station AI targeting: `crates/nova_ship/src/input/ai/acquisition.rs:313` (`ship_relation`); retaliation only on a Neutral hull `crates/nova_ship/src/input/ai/threat.rs:330`.
- Admission: `station_admission` `crates/nova_ship/src/sections/docking_section/admission.rs:54` (owned admits before the side rules), `DockAdmission::refusal` `:123`.
- HUD markers `crates/nova_hud/src/allegiance_markers.rs:98`; Map local contacts `crates/nova_interface/src/map/contacts.rs:113` (Ally, Hostile, Neutral).
- Neutralized station turns Neutral `crates/nova_scenario/src/objects/station.rs:268`, `:295`.
- Frozen with the hull: `FrozenShip.allegiance` `crates/nova_scenario/src/objects/spaceship.rs:596`; name in `FrozenBody.name` `crates/nova_world/src/frozen.rs:369`.

Ownership (`OwnedStationMarker`, `crates/nova_gameplay/src/markers.rs:145`):
- Frozen as `FrozenStation.owned` `crates/nova_scenario/src/objects/station.rs:358`, `:422`, `:449`.
- Prices: `TradePriceType::Owned` `crates/nova_gameplay/src/inventory.rs:944`, read at `crates/nova_interface/src/inventory/app.rs:1406`.
- Sale: `crates/nova_interface/src/station/station_buy.rs:86` (sale type), `:304` (the buy inserts the marker, nothing else).
- Admission: `admission.rs` (above).

Chart and Map:
- `ChartStation` keeps `derelict` and the last-seen `StationSummary.name` only `crates/nova_gameplay/src/chart.rs:83`; written by `chart_sensor_finds` and `chart_docked_station` `crates/nova_world_base/src/chart.rs:67`, `:109`.
- Map chart colour and kind by `derelict` only `crates/nova_interface/src/map/chart.rs:111`, `:548-554`, legend `:632`.

Not civilization-bound, kept: the station's hold, balance, storage, services and hull catalogue are the station's own; escort hire and hull buy price at ask (100028 owns the owned-station price rules for services).

Gaps today: the buy only inserts the marker, so a bought station keeps its civilization's side (its guns read the civilization's foes), its "<civilization> station" name, and shows on the chart and Map as any living station.

## Agent result (2026-10-14, reopen)
Representation: `OwnedStationMarker` stays the one ownership record (`FrozenStation.owned`); generated Player-side civilizations exist, so the side alone cannot mean owned. The buy inserts `owned_station()` (`crates/nova_ship/src/sections/station_buy.rs`): the marker, `Allegiance::Player` and the name `OWNED_STATION_NAME` ("Owned station"). Side and name freeze with the hull (`FrozenShip.allegiance`, `FrozenBody.name`); the chart gains `ChartStation.owned`, and the Map's chart view draws it as OWNED STATION in the ally colour. A dead owned station reads "Owned dead station". Save format stays 9 (unreleased); an old dev chart fails loudly on `deny_unknown_fields`.

Proof:
- `examples/systems/system_station_own.rs`, seed 10: the station starts its civilization's (Neutral side, "<civilization> station", its guns read an Enemy ship as Neutral); after the buy it is Player-side, named as the player's, reads an Enemy ship as Hostile and the player as Own; the chart holds it owned; it stays so across a retire, a thaw, the leave save and a Load, and admits an Enemy-flagged player; the Map's chart view reads OWNED STATION. Reproduced first: before the fix the station stayed Neutral and "Fenhalka station".
- Unit: `dead_station_name(OWNED_STATION_NAME)` in `crates/nova_world/src/streaming.rs`; prices rest on `TradePriceType::Owned`'s existing test.
- Playable: `examples/playable/station_own_sandbox.rs` docks at the same station with the price plus 50,000 cr; its overlay rings the station by side, draws a line to each ship within 10 km by relation, and reads side, civilization, marker, chart entry, admission, price and trade prices. Its smoke buys from the Station tab and shoots the overlay.

Not verified live: the station's guns firing on an Enemy ship, and a Buy at bid from the Inventory pane (the overlay reads the price kind only).

## Delivery gate
- Ownership is a field on the frozen station body and in `SavedPlayer`; an owned station's admission skips the civilization rule. Stop for the owner on the price (a multiple of its stock value) and whether income accrues (needs the clock, economy strand).

## Verification
- Asserted example: buy, leave, return after a reload, the station is owned and admits an Enemy-flagged player ship.

## Done when
- Buy works in a live run and ownership survives retire, thaw and reload.
