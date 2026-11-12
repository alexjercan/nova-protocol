# Second-tier sections and items with real sinks for the shipyard and swap

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.18.0, content

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction:
more content, items and sections, so the shipyard and family swap have
something to upgrade into.

## Agent findings
- Base pack: 20 items in 5 categories (`assets/base/items/base.content.ron`;
  categories `crates/nova_gameplay/src/inventory.rs:323-334`): Raw x4, Ammo x3,
  Repair x1, Food x1, Parts x11. Nothing consumes ore except selling it.
- 20 sections in 9 kinds (`crates/nova_ship/src/sections/base_section.rs:465-482`):
  Hull 6, Thruster 3, Turret 4, Torpedo 2; Controller, Railgun, Docking,
  CargoIntake, Mining one each.
- Tiered shipyard and same-family swap landed in tasks 20261012-100012 and
  -100014 with little to swap into.
- Item and recipe prices are marked provisional
  (`crates/nova_authoring/src/base_content/items.rs:14`,
  `sections/recipes.rs:6`); mining beam stats provisional
  (`sections/mining_beam.rs:18,107`).
- Generated hulls build from parts for 4 roles (`crates/nova_gameplay/src/market.rs:63-72`);
  new sections only reach generated ships through the layout tables in
  `crates/nova_world_base/src/ship_layout.rs`.

## Decisions (owner said continue 2026-11-12; agent calls, stated)
Map with path:line evidence in `MAP.md`. Three spec corrections: advancement
is a 0..1 value (not "3"); a swap needs the same collider footprint, so a
bigger intake is another family; layouts pick by family, advancement,
knowledge and draw weight, never by id.
- Sections: one second-tier variant per single-variant kind, same glb and
  same collider (the light/reinforced hull precedent), advancement 0.3,
  draw_weight 0.6, recipe about 1.5x with one `AlloyPlate` each:
  `wide_intake_section` (aperture 28 x 18 m inside the 30 x 20 face, 60 m
  detection, health 120); `long_mining_beam_section` (reach 160 m, pulse
  0.8 s, health 110); `armoured_controller_section` (health 160, lag 0.7,
  torque 11000; keep the compile-time durability order); `wide_docking_port_section`
  (15 m, 25 deg, 8 m/s, 8 deg/s, health 110); `heavy_railgun_section`
  (charge 2.0 s, more slug damage and power, adv 0.4). Plus a third torpedo
  bay type `dart` (fast, no weave, own tint). No extra thruster: three exist
  and no two share a footprint, so a fourth upgrades nothing.
- Items (three new, each with a consumer other than the sell button):
  `AlloyPlate` (Parts) made at a station with the new `Refine` service from
  2 IronOre + 1 CarbonOre for a fee, offered in the inventory market strip;
  every second-tier recipe needs one. `Medkit` (Repair) consumed by a
  Distress hand-in (one per rescue, reward covers it); stations stock it,
  provisioner sells it, garrison procures it. `Contraband` is a new item
  category (closed enum; UI matches updated) with one item `UnmarkedCores`
  that scavenger traders sell; a civilization seeded `lawful` refuses a dock
  to a hull carrying any Contraband item (`DockRefusalType::Contraband`,
  HUD and chart show it).
- Generated stations with civ advancement >= 0.25 offer Refine. Base
  waystation offers it too. Knowledge keeps its 50 % per-civ draw; the proof
  asserts over the census civs, not one seed.
- No new glb. No new sound. Prices stay provisional; the census and ledger
  recipe lints must pass unchanged.

### Interfaces (agent calls, 2026-11-12, from the code read)
- Refine: `ItemDesign` gains optional `refine: Option<ItemRefineRecipe>`
  (`inputs: Vec<(ItemDesignId, u32)>`, `fee_cr`), one output unit per batch;
  absent means the item is not refined. `AlloyPlate` authors 2 IronOre +
  1 CarbonOre + 30 cr. Lint: inputs known, counts > 0, no input is the
  output, and inputs at ask plus the fee is not below the output's bid (no
  arbitrage out of station stock). `StationServiceType::Refine`; pure
  `plan_item_refine` beside `plan_item_trade`; the fee goes to the station.
  The Inventory pane offers it on a hold row that is an input of a recipe.
- Medkit: new required `ItemRoleType::Medkit`. A Distress rescue takes one
  role item from the hold at the dock with the caller; without one the job
  stays open and the HUD says why. `JobType::Distress` keeps its saved shape.
  The Distress reward adds the medkit's reference ask times 1.5.
- Contraband: `ItemCategoryType::Contraband`; `LawfulStationMarker` on the
  station root, written by `nova_world_base` from a per-civilization hash
  draw that consumes nothing from the generator's stream (shipped worlds
  must regenerate unchanged); `station_admission` gains the contraband
  input and `DockRefusalType::Contraband`.
- As built: the heavy lance opens at advancement 0.6, not 0.4, so the
  armament census still mounts no lance below 0.5 (0ce447583).

## Delivery
- Owners: `nova_authoring` base_content items, sections, recipes, trade
  profiles; `nova_world_base` ship_layout and civilizations;
  `nova_gameplay` inventory categories if a new category is needed.
  Dies: nothing. May break: lint ranges, the berth census test in
  `nova_authoring`, ledger webmod expectations, price tables.
- Sequence: sections -> layouts -> items and sinks -> trade profiles -> wiki
  catalogue pages and changelog.

### Delivered (2026-11-12, master, not pushed)
| Commit | Result |
|---|---|
| 0ce447583 | Five second-tier sections (wide intake, long mining beam, armoured controller, wide docking port, heavy lance) and the `AlloyPlate` part |
| dac14e118 | `dart_torpedo_section`: third torpedo type, 450 m/s cruise, no weave |
| 09d194efe | `ItemDesign::refine`, `plan_item_refine`, `StationServiceType::Refine`, lint, base content |
| 08d80084a | Refine chip in the Inventory pane, `system_station_refine`, docs |
| bb3f9f8a5 | `ItemRoleType::Medkit`; a distress rescue spends one; `NO MEDKIT ABOARD` |
| eb7d41680 | `ItemCategoryType::Contraband`, `UnmarkedCores`, `Civilization.lawful`, `LawfulStationMarker`, `DockRefusalType::Contraband` |
| 842a57d6a | `system_station_swap_intake`, `generated_traders` integration test |
| 1d9df9444 | Torpedo run-in figures re-measured in code docs, wiki and widget |
| 8e321f92c | Price widget mirrors the Scavenger Contraband rule (web test was red since eb7d41680) |

Evidence:
- Content lint: 0 errors, 0 warnings, 0 findings after every content commit.
- `system_station_swap_intake`: the catalogue offers the wide intake, the
  swap keeps the hold (it is `ShipInventory` on the ship root), 480 cr paid.
- `ship_layout` test: an Industrial hull at mid advancement mounts a
  second-tier section. Census: 6 of 8 civilizations build the Dart at 0.5.
- `system_station_refine`: one alloy plate refined, 30 cr to the station; a
  short refine is refused with the missing input named. Frame
  `refine-form.png`.
- `system_job_distress`: the dock with no medkit leaves the call open, the
  dock with one pays 690 cr.
- `system_contraband_dock`: refused with the item, admitted without. Frames
  `contraband-hud.png`, `contraband-inventory.png`.
- `generated_traders`: a generated provisioner sells medkits and a salvage
  broker sells unmarked cores through the real base profiles.
- `lawful` is a hash draw of its own; the pinned-window body hash is
  unchanged. `Civilization` is not serialized, so old saves derive it.

Limits:
- The example marks the authored waystation lawful by hand. A generated
  lawful station that refuses the player was not flown in a live world.
- A frozen station in an old save keeps its service list and gains no
  Refine. Intact scavengers in a regenerated world may carry one core.
- Authored stations are never lawful. Double-click with Refine picked is
  not tested. The Dart was not flown or seen in a frame.
- The editor sandbox `SERVICES` list has no Refine (`is_stock_station`
  compares against it).
- `system_civilization_stance` still panics at its last beat ("has no free
  docking port"); it did so before this task.
- Torpedo docs: the weave swing reads 99 m on the rig and the docs say
  111 m; the Serpent arrival gap over the Lance is 0.53 s, not 0.68 s.
  Values are unchanged; the 320 m/s reasoning is for the owner to judge.
- Prices stay provisional.

## Verification
- Lint passes on the regenerated pack; census test counts the new sections.
- Asserted example: a shipyard at advancement 3 offers the tier-two intake and
  the swap keeps cargo; a generated Industrial hull at advancement 3 carries a
  tier-two section.
- Trade probe: a generated trader sells the new goods at a station that
  stocks them, and the contraband is refused at a Lawful station.

## Done when
- No section kind has a single variant.
- At least three items have a consumer other than the sell button.
- Generated ships and shipyards show the new content without authoring.
