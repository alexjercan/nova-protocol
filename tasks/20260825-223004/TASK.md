# Editor save/load: lower the document to a mod bundle, re-lift it

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.12.0,editor,scenario,modding

Child of the v0.12.0 editor epic (`20260812-131912`), third in the spine after
foundations. Replaces `20260824-120524`, which also carried the prefab loop;
that half is dropped from v0.12.0 (see the Resolution note there). Research:
`tasks/20260815-231945/EDITOR-STATE.md` sections 3b/3d,
`SCENARIO-PIPELINE.md` sections 1-2, `NODE-EDITOR-PRIOR-ART.md`
recommendations 1-6.

## Goal

Save = lower the editor document to a `*.content.ron` mod bundle on disk;
load = re-lift it. Ships and world objects both. No gallery tab, no stamping,
no duplicate - that loop is out of this release.


## The settled shape

- The save file is `Vec<Content>`: user-built ship designs as
  `Content::Ship` prototypes, the world as `Content::Scenario` whose ships
  are `ShipSource::Prototype` references (crates/nova_modding/src/lib.rs:71-92).
  Editing a design propagates to every instance through the reference; the
  RON never stores copies. "Export my ship" falls out for free. The prototype
  reference is a FILE-FORMAT decision and stays; only the UI that browses and
  stamps prototypes is dropped.
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

## Done when

- Build a ship, place world objects, save, restart the app, load: identical
  editor state, entity-independent ids, byte-stable re-save.
- A hand-written mod opens read-only and cannot be overwritten by a save.
- The diff test holds (Godot #67884 lesson): lower, edit the source design,
  re-lower, only overridden fields survive on instances.
- A UI-harness walk covers save and reload; probe green.


## Note (2026-08-25): the world is already nodes

`20260714-081703`'s first half landed on master: the sandbox range comes up
as `ObjectNode`s under the scenario node, seeded by `ensure_document` and
lowered back by `nova_editor/src/scenario.rs::lower_objects`. So the save
side has a document that already holds the whole world, not just the ships,
and "lower the context to a config" is written once for objects.

Two things that fall to this task rather than that one:

- The RON write/read itself, for ships AND objects.
- Instance ids: object nodes keep the id they were AUTHORED with when they
  come from the stock range (the sandbox's event handlers name
  `picket_warden`), and mint `{stem}_{n}` when placed. A save has to keep
  both, and a re-lift has to not re-mint.
