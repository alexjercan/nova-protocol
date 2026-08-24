# Editor state audit - round 4 research for v0.12.0 planning, written 2026-08-24

Scope: crates/nova_editor as of master (post v0.11.0, released 2026-08-23).
All paths relative to the repo root. Line numbers checked against the working
tree on the audit date.

## 1. Map of crates/nova_editor today

Public surface: `NovaEditorPlugin` (crates/nova_editor/src/lib.rs:71) and
`EditorSandboxSystems` (lib.rs:65), exported via `prelude` (lib.rs:55-57).
Everything else is `pub(crate)`. The module map is the crate header
(lib.rs:1-14).

States: `ExampleStates { Loading, Editor, Scenario }` (lib.rs:79-85),
driven from `GameStates::Playing` + `GameMode` (lib.rs:140-161). The main
Update chain (pick -> pose -> preview -> ghost -> skin -> gizmos) is wired at
lib.rs:212-242, gated on `Editor` state and gallery closed.

In-memory ship representation: a hybrid, not a data model.
- `PlayerSpaceshipConfig` resource (src/config.rs:13-33):
  `sections: HashMap<Entity, SpaceshipSectionConfig>`,
  `inputs: HashMap<Entity, Vec<Binding>>`, plus `skin: bool` and
  `style: Option<String>`. It holds the exact serialized shape the scenario
  consumes on hand-off.
- The map is keyed by LIVE preview entity, and the section config's `id`
  field IS the entity id stringified (src/placement.rs:146-166). The preview
  entities are ad-hoc ECS children of one `SpaceshipPreviewMarker` root
  (config.rs:53-54), spawned through the single shared spawner
  `insert_preview_section` (src/preview.rs:35-73) with roles
  Section vs Display (preview.rs:16-25). So the authoritative state is the
  resource, but its keys are scene entities; the two are re-keyed together
  on every editor re-entry (`rebuild_editor_preview_on_enter`,
  placement.rs:258-317).

Build/palette UI (src/ui/):
- `setup_editor_scene` (ui/mod.rs:54) spawns lights, the WASD camera
  (`EditorCamera`, ui/mod.rs:90-118) and the 150 px left rail (RAIL_W,
  ui/mod.rs:41). Rail rows: src/ui/rail.rs:17-129.
- Attitude readout row: ui/mod.rs:196-207 (`AttitudeReadout`,
  config.rs:111). Play Button: ui/mod.rs:259. Placement status line:
  ui/mod.rs:287 (`PlacementStatus`, config.rs:105).
- Tool selection is the `SectionChoice` resource (config.rs:38-46:
  None / Section(id) / Delete), set by `button_on_setting` observers
  (lib.rs:203) and synced back to button highlights
  (`sync_tool_selection`, placement.rs:325-344).
- Keybinds: `EditorRebind` (src/keybind.rs:17), capture in
  `apply_section_rebind` (keybind.rs:197).

Gallery / section picker (src/gallery/):
- `GalleryState` (gallery/mod.rs:37-51): open, category, filter,
  filter_focused, selected, focused. Registered at gallery/mod.rs:82-124.
- Catalog slice: `browsable(sections, category, filter)`
  (gallery/catalog.rs:73-91), honours `hide_in_editor`
  (nova_ship base_section.rs:272). Focus-card stats: catalog.rs:119-208.
- UI: `rebuild_gallery` (gallery/ui.rs:98), `paint_gallery_cells` (:610),
  one `GalleryAction` observer (:645, enum at :72-89:
  Open/Close/Back/Place/Focus/Category/Page/FocusFilter).
- 3D tiles: a stage parked at (0, 2000, 0) (gallery/scene.rs:26); the
  editor camera is parked there while open (`park_camera_for_gallery`,
  scene.rs:173). Turntable + zoom/orbit: `FocusView` (scene.rs:76),
  `pose_focused_item` and `drive_focus_view` (registered
  gallery/mod.rs:102-121).
- Keyboard: Tab toggle, Q pipette-pick, / focuses filter
  (gallery/input.rs:23-31, toggle :35, `gallery_keyboard` :55).

Link-point placement:
- One solve per frame: `update_placement_preview` (placement.rs:484-548)
  writes `PlacementPreview` (config.rs:76-80) holding `Placement`
  (config.rs:84-91: prototype id, target_section entity, solve).
- Solver: `snap::solve` (src/snap.rs:86-143), pure. Pose choices in
  `PlacementPose` (config.rs:66-72, socket offset + quarter-turn roll;
  cycled at placement.rs:386-440). `natural_source` picks the as-drawn
  socket (snap.rs:154-165).
- Refusals: `Refusal` enum snap.rs:36-51 - NoTargetSockets, NoPartSockets,
  Occupied, Ambiguous, Overlap, BlockedExit. Occupied-socket rejection is
  the mate scan in `refuse` (snap.rs:199-209); BlockedExit shares
  nova_ship's clearance rule (snap.rs:261-266).
- Ghost: `SectionGhost` (config.rs:95-101), shown/moved by
  `sync_placement_ghost` (placement.rs:570-668); status text + verdict box
  colours there; free sockets drawn by `draw_link_points`
  (placement.rs:812-890); heading arrow :909; delete outline :931.
- Commit: `on_click_spaceship_section` observer (placement.rs:719-801)
  places (commits the solved preview), deletes, or arms a rebind.

Play-test transition:
- Play button -> `continue_to_simulation` (placement.rs:346-351) ->
  `ExampleStates::Scenario` -> `setup_scenario` (src/scenario.rs:198-217)
  builds `sandbox_scenario` (scenario.rs:247-267) and triggers
  `LoadScenario`. The build state is lowered to a
  `ScenarioObjectConfig` in `player_ship` (scenario.rs:451-486): inline
  `ShipHull` from `sections.values()`, input_mapping from `inputs`,
  skin/style carried. F1 returns (`switch_scene_editor`, lib.rs:342-356)
  and `rebuild_editor_preview_on_enter` re-lifts the surviving resource.
- The sandbox registers itself in `GameScenarios` at load and repairs
  itself after mod re-merges (lib.rs:102-116, scenario.rs:236-243).

Save/load: none. No file I/O, no RON serialization anywhere in
nova_editor (checked by grep). The only persistence is the resource
surviving state transitions within one process run.

## 2. Visibility audit

Everything the probe harness needs is `pub(crate)`:
- `SectionChoice` config.rs:38, `PlacementPose` :67, `PlacementPreview`
  :77, `Placement` :84, `SectionGhost` :96, `PlacementStatus` :105.
- `snap::Placement` (transform/source/target/refusal) snap.rs:69-78 and
  `Refusal` snap.rs:36 - both pub(crate).
- `GalleryState` gallery/mod.rs:37, `ExampleStates` lib.rs:80.
The crate's whole pub surface is two types (lib.rs:55-71).

What the harnesses do instead, today:
- examples/systems/system_ship_editor.rs: `SETTLE = 10` frames (:170) and
  `SHIP_SETTLE = 40` (:175). The comment at :162-168 states the fix shape
  and names task 20260824-011329. Proxies used because state is private:
  section-count deltas (`count_sections` :1138, stamps :1178), status-line
  TEXT scraping via `subtree_text(world, "Placement Status")` (:763-770),
  "arming proven only by the next click" (:270-273), gallery-open proven
  by `ui_node_rect(world, "Parts Gallery")` (:566-571).
- examples/screenshots/shared/ui_walk.rs: `STEP_DEADLINE_SECS = 30.0`
  (:32), `GESTURE_FRAMES = 12` (:58), plus `SETTLE_FRAMES = 30` from
  crates/nova_debug/src/harness.rs:127 (used ui_walk.rs:254 and
  throughout examples/screenshots/screenshot_editor.rs:115-268).
- Task tasks/20260824-011329/TASK.md wants a public answer to "is there a
  solved placement under the pointer, and what is it". Key detail from
  that task and confirmed in code: the status line only SPEAKS on refusal
  or on a solved mate; with nothing under the pointer it is hidden
  (placement.rs:600-602), so from outside the crate "ready" and "nothing
  under the pointer" are indistinguishable.

Minimum to expose (my read):
- The placement answer: either promote `PlacementPreview` + `Placement` +
  `snap::Placement` + `Refusal` to pub and export via prelude, or (less
  API surface) add one public read-only resource, e.g.
  `EditorPlacementProbe { None | Solved { prototype, target, refused:
  Option<&'static str> } }`, written at the end of
  `update_placement_preview`. The second keeps solver internals private.
- `SectionChoice` (or a bool "a part is armed") - the harness currently
  proves arming by side effect.
- `GalleryState.open` (or a public run-condition) - currently proven by a
  named UI node existing.
- A "section landed" signal is already observable (SectionMarker count),
  so the count wait can stay; it just needs the pre-press wait to be
  "placement solved and not refused" instead of `frames(SETTLE)`.

## 3. Gap analysis against the node model

(a) Edit contexts (enter/exit a ship):
- Today there is exactly one implicit context: a single
  `SpaceshipPreviewMarker` root. `on_click_spaceship_section` takes
  `Single<Entity, With<SpaceshipPreviewMarker>>` (placement.rs:722), the
  preview solver takes `Single<&Children, ...>` (placement.rs:486), the
  skin sync takes a single root (skin.rs:73), and the build state is a
  SINGLETON resource. Two ships in the scene would break all of these.
- Nothing models "inside ship X" vs "in the world": `ExampleStates` has
  only Loading/Editor/Scenario (lib.rs:79-85).
- Needed: build state as a component on each edited root (the
  `PlayerSpaceshipConfig` shape is fine; its residence is wrong), a
  current-context handle the systems filter on, and camera/rail scoping
  per context. The entity-keyed map already anticipates this.

(b) Prefab instancing:
- The runtime reference form ALREADY exists and is the decided lowering
  target: `ShipSource::Prototype(ShipId)` resolved against `GameShips`
  (crates/nova_scenario/src/objects/ship.rs:118-143), with per-spawn
  structure overrides via `ShipSectionModification` (ship.rs:152-159).
  `SectionSource::Prototype` exists one level down
  (nova_scenario/src/objects/spaceship.rs:195).
- The editor writes only `SectionSource::Inline` (placement.rs:159) and
  DROPS non-inline sections on rebuild (placement.rs:302-307). It never
  reads or writes `GameShips`.
- Missing: a ship-level gallery (task 20260812-131901, OPEN backlog p0;
  the parts gallery scaffolding - stage, tiles, filter - is reusable but
  `browsable` is section-typed, catalog.rs:73), a stamp action producing
  a `ScenarioObjectConfig` with `ShipSource::Prototype`, in-scene
  duplication, and an editor-side notion of "instance with overrides".

(c) A "world node" for non-ship objects:
- The editor edits no world at all. The sandbox range is code-authored
  constants (scenario.rs:34-196) baked into `sandbox_scenario`.
- The target vocabulary exists: `ScenarioObjectKind`
  Spaceship/Asteroid/Beacon (nova_scenario/src/actions/spawn.rs:112) plus
  areas/events. Task 20260714-081703 (in-editor scenario builder, OPEN
  backlog, two spikes attached) is the umbrella for this.
- Missing: select/place/move for non-ship objects, a world context (see
  (a)), preview spawners for asteroids/beacons (the section preview path
  is ship-only), and gizmos for trigger radii.

(d) Lowering to ScenarioConfig RON and re-lifting:
- Lowering half-exists: `player_ship` + `sandbox_scenario`
  (scenario.rs:247-486) is precisely "editor state -> ScenarioConfig",
  today aimed at LoadScenario instead of a file. `ScenarioConfig` is
  serde-round-trippable through the mod pipeline
  (`Content::Scenario` / `Ship` / `Section`,
  crates/nova_modding/src/lib.rs:71-91; ron decode :206).
- Re-lifting does not exist: `rebuild_editor_preview_on_enter`
  (placement.rs:258-317) lifts only the in-memory resource, and it
  discards Prototype-sourced sections - the direct blocker for loading
  anything that references the catalog.
- Identity blocker: section config ids are stringified live Entity ids
  (placement.rs:155-158) and the scenario input_mapping is keyed by them
  (scenario.rs:463-466). Entity ids do not survive a process restart, so
  a saved file needs stable authored/sequential section ids before any
  save ships.

## 4. Staleness check of tasks/20260812-131912/TASK.md

The epic is OPEN, backlog p0, and its body is two release cycles old.
What actually landed vs what it assumes:

- Deferral header (TASK.md:7-26) says the editor is "the release after
  v0.11.0". v0.11.0 shipped 2026-08-23 (CHANGELOG.md), so the epic is
  due - but its premise "the sandbox is unplayable" is resolved; the
  performance release happened.
- Phase 0 (evidence): 20260805-015136, 20260806-140928, 20260804-190142
  all CLOSED. Stale in the other direction: the evidence is NOT currently
  trustworthy - `system_ship_editor` fails on CI software rendering
  (tasks/20260824-011329/TASK.md:24-27). v0.12.0 has a new Phase 0.
- Phase 1: 20260812-131852 (gallery picker) CLOSED - landed as commit
  4e89ae8a plus follow-ups; "dropdowns" became a category row + text
  filter + focus turntable. 20260804-134347 CLOSED.
- Phase 2 step 6: 20260812-131005 (link-point snapping) CLOSED - commits
  82c96801 ("Build ships by mating sockets") and 7ac30a09 ("Mate parts by
  frame"); rolled placement, real-mesh ghosts and occupied-socket
  rejection all exist (section 1). BUT its "save and reload parity"
  bullet only ever landed as in-session parity (rebuild on re-entry);
  disk save/load never existed. Step 7: 20260813-224826 CLOSED
  (0ee9cbb0), as the deferral note records. Step 8: 20260812-131901
  (prefab palette) OPEN, untouched.
- Phase 3: 20260714-081703 OPEN. Phase 4: 20260714-001140 OPEN.
- 20260714-204219 (baseline editor) CLOSED long ago (v0.6.0), and its
  centrepiece - the component drawer - was since REMOVED and replaced by
  the gallery (ui/mod.rs:6-9). Any epic reasoning built on the drawer is
  stale.
- Biggest structural staleness: the epic predates the node-editor
  direction and the decision that the graph is editor-internal and
  lowers to flat ScenarioConfig RON. Its Phase 2/3 checkpoints
  ("save -> reload -> play", "stamped and duplicated") are still the
  right outcomes, but the epic frames them as gallery+placement
  extensions rather than as the context/instancing/world model of
  section 3.
- The "Added 2026-08-19" section is partially overtaken: the attitude
  readout it demands ALREADY SHIPPED as the panel's first tenant
  (20260819-140314 CLOSED; crates/nova_editor/src/attitude.rs; rail row
  ui/mod.rs:196-207). The epic reads as though nothing exists yet.

## 5. The engineer readout

Where the model lives:
- `AttitudeEnvelope` in crates/nova_ship/src/physics/attitude.rs:56-131:
  `ceiling()` :91 (the alpha ceiling), `binds()` :97 (Torque vs
  Structure, enum :26-34 with `label()` :39), `sustained_turn_rate()`
  :108, `available(spin)` :121.
- Helpers written explicitly for a physics-free preview:
  `structural_arm` :146-166 and `hull_mass_properties` :175-186 (the
  comment at :169-174 names the editor as the caller).
- Editor side: `preview_envelope` crates/nova_editor/src/attitude.rs:18-70
  assembles the envelope from the build state (mass properties :39-46,
  summed controller torque :57-63); `readout_line` :80-90 prints
  "Turn X rad/s2 / <limit>"; painted every frame by
  `sync_attitude_readout` :97-108 into the rail row (ui/mod.rs:201-203).

What a build-screen stats panel could source TODAY (all from
`PlayerSpaceshipConfig` + `GameSections`, the same inputs
`preview_envelope` already takes):
- alpha ceiling + which limit binds: already on screen.
- time to turn (bang-bang 180): `2 * sqrt(pi / ceiling())` - the formula
  is already pinned in attitude.rs tests (:250, :262); one line to add.
- mass and centre of mass: computed and DISCARDED in `preview_envelope`
  (attitude.rs:39-46 uses only inertia and COM); returning
  `MassProperties3d` alongside the envelope is free.
- thrust: sum of `ThrusterSectionConfig.magnitude`
  (nova_ship/src/sections/thruster_section.rs:40) over the build state's
  thruster sections; per-axis split via `exit_normal` (forward is -Z,
  see the heading-arrow note placement.rs:903-907).
- max acceleration: thrust / mass, from the two above.
- weapon totals (count, damage, ammo, reload): per-part lines already
  exist in the gallery focus card (`stats`, gallery/catalog.rs:119-208);
  a ship-level sum is aggregation only.

What is MISSING a model entirely:
- power: no power/energy stat exists anywhere in nova_ship's section
  configs (checked by grep). A power line needs the gameplay stat
  invented first.
- weapon coverage: per-joint rotation limits exist
  (nova_ship/src/sections/turret_section/config.rs:71, :77 - TurretJoint
  lower/upper limits on the yaw/pitch tree), so per-turret arcs are
  derivable, but nothing computes coverage; a hull-shadowing /
  solid-angle union is genuinely new work.

Not confirmed: I did not run anything; all of the above is from reading
source. I also did not audit nova_debug's harness internals beyond the
constants cited.

## What this means for v0.12.0

Ordered, each item unblocks the ones after it:

1. Do task 20260824-011329 first. Expose the placement answer (a small
   public probe resource is enough; section 2 lists the exact types),
   convert the three editor ranges to state waits, delete SETTLE. It is
   simultaneously the CI fix, the new Phase 0, and the first proof that
   the editor's state is inspectable data - the node model's premise.
2. Move the build state from the `PlayerSpaceshipConfig` singleton to a
   per-root component with a "current edit context" handle. This is the
   enter/exit-a-ship enabler; every single-root assumption in section
   3(a) is the checklist.
3. Give sections stable ids at placement time instead of stringified
   Entity ids (placement.rs:155-158). Small, mechanical, and a hard
   prerequisite for any save format.
4. Save/load: factor the existing lowering (`player_ship` /
   `sandbox_scenario`) into "context -> ScenarioConfig", serialize with
   the serde impls the mod pipeline already uses, and write the lift
   (extend the rebuild path to consume a ScenarioObjectConfig and
   resolve `Prototype` sources via GameShips instead of dropping them).
   No format break: prefab = `ShipSource::Prototype`, exactly as decided.
5. Prefab instancing (20260812-131901): ship gallery over `GameShips`
   reusing the parts-gallery stage; stamp = scenario object with a
   Prototype hull; duplicate = clone the config. Depends on 2-4.
6. World context first slice (20260714-081703): place/move/save
   asteroids and beacons in a "world" edit context, round-trip through
   the same save. Events/objectives are a later slice.
7. Engineer readout: extend the existing rail block (ui/mod.rs:196-207)
   with flip time, mass, thrust and max acceleration - one small task,
   the data is already computed or one sum away. Treat power and weapon
   coverage as separate model spikes; do not let them gate the panel.
8. Refresh the epic (20260812-131912): mark phases 0-2 landed, fold in
   the node-model decision and this file, and re-scope the remaining
   children around items 2-6.
