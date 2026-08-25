# In-editor world editing: place scenario objects, wire objectives, save to RON

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.12.0,editor,scenario,modding

Rewritten 2026-08-24 for v0.12.0. Fourth in the editor epic's spine
(`20260812-131912`), on top of foundations (`20260824-120520`) and save/load
(`20260824-120524`). The old spikes (tasks/20260714-081636,
tasks/20260714-204059) and the old baseline/component-drawer framing are
historical context only - superseded by round 4 research:
`tasks/20260815-231945/EDITOR-STATE.md` section 3c,
`SCENARIO-PIPELINE.md` sections 1-2.

## Goal

The editor edits a WORLD, not just a ship: place, move and delete non-ship
scenario objects around the stamped ships, wire simple objectives and
win/lose, and round-trip it all through the same mod-bundle save. This is
the "world node" of the node model - the outermost edit context.

## What exists to build on

- The target vocabulary is complete: `ScenarioObjectKind` - Anchor,
  Asteroid, Spaceship, Beacon, SalvageCrate, Light
  (nova_scenario/src/actions/spawn.rs:112-127), plus `ScatterObjects`
  (seeded) and `CreateScenarioArea`.
- Objectives are POSTED, not declared: `Objective` / `ObjectiveComplete` and
  the marker attach/detach actions (actions/mod.rs:55-61).
- The editor edits NO world today: the sandbox range is code-authored
  constants (nova_editor/src/scenario.rs:34-196) baked into
  `sandbox_scenario`. Those constants become the default world document.
- The section preview path is ship-only; non-ship kinds need preview
  spawners, and trigger areas need gizmos.

## The lowering convention (settled)

The world context lowers to OnStart `SpawnScenarioObject` handlers - layout.
Spawns in non-start handlers are logic and re-lift as opaque handlers beside
the world. Instance ids are minted literals (see `20260824-120524`).

## Vertical slices, in order

1. **Objects + round-trip**: place/move/delete asteroids, beacons, salvage,
   anchors and lights in the world context; save; reload; identical.
2. **Objectives / win-lose**: attach a simple objective set (destroy X,
   reach Y, survive T) with the posted-objective actions; play it through.
3. **Events surfacing**: expose the event/handler list without overwhelming
   the panel. Lean on `Sequence` (`20260820-223059`) so a story beat is one
   handler, not nineteen. This slice and slice 2 are the epic's second cut
   line; slice 1 is core.

## Done when

- One authored scenario with stamped ships, non-ship objects and a simple
  objective saves, reloads, and completes its player path.
- The editor and hand-written mods still share one representation: the saved
  bundle loads through the ordinary mod pipeline, `content lint` clean.
- A UI-harness walk covers place -> save -> reload -> play; probe green.

## Progress: slice 1, first half (2026-08-25)

Landed on master in one commit. Objects are placeable and the sandbox range
IS the document now; the round-trip half of slice 1 (save/reload) is not
here - it belongs to `20260824-120524`, which now has world nodes to save.

What landed:

- `ObjectNode` beside `ShipNode` under the scenario node, carrying a whole
  `ScenarioObjectKind`. `ObjectChoice` is the five-kind palette with the
  stock config per kind.
- `insert_preview_object` in `preview.rs` - schematic bodies (sphere, cuboid,
  textured rock, emissive bulb, a Spaceship's own sections) plus ONE bounds
  collider per view root, which is what makes a world object pickable and
  keeps `node_of_view` pointing at the node.
- `ensure_document` seeds `default_world_objects()` under their AUTHORED ids
  (`picket_warden`, `beacon_veil`, ...), because the sandbox's own event
  handlers name them.
- `sandbox_scenario` takes the world as an argument; `world_objects` lowers
  the document when there is one and falls back to the stock range when
  there is not (registration runs at asset-load, before any document).
  Keyed on the document EXISTING, never on it being non-empty, so an emptied
  range stays empty.
- Rail: an "Add Object" palette above the Scene tree (the tree is the block
  that grows, so the actions cannot be pushed off a 768px screen). Top bar:
  Delete, greyed unless the selection is a ship or an object the scenario
  can lose.
- Drag and click-to-select generalised from ships to any staged node;
  entering a ship now takes the whole WORLD off the stage, not just the
  sibling ships.

Proof, from the driven range at 1024x768:

- `editor: back at the scenario node, listing ["ship_1", "ship_2",
  "beacon_home", "beacon_veil", "hulk_0", ... "sandbox_rim"]`
- `editor: placed asteroid_3 at Vec3(12.0, 0.58, 1.17)` then
  `editor: deleted the placed asteroid`
- `on_load_scenario: loaded scenario 'editor_sandbox' with 12 handler(s) and
  16 object(s)`, `autopilot: cycle complete, no panic`, EXIT=0

118 unit tests in `nova_editor` (9 new); `cargo check -p nova_editor --tests`
and `cargo check --examples --features debug` clean.

What is left in slice 1: save/reload (in `20260824-120524`), an inspector for
an object's own fields (radius, colour, mass - today a placed object keeps its
stock config), and gizmos for trigger areas.

## Next items: the Nova editor pass (2026-08-25)

Owner feedback on the built editor, against a reference shot of Jackdaw (a
generic Bevy scene editor). The steer: Nova Protocol's editor is NOT a generic
ECS editor. It edits SCENARIO OBJECTS. Anything that only makes sense for an
arbitrary Bevy app is out of scope.

Landed already this round (commits `5eb626b1`, `e0d9ddef`):

- Move/turn handles on the selected node; F, View > Frame Selection and a
  tree-row click all frame a node.
- Rig sizing measured once per selection - a world-axis `ColliderAabb` grows
  as the hull turns inside it, so per-frame sizing made the handles swell
  under their own turn ring.
- Ship heading survives the hand-off; the lowering hardcoded `Quat::IDENTITY`.

### Settled decisions

- **A ship's pose is its own.** Dropping the anchor rule: the player's ship no
  longer pins the fleet to `PLAYER_SPAWN`. Ships and the range are independent
  - a ship spawns where it was dragged, and moving it does not move anything
  else. Supersedes the "player anchors the range" convention in
  `lower_fleet`.
- **The Inspector is Nova-specific.** It presents a scenario object's OWN
  meaningful fields, not a reflection dump of its components. No generic
  "Add Component", no Materials/Resources/Systems tabs.
- **Out of scope, permanently:** a project-file tree and an asset browser.
  Too generic to be part of Nova. Revisit only if authored "templates"
  become a thing.
- **No scale.** Nova has translation and rotation; sections mate, they do not
  stretch. The transform block carries two rows, not three.

### Ordered slices

1. **Inspector as a Nova component view.** Full transform block (translation
   and rotation, no scale). Typed field editors per data type rather than one
   text box: a colour swatch that shows the colour, a bool toggle, an enum
   dropdown, a numeric field. Show the fields that matter per kind, grouped
   the way Nova thinks about them. Reach CHILD data: a turret's fire rate
   lives on a section, not on the node root, so the Inspector needs a way to
   present a node's parts as inspectable rows. That last piece is what makes
   this a Nova editor rather than a generic one.
2. **Floating windows.** A window host for things that do not belong in the
   rail, with the colour picker as its first tenant.
3. **Navigation and chrome.** Parts / Delete / Bind move out of the top right
   into the left menus. Entering a node ISOLATES it: only that node and the
   node above it are visible and selectable - selecting an unrelated object
   from inside a node makes no sense. Double-click on a non-ship scenario
   object does something rather than nothing. Icons per node kind, and hover
   to reveal.
4. **The stage.** A world grid. Gizmos for objects with no mesh of their own
   - lights first, then trigger areas (already owed by slice 1 above).
5. **More per-object settings.** The tail of "an inspector for an object's own
   fields" owed by slice 1; folds into slice 1 of this list once the typed
   editors exist.
6. **Keys for the verbs that have none.** Del deletes, and arms the delete
   brush inside a ship the way the menu row does. Every verb the menus carry
   either has a key or is deliberately mouse-only.
7. **Add obeys the context.** "Add" offers what can go INSIDE the node you are
   standing in: Ship and the object palette belong to the scenario node, and
   inside a ship the menu offers that ship's parts instead of nodes that
   cannot be its children.
8. **Polish pass, from a review.** A paired review of the editor - one pass
   from inside the codebase, one with fresh eyes against how Godot and other
   scene editors read - written up as a findings list before any of it is
   built. Known starters: icons where there is bare text, better wording, and
   a `Vec3` field that is three boxes (x, y, z) rather than one.

## Progress: the editor pass, slices 1-4 (2026-08-25)

Slices 1 to 4 of the list above are on master. Slice 4 is two commits:

- `38c8d28b` the world grid. Cells step by decades sized from what the camera
  LOOKS at, not from where it is; the centre snaps to a whole decade so the
  lines hold still; the origin carries its X and Z lines in the handle colours;
  the selection drops a plumb line and a footprint ring onto the plane.
  Scenario-node only, like the drag it explains.
- `320fc390` the volumes an object has no body to show: a beacon's or a crate's
  trigger sphere, a lamp's reach, a sun's direction. Lights draw in their own
  colour; a sun's arrow is screen-sized like the move handles, because a
  directional light has no size to be a picture of.

Both hang off `EditorOverlays` and off one new module, `nova_editor::stage`.
View > World Grid and View > Object Volumes turn them off.

Proof: 216 `nova_editor` unit tests (7 new across the two commits), the driven
`system_ship_editor` walk clean at 388 beats with no stall, and captures read
back by eye - the decade pass, the origin's Z line, both sky beacons' trip
spheres and the three-point rig's suns all draw where the document says.

The review asked for in slice 8 is done and lives beside this file:
`REVIEW-INSIDE.md` (43 findings from the code) and `REVIEW-OUTSIDE.md` (44
findings from the screens, against Godot/Blender/Unity habits). `BACKLOG.md`
merges the two, tags the 10 duplicates, and holds the ordered plan.

Slices 5 to 8 - the typed inspector, keys for the verbs, Add obeying the
context, and everything the review found - moved to `20260825-221015` (Editor
polish). This task keeps its own spine: save/reload (which lives in
`20260824-120524`), objectives and win/lose, and events surfacing. The last two
are HELD until `Sequence` lands in `20260820-223059`; surfacing nineteen
handlers where a story beat should be one is the wrong panel to build.
