# Design station and inventory mechanics for the themed UI

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0,stations,inventory,design

## User facts
- Continue iteration and research on stations, inventory and the mechanics offered by the new themed UI. The owner accepts PR #77's visual direction; its stock, credits, loot, trade and repair interactions are illustrative fixtures, not approved game mechanics.
- The themed UI becomes the player's normal TAB interface under the separate NOVA OS migration task. `:` remains the working advanced command entry. Do not reintroduce the terminal as the main station interface or preserve obsolete UI compatibility.
- Start with a larger candidate catalog of Raw ore/scrap/ice, purchasable repair supplies and components, three ammunition families, Food for trade, and Parts as story/objective/scavenged trade goods. Do not add a Fuel category or propulsion/RCS fuel consumption: thrust must not stop for lack of fuel. A cell-like item may be a thruster repair component, not consumable fuel. Research free ship Repair, Reload and idle ammo refill as likely inventory integration points. Design sketch-style Inventory and docked Ship surfaces in parallel; address credits and trading later. Stack implementation on PR #82 rather than putting mock stock or prices into gameplay.

## Agent findings (recheck against live code)
- `20260824-125943/TASK.md` documents docking events and theme/UI seams but no station service, cargo/credits or persistent world mutation runtime. Free repair/reload and idle-refilling base ammunition currently conflict with costed station services. `20260925-190219` already researches a dock-salvage-to-physical-refit loop; `20260925-190156` owns the shared mutation/save contract. This task coordinates them, not duplicates or overrides their unapproved decisions.

## Decision (owner-approved)
- The first real item loop is docked material repair, not ammo supply or UI-only stock. A real `ShipInventory` and a valid, repair-capable dock partner gate material consumption and a real section's `Health` change. No credits, economy or ammo-balance rewrite in this first slice. The exact source item, station identity, quantity/repair rule, interruption policy and lifetime remain to decide before implementation.
- Replace free instant Repair everywhere. The Ship pane's Repair control refuses outside a valid service dock or without material; no free full-heal path survives. M3's stock-backed idle reload and removal of manual Reload are decided below; their remaining balance and initial-stock rules are separate.
- Keep one PDC ammo good compatible with both Kinetic and Pierce mounts; the mount retains its authored damage type. Restrict ammo candidates to PDC ammo, rail slugs and torpedoes; one generic torpedo good works in both Lance and Serpent bays, whose authored type decides its flight behavior. Main thrust and RCS have no consumable fuel or Fuel inventory category. Food/water has no survival consumption, only future trade. Put parts consumed by service in Repair; reserve Parts for scavenged story/objective/trade objects. No pricing or selling is part of the first slice.
- M3 removes the Ship pane's free instant Reload action and its key/button rather than turning it into another refill path. M3 makes existing idle magazine reload draw its matching ammo item from the ship's inventory, with no free refill when reserve is empty. One inventory item represents one PDC shot, rail slug, or torpedo; already-loaded rounds remain usable. Store mass in integer grams for exact capacity/conservation arithmetic and display fractional kilograms. Migrate M2's whole-kg inventory/canister mass APIs at the M3 interface gate; HullPlate remains 10 kg and canister capacity remains 200 kg. At each existing idle cycle boundary, atomically transfer min(missing magazine slots, authored batch amount, available reserve) rounds and consume exactly that many matching items; zero reserve transfers nothing. Keep each installed weapon's authored magazine full at spawn, independent of inventory reserve; only post-spawn replenishment consumes inventory ammo. Author reserve stock explicitly; never mint reserve at spawn. Seed finite PDC, rail and torpedo reserves explicitly on the starting player ship; other ships receive only their authored stock and full installed magazines. Set exact starting counts, ammo masses and any reload timing changes in a later balance gate.
- Start with the Inventory UI against a real `ShipInventory` component on top of PR #82; no mock inventory counters. Do not name the owner or its module `Hold` or `ShipHold`: "hold" is easily confused with the hold-key input action. Next seed a small, explicit supply of hull plates in a starting scenario, then make Ship-view Repair consume plates to restore live section Health at a valid service dock. Salvage as a plate source comes later. UI layout and inventory design may proceed together, but no interface may claim transactions before real state exists.
- Reuse this design task until the mechanics contract is reviewed. Create separate implementation tasks only for approved, code-backed slices.

## Candidate goods (research proposal, not approved runtime content)

PR #77 shows ten fixture goods in six categories (`ui_app_variants.rs:396,3155-3256`). Its Fuel category, masses and prices are not production rules. Proposed Inventory filters are Raw, Repair, Ammo, Food and Parts; no empty Fuel filter. No price, mass or unit below is approved. Before shipping a good, specify its authored ID, quantity unit, mass, source, sink and lint policy. Future-trade-only goods enter live content when trade exists, not as decorative inventory.

| Category | Candidate item | Proposed role | Wave |
| --- | --- | --- | --- |
| Raw | Scrap metal | Salvaged bulk material; future trade/processing | Later |
| Raw | Iron ore | Mining/trade input | Later |
| Raw | Nickel ore | Mining/trade input | Later |
| Raw | Copper ore | Mining/trade input | Later |
| Raw | Titanium ore | Mining/trade input | Later |
| Raw | Silica | Mining/trade input | Later |
| Raw | Carbon | Mining/trade input | Later |
| Raw | Water ice | Mining/trade input, not engine fuel | Later |
| Repair | Hull plates | Docked hull repair material | First candidate |
| Repair | Sealant | Section repair supply; no breach mechanic implied | Later |
| Repair | Wiring bundles | Section repair supply | Later |
| Repair | Coolant | Section repair supply | Later |
| Repair | Actuator kits | Section repair supply | Later |
| Repair | Controller cores | Controller repair component | Later |
| Repair | Thruster assemblies | Thruster repair component | Later |
| Repair | Turret mechanisms | Turret repair component | Later |
| Repair | Docking seals | Docking-port repair component | Later |
| Repair | Reactor fuel cells | Candidate thruster repair component, NOT engine fuel | Later: name/fit review |
| Ammo | PDC ammo | One supply for Kinetic and Pierce PDC mounts | Later: balance gate |
| Ammo | Rail slugs | Railgun magazine supply | Later: balance gate |
| Ammo | Torpedoes | One good for both Lance and Serpent bays; bay determines flight | Later: balance gate |
| Food | Water | Trade good; no thirst system | Later: trading |
| Food | Rations | Trade good; no hunger system | Later: trading |
| Food | Medical supplies | Trade good; no healing system implied | Later: trading |
| Food | Oxygen canisters | Trade good; no life-support consumption | Later: trading |
| Parts | Pumps | Scavenged story/objective/trade item, not generic repair cost | Later: story/trading |
| Parts | Sensors | Scavenged story/objective/trade item | Later: story/trading |
| Parts | Batteries | Scavenged story/objective/trade item | Later: story/trading |
| Parts | Torpedo guidance units | Scavenged story/objective/trade item | Later: story/trading |

PDC mounts have separate authored Kinetic/Pierce damage profiles but share magazine/ballistics (`crates/nova_authoring/src/base_content/sections/turret.rs:1-6`); one supply does not switch their damage type. Lance and Serpent bays share hardware but author distinct flight types (`crates/nova_authoring/src/base_content/sections/torpedo_bay.rs:126-133`); one torpedo supply does not switch their flight type. M3 makes idle refill consume matching inventory ammo and removes instant Reload. Short reserve fills a partial idle batch; initial magazines remain full as installed equipment. Finite starting-player reserve is approved; exact counts, masses and any timing changes await the balance gate.

## Research and decisions, not implementation approval
- Decide what a station is, how it gets a stable identity, who may dock, how visits and interruptions work, and which services should exist. Distinguish a real dockable station from a boarded ship/derelict, with partner-specific capabilities and refusal/rollback on undock, unloaded partner, broken port or missing stock.
- Specify inventory item/stack/content ownership, ship inventory/capacity, source/sink and market stock, currency or no currency, buy/sell versus free salvage, repair/reload policy, and a physically observable ship-upgrade payoff. Contrast proposed loops with current free repair/reload and idle-batch ammo; choose what dies or stays before designing prices. Preserve simulation and content/lint boundaries.
- Define what basic map/ship/inventory views show while undocked and which real actions are dock-gated. Align station UI surfaces with the TAB migration without treating PR #77's example controls, numbers, assets or layout as production schemas. Coordinate persistence, consumed-body regeneration and service interruption with `20260925-190156`; defer implementation of saves until its owner-reviewed contract exists.
- First slice: trace `ShipSectionCommand -> apply_ship_section_commands -> apply_action_to_section` (`crates/nova_interface/src/ship/app.rs:165-207`, `ship/sections.rs:448-523`) and remove the free full-heal path when material-backed service replaces it. `DockingConnection` identifies a partner, not a service (`crates/nova_ship/src/sections/docking_section/connection.rs:59-99`). Specify station authorization, plate acquisition, material quantity per restored HP, atomic refusal and lifetime before adding types or tests.
- Owner approved the first code-backed implementation slice on 2026-09-27: Inventory UI and real `ShipInventory` on a branch stacked on PR #82. Proposed `crates/nova_gameplay/src/inventory.rs`: `ItemType { HullPlate }`, `ItemCategoryType { Raw, Repair, Ammo, Food, Parts }`, `ShipInventory { stacks: BTreeMap<ItemType, u32> }` with private stacks, `count(ItemType) -> u32`, `stacks() -> impl Iterator<Item = (ItemType, u32)>`, `FromIterator<(ItemType, u32)>` that rejects zero quantities, and `ItemType::category(self) -> ItemCategoryType` (`HullPlate` -> `Repair`); require an empty inventory on every `SpaceshipRootMarker`. No capacity, starter plates or transaction in this slice. Proposed `crates/nova_interface/src/inventory/`: `InventoryPanePlugin`, `InventoryPaneSystems`, `InventoryRuntime { filter: Option<ItemCategoryType>, selected: Option<(InventorySideType, ItemType)> }`, `inventory_body(&mut ChildSpawnerCommands)` and `update_inventory_panel`. Add `InterfacePaneType::Inventory`, five category filters (no Fuel), own/partner inventory columns and inspector, but draw only real state. Missing partner reads "Not docked."; empty inventory reads "Inventory empty."; missing required component fails loudly. M/Y cycles Map -> Ship -> Inventory -> Map. Verify ECS inventory/partner rows, pointer selection/filter, a rendered frame against PR #77, and affected wiki/changelog docs. Approval covers only the listed Inventory slice, types, functions and focused proof; starter plates and docked material repair require a separate gate.
- Follow-up slice after its own gate: seed finite hull plates in a starting scenario and make docked Ship-view Repair spend them atomically with the Health change. Add salvaging or market acquisition only after that loop is proved.
- Later slices: ammo policy (base magazines auto-refill by design: `crates/nova_ship/src/sections/ammo.rs:183-289`, `crates/nova_authoring/src/base_content/sections/mod.rs:289-329`); market stock, credits and atomic trading of Raw/Food/Parts; story objectives for scavenged Parts; persistent retirement/revisit and physical refits under `20260925-190156` and `20260925-190219`. Engine/RCS fuel and food consumption are explicitly out. None of these slices is authorized by the catalog table.

## Verification to design
- Map each proposed transaction from trigger through ownership, atomic success/refusal and world mutation. Specify tests for wrong/absent dock partner, interrupted or broken-port transaction, insufficient stock/cargo/credits, duplicate salvage, retirement/revisit and a changed real ship. Save/restart claims depend on the separately approved persistence tier. Include an asserted player flow for the first loop, not UI-only counters.

## Done when
- An owner-reviewed mechanics specification chooses the minimum real station/inventory loop, its interface and content boundaries, error policy and dependencies; cross-task persistence and UI choices are linked; resulting implementation tasks have a code-backed gate. This task alone does not add runtime station, cargo or economy systems.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on 2026-09-26. Coordinate the existing `20260925-190219` dock-salvage design and `20260925-190156` save contract instead of closing either by implication.
