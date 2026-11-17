# Crafting slice 1: feature map

Read-only map at `589f7d66a` (2026-11-16). No build was run. Line numbers are
from that tree. Design and recommendations: `DESIGN.html`.

## End state (slice 1)

- One content kind `Recipe` replaces `ItemDesign::refine`. A recipe names its
  inputs, one output stack, a per-batch duration and a site: `Ship` (any
  player hull, anywhere) or `Station((fee_cr))` (docked at a living station
  that offers `Craft`).
- A craft is a timed job of N batches on the gameplay clock, one per ship,
  saved and resumed. Inputs and fee are taken per finished batch.
- A new Craft pane in the TAB interface. The Inventory pane loses Refine.

## Owner and entry points today

| Concern | Owner | Entry |
|---|---|---|
| Item catalog and refine recipe type | `nova_gameplay` | `ItemRefineRecipe` `crates/nova_gameplay/src/inventory.rs:198-234`; `ItemDesign::refine` `:264-270`; `GameItems::refined_from` `:338-351` |
| Refine planning rule | `nova_gameplay` | `ItemRefine` `inventory.rs:1126-1143`; `ItemRefineRefusalType` `:1145-1180`; `plan_item_refine` `:1182-1263`; prelude exports `:46-61` |
| Refine lint (pack level) | `nova_assets` | `item_pack_faults` refine block `crates/nova_assets/src/items.rs:124-162`; `refine_price_fault` (lot-exact no-loss rule) `:171-217`; per-item refine faults `:256-276`; tests `:288-571` |
| Item builders | `nova_authoring` | `crates/nova_authoring/src/base_content/items.rs:54-56` (ALLOY_PLATE), `:290-326` (`refine: refine_recipe(id)`, fee 30), fixture parity test `:328-361` |
| Generated items | content | `assets/base/items/base.content.ron:213-233` (AlloyPlate `refine`) |
| Test item fixture | `nova_gameplay` | `crates/nova_gameplay/src/test_support.rs:90,158` |
| Content kind enum | `nova_modding` | `Content` `crates/nova_modding/src/lib.rs:84-178`; `kind()`/`id()` below it |
| Merge router and merged-catalog lint | `nova_assets` | items `crates/nova_assets/src/merge.rs:186-193`, router `:1207` (Item), `:1224` (TradeProfile); trade-profile merged lint `:655-688` (pattern for recipes) |
| Lint walk | `nova_authoring` | `crates/nova_authoring/src/lint_walk.rs:143-173` (collect kinds), `:466-467` (trade profile faults), `:1128` (item pack faults) |
| Base content builder list | `nova_authoring` | `crates/nova_authoring/src/base_content/mod.rs:28-57` |
| Station service gate | `nova_gameplay` | `StationServiceType::Refine` `crates/nova_gameplay/src/markers.rs:182-200`; `station_offers` `:237-243` |
| Generated station gate | `nova_world_base` | `REFINE_ADVANCEMENT = 0.25` `crates/nova_world_base/src/station_layout.rs:138-146`; service chain `:566-569`; test `:2227-2272`; re-export `crates/nova_world_base/src/lib.rs:130,154` |
| Authored waystation | `nova_authoring` | `crates/nova_authoring/src/base_content/stations/mod.rs:67,88` -> `assets/base/stations/base.content.ron:5,358` |
| Credit cause | `nova_events` | `CreditChangeCauseType::Refine` `crates/nova_events/src/lib.rs:994-996,1021,1805`; doc `crates/nova_scenario/src/events.rs:269-273` |
| Inventory pane Refine | `nova_interface` | doc `crates/nova_interface/src/inventory/mod.rs:23-27,45-47`; `InventoryActionType::Refine` `mod.rs:223-311`; app `crates/nova_interface/src/inventory/app.rs:1436-1449,1585-1630,1690-1721,1957-1978,2182-2216,2426-2459,2914-3040,3477,3665-3669,3781-3848`; tests `inventory/tests.rs:1435-1500+` |
| Section catalogue (UI model) | `nova_interface` + `nova_ship` | `crates/nova_interface/src/ship/catalogue.rs:1-25` (host over Ship pane, muted tile with reason, focus card verb); `crates/nova_ship/src/gallery/mod.rs:1-12,55-86` (tile grid caps, category row, filter, focus turntable, 3D tiles via own camera) |
| Interface panes | `nova_interface` | `InterfacePaneType` `crates/nova_interface/src/pane.rs:62-110` (ALL, context_id, next); pane bindings `crates/nova_interface/src/bindings.rs:123-158` |
| Timed jobs (model) | `nova_ship` | `crates/nova_ship/src/sections/repair_jobs.rs:1-38` (rules), `:71-110` (types), `:182-233` (`push`, `free_hold`), `:235-360` (`SavedRepairJob`), `:364-413` (cancel request, ended message, set), `:453-597` (tick on `Res<Time>` in Update), `:600-623` (cancel observer); wiring `crates/nova_ship/src/sections/mod.rs:299-320` |
| Job UI (model) | `nova_interface`, `nova_hud` | `crates/nova_interface/src/ship/repair_jobs.rs:1-30` (rows, bar, Cancel); HUD chip `crates/nova_hud/src/flight_status.rs:144-147,745-790`; HUD notice `crates/nova_hud/src/hud_notice.rs:20-27,296-336` |
| Job save | `nova_scenario`, `nova_world_base` | `FrozenShip` state `crates/nova_scenario/src/objects/spaceship.rs:670-685` (serde default for v0.18.0); freeze `:1050-1061`; parked hull drops jobs `:775-780`; thaw `:2142-2156`; round-trip test `:2644-2770`; save check `crates/nova_world_base/src/save/mod.rs:1168-1260`; policy `:18-27,95-105` |
| Saved services | `nova_scenario`, `nova_gameplay` | `FrozenStation::services` `crates/nova_scenario/src/objects/station.rs:551,568`; `StationSummary::services` (chart) `crates/nova_gameplay/src/chart.rs:82` |
| Docking state | `nova_ship` | `DockingConnection` `crates/nova_ship/src/sections/docking_section/connection.rs:154`; `DockedShip` `:200`; dock-end pattern `crates/nova_ship/src/sections/rescue_call.rs:343-356` |

## Data flow today (Refine)

```
row click (player item that a recipe consumes)
  -> InventoryActionType::Refine chip (app.rs:1449 refined_from)
  -> form: batches, refine_batch_text (app.rs:2917)
  -> Confirm -> InventoryActionCommand
  -> apply_inventory_action_commands -> refine_items (app.rs:3011)
       -> plan_item_refine (inventory.rs:1194): instant, inputs + fee at once
       -> CreditsChanged{cause: Refine} on ship and station
```

## Data flow after (slice 1)

```
Craft pane (TAB, any time)
  tile click -> focus card: inputs have/need, fee, secs/batch, batches, Craft
  Craft -> plan_craft_start(recipe, batches, site facts, free hold, credits)
        -> insert ShipCraftJob { recipe, queued, done: 0, elapsed_secs: 0 }
Update, CraftJobSystems (after RepairJobSystems), Res<Time> (Tab holds it)
  advance_ship_craft_jobs: elapsed += dt; each full duration_secs:
     plan_craft_batch(recipe, site facts, hold, credits)
       Ok  -> remove inputs, add output, fee ship->station (cause Craft)
       Err -> end CraftStopped(reason); finished batches stay
  done == queued -> end Crafted
  CraftJobEnded message -> HUD notice, pane note
Cancel -> CraftJobCancelRequest observer -> end Cancelled (nothing more taken)
Freeze/save -> FrozenShip.craft: Option<ShipCraftJob>; Load resumes it
```

## Ordering

- Tick: `Update`, `Res<Time>` (virtual), same as `advance_ship_repair_jobs`
  (`repair_jobs.rs:453`). `CraftJobSystems.after(RepairJobSystems)`: both
  write `ShipInventory`; state the order rather than leave it ambiguous.
- Dock and undock run in `FixedUpdate`; the tick reads `DockedShip` and the
  partner's `StationServices` each frame, so a station job ends on the first
  `Update` after the undock.
- Plugin: `nova_ship` (owns docking and jobs). Pure rule in `nova_gameplay`.

## IDs

- New authored catalog ID `RecipeDesignId` (string, namespaced like trade
  profiles: `base/pdc_round`). Lowest shared crate: `nova_gameplay`
  (interface, ship, scenario, assets, world_base all depend on it).
- New closed enum `CraftSiteType { Ship, Station { fee_cr } }`.
- Rename `StationServiceType::Refine` -> `Craft`,
  `CreditChangeCauseType::Refine` -> `Craft`,
  `REFINE_ADVANCEMENT` -> `CRAFT_ADVANCEMENT` (0.25 kept).
- Runtime-ID consumers of the old names: listed in "Callers" below.

## Content format and builders

- New `Content::Recipe(RecipeDesign)` (`nova_modding/src/lib.rs:84`), router
  arm in `merge.rs` beside `TradeProfile` (`:1224`), `GameRecipes` resource
  keyed by id. Overlay: a later pack's id replaces the whole recipe, like a
  trade profile.
- New builder `crates/nova_authoring/src/base_content/recipes.rs`, listed in
  `base_content/mod.rs:28-57`; generated file
  `assets/base/recipes/base.content.ron`, listed in
  `assets/base/base.bundle.ron` after `items/` (reading order only).
- Delete `refine` from `ItemDesign`, its builder (`items.rs:305,311-326`)
  and the fixture row field (`test_support.rs:90,158`). The AlloyPlate
  `about` text ("refined from iron and carbon ore at a station") stays true:
  it becomes a station recipe.

## Lint

- `recipe_faults(recipe, &GameItems) -> Vec<String>` in `nova_gameplay`
  (shared by merge and lint walk, as `trade_profile_faults`,
  `crates/nova_gameplay/src/trade_profile.rs:130`).
- Move the lot-exact no-loss arithmetic from `refine_price_fault`
  (`nova_assets/src/items.rs:179-217`) into it; delete the refine block from
  `item_pack_faults` (`items.rs:124-162,256-276`) and its tests (`:472-571`).
- Rules: empty id; no inputs; input count 0; input listed twice; input is the
  output; unknown input or output item; output count 0; `duration_secs` not
  finite or not above 0; inputs at ask (+ fee) below output at bid; output
  mass above input mass.

## UI

- New `InterfacePaneType::Craft` (`pane.rs:62-110`): title-row button,
  context `craft`, step order Map -> Ship -> Inventory -> Craft -> Jobs ->
  Station.
- New module `crates/nova_interface/src/craft/` (pane body, refresh, focus
  card, job row). 2D tiles with category icons and colors
  (`crate::icons::InterfaceIcons`, `inventory::category_color`), not the 3D
  gallery scene: items have no meshes.
- Delete from the Inventory pane: Refine chip, Refine action, `refines`
  term, refine double-click, `refine_*` helpers, refine tests (see table).
- HUD: `hud_notice.rs` posts the craft end; the flight-status chip shows the
  craft job when no rescue or repair job runs.

## Docs (ship with the code)

- `web/src/create/mod-files.md:261,286-308` (refine field -> Recipe kind).
- `web/src/create/stations.md:72,83-84,159-177,459` (Refine -> Craft).
- `web/src/create/objects.md:339-342,385`.
- `web/src/create/events.md:883,920-921` (cause `Refine` -> `Craft`).
- `web/src/wiki/interface.md:344-345,416-432` (Refine -> Craft pane).
- `assets/mods/example/example.content.ron:875-895` (add a `Recipe` after
  the item), `:1580` (service list gains `Craft`).
- `CHANGELOG.md` `[Unreleased]`: entries under Gameplay, Modding (breaking),
  Interface. Do not edit `web/src/news/0.18.0.md` (history).

## Callers that break (compiler and content find them)

- `ItemDesign { refine: None, .. }` literals:
  `crates/nova_world_base/src/station_layout.rs:1849,1950,2060,2125,2187,2245`,
  `crates/nova_ship/src/sections/hull_buy.rs:479,490`,
  `crates/nova_gameplay/src/jobs.rs:2762,4440`,
  `crates/nova_interface/src/inventory/tests.rs:354`,
  `crates/nova_gameplay/src/test_support.rs:90`.
- `StationServiceType::Refine`: `station_layout.rs:568,2270`,
  `base_content/stations/mod.rs:88`, `examples/playable/refine_sandbox.rs:282`,
  `examples/systems/system_station_refine.rs:402`, inventory pane.
- `Content` exhaustive matches: `nova_modding/src/lib.rs` (`kind`, `id`),
  `nova_assets/src/merge.rs`, `nova_authoring/src/lint_walk.rs:157`,
  `nova_editor/src/inspect/asset.rs:206,223`, `nova_editor/src/asset/verbs.rs:58,102`,
  `nova_editor/src/project/package.rs:238-252`, `nova_editor/src/asset/blank.rs:117-120`,
  `nova_editor/src/ui/list_field.rs:121`.
- Examples: delete `refine_sandbox` and `system_station_refine`
  (`Cargo.toml:159-164,990-996`), and their rows in
  `crates/nova_probe_cli/tests/catalog_drift.rs:1043-1046` and
  `examples/playable/README.md`.

## Closest proof today

- Refine planner: unit tests beside `plan_item_refine` (`inventory.rs`).
- Refine lint: `nova_assets/src/items.rs:472-571`.
- Pane: `crates/nova_interface/src/inventory/tests.rs:1435-1500+`.
- Walk: `examples/systems/system_station_refine.rs` (ECS asserted, autopilot).
- Timed job + save: `repair_jobs.rs:633-879`,
  `spaceship.rs:2644-2770`, `save/tests.rs:5112-5170`.
- Playable: `examples/playable/field_repair_sandbox.rs` (closest pattern).

## Measured inputs (from task 20261116-132139)

- A fight spends 582-1971 PDC rounds; a station sells 100 for about 11 cr
  (`tasks/20261116-132139/MEASURE.md`, run 1 and 3).
- PDC fires 100 rounds/s; magazine 500; reload 200 per 3 s.
- Hull free mass at a hand-in: 1316 kg (picket, run 1).
- Mining beam on start hulls: the miner tug and the scavenger hauler only,
  by the role `about` texts (`assets/base/roles/base.content.ron:6,28`). The
  combat roles (bounty_hunter, military, pirate) mine nothing; they buy ore
  or loot it.
