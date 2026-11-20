# Mining: canisters drift to the open intake

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.19.0, mining

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "mining is a bit tedious and hard but maybe it's fine for now". Asks for
  ideas. Lower priority than survival, crafting and jobs.

## Decisions
- Owner 2026-11-16: mining itself is fun. The tedious part is picking up
  canisters. Canisters gravitate toward an open pickup point (cargo intake).
  This is the scope; the other ideas below stay candidates.

## Agent findings
- Beam: reach 100 m, pulse every 1.0 s, carve radius 1.5 cells; long beam
  160 m, 0.8 s (`assets/base/sections/base.content.ron:4223-4225,4360-4362`).
  Stats are provisional
  (`crates/nova_authoring/src/base_content/sections/mining_beam.rs:22`).
- Waits in the loop (`crates/nova_scenario/src/mining.rs:31-42,108`):
  the first pulse on a new rock only seeds the field and yields nothing; ore
  is held until the remesh lands; canisters drop one at a time and drift at
  0.2 u/s; the intake must come within 1 m (door opens at 40 m,
  `base.content.ron:3563`).
- Ore bids per 10 kg: Stone 3, Carbon 6, Ice 9, Iron 12.
- Owner-approved canister rules: 1000 kg cap, mixed-item canisters, partial
  pickup (`tasks/20261026-003724/TASK.md:10`).

## Candidate ideas (agent, not approved)
- The first pulse yields ore.
- A short tractor pull on canisters inside the intake door range.
- A scan pulse that shows rich veins, so the player picks a target.
- Ore worth more, or rich veins with rare ore.
- Mining feeds on-ship crafting (task 20261116-132140).

## Delivery
1. While the intake door is open, canisters in a defined range drift toward
   the intake. Range and pull speed are explicit content fields on the
   intake section, not hidden constants.
2. Keep the owner-approved canister rules (cap, mixed items, partial
   pickup).
3. Measure pickup time and cr per minute before and after on the
   `mine_and_sell` example.

## Verification
- ECS asserted example: a canister inside range reaches an open intake and
  is taken; with the door closed it does not move.

## Done when
- Canisters near an open intake come to it, and the measured pickup time
  per canister drops.

## Map and proposal (2026-11-21)
Agent findings, not approved. Code at `6845da7f6`. No production code changed.

### Map
- Canister body: `cargo_canister()` (`crates/nova_ship/src/sections/cargo_intake_section.rs:247-287`):
  `RigidBody::Dynamic`, `SleepingDisabled` (:265), cuboid 9.4 x 5.8 x 5.8 m (:80),
  `ColliderDensity(1.0)` (:267), `Health` 20, `LinearVelocity`. The body mass is fixed by the
  collider; the cargo mass does not change it. `thaw_canister` builds the same body (:364-395).
- Spawners: mining `eject_mined_canisters` (`crates/nova_scenario/src/mining.rs:1101-1171`), on
  `Update` in `MiningSystems` (:1218-1224), at rock point velocity + 0.2 u/s (2 m/s) off the cut
  (:108, :1156); jettison in `run_cargo_intakes` (:674-710) at `eject_speed` 3 m/s; wreck drops
  (`crates/nova_scenario/src/wreck_cargo.rs`).
- Motion today: no system moves a canister after spawn. It keeps its spawn velocity until a
  collision. It has no `GravityAffected` (ship roots and torpedoes only,
  `crates/nova_gameplay/src/gravity.rs:239-249`).
- Intake content: builder `crates/nova_authoring/src/base_content/sections/cargo_intake.rs:94-143`,
  values :146-199. `cargo_intake_section`: detection 40 m, gap 1 m, aperture 22.2 x 15.3 m.
  `wide_intake_section`: 60 m, 1.5 m, 28 x 18 m. Both eject at 3 m/s. Generated
  `assets/base/sections/base.content.ron:3818-3822,4085-4089`.
- Runtime: `CargoIntakeSectionConfig` (:93-128, every field required, no `deny_unknown_fields`),
  kept as `CargoIntakeSectionConfigHelper` (:137). Trigger sensor: `sync_cargo_intake_triggers`
  (:436-504).
- Door (`run_cargo_intakes` :614-638): opens while an ejection queue exists, or a canister centre
  is in front of the face, within `detection_range` of the face centre, and `fits_any` the hold.
  `open` = door cue progress >= 1 (track 1.2 s, builder :76-88).
- Take (:636-672): avian contact (penetration >= 0) with the 1 m trigger and `fits_any`;
  `take_fitting` moves whole items; an empty canister despawns; a remainder keeps its id. The door
  does not gate a take (test `the_door_does_not_gate_a_take`).
- Order: `FixedUpdate`, `CargoIntakeSystems` in `SpaceshipSectionSystems` (after
  `NovaGravitySystems`), chain `(sync_cargo_intake_triggers, run_cargo_intakes)` (:837-846); a
  second trigger sync after `PhysicsSystems::Writeback` (:848-851). Avian steps in
  `FixedPostUpdate`, after the intake pass.
- Canister rules (approved, `tasks/20261026-003724/TASK.md` Decisions):
  `CARGO_CANISTER_MAX_MASS_G` 1,000 kg (`crates/nova_gameplay/src/inventory.rs:1290`), `pack`
  :1351, `take_fitting` :1380, `fits_any` :1390.
- Save: a canister freezes as `FrozenBodyType::Canister` (`crates/nova_world/src/frozen.rs:556,755`)
  and thaws with pose and linear velocity (:1062). `FrozenCanister` holds id, contents, health,
  mesh, with `deny_unknown_fields` (`cargo_intake_section.rs:289-299`). Save checks cap and ids
  (`crates/nova_world_base/src/save/mod.rs:1202-1210,1558-1566`).
- Ships with an intake: `mining_tug`, `salvage_hauler`, `light_freighter`, `interceptor`,
  `raider`, `patrol_picket`, `line_warship` (`assets/base/ships/base.content.ron`), every
  open-world start hull and generated ship (`CHANGELOG.md:377`). Stations sell
  `cargo_intake_section` at refit (`assets/base/stations/base.content.ron:369`). No mod under
  `webmods/` authors an intake.
- AI: `run_cargo_intakes` runs every intake on a ship root, so an AI hull takes a canister it
  touches. No AI behaviour flies to a canister (no `CargoCanister` reader under
  `crates/nova_ship/src/input/ai`).
- Proof today: `crates/nova_ship/src/sections/cargo_intake_section/tests.rs` (real avian, 20
  tests; guards `a_full_hold_takes_nothing_and_keeps_the_door_shut` :601,
  `the_door_opens_for_a_canister_that_fits_beside_one_that_does_not` :662,
  `an_ejected_canister_leaves_without_being_taken_back` :766). Lint tests
  `crates/nova_scenario/src/lint/ship.rs:2412-2554`, lint rules :726-745.
- Examples: `examples/playable/mine_and_sell.rs` (walk), `start_role_sandbox` (detection ring
  :466), `system_hold_events`, `lesson_cargo`, `system_station_swap_intake`.
- Docs: wiki `web/src/wiki/sections/cargo-intake.md:23` says "The intake does not reach out or
  pull"; also :31-32, :40, :49-54, :119. `web/src/wiki/sections/mining-beam.md:86`. Creator
  grammar `web/src/create/sections.md:1230-1287`. `docs/sections.md:695`. Lesson "Picking up
  canisters" (`crates/nova_authoring/src/base_content/lessons.rs:496-509`). Builder descriptions
  (`cargo_intake.rs:153-159,180-184`). Focus card (`crates/nova_ship/src/gallery/catalog.rs:180-189`).
  `examples/playable/README.md:80`.
- Format: content has no schema version (only `PORTAL_SCHEMA_VERSION`,
  `crates/nova_mod_format/src/lib.rs:30`). A new required field: add it to the struct with no
  serde default, then the builder, then every Rust literal the compiler names
  (`design_sections.rs:1175`, `ship_parts.rs:905`, `ship_layout.rs:2764`, `pickup_sight.rs:856`,
  `frozen_piece.rs:1048`, `machinery.rs:380`, `lint/ship.rs:2412`, `tests.rs:27`), `content gen`,
  `content lint`. RON without the field fails the parse. v0.18.0 saves are refused already
  (`CHANGELOG.md` Unreleased).
- Defect seen: `crates/nova_authoring/src/lint_walk.rs:1724` test RON carries
  `maximum_capture_speed: 5.0`, a field the config does not have. It parses only because the
  config has no `deny_unknown_fields`. Delete it in this change.

### Baseline: `mine_and_sell` before the change
- Runs: 3, sequential, `6845da7f6`, Xvfb + lavapipe, `NOVA_AUTOPILOT=1`, sandboxed XDG, own
  capture dir, `NOVA_PROBE_TIMELINE`. All 3 passed every walk assert, with no ERROR or panic.
  Frames were about 0.12 s, under `max_delta`, so sim time ran at real time except while paused.
- How the walk picks up: it mines until 5 or more ore is cut (pulses Ok(0), Ok(1), Ok(8) -> 9
  ore in 2 canisters: 1 and 8 ore). It tracks the 8-ore canister and flies the tug with
  `MatchVelocity` down under it, levels the dorsal intake 2.5 u below it, then lifts at
  0.25 u/s until contact. The 1-ore canister stays in space. One take. It docks and sells.
- Measure: the take's sim time is `onpickedup.scenario_elapsed` on the timeline. The walk logs the
  canister's spawn only with a wall-clock time, so born -> taken = wall-clock span minus the
  0.9 s pickup-shot pause. Error about +-0.2 s.

| run | born -> taken (sim s) | position / lift / contact (real s) | take at (scenario s) | sold | sim s to Sell | cr per sim min |
|---|---|---|---|---|---|---|
| 1 | 70.3 | 60.5 / 7.2 / 1.5 | 74.8 | 8 ore, 24 cr | 115.9 | 12.4 |
| 2 | 70.3 | 60.4 / 7.2 / 1.6 | 75.0 | 8 ore, 24 cr | 116.0 | 12.4 |
| 3 | 70.3 | 60.4 / 7.2 / 1.7 | 74.9 | 8 ore, 24 cr | 116.0 | 12.4 |

- About 86% of the pickup is the helm getting level under the canister. The pull removes that
  leg. cr per minute is set by the walk (9 ore of Stone at bid 3 per 10 kg, one canister sold),
  so it measures the flow, not the mining economy.
- The walk cannot measure per-canister sim time by itself. It needs `Time<Virtual>` stamps at
  first sight and at take (see the walk change below).
- Artifacts (not committed): scratchpad `base1..3/run.log`, `timeline.jsonl`, `measure.py`.

### Proposal
- End state: while an intake's door is fully open, each canister in front of it, within
  `pull_range` of the face centre, with at least one whole item that fits the hold, moves to the
  trigger at `pull_speed` relative to the intake. The take rule is unchanged.
- Dies: wiki "does not reach out or pull"; the fly-under-and-lift pickup in the walk; the walk
  rule "exactly one take".
- May break: jettison (a dropped canister fits the hold that just dropped it and keeps the door
  open), `mine_and_sell` asserts, canisters near hull parts in front of a face, AI hulls that
  pass loot.
- Fails loudly: a missing `pull_range` or `pull_speed` fails the parse; lint rejects a value that
  is not finite and positive, `pull_range <= capture_gap`, and `pull_range > detection_range`.

New content fields on `CargoIntakeSectionConfig` (builder `IntakeSpec` gets the same two):

| field | type | `cargo_intake_section` | `wide_intake_section` |
|---|---|---|---|
| `pull_range` | `Meters` | 40 m | 60 m |
| `pull_speed` | `MetersPerSecond` | 5 m/s | 8 m/s |

From 40 m at 5 m/s a canister arrives in about 8 s; from 60 m at 8 m/s in about 7.5 s.

Rule, each fixed tick, after the takes:
- Pulled: health > 0, centre in front of the face, within `pull_range` of the face centre,
  `fits_any` this ship's free mass after this tick's takes, no `CargoCanisterJettisoned`.
- One intake per canister: the nearest face centre, then the lower intake entity.
- Velocity: `LinearVelocity = intake point velocity + unit(trigger centre - canister centre) *
  pull_speed`. Angular velocity is kept, so a tumbling canister still tumbles; any part touching
  the trigger takes. Mass plays no part. Speed relative to the intake is never above `pull_speed`.
- Door shut or opening: no pull. The door rule does not change.
- Full hold: the door shuts, no pull. A canister the hold cannot take is not pulled, also through
  a door that another canister opened.
- Partial take: the remainder keeps its id; the hold is then full, so the pull stops; the
  remainder keeps its last velocity, meets the face and drifts off.
- Jettison: the drop spawns with a new marker `CargoCanisterJettisoned`. The pull skips it. The
  pull system removes the marker once the canister centre is beyond `pull_range` of every live
  intake. A take on contact still works as today.

```rust
/// Steer every canister an open intake pulls toward that intake's trigger.
fn pull_canisters_to_open_intakes(
    mut commands: Commands,
    q_ships: Query<
        (&Position, &Rotation, &LinearVelocity, &AngularVelocity, &ComputedCenterOfMass, &ShipInventory),
        (With<SpaceshipRootMarker>, Without<CargoCanister>),
    >,
    q_intakes: Query<
        (Entity, &ChildOf, &CargoIntakeSectionConfigHelper, &SectionCollider, &SectionAnimations),
        (With<CargoIntakeSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_canisters: Query<
        (Entity, &CargoCanister, &Position, &mut LinearVelocity, Has<CargoCanisterJettisoned>),
        (Without<HealthZeroMarker>, Without<SpaceshipRootMarker>),
    >,
    q_chain: Query<(&Transform, &ChildOf)>,
    items: Res<GameItems>,
)
```

Schedule: `FixedUpdate`, in `CargoIntakeSystems`, chained third:
`(sync_cargo_intake_triggers, run_cargo_intakes, pull_canisters_to_open_intakes).chain()`.
After the take, so the chain's sync point has despawned a canister taken whole, and a remainder
is judged on the new free mass. Before avian's step, so the step moves the canister on the new
velocity.

```text
before: FixedUpdate  sync_triggers -> run_cargo_intakes (door, take, drop)   -> FixedPostUpdate avian step
after:  FixedUpdate  sync_triggers -> run_cargo_intakes (door, take, drop
                                       + insert CargoCanisterJettisoned)
                                    -> pull_canisters_to_open_intakes        -> FixedPostUpdate avian step
```

Save: the pull keeps no state. `FrozenCanister` gets `jettisoned: bool`, so a canister dropped
just before a save is not pulled back after Load. Format change, covered by the Unreleased
save break.

Choices:
1. Which hulls pull. A: every intake on a ship root, player and AI (recommended: one rule, the
   AI door already opens and takes). B: the player ship only. Risk of A: an AI hull that passes
   within 40 m with its door to a canister takes it.
2. Pull model. S1: set the velocity each tick (recommended: speed cap is exact, no new field).
   S2: limit the change by an acceleration, which needs a third field `pull_acceleration`.
   S3: apply a force; speed is then not capped without a damping rule.
3. Range. R1: new `pull_range`, lint `<= detection_range` (recommended: owner's wording; can be
   tuned below the door range later). R2: reuse `detection_range`, add `pull_speed` only.
4. Jettison marker. J1: saved as `FrozenCanister.jettisoned` (recommended). J2: not saved;
   a Load can pull a fresh drop back.

Proof (the Verification "ECS asserted example"), in `cargo_intake_section/tests.rs` on the real
avian rig:
- `a_canister_in_pull_range_drifts_to_the_open_intake_and_is_taken`: canister at rest 30 m in
  front, off the axis. Asserts: the door opens; while it is open, the canister's speed relative
  to the ship is at most `pull_speed` and its distance to the trigger falls; it is taken (hold
  count rises, one take). The frame cap names a hang, not a budget.
- `a_canister_before_a_shut_door_does_not_move`: full hold, canister at rest 30 m in front.
  Asserts: door 0, same position, zero velocity, no take.
- `a_canister_the_hold_cannot_take_is_not_pulled_through_an_open_door`.
- `an_ejected_canister_leaves_without_being_taken_back` stays and now guards the marker; with J1,
  a freeze/thaw assertion that the marker survives.
- Lint tests for the new bounds next to `lint/ship.rs:2459`.

`mine_and_sell` walk (example code, ships with the change):
- With the pull the door opens while the tug descends under the tracked canister, so the
  canister arrives before the lift, and the 1-ore canister can also come in. The walk then waits
  900 s on a lift that never comes, and `check_take` (:617-631) fails on `taken == 1`.
- End the pickup leg on "the tracked canister is gone through a take". Replace "exactly one
  take" with: the tracked canister was taken, every gone canister went through a take, hold ore
  = taken ore (`check_ledger` already proves the ledger). Shoot the pickup frame when the door is
  fully open. Re-record `loop-section-cargo-intake`.
- Log `Time<Virtual>` elapsed at each canister's first sight, at each take, and at the Sell.
- Pull ring beside the detection ring in `start_role_sandbox` (:466); README row :80.

Docs and changelog:
- Ships & Sections: "An open cargo intake pulls a canister its hold has room for to its door: 40
  m at 5 m/s, wide intake 60 m at 8 m/s. A canister you jettison is not pulled back."
- Modding & Mod Portal: "**(breaking)** A `CargoIntake` section needs `pull_range` (m) and
  `pull_speed` (m/s). Lint requires `capture_gap < pull_range <= detection_range`."
- Wiki cargo-intake (intro, table, rules table, catalog row), mining-beam :86, creator grammar,
  `docs/sections.md:695`, lesson text, builder descriptions, focus card "Pull" row.

Measurement plan:
- Land the walk log lines first (example only). Run 3 walks on master as the named reference and
  3 after the change, same host, lavapipe, quiet host (no user game, loadavg < 3.5), one at a
  time.
- Per run: tracked canister born -> taken (sim s), canisters taken, ore sold, cr, sim s to the
  Sell, cr per sim minute. Compare medians of the two sets. Done when the after median of born ->
  taken is lower. No timing assert.
