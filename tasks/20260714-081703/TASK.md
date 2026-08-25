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
