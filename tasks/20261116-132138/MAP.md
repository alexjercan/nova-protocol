# Map: SOS rescue

Scope: open world only. A void or campaign scenario has no generator, no
`OpenWorldSession` and no station chooser, so it shows no SOS.

## Owners

| Concern | Owner | File |
| --- | --- | --- |
| Pure rules: stranded verdict, fee, duration, bill | `nova_gameplay` | `src/inventory.rs` (beside `plan_recipe_payment`) or a new `src/rescue.rs` |
| Neutralize verdict, and its inverse for the player | `nova_gameplay` | `src/integrity/neutralize.rs` |
| Call state on the player root, its tick, cancel, end message | `nova_ship` | `src/sections/rescue_call.rs` (new, beside `repair_jobs.rs`) |
| Rebuild chain plan (port, thruster, computer) | `nova_ship` | `src/sections/design_sections.rs` (beside `lost_section_facts`) |
| Station chooser, bill, jump, berth and dock | `nova_world_base` | `src/rescue.rs` (new) + `src/start.rs` (berth made generic) |
| Section respawn | `nova_scenario` | `src/objects/spaceship.rs` `rebuild_design_section` (unchanged) |
| Ship pane Distress block | `nova_interface` | `src/ship/rescue.rs` (new) |
| Flight chip and notices | `nova_hud` | `src/flight_status.rs`, `src/hud_notice.rs` |
| Credit cause | `nova_events` | `src/lib.rs` `CreditChangeCauseType::Rescue` |

Why the split: `nova_hud` depends on `nova_ship` but not on
`nova_world_base` (`crates/nova_hud/Cargo.toml`), so the state the chip reads
must live in `nova_ship`. The chooser needs the generator, the stances and the
session, which only `nova_world_base` has. `nova_interface` depends on both.

## Before

```
damage -> detect_neutralized (Update, after IntegritySystems)
       -> try_insert(DefeatedMarker, NeutralizedMarker), fire OnDefeated + OnNeutralized
          (inserted once, never removed for a live hull)
player stranded: no SOS. Field rebuild (repair_jobs) can bring back a lost
anchored section from hold parts; nothing moves the ship to a station.
```

## After

```
Ship pane Distress block (undocked, stranded, world armed)
  "Call SOS  200 cr + DOCK-1 140 cr"  -> RescueCallCommand
  -> apply_rescue_call_command (nova_interface)
       rescue_station(...) (nova_world_base, pure)  -> target id, spawn
       rescue_rebuild_chain(design, live, needs)    -> section ids
       insert ShipRescueCall on the player root     (nothing spent)
advance_ship_rescue_call (nova_ship, Update, Time<Virtual>, RescueSystems)
  -> elapsed >= RESCUE_SECS -> RescueCallEnded { Answered } ; remove call
answer_rescue_calls (nova_world_base, exclusive, chain after Retire)
  -> take bill (hold parts first, credits saturate at 0, cause Rescue)
  -> trigger SectionRebuildRequest for each chain id (in order)
  -> wait until the rebuilt port is live (one or more frames)
  -> jump player (+ fleet by the same delta) to the target spawn, at rest
  -> insert StationBerth { station, purpose: Rescue }
berth_at_station (was berth_at_start, same chain slot)
  -> station stands -> berth pose on a live free player port -> DockingConnectionRequest
  -> docked: remove StationBerth, RescueCallEnded { Docked { station } }
restore_neutralized (nova_gameplay, player only)
  -> verdict no longer holds -> remove NeutralizedMarker (DefeatedMarker kept)
```

## Ordering

- `detect_neutralized`: `Update`, `.after(IntegritySystems)`
  (`crates/nova_gameplay/src/integrity/neutralize.rs:113`). The new
  `restore_neutralized` goes in the same slot, chained after it.
- `advance_ship_repair_jobs` is in `RepairJobSystems`, `Update`
  (`crates/nova_ship/src/sections/mod.rs:298-305`). `advance_ship_rescue_call`
  joins a new `RescueSystems` set beside it. Both read `Res<Time>` (virtual), so
  TAB and pause hold them.
- `berth_at_start` runs in the `nova_world_base` `Update` chain
  `.after(NovaWorldSystems::Retire).run_if(resource_exists::<WorldConfig<NovaLayeredWorld>>)`
  (`crates/nova_world_base/src/lib.rs:286-337`). `answer_rescue_calls` goes
  directly before `berth_at_station` in that chain. Both are exclusive.
- Save: `save_systems()` runs after `Retire` and after `track_role_goal`
  (`crates/nova_world_base/src/lib.rs:349-366`). A jump changes the cell, so a
  `SaveReason::Crossing` save follows (`crates/nova_world_base/src/save/session.rs:411-413`).

## IDs and content

- New enum value `CreditChangeCauseType::Rescue` (`crates/nova_events/src/lib.rs:969-1001`)
  and its `as_str`. Additive in the `OnCreditsChanged` payload, not breaking.
- No content, no RON, no runtime ID. Constants in Rust: `RESCUE_SECS`,
  `RESCUE_FEE_CR`. Suggest `RESCUE_FEE_CR = DISTRESS_BASE_CR` (200,
  `crates/nova_gameplay/src/jobs.rs:126`): a rescue costs what answering one pays.
- The target station is a `GeneratedBodyId` (`crates/nova_events/src/generated.rs:189-197`).

## UI

- Ship pane: a Distress block under the Repairs block. Closest pattern:
  `spawn_ship_repair_jobs` / `update_ship_repair_jobs`
  (`crates/nova_interface/src/ship/repair_jobs.rs:50,99`), registered in the
  second chain of `ShipPaneSystems` (`crates/nova_interface/src/ship/mod.rs:190-209`).
- HUD chip: `drive_repair_chip` (`crates/nova_hud/src/flight_status.rs:749-785`)
  shows the first repair job on the row at `REPAIR_CHIP_LIFT_PX` (`:102`).
- HUD notice: `post_repair_job_notice` (`crates/nova_hud/src/hud_notice.rs:284`)
  reads `RepairJobEnded`. A `post_rescue_notice` reads `RescueCallEnded`.

## Docs to touch

- `web/src/wiki/interface.md` `## The ship` (line 108): the Distress block.
- `web/src/wiki/hud.md:63,67`: the SOS chip and the rescue notices.
- `web/src/wiki/ships.md` `## Taking a ship apart` (line 27): neutralized
  clears for the player when the rule stops holding; SOS.
- `web/src/wiki/combat-weapons.md:192`: "out of the fight for good" is no
  longer true for the player.
- `web/src/create/events.md:878` (cause list adds `Rescue`), and the
  `OnDefeated`/`OnNeutralized` sections (lines 449-524): the player can fire
  them again after a recovery.
- `docs/sections.md` `### Repair jobs` (line 100) and the concept list
  (line 1971): the rescue call, its owner and proof.
- `CHANGELOG.md` `[Unreleased]`: Gameplay & Flight (SOS), Combat & Weapons
  (player neutralized clears on repair), Interface & HUD (block, chip,
  notices), Modding & Mod Portal (`Rescue` cause; repeat defeat events).

## Callers to update

- `StartBerthMarker` (`crates/nova_scenario/src/objects/start_role.rs:82-89`),
  inserted at `crates/nova_scenario/src/actions/spawn.rs:287`, read by
  `berth_at_start` (`crates/nova_world_base/src/start.rs:242-328`). Replace it
  with `StationBerth(GeneratedBodyId)`; delete the old marker.
- `outward_berth` (`crates/nova_ship/src/sections/hull_buy.rs:315-354`) takes
  every design docking port of the ship. It needs the ship's live free port
  indices too (see risk R6).
- `detect_neutralized` (`neutralize.rs:122-224`): extract the verdict into a
  pure function both passes share.

## Closest proof

- Unit, pure rules: `inventory.rs` tests near `change_credits_reports_each_real_move_once_and_skips_a_no_op`
  (`crates/nova_gameplay/src/inventory.rs:2390`).
- Unit, chooser: `the_start_is_the_living_ally_station_nearest_the_origin`
  (`crates/nova_world_base/src/start.rs:457`) on `tests::fixture_world`.
- ECS, neutralize: `neutralize_app` and its tests
  (`crates/nova_gameplay/src/integrity/neutralize.rs:283-700`).
- ECS, timed job and frozen clock: `plate_app`,
  `a_plate_job_does_not_progress_while_the_clocks_are_frozen`
  (`crates/nova_ship/src/sections/repair_jobs.rs:500,618`).
- ECS, rebuild spawn: `a_field_rebuild_restores_a_lost_docking_port_and_takes_the_recipe_parts`
  (`crates/nova_scenario/src/objects/spaceship.rs`).
- Asserted full-app flow: `examples/systems/system_start_roles.rs` (New Game,
  waits for the dock at the session station) and
  `examples/playable/shared/open_world_stage.rs` `advance_station_dock`
  (`:556-640`: undock, jump, wait for the window, berth until docked).
- Playable: `examples/playable/supply_sandbox.rs` (open world via
  `OpenWorldStartPlugin`) and `examples/playable/field_repair_sandbox.rs`
  (damage staging, pane clicks, smoke walk, `assert_at_rest`).

## Answers

### 1. Stranded rule

Claim: no predicate exists. Each fact is cheap to read off the root's children.

- Neutralized: `NeutralizedMarker` on the root. Armed rule: no working weapon,
  or a lost computer it had. Unarmed rule: lost its last thruster. Station rule:
  lost computer (`neutralize.rs:192-224`). It ignores docking ports and ammo.
- Working thruster / computer: `q_thruster`, `q_controller` with
  `Has<SectionInactiveMarker>` (`neutralize.rs:149-150,166-173`).
  `LiveFlightComputers` and `ship_has_attitude_authority`
  (`crates/nova_ship/src/flight/capability.rs:142-160`) are the flight-side reads.
- Free live port: `DockingSectionMarker`, `Without<DockedPort>`,
  `Without<SectionInactiveMarker>` (`DockAdmission::free_ports`,
  `crates/nova_ship/src/sections/docking_section/admission.rs:131-141`;
  `DockingPorts::has_free_port`, `port.rs:200-202`).
- Lost and rebuildable: `lost_section_facts` (`design_sections.rs:133-151`)
  gives `lost` and `anchored` per design section.
- Fuel: none exists. RCS is a recovering magazine (`flight/state.rs:260-268`).
- Ammo: a dry ship still flies; not stranded by this rule.

Proposed rule: player root, undocked, world armed, and at least one of:
neutralized, no working thruster, no working flight computer, no free live
docking port. Pure: `stranded_reasons(StrandedFacts) -> Vec<StrandedType>`.

Review decision: neutralized is not a reason. A ship that lost only its guns
flies and docks. The rule is no working thruster, no working flight computer
or no free live docking port, and no `StationBerth` under way.

### 2. Neutralized

Claim: removing the marker is safe for the player, but it comes straight back
while the rule still holds, and it fires `OnDefeated` + `OnNeutralized` again.

- Detection filters `Without<NeutralizedMarker>` (`neutralize.rs:137`) and
  re-runs every frame. `neutralize()` fires both events with no once-guard
  (`neutralize.rs:230-260`). `DefeatedMarker` is a `try_insert`; kept, it adds
  no new `Add<DefeatedMarker>`, so `kill_credit` does not count twice
  (`crates/nova_ship/src/kill_credit.rs:120-135`).
- Removal precedent: `rehull_ship` removes `NeutralizedMarker`,
  `DefeatedMarker` and the history stamps on the live player root
  (`crates/nova_scenario/src/objects/spaceship.rs:781-860`, removal `:824-831`).
- Consumers, and what a removal does for the player:
  - AI target pick drops neutralized contacts
    (`input/ai/acquisition.rs:200`; flag from `input/targeting/sensing.rs:177`).
    Removal makes the player a target again. Intended.
  - Brake: player excluded (`flight/neutralized_brake.rs:34-53`).
  - AI stand-down: AI only (`input/ai/mod.rs:337-345`); follow, maneuver,
    passive, threat filters are AI-side.
  - Fleet: `release_neutralized_escort` reads `PlayerFleetMarker` only
    (`crates/nova_scenario/src/objects/fleet.rs:257-271`).
  - Stance offence: hulls with `TraderMarket` and a player `LastAttacker`
    only (`crates/nova_world_base/src/stance.rs:207-236`).
  - Dock helm: a living station decides first (`connection.rs:943-975`,
    `helm_on_capture` `:130-140`); the player's own marker does not change it.
  - Admission: "a neutralized ship always admits" is target-side
    (`admission.rs:209-211`).
  - Station-side: PDC reserve, economy, jobs, station visits, pdc_reserve,
    HUD target inset and allegiance markers read it for other hulls.
  - Save: frozen with the root and thawed (`spaceship.rs:2051-2063`); probe
    snapshot reports it (`crates/nova_probe/src/capabilities/snapshot.rs:1110`).
  - Scenario: `OnNeutralized` lose handlers in tutorial and drills
    (`crates/nova_authoring/src/base_content/scenarios/drills.rs:194`,
    `tutorial/mod.rs:706`). Open world handles only `OnDestroyed`
    (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:109`).
- Finding (existing defect, verified by code read only): a neutralized player
  who field-rebuilds or station-rebuilds a gun or computer keeps the marker
  forever, so AI never targets the player again. Nothing outside tests and
  `rehull_ship` removes it (grep of `remove::<NeutralizedMarker`).

Proposed: keep the marker a live verdict for the player only. Extract
`neutralize_verdict(facts) -> bool`; add `restore_neutralized`
(`With<PlayerSpaceshipMarker>, With<NeutralizedMarker>`) that removes
`NeutralizedMarker` when the verdict is false. Keep `DefeatedMarker`. The
rescue then needs no special clear: it clears the frame the ship is fixed.

### 3. Placement at a station

- `berth_at_start` (`start.rs:242-328`) is the production path. Each frame the
  ship has `StartBerthMarker` and the station stands (Position, no
  `PendingParkedHulls`, parked hulls docked), it computes `clear_berth`
  (`:375-445`), writes `Position`, `Rotation`, `LinearVelocity`,
  `AngularVelocity` from the station, resets every `PointRotation` rig, and
  triggers `DockingConnectionRequest` in the same exclusive call. The marker
  comes off once `DockedShip` is on the ship.
- Same-frame request is safe: capture reads root `Position`/`Rotation` plus the
  local section chain (`port.rs:173-189`), not `GlobalTransform`.
- `restore_docking_connection` (`connection.rs:543-623`) rebuilds a saved
  joint from saved frames. Load and thaw only. Not for a live move.
- Examples: `open_world_stage::berth` / `berth_and_dock`
  (`examples/playable/shared/open_world_stage.rs:443-511`) and
  `station_void::berth_and_dock_at` (`station_void.rs:168-190`) copy the same
  pose rule (`parked_hull_pose`, `hull_buy.rs:223-236`).
- Needs: a live free player port, a free station port, the station live
  in the window, and admission (`DockAdmission::refusal`).

Proposed: generalize to `StationBerth(GeneratedBodyId)` and
`berth_at_station`. The start inserts it with the session's start id.

### 4. Dockless player

- Station Rebuild needs a dock: `StationSite::resolve` reads the partner from
  `DockedShip` (`crates/nova_interface/src/ship/rebuild.rs:206-241`), plans by
  `plan_section_rebuild` (`inventory.rs:2282-2318`) and pays by
  `plan_recipe_payment` (`:2185-2230`). It refuses `NotBuilt` when the station
  does not build the prototype. A generated station builds only the parts its
  civilization knows (`crates/nova_world_base/src/station_layout.rs:566-570`).
- Field rebuild already brings back an anchored lost port from hold parts
  (`crates/nova_ship/src/sections/repair_jobs.rs:17-22`).
- `rebuild_design_section` does not check anchoring; it panics only on a
  missing design id or a live section (`spaceship.rs:1337-1385`). So a chain
  of requests in design order works in one frame.
- Gap: an inactive non-leaf section (`SectionInactiveMarker`, still a child) is
  neither repairable (`PlateRepairRefusalType::Destroyed`, `inventory.rs:1798`)
  nor rebuildable (`SectionNotLost`). A ship whose only port is inactive has no
  rebuild path.

Proposed: the rescue rebuilds, before the jump, the shortest chain of lost
design sections that ends in a live port, and also in a thruster and a flight
computer when those are missing, through `SectionRebuildRequest`. The bill
prices each recipe. This keeps the ship flyable and dockable after the
rescue, whatever the station builds.

### 5. Nearest station

- Stations are sparse. One civilization per 240 km lattice node
  (`civilizations.rs:42`), 1 to 3 stations each (`sector_stations.rs:115-123`).
  The live window is 5x5x5 cells of 32 km (`lib.rs:197-209`), so the nearest
  station is often outside the window and not an entity.
- Live chooser (examples only): `open_world_stage::nearest_station`
  (`open_world_stage.rs:261-312`) queries live `StationMarker` roots.
- Pure chooser: `starting_station` (`start.rs:62-130`) walks the 3x3x3 civ
  block, keeps living Ally civs, claims nodes, poses each station
  (`clusters.rs:1507`, `station_pose`), and returns id, position and a spawn
  past its clearance. Copy this with the player's position as centre.
  `CivilizationField::in_reach(position)` (`civilizations.rs:399`) lists the
  civs in reach of a point.
- Stance: `CivilizationStances::effective(civ, seeded, now)`
  (`crates/nova_gameplay/src/civilization.rs:312`), `seeded_stance`
  (`stance.rs:58-62`). Enemy and provoked stations refuse
  (`admission.rs:82-100`). Accept Ally and Neutral.
- The chart has stations by id and civilization but no position
  (`crates/nova_gameplay/src/chart.rs:89-110`). It marks derelict stations.
- Travel: no floating origin and no travel concept beyond streaming. A
  `Position` write on the player moves the `WorldObserver`; the window retires
  and streams around the new cell. `advance_station_dock` waits for
  `window_is(cell)` and for the station to stand (`open_world_stage.rs:589-618`).
  `berth_at_start` already waits for the station entity.
- Orbiting stations: a station in a world cluster orbits
  (`clusters.rs:1494-1501`). The pure pose is the epoch pose. The berth reads
  the live station, so only the jump spawn uses the epoch pose.

### 6. Credits

- `ShipCredits(pub u32)` (`inventory.rs:715`). No debt. Every change goes
  through `ShipInventoryReports::change_credits` (`:893-909`), which reports
  `OnCreditsChanged` with a cause.
- Charge precedents: `GrantCredits` refuses a negative result
  (`crates/nova_scenario/src/actions/hold.rs:135-156`); Settle pays a fine to
  the docked station (`crates/nova_interface/src/station/settle.rs:130-140`).
  "A frozen station has no live balance to pay" (`settle.rs:8-9`).
- Rebuild price: recipe credits plus missing parts at
  `reference_price().buy_cr` (`inventory.rs:2185-2230`, `market.rs:347`).

Proposed: the bill is `RESCUE_FEE_CR` plus, for each chain section, its
recipe credits plus its missing parts at reference ask; hold parts first. Pay
`min(bill, balance)`; never refuse. No station receives it (the station may be
frozen when the bill is taken).

Review decision: the answer takes the bill, after it checks that the station
would admit the dock and before it rebuilds and moves the ship. A refusal
takes nothing; a berth that gives up leaves the paid, fixed ship beside the
station.

### 7. UI entry

- Ship pane is the natural owner: it already holds Repairs and the lost list.
  Closest pattern: the Repairs block (`ship/repair_jobs.rs:1-60`) and the
  Rebuild button flow (`ship/rebuild.rs:571-700`).
- The Map pane holds the chart and Settle; it reads stations, not the ship's
  state. Not the first choice.
- HUD: put the call on the repair chip row with priority (`SOS 40%`), or a new
  chip one row up. One-shot HUD notice on the stranded edge
  (`STRANDED: TAB > SHIP > SOS`), and on each end (`SOS ANSWERED`,
  `DOCKED AT STN-3: 340 CR`, `SOS: NO STATION ANSWERS`).
- Confirm: nothing is spent at the click, the row has Cancel, and the bill is
  on the button. That meets "cost shown before confirm" without a modal.

### 8. Saves

Owner decision: jobs are saved. This replaces the first reading below it.

- `FrozenShipState` keeps `repairs: Vec<SavedRepairJob>` and
  `rescue: Option<ShipRescueCall>` by design section id
  (`crates/nova_scenario/src/objects/spaceship.rs`). A Load resumes both.
- `check_saved_jobs` (`crates/nova_world_base/src/save/mod.rs`) refuses at
  open a job or chain section the design lacks. It runs on the read path
  only: the write path takes ids from live sections of the same design.
- The fields are `serde(default)`: a v0.18.0 save loads with no jobs and no
  call. Not a breaking change.
- `StationBerth` is not saved. The answer bills before the jump, so a save
  between the answer and the dock loads the ship paid, fixed and undocked
  beside the station; the player docks by hand.
- A save while neutralized already works (`save/session.rs:439-452`).

### 9. Tests and proof

- Unit: `stranded_reasons`; `plan_rescue_bill` (fee, hold first, saturate);
  `rescue_rebuild_chain` (direct anchored port; port behind a lost plate;
  no lost port -> `None`); `rescue_station` on `fixture_world` (living, not
  Enemy, nearest, seeded, excludes an id); `neutralize_verdict`.
- ECS (`neutralize_app`): a neutralized player who regains a weapon loses the
  marker; an NPC never does; `DefeatedMarker` stays.
- ECS (`plate_app` style): the call does not progress with the clocks frozen;
  ends once; cancel ends with nothing spent.
- Asserted example `examples/systems/system_sos_rescue.rs`: New Game seed,
  undock, disclosed fixture jump away and fixture damage (lose thrusters and
  port), call SOS, wait, assert docked at the chosen station, credits down by
  the bill, port and thruster live, `assert_at_rest`.
- Playable `examples/playable/sos_rescue_sandbox.rs` plus a README row.

### 10. Risks

- R1 Teleport: write `Position`, `Rotation`, `LinearVelocity`,
  `AngularVelocity`, never `Transform`, in an exclusive system; reset the
  `PointRotation` rigs (`start.rs:307-321`). A dock at a stale pose throws
  both hulls (`station_void.rs:192-197`).
- R2 Wait at spawn: while the window streams, the player stands at the spawn.
  In an orbit cluster the well pulls it. Hold it at rest each frame until the
  berth writes the pose.
- R3 Streaming: a jump of 100+ km retires and streams up to 125 cells, like a
  Load. Expect a hitch. The chooser lays out each candidate civ's station
  design (`StationClaims::station`); cost not measured (unverified).
- R4 f32: no floating origin. At 24 000 u (240 km) one ULP is about 2 mm; at
  400 000 u about 3 cm. The berth tolerance is 0.05 u (`open_world_stage.rs:497`);
  fine for a placed berth, but slow closes stall far out (known).
- R5 Station refuses or is gone: stance can change during the call; a station
  the player killed is dead in the ledger; all ports may be held.
  `berth_at_start` logs `error!` and leaves the ship undocked
  (`start.rs:283-290`). For a rescue this must be a notice and a fresh offer,
  with a frame budget so it never waits forever.
- R6 Berth port: `outward_berth` picks among all design ports, lost ones too
  (`hull_buy.rs:323-337`). Pass the live free port indices.
- R7 Reserved-berth check: `clear_berth` logs an error when the berth is not
  the one reserved for the start ship (`start.rs:424-441`). Gate it to the
  start purpose.
- R8 Helm: docked at a living station, the station holds the helm and `HELM`
  is refused (`connection.rs:130-145`). The player undocks with DOCK.
- R9 Fleet: escorts belong to the player record and are not retired by cells
  (`crates/nova_world/src/streaming.rs:1225-1256`). Left behind, they are
  100+ km away. Move them by the same delta.
- R10 Camera: the chase camera may ease across the jump (unverified).

## Recommended design

SOS is a timed rescue call on the game clock, offered in the Ship pane to an
undocked player in an armed open world who is neutralized, has no working
thruster, no working flight computer or no free live docking port. The button
shows the bill. Nothing is spent at the click. The call picks its station at
the click: the nearest living station whose civilization is Ally or Neutral to
the player, from the pure generator search round the player. Hostile stations
never answer. After `RESCUE_SECS` of game time (TAB and pause hold it) the
rescue takes the bill (fee plus the recipes of the sections it rebuilds; hold
parts first; credits saturate at zero; cause `Rescue`), rebuilds the shortest
lost chain to a live port, thruster and flight computer, jumps the player and
its fleet to the station's spawn at rest, and berths and docks it through the
generalized start berth. Neutralized becomes a live verdict for the player: it
clears the frame the rule stops holding, so a disarmed ship stays neutralized
until it rebuilds a gun, and any repair path (field, station, rescue) clears it.

```rust
// nova_gameplay
pub const RESCUE_SECS: f32 = 60.0;
pub const RESCUE_FEE_CR: u32 = 200;
pub struct StrandedFacts { pub neutralized: bool, pub working_thruster: bool,
    pub working_computer: bool, pub free_port: bool }
pub enum StrandedType { Neutralized, NoThruster, NoFlightComputer, NoDockingPort }
pub fn stranded_reasons(facts: StrandedFacts) -> Vec<StrandedType>;
pub struct RescueBill { pub from_hold: BTreeMap<ItemDesignId, u32>,
    pub owed_cr: u64, pub charged_cr: u32 }
pub fn plan_rescue_bill(items: &GameItems, recipes: &[&BuildRecipe],
    own: &ShipInventory, own_cr: u32) -> RescueBill;
pub struct NeutralizeFacts { /* was_armed, had_computer, had_thruster, station,
    working_weapon, working_computer, working_thruster, has_weapon_section */ }
pub fn neutralize_verdict(facts: NeutralizeFacts) -> bool;

// nova_ship
pub fn rescue_rebuild_chain(design: &ShipDesignSections,
    live: &[(&str, PlacedSectionLinkPoints<'_>)], needs: &[SectionClass])
    -> Option<Vec<String>>;
#[derive(Component)] pub struct ShipRescueCall { pub station: GeneratedBodyId,
    pub spawn: Meters3, pub chain: Vec<String>, pub elapsed_secs: f32 }
#[derive(EntityEvent)] pub struct RescueCallCancelRequest { pub entity: Entity }
#[derive(Message)] pub struct RescueCallEnded { pub ship: Entity, pub end: RescueEndType }
pub enum RescueEndType { Answered, Docked { station: GeneratedBodyId, paid_cr: u32 },
    Refused(DockRefusalType), NoBerth, Cancelled }
#[derive(SystemSet)] pub struct RescueSystems;
pub fn advance_ship_rescue_call(/* Time, Commands, Query<&mut ShipRescueCall>, MessageWriter */);

// nova_world_base
pub struct RescueStation { pub id: GeneratedBodyId, pub position: Meters3, pub spawn: Meters3 }
pub fn rescue_station(seed: u32, generator: &NovaLayeredWorld, stances: &CivilizationStances,
    now: f64, from: Meters3, exclude: &[GeneratedBodyId])
    -> Result<Option<RescueStation>, SectorFault>;
#[derive(Component)] pub struct StationBerth { pub station: GeneratedBodyId,
    pub purpose: StationBerthType }   // replaces StartBerthMarker
pub enum StationBerthType { Start, Rescue }
pub(crate) fn answer_rescue_calls(world: &mut World);
pub(crate) fn berth_at_station(world: &mut World);   // was berth_at_start

// nova_interface
#[derive(Message)] pub struct RescueCallCommand;
pub(crate) fn spawn_ship_rescue(panel: &mut ChildSpawnerCommands);
pub(crate) fn update_ship_rescue(/* ... */);
pub(crate) fn apply_rescue_call_commands(/* ... */);

// nova_events
CreditChangeCauseType::Rescue
```

## What dies

- `StartBerthMarker` and `berth_at_start` (replaced by `StationBerth` and
  `berth_at_station`).
- "Inserted once and never removed" on `NeutralizedMarker` for the player
  (doc comment `neutralize.rs:54-60`, module doc `:12-15`).

## What may break

- New Game docking: every start walk (`system_start_roles`, menu world-setup
  tests, `open_world_stage::advance_station_dock`) runs through the renamed
  berth, and `outward_berth` now skips held or inactive ship ports
  (`ship_taken`).
- Campaign drills and tutorial: a neutralized player that recovers fires
  `OnDefeated`/`OnNeutralized` again if neutralized again. Their lose handlers
  end the run first, so low risk; check `bug_neutralized_quiet` and
  `system_scenario_grammar` examples.
- AI: a recovered player is a target again. Today a repaired player is never
  targeted again (defect fixed by this change; say so in the changelog).
- Save and Load: a saved neutralized player whose rule no longer holds loses
  the marker on the first frame after the thaw. A save with a job or a call
  on a section the design lacks is refused at open.
- Examples that read the player's `NeutralizedMarker` as permanent
  (`examples/screenshots/loop_helm_orders.rs:255`).
- A field rebuild and the answer that end in one frame: the answer runs after
  `RepairJobSystems` and its sync point, so it sees the section live and skips
  it. No test covers that frame; the order is the guard.
- Mods: `OnRebuilt` from a rescue carries hold parts and a credit share, not
  `price_cr: 0`.

## Must fail loudly

- A chain id the design lacks: the existing `rebuild_design_section` panic.
- A rescue call on a root with no `ShipDesignSections`: panic, the pane only
  offers SOS to the player's designed ship.
- No station answers: refuse at the click with a pane note, never start a call
  with no target.
