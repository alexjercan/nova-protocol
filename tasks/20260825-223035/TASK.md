# Editor events: the whole vocabulary, authored in the panel

- STATUS: CLOSED
- PRIORITY: 64
- TAGS: v0.12.0, editor, scenario, events

Successor to `20260714-081703` slice 3, and it now carries slice 2 as well:
`20260825-223024` (objectives and win/lose) is FOLDED IN. Its open question -
"is an objective a first-class config the editor edits, or a handler set it
authors" - is answered by doing this task: `Objective`,
`ObjectiveComplete`, `ObjectiveMarkerAttach/Detach` and `Outcome` are already
action arms, so an editor that authors the action vocabulary authors
objectives and win/lose for free. No second objective type is invented.

Both blockers landed: `20260820-223059` (`once` + `Sequence`) and
`20260825-223004` (save/load).

## Goal

The editor authors the SCRIPT, not just the layout. Every event, every filter,
every action - the closed vocabulary as it stands - is created, edited, saved
and reloaded from the editor, and plays through.

## The settled shape

- **Two tabs in the left rail.** `SCENE` is today's tree (ships, sections,
  world objects). `EVENTS` is the same kind of tree over the script: the
  scenario at the root, one row per handler, filters and actions as its
  children, and a `Sequence` action's steps as children of their own.
- **An event node is a document node.** `EventNode`, `FilterNode`,
  `ActionNode`, `StepNode` join `ShipNode` / `ObjectNode` under the scenario -
  same `NodeId` minting, same selection, same delete. The tree IS the config;
  lowering re-assembles the nested lists from the children.
- **The inspector edits them by reflection**, exactly as it edits a rock: the
  action vocabulary grows a `Reflect` derive, and `walk` does the rest. No
  match arm per action kind in the editor.
- **Ids autocomplete and validate.** An action field that names an object id
  is marked at the source (`nova_scenario`) with a reflect attribute; the
  inspector draws it as a reference row - a dropdown of the ids the document
  actually spawns, painted as a fault when the text names nothing.
- **Expressions get a text syntax.** The variables DSL is a recursive enum
  with fields, so reflection alone would draw a dozen rows for
  `picket_warden_awake == false`. `Display` + `FromStr` on the grammar makes
  it one text row that round-trips.

## What stays generated

The `OnStart` handler that spawns the world is DERIVED from the object nodes
and stays derived - it is the layout, and authoring it by hand would let the
tree and the script disagree. Everything else the sandbox scripts today (the
standing objective, the range briefing, the death/retry, the picket wakes, the
beacon sky swaps) becomes seeded event nodes a new document opens with.

## Done when

- The sandbox range's own script reads as event nodes in the EVENTS tab, and
  every one of them is editable.
- Author -> save -> reload -> play round-trips a scenario whose script the
  editor wrote, objectives and outcome included.
- A destroy-X / reach-Y / survive-T objective set is authorable with no code
  change, and completes its player path.
- A UI-harness walk covers it; probe green.

## What shipped

- Two rail tabs. `SCENE` is the world; `EVENTS` is the script - the scenario at
  the root, a row per handler, its filters, actions, sequence steps and gates
  under it. Selecting in one tab clears the other.
- The script IS document nodes: `EventNode`, `FilterNode`, `ActionNode`,
  `StepNode`, `GateNode` join `ShipNode` / `ObjectNode` under the scenario, with
  the same `NodeId` minting, selection and delete. `lift` takes a handler apart
  into nodes, `ScriptNodes::lower` re-assembles the nested lists.
- The Inspector edits a handler like a rock, by reflection: the trigger, `once`,
  and every field of every filter and action. The action and filter vocabulary
  grew `Reflect`; the editor has no match arm per action kind, only the
  `leaf_config` payload seam the compiler enforces.
- A filter or action is switched to any of the other 6 / 26 kinds from its own
  row, keeping the operands the new kind can hold.
- Add builds the script (Handler, Filter, Action, Step, Gate); a container
  arrives shut and two clicks open it; adding into a shut handler opens the way
  down to the new node.
- `Names` (`nova_scenario/src/names.rs`) marks at the SOURCE what a config
  string refers to. A reference row offers the ids the document actually spawns
  and reads `unknown` beside one that names nothing - the handler a save drops.
- Expressions have a text form (`nova_scenario/src/syntax.rs`):
  `scenario.elapsed > 90` in one row, round-tripping in both directions.
- Save carries the script and Open reads it back; only the `OnStart` handler
  that spawns the world stays derived from the object nodes.

## Decisions

- **The tree is the config.** A node holds what nothing else holds - a
  `Sequence` keeps its key and not its steps, an `And` keeps no config at all -
  so no save has to pick between two answers to the same question.
- **No declarative objective type.** The folded task's open question is answered
  by the action vocabulary: destroy X / reach Y / survive T is four handlers of
  arms that already exist.
- **Collapse uses the existing double-click gesture**, not caret buttons: a
  Button inside a Button makes picking ambiguous. Adding a node has to open its
  ancestors, because `sync_scene_list` clears a selection whose row does not
  exist.
- **The id picker reuses the colour window idiom**; both now share
  `window_frame`.
- **`AssetRef` reflects as OPAQUE**: a config's asset ref is one authorable
  path to whoever is looking at it, and nothing walks into the handle arm.
- **One lossy case, stated in `bundle.rs`:** an authored unfiltered `OnStart`
  that only spawns is read back as world objects, because that is what the
  derived spawn handler is.

## Proof

- `cargo test -p nova_editor --lib`: 378 passed, 0 failed. Includes
  `a_script_the_editor_wrote_survives_the_file` (author -> lower -> file -> lift
  equality), `a_handler_naming_nothing_the_document_spawns_is_dropped`,
  `the_objective_set_is_authored_as_nodes_and_plays_through` and
  `an_objective_stands_until_the_beat_it_names_happens` (both run the authored
  nodes through `GameEventsPlugin` and assert on `GameObjectives` and
  `CurrentOutcome`), and the tree tests for the caret, the two-click open and
  the id picker.
- `cargo test -p nova_scenario --lib`: 245 passed, 0 failed, including
  `tests::syntax` (7) and `tests::names` (3).
- Live walk under Xvfb: the permanent events walk in
  `examples/screenshots/screenshot_editor.rs` runs Events tab -> Add Handler ->
  Add Filter -> pick `player_spaceship` from the id window -> Add Action, and
  captures `feature-editor-events.png`. Two cycles, no panic; the second frame
  shows 13 shut handlers plus the authored one open, all inside the rail.
- `probe run screenshot_editor`: PASS on process_exit, run_completed,
  reached_playing (frame 21), invariants_held (0 violations over 478 frames),
  log_clean and artifacts_loadable.
- Docs: CHANGELOG (Interface & HUD, Modding, Internals), `docs/scenario-system.md`
  (the three authoring surfaces, `Names`, the text form),
  `docs/guide-extend-scenarios.md` (what a new filter or action owes the
  editor), `/create/expressions/` (the typed form) and `/create/author-a-scenario/`
  (the second way in).

## Done when - answered

- The sandbox script reads as event nodes and every one is editable: YES,
  `default_script` seeds them and the walk edits one.
- Author -> save -> reload -> play round-trips: YES, proved by the bundle
  round-trip test plus the two headless play-through tests. The live walk covers
  authoring, not the save/reload leg.
- A destroy-X / reach-Y / survive-T set is authorable with no code change: YES,
  `objective_set()` in `event/tests.rs` is exactly that, authored as nodes.
- A UI-harness walk covers it, probe green: YES, both above.
