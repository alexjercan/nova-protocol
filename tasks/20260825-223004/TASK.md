# Editor save/load: lower the document to a mod bundle, re-lift it

- STATUS: CLOSED
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

## Progress (2026-08-25): landed on master

Two commits, `c7b23350` (the feature) and `ec976236` (the driven proof).

### What a save is

`editor_save.content.ron` in the mod cache, beside its `.bundle.ron` and in
`installed.mods.ron` - an ordinary enableable mod, not a private format. Every
user-built ship is a `Content::Ship` design under its node id; the world is one
`Content::Scenario` whose ship objects hold `ShipSource::Prototype("ship_1")`
plus a pose and a controller, and nothing else. A design is written once and
referenced, so there is no copy on an instance to drift from it.

The stock range's own ships stay inline, because the editor did not write them
and cannot offer to edit them.

### The convention, both ways

The editor owns the LAYOUT: OnStart `SpawnScenarioObject` spawns lower from
nodes and lift back into nodes. The SCRIPT is re-derived from constants at every
save. That is what makes save -> load -> save byte-stable, and it costs exactly
one thing, stated in `bundle.rs`: a hand edit to the script does not survive a
re-save.

### Decided while building

- **One save slot.** `SAVE_MOD_ID = "editor_save"`. Read-only for hand-written
  mods is structural rather than a check - the editor cannot address another
  mod id, so it cannot overwrite one.
- **The saved sky is a path, not a handle.** `range_scenario` took the live
  cubemap `Handle` at first and the round-trip test caught it: an
  `AssetRef::Handle` has no authorable path and refuses to serialise. Play
  passes the handle, a save passes `DEFAULT_SKY`.
- **`resume_ordinal`.** A loaded document sets `NextChildOrdinal` above the
  `_{n}` suffixes it read, on the scenario node and on each ship, so a node
  minted after a load cannot collide with one that came off disk.
- **A second status slot.** `EditorStatus` holds the placement readout and what
  a verb said for four seconds. Save and Open had no other visible outcome.
  A narrow down-payment on step 6 of the polish plan.
- **The clock lives in `sync_status_line`.** It expires a message; the line and
  the probe both just read, so no reader needs a `Time` of its own.

### Against the Done-when list

- **Round trip.** Driven range at 1024x768: 16 nodes stamped and saved,
  `opened the saved document - 2 ship(s), 14 object(s)`, `reopened 16 node(s) on
  the same ids and poses`. A walk cannot restart the process, so File > New
  Scenario stands in for one - it reseeds the stock range, which has no `ship_2`
  in it, so a `ship_2` after the Open can only have come off disk.
- **Byte-stable re-save.** `a_re_save_writes_the_same_bytes`. Every map the
  lowering touches sorts on the way out.
- **Entity-independent ids.** `minted asteroid_5 beside the loaded ids`, above
  the loaded `hulk_4`, plus a beat asserting no id appears twice.
- **Hand-written mods read-only.** Structural, see above.
- **The diff test (Godot #67884).** `editing_a_design_leaves_its_instances_
  untouched`: edit the design, re-lower, the design's bytes changed and the
  range's did not. In this format an instance carries no overridable copy at
  all, so the drift the lesson is about has no room to happen.
- **Walk covers save and reload; probe green.** Six new beats before Play, so
  Play flies the RELOADED document and every hand-off assert after it - whole,
  clad, the second ship, the same 7-mate graph - is an assert about what came
  back off disk. `cycle complete, no panic`, EXIT=0.

228 unit tests in `nova_editor` (13 new), 1 new in `nova_modding`. Per the
standing instruction the local full suite and Clippy were not run; CI covers
them.

### Known loss

An AI ship's key bindings are not written. A spawned AI ship has no input
mapping to write them into, so the file has nowhere to put them. Stated in the
module rather than left to be discovered.

## Resolution (2026-08-25, done)

The Done-when list holds. Save and load are on master and driven.
