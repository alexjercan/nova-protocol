# Sections and items: slice map (read-only, 2026-11-12, master 7b4b9194b)

## Corrections to the spec
- Advancement is a value in [0, 1] (`crates/nova_ship/src/sections/base_section.rs:363-366`,
  `crates/nova_world_base/src/civilizations.rs:57-61,161-163`). The first four
  rings read at most 0.15. "Advancement 3" does not exist; three stations is
  `1 + floor(adv * 2)` (`sector_stations.rs:52,116-118`).
- A swap needs the same `SectionKind::class` AND the same collider footprint
  (`crates/nova_world_base/src/design_sections.rs:170-195`). A heavier intake with a
  bigger collider is another family. Section mass is collider volume
  (`base_section.rs:51-56`).
- Generated layouts pick by family, advancement, civilization knowledge and
  `draw_weight`, never by id (`ship_layout.rs:1405-1466`). Only thrusters
  prefer their tier band (:824-830, :1436). Nothing in a table names a section.
- No berth census test exists in `nova_authoring/tests`; the census is
  `generated_armament.rs:19-90` (turrets, bays, lances over 8 civs).

## Sections
- Kinds `base_section.rs:463-482`; `BaseSectionConfig` :348 (health, build,
  advancement :366, draw_weight :371, collider :392, link_points, animations).
  `RenderMeshTransform` :283-302, applied to render children only; colliders
  and intake/emitter face math assume no scale (`cargo_intake_section.rs:397-401`,
  `mining_section.rs:190-192`). No material or tint field on sections.
- Per-kind fields: intake `cargo_intake_section.rs:94-127` (detection_range,
  capture_gap, aperture w/h, eject_speed); mining `mining_section.rs:38-65`
  (reach, pulse_interval_seconds, carve_radius_cells); controller
  `controller_section.rs:33-43` (steering_lag, max_torque); docking
  `docking_section/mod.rs:87-140` (capture_distance, capture_angle, max
  relative speed, max angular speed); thruster `thruster_section.rs:46`
  (magnitude, exhaust, loop_sound); torpedo type `torpedo_section/mod.rs:379-436`
  (tint, max_speed, weave_angle, weave_rate).
- Precedent for a re-tuned variant on one mesh: reinforced vs light hull share
  `hull_personnel.glb`, differ in health 200/60, adv 0.3/0.0 and recipe
  (`base_content/sections/hull.rs:55-196`). Turrets differ by art, fire rate,
  damage type, adv 0.0/0.25 (`turret.rs:474-531`). Torpedo bays differ only in
  `torpedo_type`; test `torpedo_types_differ_only_in_how_the_ordnance_flies`
  pins every other stat equal at equal blast damage, and each type needs a
  distinct tint.
- Durability ordering is compile-time: THRUSTER < CONTROLLER < TURRET health,
  CONTROLLER == TORPEDO (`base_content/sections/mod.rs:61-90`).
- Single-variant kinds, all adv 0.0 draw 1.0: controller (`controller.rs:19-56`,
  health 100, lag 0.5, torque 9760), docking (`docking_port.rs:59-105`, health
  90, 10 m / 15 deg / 5 m/s / 5), intake (`cargo_intake.rs:87-128`, health 90,
  cuboid 3x2x1, 40 m / 1 m / 22.2x15.3 / 3 m/s), mining (`mining_beam.rs:76-113`,
  health 90, cuboid 1, reach 100 / 1 s / 1.5 cells).
- Lint `crates/nova_scenario/src/lint/ship.rs`: all kinds :397-425 (adv in
  [0,1], health > 0, draw > 0); controller :427; torpedo :486 + reload :445;
  docking :588-680 (angle in [0,90), cuboid <= 1 cell per axis :654);
  intake :692-790 (aperture <= collider face :758-768, IntakeDoor track);
  mining :804-850 (StowDoors, StowLift tracks). Recipe lint :266-333.
- Ids: engine-visible `crates/nova_ship/src/sections/catalog_ids.rs:17-36`;
  authoring-local consts per builder file. Hardcoded consumers:
  `nova_assets/tests/section_load_gate.rs:299-378,583` (mining_beam_section),
  `examples/playable/station_docked_sandbox.rs:156,162` (cargo_intake_section),
  `base_content/ships/block.rs:39,151,207,816`.
- Art (`base_content/assets.rs:20-64,239-266`): one glb per section family;
  animation tracks bind glb node names (`intake_slat_`, `dock_tube`,
  `stow_lid_`, `beam_tip`), so a variant on the same glb keeps its tracks.

## Shipyard, swap, knowledge
- `ShipPartSnapshot` (`nova_world_base/src/ship_parts.rs:156-170`): families :67-80
  (Turret/Torpedo/Railgun = Weapon), `allowed_on` :113-121 (Mining only
  Industrial, Weapon only Scavenger/Armored). `open()` :504-512 keeps
  `advancement <= adv`; `known()` :526-557: adv-0 parts known by all, others
  per-civ per-id draw under `KNOWN_CHANCE = 0.5` (:51), `guarantee_one` per
  family. `station_carries` :654-664 (no thruster, intake, mining on stations).
- Station builds = `known(civ, adv)` (`station_layout.rs:485,552`), stock one
  recipe of every known part :513-538 (`StockOverHold` fault). Base waystation
  fixed list `base_content/stations/mod.rs:42-53`. Runtime `StationBuilds`
  (`nova_gameplay/src/markers.rs:198-210`). Lint refuses unknown ids
  (`lint/station.rs:148`).
- Swap (task 20261012-100014, ccba31595): `plan_section_swap`
  `design_sections.rs:226-251` (NotDocked, NoRefitService, NotBuilt, Family,
  payment). Full recipe price, no refund. Keeps section code, binding slot;
  fresh health; cargo in the root `ShipInventory` untouched (inference).
  Catalogue offers `nova_interface/src/ship/catalogue.rs:111-130,172-178`.
- Shipyard hulls `station_hulls.rs:139` via `generate_ship` at civ adv; test
  :176. Owner note 2026-10-16: which sections a station offers is opaque.
- Roles `nova_gameplay/src/market.rs:63-72`; role weights
  `civilizations.rs:183-197`; sector ships `sector_ships.rs:225-260`.
  Layout: intake :1564-1569, dock :1570, controller :1641-1646, mining :1757
  (chance 0.25..0.75 by adv, pair from 0.5 :144-149). Fixture ids :2601-2613,
  tier test :3215.

## Proof surfaces
- `nova_authoring/tests/generated_armament.rs`, `generated_stations.rs:49,195,264`,
  `content_ron_parity.rs:36,63`, `balance_audit_gate.rs`.
- Examples `system_station_swap.rs` (claims :23-25), `system_station_buy.rs`
  (:29-35), `system_station_add.rs` (:227-251), `system_station_rebuild.rs`,
  `system_generated_stations.rs`, `examples/playable/shipyard_sandbox.rs`.
- Wiki: `web/src/wiki/sections/*.md` hand-written catalogue rows
  (`catalog__id`), `web/src/wiki/sections.md`, `web/src/create/base-content.md:62-63`,
  `web/src/create/sections.md:1196,1263` ("One ships: cargo_intake_section").
- Recipes `base_content/sections/recipes.rs` (provisional :6): intake 60 cr,
  mining 100, controller 100, docking 80; parts must be category Parts items.

## Items
- `base_content/items.rs:46-264` 20 rows (id, name, about, category, mass_g,
  price_lot, ask_cr, bid_cr); no volume field. `ItemDesign`
  `nova_gameplay/src/inventory.rs:188-214` (deny_unknown_fields). Roles
  `ItemRoleType` :105-163 (10 roles, base must ship all). Categories
  `ItemCategoryType` :323-334 (Raw, Repair, Ammo, Food, Parts); exhaustive
  matches in `nova_interface/src/inventory/app.rs:89-93,343-347` and
  `icons.rs:725-803`. Test `items.rs:276` mirrors
  `nova_gameplay/src/test_support.rs` test_items.
- Consumers today: HullPlate `plan_plate_repair` :1550 (ship pane
  `ship/sections.rs:491,550`); ammo `sections/ammo.rs:425-476`; parts via
  `BuildRecipe` :1595 / `plan_recipe_payment` :1904 (refit `ship/refit.rs:295`,
  rebuild `ship/rebuild.rs:603`); Procure hand-in `interface/jobs.rs:2080-2094`
  (station receives nothing). Ore, Rations, SalvagedParts: trade only.
- Market `nova_gameplay/src/market.rs`: rules keyed by category (:191,:205);
  `price()` :421; `refusal` :636 (Enemy REFUSED badge). Base table
  `base_content/market.rs` (tests :186,191). Trade `plan_item_trade`
  `inventory.rs:964`.
- Station stock `station_layout.rs:465`: `GOODS` :114-121 (no ore), ammo by
  known reloads :485-491, parts kit :510-540, `SERVICES` :137-143 (Rebuild,
  Refit, Trade, Storage, Jobs). Restock `economy.rs:27` thaw only, target 0
  never moves. Ship holds `sector_ships.rs:464-525`.
- Trade profiles `nova_gameplay/src/trade_profile.rs` (`TradeIntent` :82,
  `VisitJobType::Procure` :99, faults :130, draw :215); base rows
  `base_content/trade_profiles.rs:66-120` (ore_hauler, provisioner,
  salvage_broker, garrison_supply, idle_traffic). Visits
  `station_visits.rs:198,581-660`.
- Jobs `nova_gameplay/src/jobs.rs:727` (Procure, Recover, Escort, Distress,
  Neutralize); offers :1476, wanted categories :1399-1429, hand-in :1935;
  Distress done on dock (`world_base/jobs.rs:820,1253`), needs no item.
- Stance `civilization.rs:154` (Ally/Neutral/Enemy); `Civilization`
  `civilizations.rs:153-172` has no trait field. Admission
  `docking_section/admission.rs`: `DockRefusalType` :30-41, `DockAdmission`
  :103 reads no inventory; `refusal()` :147 callers `connection.rs:377`,
  `hints.rs:249,392`, `flight/order.rs:730,743`, `map/chart.rs:393`,
  `station_visits.rs:209`.
- No refinery, crafting or conversion code. Shipyard hull buy is credits only
  (`hull_buy.rs:112`).
- Lint `nova_assets/src/items.rs:45,114`; recipe parts must be Parts
  (`lint/ship.rs:266-333`); station stock unknown item `lint/station.rs:181`.
- Docs: `web/src/create/mod-files.md:260-280`, `objects.md:296+`, `prices.md`,
  `trade-profiles.md:77-90`, `lore/economy.md`, `wiki/interface.md:258,290-410`.
  No wiki items page.
- Examples: `system_station_restock`, `_visits`, `_jobs`, `_storage`,
  `system_job_distress`, `system_civilization_stance`, `news_0160_market_inspector`.
