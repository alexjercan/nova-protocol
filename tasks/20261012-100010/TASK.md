# Wizard: seed then start role; six roles as content

- STATUS: OPEN
- PRIORITY: 78
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decisions 5 and 5b (2026-10-12): the New World modal becomes a wizard: seed, then a start role; six roles, each resolving to a small base hull plus a kit (credits, plates, ammo, goods) at inner-ring advancement; relations start the same for every role; the Line Warship is no longer the start.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- The modal pre-fills a seed and a name (`crates/nova_menu/src/menu_ui.rs:111-115`, `crates/nova_menu/src/world_setup.rs:53`, `:98-103`, `:324-370`); the bootstrap scenario spawns the Line Warship with `credits: 2000` (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:38-100`). Base hulls at `crates/nova_authoring/src/base_content/ships/mod.rs:31-56`.
- Worker survey 2026-10-14 (old block ships, before deletion). The generator NPC and raider pools, wrecks and derelicts, the station shipyard catalogue and hire offers are all `generate_ship`/`generate_wreck` output (`crates/nova_world_base/src/ship_layout.rs`, `station_hulls.rs`) and name no base ship id. Station designs (ring, spire, yard, waystation) reuse no old ship; the waystation keeps the block builder and the `bridge`/`port_collar` section ids. Consumers per old id (runtime-ID strings and constants; released news and old task records are history and stay):
  - `block_cutter`: menu backdrop orbiter and weave runner (`main_menu/shared.rs`, `weave.rs`), webmod gauntlet course ship, `built_in_ships`, examples `first_shift_map`, `first_shift_ships`, `screenshot_diagonal_skin`, `screenshot_solar_wing`, `lesson_combat_field`, `shared/drydock`, `shared/ring`, `system_hud_shell`, docs `docs/sections.md`, web `create/ships.md`, `widgets.ts`, wiki `thruster.md`. New: `mining_tug`.
  - `block_hauler`: menu backdrop cargo orbiter (`shared.rs`, `menu_waystation`), bench `range`, `arsenal`, `built_in_ships`, examples `ui_app_variants`, `screenshot_goto_moving_target`, `shared/drydock`, docs, web. New: `light_freighter`.
  - `block_workship`: season one Kaveri (`season_one/stage.rs`, tests), `system_station_add` base hull, web, wiki. New: `mining_tug`.
  - `block_frame_tender`: season one tests (intact vs damaged collar), example mod scenario, `loop_docked_helm`, web. New: `light_freighter`.
  - `block_frame_tender_damaged`: season one Gantry, bench `docking_warship_tender`, `lesson_cargo`, web. New: `freighter_derelict`.
  - `block_gunship`: menu duel victor, menu gauntlet stand (six turret magazines), bench `range`, `hunt`, `slingshot`, `built_in_ships`, test fixture strings in `nova_command`, `nova_console`, `nova_editor`, `nova_assets/merge.rs`, `nova_scenario/lint/ship.rs`, 20 screenshot and system examples (`shared/hollow`, `shared/kit`, `shared/drydock`, `lesson_*`, `loop_*`, `screenshot_*`, `system_helm_orders`), web `objects.md`, `widgets.ts`, `widgets.test.ts`, wiki `controller.md`, docs. New: `interceptor`; the gauntlet stand takes `line_warship` (it needs PDC cover on both hemispheres).
  - `block_line_warship` (`BLOCK_LINE_WARSHIP_SHIP_ID` in `nova_world_base`): open world player, `docked_helm` and inventory tests, bench `docking_warship_tender`, example mod scenario, examples `loop_docked_helm`, `loop_predicted_path`, `screenshot_goto_moving_target`, `lesson_cargo`; mining examples `mine_and_sell`, `screenshot_mining_beam`, `lesson_mining`; open-world examples and probes through the start spawn. New: the open world spawns the chosen role's hull; `line_warship` (new design, military goal) for the warship scenes; `mining_tug` for the mining scenes.
  - `block_picket`: tutorial and four drills trainer (`tutorial/range.rs`, every section patched), bench `arsenal`, fixture strings in `nova_command`, `nova_console`, examples `first_shift_ships`, `ui_app_variants`, `system_ai_combat`, `system_ai_evade`, `system_ai_patrol`, web. New: `patrol_picket`.
  - `block_wreck_plate`: season one debris (`stage.rs`, tests), web. New: `debris_plate`.
  - `inline_raider` (uncatalogued `salvage_raider` hull): menu duel rival. New: inline copy of `raider`.
- New ship list (all block-built on the current sections, derived skin; every crewed hull has a docking port named `port_collar` and a flight computer named `bridge`):
  - `mining_tug` Mining Tug, industrial start: bow mining beam, dorsal cargo intake, flank dock, unarmed.
  - `salvage_hauler` Salvage Hauler, scavenger start: big cargo-hull hold, cargo intake, dock, one PDC.
  - `light_freighter` Light Freighter, transport start: cargo and tank hull body, dock, unarmed.
  - `interceptor` Interceptor, bounty hunter start: two PDCs and one torpedo bay, dock.
  - `raider` Raider, pirate start: two PDCs and a nose railgun, dock.
  - `patrol_picket` Patrol Picket, military start (weakest): one nose PDC on light plate, dock.
  - `line_warship` Line Warship, the military goal class: six PDCs, a railgun, two torpedo bays, two docks, cargo intake.
  - `freighter_derelict` Light Freighter: Derelict: the freighter with its drive and stern gone, dock kept in place.
  - `debris_plate` Debris Plate: loose plating with nothing that works.

## Owner decisions
- 2026-10-14: the six roles are approved (industrial, scavenger, transport, bounty hunter, military, pirate), and so are the eight design choices that were proposed (role as content with id, name, about, hull, kit, lore and no side; kit = credits plus item stacks checked by the ship stock lint; bindings derived from sections; session resource plus spawn action, player id stays `player`; role in the save header and on the Load screen; two-step wizard; example mod adds a role; goals later).
- 2026-10-14: do not add docking variants of the old hulls. Delete the old block-built ships. They were made before the current sections (docking collar, mining, truss, solar, link-point skin) existed. Make new ships, one start hull per role that makes sense for that role, and migrate every consumer to the new ships.
- 2026-10-14: no backwards compatibility. Nothing is released and old saves do not matter, so a save format change needs no migration and no compatibility path.
- 2026-10-14 (owner addition): the main menu Load panel looks wrong, because its Load and Delete buttons are full width. Restyle it after the Mods panel.

- Owner decisions 2026-10-14 (after the roles landed):
  - The patrol_picket (military start) is worth too little at 3,860 cr. Grow it to about 10,000 cr of sections. It stays the lightest armed start.
  - Every open-world ship has a cargo intake section, so it can jettison and pick up items. This covers the six role hulls, every hull the shipyard sells and every escort for hire. Lint refuses an open-world ship design without one. Campaign and scenario ships are exempt.

## Delivery gate
- Propose `StartRoleDesignId`, `StartRole { id, name, about, hull: ShipDesignId, kit, lore }` as content; the harness spawns the role's hull; the save header records the role. Stop for the owner on the six hull and kit assignments.

## Verification
- Each role start spawns, passes post-build checks and survives a save round trip (asserted example per role); lint refuses a role naming an unknown hull.

## Done when
- The wizard ships with six roles as base content and the example mod can add one.
