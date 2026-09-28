# Design station and inventory mechanics for the themed UI

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0,stations,inventory,design

## User facts
- Continue iteration and research on stations, inventory and the mechanics offered by the new themed UI. The owner accepts PR #77's visual direction; its stock, credits, loot, trade and repair interactions are illustrative fixtures, not approved game mechanics.
- The themed UI becomes the player's normal TAB interface under the separate NOVA OS migration task. `:` remains the working advanced command entry. Do not reintroduce the terminal as the main station interface or preserve obsolete UI compatibility.
- Start with a larger candidate catalog of Raw ore/scrap/ice, purchasable repair supplies and components, three ammunition families, Food for trade, and Parts as story/objective/scavenged trade goods. Do not add a Fuel category or propulsion/RCS fuel consumption: thrust must not stop for lack of fuel. A cell-like item may be a thruster repair component, not consumable fuel. Design sketch-style Inventory and Ship surfaces in parallel; address credits and trading later.
- 2026-09-28 owner direction overrides the earlier docked-repair decision: repair your own ship anywhere if the inventory holds plates. No dock, partner or repair service gates M0. A station "repair service" may come later as a separate, unapproved idea.

## Decisions (owner-approved)

### Standing
- Keep one PDC ammo good compatible with both Kinetic and Pierce mounts; the mount retains its authored damage type. Restrict ammo candidates to PDC ammo, rail slugs and torpedoes; one generic torpedo good works in both Lance and Serpent bays, whose authored type decides its flight behavior. Main thrust and RCS have no consumable fuel or Fuel inventory category. Food/water has no survival consumption, only future trade. Put parts consumed by service in Repair; reserve Parts for scavenged story/objective/trade objects.
- Do not name the inventory owner or its module `Hold` or `ShipHold`: "hold" is easily confused with the hold-key input action. No interface may claim a transaction before real state exists.
- Reuse this design task until the mechanics contract is reviewed. Create separate implementation tasks only for approved, code-backed slices.

### M0: plate repair anywhere (next worker scope, only this)
Source: owner direction of 2026-09-28 (plate rule, stock, UI and refusal policy below) and comments `c-a8b85d98e1cb1139` (repair anywhere with plates), `c-9637f2cac9d955fd` (D1: no partner feature yet), `c-650e12d40ca8fcd2` (not only if docked), `c-410509389a918381` (D2 option b, partial), `c-38fa9baa11da9441` (D3 option a, instant) and `c-fda135c07073cf84` (M0 is plate repair only). The ratio of 20 comes from the 2026-09-28 direction; the report only used it as an illustration.
- Scope: the Ship pane Repair action on a damaged, non-destroyed player-owned section spends `HullPlate` from the player ship's `ShipInventory` and raises that section's `Health`. It works undocked and docked alike; docking state is not read. The owner excluded destroyed (0 HP) sections from M0 repair on 2026-09-28; do not revive or clear their disabled markers.
- Stock: the 12 authored `HullPlate` on the open-world player ship are the only stock. Season-one and tutorial ships stay empty. No other source adds plates in M0.
- Rule: one plate restores up to 20 Health. For missing = `max - current`, spend `min(available, ceil(missing / 20))` plates and set `current = min(max, current + 20 * spent)`. Fractional missing Health rounds up to one more plate. There is no partial plate credit: leftover restore capacity of the last plate is lost, not banked.
- Atomicity: instant, in one system run. Plate spend and Health change commit together or not at all. No timed job, no interruption, no refund (D3 comment: "never anything else").
- No free path: the free full-heal branch dies. A Repair with zero plates changes nothing.
- UI: the Repair button is disabled with a reason when the selected section is at full Health, destroyed (0 HP), or the player has no plates. A command that still arrives (the `ship_repair` key sends without the panel gate, stale selection, wrong ship, non-section entity) refuses and mutates nothing.
- Out of M0: repair of any other ship, dock or station service, partner checks, Reload, idle ammo refill, ammo goods, credits, prices, capacity, pickup, transfer, jettison, and persistence.
- Lifetime: session-only. Spent plates return when the authored scenario loads again. Persistence waits for the separate contract in `20260925-190156`; M0 docs must state the limit, not hide it.

### Later milestones (owner-approved direction, each needs its own code-backed gate)
Source: owner comments on [item-loop-design.html](item-loop-design.html), export `/home/alex/Downloads/item-loop-design-comments-2026-09-28T05-49-17-668Z.json` (`exportedAt` 2026-09-28T05:49:17Z, 24 comments). The comment id follows each item.
- Milestone order: M0 plate repair only; M1 dock Take/Give; M2 intake pickup and jettison; M3 ammo reserve; M4 station refit; M5 trade and mining. M2-M5 "sounds good" (`c-fda135c07073cf84`).
- M1 transfer authorization (D4 option b as amended, `c-30a94a2fa474a53e`, `c-fda135c07073cf84`, `c-3ee09c4b5c0d48ed`): Take only from a docked partner that is neutralized or explicitly labelled derelict/lootable. Never Take from a live healthy ship; that is stealing. Getting goods from a live ship is trading for credits (M5), not Take. Give works to any valid docked ship, for example quest delivery.
- Destruction loot (`c-3ee09c4b5c0d48ed`, `c-7d5add5a6ccb9958`, `c-d9f78be1000375fd`; "only remaining cargo, no bonus" from the 2026-09-28 direction): an authored loot table seeds a ship's inventory before destruction; destroying it drops only the cargo it still carries as canisters. No second roll, no bonus loot on top of the inventory.
- M2 pickup (D5 option a, "never something else", `c-db1b35a8bf5532c2`, `c-39c5125000d52307`; two-stage form and slow crossing from the 2026-09-28 direction): a cargo intake component with an automatic two-stage front sensor. A larger detection volume opens the intake with no player action; a smaller capture volume takes a canister that crosses it slowly. Other hull collisions never collect cargo.
- M2 jettison (D10 option a, `c-2f3bdf95ef0c51cd`, `c-b2eef5a0ab2d89ad`): drop a chosen quantity from the inventory as one canister that carries exactly those items.
- M2 containers (`c-3e97b24b96a13ca8`): one generic canister model for every good; a tooltip or lock detail names the contents when close or locked.
- M2 salvage crate (D13 option b, `c-7e479c9a4164adb8`): the cargo canister replaces the original `SalvageCrate` at M2, as a follow-up once the loop works.
- Capacity (D8, `c-0386f4fbe225e08b`): none through M1; count-based capacity (option b) at M2, when jettison exists.
- M3 ammo (D9 option b, `c-0b385ddee52eaa57`): idle batch refill draws from an inventory reserve; free Reload dies. Its own balance gate decides rates. Ammo can also come from salvage of destroyed or neutralized ships (`c-1af4b52ea6464572`, `c-a15788ea2e4dd648`, `c-d9f78be1000375fd`).
- M3 removes the Ship pane's free instant Reload action and its key/button rather than turning it into another refill path. M3 makes existing idle magazine reload draw its matching ammo item from the ship's inventory, with no free refill when reserve is empty. One inventory item represents one PDC shot, rail slug, or torpedo; already-loaded rounds remain usable. Store mass in integer grams for exact capacity/conservation arithmetic and display fractional kilograms. Migrate M2's whole-kg inventory/canister mass APIs at the M3 interface gate; HullPlate remains 10 kg and canister capacity remains 200 kg. At each existing idle cycle boundary, atomically transfer min(missing magazine slots, authored batch amount, available reserve) rounds and consume exactly that many matching items; zero reserve transfers nothing. Keep each installed weapon's authored magazine full at spawn, independent of inventory reserve; only post-spawn replenishment consumes inventory ammo. Author reserve stock explicitly; never mint reserve at spawn. Seed finite PDC, rail and torpedo reserves explicitly on the starting player ship; other ships receive only their authored stock and full installed magazines. Set exact starting counts, ammo masses and any reload timing changes in a later balance gate.
- M4 station refit: coordinated with `20260925-190219`, which researches refit form. Its proposal text still weighs costed dock repair and undocked refusal (`20260925-190219/TASK.md:14-18`); the M0 decision here supersedes that for repair.
- M5 trade and mining: credits, market stock, Raw/Food/Parts. Mining is a short-range beam that is not usable as a melee weapon and only yields ore canisters (D6 option b, `c-58fca19e8e99965a`, `c-3e97b24b96a13ca8`); combat carving yields nothing (D7 option a, `c-d8d5e922d1b0a8cb`).
- Salvage persistence (D11 option a, `c-c0b2577c4e5749f6`): loot only on authored scenario objects until `20260925-190156` lands.
- Item identity (D12 option a, `c-d46557466093e3c0`): closed `ItemType` until trade; revisit at M5, possibly moddable.

### Not approved (open; decide at the owning gate)
- M0 gate details listed under Delivery: mutation API, rule owner, target ownership check, disabled-reason precedence, and refusal text.
- Intake detection and capture sizes, the value of the slow-crossing speed limit, and the intake's hull placement.
- Ore yield per carve, beam range, asteroid-kind-to-good map.
- Transfer quantity selection and its UI; loot table identities and contents; the name of the derelict/lootable label.
- Ammo reserve rates and balance; the fate of the rate-limit catalog test.
- Prices, credits, market stock; the capacity count; refit form; save tier and claim-once.

## Candidate goods (research proposal, not approved runtime content)

PR #77 shows ten fixture goods in six categories (`ui_app_variants.rs:396,3155-3256`). Its Fuel category, masses and prices are not production rules. Proposed Inventory filters are Raw, Repair, Ammo, Food and Parts; no empty Fuel filter. No price, mass or unit below is approved. Before shipping a good, specify its authored ID, quantity unit, mass, source, sink and lint policy. Future-trade-only goods enter live content when trade exists, not as decorative inventory. The Wave column below predates the owner milestones; the milestone list above wins.

| Category | Candidate item | Proposed role | Wave |
| --- | --- | --- | --- |
| Raw | Scrap metal | Salvaged bulk material; future trade/processing | Later |
| Raw | Iron ore | Mining/trade input; also loot-table cargo | Later |
| Raw | Nickel ore | Mining/trade input | Later |
| Raw | Copper ore | Mining/trade input | Later |
| Raw | Titanium ore | Mining/trade input | Later |
| Raw | Silica | Mining/trade input | Later |
| Raw | Carbon | Mining/trade input | Later |
| Raw | Water ice | Mining/trade input, not engine fuel | Later |
| Repair | Hull plates | Plate repair material, used anywhere | M0 (exists as `ItemType::HullPlate`) |
| Repair | Sealant | Section repair supply; no breach mechanic implied | Later |
| Repair | Wiring bundles | Section repair supply | Later |
| Repair | Coolant | Section repair supply | Later |
| Repair | Actuator kits | Section repair supply | Later |
| Repair | Controller cores | Controller repair component | Later |
| Repair | Thruster assemblies | Thruster repair component | Later |
| Repair | Turret mechanisms | Turret repair component | Later |
| Repair | Docking seals | Docking-port repair component | Later |
| Repair | Reactor fuel cells | Candidate thruster repair component, NOT engine fuel | Later: name/fit review |
| Ammo | PDC ammo | One supply for Kinetic and Pierce PDC mounts; station or salvage | M3: balance gate |
| Ammo | Rail slugs | Railgun magazine supply; station or salvage | M3: balance gate |
| Ammo | Torpedoes | One good for both Lance and Serpent bays; bay determines flight; station or salvage | M3: balance gate |
| Food | Water | Trade good; no thirst system | M5: trading |
| Food | Rations | Trade good; no hunger system | M5: trading |
| Food | Medical supplies | Trade good; no healing system implied | M5: trading |
| Food | Oxygen canisters | Trade good; no life-support consumption | M5: trading |
| Parts | Pumps | Scavenged story/objective/trade item, not generic repair cost | Later: story/trading |
| Parts | Sensors | Scavenged story/objective/trade item | Later: story/trading |
| Parts | Batteries | Scavenged story/objective/trade item | Later: story/trading |
| Parts | Torpedo guidance units | Scavenged story/objective/trade item | Later: story/trading |

PDC mounts have separate authored Kinetic/Pierce damage profiles but share magazine/ballistics (`crates/nova_authoring/src/base_content/sections/turret.rs:1-6`); one supply does not switch their damage type. Lance and Serpent bays share hardware but author distinct flight types (`crates/nova_authoring/src/base_content/sections/torpedo_bay.rs:126-133`); one torpedo supply does not switch their flight type. M3 makes idle refill consume matching inventory ammo and removes instant Reload. Short reserve fills a partial idle batch; initial magazines remain full as installed equipment. Finite starting-player reserve is approved; exact counts, masses and any timing changes await the balance gate.

## Agent findings (code at `a61d8fbcc`; recheck against live code)
- Inventory owner, count-only: `ShipInventory { stacks: BTreeMap<ItemType, u32> }` has private stacks and read-only `count`, `stacks`, `is_empty` (`crates/nova_gameplay/src/inventory.rs:52-75`). The only constructors are `FromIterator`, which panics on a zero or repeated stack (`inventory.rs:77-93`), and the hand-written `Deserialize` (`inventory.rs:113-148`). The module doc says nothing spends or adds items (`inventory.rs:4-8`). `ItemType` has one variant, `HullPlate`, filed under Repair (`inventory.rs:19-34`).
- Stock: `crates/nova_authoring/src/base_content/scenarios/open_world.rs:70` authors `HullPlate: 12`; generated `assets/base/scenarios/open_world.content.ron:70-71` carries it.
- Command entry: the `ship_repair` key writes `ShipSectionCommand { target: runtime.selected, action: Repair }` with no panel gate (`crates/nova_interface/src/ship/scene.rs:546-551`). The panel button writes the same message only when `panel_repair_enabled` (`scene.rs:914-932`).
- Command apply: `apply_ship_section_commands` reads each message, skips silently when the target is not a classified section, and calls `apply_action_to_section` (`crates/nova_interface/src/ship/app.rs:141-182`). It reads no inventory, no docking state and no ship ownership: a section of any ship passes the query.
- Free heal: `ShipAction::Repair` sets `health.current = health.max` when `max > 0`, else returns "no integrity to restore" (`crates/nova_interface/src/ship/sections.rs:499-514`). The doc comments at `sections.rs:443-444,463-465` still describe a future queued job model; D3 rejects it.
- Panel gate: `panel_action_state` enables Repair on `Health.max > 0` only (`sections.rs:415-441`). `PanelActions.reason` is one `Option<String>`, and for a non-weapon section the reload reason wins, so a hull section never shows a repair reason today (`sections.rs:422-434`).
- Ordering: `apply_ship_section_commands` runs second in the chained `ShipPaneSystems` (`crates/nova_interface/src/ship/mod.rs:88-107`); `update_inventory_panel` reads the player inventory every frame while the pane body exists, in `InventoryPaneSystems` (`crates/nova_interface/src/inventory/app.rs:550-590`, `inventory/mod.rs:32`). Both sets run after `InterfaceSystems` and before `UiThemeSystems` (`crates/nova_interface/src/lib.rs:124-144`), with no order between them.
- Health is `f32` (`crates/nova_gameplay/src/integrity/health.rs:29-37`). Accumulated `f32` damage can leave missing Health a hair above a multiple of 20, which the round-up rule charges one more plate. At 0 HP `on_damage` inserts `HealthZeroMarker` (`health.rs:148-151`), which the integrity core turns into `IntegrityDisabledMarker` (`crates/nova_gameplay/src/integrity/core.rs:361`). Today's free Repair raises such a section's Health without removing those markers.
- Docs that M0 invalidates: `web/src/wiki/interface.md:139` (Repair row) and `:151` ("instant and free today"), `CHANGELOG.md:66-68` ("nothing adds or spends items yet").
- The closest real-game driver is `examples/systems/system_open_world.rs` (New Game, Retry, back to menu); it declares one subject, so a repair flow may need its own example. Unverified: whether Retry respawns the player ship with the authored 12 plates.

## Delivery

### M0 end state
- Repair on a damaged player-owned section spends plates by the M0 rule and updates Health in the same system run, anywhere.
- Dies: the free full-heal branch (`sections.rs:499-509`), the `Health.max > 0`-only enable rule (`sections.rs:418`), the stale queued-job doc comments, and the wiki line "instant and free today".
- May break: `crates/nova_interface/src/ship/tests.rs` repair cases at 121-160 (`ship_action_keys_mutate_through_message_handler`) and 1066-1105 assume a free heal; `panel_action_state_gates_repair_and_reload` at 894 assumes the old gate. Update or delete them with the change.
- Must fail loudly: a player ship without `ShipInventory` (existing panic contract, `inventory/app.rs:582-585`).

### M0 implementation gate (PROPOSED, not approved)
Nothing here is an approved name or signature. The next worker must show exact paths, lines, types, signatures and a before/after call graph and get approval before code.
- Inventory mutation: `ShipInventory` needs one spend operation that removes a count only when the stack holds it and deletes an emptied stack. Name, signature, return type and error type are open.
- Rule owner: a pure plate-count and restore function, likely in `nova_gameplay` beside `ShipInventory` so it can be unit-tested. Crate and name are open.
- Command path: `apply_ship_section_commands` gains write access to the player ship's `ShipInventory` and validates, in a fixed order, live section, player-owned, `Health.max > 0`, damaged, plates > 0, before one commit. How ownership is resolved (section to root) is open.
- Panel: `panel_action_state` needs the player plate count. Reason text and precedence against the reload reason are open.
- Refusal feedback: whether stale or non-section targets write a note line or stay silent is open.
- Destroyed section: 0 HP is excluded from M0 repair by the owner. Both the panel and command handler must refuse it without plate spend or marker changes. Do not assume raising Health alone restores a disabled section.
- Docs and changelog: wiki interface page and an `[Unreleased]` entry, per AGENTS.md changelog policy.

### M0 proofs to gate (candidates; each new test needs approval)
- Understock: 2 plates, section at 40/100: spend 2, Health 80/100, stack gone.
- 1 HP scratch: 99/100 costs 1 plate and ends at 100/100; fractional missing (99.5/100) also costs 1.
- Max clamp: 3 plates, 55/100: spend 3 (ceil(45/20)), Health exactly 100, not 115.
- Zero stock: damaged section, 0 plates: button disabled with reason; a forced command leaves Health and inventory unchanged.
- Full section: button disabled with reason; a forced command spends nothing.
- Wrong target: a section of a docked or other ship, a despawned entity, a non-section entity: no mutation on either ship.
- Destroyed or invalid section: `Health.current <= 0`, `Health.max <= 0`, or no `Health`: refuses, no spend; a destroyed section retains its disabled markers.
- Repeated same-frame commands: two Repair messages for one section in one update validate against committed state; the second spends nothing when the first healed fully, and total spend equals the rule once.
- No free repair: no code path raises Health from the Ship pane without a matching plate spend.
- Inventory UI: after a repair, the Inventory pane row for Hull plates shows the new count; an emptied stack reads "Inventory empty." when it was the last stack.
- Real game: open world New Game, take damage, repair undocked, assert plates and Health at each beat (example or `nova-bench` flow, chosen at the gate).
- Saved limitation: state in docs that spent plates return on a new session or scenario reload; if Retry restores 12 plates, record it as current behavior, not a bug.

### After M0
- Each later milestone gets its own code-backed gate with its open items resolved first. Coordinate persistence and claim-once with `20260925-190156` and refits and dock authorization with `20260925-190219`.

## Design report
- [item-loop-design.html](item-loop-design.html) holds current-code evidence, milestone staging M0-M5, candidate transactions and decisions D1-D13. It was reconciled with the 2026-09-28 owner decisions: M0 is plate repair anywhere with no dock or partner, D1-D3 carry the owner answers, D4-D13 carry owner direction, and a station repair service appears only as a later, unapproved proposal. Its tags separate Current code, Decided M0 scope, later Direction, Proposal and Open. If the report and this file disagree, this file wins.
- Comment anchors: 22 of the 24 exported comments stay attached. Two anchored texts had to change because they stated the replaced M0: `c-a8b85d98e1cb1139` (heading "Costed docked repair (M0)") and `c-650e12d40ca8fcd2` (Example 1 rule "r = 20, not approved"). The page shows both under Detached comments with their original text; the export file keeps them too.

## Done when
- An owner-reviewed mechanics specification chooses the minimum real inventory loop, its interface and content boundaries, error policy and dependencies; cross-task persistence and UI choices are linked; resulting implementation tasks have a code-backed gate. This task alone does not add runtime station, cargo or economy systems.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on 2026-09-26. Coordinate the existing `20260925-190219` dock-salvage design and `20260925-190156` save contract instead of closing either by implication.
