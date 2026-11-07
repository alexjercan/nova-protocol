# Production-ready mod editor

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.17.0,editor,modding

## User facts

- The game editor becomes a production-ready editor, and a mod builder, not
  only a scenario builder.
- Entering the editor starts from a mod: create a new mod, then add scenarios,
  sections, and the other things a mod can contain, all in that one mod.
- Mods can be packaged and can depend on other mods.
- UX matches common engines (Unity, Godot): WASD navigation, orbit camera,
  normal camera, Q/W/E style tool selection for translate, scale, and so on.
- The normal camera has no acceleration.
- Nicer, informative UI panels: entity hierarchy of the scenario, the catalog,
  an assets panel, and the ship viewer, which is the scene again.
- Panels are not dockable. Docking is out of scope.
- The scenario event editor looks like Scratch: visual blocks with puzzle-like
  completion.
- Other asset builders exist, such as a ship section builder.
- Undo/redo style user experience, "no mistakes".
- Work happens in a separate Sprout branch.

Rejected: dockable panels. Asked for: "no mistakes" means every edit can be
undone and a save or leave never loses work silently.

Owner review of the shell and blocks, 2026-11-07:

- This is a migration to the new version, so old things may be removed.
- The old catalog was much better for sections than the new Catalog tab.
- Merge the two top bars into one.
- The events view needs no hierarchy: the blocks are enough.
- The right panel is not needed in events if values fill in on the block through
  pop-ups, as in Scratch.
- In events, Problems and Log are useful, and probably Assets.
- Assets should look like the old catalog, like files in Unity.
- Use space better: buttons are too big, and added bars and headers take too much
  space. Make it more production ready.

- Do not change the main menu much. The editor stays called "Sandbox" or "Editor".
- Creating a mod is simpler, like creating a project in Unity.

## Decisions

Approved by the owner:

- No dockable panels. The panels are fixed.
- One merged top bar. There is no second toolbar.
- Events has no Hierarchy and no right panel. Values are filled on blocks, with
  pop-ups.
- Problems, Log and Assets stay in Events.
- The old catalog is preferred for sections.
- Assets look like files, as in Unity.
- The main menu barely changes. The button stays "Sandbox".
- Creating a mod is simple, as a Unity project is.
- Old things may be removed in the migration.

Agent assumptions, not confirmed by the owner:

- "Old catalog" means the Tab gallery.
- The Inspector stays in Scene and in asset forms.
- Old `editor_*` saves keep opening.

## Agent findings

All line numbers are from `51397ef7a`. Read before building on them.

Document and undo:

- The ECS node tree is the only live document (`ScenarioNode`, `ShipNode`,
  `SectionNode`, `ObjectNode`, `EventNode`, `FilterNode`, `ExpressionNode`,
  `ActionNode`, `StepNode`, `GateNode`, plus `NodeId`, `NextChildOrdinal`,
  `Transform`, `AutoLayout`, `ChildOf`). Content exists only when lowered by
  `document_content` (`bundle.rs:173`). `NodeId` is unique per parent only.
- No undo exists. `ui/window.rs:253` says so; `ui/mod.rs:147` has greyed rows.
- No single mutation path exists. Write sites: `placement.rs:254,287,422,519,
  1043,1206`, `gizmo.rs:524`, `ui/inspector.rs:2471` (`EditTargets::edit`) and
  its callers `:2672,2725,2798,2822,2864,2889`, `ui/window.rs:708,801,1129`,
  bypasses `ui/inspector.rs:2916`, `event.rs:1787`, `event.rs:2125`,
  `ui/inspector.rs:2626`, `ui/mod.rs:1190,2218,2661`, `keybind.rs:352`,
  `generate.rs:132`, `node.rs:1017`, `bundle.rs:715,836`, `ui/files.rs:485`.
- `reflow_auto_ships` (`node.rs:845`) writes `Transform` without user intent,
  so `Changed<Transform>` is not an edit signal.
- Rebuilding through save/load per undo destroys entity identity, `EditContext`,
  `AutoLayout`, `Expanded`, and ordinals. Scrub and colour slider have no
  release hook.

Viewport:

- Camera is the shared `WASDCamera`/`WASDCameraController` in `nova_ship`,
  spawned at `ui/mod.rs:449`, with default `accelerate: true`
  (`nova_ship/src/camera/wasd.rs:110`). Pose writes remove and re-insert it:
  `node.rs:779`, `frame.rs:242,279`. Other crates use the rig, so do not change
  it. `frame_stage` must keep its pose; driven walks depend on it.
- Gizmo shows 3 arrows and 3 rings together, world axes, no modes
  (`gizmo.rs:138`). Hidden inside a ship (`:448`). Section pose is solved by
  the mating snap and must stay that way; hulls do not scale
  (`inspect.rs:3039`).
- Key collisions: Q is the pipette (`placement.rs:661`), R is roll and F socket
  while armed (`:601,:604`), W/A/S/D are live without RMB. Z, Y, E, Alt, MMB
  are unused. Selection is single (`config.rs:52`), no click-empty deselect.

UI and script:

- Raw bevy_ui plus `nova_ui` factories. No tree, dropdown, drag-and-drop, or
  X-scroll widget. `ui/window.rs:9` rejects stacked panels; this task reverses
  that. egui is debug-only; keep it out of `nova_editor`.
- `ScriptNodes`, `lift`, and `lower` already model the script tree. Shapes:
  hat `EventNode`; stack `ActionNode` Leaf; C-block Sequence/Cinematic with
  `StepNode` and `GateNode`; boolean `FilterNode` and compare `ExprKind`;
  reporter arithmetic `ExprKind` and `Value`. Legal nesting: `event.rs:2049-2119`.
- Blocks come from tables, not bare reflection: `ActionTag::ALL`,
  `EventConfig::ALL`, `FilterChoice::ALL`, `ExprChoice::ALL`, through
  `action_rows`. No category column exists.
- Missing: reorder/reparent verbs (order is id ordinals, `event.rs:1386`), live
  `lint_scenario`, handler position data. Incomplete conditions vanish silently
  in `lower_filter` (`event.rs:1496`); Add > Gate leaves a lint error.

Mod format:

- `Content` has 12 kinds (`nova_modding/src/lib.rs:84`). The format already
  holds many scenarios and sections per mod. The limit is the editor: one
  scenario, one file, `editor_<slug>` id, empty `dependencies` and `resources`
  (`bundle.rs:207`). `lift_content` keeps the first Scenario (`:289`), so a
  save drops other kinds.
- `EDITOR_ID_PREFIX` guards local ownership (`mod_cache.rs:265`), and the
  portal refuses `_` ids (`scripts/gen-portal.py:486`).
- No archive format and no file picker exist. Lint walk is workspace-only
  (`nova_authoring/src/lint_walk.rs:96`); per-kind lints are reusable. Any merge
  refusal in `GameStates::Playing` is fatal (`nova_assets/src/merge.rs:290`).
- Suspected bugs, unreproduced: `teardown_document` keeps `DocumentSlot`
  (`node.rs:880`), so Sandbox then Ctrl+S may overwrite the old save; ship
  prototypes use bare ids (`bundle.rs:186`), so two saves can collide; non-base
  `dep://` picks pass Play and fail the merge gate after save.
- Web has no synchronous save (`bundle.rs:602`). The mod builder is native-first.
- Section builder blockers: no `self://` in `AssetIndex::resolved`
  (`asset_index.rs:149`), `render_mesh` is `#[reflect(ignore)]`, `SectionKind`
  is closed so a builder starts from an existing prototype. Unsaved sections
  register for the session like stations, not by a remerge.

## Delivery

Intended result: the Sandbox button opens a mod hub. A mod holds scenarios,
sections, and other content; its contents open in an engine-style shell with
hierarchy, viewport, inspector, and assets/catalog panels, a block script
canvas, undo/redo, and dependency and package commands.

Dies: `WASDCamera` in the editor, the combined gizmo, the one-scenario save
path, the `editor_<slug>` name-is-everything slot, the text-tree event editing
surface once blocks cover it, the "There is no undo" warnings.

May break: driven walks and lesson examples that click by name or fixed screen
point, screenshot examples, saves written under the one-scenario layout (shipped
format, so old `editor_*` bundles must keep opening).

Must fail loudly: unknown ids, missing dependencies, lint errors on save and
package, a move that would orphan a `dep://` reference.

Sequence, one writer at a time in this sprout, one commit or more per slice:

1. History: `EditHistory`, explicit commit at every write site, gesture
   coalescing, diff restore that keeps entities, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y,
   dirty mark, leave and close guards.
2. Viewport: editor-only rig (RMB fly, flat speed with a wheel dial, MMB and
   Alt-LMB orbit, pan, F frame), Q select, W move, E turn, R scale tools, world
   or local, snap, multi-select, duplicate, copy and paste.
3. Shell: fixed panels (toolbar, hierarchy, viewport, inspector, assets and
   catalog dock strip), tree and dropdown widgets, drag-and-drop in `nova_ui`.
4. Mod project: `ModProject`, hub (new and open), many scenarios per mod,
   preservation of every kind, metadata, dependency panel, package command.
5. Blocks: Scratch-style canvas over `ScriptNodes`, palette, reorder and
   reparent verbs, typed sockets, live lint on blocks.
6. Asset builders: section builder first, then the kinds reflection reaches
   (styles, start roles), then hand forms (items, themes, prices, trade).
7. Docs, changelog, playable example, rendered review.

### Slice 1 notes

Decisions:

- `crates/nova_editor/src/history.rs` owns history. Snapshots hold one `Arc`
  record per node, so a step costs what changed. `HISTORY_DEPTH` is 128.
- A write site calls `commit_edit(&mut commands, label)` after the write, or
  `delete_node`. A commit that changed nothing is no step, so a verb may commit
  twice, and a drag commits on release. No system commits per frame.
- A restore tries the recorded entity first, then the path, so a retype keeps
  its entity. Ordinals keep a high-water mark per path.
- Undo is refused while a mouse button is held: a drag owns its node.
- Dirty compares revisions. A save during a held drag compares the live
  document, and a saved state on a discarded branch never reads clean again.
- Back to Main Menu asks only when dirty. New always asks: its window picks the
  template. A fresh document counts as clean.
- Open asks at the row, because the file window is not modal.
- `UnsavedWork` in `nova_gameplay` carries the guard to `nova_menu` and
  `nova_scenario`. The pause menu, outcome panels, Exit and window close stop
  there, and `ask_about_stopped_leave` returns to the editor to ask.
- `teardown_document` clears `DocumentSlot`. The test
  `ctrl_s_in_a_new_session_asks_where_rather_than_overwriting_the_last_save`
  failed on `51397ef7a` ("nothing may be written to the old slot").
- Holding Ctrl parks the free-fly rig, because the chords share letters with it.
- A Save As writes the typed name onto the range, so it is a `Rename Scenario`
  step when the name changed. The saved point is after that step.

Limits:

- An app with no `nova_menu` closes at once on `close_when_requested`.
- The dev content watcher (`restart_for_content`) reloads with no guard.
- Keep editing after a stopped leave from a flown scenario stays in the editor.
  The flight is not resumed.

Skipped checks: workspace tests and Clippy, as instructed. `system_ship_editor`
was not rerun: its diff is one comment and one assert message. No web or wiki page
stated the old behaviour, so docs wait for slice 7.

### Slice 2 notes

Decisions:

- The rig pose is the camera `Transform`. The shared `WASDCamera` is unchanged.
- The default tool is Move. Q and E fly down and up with RMB held. Space dropped.
- The pipette moved from Q to P. The gallery's own Q over a tile stays.
- The editor camera has no `CameraShake`, so `authority.rs:101` needs no change.
- Rig size is `reach.max(watched * SPAN)` for one node and `watched * SPAN` for a
  group. Handles are depth-tested and nearest-hit picked. An overlay camera was
  rejected as too large a change.
- Scale drives a size field in meters and scales the view stand-in until the
  release rebuilds the body. A ship, a section and a group refuse it.
- A copied Player ship becomes AI. A move-tool drag on empty space does nothing.
- Ctrl+A on the Events tab selects every event node.
- `sync_viewport_pointer` reads the mouse only: the CRT holds a second pointer at
  (-1000, -1000).

Limits:

- Script nodes and sections are not copyable.
- The box tests node origins in the viewport, not bounds.
- World or local is on X only, with no menu row. Select All has no menu row.
- Undo of a paste leaves the selection on the parent, not on what was selected
  before the paste.

Proof: `nova_editor` lib tests, 594 pass. `editor_viewport`, `system_ship_editor`
and all five `lesson_build_*` walks exit 0 under lavapipe.

Skipped checks: workspace tests and Clippy, as instructed.

### Slice 3 notes

Layout: `Editor Root` column holds Top Bar, Toolbar, `Editor Content` (`Editor
Rail` 240 | `Editor Stage` | `Script Mount`), `Editor Browser` (176, folds to its
tab row), `Editor Foot` (26). `ui/shell.rs` owns the frame, `ui/hierarchy.rs` the
tree, `ui/browser.rs` the strip, `problems.rs` the live lint.

Seams:

- Slice 4: `browser::asset_entries` returns the `AssetEntry` list. Replace its
  body with the mod's contents. `sync_assets` draws the list, with an 80-row cap.
- Slice 5: `ScriptMount` ("Script Mount") holds the inspector panel. Events grows
  it and hides the stage. Spawn the canvas as its first child. `sync_panels` owns
  the widths.

Decisions:

- Tree, tabs, popup and select widgets are in `nova_ui`. `popup_row` closes the
  popup on `Activate`; one popup is open at a time.
- These factories use `ui_widgets::Button`. `bevy::prelude::Button` is the old
  `Interaction` marker. With it a row hovers and never activates, and a test
  that triggers `Activate` by hand still passes. Only a walk shows this.
- Skin, styles and Generate moved to a Ship tab in the Browser, which shows only
  in a ship and falls back to Catalog on leaving. The ship readout stays at the
  foot of the Hierarchy.
- Drag to reparent is skipped: the document has no reparent verb. Slice 5 owns it.
- Tab gallery stays. Catalog cards have no 3D turntable and walks use the gallery.
- Problems run on a history revision, after 0.25 s, and never with a mouse button
  held. A row selects its node and enters its ship.
- Log is `StatusLog`, a ring of 200.
- The Snap button inverts what Ctrl does. Inspector folds are keyed by group path.
- With nothing selected the Inspector shows one `Inspector Empty` line.
- The foot stays visible in Events.
- `editor_shell` skips the diagnostics panel when capturing: it covers the toolbar.

Limits:

- Frames centre on the window, not the stage. Panels are not resizable. Catalog
  cards have no thumbnails.
- `Inspector` text boxes keep the old value column, so a long path wraps.
- A Problems row finds its node by an id in the lint message. A message with no
  id, such as "Sequence has an empty key", draws a row that selects nothing.

Proof: `nova_ui` lib tests 92 pass, `nova_editor` lib tests 611 pass. `content
lint` is clean. Under lavapipe `editor_shell` exits 0 at 1920x1080 and 1280x720,
and so do `system_ship_editor`, `system_ui_scale`, `editor_undo`,
`editor_viewport`, all five `lesson_build_*`, `screenshot_editor` and
`loop_hull_generate`. Frames are in `frames/slice3/`.

Skipped checks: workspace tests and Clippy, as instructed. The `editor_undo`,
`editor_viewport`, `system_ui_scale` and four `lesson_build_*` runs came before
the Ship tab's two-column style list and rustfmt. `lesson_build_generate` ran
after them. A Problems row click is proved in a test, not in the walk.

### Slice 4 notes

Layout: `project.rs` owns `ModProject`, `ProjectData` and `KINDS`. `project/ids.rs`
owns the id scheme, `project/store.rs` the disk, `project/verbs.rs` the undoable
verbs, `project/package.rs` the checks, zip and import. `ui/hub.rs` is the Mod
Hub, `ui/settings.rs` Mod Settings, `ui/browser.rs` the Assets tab.

Seam for slice 6: add a `KINDS` row with `KindRow::opens` and an opener in
`ui/browser.rs`. A `.glb` resource adds a `PathPurposeType` row in `ui/files.rs`.

Decisions:

- Ids. A local mod is `editor_<slug>` with dashes. Publish strips the prefix and
  maps `_` to `-`. An old save with `_` publishes the same way. A scenario id is
  `<namespace>__<slug>` in snake case, because `ScenarioConfig::new` requires
  it, and it survives a rename. A design is `<scenario>.<node>`. An old save
  keeps the mod id as its scenario id. A scratch project is `scratch-<token>`.
- A package maps `editor_*` dependencies and `dep://editor_x/` refs to `x`. Import
  maps them back when `editor_x` is local and `x` is not installed.
- Undo is one chronological stack. A snapshot is the nodes, the project data and
  the lowered baseline. Switching asset is not a step; undo reopens the asset of
  the step.
- Switching lowers only when the live document differs from its baseline, so
  `ScenarioConfig` fields the document does not own are kept. Hand-written
  designs are never dropped.
- Save writes the changed content files and the manifest. New Mod writes at
  creation. Save As Copy and hub Duplicate write a new mod and do not switch.
- `EditorOpensScratch` puts a rig past the hub. A resource, not an env var,
  and nothing in the player path sets it.
- The enabled set changes on leaving Playing (`PendingMods`), because a change in
  Playing re-runs the merge and a refusal there is fatal.
- A project sees every loaded bundle, enabled or not (`AssetIndex::installed`). A
  dependency just declared is off until the builder leaves, and the walk failed
  on this before the change. The picker lists still offer the enabled set only.
- An undeclared pick is declared by one hook in `commit`. A remove refused by
  `references_to`; a cycle refused through `transitive_deps`.
- Package lints a `merge_bundles` of the visible packs and the project, not the
  live registries. The zip is written to `.zip.partial`, then renamed.
- F1 just after Play panicked: `UnloadScenario` fired inside `Update`, so a
  queued `ConnectedTo` or `CombatLock` insert met a despawned entity. The race
  is in v0.16.0. The unload now runs on the `Scenario` to `Editor` transition,
  where no commands are pending. No per-insert `try_insert`.
- A scratch mod is named `Scratch Mod`. Its name is offered on the first save,
  and an empty name greys Save.
- `NodeParts::key` leaves out `Visibility`. `sync_ship_focus` rewrites it each
  frame, so Add then Ctrl+S, or a save inside a ship, left UNSAVED stuck. The old
  Save As rename step hid this; `editor_undo` found it.

Limits:

- `register_project_content` registers ships, stations and scenarios for Play.
  Sections, styles and items reach the game only through the merge, so Play
  sees them after a save and a restart of content. Slice 6 owns this.
- Import checks the exact id. `editor_a_b` and `editor_a-b` are told apart by
  `is_taken` at New Mod, not at Import.
- Import overwrite replaces the mod and has no undo. The confirm says so.
- The package keeps the local content file name, for example
  `editor_red-run.content.ron`. The manifest names it, so it loads.
- The Assets strip shows about one and a half rows and scrolls.
- The web build has no store: the hub says saving is unavailable and works in
  memory. Package and Import refuse with a line.

Proof:

- `cargo test -p nova_editor --lib`: 631 passed. `nova_menu --lib menu`: 33
  passed. Before the fix, `two_saves_with_the_same_ship_ids_do_not_collide`
  failed 1 against 2, and the F1 panic and stuck UNSAVED mark came from walks.
- `editor_mod_project` under Xvfb on lavapipe, sandboxed: New Mod, second
  scenario, dependency on `example`, save with a clean mark, reopen through the
  hub, package, import over the mod, Play, Scenarios list. EXIT 0 three times at
  1920x1080 and once at 1280x720, with no panic and no stall.
- The zip holds `red-run.bundle.ron` and `editor_red-run.content.ron`, depends
  on `example`, and the installed mod holds two scenarios.
- Also EXIT 0: `editor_undo`, `editor_viewport`, `editor_shell`,
  `system_ship_editor`, every `lesson_build_*`.
- `cargo check -p nova_editor --target wasm32-unknown-unknown`: clean.
- Frames: `frames/slice4/` holds hub, assets, settings and package at 720, and
  assets and settings at 1080. All read correctly.

Real dirs: `~/.config/nova-protocol/enabled_mods.ron` and
`~/.local/share/nova-protocol/installed.mods.ron` hold `editor_arena` and
`editor_saved_range` from earlier unsandboxed walks, written before this slice.
Nothing there is newer than 02:25, so this slice wrote nothing. They were left
as found. `isolate_the_profile` now also moves the mod cache.

Skipped checks:

- Workspace tests and Clippy, per the repo rule.
- Compiled, not run: `system_ui_scale`, `system_input_modes`,
  `system_sandbox_station`, `system_field_controls`, `bug_sandbox_soak`,
  `screenshot_editor`, `screenshot_planet_editor`, `loop_hull_generate`.
- `nova_assets` tests after the last edits: that crate did not change.
- The web build was compiled, not run.

### Slice 5 notes

Decisions:

- Loose blocks are refused. Only a hat stands on bare canvas, because a stack
  or a reporter with no hat has no lowering, and layout data for one would be a
  second document. A refused drop returns the block and says why on the status
  line.
- A value socket is never empty: its literal is the socket's own field, a
  dropped reporter replaces it, and a reporter pulled out leaves stock `0`. A
  reporter dropped on a reporter is refused rather than popped out, since a
  popped block would be loose.
- Combinator sockets slide. A filled boolean socket refuses.
- A variable reporter's stock `Name("")` is a gap until named. A query's stock
  is `scenario.elapsed`.
- Reorder permutes the sorted ordinals of the family. An arriving block takes a
  fresh ordinal, so the no-reuse rule holds.
- Incomplete blocks raise Error Problems, which block Play, Package, Save, Save
  Copy and Open asset.
- A new Gate carries `NEW_GATE_DEADLINE`. A new Sequence or Cinematic arrives
  with one step.
- A selected reporter shows its owner in the Inspector. Del works on it through
  `delete_reporter_key`. Reporters are not copied: a copy would be loose.
- An id chip opens the list and a text box together, because an id can name
  what the document does not hold yet, as a first write of a variable does.
- The Add menu in EVENTS is one disabled hint row. The strip folds in EVENTS,
  restores on leaving, and a strip tab unfolds it.
- The Events rail is back to the Scene rail's width. The canvas shows a
  condition whole, so a long tree row is elided like any other.
- A hat drop freezes the hats still in the flow at where they stand, in the
  same undo step, so nothing closes up under the moved one.
- Mod Settings had an Add path and a scroll bar all along. The window looked
  cut off once no candidate was left, so the offers line now says so.

- Ctrl+wheel zoom stays. `UiTransform::scale` keeps picking correct: the
  walk zooms in, carries a block back under the scale, and the drop lands.
- `Mod Hub` marks the chosen template with `Selected`.

Limits:

- Hierarchy drag to reparent for script nodes is skipped. The verbs would
  serve it, but the tree needs its own slot aiming. The canvas owns reparenting.
- The filter tree rows stay one line. Long conditions read on the canvas.
- A long Description in the narrow Inspector wraps a few pixels past its box.
  The field is older than this slice, and it was left alone.
- A dropped hat can stand over another hat. Nothing pushes hats apart.

Proof:

- `editor_blocks` at 1920x1080 and 1280x720: EXIT 0. It drags a hat, a stack,
  a C-block with a gate, an Or with comparisons in its sockets, undoes and
  redoes, is refused with a reason, zooms, pans, bins, saves, and plays:
  `blocks_fired` reads 7 in the flown scenario.
- A rerun caught a real fault: the Inspector column grew past the window in
  Scene. `system_ship_editor` stalled on it. `DETAIL_W` now holds in both modes.
- Also EXIT 0: `editor_shell`, `editor_undo`, `editor_mod_project`,
  `system_ship_editor`, `screenshot_editor`, and every `lesson_build_*`. No
  `lesson_build_*` touches Events or saves, and none needed a fix.
- `cargo test -p nova_editor --lib`: 640 pass. `nova_scenario` position test
  passes.
- Frames: `frames/slice5/`.

Skipped checks: workspace tests and Clippy, per the repo rule.

### Slice 6 notes

Decisions:

- A section opens on the same shell as a scenario. The root keeps
  `ScenarioNode` and gains `AssetNode`; children are `AssetPartType` parts and
  `SocketNode` sockets, so the tree, the Inspector, and the gizmo need no second
  path.
- A socket drag always snaps to the 5 m grid, with Snap on or off. A socket off
  the grid mates nothing, so a free move has no use.
- The root row clears the selection. Without this, a socket stays selected and
  the item's form does not return.
- A dropped hat takes the nearer of the clear spots down and right, and the
  canvas pans if that spot leaves the view. The first down-only nudge pushed a
  hat under a tall flow below the canvas, and `editor_shell` stalled on it.
- Pan already reaches wide blocks: `drive_canvas_view` has no clamp
  (`ui/canvas/mod.rs:451`). Long chips do not wrap.
- A long ref clips inside its box, because it is one word and wraps nowhere.

Editable kinds:

- Forms, undoable: Section, Style, StartRole, Item, PriceTable, TradeProfile,
  UiTheme, Campaign. Scenario keeps its own editor.
- Not forms: Ship and Station are edited in a scenario. Lesson is kept as
  written and is not editable.

Limits:

- Section mesh pose and collider size are in 10 m cells, and display as cells.
- Style `greeble_rules` are preserved and show a read-only count.
- UiTheme roles are kept as written. The palette is text. There is no panel
  preview of a theme.
- Campaign members are an ordered text list.
- Build parts are text, as `Item=2, Other=1`, because reflection draws a map
  read-only.
- A scenario with no player ship flies the documented empty placeholder hull
  (`scenario.rs:753`). avian logs one `no mass or inertia` warning for it. The
  warning names that hull, not the section: the section's ship has mass.
- This closes the Slice 5 Description and hat overlap limits.

Proof:

- `editor_section_builder` at 1280x720 and 1920x1080: exit 0, 134 steps. New
  mod, two scenarios, a section with `positive_x` moved to 5, 10, 0 as one
  `Move` undo step, `dock_flush.glb` imported, placed on Picket Warden, saved,
  reopened, packaged, and flown. The log reads `section: PASS flown
  spar_works__plain_hull` with the moved socket's world position.
- Rerun, exit 0: `editor_undo`, `editor_shell`, `editor_mod_project`,
  `editor_blocks` at 1920x1080, `system_ship_editor`.
- `cargo test -p nova_editor --lib`: 665 pass. `nova_ui` text field: 4 pass.
  `nova_assets` `self` filter: 5 pass.
- Owner profile `stat` records were equal before and after the run set.
- Frames: `frames/slice6/`.

Skipped checks: workspace tests and Clippy, per the repo rule.

### Slice 7 notes

Scope cut by the owner to findings and sync. Skipped on purpose: CHANGELOG
collapse, docs, the final proof set, and the Done-when table. A UI rework
follows and would redo them.

- All 22 findings in `REVIEW.md` are FIXED with named tests. None is deferred.
- Sync merged master with three conflicts: `CHANGELOG.md`, `menu_ui.rs`, and
  `events.rs`. Semantic breaks fixed: `ObjectiveChipHold` category, event
  `position` in `open_world.rs` and `screenshot_key_hints`. Three flight
  examples already failed on master for want of a `MatchTarget` arm.
- Test isolation: 17 tests in `asset/tests.rs` founded a mod with no `disk()`.
  They raced on one index (2 of 12 runs failed) and could reach the real
  profile. All hold `disk()` now, and `open` refuses a founding without it.

Limits:

- `webmods/gauntlet`, `webmods/the-ledger`, and `assets/mods/example` have no
  layout handler: `OnStart` mixes actions or sets `once`. Their spawns are Events
  blocks and are not on the stage, as before this branch. The raw route is proved
  on the same spawns moved into a layout by the test fixture.
- `CHANGELOG.md` holds both sides of the merge, not yet collapsed.
- The main menu still reads Mod Editor. The owner fact above asks for Sandbox or
  Editor; that is left to the rework.

### Slice 8 notes

The owner tried the build and reviewed it. This slice follows that review.

Decisions:

- Assets filter: a slim kind column, not a chip row. A chip row costs a line of
  the 140 px strip; the column leaves the tiles the full height.
- Parts and Tab open the gallery only inside a ship (`gallery_reachable`). The
  button greys and its tooltip names the reason.
- The `...` pop-up reuses the Inspector column, so there is one row path. A click
  away from a nested list closes only the list. Esc order: drag, list pop-up,
  block pop-up, menu.
- A tile wears a fault badge when Problems names it: the open asset owns document
  findings with a node; other items are named by id in a finding message.
- Waiting kinds (ship, station, lesson) and resources hold no verbs; right click
  says why. The price table has no Rename or Duplicate. A scenario renames by
  name and the rest by id through `rename_asset` with its refusals. The
  rename-scenario window is deleted, because rename is inline.
- Thumbnails render one at a time. A failure keeps the glyph. Stations keep the
  glyph. Tile tooltips are skipped, because the scroll clips them.
- A filter block shows its empty optional fields as chips, so a fresh Entity
  filter opens its id list on the block.
- A row with no unit keeps its unit node at zero width. The node stays, because
  `paint_field_reasons` writes refusals there.
- Variable keys on a Set block are reference chips, so a walk types into
  `Pick Filter` and Enter takes a new name.
- The hub head stands over the game status bar (`layer::CHROME_Z`), which would
  otherwise cover Import and New.

Lessons: copy names Hierarchy in Scene only, so no regeneration was needed.

Verified on the final code, scratch profile, Xvfb :99 and lavapipe:

- `cargo test --lib`: `nova_editor` 701, `nova_menu` 232, `nova_ship gallery` 24,
  `nova_ui widget` 35. All pass. No workspace test or Clippy run.
- Walks exit 0: `editor_undo`, `editor_viewport`, `editor_shell`,
  `editor_mod_project`, `editor_blocks`, `editor_section_builder`,
  `system_ship_editor`, `system_ui_scale`, all five `lesson_build_*`,
  `screenshot_editor`. `screenshot_editor` also passed with `NOVA_CAPTURE=1`
  before the last Inspector change.
- Frames in `frames/slice8/`, read at 1920x1080 and 1280x720. Top bar 36 px, strip
  140 px, foot 22 px. At 1280 the bar drops to `Wld`, `#`, `<`, `>`, Play stays
  Play and centred.
- Owner profile `stat` is the same before and after every run.

Limits:

- A walk opens tiles with mark and Enter. Double click does not land on lavapipe,
  so it is proved by unit test only.
- Thumbnails render one at a time, so a frame taken early shows glyphs on the
  later tiles.
- `section-bench-1920.png` predates the compact segmented options, so Clad and
  Bare show 28 px there. `gallery-1920.png` is the same age; nothing in it changed.
- List and tree rows keep the 1 px border of the shared `ListRow` theme.
- At 1280 the foot key legend clips at the right edge.
- CHANGELOG collapse and docs wait for the next slice, as the brief says.

### Slice 9 notes

Slice 8 limits closed here: tile labels, stage tools in Events, and the foot
legend at 1280.

Decisions:

- A tile prints a short name: a design prints its node, a resource its file.
  A line breaks only after a space, `_`, `-`, `.` or `/`, and a long label ends
  in an ellipsis.
- Tiles of one kind that would print alike print the end of what tells them
  apart on line two. A ` #N` mark is the guard for names that stay equal.
- Tile hover is a `Tooltip` with the whole id or path. The slice 8 reason to skip
  it does not hold: the tooltip uses `OverrideClip`, so the scroll does not clip
  it.
- Events hides the stage tools with `Visibility::Hidden`, not `Display::None`, so
  Undo, Redo, Parts and Play hold their place. A hidden node takes no hits.
- `tool_keys` returns in Events, so Q, W, E, R and X do nothing there. Nothing on
  the canvas uses a tool.
- The legend budgets hints against its own width. Escape is budgeted first,
  because the wrap had dropped it at 1280 and at 1920.

Docs: `web/src/create/mod-editor.md` is the panels and keys page. The wiki key
list had Q for the build view pipette; it is P.

Verified on the final code, scratch profile, Xvfb :99 and lavapipe:

- Frames in `frames/slice9/`, before and after, read at 1280 and 1920.
- `editor_shell` asserts that Play stands in the same place in Scene and Events.
- `tool_keys_do_nothing_in_events` failed with the gate off, then passed.
- `mdbook build` and `npm run ci` in `web/` pass.
- `cargo test -p nova_editor --lib` in a sandbox: 709 pass. No workspace test or
  Clippy run. Two earlier lib test runs in this slice ran with no sandbox.
- Owner profile `stat` changed during this slice: `mods/editor_test2`, its
  `installed.mods.ron` and `enabled_mods.ron` rows, and
  `editor_opened_projects.ron`, at 13:35 and 13:36. No walk ran then. The cause
  is not proved. The sandboxed test run changed nothing.

Limits:

- A ` #N` mark is proved by unit test only. No real content reached it.

### Slice 10 notes

Second review, 15 findings. Each one was reproduced red first, then fixed. None
is deferred. Tests are named where the brief asked for a permanent one.

| # | Finding | Status | Proof |
| --- | --- | --- | --- |
| 1 | Kind row unreachable | FIXED | `every_action_and_filter_block_reaches_its_kind_row`; `editor_shell` opens a pop-up by its `...` chip and finds the Filter row |
| 2 | Enter picked a near row | FIXED | `typed_text_with_no_exact_row_stays_what_was_typed`, `an_exact_match_picks_its_row`, `an_empty_box_picks_nothing` |
| 3 | Refused edit lost on click away | FIXED | `a_refused_field_holds_the_popup_for_one_click`, which also proves no stale red text on reopen |
| 4 | Normal verbs live under a pop-up | FIXED | `an_open_popup_takes_the_keyboard_from_the_verbs`; `editor_shell` presses Del under the pop-up and the block stays |
| 5 | No-change scenario Rename made a step | FIXED | Reproduced with a temporary test |
| 6 | Optional Ref could not be cleared | FIXED | `None` row in the picker; `ui/window/tests.rs` expects the clear row |
| 7 | Own parts read UNDECLARED | FIXED | Reproduced with a temporary test |
| 8 | Tab opened the gallery in Events | FIXED | Reproduced with a temporary test; Parts and the speed dial are hidden and keep their space, and `editor_shell` asserts Play holds its place |
| 9 | No-change asset Rename rewrote an id | FIXED | Reproduced with a temporary test |
| 10 | Esc lost after undo past `since` or eviction | FIXED | `undo_under_a_popup_closes_it_first`, `esc_says_so_when_the_history_lost_the_popups_start` |
| 11 | Right click was half a double click | FIXED | Reproduced with a temporary test |
| 12 | Gallery reopened on an empty Mod filter | FIXED | Reproduced with a temporary test |
| 13 | Hub kept `selected` after a delete | FIXED | Reproduced with a temporary test |
| 14 | Colour window outlived the pop-up | FIXED | Reproduced with a temporary test |
| 15 | `sync_assets` rebuilt every frame | FIXED | 4 builds fell to 1 in a temporary test; a project or catalog change still rebuilds, so a thumbnail still shows |

Also:

- `ui/inspector.rs` now links `sync_panels`, not `sync_editor_mode`.
- The preserved-hull hint and the docs now say the same thing: the editor keeps
  the rest as written, and that hull is edited in the file.
- Doc claims checked: "comments and layout go" (`project/store.rs` `file_bytes`
  and `write_project`), "cannot go inside" (every `context.enter` gate holds
  `ShipNode`), "Play leaves out a bounds handler" (`following_the_objects`, by a
  temporary test). The Esc ladder in `keybinds.md` was rewritten to what the code
  proves.
- `event.rs` `condition_text` and `assignment_text` had no caller outside tests.
  They moved into `event/tests.rs`, which clears the dead code warning.
- `editor_mod_project` now shoots the hub and the New window apart.
- The wasm check found five `nova_editor` errors. Two `dead_code` expectations
  no longer held, and three lints asked for smaller code. All are fixed.
- `editor_shell` first opened a pop-up by hand here, and 8 of 10 runs stalled on
  the `...` release. The logs show the fault mark rebuilt the handler between
  press and click. `release_mouse` clears `ButtonInput` a frame before picking
  sees the release, so the held-button guard in `sync_problems` lifted early. The
  walk now waits for the mark first: 6 of 6 runs pass. A real mouse has no such
  gap by reading the code. That part is not run.

Final proof, at `50379a13b`, all in a sandbox, owner profile `stat` unchanged:

- Lib tests: `nova_editor` 717, `nova_menu` 232, `nova_authoring` 116,
  `nova_scenario` 543, `nova_ui` 102, `nova_ship` `gallery` filter 24. All pass.
- `cargo check --all-targets --features debug` passes with no new warning. The
  wasm check in `ci.yaml`, `cargo clippy --target wasm32-unknown-unknown` with
  `CLIPPY_CONF_DIR=ci/wasm-clippy`, passes.
- `content lint`: 0 errors, 0 warnings. `content gen`: no diff.
- `mdbook build` and `npm run ci` in `web/` pass at `9e2ad1eb8`, the last docs
  change.
- Walks, no stall, ERROR or panic: `editor_undo`, `editor_viewport`, `editor_shell`
  (1920, 1280), `editor_mod_project` (1920, 1280), `editor_blocks`,
  `editor_section_builder` (1920, 1280), `system_ship_editor`, `system_ui_scale`,
  `lesson_build_sections`, `_geometry`, `_generate`, `_flight_test`, `_rail`,
  `screenshot_editor`.
- Frames in `frames/final/`, all 16 read: hub, New window, Scene, Events with the
  pop-up, gallery, Assets, Problems, `section-bench`, at 1920 and 1280. Clad and
  Bare now show the compact options.
- `[Unreleased]` reread: nothing went stale, and no line passes 200 characters.
  These fixes are all bugs from this branch, so no entry is added.

Foot legend follow-up, at `a68dfae53`. The old `events-popup` frames showed `Del delete`
and `Esc leave the ship` under an open pop-up:

- `events_line` and `escape_rung` read the `InputMode` that gates the verbs. Under
  a pop-up the line reads `POP-UP`, `click away keep it`, `Esc cancel the pop-up`.
  `Del` and `F` go whenever the keyboard is not Normal. In a field, Esc says
  `leave the field`.
- `Foot Speed` hides in Events with the other `StageTools` and keeps its space.
- `a_block_popup_takes_the_canvas_hints_off_the_legend` and
  `escape_names_the_rung_the_next_press_takes` hold it. The wiring is proved by the
  new `events-popup` frames, read at 1920 and 1280.
- Sandbox, fresh `stat` unchanged: `nova_editor` lib 718 pass, `cargo check
  --all-targets --features debug` passes, `editor_shell` and `editor_blocks` at
  1920 and 1280 have no stall, ERROR or panic. `mdbook build` and `npm run ci` pass.

Limits:

- Every walk is driven on Xvfb under lavapipe. Nobody played this by hand.
- Finding 4 has one real-wiring step, Del in `editor_shell`. The other verbs rest
  on the single Browse claim and `an_open_popup_takes_the_keyboard_from_the_verbs`.
- The final Problems frames show the clean state, and the hub frames show an empty
  list, since a sandbox has no mods. A listed row is in no final frame.
- `system_ui_scale` shoots no frames. It is judged by its log line, 9 nameplates
  and none over another.
- Beyond the final 16, only three other frames were read:
  `feature-editor-events.png`, `editor-blocks-popup.png`, and
  `editor-undo-confirm.png`.
- The wasm check compiles and lints. No wasm build ran.
- Project and section walks log an avian `no mass or inertia` warning. Its source
  was not traced.

## Verification

- Undo: every verb in the write-site list undoes and redoes to an equal
  lowered document, and selection survives; asserted in unit tests.
- Camera: a held key moves at constant speed; orbit keeps pivot distance.
- Package: a mod built in the editor installs, merges, and plays; old `editor_*`
  bundle opens and resaves.
- Blocks: lift, block edit, lower round-trips real base scenarios unchanged.
- Rendered frames for every panel and block shape, inspected by eye.

## Done when

| Condition | Evidence | Status |
| --- | --- | --- |
| A new mod with two scenarios and one custom section is created, saved, reopened, packaged, and played without leaving the editor flow | `editor_section_builder` at `50379a13b`, 1920 and 1280: real buttons from the hub, reopen through the hub, a package written under `exports/`, and `section: PASS flown spar_works__plain_hull` | MET by a driven walk; not played by hand |
| Every document edit is undoable | Write-site verbs by unit test (`assert_one_step` and the verb tests in `nova_editor`, 717 pass); `editor_undo`; pop-up commits by `a_click_away_commits_a_block_popup_as_one_step`. No machine census of write sites exists | MET for the write-site list |
| No "no undo" text remains | The "There is no undo" warnings are gone. Two confirms still say "undo cannot bring it back": Delete mod and Import over a mod (`ui/window.rs`). Both remove files on disk, not a document edit, and the line is true | MET for document edits; the owner decides if those two confirms count |
| Each slice has a hand-playable example row | `examples/playable/README.md`: `editor_undo`, `editor_viewport`, `editor_shell`, `editor_mod_project`, `editor_blocks`, `editor_section_builder`, for slices 1 to 6. Slices 7 to 10 add no feature | MET |
| Invalidated docs are updated | `web/src/create/mod-editor.md`, `web/src/wiki/keybinds.md`; each claim the review doubted is checked or rewritten; `mdbook build` and `npm run ci` pass | MET; the foot legend under a pop-up is true now and `keybinds.md` says so |
