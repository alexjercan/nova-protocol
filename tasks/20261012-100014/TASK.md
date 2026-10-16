# Upgrades: buy a whole ship from a tiered station catalogue

- STATUS: CLOSED
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

## Owner feedback (2026-10-15)
- Show escorts for hire in the Station tab like the ships for sale. Spawn the hire offers in the world near the station, not docked, coasting; on hire they become the player's and follow.
- Cards: an image of the ship plus a short line (name, type, Mk, size, price) and two buttons, Buy (or Hire) and Info. Drop the long section-count text from the card.
- Info opens the ship in a read-only Ship-style view: rotate it, select and inspect sections, no editing.
- "Trade-in" is unclear to the owner: say what the player gets for the current ship in plain words.

- Agent decision 2026-10-15, item 1 (escorts for hire in the world). The owner said to work the list; the agent chose the marked options.
  - Owner of an offer:
    - (A, chosen) Reuse `ParkedHull` / `ParkedHulls` with a new `ParkedHullType::Hire { slot }`. Consequence: an offer freezes and thaws inside `FrozenStation.parked`, despawns with the station and stays out of the cells (`PersistentBody` is `Without<ParkedHull>`) with no new code; `StationHullCatalogue`, `HullOffers` and `FrozenStation.hulls` die, so the save format goes from 10 to 11.
    - (B) A separate `HireOffer` relationship. Consequence: a second copy of the freeze, thaw, despawn and save paths.
    - (C) Rename the relationship to a neutral name first. Consequence: a wide rename across lanes that edit the same files now.
  - Where an offer stays:
    - (A, chosen) Same orbit, phase shift. In the gravity well that holds the station, `hire_offer_place` rotates the station's pose and velocity about the orbit pole by offset / radius, so the offer has the station's orbit a little ahead or behind and does not drift. In flat space it stands on the station's +X with the station's velocity plus spin x arm. A thaw places the offers again by the same rule. Consequence: an elliptic orbit gives a bounded oscillation; offers are not checked against asteroids or the far approach of a port.
    - (B) Pure coast from the spawn velocity. Consequence: in a well, offers drift away from an orbiting station.
    - (C) `AIFollow` station-keeping on the station. Consequence: an AI driver, fuel and formation code on inert offers.
  - Spacing: `hire_offer_offsets` puts offers on one line, alternately ahead and behind, each past the station's reach (`design_reach`) and the one before it plus `HIRE_OFFER_CLEARANCE` (10 units, 100 m).
  - "AI idle": no driver (`SpaceshipController::None`), Neutral, capabilities off, the same as a parked hull for sale.
  - Hire: the offer hull freezes and despawns, and the record thaws as a new entity under `escort_id`, where the offer coasted and at its velocity: an AI driver leashed at `ESCORT_LEASH`, Player side, all capabilities on, with `PlayerFleetMarker` (so `OwnedShipMarker`) and `AIFollow`. Rejected: change the live hull in place. Consequence: every component that a parked hull has and an escort does not have would need a separate removal.

- Agent decision 2026-10-15, item 2 (cards). The agent chose the marked options.
  - How a card shows the hull:
    - (A, chosen) Mirror the parked hull's own meshes. `mirror_hull_previews` (`nova_interface/src/station/hull_preview.rs`) gives each `Mesh3d` under the hull a twin with the same mesh and material handles (`StandardMaterial` or `SectionCracksMaterial`), posed in the hull's frame, on render layer 26 + slot under a turntable that turns at 0.2 rad/s of real time. Consequence: plating, greebles, damage and a changed part show as in the world; one entity per world mesh while the pane is open.
    - (B) Add the card layer to the world hull's own meshes. Consequence: the card camera also draws the hull where it stands in the world, so the hull cannot turn or be framed alone, and the world camera's layer filter changes.
    - (C) Point each card camera at the parked hull in the world. Consequence: the station, other hulls and rocks fill the frame; world lights and shadows decide what the card shows.
    - (D) Keep the block diagram. Consequence: the owner's ask is not met.
  - Light: one key light and one rim light on the union of the live card layers, shadows off, ambient 220 on each card camera. World directional lights stay on the world layer, so a card is lit the same at every station.
  - Cost: the card cameras are active, and the mirrors update, only while the Station pane is on screen; the mirrors are not rebuilt when hidden.
  - Card line: name (it carries civilization, role and Mk), length and price, for example `Fenhalka scavenger Mk 1  60 m, 4400 cr`. The section count, kind summary and trade-in left the card; item 3 puts the trade-in on the Buy action.
  - Escort cards use the same head (`spawn_hull_card_head`) and preview.
  - The Info button is not on the card yet. It comes with item 4, so no card has a button that does nothing.
  - Proof: `shipyard_sandbox` opens the Station tab and shoots `shipyard-sandbox-cards.png`, then `shipyard-sandbox-cards-turned.png` 30 frames later: six lit hulls with plating and greebles, each turned between the shots.

- Agent decision 2026-10-15, item 3 (trade-in in plain words). The agent chose the marked options.
  - Where the words go:
    - (A, chosen) A line over the Buy button (`buy_terms_text` in `nova_interface/src/station/shipyard.rs`): `Your current ship is taken in for 4,665 cr; you pay 795 cr.`, `you get 3,075 cr back.` when the current ship is worth more, `you pay nothing.` when the two are equal. A refused buy shows the refusal in its place. Consequence: a card carries a second line, but only while Buy is shown.
    - (B) In the button label (`Buy: pay 795 cr`). Consequence: the trade-in value is not shown and the button gets long.
    - (C) A confirm step after Buy. Consequence: one more click for every buy.
  - The note after a buy says the same in the past tense: `bought Fenhalka industrial Mk 1 for 5,460 cr: your old ship was taken in for 4,665 cr; you paid 795 cr.`
  - Refusals say it the same way: `buy: X costs 9,540 cr; with your current ship taken in you have 8,000 cr` and `buy: the station would owe you 3,075 cr for your current ship and holds 1,000 cr`.
  - Credits on the cards, the terms, the refusals and the notes, escorts included, are grouped by thousands through the Inventory pane's `cr_text`, now `pub(crate)`.
  - The scripting event keeps `trade_in_cr`: it is a field name for authors, not text for the player.
  - Proof: `shipyard_sandbox` asserts the buy note says the trade-in in plain words; the run logged the note above, and `shipyard-sandbox-cards.png` shows the terms line on each card.
  - For the owner: a just-bought hull trades back in for much less than it cost (the Fenhalka industrial Mk 1 bought for 5,460 cr is taken in for 510 cr), because the trade-in pays strip value at bid. The cards now make that visible.

- Agent decision 2026-10-15, item 4 (Info, read-only). Built after the Ship view lane landed 7e0938571. The agent chose the marked options.
  - How Info reuses the Ship view:
    - (A, chosen) A subject for the Ship pane. `ShipViewSubject(Option<Entity>)` (`nova_interface/src/ship/sections.rs`) names the hull the Ship view shows in place of the player ship. `ShipSections` lists its sections and `assign_section_codes` codes them, so the same scene, badges, orbit, selection and panel serve it. Consequence: small edits in the Ship lane's files (`sections.rs`, `scene.rs`, `catalogue.rs`, `mod.rs`, one init line per test app).
    - (B) A second viewer in the Station pane built from the scene's parts. Consequence: a copy of the orbit, badges and selection code to keep in step.
    - (C) Point the Ship pane's camera at the parked hull in the world. Consequence: world lights and other ships in the view; no section badges.
  - What read-only turns off while the subject is set: `ship_input` takes no edit keys; `update_ship_panel` hides the repair form, the Control row and Rebind; the Catalogue and the gallery stay shut (`refits` is false); `pick_ship_part`, `sync_ship_station_blocks` and `update_ship_rebuild_list` do not run (`ship_view_editable`). Delete stays hidden by its own rule: `plan_ship_delete` finds no player section for the selection.
  - Way in and out: Info on every card, Buy or Hire beside it while allowed, a refused card keeps Info. The Ship panel gets a header with the card's line and Close (`station/hull_info.rs`). Close goes back to the Station pane; any other way out of the Ship pane (a tab, closing the interface) also clears the subject, so the Ship tab shows the player ship again.
  - What the view shows: the Ship view's schematic blocks, badges and the selected section's code, name and condition bar. The long stats text does not come back; the header is the card's one line.
  - Proof: `shipyard_sandbox` clicks Info on the first hull for sale, asserts the Ship pane shows that hull and a section of it is selected, and that the only buttons on screen are the tabs, Fit, Reset and Close; it clicks another section badge, asserts the selection moved, clicks Close, asserts the Station pane and no subject, then opens the Ship tab and asserts the player ship with its Catalogue. Frames: `shipyard-sandbox-info.png`, `shipyard-sandbox-info-section.png`, `shipyard-sandbox-info-closed.png`, `shipyard-sandbox-ship-after-info.png`. `cargo test -p nova_interface --lib ship::` passes (38).

- Owner feedback 2026-10-15, item 5: "when you buy a new ship the old one is for free to switch it back to it, doesn't make sense to ask for money". Agent decision 2026-10-15; the agent chose the marked options.
  - What happens to the old ship on a buy:
    - (A, chosen) It stays the player's. The buy charges the full price (`plan_hull_buy(price, partner, own_cr)`), and the old ship parks where it stood as `ParkedHullType::Owned` with `OwnedShipMarker` and its hold. Its card says `your ship` in place of the price, over a free Board. Consequence: the trade-in, its money and its wording go, and so does the low trade-in point from item 3.
    - (B) Keep the trade-in and make the rebuy free. Consequence: the player is paid for a ship that stays theirs.
    - (C) Sell the old ship for credits. Out of scope.
  - Board raises the same `HullBuyRequest` as a buy, so the swap, the redock and the parking are one path; a board moves no credit and reports no buy (no `OnShipBought`). The request keeps its name; it lost `trade_in_cr`.
  - Removed with no alias: `hull_trade_in_cr`, `HullBuyPlan::trade_in_cr`, `HullBuyRefusalType::StationShortOfCredits`, `ShipInventoryReport::ShipBought::trade_in_cr`, `ShipBoughtEventInfo::trade_in_cr` (the events reference too), `ParkedHullType::Rebuy { price_cr }`, the card's terms line (`buy_terms_text`). The parked-hull save carries `Owned` in place of `Rebuy`; no world save format has shipped, so the format number stays 11 and nothing migrates.
  - Ownership stays one record per ship (`OwnedShipMarker`). Two owned ships may park at one station after two buys; the Inventory pane already lists each one's hold as storage.
  - Notes: a buy says `bought X for 4,400 cr; your last ship is parked here.`, a board `boarded X; your last ship is parked here.`, a refusal `buy: X costs 9,540 cr; you have 8,000 cr`.
  - Port: the bought hull's port is taken by the player and the old ship keeps the port it held, so the swap needs no free port.
  - Proof: `shipyard_sandbox` asserts the buy charged the full price and parked the old ship `Owned`, then clicks Board on its card and asserts both balances unchanged, the player in the old ship with its hold, and the bought hull parked `Owned` with `OwnedShipMarker`. Frames: `shipyard-sandbox-board-before.png` (the old ship's Board card) and `shipyard-sandbox-board-after.png` (the bought hull's). `system_station_buy` claim 5 is now a free board (`outcome: a board hands the player the old ship back for free`).
  - Follow-up 2026-10-15 (orchestrator): owned ships list in their own **Your ships** section above **Ships for sale**, each section shown only while it has a card; the cards stay as they were. A button keeps its index among the station's owned and for-sale hulls in parked order (`HullBuy-{i}`, `HullBoard-{i}`, `HullInfo-{i}`), so a name does not change when a hull moves between sections. The wiki Station paragraph also stopped saying the wreck strip is in the Ship pane; it is in the Inventory pane. Proof: `shipyard_sandbox` asserts the Board button sits above every Buy button (`PASS HullBoard-2 lists under Your ships, above 2 Buy buttons`), frame `shipyard-sandbox-board-before.png`; `system_station_buy` passes, and its `station-buy-offer.png` shows no Your ships before the buy.

## Orchestrator close (2026-10-16)
- The owner tested the shipyard in a live run on 2026-10-16 and accepted it: cards with the hull's model, Buy at the full price, the old ship kept and boarded back for free under Your ships, Info read-only, escorts for hire coasting near the station.
- Landed through d4fb0df00..20a3770ee (e58c84dd0 replaced trade-in with a free board).
- Playable: `nix develop --command cargo run --example shipyard_sandbox --features debug`.

## Owner feedback 2026-10-16
- Technology availability is opaque. Torpedoes and railguns in low brackets are fine, but the player is guessing how a station's ship builder decides what sections it offers (station limits per advancement vs. which section parts the player holds for crafting). Make the rule transparent in the station UI. Recorded here because no open task owns tech locks; the owner may re-home it.
