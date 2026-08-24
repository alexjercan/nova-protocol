# Editor save/load: lower to a mod bundle, stamp prefab ships

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.12.0,editor,scenario,modding

Child of the v0.12.0 editor epic (`20260812-131912`), third in the spine
after foundations. Absorbs closed `20260812-131901` (full-spaceship
copy-paste palette) - nothing cancelled, the stamp/duplicate scope lives
here. Research: `tasks/20260815-231945/EDITOR-STATE.md` sections 3b/3d,
`SCENARIO-PIPELINE.md` sections 1-2, `NODE-EDITOR-PRIOR-ART.md`
recommendations 1-6.

## Goal

Save = lower the editor document to a `*.content.ron` mod bundle on disk;
load = re-lift it. Plus the prefab loop: browse complete ships in the
gallery, stamp instances into the scene, duplicate an in-scene ship.

## The settled shape

- The save file is `Vec<Content>`: user-built ship designs as
  `Content::Ship` prototypes, the world as `Content::Scenario` whose ships
  are `ShipSource::Prototype` references (crates/nova_modding/src/lib.rs:71-92).
  Editing a design propagates to every instance through the reference; the
  RON never stores copies. "Export my ship" falls out for free.
- Instance ids are minted LITERALS at instance creation (`corvette_1`),
  stored in the document, never re-derived at save. Duplicate spawned ids in
  one handler are a lint Error (lint/scenario.rs:142-152).
- Re-lift by convention: OnStart `SpawnScenarioObject` handlers are layout;
  spawns in other handlers are logic and re-lift as opaque handlers. No
  sidecar file in v0.12.0.
- Hand-written mods open READ-ONLY: a serde round-trip destroys their
  comments, and the ledger campaign is the best authored content the project
  has.

## Known facts from the audit

- There is NO runtime save path anywhere today; only `content gen` writes
  scenario files. This is new capability, not wiring.
- The whole config tree round-trips through RON - proven by
  `a_scenario_config_round_trips_through_ron` (nova_scenario
  loader/mod.rs:579-724).
- The lowering half-exists: `player_ship` / `sandbox_scenario`
  (nova_editor/src/scenario.rs:247-486) is editor state -> ScenarioConfig,
  aimed at LoadScenario instead of a file. Factor it into
  "context -> config", then serialize.
- The re-lift blocker: the editor writes only `SectionSource::Inline`
  (placement.rs:159) and DROPS Prototype-sourced sections on rebuild
  (placement.rs:302-307). The lift must resolve `Prototype` via `GameShips`
  instead of dropping.
- Determinism: the current lowering iterates HashMaps
  (scenario.rs:478 `sections.values()`; input_mapping likewise). Sort
  everything before serialising or every save diffs spuriously.
- Test the diff (Godot #67884 lesson): lower, edit the source design,
  re-lower, assert only overridden fields survive on instances.

## The prefab loop (ex-20260812-131901)

- Ship gallery tab over `GameShips`: the parts-gallery stage, tiles, filter
  and turntable are reusable, but `browsable` is section-typed
  (gallery/catalog.rs:73) - generalise or sibling it. Assembled preview,
  name + section-count/mass readout.
- Stamp: place a `ScenarioObjectConfig` with a Prototype hull at
  cursor/anchor; repeated stamping mints fresh instance ids.
- In-scene duplicate: clone the config under a fresh minted id.

## Done when

- Build a ship, save, restart the app, load: identical editor state,
  entity-independent ids, byte-stable re-save.
- Open the ship gallery, stamp two copies, duplicate an in-scene ship, save,
  reload: all copies intact; play works.
- A UI-harness walk covers the loop; probe green.
