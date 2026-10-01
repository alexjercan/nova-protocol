# Review: task #91 runtime slice (regional-ship-runtime, uncommitted)

Scope: `git diff` + untracked `crates/nova_world_base/src/sector_ships.rs`,
against the "Approved runtime contract" (task #91) and AGENTS.md.

## Verification performed

- `nix develop --command cargo check -p nova_world -p nova_world_base
  -p nova_scenario -p nova_gameplay --tests` - clean.
- `cargo check --examples --tests -p nova-protocol` - clean.
- `cargo test -p nova_world -p nova_world_base --lib` - 46/51 nova_world_base
  pass, 28/28 nova_world pass. The 5 nova_world_base failures are exactly the
  known seed-20260922 `Unruined` sector faults the task calls out as accepted
  (`a_cluster_is_the_same_cluster_from_every_cell_that_replays_it`,
  `every_cluster_body_has_one_owner_and_clusters_cross_faces`,
  `generated_rocks_carry_a_well_from_50_m`,
  `every_placed_hull_has_a_companion_of_its_own_cluster_in_its_cell`,
  `tests::a_pinned_window_generates_the_recorded_bodies`). Not reported as
  findings per instructions.
- `cargo test --example world_clusters` - 11/15 pass; the 4 failures are the
  same accepted `Unruined` fault on the fixture's own catalog
  (`every_planned_body_has_one_owner_that_places_or_skips_it`,
  `world_and_hull_groups_in_the_home_window_place_bodies_on_both_sides_of_a_face`,
  `a_group_is_the_same_group_from_every_cell_that_replays_it`,
  `a_window_generated_in_reverse_order_is_the_same_window`). Matches the
  brief's stated count (5 + 4) exactly. Not reported as findings.
- `cargo test -p nova_gameplay --lib neutralize` - 9/9 pass.
- `cargo test -p nova_scenario --lib spaceship` - 19/19 pass.

## Findings

### MINOR, CONFIRMED - stale marker count in doc comment
`crates/nova_gameplay/src/markers.rs:4`: "These are the eleven markers that
both sides of the ship seam read." The `prelude` re-export list this diff
also edited (`markers.rs:27-32`) names 14 marker types (counted: Controller,
Derelict, GunRound, PlayerSpaceship, RailgunSection, RailgunSlugProjectile,
SectionInactive, Section, SpaceshipRoot, ThrusterSection, TorpedoProjectile,
TorpedoSection, TurretBulletProjectile, TurretSection). The pre-existing text
said "ten" against an actual pre-diff count of 13 (already stale before this
change), and this diff incremented it by one to "eleven" instead of
recounting, so the drift is carried forward and grows. Consequence: a reader
trusts a stale inventory number in a doc comment this diff directly touched.
Fix is a one-word edit ("fourteen").

### PLAUSIBLE, minor performance - primary civilization redrawn per hull, not per cluster
`crates/nova_world_base/src/sector_ships.rs:130` (`plan_ship`) calls
`primary_civilization` for every hull a cell owns. `primary_civilization`
(`sector_ships.rs:214-236`) draws its stream and scans
`CivilizationField::in_reach` (a node-window scan over up to roughly 4 nodes
per axis, ~60 candidate nodes at the 320 km reach / 240 km lattice spacing)
keyed only by `(seed, node, anchor)` - none of which vary by hull slot within
one cluster. Every hull in the same cluster therefore repeats the same
`in_reach` scan and stream draw. Correctness is unaffected (deterministic,
same result each time) and the work stays off the main thread (workers), and
cluster hull counts are small, so this is a bounded, likely negligible cost
today - flagging as a PLAUSIBLE, not urgent, optimization: hoist the primary
civilization to one draw per cluster (e.g. compute once in `plan_sector`/
`plan_cell` and pass it into `plan_ship`, or into `HullSlot`) instead of
recomputing it once per owned hull.

### Note, not a defect - fixture rebuilds a whole `NovaLayeredWorld` to read its parts
`examples/shared/world_fixture/mod.rs` (`clustered_world_config`) constructs
a full `NovaLayeredWorld::from_loaded(loaded)` and then clones `.parts()`
into a fresh `Arc` for `ClusteredWorld`, discarding the `NovaLayeredWorld`
itself. This is indirect (a `ShipPartSnapshot::build` call would do it
directly, as the ship_layout test helpers already do), but it reuses the
same validated arming path the live game uses, so it is a reasonable and
deliberate DRY choice rather than dead weight. Not flagged as a finding.

## Contract checks that came back clean (no finding)

- `SectorShip` carries inline `ShipDesign`, full `Quat` rotation, a
  root-centred `clearance` capped by `SECTOR_SHIP_CLEARANCE_MAX` (400 m,
  `crates/nova_world/src/generation.rs:276`) and `SectorShipConditionType`
  (`generation.rs:53-92, 216-232`). `validate_manifest`
  (`generation.rs:436-466`) refuses a non-finite/non-unit rotation, a
  non-positive or over-ceiling clearance, and (`check_ship_design`,
  `generation.rs:482-503`) an empty/duplicate/inline-sourced/non-finite
  design - all worker-side, no catalog needed.
- `canonical_rotation` (`generation.rs:222-232`) sign-normalizes `q`/`-q`
  alike; exercised by
  `canonical_text_describes_a_rotation_and_its_negation_alike`.
- Selection is pure and keyed by `(seed, node, slot)` before any spatial
  check: `plan_ship` runs inside the cell's hull loop
  (`clusters.rs:284-341`, `clustered.rs:906-931`) *before* `resolve()`'s
  companion/clearance checks, so a layout/content fault (`Unruined`
  included) propagates via `?` and fails the whole `plan_sector`/`plan_cell`
  call - not a named skip. Only `resolve()`'s `SkipType::Face` /
  `SkipType::Clearance` / `SkipType::Companion` are named skips
  (`clusters.rs:1339-1420`). `ClusterBody::Hull{..}` reaching the manifest
  match is `unreachable!()` and correctly so: only owned hulls (all
  converted to `Ship`) are ever pushed into a cell's own `hulls`/`members`
  list.
- Primary civilization draw is keyed by `(seed, node)` only
  (`sector_ships.rs:220-229`, no `HashMap` iteration, node ranges are fixed
  nested `for` loops - deterministic). Secondary-civilization candidates
  come from `CivilizationField::in_reach`, itself a bounded fixed-order
  triple loop (`civilizations.rs:373-392`), not a hash-ordered collection.
- Clearance math: `clearance()` (`ship_layout.rs:715-720`) now measures from
  the design origin to the farthest cell-edge corner
  (`max(|low-0.5|, |high+0.5|)` per axis) instead of the old
  center-assuming half-span; `centred_clearance()` (`ship_layout.rs:726-733`)
  matches the actual post-`shift` centring in `finalize`
  (`ship_layout.rs:1597-1598`, `-((low+high)).div_euclid(2)`, x untouched as
  the mirror axis) - hand-verified against even/odd cell-count cases. No
  direct numeric-value unit test pins the formula, but it is exercised by
  every passing generated-ship/wreck test and the ceiling/attempt-bound
  tests, none of which regressed.
- ECS ordering: `(materialize_ready_sector::<G>, materialize_pending_ships)`
  are `.chain()`-ed inside `NovaWorldSystems::Materialize`
  (`lib.rs:823-825`), consistent with the pre-existing set-level `.chain()`
  pattern the rest of the plugin already relies on for command visibility
  across stages.
- `DerelictShipMarker` is inserted in the same `commands.spawn((ship,
  DerelictShipMarker))` bundle as the rest of the ship
  (`streaming.rs:240-243`), and `insert_spaceship_sections` observes
  `On<Add, SpaceshipRootMarker>` (`spaceship.rs:527-541`) reading
  `Has<DerelictShipMarker>` off the same entity - since Bevy applies a whole
  spawn bundle before firing its `Add` observers, the marker is visible.
  Confirmed by the passing
  `a_derelict_spawns_every_section_but_hull_and_docking_inactive` test.
  `detect_neutralized` excludes `Without<DerelictShipMarker>`
  (`neutralize.rs:108-115`); confirmed by
  `a_derelict_with_inactive_weapons_is_never_neutralized`.
- Pending-ship lifetime: `materialize_pending_ships` despawns the held
  entity and spawns the real one in the same `Commands` buffer, in that
  order (`streaming.rs:321-322`), so no duplicate-id entity window opens;
  retiring a root despawns its `PendingSectorShip` children (recursive
  despawn), confirmed by `a_held_ship_retires_with_its_cell`.
  `materialize_ready_sector` and `materialize_pending_ships` each
  independently panic via `SectorFault::AbsentObserver` on
  `observer.single()` failure, but this is reachable-but-redundant: the
  `Materialize` set only runs once `CurrentSector` exists, which
  `track_current_sector` (earlier in the same chained frame) only inserts
  after its own `observer.single()` succeeded - not a new panic surface.
- Docs/changelog: `docs/architecture.md`'s `nova_world`/`nova_world_base`
  rows, `CHANGELOG.md`, and `web/src/wiki/{getting-started,scenarios}.md`
  were updated to match the new behavior (generated ships, pending-ship
  delay, no cargo yet); spot-checked against the code, no stale claims
  found. The authored Derelict Tender
  (`crates/nova_authoring/src/base_content/scenarios/open_world.rs`) and its
  Take/Give are untouched, as the review boundary requires; generated
  wrecks spawn with `ShipInventoryStock::new([])` (no loot), matching the
  stated task #92 boundary.
- No leftover references to the old `SECTOR_SHIP_CLEARANCE`,
  `SectorFault::UnknownShip`, `ShipDesignId`-typed ship field, or
  `DERELICT_DESIGNS` anywhere in the tree.

## Skipped checks / uncertainties

- Did not run workspace-wide tests or Clippy (per instructions).
- Did not run a game/GPU capture; no rendered-travel or in-editor visual
  check was performed for this review.
- Did not audit `ship_parts.rs`/`ship_layout.rs` generation internals beyond
  the touched `clearance`/`centred_clearance` functions and the
  `MAX_HULL_CLEARANCE` constant change - the bulk of that file's generation
  logic is unchanged by this diff and was presumably reviewed in the PR A
  gate.
- Did not independently re-derive the `civilizations.rs` influence/reach
  math (file has only a doc-comment diff here); treated it as already
  reviewed under V1.
- Did not verify at runtime that the production player ship's
  `IntegrityEnvelope` is actually populated (so `ObserverBody::reach` is
  nonzero for the real game, not just the free-fly example camera); this
  component is pre-existing and unmodified by this diff, so out of scope,
  but flagging as unverified since it is load-bearing for the pending-ship
  overlap gate.
