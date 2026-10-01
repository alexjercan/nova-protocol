# Review: generated open-world wrecks replace the authored Derelict Tender

Scope: only the loot/stock slice (design bullets 1-5). This worktree carries
other uncommitted work (PendingSectorShip/ObserverBody clearance-hold,
SectorFault::UnknownShip -> InvalidShipDesign, inline ship designs, the
Unruinable fix) that is NOT part of this slice; not reviewed for correctness
here.

## Verdict
No BLOCKER or MAJOR findings against the approved design. One MINOR proof gap.

## Findings

**MINOR - CONFIRMED.** `plan_ship`'s `stock` failure branch
(`crates/nova_world_base/src/sector_ships.rs:197-208`, `SectorFault::Generation`
field `"stock"`) has no direct test. `wreck_stock` returning `None` is tested
directly (`crates/nova_world_base/src/tests.rs:169-176`), and the mass-clamp
is tested through `plan_ship`'s real output (`tests.rs:111-160`), but no test
drives `plan_ship` itself to a hull-less wreck to observe the `Err` and its
field name. Not one of the three named tests, so within the approved scope
this may be an accepted gap - flagging so the owner can decide.

## Checks performed (all CONFIRMED)

- `SectorShip.stock` field added (`crates/nova_world/src/generation.rs:93`),
  printed in `canonical()` as `stock {:?}` of `stacks()`
  (`generation.rs:211-216`).
- `spawn_sector_ship` (`crates/nova_world/src/streaming.rs:330-361`) stays
  private (`fn`, no `pub`); spawns `inventory: stock`; sets
  `lootable: derelict` on `SpaceshipConfig` (inert at this call site since
  `spaceship_scenario_object` never reads `.lootable` - only the scenario
  spawn ACTION does, per `nova_scenario/src/actions/spawn.rs:216`, which this
  path bypasses) and separately inserts `LootableShipMarker` beside
  `DerelictShipMarker` only for `condition == Derelict`; the intact branch
  spawns neither marker and stock is always `ShipInventoryStock::default()`
  for intact ships (`sector_ships.rs:197-198`).
- `sector_ships.rs`: `PlannedShip.stock`, `WRECK_PLATES = 1..=8`,
  `wreck_stock(parts, design, draw)` counts Hull-kind sections of the passed
  design, cuts to `hulls * HULL_SECTION_CARGO_G / HullPlate.mass_g()`, returns
  `None` at zero plates. `plan_ship` maps `None` to
  `SectorFault::Generation { field: "stock", .. }` via `ok_or_else`
  (`sector_ships.rs:197-207`).
- Stock draw independence: `plan_ship` draws `civilization_draw`, `role_draw`,
  `turn` from stream keyed `b"sector_ship"` up front, then opens a *separate*
  `SeedStream::new(key(b"sector_ship_stock"))` only for the stock draw
  (`sector_ships.rs:200`) - adding/removing draws on one stream cannot
  perturb the other.
- Clamp matches `ResolvedShipDesign::cargo_capacity_g`: both count Hull-kind
  sections (`ship_design.rs:402-412` vs `sector_ships.rs:256-270`); test
  `every_planned_wreck_carries_one_to_eight_plates_its_hull_holds_and_an_intact_ship_none`
  resolves the real design and asserts `stock.mass_g() <= cargo_capacity_g()`
  (`nova_world_base/src/tests.rs:150-156`) - non-vacuous, uses the real
  resolver, not a hand-built count.
- Authored derelict fully removed: `open_world.rs` derelict object, constants
  `DERELICT_ID`/`DERELICT_NAME`/`DERELICT_POSITION`/`DERELICT_ROTATION`, and
  the generated `assets/base/scenarios/open_world.content.ron` entry are all
  deleted together; scenario description and doc comment text updated to
  match (no leftover "derelict" singular noun describing a scripted ship).
- `BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID` / `BLOCK_WRECK_PLATE_SHIP_ID` moved
  `nova_world_base` -> `nova_authoring/base_content/ships/mod.rs`; every
  caller (`season_one/stage.rs`, `season_one/tests.rs`) updated to the new
  import path; doc comments correctly dropped the now-false "one of the two
  hulls a cluster's derelicts are drawn from" claim.
- No leftover `DERELICT_DESIGNS`, `derelict_tender`, "Derelict Tender", "140
  m", or "8 plates beside" language anywhere under `crates/`, `web/`,
  `docs/`, `CHANGELOG.md`, or the generated RON (grepped). `web/src/create/
  ships.md` and `crates/nova_bench/scenarios/docking_warship_tender.content.
  ron` still reference `block_frame_tender_damaged` by design - that ship
  prototype itself still exists and is unrelated to open-world loot.
- `web/src/wiki/interface.md`, `scenarios.md`, `getting-started.md`: rewritten
  to describe generated wrecks (1-8 plates, generic `<ship>` name in Take/Give
  note-line examples) instead of the fixed "Derelict Tender" name; the
  `<!-- proof -->` owner-comment block at the top of `interface.md` updated to
  point at `wreck_stock` / `spawn_sector_ship` as the new proof paths.
- `docs/architecture.md` `nova_world` / `nova_world_base` rows: describe
  `stock`, `DerelictShipMarker`/`LootableShipMarker` insertion, restocking on
  cell retirement, and `wreck_stock`'s clamp - matches the code read above.
- `CHANGELOG.md` `[Unreleased]` Scenarios entries: both new/edited bullets
  measured at 137 and 192 chars joined, under the 200-char limit; no
  `**(breaking)**` marker used, correctly, since this is unshipped open-world
  content (not a format break).
- Neutralization: `DerelictShipMarker` excludes a root from
  `detect_neutralized`'s query (`nova_gameplay/src/integrity/neutralize.rs:
  108-118`) and every non-hull/non-docking section on a derelict spawns
  `SectionInactiveMarker` (`nova_scenario/src/objects/spaceship.rs:619-629`),
  covered by `a_derelict_with_inactive_weapons_is_never_neutralized`
  (pre-existing plumbing this slice depends on, not itself part of the
  reviewed bullets).
- Ran the three named tests directly in this worktree, all pass:
  `nova_world_base::tests::every_planned_wreck_carries_one_to_eight_plates_its_hull_holds_and_an_intact_ship_none`,
  `nova_world::tests::a_materialized_derelict_is_lootable_with_its_manifest_stock_and_an_intact_ship_is_not`,
  `nova_interface::inventory::tests::generated_wreck::a_docked_generated_wreck_gives_its_plates_by_take_and_takes_them_back_by_give`.
  The last is a real-ECS integration test (real docking systems, real
  catalog RON parsed from disk, real inventory transfer system) - not
  vacuous.
- `nova_interface/Cargo.toml` dev-dependency additions carry a comment naming
  what each new dep is for (`ron`, `serde`, `nova_gameplay test-support`,
  `nova_scenario serde`, `nova_world`, `nova_world_base`) - matches AGENTS.md's
  "explain a concrete reason" comment rule.

## Skipped / not checked

- Correctness of `PendingSectorShip`/`ObserverBody`/`materialize_pending_ships`
  (clearance-hold mechanism), the `SectorShip.design` inline-vs-catalog
  rework, `SectorFault::InvalidShipDesign`, and the wreck-ruination
  (`Unruinable`) fix in `crates/nova_authoring/tests/generated_wrecks.rs` -
  out of this slice per the approved design, left for their own review.
- No workspace-wide test run or Clippy run, per instruction.
- Did not run the game or a GPU measurement.
- Did not check `Cargo.lock` diff content (dependency-graph churn from the
  unrelated work above, not this slice).
- Did not independently verify the FNV digest recompute
  (`0xed7e_d8f2_b5b5_92fd`) by hand; trusted the passing pinned-window test.
