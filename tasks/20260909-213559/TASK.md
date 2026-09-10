# Scenario, editor and authoring sizes derive from the hull and the body

- STATUS: OPEN
- PRIORITY: 82
- TAGS: v0.14.0, bug, editor, scenario, review

## Goal

Every scenario, editor and authoring size that is fixed in world units but
really depends on the hull, the body, or the document derives from it.
Sibling of `20260909-213350` (HUD and camera) and of the ship and gameplay
sweep. From the 2026-09-09 sweep of `nova_scenario`, `nova_editor`,
`nova_authoring`, `nova_wfc`, `nova_assets`.

Sizes to derive from: `HullRadius`, `BodyRadius`, `node_bounds` (the editor
already merges subtree bounds in `frame::apply_frame_request`), a section
prototype's `SectionFootprint` or collider half-extents, the camera's live
framing distance, the authored radius. Author margins in meters.

## Editor

- [ ] `crates/nova_editor/src/node.rs:712` entering a ship frames with
      `frame_stage(target.translation, 0.0)`: a zero spread, so the pose is
      always 50 m up and 100 m back from the ORIGIN. Double-click a 30-cell
      carrier and the camera lands inside plate. Use the subtree bounds.
- [ ] `node.rs:721` the scenario-level frame spreads over ship translations
      only, ignoring hull size: one carrier at the origin has spread 0.
- [ ] `node.rs:42,1111` `SHIP_NODE_SPACING` 24 u between minted ships: two
      carrier designs interpenetrate on the stage AND in the flown sandbox,
      since `lower_ship` hands the node transform to the spawn. Sum the two
      neighbours' `HullRadius` plus a margin.
- [ ] `preview.rs:345` `hull_extents` pads a fixed half cell, assuming a
      1x1x1 collider: a 3x3x2 vector thruster or a 5x5x3 capital drive at
      the stern is unclickable and under-reported to `node_bounds`. Add the
      outermost section's own half-extents.
- [ ] `preview.rs:173` a rock preview is drawn at radius times
      `ASTEROID_GEOMETRIC_FACTOR_MIN` (3.5) while the flown body is up to
      6.0 times: a belt laid flush by eye explodes apart on the first
      physics step. Bound from the same `pristine_field` the spawn uses.
- [ ] `placement.rs:244` `NEW_OBJECT_DISTANCE` 30 u in front of the camera:
      a beacon lands behind everything when zoomed in and on the lens when
      framing a 7 km range. Use the framing distance or the grid-plane hit.
- [ ] `inspect.rs:2710` `POSE_STEP` 0.5 m per pixel of drag: 16,000 px to
      cross the tutorial's range, five metres a pixel inside a hull. Scale
      with camera distance like `SOCKET_SCREEN_SIZE` does.
- [ ] `scenario.rs:231` `BEACON_LOCK_SIGNATURE` 300 m hand-derived from
      `signature_range_per_unit` and "the deepest beacon": drag the Veil
      beacon out and it drops off the lock list. Compute from the furthest
      beacon and the live setting.
- [ ] `gallery/scene.rs:30` `STAGE_ORIGIN` at y = 2,000 m assumes a bounded
      build area; an object authored near y = 20,000 m draws through the
      gallery tiles. Offset from the document's merged bounds.
- [ ] `gallery/mod.rs:30` `COLS` 4, `ROWS` 3, `PAGE` 12 regardless of
      viewport: 200 prototypes are 17 pages on a 4K display with room for
      40. Derive from the stage width and height.

## Scenario runtime

- [ ] `crates/nova_scenario/src/loader/lifecycle.rs:302` every scenario
      spawns its camera at a fixed 100 m up, 200 m back from the origin: a
      big player hull at the origin opens with the camera inside it; a
      player kilometres out opens on empty space. Sit off the player spawn
      at a `HullRadius`-derived distance.
- [ ] `actions/spawn.rs:358` `sample_clear_of` rejects against one authored
      scalar `min_separation` and `scatter_placements` stores bare
      positions: the tutorial's shallow belt (450 m) scattered against the
      deep belt's 240 m geometric rocks overlaps and penetration-shoves.
      Measure each pair against both bodies' derived radii.
- [ ] `objects/planet_surface.rs:71` `PLANET_SUBDIVISIONS` 48 for every
      radius: a 5 km world has 113 m facets, a 200 m planetoid wastes 24k
      triangles. Solve for a target facet size and clamp to the max, as
      `asteroid_carve::field_resolution` does.
- [ ] `loader/preload.rs:32` `PRELOAD_TIMEOUT_SECS` 10 s wall clock for
      every glTF a scenario spawns: a mod with hundreds of meshes, or the
      wasm build over the network, trips it and ships pop in live. Reset
      the budget on progress.

## Authoring

- [ ] `crates/nova_authoring/src/base_content/scenarios/pacing.rs:43`
      `REVEAL_GAP`, `INSTRUCTION_GAP`, `MID_GAP` come from the DEFAULT comms
      dwell, not the dwell of the line each beat paces against: a 20 s
      reveal line gets its objective at 8.4 s. Read the cue's own dwell.
- [ ] `base_content/ships/block.rs:46` `TURRET_SEAT` hand-copies the PDC
      prototype's link point; `:171,207,475,515,531` hand-compute thruster
      and bay seats from collider depths. Retune a prototype and every
      shipped hull floats or overlaps and fails lint. Read the prototype's
      `SectionFootprint` and link point.

## Judged correct, leave alone

`FIELD_CELL_WORLD` (count derives from radius), the gallery tile fit
(scaled to measured bounds), the stage grid step (solved from camera
reach), `MARK_MARGIN` (fraction of the collider AABB), `AreaOccupancy` as a
set, the WFC `runnable` bounds, `MAX_SCATTER_COUNT`, the portal size caps,
the menu backdrop camera.

## Proof

- `system_ship_editor` extended: enter a carrier design and assert the
  camera sits outside its bounds; mint two carriers and assert no overlap.
- A scatter unit test with two radii that the scalar rule passes and the
  pair rule rejects.
- A pacing unit test with an authored 20 s dwell.
- The hull lint run over the regenerated base fleet after the seat
  derivation, byte-identical RON expected where nothing moved.
