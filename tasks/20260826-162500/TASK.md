# Review fixes: green the gates, make the document round-trip

- STATUS: CLOSED
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

## Landed

1. `271016ef` - the gates. 15 clippy errors, not the 11 the review reported:
   the earlier count came from a wasm invocation without `--all-targets`, which
   never reached the test targets. `catalog_drift` roster set to 45 slugs,
   `SYSTEMS_INVARIANTS` 145 -> 173. Workspace clippy, wasm clippy,
   `catalog_drift` and `cargo fmt --check` all exit 0.

2. `4ef3c31b` - lint on load. `on_load_scenario` lints the config it is handed
   and refuses on any Error, whether or not the merge ever saw the scenario.
   Two tests: a scenario the merge never saw is linted on load, and a range
   that chains to itself still starts.

8. `6dcaf6e9` - the comment convention in `AGENTS.md`.

3. Save and open are inverses. `Range` gained `flight`, which is the one thing
   Play and a save disagree about. A save spawns the document and nothing else;
   Play adds the hull an unfinished document has nothing to fly and skips a
   ship node with nothing built on it. `sandbox_events` derives its picket
   wakes, beacon sky swaps and death retries from the objects actually spawned,
   so a deleted picket takes its handlers with it instead of leaving a filter
   the loader now refuses. The second Player ship is refused on the driver row
   through `EditorSays::refuse`, naming the ship in the way and the way out -
   the status line rather than a greyed segment, because that is where the
   editor already says no and one segment of a segmented control has no greyed
   state. Add Ship already gave the second ship an AI pilot
   (`placement.rs:176`), so the row was the only way in.

   Proof: `the_script_only_names_what_the_range_spawns` (three ranges: flown,
   unflown, pickets and beacons deleted), `a_save_never_invents_the_player_ship_the_document_lacks`,
   `a_ship_with_nothing_built_on_it_survives_the_file`,
   `a_second_ship_cannot_take_the_controls_while_another_flies`. Each was
   mutation-checked against the old behaviour and failed. 306 `nova_editor` lib
   tests pass. `system_ship_editor` run live under Xvfb: 45 steps, exit 0, no
   panic - the saved document still reopens as 10 ships and 6 objects, which is
   the unchanged path for a document that HAS a player ship.

4. Reload becomes a restart. `ReloadContent` now means "come back up on the
   content that is on disk": `restart_for_content` re-reads the mod index and
   every bundle and content file, then sets `GameAssetsStates::Loading` and
   `GameStates::Loading` - the boot path, with the boot loading screen over it,
   ending in the main menu. `ContentReload`, `ReloadPhase`, `raise_reload_cover`,
   `settle_reload`, the three `COVER_*` constants, `nova_core`'s
   `sync_reload_screen` / `ReloadScreenMarker` and the 0.6s wall-clock sleep in
   `reload/tests.rs` are gone. A file that never comes back is now the boot
   path's problem, which already names it: `GameAssetsStates::Failed`.

   The ways in are exactly the four the review settled on: F5 gated on
   `GameStates::MainMenu`, `on_mods_back` (unchanged), and `OnExit(Playing)` in
   `NovaMenuPlugin`, which covers leaving a scenario AND leaving the editor for
   the menu. F1 out of the editor to the range does NOT restart - it never
   leaves `Playing`.

   Consequence, and it reverses a v0.11 behaviour: the editor's save no longer
   asks for a reload. Restarting a builder mid-build to publish their own save
   is worse than the wait, and the Scenarios picker is behind the way out
   anyway - so the save switches its own mod on and the restart on the way out
   reads it. `system_ship_editor`'s `a saved range is playable without
   restarting` beat is replaced by `a saved range is switched on for the way
   out`, and its four F5-cover beats are deleted.

   Known soft edge, recorded rather than hidden: `bevy_asset_loader` does not
   wait on the re-read, so `OnEnter(Processing)` can merge before a MOD's
   content file has landed (the shipped collection is fine - it is what the
   loading state waits on). `remerge_on_replaced_content` is kept for exactly
   that: a late file rebuilds the registries a few frames after the loading
   screen goes down. The old code's answer to this was the per-file counting
   the review flagged.

   Proof: 5 reload tests, including `a_reload_puts_the_game_back_through_the_boot_load`
   and `an_unasked_frame_leaves_the_game_where_it_is`. New live beats in
   `system_menu_boot`: F5 takes the menu down, the restart puts it back
   (`outcome: F5 restarts the game onto the content on disk`). Run under Xvfb:
   28 files re-read, `GameAssetsStates::Loading` entered and done, the menu
   backdrop scenario reloaded, back in the menu in 470ms, exit 0.
   `system_ship_editor` run under Xvfb: exit 0, no panic. Workspace clippy,
   wasm clippy, `catalog_drift` (174 invariants) and `cargo fmt --check` all
   exit 0. nova_assets 73, nova_core 9, nova_editor 306, nova_menu 80 lib tests
   pass.

5. Numbers that are not numbers, and one control too many. `write_field` now
   refuses a value that is not FINITE before it refuses one under a floor, so
   `nan` and `inf` typed into Position/Rotation/Scale bounce with `finite`
   instead of writing a pose no transform can hold. `check_floor` and the new
   `check_finite` share one `as_number` helper. Separately, `aim` is off the
   Light picks: the node ROTATION already aims the light (`node.rs`), and the
   defect there was two controls on one output, not a missing rule.

   Proof: `a_number_that_is_not_finite_is_refused_wherever_it_is_typed` covers
   the pose axis and the asteroid radius, and asserts the pose is unchanged
   after the refusal and that a legal `-40` still writes.

7. Delete stands down while a rebind is armed. `rebind_armed` in `keybind.rs`
   is a run condition on both `delete_key` and `save_key`, next to the existing
   `typing_into_a_field`. Binding Delete to a part used to delete the part on
   the way in - the capture read the press and so did the tree, with nothing to
   undo it. The guard is a stop-gap and says so: the input-mode task
   (`20260826-162503`) gives the keyboard one owner at a time and deletes it.

   Proof: `del_does_not_delete_while_a_rebind_waits_for_its_key`,
   mutation-checked - with `rebind_armed` forced to `false` the part is deleted
   and the test fails. 308 `nova_editor` lib tests pass, `cargo check
   -p nova_editor --all-targets` and `cargo fmt --check` exit 0. Workspace
   Clippy skipped locally per the standing instruction; CI runs it.

6. Loose ends, one commit, each with a test that fails without it.

   - `hide_idle_scroll_bars` asks the pane itself instead of taking
     `max_scroll_y`'s unbounded default. That default answers "how far may this
     pane scroll", where an unmeasured node must not clamp; the bar was reading
     it as "scrolls forever" and standing full-track over a despawned pane.
   - One field holds the focus. `text_field_keyboard` reads `single_mut`, so a
     second `TextFieldFocused` - which `TextFieldFocused::at_end` lets anything
     insert - killed EVERY key in the app, not just that field's. The newest
     focus now wins and the one it displaces commits, exactly as a click into
     another field already did.
   - `ui_walk`'s parked-camera predicate and its assert read one rule. The
     predicate advanced on ANY camera and the assert then read the FIRST, so a
     walk could advance on one camera and fail on another.
   - `bug_sandbox_soak` acks its founding gesture. The move to empty space had
     no `until`, so the click could land where the pointer used to be - on the
     gallery it had just closed. New predicate `pointer_at` in `nova_autopilot`
     for that ack. Its two `EditorProbe` predicates also read the resource
     outright, which panics the run before the editor is up; both stall now.
   - `serialize_content` stopped cloning the whole document to serialize it
     (`&content.to_vec()`). The existing round-trip test proves the bytes are
     unchanged.
   - The `nova_autopilot` prelude gained the six names it was missing - `or`,
     `ui_node_present`, `pointer_pressed`, `pointer_released`,
     `AutopilotCompletionSystems`, `LoopRecorder` - plus `pointer_at`.
     `nova_debug` was reaching past the prelude into `predicate::` for four of
     them.
   - `widget_zoo` stopped adding `slider_self_update` a second time.
     `NovaUiPlugin` brings it, which the line above it already said.
   - `GizmoReach` keys its measurement on the node AND its scale. Measuring
     once is deliberate - a world-axis box grows as a hull turns - but the
     Inspector's Scale field resizes a node with the rig still up, and the rig
     kept the size the node used to be.

   Proof: `a_bar_over_a_pane_that_is_gone_is_not_painted`,
   `a_second_focus_takes_the_field_over_instead_of_killing_the_keyboard`,
   `the_position_ack_reads_the_pointer_the_backend_moved`,
   `resizing_a_node_resizes_its_rig`. The focus and gizmo tests were
   mutation-checked against the old behaviour and failed.

   Three stale walks found by RUNNING them, all older than this task and all
   fixed here rather than filed:

   - `bug_sandbox_soak` waited on `ui_node_present("Add Ship Button")` while
     that row lives inside the Add menu, which spawns its items when it opens
     (`5198d3de`). It had been stalling at that beat ever since.
   - `count_sections` in `ui_walk` and the soak's `the_ship_is_up` swept every
     `SectionMarker` in the WORLD. A document now opens seeded with the stock
     range (`0244191e`), whose hulks and pickets are hulls with sections, so
     `screenshot_editor` counted 45 sections against an expected 5 and the
     soak's founding ack was true before the click. Both read the probe's
     `ship` list now, which is scoped to the edit context and says so in its
     own docstring.
   - The soak's founding click at (760, 640) lands ON the inspector at 1024
     wide (rail 210 + inspector 300). Moved to (460, 660) and given the same
     `editor_placement_clear()` ack `ui_walk::found` already had.

   Live runs, all exit 0 under Xvfb on a profile sandbox
   (`XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`NOVA_MODDING_CACHE_ROOT` in the
   scratchpad, so the operator's enabled-mod set is not read or written):
   `bug_sandbox_soak` (46.9s, founds a real ship and soaks it),
   `screenshot_editor` (5 sections on the preview ship, gallery camera parked
   and verified), `system_ship_editor` (11.4s), `system_menu_boot` (2.4s).
   `cargo check --workspace --all-targets --features debug`, `cargo fmt
   --check` and `catalog_drift` exit 0. Workspace Clippy skipped locally per
   the standing instruction; CI runs it.

## Proof

Both gates run locally. A test that a saved document reopens as itself, and one
that a document with a deleted picket is refused at load rather than at play.
Live editor run for the reload path and the refusal UI - `cargo check` misses
both.

## Not in scope

Input modes, the UI responsiveness pass, the inspector field-rule system, and
the whole-tree comment sweep. The sweep is last, after all three tasks, without
a task of its own.
