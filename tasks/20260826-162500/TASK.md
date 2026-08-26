# Review fixes: green the gates, make the document round-trip

- STATUS: IN_PROGRESS
- PRIORITY: 96
- TAGS: v0.12.0, review, editor, scenario, assets

Round 1 of `/nova-review` over `v0.11.0..HEAD`: 88 commits, 35,100 insertions,
13 reviewer lanes across 4 shards. 30 findings, 8 root causes. The rendered
design doc in this folder carries the reasoning; this file carries the work.

This task holds every review fix that is not a new subsystem. Input modes and
the UI pass are separate tasks and run after this one, in that order.

## Sub-tasks

1. Green the gates. `cargo clippy --workspace --all-targets --features debug --
   -D warnings` fails with 11 errors: 3 `doc_lazy_continuation` in
   `nova_assets/src/reload.rs:23-25`, and 8 in `nova_editor` (3 wasm-only dead
   code in `bundle.rs`, 5 target-independent in `node.rs:398`,
   `placement.rs:222`, `stage.rs:267`, `ui/callout.rs:185`, `ui/mod.rs:1118`).
   `cargo test -p nova_probe_cli --test catalog_drift` fails: `system_ship_editor`
   emits 45 markers against a 17-slug roster. Set the roster to the 45 current
   slugs and `SYSTEMS_INVARIANTS` to 173. Name the 4 markers that ride a step
   predicate rather than an assert in the roster docstring's existing list.

2. Lint on load. `on_load_scenario` lints the config it is handed instead of
   consulting `ContentIssues`, which the merge fills and the editor's Play path
   never enters (`nova_editor/src/scenario.rs:206-232`). Keep the merge pass, so
   a broken mod is still reported at startup and on reload. This is the proof
   harness for sub-task 3.

3. Save and open are inverses. Split fly-time composition from save-time
   composition in `nova_editor/src/scenario.rs`. `sandbox_objects:549` must stop
   pushing `player_ship` into a saved document and stop dropping standing ships
   on `!ship.sections.is_empty()`; `sandbox_events:1038-1039` must stop emitting
   `PICKETS` wake handlers and `SKY_BEACONS` sky swaps that name objects the
   document no longer contains. Refuse a second Player ship in the builder,
   greyed with a reason, following `ui/mod.rs:325-331`
   (`PLAY_BLOCKED = "Play (leave the ship)"`). Correct the `LiftedShip::pilot`
   docstring at `bundle.rs:134`, which claims the opposite of `bundle.rs:219`.

4. Reload becomes a restart. F5 only from the main menu, blocking into the real
   loading screen, the same on leaving the mods panel, the editor, or a
   scenario. Delete the `ContentReload` phase machine and its per-file counting
   - the cover exists only to hide a merge frame from a player mid-game, and a
   blocking load has no frame to hide. Keep a message naming a file that never
   came back. `reload/tests.rs:159` (a 0.6s wall-clock sleep) goes with it.

5. Precision fixes pulled forward from the UI task. Reject `NaN` and `inf` in
   the Position/Rotation/Scale boxes (`inspect.rs:305-323` returns no rule for
   `x`/`y`/`z`). Drop `aim` from the Light picks at `inspect.rs:1002` - the node
   rotation aims the light (`node.rs:325`) and two controls on one output is the
   defect, not the missing validation.

6. Loose ends. `screen/scroll.rs:217` full-track bar over a despawned pane;
   `text_field.rs:47,:283` `at_end` public with no caller and able to wedge all
   typing; `ui_walk.rs:110` vs `:128` reading different cameras;
   `bug_sandbox_soak.rs:226` unacked founding gesture and `:115,:121` panicking
   on a missing `EditorProbe`; `nova_modding/src/lib.rs:211` cloning the
   document on every save; the `nova_autopilot` prelude missing 6 exports;
   `widget_zoo.rs:90` re-adding `slider_self_update`. Plus `GizmoReach::measure`
   (`gizmo.rs:322`) re-measuring after an inspector resize - a cache keyed on
   the wrong thing, not a performance item.

7. Temporary guard for the rebind collision. `delete_key` (`placement.rs:276`,
   wired at `lib.rs:175`) is guarded only by `not(typing_into_a_field)` and the
   editor state, so pressing Delete to BIND Delete also deletes the marked
   section - silent, no undo. Add a predicate for an armed rebind here. The
   input-mode task deletes this guard and makes the collision unreachable.

8. Write the comment convention into AGENTS.md. It currently says only "Keep
   module comments short. Explain ownership and constraints, not code or
   history." Add the rule it is missing: code reads as documentation,
   docstrings on public items, in-code comments only where the reason is not
   obvious from the code. The convention drifted this cycle because it was
   written down nowhere.

## Decisions taken in review

- No second Player ship in the builder, refused with a reason rather than
  resolved silently.
- Content lint runs on load AND at startup/reload, as Wesnoth does it.
- F5 is a restart, not a live reload.
- The prototype focus-out fork (`inspect.rs:934`) is HELD: no prototypes exist
  and none are planned for v0.12.0, so the rewrite is inert. Revisit when
  prototypes land.
- `sections_of` (`node.rs:460`) and `report_duplicate_ids` (`node.rs:1209`) are
  HELD until a probe run says they cost something. The editor does not lag and
  nothing was measured in this review.

## Proof

Both gates run locally. A test that a saved document reopens as itself, and one
that a document with a deleted picket is refused at load rather than at play.
Live editor run for the reload path and the refusal UI - `cargo check` misses
both.

## Not in scope

Input modes, the UI responsiveness pass, the inspector field-rule system, and
the whole-tree comment sweep. The sweep is last, after all three tasks, without
a task of its own.
