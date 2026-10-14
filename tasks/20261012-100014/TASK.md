# Upgrades: buy a whole ship from a tiered station catalogue

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.16.0, wizard

## User facts
- Epic `20261012-100008`. Decision 6, third rung: buy an entire ship at a station; the catalogue is tiered by the station's advancement; the old hull is sold or kept in station storage is NOT in scope (sections are never items; a hull is not an item either). Decide the fate of the old hull with the owner.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Generated hull sizes follow advancement (`crates/nova_world_base/src/ship_layout.rs:89-95`); `plan_ship` lays out hulls by role (`crates/nova_world_base/src/sector_ships.rs:1-35`).

- Design 2026-10-14 (reopen: parked offers, rebuy, default bindings). Owner direction relayed: a station sells docking ports minus one hulls at a time and keeps one port open for the player; the old ship takes a port; no hold-point parking. The parking mechanism waits on the owner's pick, A or B below.
  - Docking today is pairwise: `DockedShip` holds one connection per root (`crates/nova_ship/src/sections/docking_section/connection.rs`), `station_admission` refuses `StationOccupied` once one hull docks, `DockedAssembly` measures two roots. A parked offer cannot be a real dock as the code stands.
  - (A) Station hub: a station root holds N connections, `DockedAssembly` measures N+1 roots, admission is per port, and a docked NPC pair at retire freezes with its station (a fault today). Consequence: the docking, assembly, helm and retire rules all change, and each parked hull's mass enters the station's assembly.
  - (B, recommended) Berth: a parked hull rides its station's port pose with no `DockingConnection`, no helm and no assembly. Consequence: docking does not change; a parked hull is a kinematic body the station carries.
  - Entry points. Generation: `station_hulls` (`crates/nova_world_base/src/station_hulls.rs`) lays out ports minus one hulls for sale (was a seeded 3 to 5), from the port count of the pinned station design. Spawn: `spawn_sector_station` (`crates/nova_world/src/streaming.rs`) inserts `StationHullCatalogue`; under B a scenario system parks each Sale entry on a free port, in design order, the last port left open, and the catalogue keeps only the hire offers. Freeze and thaw: `freeze_station` / `thaw_station` (`crates/nova_scenario/src/objects/station.rs`). Buy: the Station tab Buy (`crates/nova_interface/src/station/shipyard.rs`) -> `plan_hull_buy` -> `HullBuyRequest` -> scenario `buy_station_hull`.
  - New components (B): `ParkedAt(Entity)` on a parked hull, a relationship whose target `ParkedHulls` sits on the station (linked spawn: the parked hulls go with the station's despawn). `ParkedHull { port: SectionId, sale: HullSaleType }`. `HullSaleType { Offer, TradeIn { price_cr } }`; `hull_sale_price_cr` prices an Offer at its recipes at ask and a TradeIn at `price_cr`.
  - Save fields (B): `FrozenStation.berths: Vec<FrozenBerth { name, port, sale, ship: FrozenShip }>`; `FrozenStation.hulls` keeps the hire offers only once the sales are parked. Save format bump, no migration.
  - Control. The player root stays the player root: 116 files read `PlayerSpaceshipMarker`, and the HUD, the observer, the scenario id, the credits and the fleet's leader all key on that entity. A buy freezes the parked hull (`freeze_ship`) and the player's hull, despawns the parked one, rebuilds the player root from the parked record at the parked pose (undocked, at the station's velocity, the camera follows the root), and spawns the old hull parked on the port the player was docked at, `TradeIn { price_cr: <trade-in credited> }`. A rebuy is the same swap from a TradeIn berth.
  - Call graph after (B): pane Buy -> `plan_hull_buy(price from hull_sale_price_cr)` -> hold to storage, credits both ways -> `HullBuyRequest { entity, station, hull, trade_in_cr }` -> `buy_station_hull` -> freeze both -> rebuild the player root with `start_role_bindings` -> park the old hull. Every fixed tick: `hold_parked_hulls` sets each parked pose to the station pose times its berth.
  - Bindings: a bought hull takes `start_role_bindings` (`crates/nova_scenario/src/objects/start_role.rs`): turrets on the trigger, railguns R, torpedoes F, mining V. `bought_hull_bindings` and the `NoFreeKey` refusal go.
  - A parked hull vs a live AI ship: `SpaceshipController::None` (no AI marker, no orders), `RigidBody::Kinematic` held on its berth (no gravity, no thrust), `Allegiance::Neutral`, every `ShipCapabilities` flag off (no PDC, no RCS, no lock, no dock), no `LootableShipMarker` (no Take, no strip), every section live (never neutralized, so never a wreck), not a sector body (adoption, the sector freeze and the snapshot skip it; it freezes inside its station's record).
  - Storage: base storage is 100 kg and `plan_hull_buy` refuses a hold that does not fit, so every start role's kit refuses every buy until the player sells or stores. Unchanged; the proofs use the disclosed fixture hold `system_station_buy` already sets.

## Agent decision
Owner told the orchestrator to work the list without a stop at the gate. The agent chose the marked options and implemented them (2026-10-14).

- Old hull:
  - (A, chosen) Trade-in. The station pays the old hull's strip value at bid: for each live section, `strip_value_cr` of its recipe times its health share (current/max), rounded down. A lost section pays nothing. The old sections go. Consequence: no new body, so the save and the ledger do not change shape for it.
  - (B) Leave it as a derelict beside the station. Consequence: a new persistent body that a handler spawns, and handler-spawned persistent ships block the save (`crates/nova_world/src/frozen.rs` ~795-832).
- Cargo:
  - (A, chosen) The whole hold moves into the station's storage, and the buy is refused when storage cannot take it. Consequence: the new hold starts empty and the player withdraws what fits; storage is 100 kg on base stations, so a full hold refuses the buy until the player sells or stores.
  - (B) The hold moves into the new hold, refused when it does not fit (design page line 421). Consequence: a bigger old hold blocks a smaller hull even with free storage.
  - (C) New hold first, overflow to storage. Consequence: two refusal paths and a split the player did not ask for.
- Gate: the existing `Refit` service; no content change. A station with no catalogue sells no hull, so authored scenario stations and dead stations show no list.
- Catalogue: generated in `crates/nova_world_base/src/station_hulls.rs` for each living generated station when its cell is planned. 3 to 5 hulls (seeded), deterministic from the world seed and the station id. Roles cycle from a seeded start through the roles the civilization's advancement can build (`eligible_roles`). Each hull is `generate_ship` at the civilization's advancement, so the size ceiling is the generator's. Name: `<Civilization> <role> Mk <n>`. A layout failure fails the sector, as a hull's does. The list rides `SectorStation.hulls`, is checked by `validate_manifest` (each design as a ship design, none on a dead station), is part of the canonical description (the pinned window digest `0x2632_b928_86d6_9e8f` is unchanged: that window holds no living station), and is frozen with the station (`FrozenStation.hulls`), so a station that drifts across cells keeps it and a save keeps it.
- Price: the sum of the new hull's section recipes at ask (`value_cr`). Net credits: player `balance - price + trade-in`; station `balance + price - trade-in`. Refused when the player cannot pay, the station cannot pay a net payout, or a balance overflows.
- Buy: the player root stays (HUD, observer and scenario id keep their owner). The old sections despawn, the inline design becomes the offer, the new sections spawn through the same per-section spawn, and the root is posed so the new hull's first docking port face lies on the old port face with the same axis, at the station's velocity. The old port's despawn releases the dock (`OnUndocked`), and the ship redocks once the new port is live (`OnDocked`).
- Bindings: thrusters stay unbound (the flight computer drives them). A weapon takes the binding of the first old live section of its class, else an empty binding that Rebind sets. A mining emitter takes the next free key of V, B, N, C among the new emitters; the buy refuses when all four are held.
- Event: `OnShipBought` (`onshipbought`): `id`, `type_name`, `hull`, `price_cr`, `trade_in_cr`. Credit cause `ShipBuy` on both parties. Each moved stack also fires `OnStored`.
- UI: the Ship pane's side panel, a "Ships for sale" list under the Add list (the pane that already holds the refit lists; there is no Station pane). Each row: name, length in m and section count, a class summary, price, trade-in, and a Buy button or the refusal.

- Agent result 2026-10-14: implemented as decided. Proof: `examples/systems/system_station_buy.rs` (seed 10, the first hull of the nearest living Refit station, bought by the Buy button; 3 roster invariants) passed under lavapipe; the tier test is `a_first_ring_station_sells_small_hulls_and_the_same_catalogue_for_a_seed` (`crates/nova_world_base/src/station_hulls.rs`); the buy rule has 5 unit tests in `crates/nova_ship/src/sections/hull_buy.rs`. Found on the run: the starting warship trades in for 12,615 cr against a 9,540 cr first hull, so the first buy pays the player; the economy page should judge if the trade-in share or the starter's recipes need a cut. A 100 kg base storage refuses the buy for a hold over that; the player must sell or store first. The kinds line names classes by pane code prefix (`41 HULL, 2 CTL, 6 THR`). No ship preview in the list.

- Orchestrator close 2026-10-14: landed in eb3a0aa01. A generated living station with `Refit` sells 3 to 5 hulls (`station_hulls.rs`, seeded by world seed and station id, size by advancement, kept in `FrozenStation.hulls` and the save). Price is the new recipes at ask. The old hull trades in at strip value times health share. The hold moves to station storage, and the buy is refused if storage cannot take it. The ship root stays and the sections are replaced and redocked at the same port. New `OnShipBought` event and credit cause `ShipBuy`. Proof: `examples/systems/system_station_buy.rs` passed twice (buy, save, load gives back the 54-section hull). The orchestrator reviewed the Ships for sale crop and the docked frame (STATION HELM, DOCK active; a rock sits close under the new hull) and re-ran `content lint`: 0 errors. For the owner: the starting warship trades in for 12,615 cr against a 9,540 cr first hull, so the first buy pays the player; base station storage of 100 kg refuses a full hold.

## Owner feedback (2026-10-14, reopens the task)
- The player can rebuy their old ship at the station after a trade-in.
- A bought ship comes with default bindings on turrets and tools.
- Every ship for sale is visible, parked (docked) at the station. Buying one undocks it and hands the player control. The traded-in old ship is parked there too, where the player can see it.

- Owner decision 2026-10-14: a station sells as many ships as it has docking ports minus one, keeping one port free for the player. The traded-in old ship takes a port too. Mechanism: (A) real multi-dock. A station becomes a hub holding N docking connections, one per port. Each ship still holds one connection. The docked assembly covers the station plus every docked hull, and the station's helm and self-levelling act on that assembly. Parked offers and the old ship are real docks. A buy hands control to the docked offer, which can then undock normally. Rejected: (B) kinematic berths without a connection.

- Owner decision 2026-10-14 (answers the 100 kg storage question): storage at a station is the player's own. The station's storage view shows the holds of the player's own ships docked at that station, such as a traded-in old ship parked at a port, as storage. The player moves items between the current ship and those holds. A trade-in no longer moves the old ship's cargo into station storage; the cargo stays in the old ship's hold, so the hold-does-not-fit refusal goes away. The station storage capacity is not raised.

- Owner feedback 2026-10-14: Ships for sale is boring as text rows. Show each hull as a 3D preview (turntable or framed render) with its name, price and stats.

## Delivery gate
- The catalogue is generated from the station's civilization and advancement, deterministic from the seed; buying swaps the player's design and respawns the hull docked. Stop for the owner on the old hull (trade-in credit vs. left as a derelict).

## Verification
- Asserted example: buy, undock, save, load, the new hull is the player's; the catalogue of a ring-1 station holds only small hulls.

## Done when
- Buy works in a live run and the catalogue tiers are a permanent test.
