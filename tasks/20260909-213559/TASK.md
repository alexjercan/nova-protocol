# Scenario, editor and authoring sizes derive from the hull and the body

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.14.0, bug, editor, scenario, review

## Goal

Every scenario and editor size that is fixed in world units but really depends
on the hull, body, document, camera framing, or viewport derives from it. Make
the shared WASD camera frame-rate independent and optionally accelerated. Make
scenario loading atomic: build the complete scenario while simulation is
frozen, then expose frame zero only after loading succeeds.

Sibling of `20260909-213350` (HUD and camera) and of the ship and gameplay
sweep. From the 2026-09-09 sweep of `nova_scenario`, `nova_editor`,
`nova_authoring`, `nova_wfc`, `nova_assets`.

Sizes derive from `HullRadius`, `BodyRadius`, `node_bounds`, section collider
half-extents, the camera's stored framing distance, the viewport, and authored
body radii. Author margins in meters.

## WASD camera

- [x] Make movement frame-rate independent at a base 60 m/s and normalize
      combined directional input.
- [x] Add per-camera optional quadratic acceleration: two seconds to a 32x
      maximum. Any translation input advances it; releasing all translation
      resets it; direction changes do not.
- [x] Add independent optional FOV feedback: up to 8 degrees with acceleration,
      easing back over 0.25 seconds. Restore the original FOV when control is
      removed.
- [x] Preserve camera settings across controller removal and restoration.
      Enable the same accelerated profile for editor and scenario cameras;
      examples can select constant speed by disabling acceleration and FOV
      feedback.

## Editor

- [x] `crates/nova_editor/src/node.rs:712` entering a ship frames with
      `frame_stage(target.translation, 0.0)`: a zero spread, so the pose is
      always 50 m up and 100 m back from the ORIGIN. Double-click a 30-cell
      carrier and the camera lands inside plate. Use the subtree bounds.
- [x] `node.rs:721` the scenario-level frame spreads over ship translations
      only, ignoring hull size: one carrier at the origin has spread 0.
- [x] `node.rs:42,1111` `SHIP_NODE_SPACING` 24 u between minted ships: two
      carrier designs interpenetrate on the stage AND in the flown sandbox,
      since `lower_ship` hands the node transform to the spawn. Keep newly
      minted ships in automatic +X layout as their bounds change, with 20 m
      between adjacent collider bounds, until the creator positions one
      manually.
- [x] `preview.rs:345` `hull_extents` pads a fixed half cell, assuming a
      1x1x1 collider: a 3x3x2 vector thruster or a 5x5x3 capital drive at
      the stern is unclickable and under-reported to `node_bounds`. Add the
      outermost section's own half-extents.
- [x] `preview.rs:173` a rock preview is drawn at radius times
      `ASTEROID_GEOMETRIC_FACTOR_MIN` (3.5) while the flown body is up to
      6.0 times: a belt laid flush by eye explodes apart on the first
      physics step. Bound from the same `pristine_field` the spawn uses.
- [x] `placement.rs:244` `NEW_OBJECT_DISTANCE` 30 u in front of the camera:
      a beacon lands behind everything when zoomed in and on the lens when
      framing a 7 km range. Use the framing distance or the grid-plane hit.
- [x] `inspect.rs:2710` `POSE_STEP` 0.5 m per pixel of drag: 16,000 px to
      cross the tutorial's range, five metres a pixel inside a hull. Scale
      with camera distance like `SOCKET_SCREEN_SIZE` does.
- [x] `scenario.rs:231` `BEACON_LOCK_SIGNATURE` 300 m hand-derived from
      `signature_range_per_unit` and "the deepest beacon": drag the Veil
      beacon out and it drops off the lock list. Compute from the furthest
      beacon and the live setting.
- [x] `gallery/scene.rs:30` `STAGE_ORIGIN` at y = 2,000 m assumes a bounded
      build area; an object authored near y = 20,000 m draws through the
      gallery tiles. Offset from the document's merged bounds.
- [x] `gallery/mod.rs:30` `COLS` 4, `ROWS` 3, `PAGE` 12 regardless of
      viewport: 200 prototypes are 17 pages on a 4K display with room for
      40. Derive from the stage width and height.

## Scenario runtime

- [x] `crates/nova_scenario/src/loader/lifecycle.rs:302` every scenario
      spawns its camera at a fixed 100 m up, 200 m back from the origin: a
      big player hull at the origin opens with the camera inside it; a
      player kilometres out opens on empty space. Sit off the player spawn
      at a `HullRadius`-derived distance.
- [x] `actions/spawn.rs:358` `sample_clear_of` rejects against one authored
      scalar `min_separation` and `scatter_placements` stores bare
      positions: the tutorial's shallow belt (450 m) scattered against the
      deep belt's 240 m geometric rocks overlaps and penetration-shoves.
      Measure each pair against both bodies' derived radii.
- [x] `objects/planet_surface.rs:71` `PLANET_SUBDIVISIONS` 48 for every
      radius: a 5 km world has 113 m facets, a 200 m planetoid wastes 24k
      triangles. Solve for a target facet size and clamp to the max, as
      `asteroid_carve::field_resolution` does.
- [x] `loader/preload.rs:32` uses a 10 s total timeout and then permits pop-in.
      Replace it with a 10 s no-progress timeout. A settled root asset resets
      the budget; total loading time is unlimited while progress continues.
- [x] Make scenario loading atomic. Hold virtual and physics clocks, gameplay
      input, and camera control while queued spawns and required glTF
      dependencies settle. Remove the loading screen and release the freeze
      together, then begin simulation on the following frame at scenario time
      zero.
- [x] An explicit asset failure or no-progress timeout fails closed through the
      existing `FAILED TO START` report, listing failed or stalled paths. Keep
      simulation frozen; Main Menu is the only exit. Do not continue with
      placeholders or pop-in.

## Authoring

- [x] Remove the generic `REVEAL_GAP`, `INSTRUCTION_GAP`, and `MID_GAP`
      timing styles. Dialogue dwell and every objective or follow-up sequence
      delay are independent, explicit authored values. Add no dwell-derived
      timing calculation or lint relationship. Preserve current generated
      timings where no deliberate retiming is requested.

## Excluded after review

`base_content/ships/block.rs` seat derivation is not a defect. Current ships
match current section geometry and their link graphs are tested. Keep ship
transforms explicit; when prototype geometry changes, hull lint identifies the
ships whose authored transforms must be corrected. Do not change sections or
block-fleet placement for this task.

## Decided implementation

- Context framing uses complete subtree bounds. Focusing a ship or object shows
  that node; focusing the scenario shows the complete document. Empty nodes
  fall back to their transform.
- Store the framing point and distance whenever context focus, Frame Selection,
  or a preset frames a node. Object creation and position-field drag use that
  stable distance. Place objects on the forward ray at the stored distance;
  set position drag to `framing distance in meters / 200` with a precision
  floor and freeze the scale through one gesture.
- Newly minted ships remain in automatic +X layout until manually positioned.
  Reflow them as hull bounds change with 20 m between adjacent collider bounds.
  Any manual position edit or drag permanently removes that ship from automatic
  layout.
- Fixed spaceship previews merge exact resolved section collider bounds,
  including section rotation and half-extents. Keep the empty/unresolved hull
  fallback.
- Asteroid previews remain cheap schematic spheres. Resolve the same seed as
  runtime, including ID-derived seeds, and size them from the same conservative
  pristine-shape bound.
- Derive sandbox beacon lock signature from player spawn to the furthest
  authored beacon using live `TargetingSettings::signature_range_per_unit`.
- Offset the gallery stage beyond the document's merged bounds. Derive rows and
  columns from available viewport space with minimum readable tile dimensions,
  capped at 8 columns by 5 rows. Paging and keyboard navigation use the same
  derived layout and update after resize.
- Position the scenario camera from the actual player spawn, player rotation,
  and hull-derived chase framing. Use the origin fallback only when there is no
  player.
- Store each scatter placement as position plus derived body radius. Reject at
  `max(min_separation, existing_radius + candidate_radius)`. Keep
  `min_separation`'s format and absolute center-distance meaning. Draw randomized
  asteroid radius before placement from a separate seeded RNG stream.
- Runtime planet meshes target 20 m facets from body radius and clamp
  subdivisions to the existing hard maximum of 79. Both are engine-level
  constants; editor preview detail remains independently limited.
- Add a dedicated scenario-load freeze owner/state. Real time continues for
  loading and its animation; simulation does not advance behind the panel.

## Judged correct, leave alone

`FIELD_CELL_WORLD` (count derives from radius), the gallery tile fit
(scaled to measured bounds), the stage grid step (solved from camera
reach), `MARK_MARGIN` (fraction of the collider AABB), `AreaOccupancy` as a
set, the WFC `runnable` bounds, `MAX_SCATTER_COUNT`, the portal size caps,
the menu backdrop camera.

## Proof

- [x] WASD travel is equal at different frame rates. Constant and accelerated
      profiles, normalized input, FOV recovery, and controller restoration are
      covered. `camera/wasd/tests.rs`:
      `a_held_key_crosses_the_same_distance_at_any_frame_rate`,
      `a_diagonal_is_no_faster_than_an_axis`,
      `a_partial_press_keeps_its_share_of_the_speed`,
      `the_ramp_reaches_its_top_in_two_seconds_and_only_when_asked`,
      `releasing_translation_resets_the_ramp_and_a_direction_change_does_not`,
      `the_lens_widens_with_the_ramp_and_is_given_back_on_removal`,
      `a_camera_can_ramp_without_moving_its_lens`; and in
      `camera/wasd_controller.rs`,
      `a_camera_keeps_its_profile_across_losing_and_regaining_the_rig`.
- [x] `system_ship_editor` enters a carrier outside its bounds and frames the
      whole scenario at scenario context. Two automatic carriers do not
      overlap; manual movement opts one out of reflow. `node.rs`:
      `entering_a_carrier_frames_it_from_outside_its_own_bounds`,
      `the_scenario_context_frames_the_whole_document`,
      `two_automatic_carriers_stand_clear_of_each_other`,
      `the_row_reflows_as_a_hull_grows`,
      `moving_a_ship_by_hand_takes_it_out_of_the_row`. The example itself was
      flown: `autopilot: cycle complete, no panic (t=13.4s)`, exit 0.
- [x] A rotated multi-cell section expands a fixed-hull preview correctly. An
      ID-derived asteroid seed produces the same conservative preview bound as
      runtime. `preview.rs`:
      `a_rotated_multi_cell_section_expands_the_hull_box`,
      `an_off_centre_hull_is_bounded_where_it_actually_stands`,
      `a_hull_with_nothing_resolved_keeps_a_unit_cell`,
      `a_rock_without_an_authored_seed_is_drawn_at_the_reach_its_id_gives_it`.
- [x] New-object placement and position drag use the stored framing distance.
      `frame/tests.rs`: `a_frame_records_the_point_and_the_reach_it_stood_at`;
      `placement.rs`: `a_placed_object_lands_at_the_distance_the_camera_is_framing`;
      `ui/inspector/tests.rs`: `a_position_drags_at_the_scale_the_camera_is_framing`.
- [x] Gallery layout and keyboard paging agree before and after viewport
      resize. `gallery/mod.rs`:
      `the_grid_grows_with_the_window_and_stops_at_the_caps`,
      `a_resize_moves_the_grid_the_paging_reads`; `gallery/input.rs`:
      `the_arrow_keys_step_the_grid_the_window_gives`. The stage offset has its
      own pair in `gallery/scene.rs`:
      `the_stage_stands_clear_of_whatever_the_document_holds`,
      `the_stage_holds_still_while_the_gallery_is_open`. The live editor run
      drew 4x3 tiles at 1024x768 ("17 parts page 1/2"), which is what the
      derived layout asks for at that size.
- [x] A large player hull spawned away from the origin starts with the camera
      outside it. `loader/lifecycle.rs`:
      `a_large_player_hull_starts_with_the_camera_outside_it`,
      `the_camera_opens_behind_a_player_spawned_far_from_the_origin`,
      `a_scenario_with_no_player_opens_on_the_origin`. Flown: the tutorial's
      opening frame sits behind the trainer hull with the belt and the ALPHA
      mark already drawn.
- [x] A scatter unit test uses two radii that the scalar rule passes and the
      pair rule rejects. Seeded radius draws remain stable across rejection
      attempts. `actions/spawn.rs`:
      `a_pair_of_bodies_is_measured_against_both_of_them`,
      `a_scattered_rock_keeps_its_size_however_crowded_its_field_is`.
- [x] Planet subdivision tests cover a small body, the common 800 m body, and
      the 79-subdivision cap. `objects/planet_surface.rs`:
      `a_planet_is_meshed_at_the_facet_size_not_at_a_fixed_count`,
      `planet_subdivisions_clamp_at_both_ends`.
- [x] Loading tests prove physics, input, camera, and scenario time remain
      frozen; progress resets the timeout; explicit failure and inactivity
      report `FAILED TO START`; successful release exposes frame zero.
      `loader/gate/tests.rs`:
      `a_scenario_load_holds_the_world_until_it_is_built`,
      `the_first_playable_frame_is_scenario_time_zero`,
      `a_failed_load_never_gives_the_world_back`,
      `leaving_a_failed_scenario_starts_the_world_again`,
      `nothing_the_player_drives_runs_while_a_load_is_held`;
      `loader/preload.rs`: `a_load_that_stops_moving_fails_closed`,
      `progress_buys_the_whole_budget_again`.
- [x] Generated base scenario timing remains byte-identical after replacing
      generic pacing constants with explicit authored delays - with one line
      of deviation, recorded under Generated output below.
- [x] Run only affected crate checks, content generation/lint, and relevant
      docs or web checks. Inspect generated output. See Checks below.

## Documentation

Check and update `CHANGELOG.md`, `docs/scenario-system.md`,
`web/src/wiki/keybinds.md`, `web/src/create/actions.md`, and
`web/src/create/objects.md`. Document the load freeze/failure contract, editor
camera behavior, and pair-radius scatter rule. There is no content format break.

## Implementation

- `nova_ship/src/camera/wasd.rs` carries the whole movement rule: a 60 m/s
  base, normalized input, an optional quadratic ramp (`WASD_ACCELERATION_SECS`
  2 s to `WASD_ACCELERATION_MAX` 32x) and optional FOV feedback
  (`WASD_FOV_FEEDBACK` 8 degrees, easing back over `WASD_FOV_EASE_SECS`).
  `accelerate` and `fov_feedback` are independent per-camera flags.
  `wasd_controller.rs` stashes `WASDCameraProfile` so a camera keeps its
  settings across losing and regaining the rig. Editor and scenario cameras
  take the accelerated profile; `examples/playable/first_shift_map.rs` turns
  both off and keeps constant movement.
- `nova_editor`: `frame.rs` records the framing point and reach on every frame
  request; `node.rs` frames a node from its own subtree bounds and keeps minted
  ships in an automatic +X row (`AutoLayout`, `reflow_auto_ships`, 20 m between
  adjacent collider bounds) that a manual move permanently leaves;
  `preview.rs` merges rotated section half-extents and draws a rock at the same
  conservative pristine bound the spawn uses; `placement.rs` and
  `ui/inspector.rs` spend the stored framing reach on new objects and on drag
  scale (`reach / FRAMING_PIXELS`, floored at `POSE_STEP_FLOOR`);
  `scenario.rs` signs sandbox beacons from the furthest authored beacon and the
  live `signature_range_per_unit`; `gallery/` derives rows and columns from the
  viewport (capped 8x5) and stands the stage clear of the document's merged
  bounds.
- `nova_scenario`: `loader/gate.rs` is new and owns `ScenarioLoadGate`, the
  `ScenarioLoad` freeze hold, the settle-driven release and the fail-closed
  path; `loader/lifecycle.rs` gates gameplay input and camera control on
  `scenario_play_is_free` and opens the chase camera from the player spawn and
  a hull-derived framing distance; `loader/preload.rs` counts a 10 s
  NO-PROGRESS budget; `actions/spawn.rs` stores each scatter placement with its
  derived body radius and rejects at
  `max(min_separation, placed reach + candidate reach)`;
  `objects/planet_surface.rs` solves subdivisions for a 20 m facet target and
  clamps to 79.
- `nova_gameplay/src/freeze.rs` gained the `ScenarioLoad` owner.
- `nova_authoring`: `scenarios/pacing.rs` lost `REVEAL_GAP`,
  `INSTRUCTION_GAP` and `MID_GAP`; `scenarios/tutorial/mod.rs` names each delay
  beside the line it follows.

## Decisions made during work

- The gate registers `ClockFreeze` itself. The hold ledger belongs to
  `nova_gameplay`, which a loader-only rig (an example, an editor sandbox) need
  not carry, and a missing ledger made the hold observer fail parameter
  validation. `init_resource` is idempotent, so an app that does carry it keeps
  one ledger.
- `EditorProbe` gained `inside_origin`: where the node the probe is inside
  stands in the world. Every pose the probe reports inside a ship is
  ship-LOCAL, and a driven run that wants to aim at a face needs the ship's own
  origin to add to it. `system_ship_editor` was aiming at a world literal left
  over from the removed `SHIP_NODE_SPACING`, which held the pointer off-window
  for 20 s once ships stopped standing at x = 24. Found by flying the example,
  not by `cargo check`.
- The `WASDCameraProfile` stash uses `try_insert`. The commonest way a rig
  comes off is the camera being despawned - leaving the editor scene takes the
  whole stage with it - and `insert` errors on a despawning entity. Found by
  flying the example.
- `min_separation` keeps its format and its absolute centre-distance meaning.
  The pair rule is an EXTRA floor, so an authored value still spaces bodies
  that are small enough not to reach each other.
- The load gate and the loading panel read one condition
  (`scenario_has_settled`), piped rather than copied, so the panel cannot come
  down over a held world and the world cannot start behind a panel.

## Checks

- `cargo test -p nova_ship --lib`: 949 passed.
- `cargo test -p nova_gameplay --lib`: 316 passed, 1 ignored.
- `cargo test -p nova_editor --lib`: 504 passed.
- `cargo test -p nova_scenario --lib`: 414 passed.
- `cargo test -p nova_authoring --lib`: 95 passed.
- `cargo run content gen` then `cargo run content lint`: 0 errors, 0 warnings,
  0 findings, 9 scenarios balance-audited.
- `cargo fmt`.
- Flown on Xvfb :99: `system_ship_editor` (cycle complete, no panic, t=13.4s,
  exit 0), `system_menu_boot` (cycle complete, no panic, t=1.6s, exit 0), and
  the tutorial through `cargo run -- --scenario tutorial`.

## Generated output

- `assets/base/scenarios/tutorial.content.ron` is the only regenerated file
  that changed, on one line: `after: Some(8.399999618530273)` became
  `after: Some(8.4)`. The old figure was an `f32` sum of the generic constants
  widened to the `f64` the step delay is; the new one is the authored `f64`
  literal. The beat moves by 3.8e-7 s, which is under a microsecond and far
  under a frame. Every other delay in every other generated scenario is
  unchanged.
- `editor-gallery.png`: 4 columns by 3 rows at 1024x768, "17 parts page 1/2" -
  the layout the derived-grid test asks for at that viewport.
- `editor-stage.png`: stage, tree, gizmo and inspector all reading in meters,
  with Ship 2 standing clear of Ship 1 in the automatic row.
- The tutorial's opening frames show the chase camera behind the trainer hull
  with the belt, the ALPHA mark at 900 m and the HUD complete on the first
  visible frame. Two frames taken seconds apart hold the same scene: nothing
  popped in behind the panel.

## Residual risk

- The preload budget is now unlimited while progress continues. A mod whose
  asset never settles and never errors would hold the panel until the 10 s
  no-progress budget expires, which is the intended trade for a slow network
  load that is still moving.
- The gallery grid caps at 8x5. A display wider than 4K gains no further tiles.
