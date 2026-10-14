# Own station: buy a station from a living civilization

- STATUS: CLOSED
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

## Delivery gate
- Ownership is a field on the frozen station body and in `SavedPlayer`; an owned station's admission skips the civilization rule. Stop for the owner on the price (a multiple of its stock value) and whether income accrues (needs the clock, economy strand).

## Verification
- Asserted example: buy, leave, return after a reload, the station is owned and admits an Enemy-flagged player ship.

## Done when
- Buy works in a live run and ownership survives retire, thaw and reload.
