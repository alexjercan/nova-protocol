# Crafting slice 2: consumables, code map

Read-only map at `7e53f8721` (2026-11-17). No build was run. Line numbers are
from that tree. Design and recommendations: `DESIGN-2.html`. Slice-1 map:
`MAP.md`.

## End state (slice 2, recommended)

- A new content kind `Consumable` makes an item usable. It names the item and
  one effect from a closed enum: `Repair`, `ThrustBoost`, `Flare`.
- Three new items, each with one Ship recipe: `RepairKit`, `BoostCell`,
  `DecoyFlare`. Stations stock them; combat start kits carry some.
- One flight action per effect type: keys `1`, `2`, `3`. The Inventory pane
  gets a `Use` verb on own rows. Timed effects run on the game clock and are
  saved.

## Owner and entry points today

### Items, recipes, market

| Concern | Owner | Entry |
|---|---|---|
| Item design | `nova_gameplay` | `ItemDesign` `crates/nova_gameplay/src/inventory.rs:198-224` (id, name, about, category, mass_g, price_lot, ask_cr, bid_cr; `deny_unknown_fields`). No use or effect field. |
| Category enum | `nova_gameplay` | `ItemCategoryType` `inventory.rs:332-351` (Raw, Repair, Ammo, Food, Parts, Contraband). Doc `:332-333` "There is no fuel category: thrust never consumes stock" dies with a boost item. |
| Engine-named items | `nova_gameplay` | `ItemRoleType` `inventory.rs:105-186` (11 ids, fixed category `:175-185`). |
| Item catalog resource | `nova_gameplay` | `GameItems` `inventory.rs:238-290`; `market_deals_in` `:363-365`. |
| Item builders | `nova_authoring` | `crates/nova_authoring/src/base_content/items.rs:32-55` (id consts), `item_catalog()` `:58-305`, fixture parity test `:318` against `crates/nova_gameplay/src/test_support.rs:79`. |
| Item lint | `nova_assets` | `crates/nova_assets/src/items.rs` `item_pack_faults` `:60-182`, `item_faults` `:185-227`. |
| Recipe kind | `nova_gameplay` | `crates/nova_gameplay/src/crafting.rs`: `RecipeDesignId` `:41-63`, `CraftSiteType` `:65-96`, `RecipeDesign` `:98-115`, `GameRecipes` `:129-165`, `recipe_faults` `:169-173`, shape `:180-223`, value (no-loss price, mass) `:230-320`. |
| Recipe builders | `nova_authoring` | `crates/nova_authoring/src/base_content/recipes.rs:22-87`; doc `:5-7` "No recipe makes a new item" dies. Generated `assets/base/recipes/base.content.ron:1-114`. |
| Content kind enum | `nova_modding` | `Content` `crates/nova_modding/src/lib.rs:84-189` (`Recipe` `:182-188` is the pattern). Wiring a kind: merge router `crates/nova_assets/src/merge.rs` (`GameRecipes` insert `:795`), `crates/nova_authoring/src/generation.rs:184-193,281`, `lint_walk.rs`, editor `project.rs:81`, `asset/blank.rs:121`, `asset/verbs.rs:61`. |
| Catalog digest | `nova_assets` | `content_catalog_digest` `merge.rs:980-1020` hashes every content item, so a new kind changes the digest and refuses older saves at the header. |
| Market rules | `nova_gameplay` | `crates/nova_gameplay/src/market.rs:22-25` (by category only), `price()` `:421-470`; base table `crates/nova_authoring/src/base_content/market.rs:14-140`. |
| Station stock | `nova_world_base` | `GOODS` `crates/nova_world_base/src/station_layout.rs:110-122` (role ids only: PdcRound, RailSlug, Torpedo, HullPlate, Medkit, Rations, SalvagedParts); ammo filter and stock loop `:500-528`; parts kit `:532-557`. No raw ore and no recipe-only output is stocked. |
| Start kits | `nova_authoring` | `assets/base/roles/base.content.ron:8-125`; builder `crates/nova_authoring/src/base_content/start_roles.rs:150-176` takes `ItemRoleType` stock only (unverified line span; agent read). |

### Repair and rebuild (Repair kit hooks)

| Concern | Entry |
|---|---|
| Plate constants | `HULL_PLATE_HEALTH = 20.0` `inventory.rs:1563`; `PLATE_REPAIR_SECS = 4.0` `:1568`; field rebuild `:1572-1584`. |
| Plate rule | `plan_plate_repair` `inventory.rs:1613-1652`; `plan_plate_step` `:1659-1665`. |
| Plate tick (health write) | `advance_ship_repair_jobs` `crates/nova_ship/src/sections/repair_jobs.rs:453-596`; section query skips a section not a child, disabled, or at 0 HP `:486-488`; `health.current = current` `:498`. |
| Free hold | `ShipRepairJobs::free_hold` `repair_jobs.rs:203-230`. |
| Field rebuild | `RebuildSiteType` `inventory.rs:2053-2069`; `plan_section_rebuild` `:2071-2120`; `plan_field_parts` `:2122-2142`. |
| Section destroyed at 0 HP | `crates/nova_gameplay/src/integrity/core.rs:371-381`; `crates/nova_ship/src/sections/integrity.rs:407-427` (task 20261116-132137). |
| Dock rebuild parts | docking port build: DockingCollar 1, ActuatorPack 1, SectionPanel 2, 80 cr (`assets/base/sections/base.content.ron`, builder `crates/nova_authoring/src/base_content/sections/recipes.rs:20-189`). Only SectionPanel and AlloyPlate have recipes today. |

### Flight (Boost hooks)

| Concern | Entry |
|---|---|
| Thrust impulse | `thruster_impulse_system` `crates/nova_ship/src/sections/thruster_section.rs:573-624`; impulse `**magnitude * input.clamp(0.0, 1.0)` `:620`. |
| Magnitude | `ThrusterSectionMagnitude` `thruster_section.rs:481-501` (impulse per fixed tick); `ThrusterSectionInput` `:503-506`. |
| Manual burn | `manual_burn_system` `crates/nova_ship/src/flight/manual.rs:47-208`; skips `Autopilot` ships `:61`; `demand = burn * authority` `:181`. |
| Burn input | `FlightIntent` `crates/nova_ship/src/flight/state.rs:36-42`; `on_flight_burn_input` `crates/nova_ship/src/input/player/flight_rig.rs:401-422`. |
| Readers of magnitude | autopilot planning `crates/nova_ship/src/flight/autopilot.rs:499`; signature `crates/nova_ship/src/sections/signature.rs:111-140`. A factor at `:620` leaves both unchanged. |
| Speed cap | none for the player (`crates/nova_scenario/src/objects/spaceship.rs:68-71`). |
| Plume | `thruster_shader_update_system` `thruster_section.rs:640-702`; input clamped to 0..1, 16 buckets `:315-328`. A boost does not brighten the plume. |

### Torpedoes, PDC, AI (Flare hooks)

| Concern | Entry |
|---|---|
| Torpedo target | `TorpedoTargetEntity` `crates/nova_ship/src/sections/torpedo_section/mod.rs:723-727`; `TorpedoTargetChosen` `:729-737` (doc "keeps its first target for life" dies); `TorpedoTargetPosition` `:742-746`. |
| Target gone fallback | `update_target_position` `torpedo_section/projectile.rs:12-56`: drops the entity link, flies to the last position `:25-38`. |
| Fuze | `torpedo_detonate_system` `projectile.rs:230+`; with no target entity the reach is `blast.radius * 0.5` from the position `:294-299`. |
| Blast | `blast_radius: 300.0` m, `blast_damage: 750.0` `assets/base/sections/base.content.ron:2396-2397,2606-2607`; converted `bay.rs:380`. |
| Torpedo save | `SavedTorpedoTarget::Frozen(Vec3)` `torpedo_section/frozen.rs:89-93` already saves a torpedo with no target and a fixed point. A flared torpedo needs no new save state. |
| AI launch commit | `update_torpedo_target_input` `crates/nova_ship/src/input/ai/torpedo.rs:215-245`. |
| Player launch commit | `crates/nova_ship/src/input/player/intent.rs:220-254`. |
| PDC point defense | `update_turret_point_defense` `crates/nova_ship/src/input/point_defense/assignment.rs:175-330`; only torpedoes qualify `:205`; range `AI_POINT_DEFENSE_RANGE` 150 u `crates/nova_ship/src/input/ai/acquisition.rs:237-259`. Runs on the player ship too. |
| AI target | `AITarget` `acquisition.rs:18-25`; `pick_ai_target` `:105-156`; `update_ai_target` `:158-227` (filter `:199-211`). |
| Sensors | `crates/nova_ship/src/input/targeting/sensing.rs`: ranges `:28-48`, `class_range` `:202-213`, `update_sensor_contacts` `:215-330`. `LockSignature` `targeting/state.rs:9-22`. Stealth term note `signature.rs:71-73`. |
| AI torpedo carriers | generated Scavenger and Armored ships take any known weapon family part (`crates/nova_world_base/src/ship_parts.rs:102-116`) and carry torpedoes in the hold (`sector_ships.rs:514-528`). How often an AI fires at the player: unmeasured. |
| Threat cues | `play_threat_lock_cue` `crates/nova_ship/src/ship_audio/cues.rs:307`; `play_torpedo_closing_tone` `:124`. |

### Input

| Concern | Entry |
|---|---|
| Flight actions | `flight_bindings` `crates/nova_ship/src/input/bindings.rs:30-130` (W, Space, Q, E, X, G, O, Z, D, H, Ctrl, `[`, `]`, Shift). |
| Section defaults | `section_default_bindings` `bindings.rs:193-221` (Space, LMB, F, R, mining V/B/N/C `:177`). |
| Reserved sources | `flight_rig_reserved_sources` `bindings.rs:152-171`: a new flight action reserves its key against content section mappings at lint. |
| Always keys | Tab `crates/nova_interface/src/bindings.rs:50-53`; backquote `crates/nova_hud/src/lib.rs:431-438`; Enter `crates/nova_scenario/src/loader/lifecycle.rs:709-723`; Esc `crates/nova_menu/src/pause.rs:89-90`. |
| Free in flight | digits 0-9, T, Y, U, I, J, K, L, P, F2, F4-F10 and others (agent grep). No base content maps a digit (`grep Digit assets/` is empty). Digit keycap glyphs exist `crates/nova_hud/src/key_glyphs.rs:87-97`. |
| Gamepad | no free pad button in flight (`bindings.rs:36-39`). |
| Persistence | `PersistedSettings.keybinds` `crates/nova_menu/src/settings_store.rs:119`; applied `:532-560`. |
| Registry | `InputBindings` `crates/nova_input/src/registry.rs:488`, `register` `:511`. |

### HUD, audio, VFX

| Concern | Entry |
|---|---|
| Status chip | `drive_repair_chip` `crates/nova_hud/src/flight_status.rs:745-822`: SOS, else first repair job, else craft (`:779`, craft only when no repair job runs). |
| Notice | `HudNotice(String)` `crates/nova_hud/src/hud_notice.rs:93-96`; one slot, 4 s `:73`; `show_hud_notice` `:601-645`. |
| Keybind dock | `DOCK_VERBS: [&str; 15]` `crates/nova_hud/src/keybind_dock.rs:141-157`; hints `crates/nova_ship/src/input/player/hints.rs:25-71`. |
| Ammo readout | `crates/nova_hud/src/ammo_readout.rs` (combat only `:250`). |
| Ship cues | `crates/nova_ship/src/ship_audio/cues.rs:39` (`play_cockpit`); `ShipFeedbackSounds` `ship_audio/feedback.rs:32-77`. |
| UI sounds | `UiSfx` `crates/nova_sound/src/lib.rs:143-203`. |
| Stand-in sounds | `assets/base/sounds/`: `torpedo_ignite.wav` (boost), `cargo_eject.wav` or `torpedo_launch.wav` (flare), `dock_clamp.wav` or `salvage_pickup.wav` (kit). No dedicated asset exists. |
| Puff VFX | torpedo launch puff `crates/nova_ship/src/sections/torpedo_section/render.rs:785-874`; dud puff `death.rs:239-277` (gated on `GraphicsBudget::particles`, `death.rs:7`). |

### Running state and save

| Concern | Entry |
|---|---|
| Timer primitive | `Cooldown` `crates/nova_gameplay/src/cooldown.rs:55-120` (serde). |
| Timed precedent | `ShipRescueCall` `crates/nova_ship/src/sections/rescue_call.rs:40-105`; `ShipCraftJob` `crates/nova_ship/src/sections/craft_job.rs:51-90`. |
| Frozen ship | `FrozenShipState` `crates/nova_scenario/src/objects/spaceship.rs:650-678` (`repairs` `:673`, `rescue` `:675`, `craft` `:677`, all required). |
| Freeze, thaw | `freeze_ship` `spaceship.rs:980`, jobs `:1054-1065`; thaw `:1197-1230`; restore `:2146-2163`. |
| Hull swap strips | `spaceship.rs:880-887` removes `ShipRepairJobs`, `ShipRescueCall`, `ShipCraftJob`. |
| Save check | `check_saved_jobs` `crates/nova_world_base/src/save/mod.rs:1185-1326` (called `:671`). |
| Version | `WORLD_SAVE_VERSION = "v0.18.0"` `save/mod.rs:108`. |

### Inventory pane

| Concern | Entry |
|---|---|
| Verbs | `InventoryActionType` `crates/nova_interface/src/inventory/mod.rs:219-237` (Take, Give, Buy, Sell, Jettison, Store, Withdraw). No Use. |
| Row offers | `offered_action` `crates/nova_interface/src/inventory/app.rs:1645-1688`; `own_row_actions` `:1605-1633`; `action_chip` `:1228`; apply `:2413`. |
| Craft button | filter row `app.rs:416-419`; `spawn_craft_open_button` `crates/nova_interface/src/craft.rs:459-475`. |
| Craft view | `crates/nova_interface/src/craft.rs` (`player_site` `:441-456`, start `:1255-1300`). |

### Sections (fabricator, sections as items)

| Concern | Entry |
|---|---|
| Section kind | `SectionKind` `crates/nova_ship/src/sections/base_section.rs:458-485` (9 kinds); mirror `SectionClass` `crates/nova_gameplay/src/damage.rs:115-146`. About 40 match sites across nova_ship, nova_scenario, nova_world_base, nova_editor, nova_interface (agent grep). |
| Lost and rebuilt | `ShipDesignSections` `crates/nova_ship/src/sections/design_sections.rs:39-150`; respawn `rebuild_design_section` `crates/nova_scenario/src/objects/spaceship.rs:1396-1460`. |
| Add, swap, hull buy | `plan_section_add` `crates/nova_ship/src/sections/section_add.rs:374-440`; `plan_section_swap` `design_sections.rs:309-364`; `plan_hull_buy` `hull_buy.rs:168-197`; payment `plan_recipe_payment` `inventory.rs:1980-2033`. |

## Data flow after (recommended)

```
key 1/2/3 (Flight action "use_repair_kit"/"use_boost"/"use_flare")
  or Inventory own row -> Use chip
  -> ConsumableUseRequest { ship, effect type }
  -> plan_consumable_use(consumable, facts) (nova_gameplay, pure)
       Err -> HudNotice "NO REPAIR KIT" / "NOTHING TO REPAIR" / "BOOST RUNNING" /
              "BOOST: UNDOCK FIRST" / "NO TORPEDO ON YOU"; nothing taken
       Ok  -> remove 1 item from ShipInventory
              Repair, ThrustBoost: push RunningConsumable { item, elapsed_secs: 0 }
              Flare: re-aim tracking torpedoes now (no running state)
Update, ConsumableSystems (after CraftJobSystems), Res<Time>:
  Repair: health/s to the live damaged section with the lowest health share
  ThrustBoost: factor read by thruster_impulse_system (FixedUpdate) while no
               autopilot verb flies
  elapsed >= duration -> remove the entry
Freeze/save -> FrozenShipState.effects: Vec<RunningConsumable>
```

## Ordering

- Use requests: `Update`, after the flight input rig fires and before
  `ConsumableSystems` tick. TAB holds `Time<Virtual>`; a Use from the pane
  starts the effect, which advances only after the pane closes.
- The thrust factor is read in `FixedUpdate` by `thruster_impulse_system`.
  It reads the root component; no ordering edge beyond component presence.
- Flare re-aim must run before `update_target_position` in the same frame,
  or that system rewrites `TorpedoTargetPosition` from the live target.
  State it as `.before(update_target_position)` (or the torpedo set).

## IDs

- `ConsumableType` closed enum (Repair, ThrustBoost, Flare) in
  `nova_gameplay` (`crafting.rs` neighbour, new `consumable.rs`).
- Keyed by `ItemDesignId` (one consumable per item). No new design id.
- New item ids `RepairKit`, `BoostCell`, `DecoyFlare` (base only, not
  `ItemRoleType`).
- New recipe ids `base/repair_kit`, `base/boost_cell`, `base/decoy_flare`;
  optional `base/docking_collar`, `base/actuator_pack`.
- New flight action labels `use_repair_kit`, `use_boost`, `use_flare`.

## Callers that break (compiler and content find them)

- `Content` exhaustive matches (same list as `MAP.md` "Callers that break").
- `FrozenShipState` literals in save tests (new required field).
- `DOCK_VERBS` array length (`keybind_dock.rs:141`).
- Generated station stock tests (`station_layout.rs` tests, `nova_authoring`
  generated_stations) when consumables join the stock.
- Start kit builder signature if kits carry non-role items.
- `catalog_drift.rs` rows for new examples.

## Docs (ship with the code)

- New wiki page `web/src/wiki/crafting.md` ("Crafting and consumables": the
  loop), registered in `web/src/docs-manifest.js` (pattern `:354-366`).
- `web/src/wiki/keybinds.md` (three rows), `web/src/wiki/hud.md` (dock chips,
  status chip), `web/src/wiki/interface.md:432-461` (Use verb, link to the
  new page), `web/src/wiki/combat-weapons.md:165` (flares vs torpedoes).
- `web/src/create/mod-files.md` (Consumable kind), example mod
  `assets/mods/example/example.content.ron` (one consumable).
- `CHANGELOG.md` `[Unreleased]`: Gameplay, Modding, Interface.

## Closest proof today

- Recipe lint tests in `crafting.rs`; craft job ECS tests in `craft_job.rs`.
- Plate job ECS tests `repair_jobs.rs:633-879`.
- Torpedo commit tests `crates/nova_ship/src/input/ai/torpedo.rs:248+`.
- Save round trip `spaceship.rs:2644-2770`.
- Examples `system_ship_craft`, `craft_sandbox`, `field_repair_sandbox`,
  `ammo_warning_sandbox`.
