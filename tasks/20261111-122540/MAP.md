# Scenario vocabulary for jobs, stance, credits: slice map (read-only, 2026-11-12, master 6cd1b7c01)

Paths are under `crates/` unless they start with `web/`, `docs/`, `examples/`,
`assets/` or `tasks/`. "Unverified" marks a claim with no line read.

## Spec corrections
1. "Completed" is the wrong word. `Jobs::complete` is the hand-in
   (`nova_gameplay/src/jobs.rs:1305-1309`); the work-done state is "done"
   (`JobDone` :1073-1077, `mark_done` :1250-1255). A Procure job has no done
   state before the hand-in (module doc :1-58). Works: `OnJobDone` and
   `OnJobHandedIn`. The "three events in order" probe needs a located kind
   (Recover, Escort, Distress, Neutralize), never a Procure.
2. No failed state exists. A held job never expires; it only drops
   (`Jobs::drop` :1320-1324). Drop sites: `drop_on_loss`
   (`nova_world_base/src/jobs.rs:1082-1150`, `jobs.drop` :1138, `warn!` only)
   and the player Drop button (`nova_interface/src/jobs.rs:2137-2160`, :2149).
   Works: `OnJobDropped` with `cause` = `Lost` or `Abandoned`.
3. Hand-in and drop are not in `nova_world_base`. They run in
   `apply_job_commands` (`nova_interface/src/jobs.rs:1962-2230`; hand-in
   :2048-2135, `jobs.complete` :2108). Neither raises anything today.
4. A handler cannot select a job by id. The payload `id` is generated text
   (`nova_events/src/lib.rs:1406-1413`). The Entity filter reads `data["id"]`
   (`nova_scenario/src/filters.rs:90-150`), but the lint refuses an id nothing
   spawns (`nova_scenario/src/lint/scenario.rs:2015-2023`). An Expression
   filter drops the payload (`filters.rs:303-306`). So `kind`, `reward_cr`,
   `cause`, stance `to` and the sector are write-only fields today. Works: one
   new factor `event.<field>` in the expression grammar, legal in filters.
   Without it the new events can only be counted, not told apart.
5. A civilization has no authored id. `CivilizationId { world_seed, node }` is
   seed-derived, "never a hash and never a name"
   (`nova_gameplay/src/civilization.rs:55-60`). "Unknown civilization id fails
   lint" cannot work. Authored stations have no civilization
   (`nova_world_base/src/stance.rs:28-29`). Works: address "the civilization
   of body X", where X is a payload binding (`$id`, `$other_id`) to a body
   with a `TraderMarket` (`nova_gameplay/src/market.rs:279-284`).
6. `SetStance` cannot run inside `nova_scenario`. `CivilizationStances` has no
   public set; `step` is private (`civilization.rs:318-332`). Every method
   needs the seeded stance, which is `seeded_stance` in `nova_world_base`
   (`stance.rs:56-60`), above `nova_scenario`. Works: the action raises a
   gameplay request Event; a `nova_world_base` observer applies a new
   `CivilizationStances::set` and keeps `validate` (:525).
7. No "board id" exists. A board is keyed by the station `GeneratedBodyId`
   (`jobs.rs:136-143,320-327`). An authored station never gets a board:
   `jobs_partner` needs a `GeneratedBodyId`
   (`nova_interface/src/jobs.rs:305-315`) and base waystations offer no Jobs
   (`nova_authoring/src/base_content/stations/mod.rs:82-88`). Works:
   `PostJob((station: "$other_id", ..))` on `OnDocked`, Procure only. Other
   kinds need generated target bodies (`spawn_job_bodies`,
   `nova_world_base/src/jobs.rs:637`) and a `target: GeneratedBodyId` no author
   can write (`lint/scenario.rs:1561-1572`).
8. `nova_mod_format` holds no lint (`nova_mod_format/src/lib.rs:1-17`). Id
   lint is `lint_scenario` (`nova_scenario/src/lint/scenario.rs:132-141`), and
   the same function gates the load (`loader/lifecycle.rs:240-272`).
9. No `Refused` outcome exists for actions. `EventAction::action` returns `()`
   (`nova_events/src/engine.rs:68-76`), effects are deferred
   (`nova_scenario/src/world.rs:586-591`), and "Outcome" means Victory or
   Defeat (`actions/flow.rs:13-18`). `ActionOutcomeType` is a pane notice
   (`nova_interface/src/pane.rs:766-771`). Works: `error!` plus one new event
   `OnActionRefused` (`action`, `id`, `reason`).
10. "Non-player target fails lint" is not needed for credits or items. Every
    spawned ship has a hold and a purse
    (`nova_scenario/src/objects/spaceship.rs:485-531`). The lint only needs
    "the id is a spawned ship", which `check_object_names` gives through
    `Names::Object` (`lint/scenario.rs:1523-1601`).
11. `OnSectorEntered` and the cell query exist only on a `Generated` world.
    Every shipped authored scenario is `Void`
    (`nova_scenario/src/loader/mod.rs:246-254`). `SectorCrossed` and
    `CurrentSector` live in `nova_world`, above `nova_scenario`
    (`nova_world/src/streaming.rs:137,147-153`), and the message is not
    written on the arming frame (:140-146). The bridge and the sampler must
    sit in `nova_world`.
12. `SpawnWreck` adds nothing over `SpawnScenarioObject(Spaceship)` with
    `controller: None, lootable: true`, a stock and credits
    (`objects/spaceship.rs:485-531`). No "drop list" type exists; canisters
    appear only on destruction (`nova_scenario/src/wreck_cargo.rs:1-34`). The
    ruined look is `generate_wreck` in `nova_world_base`
    (`ship_layout.rs:511-514`), out of reach. Works: drop the action; ship a
    docs recipe.
13. The sequence "events -> queries -> actions -> lint" splits the lint from
    its action. The stock-action test fails if a new `Names::Object` field has
    no lint (`nova_editor/src/event.rs:1998-2057`). Lint ships with each row.

## 1. Events end to end
- Declaration (`nova_events/src/lib.rs:1130-1133`):
  `#[derive(Debug, Clone, EventKind, Reflect)] #[event_name("oncreditschanged")]
  #[event_info(CreditsChangedEventInfo)] pub struct OnCreditsChangedEvent;`
  Info struct :1137-1157. Derive: `nova_events_macros/src/lib.rs:20-77`.
- Prelude list `nova_events/src/lib.rs:39-77`: add each event, info and const.
  Field-name consts :116-137, :880-898.
- Table row (`nova_scenario/src/events.rs:136-296`, macro :77-134):
  `OnJobAccepted => OnJobAcceptedEvent { label: "On Job Accepted", category: Trade }`
  (:274-277). The row gives `EventConfig`, `ALL`, `name()` (the RON tag),
  `label()`, `category()`. The row doc comment is the editor hint.
- Fire: `commands.fire::<E>(info)` (`nova_events/src/engine.rs:271-281`).
  Accept fires direct from the pane (`nova_interface/src/jobs.rs:2035-2040`).
  Hold events go through `ShipInventoryReports` and
  `track_ship_inventory_reports` (`nova_scenario/src/loader/trackers.rs:575`),
  registered in `FixedUpdate` after `SpaceshipSectionSystems`
  (`loader/mod.rs:826-832`). `docs/scenario-system.md:1018-1027` says a new
  occurrence below `nova_scenario` uses that queue.
- Files a new event touches: `nova_events/src/lib.rs` (struct, info, consts,
  prelude), `nova_scenario/src/events.rs` (row), the fire site,
  `web/src/create/events.md`, `web/src/create/reference.md`,
  `web/src/docs-manifest.js`, `CHANGELOG.md`. Table tests cover the row
  (`events.rs:298-351`). The editor palette needs no edit (section 7).

## 2. Job lifecycle
- States: offer -> held (`accept` `nova_gameplay/src/jobs.rs:1228`) -> done
  (`mark_done` :1250) -> completed (`complete` :1305); dropped (`drop` :1320).
  `Jobs { held, done, completed, legs, dropped }` :1135-1144.
- `Job { id, kind, target: GeneratedBodyId, reward_cr }` :803-815.
  `JobType` :727-781. `JobId { station, source }` :136-143, text :234-246.
- Done is raised only by world observers
  (`nova_world_base/src/jobs.rs:1196,1246,1307,1345`; registered :1623-1627).
  A Load restores done jobs without it (`nova_gameplay/src/jobs.rs:1073-1077`).
- Gameplay Events now: `JobAccepted { job }` :1062-1067, `JobDone { id }`,
  `DistressRefused` :1102-1108. Consumers: `nova_hud/src/hud_notice.rs:342,361`.
- Plug-in point: add `JobHandedIn { job }` and `JobDropped { job, cause }`
  beside `JobDone`. Trigger at `nova_interface/src/jobs.rs:2108`, :2149 and
  `nova_world_base/src/jobs.rs:1138`. One observer set in `nova_scenario`
  bridges `JobDone`, `JobHandedIn`, `JobDropped` to scenario events.
  `nova_scenario` already depends on `nova_gameplay`.
- Payload for all three: `id`, `kind`, `station`, `reward_cr`; dropped adds
  `cause`. `JobAcceptedEventInfo` should gain the same fields.

## 3. Stance and sector
- `StanceType { Ally, Neutral, Enemy }` `nova_gameplay/src/civilization.rs:154-163`.
  `StanceStep { civilization, from, to }` :249-256. Id text
  `civ_<x>_<y>_<z>@<seed>` :113-124.
- `pub fn stance(&self, civilization: &CivilizationId, seeded: StanceType) -> StanceType`
  :270; `effective(.., now: f64)` :278-283.
- Step sources return steps by value; no message exists. Sites:
  `record_offence` (`nova_world_base/src/stance.rs:204-231`, logs :227),
  `expire_stances` (:262-269, logs :267), merit
  (`nova_interface/src/jobs.rs:2119`), settle
  (`nova_interface/src/station/settle.rs:137-138`). `log_step` (:334) covers
  only the first two. Plug-in: a gameplay Event `StanceChanged(StanceStep)`
  raised at all four sites, bridged in `nova_scenario`.
- A provocation changes the effective stance with no step (`provoke`
  `civilization.rs:411`). `OnStanceChanged` will not fire for it.
- Sector: `CurrentSector(pub SectorCoord)` `nova_world/src/streaming.rs:137`;
  `SectorCrossed { from, to }` :147-153; writer `track_current_sector`
  :588-629. Cell text: `SectorCoord::slug()` `nova_events/src/generated.rs:95`
  (`sector_x_y_z`, `n` for minus). Existing reader:
  `nova_interface/src/sector_notice.rs:28,52`.
- Plug-in: a `nova_world` system reads `MessageReader<SectorCrossed>` and
  fires `OnSectorEnteredEvent` with `sector` (slug), `x`, `y`, `z`, `from`.

## 4. Credits, items, targets, refusals
- `ShipCredits(pub u32)` `nova_gameplay/src/inventory.rs:695`. All credit
  moves go through
  `change_credits(&mut self, credits: &mut ShipCredits, current_cr: u32, cause: CreditChangeCauseType)`
  :870-886. It queues the `OnCreditsChanged` report.
- `CreditChangeCauseType` `nova_events/src/lib.rs:935-965`, `as_str` :967-986.
  `GrantCredits` needs a new variant (`Scenario`). The serde-name test
  :1537-1555 lists 9 of 13 causes; add the new one there.
- `ShipInventory`: `free_g(&self, items: &GameItems) -> u32` :527;
  `count` :532; `add(&mut self, items: &GameItems, item: &ItemDesignId, count: u32)`
  :552-563 panics past `free_g`; `remove(&mut self, item, count)` :570-582
  panics past the held count. Check first. Precedent:
  `nova_console/src/cheats.rs:140-204` (items), :228-286 (credits, i128 bound).
- Target resolution: `scoped_ship(world: &mut World, id: &str) -> Option<Entity>`
  `nova_scenario/src/actions/mod.rs:115`. No `player_ship` helper exists. The
  open world names its player `"player"` through `SpawnStartRole`
  (`nova_authoring/src/base_content/scenarios/open_world.rs:29,36-37`).
- Refusals today are `warn!` or `error!` only (`actions/ship.rs:522-560`).

## 5. Actions end to end
- Row (`nova_scenario/src/actions/mod.rs:475-482`):
  `SetTradeProfile(SetTradeProfileActionConfig) { label: "Set Trade Profile",
  category: Trade, stem: "trade_profile", effect: Injection, inspect: Reflect }`.
  Macro `actions/registry.rs:77-221` generates `EventActionConfig`, dispatch
  and `ActionTag`.
- Payload model: `SetTradeProfileActionConfig { #[reflect(@Names::Object)]
  ship: String, profile: Option<TradeProfileDesignId> }` `actions/ship.rs:508-520`;
  impl :522-560 (`push_command`, resolve, catalog check, `error!`).
- Trait: `fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo)`.
  World access is `world.push_command(|commands| commands.queue(|world: &mut World| ..))`.
- Payload binding: `bind_object_reference(reference: &str, info: &GameEventInfo) -> Option<String>`
  `actions/mod.rs:171`; `generated_entity(world, id: &GeneratedBodyId)` :136.
  Model: `ObjectiveMarkerAttach` `actions/mission.rs:297-341`. The lint allows
  a binding only on the two marker actions (`lint/scenario.rs:1533-1548`);
  `PostJob` and `SetStance` must join that list.
- Lint: `check_action` `lint/scenario.rs:1057-1066`; SetTradeProfile arm
  :1353-1367. `Catalog` :43-51 already holds `items` and `ships`. With the
  designs above no new `lint_scenario` parameter is needed. A new parameter
  would touch seven callers: `loader/lifecycle.rs:257`,
  `nova_editor/src/problems.rs:414`, `project/package.rs:196`, `event.rs:2035`,
  `nova_assets/src/merge.rs:690`, `nova_wfc/src/check.rs:240`,
  `nova_authoring/src/lint_walk.rs:351`.
- New lint rule: a `Void` scenario that uses a job, stance or sector event,
  `PostJob` or `SetStance` is an error (`loader/mod.rs:246-254`).
- `PostJob` needs: a new `JobSourceType` variant (`jobs.rs:147-168`; `Slot`
  and `Visit` only), because `post_visit` asserts a Visit source with a
  `StationVisitId` (:377-395). `Job::validate` holds a visit job to Procure
  and `target == id.station` (:955-1014). `JobPostings` is saved (section
  Risks).
- Order: action effects are deferred. A query read in the same handler sees
  the old sample (`loader/clock.rs:37-64`).

## 6. Queries
- `QueryConfig { Scenario(ScenarioQuery), Entity(EntityQuery) }`, derives
  `Clone, Debug, PartialEq, Eq, Hash` (`nova_scenario/src/queries.rs:14-71`).
  `EntityQuery { filter: EntityQueryFilter { id: String }, property: EntityProperty }`
  :40-53; `EntityProperty::Speed` :58-61.
- Values are sampled into `query_values: HashMap<QueryConfig, VariableLiteral>`
  (`world.rs:209`), read by
  `pub fn query_value(&self, query: &QueryConfig) -> Option<&VariableLiteral>`
  :1175. `VariableLiteral { String, Number(f64), Boolean }` `variables.rs:40-47`.
- Sampler: `sample_scenario_queries(entities: Query<(&EntityId, &LinearVelocity), ScenarioAddressable>, mut world: ResMut<NovaEventWorld>)`
  `loader/clock.rs:37-64`; chain :93-112. Credits and item count fit here as
  `EntityProperty::Credits` and `EntityProperty::Items(ItemDesignId)`
  (`ItemDesignId` is `Eq + Hash`, `nova_gameplay/src/inventory.rs:70-73`).
- Grammar is closed: `named` accepts only `scenario` and `entity`
  (`syntax.rs:365-402`); `Display` :66-80. Each query needs both.
- Wake profile: `clock_watches` and `sampled_watches`
  (`loader/wake.rs:122-145`) and `reads_clock` (:234-243). A query not
  classified there makes `OnUpdate` handlers miss changes or run every frame.
- Lint: `check_query` `lint/scenario.rs:2104-2124`; watch lint :306-327.
- Cell query: the value is in `nova_world`. `NovaEventWorld` has no public
  setter for a query value. Add one, and sample from a `nova_world` system
  ordered before `fire_on_update` (`loader/clock.rs:93-112`).
- Stance query: no subject can be written (correction 5). Defer it, or read
  `event.to` in the `OnStanceChanged` handler (correction 4).

## 7. Editor palette
- `BlockItemType::all()` builds the palette from `EventConfig::ALL`,
  `ActionChoice::ALL`, `FilterChoice::ALL`, `ExprChoice::ALL`
  (`nova_editor/src/event/blocks.rs:170-178`). A new event row is a Hat with
  no editor edit. Category comes from the row (:211-217); hint is the row doc
  comment (:254-265). `BlockCategoryType` has Trade, World and Mission
  (`nova_scenario/src/events.rs:23-40`); no new category is needed.
- `pub(crate) type ActionChoice = ActionTag;` `nova_editor/src/event.rs:568`.
  `fn stock(self) -> ActionKind` :587-880 is an exhaustive match. Each new
  action needs an arm with empty ids, or the crate does not compile.
- Id pickers are free for typed ids: `CatalogType::TYPED` maps `ShipDesignId`,
  `ItemDesignId`, `TradeProfileDesignId` (`inspect.rs:1261-1267`). Object ids
  come from `Names` (`nova_scenario/src/names.rs:55-83`, `inspect.rs:3777-3783`).
- One Query block exists, hardcoded to Elapsed (`blocks.rs:161,189,261,868`).
  New queries need a choice list there.
- Tests: `the_palette_covers_every_kind_in_the_tables`
  (`event/tests.rs:1221-1241`), `every_shipped_scenario_lifts_and_lowers_unchanged`
  (:90-100), `every_object_an_action_names_is_resolved_by_the_lint`
  (`event.rs:1998-2057`), `ui/canvas/tests.rs:303`.

## 8. Content and proof
- Catalog `nova_authoring/src/base_content/scenarios/mod.rs:30-45`. Only
  `open_world` runs on a `Generated` world (`open_world.rs:57-60`). It holds
  the one `OnJobAccepted` handler (:93-106).
- Append new handlers at the end: a save names spent `once` handlers by index
  (`open_world.rs:93-94`; `nova_scenario/src/resume.rs:49-52`).
- Beat: `OnJobDone` posts "hand it in"; `OnJobHandedIn` completes it and runs
  `GrantCredits`. A Procure job skips `OnJobDone`, so the hand-in handler must
  not depend on the done handler.
- Gen and lint: `content gen`, `content lint` (`nova_authoring/src/cli.rs:60-64,93-94`);
  walk `lint_walk.rs:351`. Gates: `tests/content_ron_parity.rs:1-17`,
  `tests/content_lint_gate.rs:9-30`.
- Balance audit covers combat openings only (`tests/balance_audit_gate.rs:1-10`).
  A credit grant is not audited (unverified beyond that header).
- `probe scenario` proves a boot, not a goal (`.claude/skills/nova-probe/SKILL.md`).
  The chain needs an asserted example. Closest: `examples/systems/system_job_recover.rs:1-28,64-75`
  on `examples/systems/shared/located_jobs.rs` (board, accept, work, hand-in).
  Assert handler-written variables with `scenario_variable_is`
  (`nova_debug/src/harness.rs:235`); model `examples/systems/system_hold_events.rs:463,595`.
- New `outcome:` slugs go in `SYSTEMS_ROSTER`
  (`nova_probe_cli/tests/catalog_drift.rs:137`); `SYSTEMS_INVARIANTS` is 637
  (:1283). Examples register in `Cargo.toml` (:787, :985).
- Other ranges: `system_station_jobs.rs`, `system_job_distress.rs`,
  `system_civilization_stance.rs`, `system_world_sectors.rs` under
  `examples/systems/`.
- Mod proof: `webmods/the-ledger/` and `nova_authoring/tests/ledger_campaign.rs`.
  New variants do not break it; a new lint on existing content would.

## 9. Docs
- Nothing is generated. `docs/scenario-system.md:254-258` names `/create/` as
  the one catalog; `web/src/docs-manifest.js:1-9` is a hand-kept page list.
- `web/src/create/events.md`: payload table (:54, :65), one section per event
  (:911), cause list (:834), dispatch order (:937).
- `web/src/create/actions.md`: index row (:53), section (:1457-1478).
- `web/src/create/expressions.md`: queries (:105-125), typed forms (:176-177).
- `web/src/create/reference.md`: family rows with hand counts, "Events (29)"
  :44 against 45 table rows, "Actions (55)" :46; index entries :131, :188.
- `web/src/docs-manifest.js`: `headings` lists (:844, :920).
- `web/src/create/stations.md:623` (job events), `docs/scenario-system.md:1009,1016-1036`,
  `assets/mods/example/README.md:45`, `CHANGELOG.md`.
- Design prior art: `tasks/20260824-125938/RESEARCH.md:316-338`.

## Recommended slice order
Each step compiles and ships its lint, stock value, docs row and test.
1. Payload factor `event.<field>` in filters: grammar (`syntax.rs`), factor
   node (`variables.rs:60`), evaluation with `info` (`filters.rs:303-306`),
   wake class, lint, editor reporter, `expressions.md`.
2. Job events: gameplay `JobHandedIn`, `JobDropped`; scenario `OnJobDone`,
   `OnJobHandedIn`, `OnJobDropped`; richer payload on `OnJobAccepted`; the
   `Void` lint rule.
3. `OnStanceChanged` (`StanceChanged` at four sites) and `OnSectorEntered`
   (bridge in `nova_world`).
4. Queries: `EntityProperty::Credits`, `Items(..)`; cell query with the public
   sampler setter; grammar, wake class, editor query list.
5. `OnActionRefused`, `CreditChangeCauseType::Scenario`, `GrantCredits`.
6. `GiveItems`, `TakeItems`.
7. `SetStance`: `CivilizationStances::set`, request Event, binding lint.
8. `PostJob`: new `JobSourceType` variant, Procure only, board UI check.
9. `open_world` job-chain beat (appended handlers), `content gen`, the
   asserted example and its roster rows.
10. Changelog pass and count fixes in `reference.md`.

## Risks
- World save: `Jobs`, `JobPostings`, `CivilizationStances` are saved
  (`nova_world_base/src/save/mod.rs:309-321`) with `deny_unknown_fields`
  (`nova_gameplay/src/jobs.rs:137,253,298,321`). A new `JobSourceType`
  variant is additive for old saves, but `WORLD_SAVE_VERSION` is `v0.17.0`
  (`save/mod.rs:100`) and another layout is refused, not migrated (:1827-1836).
  Decide whether step 8 bumps it.
- Scenario save: `spent_handlers` are indexes (`resume.rs:49-52`). A handler
  inserted before the end of `open_world` corrupts shipped saves.
  **(breaking)** if done; append only.
- `JobAcceptedEventInfo` gains fields: additive for mods. Renaming
  `OnJobAccepted` or its `id` would be **(breaking)**; do not.
- A new `CreditChangeCauseType` variant is a new `cause` string for shipped
  mods that match on it; additive.
- The `Void` lint rule is an error at load. A shipped mod that already uses
  `OnJobAccepted` on a `Void` world would stop loading. No file under
  `webmods/` or `assets/mods/` names `OnJobAccepted` (grep, this date), so
  repo content is safe. **(breaking)** for outside mods if an error.
- `GrantCredits` below zero and `GiveItems` past the hold must refuse before
  the mutation; `add` and `remove` panic (`inventory.rs:552-582`).
- Wake profile: an unclassified query makes handlers stale with no error
  (`loader/wake.rs:112-145`). Prove it with an `OnUpdate` handler test.
- `SetStance` skips the offence and merit bookkeeping. `validate`
  (`civilization.rs:525`) must pass after a set; `enemy_since` and the outlaw
  count need defined values (:318-321).
- Station bodies carrying `TraderMarket` is inferred from
  `sync_civilization_sides` (`nova_world_base/src/stance.rs:274-284`) and
  `nova_world/src/generation.rs:149,197`. Unverified for stations; confirm
  before step 7.
