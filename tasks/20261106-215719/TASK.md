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

### Owner review 2 (after the land at 3a47e9390)

R1. Fix the WARN spam in the log: `despawn` and `remove::<TextFieldFocused>` on
    entities already despawned, after a press.
R2. Events: sort and group the blocks. Only event handlers such as OnStart can
    be placed first, and today that is not clear. Make it obvious what can be
    placed where, and how to build a script.
R3. The Log tab is more useful: clear, and similar tools.
R4. Problems shows problems only, and no "no problems: ..." line.
R5. The bottom panel can be larger.
R6. Assets is more useful: it also shows images and the other files of the mod.
R7. "edit in its scenario" does nothing. Assets are edited as a scenario is:
    double click an asset to go into edit mode for it. A scenario opens in the
    scenario editor, a ship in the ship editor.
R8. A ship opened from a rotated scenario node is rotated in the ship viewer.
    In the ship viewer the transform is identity. Only the scenario has one.
R9. The xyz axis gizmo in the scenario editor is not stable: it flickers on move.
R10. WASD does not work in the editor any more. Remove what it needs.
R11. "Scene | Events" is the scenario view. The ship editor and the section
    editor get their own views. A new section shows in the Assets panel.
R12. Editing a UI Theme palette is simpler, with a color picker.
R13. No "Start from". A new asset, for example a campaign, starts from scratch.
R14. Rework the top menus and context menus. They detect what you edit and offer
    only what fits: a section can add a mesh and a collider, a section offers no
    asteroids, a scenario offers no sections.

### Owner review 3 (after the land at 6fc2b0cf7; work on master)

S1. Fix the WARN `remove::<nova_ui::widget::Selected>` on an entity already
    despawned.
S2. The top Add is not liked. Make it more like the Add menu in Blender. An
    ad-hoc ship for a scenario stays useful, so Add stays.
S3. Lean on "add ship" in the Assets browser: add a ship FROM the assets, a
    predefined ship, into a scenario.
S4. The Events palette group buttons do not filter. A click on "Events" shows
    only events.
S5. Text in Log and Problems can be copied out of the game.
S6. Better labels in the Assets browser. Today it feels different from the
    rest of the UI.
S7. A right click in the Assets viewer can add a new asset, as `+` does.
S8. The top Add relates to this: it can add an asset too.
S9. Every asset opens on its own. A double click on a ship, a design or an
    ad-hoc scenario ship, opens the Ship editor alone: no scenario around it.
    Esc goes back to the scenario you came from. The same rule for every kind.
S10. File > Package opens a modal with the packager's real settings first,
    and after a package it shows the path with Copy Path and Open Folder.
S11. Mod Settings shows dependencies as a list, and the last row is a
    dropdown of installed mods.
S12. The `+` in Assets opens a proper popup. A follow-up choice, such as the
    section kind, is a centred modal, not a loose list in a corner.
S13. No text box for complex data. Section Build > Parts is a list with an
    add dropdown. Audit every field of every form and give each a fitting
    widget; a text box stays only for real free text.
S14. An ad-hoc ship shows in Assets under its scenario and opens the same
    independent way as a design.
S15. The `x` close and remove buttons look bad in the phosphor theme. Make
    one shared icon button for every close, remove and delete.
S16. The Ship editor has its own input, not the scenario's. The wheel cycles
    the part's turn and Q picks Select. The Ship view has no use for move and
    turn tools, so drop them, keep the scenario camera, and fix the turn key.
S17. A placed Events block flickers to the top left corner while it is
    dragged.
S18. Never offer what cannot be made. Owner: "why would you show Lesson in
    the add menu if you cannot add a lesson???? like what even is the
    point?? station too??". A New Asset tree (Add > Asset, the Assets `+`,
    the Assets right click) shows a kind only if choosing it makes a working
    asset. Anything else is hidden, not greyed. The same rule holds for the
    scenario and section Add submenus: no row that refuses when chosen.
S19. One fixed set of colour variables for every UI theme. Owner: "for UI
    theme why are they different like why don't we have a set of rules, we
    should have the same color names like the same variables for all color
    UI themes". The palette is a typed struct with job-named fields. A root
    theme sets every field; a derived theme sets the fields it changes. An
    unknown colour name fails at parse or lint. The editor form shows the
    same swatch rows for every theme.

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
- S16, the Ship view input. Approved through the owner's request; the
  coordinator chose the keys. The Ship view has no stage tools: Q/W/E/R,
  World/Local and Snap hide. The camera works as in the scenario: MMB or
  Alt+LMB orbits, Shift+MMB pans, F frames, and the wheel always dollies,
  with or without a part in hand. Q is the pipette. R turns the part in hand
  one step and Shift+R turns it back; the wheel no longer turns it. If the
  in-game build view uses other turn keys, the Ship view matches them.

Agent assumptions, not confirmed by the owner:

- "Old catalog" means the Tab gallery.
- The Inspector stays in Scene and in asset forms.
- Old `editor_*` saves keep opening.
- A new UI theme inherits from `base/phosphor`. The Inherit row in its form
  can change the parent. Slice 13 chose this; the owner has not confirmed it.
- S9: every ship opens as its own document, `open = {ship|station, design
  id}`. A scenario's own ship opens under the design id its scenario writes,
  `<scenario>.<node>`. Its document writes the design and the spawn's facts
  (name, keys) back to that scenario, so one undo history holds both. Open and
  Back are not steps and mark nothing. Slice 14 chose this.
- S9: the AutoLayout row of ships that were never moved survives a document
  switch (`RowShips`, session only). Without it a ship stacked at the origin
  after Back. Slice 14 chose this.
- S9: the lint of a ship document covers its design only. The spawn facts
  are linted when the scenario is open. Slice 14 chose this.
- S3: a design placed in a scenario is spawned by reference,
  `ShipDesignSource::Prototype { id }`, not copied inline. The spawn resolves
  the id against the design catalog at load
  (`nova_scenario/src/objects/ship_design.rs`, `Prototype`), so an edit to the
  design reaches every placement. The format already had it: no format
  change. It stays an object node on lift, because its spawn id is not the
  design's (`bundle.rs`, `writes_back`). A scenario's own ship is not offered
  for placement. Slice 14 chose this.
- S14: Assets lists a scenario's own ships right after that scenario, not
  under Ships, and the kind column has one row per scenario that made ships.
  Slice 14 chose this.
- S16: the in-game Ship pane uses R (roll), F (socket) and Q (pick)
  (`nova_interface/src/bindings.rs`). The editor matches them and adds Shift
  to step back. The editor keys are constants; the editor has no rebind rows
  for them. Slice 14 chose this.

- S15: one nova_ui `icon_button` for every close, remove and delete x.
  `Dismiss` loses no data (a window close, an entry taken off a list, which
  undoes); `Delete` deletes data and is the only type lit in the danger
  colour. The x of a marked row is armed: lit at rest, because the muted
  glyph did not show on the selected fill. Slice 14 chose this.
- S5: Ctrl+A and Ctrl+C act on the Log or Problems list only while the
  pointer is over it; elsewhere they keep their document meaning. Copy is
  desktop only through bevy `system_clipboard` (arboard, already in the
  lock); the web build refuses with a reason. Slice 14 chose this.
- S13: Greeble Rules and UI theme Roles stay readouts. Both types are
  `#[reflect(ignore)]` and not Reflect, so the row walk cannot reach them.
  Ids the author mints (Base Id, Scatter Id Prefix, Sequence Key, fixture
  ids) stay text with the content lint's rule; ids that name something are
  pickers. Slice 14 chose this.

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

### Slice 11 notes

Owner review 2: R1, R3, R4, R5, R8, R9 and R10. Commits: `c47ad0ecc` (R1),
`e868c9e5e` (R3, R4, R5, R9), `c2c682853` (R8), `b924c9173` (R10 alone, so it
reverts cleanly). Frames and traces are in `frames/slice11/`.

| # | Item | Status | Proof |
| --- | --- | --- | --- |
| R1 | WARN on despawn and `remove::<TextFieldFocused>` | DONE | Root: `text_field_on_pointer` and `close_popup_on_press` are global `Pointer<Press>` observers. A press bubbles through each parent and then the window, so they ran once per level. Both now act only when `press.entity == press.original_event_target()`. Red: with the fix taken out, 3 `nova_ui` press tests failed with the real WARN. Green: `a_press_outside_a_popup_closes_it_and_its_focused_field_without_a_warning`, `a_press_elsewhere_submits_the_focused_field_once`. `editor_blocks` opens a chip list, presses outside it and asserts no `despawned` line in the Log |
| R3 | Log tab tools | DONE | Clear, All/Warnings/Errors, text filter, line count. Fed by the status lines (`EditorStatus::said`) and by every tracing WARN and ERROR through `editor_log_layer`, a `LogPlugin.custom_layer` in `nova_core`. INFO stays in the terminal. Newest line on top. `editor_shell` logs a warn and an error, filters by level and text, clears, then sees a new line arrive. 4 `ui/log.rs` tests and `warnings_and_errors_reach_the_feed_and_info_does_not` |
| R4 | Empty Problems says nothing | DONE | The "no problems" row and its Clear row are gone; the tab shows no count at zero. No walk read that text. Frames `1920-shell-problems.png`, `1280-shell-problems.png` |
| R5 | Taller strip, drag handle | DONE | 220/1080 of the window height: 220 px at 1080, 147 px at 720. A 4 px handle on the top edge drags it between 100 px and 60 percent of the window. Ctrl+3 and Fold still fold it. `the_strip_opens_in_proportion_to_the_window`, `a_resize_stops_at_the_limits`; `editor_shell` pulls the edge 80 px and asserts the growth within 2 px (`*-shell-strip-tall.png`) |
| R8 | Ship viewer shows the scenario transform | DONE | Inside a ship the scenario root takes the inverse of the ship's pose, so the ship stands at identity. The ship's `Transform` stays document data; `settled()` keeps the root pose out of history. Red: `entering_a_turned_ship_stands_it_at_identity_and_is_no_edit` failed without `sync_stage_frame` (ship shown at (40, -12, 7), turned). Green: that test and `a_step_undone_inside_a_ship_leaves_the_stage_where_it_stands` prove no undo step or dirty mark on enter or leave, and the pose back unchanged. `system_ship_editor` turns a moved ship by its Yaw field, enters it, asserts identity and the same step count (`r8-inside-turned-after.png`). The Inspector hides the entered ship's pose rows; the probe's `inside_origin` is gone |
| R9 | Gizmo flicker | DONE | Per-frame trace in `editor_viewport` (`r9-trace-before.txt`, `r9-trace-after.txt`, 1920x1080). The move handle was stable before: rig to node gap 0.00 px on all 24 frames. The corner axis rose was not: at rest (1544, 838), on orbit frames 2 to 12 about (17396, 7144), off the screen. Cause: `draw_axis_rose` ran in `Update` on last frame's camera pose. Fix: the overlays move to `EditorOverlaySystems` in `PostUpdate` after `TransformSystems::Propagate`. After: the rose holds (1544.0, 758.0) within 0.1 px on all 24 orbit and drag frames, and the walk asserts drift under 1 px. It sits at y 758 now because the strip is taller |
| R10 | Remove WASD fly | DONE | One commit. Gone: WASD/Q/E fly, Shift sprint, `FLY_SPEED` and the wheel dial, the top bar speed buttons and readout, `Foot Speed`, the status line camera slot, `EditorProbe::fly_speed`, the `RMB+WASD fly` legend hint, `EDITOR_CAMERA_KEYS` (and its two tests), the RMB gate on the tool keys and on the ghost wheel. RMB is now `CameraGestureType::Look`: it turns the view about the eye. Orbit, dolly, pan and F never read the fly speed. New test `rmb_turns_the_view_and_the_letter_keys_never_move_the_eye`; `editor_viewport` holds RMB, presses E and swings the mouse, asserts the eye did not move and E took the Turn tool, then W takes Move back. Frames `r10-*-shell-scene-after.png` |

Also:

- `ui/shell.rs` module doc said nothing drags; it names the strip edge now.
- Changelog: the panels entry names the resizable strip and the Log filters;
  the camera entry says RMB looks and WASD no longer flies; the "Ctrl stops the
  free-fly camera" entry goes, as there is no fly left. R1, R8 and R9 are bugs
  in unreleased work and get no entry.

Proof on the final tree (`b924c9173` plus the doc fix): `nova_editor --lib`
719 pass, `nova_ui --lib` 105 pass, `cargo check --all-targets --features
debug` with no warnings, `cargo fmt --all --check` clean. Walks EXIT 0 with no
panic and no `despawned` line: `editor_viewport`, `editor_shell` at 1920 and
1280, `editor_blocks` at 1920, `editor_undo`, `system_ship_editor`. Every run
had its own XDG and modding cache; the owner's profile folders did not change.

Limits:

- R1: the walk check never ran red. The red run was the unit tests with the fix
  taken out.
- R1: the field's unfocus keeps `try_remove`. Press observers have no fixed
  order, so a press outside a popup can close the popup that holds the field
  first. That race is valid at runtime.
- R3: a line logged before `nova_core` installs the layer is not in the Log.
  Unit test apps do not install it.
- R8: no walk places a part inside the turned ship. Grid, handles and sockets
  are right by construction: the entered ship's frame is the world frame. The
  walk does not leave the ship and compare; the unit test does.
- R9: lavapipe only, one camera path. Node marks read `ColliderAabb`, which
  moves with the physics step, and name plates read the camera in `Update`;
  neither was measured. If the owner saw a flicker other than the rose, it is
  not proven gone.
- Commits B and C were cut from one tree by hunk; only the final tree was
  built and walked.
- The lane once ran read-only `git` in the main checkout before it read the
  lane rules. It changed nothing.

### Slice 12 notes

Owner review 2: R7, R11, R12, R13 and R14. R2 and R6 are slice 13. Commits:
`f5a1d1f33` (R13), `a65e09eb3` (R7, R11), `f487df515` (R14), `5bd701c6e`
(R12), and commit E (two menu paint fixes, the walk, docs, these notes). Frames
are in `frames/slice12/`.

The document and view model. The open document is a `DocumentType`:
`Scenario`, `Ship` (a ship entered in its scenario), `Section` or `Form` (every
other kind). Each document has its views, `EditorViewType`: Scene and Events for
a scenario, Ship, Section and Form for the rest. `EditorViewType` replaces
`RailTab`. The view switch shows only the open document's tabs, and one system
keeps the view one the document has. A view change is not a document edit: it
makes no undo step and no dirty mark.

```
tile double click / Enter -> open_asset(kind, id)
  scenario -> DocumentType::Scenario -> Scene | Events
  ship     -> its scenario, entered  -> Ship   (Leave Ship -> Scene)
  section  -> DocumentType::Section  -> Section
  other    -> DocumentType::Form     -> Form
every menu row / block -> MenuVerbType -> verb_state(verb, MenuContext)
  -> Live | Greyed(reason on hover) | Hidden
```

| # | Item | Status | Proof |
| --- | --- | --- | --- |
| R7 | Double click opens each kind in its own view | DONE | `open_asset` per kind; Enter does the same on the marked tile. A ship tile opens its scenario with the ship entered. `editor_views` double clicks the scenario, `picket_warden`, the section and the theme tiles and asserts the view tabs each time (`views: PASS the scenario opens on ["Scene", "Events"]`, ship `["Ship"]`, section `["Section"]`, theme `["Form"]`) at 1920 and 1280. `editor_shell` leaves a ship to the scenario before Events |
| R11 | One view set per document; a new section shows as a tile | DONE | Same walk: the switch shows only the open document's tabs. The blank `new_hull` tile is in Assets (`views-*-blank.png`). `a_ship_tile_opens_its_scenario_with_the_ship_entered` asserts the ship is entered, opening is not a step, and nothing is marked dirty. No unit test switches views and counts steps |
| R12 | Palette grid and one colour picker | DONE | The palette is a grid of named swatches; a dim name is inherited. The picker: SV square, hue strip, hex field, alpha strip only where the format has alpha (palette: none, asserted in the walk). Live write with no step; release or close folds the session into one step; Esc reverts and leaves nothing to undo or redo. The four-slider window is deleted. Tests: `a_press_in_the_picker_writes_the_colour_into_the_document`, `the_picker_follows_a_colour_typed_into_the_row`, `a_picker_session_is_one_step`, `escape_takes_the_picked_colour_back`, `a_palette_pick_overrides_its_entry_and_refuses_an_alpha`. Walk: pick in the square, close, `next_undo == "Edit Colour primary"`, depth +1, Edit > Undo returns the palette, Edit > Redo restores it (`views: PASS Undo returned the colour and Redo restored it`) |
| R13 | No Start From; Add makes a blank | DONE | `asset/blank.rs` builds the least value of every reflected field. The Start From chooser, its offer rows and the plain hull are gone. Add > Sections asks for the kind once. The blank hull opens with 5 errors in Problems, and Save, Play and Package wait (`views: PASS a blank hull opens with 5 errors`; `editor_section_builder` fills it and flies `spar_works__new_hull`). Duplicate stays on the tile menu. `every_addable_kind_has_a_blank_a_file_can_hold` |
| R14 | Menus from one context | DONE | `menu_context::verb_state` decides every bar row and the Hierarchy right-click menu. Add holds only the document's verbs; Mesh and Collider read Add until set, then Replace; the Ship menu shows only in a ship. No content-format change. Tests: `each_document_offers_only_its_own_add_verbs`, `a_section_part_reads_add_until_it_is_set_then_replace`, `a_greyed_verb_says_why_until_it_can_run`. Walk asserts each Add menu: scenario shows Asteroid and New Ship and hides Structure, Socket, Mesh; ship shows Structure and hides Asteroid, New Ship, Socket; section shows Socket, Mesh, Cuboid Collider and hides Asteroid, New Ship, Structure; a form has no Add and no Ship menu |

R14 menu audit (before at `8378bc9df`, after at commit E):

| Menu / row | Before | After |
| --- | --- | --- |
| Add menu, form open | shown | hidden |
| Ship menu | always shown, rows greyed outside a ship (`sync_ship_menu`) | hidden outside a ship |
| Add > world (New Ship, Anchor, Asteroid, Planet, Beacon, Light) | greyed inside a ship (`sync_scenario_menu`) | Scene view only; hidden elsewhere |
| Add > script | one of two palettes by mode (`sync_add_palette`) | Events view only, greyed: blocks are dragged out of the palette |
| Add > parts (Structure... to Docking...) | by mode | Ship view only |
| Add > section (Mesh, 4 colliders, Socket) | did not exist | Section only. Mesh... / Replace Mesh...; `<Shape> Collider` / Replace Collider with `<Shape>`; the held shape greyed; a turret's per-joint mesh greyed with the reason |
| Edit > Copy, Duplicate | greyed by selection | scenario: greyed until a selection; ship: greyed with the reason; section, form: hidden |
| Edit > Paste | greyed by clipboard | scenario: greyed until a copy; ship: greyed with the reason; section, form: hidden |
| Edit > Delete | greyed by selection (`sync_menu_delete`) | form: hidden; section: greyed until a socket is selected; scenario, ship: greyed until something deletable is selected |
| Edit > Select All | always | scenario, ship only |
| Edit > Undo, Redo | `sync_history_menu` | unchanged |
| View > Handle Axes, Key Legend, World Grid, Frame | always | stage views only (Scene, Ship, Section); Frame greyed with nothing to frame |
| View > Link Points | always | Ship, Section |
| View > Object Volumes | always | Scene |
| View > Ids | always | Scene, Events, Ship |
| Ship > Pose rows | greyed with no part in hand (`sync_armed_menu`) | same rule, in `verb_state` |
| Ship > Rebind | `sync_rebind_button` | same rule, in `verb_state` |
| Toolbar Parts | greyed outside a ship (`ShipMenuItem`) | Ship document only |
| Hierarchy right-click | own flags per node | the same nodes, each verb also asked of `verb_state` |
| Separators | always | hidden when they would rule off nothing |

R13 blank per kind: scenario: empty, added at once (no template window).
Section: asks for the kind; Mining and Cargo Intake are greyed (their blank holds
sound or mesh refs the form cannot set). Campaign, style, item, start role, price
table, trade profile: least value of every field. UI theme: derives from the
first root theme in sight, because a root theme must declare roles the form
cannot write. Ship and station: no Add (placed in a scenario). Lesson: waits
(kept as written).

Commit E also fixes two menu bugs found in the walk frames:

- Add > Section rows drew with no label (`before-blank-labels-section-add-1920.png`).
  Cause: `MenuVerbType` requires `Tooltip`, whose `On<Add>` observer spawns the
  panel as the row's FIRST child, and `sync_menu_item_paint` read lead, label and
  tail by position. The label stayed `Color::NONE`. The Edit rows carry
  `MenuVerbType` too. Fix: a `MenuTailMark`, and the paint pass and
  `sync_view_menu_marks` find each part by marker. Test:
  `a_row_with_a_tooltip_panel_first_still_paints_its_label`.
- The Edit menu kept an empty band about one row tall under Redo
  (`before-edit-menu-gap-1920.png`). Measured: dropdown 92 px for 66 px of
  rows, padding and border; still 92 with every hidden row despawned; 66 with a
  400 px `min_width`. Cause: the wrappable label let taffy measure the
  auto-height absolute dropdown at a narrower width. A long Undo name ("Undo Edit
  Colour primary") showed it. Fix: menu labels do not wrap. The walk asserts the
  Edit menu ends under 10 px below its last shown row.

`catalog_drift` failed on `systems_ranges_assert_their_invariant_roster`, and it
fails on `3a47e9390` too: that land removed the `a saved range is switched on for
the way out` and `a named save writes a bundle of its own` beats from
`system_ship_editor` but kept their roster slugs. Commit E drops the two slugs
(632 to 630 invariants) and swaps `a destructive verb asks first` for `New
Scenario opens a blank scenario beside the open one`.

Docs: `web/src/create/mod-editor.md` (top bar, view switch, palette and picker,
tiles open in their view, blank Add, Enter, Esc) and `web/src/wiki/keybinds.md`
(top bar, view switch, palette, Enter row, Esc ladder). Changelog: three entries
(views and menus; blank Add; palette picker). The two menu bugs are in
unreleased work and get no entry. New playable example `editor_views` with a
README row.

Proof on the final tree: `nova_editor --lib` 723 pass, `catalog_drift` pass,
`cargo check --all-targets --features debug` clean, `cargo fmt --all --check`
clean. Walks EXIT 0 with no panic: `editor_views` at 1920 and 1280 (10 PASS
lines each), `editor_undo`, `editor_shell`, `editor_mod_project`,
`editor_section_builder`, `editor_blocks`, `system_ship_editor`. Every run had
its own XDG and modding cache; the owner's profile folders did not change.

Limits:

- The lint accepts a blank section's health of 0; only draw weight, credits,
  strip yield and advancement are refused.
- A UI theme blank derives from the first root theme. Which theme a new one
  should start from is an open decision for the owner.
- Mining and Cargo Intake sections cannot be added until the form can set their
  sound and mesh refs.
- The Form view keeps the empty stage, grid and gizmo behind the form
  (`views-*-theme.png`).
- The W tool shows active in the blank and Form frames; not checked against
  `8378bc9df`.
- On the new section and theme tiles the `open` mark sits under the thumbnail
  (`views-1920-section-add.png`).
- The stage and the block canvas have no right-click menu; tile menus are by
  kind, not by `verb_state`.
- `bevy_asset` "Asset Source self does not exist" is in the walk logs; it is in
  the slice 9 logs too.
- The walk drives the picker by one press in the square. A drag, the hue strip
  and the hex field are proven by unit tests only.
- The menu label fixes were proven at 1920 and 1280 in `editor_views`; the other
  walks ran on the same tree but do not shoot menus.

### Slice 13 notes

Owner review 2: R2 and R6, and the slice 12 leftovers. Commits: `7236336f7`
(health and empty file refs in lint), `dde58bb3d` (Mining and Cargo Intake
refs in the form), `c345ffaf9` (R2 palette), `79a5b05c7` (stage and canvas
menus), `9222da972` (Form view), `ce1dbaa4a` (R6 files), `ec64bb250`
(changelog), `232339a2d` (docs), then the final-proof fixes, the
`editor_files` walk and these notes. Frames are in `frames/slice13/`.

```
Assets grid = items (by kind) + mod_files(project, folder listing)
  folder file   -> ModFile { packaged: is_declared, on_disk: true }
  declared only -> ModFile { packaged: true,        on_disk: false } (fault mark)
  tile menu     -> MenuVerbType::File(verb, FileFacts) -> verb_state
  Import File   -> write_mod_file (disk, no undo) + project_step "Import File"
  Delete        -> in use: refused, users named
                   resource: project_step "Remove File", stays in the folder
                   undeclared: removed from the folder, no undo
right press -> RightPress { from, travel } -> release under CLICK_SLOP -> menu
```

| # | Item | Status | Proof |
| --- | --- | --- | --- |
| R2 | Events palette by role; what goes where | DONE | The palette lists Events, Conditions, Actions, Flow, Values and logic in build order. Each group has a folding heading in its canvas colour and a line that says what the group does ("Events start a script"). Each entry is drawn in its own shape: a hat, a stack, a condition, a value. A hover says where the block goes. A drag lights every legal slot. A block that is not a hat over bare canvas rings red, and the canvas says why. An empty canvas says "Drag an Events block here to start a script". Tests: `the_palette_lists_its_groups_in_build_order_with_only_hats_first`, `the_canvas_says_how_to_start_and_why_a_loose_block_will_not_stand`. Walk `editor_files`: `files: PASS the palette lists Events above Actions`, `PASS a loose block over bare canvas says why it is refused`, `PASS a loose drop is refused and makes no step`, `PASS the empty canvas says to start with an Events block`. Frames `files-*-palette.png`, `files-*-drag.png` (lit slots), `files-*-loose.png`, `files-*-empty-canvas.png` |
| R6 | Assets shows the mod's files | DONE | Folder files stand beside the items in one grid, sorted by kind: Images, Meshes, Audio, Files, each with a filter row. An image tile shows the image, a mesh tile its rendered picture, a sound or other file a glyph. A file the package leaves out wears a dotted mark; a declared file missing from the folder wears the fault mark. Right click: Play or View, Reveal Path, Rename, Include in Package, Delete, Import File... (desktop only; `verb_state` hides Rename and Import on the web). Delete and Rename refuse a file that content uses and name the users. Double click views an image or a mesh in a pop-up and plays a sound. Tests: `the_file_list_marks_what_the_package_leaves_out`, `an_imported_file_is_declared_as_one_step`, `a_file_in_use_is_not_deleted_and_the_refusal_names_the_user`. Walk: three imports through + > Import File... and the path window (`PASS three files land as tiles, each one undo step`), Delete on the sound (`PASS a deleted declared file stays in the folder, marked`), double click on the image (`PASS a double click views the image large`) and on the sound (`PASS a double click plays the sound`). Frames `files-*-assets.png`, `files-*-file-menu.png`, `files-*-image-viewer.png` |

Leftovers from slice 12:

| Leftover | Status | Proof |
| --- | --- | --- |
| Form view takes the main area | DONE | In Form the form stands in the main area at a readable width; the stage, the rail and the stage tools hide. Walk: `PASS the form takes the main area and the stage tools hide`; `files-*-form.png` |
| The open mark stays off the tile art | DONE | A dot in the tile's top left corner (`files-*-form.png`, the `new_ui_theme` tile) |
| Mining and Cargo Intake refs in the form; the kinds can be added | DONE | Their sound and mesh refs reflect, so the form shows them with the asset picker. Add > Sections offers every kind. `every_addable_kind_has_a_blank_a_file_can_hold` covers both blanks. No walk adds one |
| Right-click menus for the stage, the canvas and the tiles, by `verb_state` | DONE | Stage: the view's Add verbs, Paste, and Frame and Delete on a node; an object added from it lands at the clicked point. Canvas: Paste, Clean Up, and Duplicate and Delete on a block. Tiles: every row asks `verb_state`. `clean_up_stands_the_scripts_in_one_column_as_one_step`; walk `PASS a right click on the stage offers the scenario's Add verbs`, `PASS a right click on the canvas offers Clean Up`, `PASS Clean Up is one undo step`. Frames `files-*-stage-menu.png`, `files-*-canvas-menu.png`, `files-*-cleaned-up.png` |
| Lint refuses health 0 | DONE | `check_part_tier` refuses health that is not finite and positive, with the field named. `a_section_tier_outside_its_range_is_an_error` now covers 0, -5 and NaN health. `content lint` passes on base content |
| UI theme parent default under Decisions | DONE | "A new UI theme inherits from `base/phosphor`" is in Decisions as an agent assumption the owner has not confirmed |
| The W tool in Section view | CHECKED | W is right there: in Section view the move handles act on the selected socket. In Form view the tools now hide |

Bug found in the final walks: a right-button look opened the stage menu on
release. Look locks the cursor, so the pointer came up where it went down and
the slop test passed. `editor_viewport` stalled at `hold MMB` behind the open
menu (`runs/final-editor_viewport`, before). Fix: `RightPress` also counts the
mouse's own travel while the button is held, for the stage and the canvas.
Tests: `a_look_with_the_cursor_locked_is_not_a_click`,
`a_right_press_that_stays_put_is_a_click`,
`a_right_drag_or_a_press_off_the_target_is_not_a_click`. Before: the walk
stopped with `stalled after 60.0s` at `hold MMB`. After: `editor_viewport`
exits 0 with `cycle complete, no panic`.

Other faults the final proof found and fixed:

- `editor_section_builder` logged `Asset Source 'self' does not exist` three
  times (seen since slice 9). A section tile's picture read the raw section,
  whose refs still say `self://`. The tile now reads the catalog's resolved copy
  (`GameSections::get_section`). After: 0 ERROR lines.
- The CI wasm clippy refused `PathPurposeType`'s variant names (all ended in
  `Import`) and an `#[expect(clippy::too_many_arguments)]` the lint never met.
  The variants are now `Package`, `Mesh` and `File`; the expect is gone.
- `editor_files` at 1280 stalled on the loose-drop beat: the bare point it chose
  sat on a script. The beat now drags over the blank scenario's empty canvas and
  also proves the drop adds nothing.

Changelog, checked against v0.16.0:

- R8 (a turned ship in the ship view) and R9 (the axis rose flicker) are in
  v0.16.0, so each gets a Fixes entry.
- R1: the despawn half of the WARN comes from the popup widget, which v0.16.0
  does not have, so it gets no entry. The text-field half was in v0.16.0; it
  gets no entry either, as nobody saw it there. This is a judgement, not
  proven.
- New entries: right-click menus; palette grouped by role; folder files in
  Assets; a **(breaking)** Modding entry for health 0 and empty file refs.
  Every entry is 200 characters or less.

Docs: `web/src/create/mod-editor.md` (stage and canvas right click, Clean Up,
Form view, palette groups, Assets files, marks and verbs) and
`web/src/wiki/keybinds.md` (right click and double click rows). `mdbook build`
and `npm run ci` pass. New playable example `editor_files` with a README row.

Proof (all sandboxed: `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and
`NOVA_MODDING_CACHE_ROOT` in a fresh temp dir; `~/.local/share/nova-protocol`
and `~/.config/nova-protocol` unchanged by a stat before and after every run):

| Check | Result |
| --- | --- |
| `nova_editor --lib` | 732 pass (733 after the palette pass) |
| `nova_ui --lib` | 105 pass |
| `nova_menu --lib` | 232 pass |
| `nova_authoring --lib` | 116 pass |
| `nova_scenario --lib` | 543 pass |
| `nova_ship --lib gallery` | 24 pass |
| `nova_probe_cli --test catalog_drift` | 2 pass |
| `cargo check --all-targets --features debug` | rc 0 |
| CI wasm clippy (`ci/wasm-clippy`, `-D warnings`) | rc 0 |
| `content lint` | 0 errors, 0 warnings, 17 scenarios audited |
| `content gen` | rc 0, no diff under `assets/` |
| `cargo fmt --all --check` | clean |

Walks (Xvfb, lavapipe, `NOVA_AUTOPILOT`, a capture dir per run): `editor_undo`,
`editor_viewport`, `editor_shell`, `editor_mod_project`, `editor_blocks`,
`editor_section_builder`, `editor_views`, `system_ship_editor`,
`system_ui_scale`, `screenshot_editor` (capture on), `lesson_build_sections`,
`lesson_build_geometry`, `lesson_build_generate`, `lesson_build_flight_test`,
`lesson_build_rail`, and `editor_files` at 1920 and 1280. Each exits 0 with
`cycle complete, no panic`. No log has a panic or a `despawned` WARN. The only
ERROR line is `editor_shell`'s own one for the Log tab. PASS lines:
`editor_files` 12 at each size, `editor_views` 10, `editor_section_builder` 2,
`editor_undo` 1; the other walks prove by their step predicates. The
`PathPurposeType` rename came after that batch; it changes no behaviour, and
`editor_files`, which drives Import File through it, ran again after it.

Frames in `frames/slice13/`, 1920 and 1280, all read: `files-*-palette`,
`-drag` (lit slots), `-loose`, `-empty-canvas`, `-stage-menu`,
`-canvas-menu`, `-cleaned-up`, `-file-menu`, `-assets`, `-image-viewer`,
`-form`.

Palette pass (coordinator review of `files-1920-empty-canvas.png`): the Events
group alone filled the palette with about 20 tall hats, and the other groups sat
below the fold. Now:

- A rail at the palette's left edge holds one dot and short name per group
  (Events, Conditions, Actions, Flow, Values) in the group colour. A click
  scrolls the list to that group's heading. The rail marks the group in view:
  the group the rail scrolled to until the list moves, else the last heading at
  or above the list's top. `group_in_view` is pure;
  `the_rail_marks_the_group_at_the_top_or_the_one_it_scrolled_to` covers it.
- Each entry is one 20 px line in its block's shape, drawn by the palette and
  not by the canvas paint: a hat's round top and tab, a stack piece's notch and
  tab, a C-block's mouth and foot, a condition's points, a value's round ends.
  A heading still folds its group, and a filter still shows the matches from
  every group. The canvas paint is unchanged; only `paint_points` became
  `pub(crate)`.
- The palette is 240 px wide, up from 200, for the rail. A heading shows the
  short name, so "Values fill a socket" stays on one line.

Proof (sandboxed, owner dirs unchanged): `nova_editor --lib` 733 pass;
`cargo check --all-targets --features debug` rc 0; `cargo fmt --all --check`
clean. `editor_files` at 1920 and 1280 exits 0 with 13 PASS lines each, the
new one `a rail click brings the Values group into view`; `editor_blocks` at
1920 and 1280x720 exits 0 with `cycle complete, no panic`. No panic, ERROR or
`despawned` line in the four logs. Every Events frame in `frames/slice13/`
was shot again, and `files-*-rail.png` is new. Read: at 1080 the five rail dots
stand at the top, Events fills the list, and one rail click puts Values in view
(`files-1920-rail.png`); at 720 the same holds (`files-1280-rail.png`). The
drag frames show the filtered list with Actions marked and the slots lit.

Limits:

- The disk half of Import, Rename and an undeclared Delete is not undone; the
  status line says so. Undo puts back only the declaration.
- Rename refuses a file that content uses instead of rewriting the refs.
- The mesh viewer shows the 112 px thumbnail, scaled up. An image smaller than
  the viewer shows at its own size, in the viewer's top left.
- jpeg, mp3 and flac list under their kind but this build does not load them;
  View and Play refuse them with the reason.
- A ship added from the stage menu keeps its slot placement; only objects land
  at the clicked point. A socket uses its default placement.
- The unpackaged mark is a faint dotted ring (`files-1920-assets.png`); it
  reads at 1920 and is small at 1280.
- Rename, Include in Package and Reveal Path have no unit test or walk.
  Rename shares the in-use check that Delete's test proves.
- No walk adds a Mining or Cargo Intake section or picks its refs.
- The hover hint of a palette entry is proven in code and by eye in no frame.
- At 1280 the kind filter list shows Audio only after a scroll, and in
  `files-1280-form.png` a tile's hover label covers the asset filter field.
- A refusal stays on the status line after a view change
  (`files-*-stage-menu.png` still shows the loose-drop line).

### Slice 14 notes

Owner review 3, S1 to S17, on master from `0f864d840` to the commit that
adds these notes. Frames are in `frames/slice14/`; each one was read.

| Item | Status | Commits | Evidence |
| --- | --- | --- | --- |
| S1 despawned `Selected` WARN | DONE | `0f864d840` | 0 `despawned` lines in every final walk |
| S2/S8 Blender-style Add menu | DONE | `b9b4435c4`, `b513f40fb`, `3ce8ff9ad`, `de74aa221` | nova_ui `cascade` tests; `editor_shell` Add and Shift+A beats; `add-menu-*`, `add-search-*` |
| S3 place a ship design | DONE | `d90c9faff`, `3ce8ff9ad` | `a_placed_design_stays_a_reference_to_the_design`; drag, Place in Scenario, Add > Ship > design |
| S4 rail filters the palette | DONE | `461b50b3f` | `a_rail_click_shows_one_group_and_a_second_click_shows_all`, `the_palette_shows_what_the_group_and_the_filter_both_allow`; `editor_blocks` "the Events group shows 45 of 120 blocks"; `s4-rail-filter-*` |
| S5 copy out of Log and Problems | DONE | `f74bee0b8`, `2b51d5f60`, `4a1066b1a` | 7 `ui::row_copy` tests, `a_copied_problem_line_carries_its_node_path`; `editor_mod_project` `PASS Copy said "Copied 3 lines"`, `PASS Ctrl+C said "Copied 16 lines"`; `s5-copy-*` |
| S6 asset labels | DONE | `4cc274e17`, `067d96900` | `a_tile_prints_its_name_over_its_kind`; `s6-assets-before-*`, `s6-assets-after-*` |
| S7 right click on empty Assets | DONE | `3ce8ff9ad` | `assets-empty-menu-1920.png` |
| S9 every ship opens alone | DONE | `243513ff3`, `3fad29d39`, `3dac6b273`, `639fd37ab` | `editor_shell` Back and Open Ship beats; `system_ship_editor`; `back-button-*` |
| S10 Package window | DONE | `f74bee0b8`, `2b51d5f60` | `a_bump_steps_one_part_and_zeroes_the_parts_after_it` and 4 more; `editor_mod_project` `PASS packaged ... red-run-0.1.0.zip, 3266 bytes`, `PASS a bump is one undo step`, the refusal PASS; `s10-package-*` |
| S11 dependencies as rows | DONE | `151ad27eb`, `e917feaf7` | list editor tests; `editor_mod_project` dependency beats; `deps-*` |
| S12 `+` popup and centred follow-up | DONE | `3ce8ff9ad` | `assets-plus-*`, `section-kind-*` |
| S13 no text box for complex data | DONE | `ab155411b`, `e7b8cf91c`, `c53f7d77e`, `7e5e47d48`, `bcc27a8b4`, `ee2b27f12`, `66852fca3`, `2ae845621`, `f6b0059c6`, `2808f8d72` | the audit table below; `every_list_control_of_every_form_and_block_lands` and the per-list add, remove and refusal tests; `s13-*`, `parts-*` |
| S14 ad-hoc ship under its scenario | DONE | `243513ff3` | `a_scenario_ship_shelves_under_its_scenario` |
| S15 shared icon button | DONE | `c9ebd8b04`, `d515cbf1a`, `575931f29` | `an_icon_button_rests_muted_and_lights_primary_on_hover_and_focus`, `only_a_delete_icon_button_lights_in_the_danger_colour`, `an_armed_delete_shows_danger_at_rest_without_the_hover_fill`; glyph U+2715 is in `SGr-IosevkaTerm-Medium.ttf` (`fc-query` charset); `s15-x-*`, `s15-row-*`, `s15-list-x-after-*` |
| S16 Ship editor input | DONE | `7388e6b8c`, `fd9422d80` | `r_rolls_the_part_in_hand_and_shift_r_rolls_it_back`, `q_picks_a_part_only_in_the_ship_editor`, `the_wheel_with_no_gesture_dollies_even_with_a_part_in_hand`; `system_ship_editor`; `ship-parts-on-bar-*` |
| S17 dragged block flicker | DONE | `812b98fb3` | see below; `a_lifted_block_spawns_its_ghost_under_the_pointer`; `s17-drag-before-1920.png`, `s17-drag-after-1920.png` |

Other fixes: a long status line ends in "..." before the selection readout
(`d10cd9960`, `status-ellipsis-after-*`). The section walk presses the new
Package window's button.

S17 root cause: `spawn_ghost` spawned the drag ghost with `left` and `top`
at `Auto`, so layout put it at the window's (0,0) for one frame. It now uses
`ghost_place(at, grab, zoom)` on spawn, as the per-frame update does. A
per-frame trace in `editor_blocks` gave:

| | Ghost frames | First frame at (0,0) |
| --- | --- | --- |
| Before | 13 drags | 13 of 13 (frame 92 palette On Start, 116 placed hat, 143 Variable Set) |
| After | 115 frames | 0; the placed hat's first frame is at (284,62) |

Palette drags had the same fault and the same fix. No slot highlight came
near (0,0) before or after. The trace was not committed.

S13 audit (re-audited from a dump of every row of every form kind and every
block pop-up; 99 text rows remain, all free text or minted ids with a rule):

| Form | Field | Content type | Widget before | Widget after (commit) | Status |
|---|---|---|---|---|---|
| Mod Settings | Dependencies, Screenshots, Icon | lists / optional path | list editors, select | unchanged (earlier slice) | kept: already fitting |
| Mod Settings | Name, Description, Author | free text | text | text | kept: free text |
| Mod Settings | Version | version id | text + rule | text + rule | kept: a typed version with its rule |
| section | Build > Parts | map item -> count | list editor | unchanged | kept: already fitting |
| section | Damage Effects | list of distinct enums | one choice per entry | one toggle per effect: Cracks, Sparks, Plume (B) | DONE |
| section | Animations | list of structs | rows per entry, no add/remove | x per entry + Add (B) | DONE (found in re-audit) |
| section | Name, Description | free text | text | text | kept: free text |
| section | Id | asset id | text | text with rename rule | kept: minted asset id, renamed by the rename path |
| section | Kind, Mass | fixed by design | readout | readout | kept: kind is chosen at creation; mass is computed |
| section | numbers, Hide In Editor, Destroy Sound | numbers / bool / file | number, toggle, file picker | unchanged | kept |
| item | Name, About | free text | text | text | kept |
| item | Category, numbers | enum / numbers | choice, numbers | unchanged | kept |
| start role | Hull, Icon, Jobs, Credits, Goal Kind, Stock | refs / enums / numbers | pickers, choices | unchanged | kept |
| start role | Name, About, Lore, Goal > Title | free text | text | text | kept: free text |
| trade profile | Trades, Jobs, Generated | lists of structs / variants | rows per entry, no add/remove | x per entry + Add; Jobs offers one + per job variant (B) | DONE (found in re-audit) |
| trade profile | Item, numbers | item id / numbers | item picker, numbers | unchanged | kept |
| price table | Clusters, Roles | lists of structs | rows per struct | x per entry + Add; a new rule takes the first free cluster/role and category; refused "every cluster already has an entry" when none is left (B) | DONE |
| price table | Cluster, Role, Category, Ask, Bid | enums / numbers | choice, number | unchanged | kept |
| price table | Id | fixed id | readout | readout | kept: one table per mod, its id is fixed |
| style | Fixtures | list of structs | rows per struct | x per entry + Add; a new fixture gets a minted id `fixture_N` (B) | DONE |
| style | Fixtures N > Model | mesh path (reflect-ignored) | missing | file picker of meshes; empty refused "a fixture needs a model" (B) | DONE |
| style | Fixtures N > Id | minted id | text | text | kept: minted text, made distinct on add |
| style | Greeble Rules | WFC rule set | readout | readout | kept: GreebleRules is `#[reflect(ignore)]` and not Reflect; a rule set with symmetric connects, no row model fits it |
| ui theme | Roles | 16 nested optional role structs | readout | readout | kept: UiRoles is `#[reflect(ignore)]` and not Reflect |
| ui theme | Inherit, palette | theme id / colours | picker, swatches | unchanged | kept |
| block pop-up | HUD Readout > Slot | minted slot key | text | picker of the slots the script uses (new Names::HudSlot); empty refused (D) | DONE |
| block pop-up | Refill Ammo > Section | section id of the named ship | text | picker of the named ship's sections; unknown marked; "name the ship first" when no ship (D) | DONE |
| block pop-up | Railgun / Torpedo Fire > Section | same | text, no offers | same scoped picker (D) | DONE (found in re-audit) |
| block pop-up | Spawn / Scatter Base Id | minted object id | text | text with the lint rule: empty, '/' and generated body ids refused (D); the reference picker of A offers nothing on purpose (a taken id is a collision) | DONE |
| block pop-up | Scatter Id Prefix | minted prefix | text | text, empty refused (D) | DONE |
| block pop-up | Sequence Key | minted key | text | text, empty refused (D) | DONE |
| block pop-up | Rotation (Quat) | rotation | text | three degree boxes, yaw/pitch/roll, one write per box (C) | DONE |
| block pop-up | Timer Start Seconds | expression | text | text | kept: a VariableExpressionNode operand (number, variable or arithmetic); its parser refuses bad syntax on the status line |
| block pop-up | Asteroid Kinds | list of (kind, weight) | readout | Kind choice + Weight number per entry, x + Add (A, B) | DONE |
| block pop-up | Scatter Region | enum with data | readout | Box/Ring choice, fields fresh on switch (A) | DONE |
| block pop-up | Camera Blend | optional struct | readout | none/set choice, fields when set (A) | DONE |
| block pop-up | Look At | enum with data | readout | choice, object id picker (A) | DONE |
| block pop-up | AI Leash | optional struct | readout | none/set choice (A) | DONE |
| block pop-up | Ship Order Kind | optional enum | readout | one choice of none + each kind (A) | DONE |
| block pop-up | Spawn Object Kind | enum with data | readout | 7-way choice, palette stock payload (A) | DONE |
| block pop-up | Patrol Ship > Waypoints | list of points | rows, no add/remove | x per entry + Add (B) | DONE (found in re-audit) |
| block pop-up | Hint Emphasis Verb, Hint Clear Verb | closed words | choice; unknown stays text | unchanged | kept |
| block pop-up | Screenshot Path | file path the action writes | text | text | kept: an output path, nothing to pick from |
| block pop-up | Labels, Messages, Speaker, Text, Names, Title/Location/Date/Note, event Label | free text | text | text | kept: free text |
| any | object's own kind on a root path | enum | readout | readout | kept: the palette places node kinds; switching a placed node's kind is a delete + place |

Final proof on `575931f29`:

- `cargo test -p nova_editor --lib`: 788 passed. `cargo test -p nova_ui
  --lib`: 119 passed. `cargo test -p nova_scenario --lib name`: 28 passed.
  Every run sandboxed: owner dirs unchanged.
- `cargo check --all-targets --features debug` and `cargo fmt --all --check`:
  clean. The CI wasm clippy job: exit 0 after the `copy_out` wasm branch
  (`f74bee0b8`); later commits add no cfg-gated code.
- Content lint (sandboxed): 0 errors, 0 warnings, 0 findings. No content RON
  changed.
- Walks, each EXIT 0 with 0 panic and 0 `despawned` lines, owner dirs
  unchanged: `editor_undo`, `editor_shell`, `editor_blocks`,
  `editor_mod_project` (12 PASS), `editor_section_builder` (3 PASS) at
  1920x1080 and 1280x720; `system_ship_editor` at 1024x768. The only ERROR
  is the line `editor_shell` logs on purpose for the Log tab.
- `npm run ci` in `web/` and `mdbook build`: pass.

Limits:

- `system_ship_editor` opens a 1024x768 window, so S9 and S16 have no walk
  at 1920 or 1280.
- New Mod Up and Down and Add submenu hover are not walked.
- `s15-x-after-*` was taken before the list x moved to the icon button;
  `s15-list-x-after-*` shows it after.
- The icon button focus ring shows only when `InputFocus` names it; the
  editor has no Tab navigation. A row delete's 22 px target is clipped to
  the 20 px row. No modal has an x; they use Cancel and Close.
- Open Folder is not pressed in a walk. The copied text is not read back
  from the clipboard; the walks judge the status line. On Xvfb, arboard
  logs one WARN at exit (no clipboard manager).
- No walk drives a price table or style form, or the Stock and Scenarios
  lists; their add and remove have unit tests only.
- An unknown Refill Ammo section is marked only for a ship the editor holds.
- `references_to` counts an overriding base id as a dependency use, so
  "example" cannot be removed from the template mod.
- Two 1920 walks first failed because a foreign X server held :99 and closed;
  they passed on a private display.
- The rail rows now carry a theme border and look boxier than the old dots.
  The owner has not seen the new widgets.

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
| Each slice has a hand-playable example row | `examples/playable/README.md`: `editor_undo`, `editor_viewport`, `editor_shell`, `editor_mod_project`, `editor_blocks`, `editor_section_builder`, for slices 1 to 6. Slices 7 to 10 add no feature; slices 12 and 13 add `editor_views` and `editor_files` | MET |
| Invalidated docs are updated | `web/src/create/mod-editor.md`, `web/src/wiki/keybinds.md`; each claim the review doubted is checked or rewritten; `mdbook build` and `npm run ci` pass | MET; the foot legend under a pop-up is true now and `keybinds.md` says so |
| Owner review 2, R1 to R14 | Slice 11 notes: R1, R3, R4, R5, R8, R9, R10. Slice 12 notes: R7, R11, R12, R13, R14. Slice 13 notes: R2 (`editor_files` palette, drag, loose and empty-canvas PASS lines and frames) and R6 (import, delete, view and play PASS lines; `files-*-assets.png`). Each row has its tests, walk lines and frames | MET by unit tests and driven walks; not yet reviewed by the owner |
| Owner review 3, S1 to S17 | Slice 14 notes: a status, commits and evidence per item, the S13 audit table, the S17 before and after numbers, and the final proof on `575931f29` | MET by unit tests and driven walks; not yet reviewed by the owner |
