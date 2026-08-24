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
