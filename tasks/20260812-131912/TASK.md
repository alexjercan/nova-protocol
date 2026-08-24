# Epic: the node editor is the star of v0.12.0

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.12.0,epic,editor

Rewritten 2026-08-24 for v0.12.0 from the round 4 research. The plan of
record is `tasks/20260815-231945/V0.12.0-PLAN.md`; its five evidence files
carry the code-level detail. The previous body (the v0.11.0-deferred phased
plan) is superseded - phases 0-2 of it landed and closed in v0.10/v0.11, the
component drawer it reasoned about was removed, and the attitude readout it
demanded already shipped as the rail row's first tenant. See git history for
the old text.

## The release story

Godot-style node editing, owner direction 2026-08-24: build a ship as a
scene, "go into" it to edit, "go out" and stamp it as a prefab instance in
the world, place non-ship objects around it, wire a scenario, save, and play
it. The whole thing is a graph of entities and components - and the graph is
EDITOR-INTERNAL. Saving LOWERS it to the flat `ScenarioConfig` RON the
runtime already loads. No format break.

## Settled design (owner + round 4 research)

- **The editor saves a mod bundle.** `*.content.ron` is already
  `Vec<Content>`: user-built ship designs lower to `Content::Ship`
  prototypes, the world to `Content::Scenario` whose ships are
  `ShipSource::Prototype` references. Editing a design propagates to every
  instance through the reference - Godot semantics in the editor, plain RON
  on disk. "Export my ship" falls out for free.
- **Instance ids are minted literals** at instance creation (`corvette_1`),
  stored in the document, never re-derived at save. Keeps `content lint`
  decidable and saves byte-stable.
- **Property-level overrides only** (`ShipSectionModification` exists);
  structural overrides inside an instance are out for v0.12.0 - they are the
  documented data-loss source in Godot and Unity both. If needed later, a
  one-way "make local" inlines the sections.
- **Re-lift by convention, no sidecar**: OnStart `SpawnScenarioObject`
  handlers are layout, everything else is logic. Sort every map before
  serialising. Hand-written mods open READ-ONLY (round-trip destroys their
  comments).
- **Enter/exit = switch the edit root**, relax validation inside, validate at
  the boundary (Godot edit context + Cosmoteer blueprint mode).

## Children, in order

The dependency spine; each unblocks the next:

1. `20260824-011329` (p95) - editor state probe + wait predicates. The CI
   fix AND the node model's premise: editor state is inspectable data.
2. `20260824-120520` (p85) - foundations: per-root build state, edit
   contexts, stable section ids.
3. `20260824-120524` (p80) - save/load + prefab stamping: lower/lift the mod
   bundle, ship gallery, stamp, duplicate. Absorbs closed `20260812-131901`.
4. `20260714-081703` (p75) - world editing: non-ship objects, areas, then
   objectives/events slices.
5. `20260824-120535` (p50) - engineer readout: flip time, mass, thrust, max
   acceleration on the existing rail block. Factorio: the consequence of a
   choice is visible at the moment the choice is made.

Running beside the spine, same release: `20260820-223059` (once/Sequence),
`20260820-174148` (game as a process; its bindings registry feeds the
settings task), the settings overhaul, thrusters 1x1 + multi-cell, vacuum
VFX. The board with priorities lives in V0.12.0-PLAN.md.

## The one big single-root debt

Today the editor has exactly one implicit context: a singleton
`PlayerSpaceshipConfig` resource keyed by LIVE preview entities, and
`Single<SpaceshipPreviewMarker>` assumptions through placement, preview,
skin, and commit. Two ships in the scene would break all of it. The
foundations child owns this checklist (EDITOR-STATE.md section 3a).

## Release definition of done

- Every v0.12.0 child is closed or explicitly cut before tagging. The first
  cuts, in order: the readout panel, then the objectives/events slices of
  `20260714-081703`. Foundations and save/load are the core and do not get
  cut.
- The full editor player path - build, enter/exit, stamp, world, save,
  reload, play - has harness coverage and reviewable rendered output.
- Full correctness probe, affected content lint, Rust checks, and web CI
  pass.
- Editor, modding, scenario, input, and screenshot documentation ships with
  the behavior it describes.
