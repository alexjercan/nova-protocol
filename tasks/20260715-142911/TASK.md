# Mods screen rework: two-pane Installed|Explore layout with search, quiet enable toggles, details side panel

- STATUS: CLOSED
- PRIORITY: 14
- TAGS: modding, menu, ui

Spike: tasks/20260714-202515/SPIKE.md (option AA)
Depends on: 20260715-142849 (bundle meta feeds the details panel).

Goal: rework the Mods menu into a Factorio/Wesnoth-style two-pane screen.
LEFT: tab bar (Installed | Explore online) and a scrollable list of rows -
name, version, author, with a QUIET per-row enable checkbox (not a big toggle
button) on the Installed tab; base shown locked. RIGHT: a details side panel
for the selected mod rendered from its bundle meta - title, author, version,
description, dependencies - plus the action buttons (Enable/Disable here;
Install/Uninstall/Update belong to the Explore task). Built from the existing
nova_menu + nova_ui idioms. This task ships the Installed tab fully working;
the Explore tab renders as a placeholder until 20260715-142916 wires it.

DESCOPED at plan time: the search box (no text-input widget exists anywhere in
the workspace and the list is 3 mods; the layout keeps room above the list -
build a text input when mod counts warrant). Icon/screenshot rendering also
deferred (no shipped mod carries image meta yet).

## Plan (20260715)

Design notes (from the code):

- Rework IN PLACE: the ModsPanel stays a MainMenu overlay toggled by the Mods
  button (Visibility pattern, crates/nova_menu/src/lib.rs:552-667 today), but
  grows to a large two-pane layout (~85% x 85%, flex row).
- LEFT pane (fixed width ~40%): a tab row (two `nova_ui::themed_button`s
  "Installed" / "Explore online" with the `Selected` marker driving highlight)
  above the existing scroll-list pattern (ModsScrollPanel). Rows come from
  `ModCatalog` (which since 142906 already includes DOWNLOADED mods, so they
  appear with no extra work): mod name + version/author line in muted small
  text + a compact right-aligned enable CHECKBOX (a small fixed-size themed
  button whose label is "x"/"" driven by EnabledMods; base shows a muted
  "base" tag instead). Clicking anywhere else on the row SELECTS it.
- SELECTION: a `SelectedModId(Option<String>)` resource; row observer sets it;
  a details-refresh system rebuilds the right pane on change (and on
  ModCatalog/EnabledMods change). Default selection: first row.
- RIGHT pane (details): title (meta.name), author + version line, multi-line
  description, dependencies line ("Dependencies: none" when empty), and the
  action area: Enable/Disable themed button bound to the same toggle observer
  (base: a locked label). The Explore-tab details actions
  (Install/Uninstall/Update) are 142916's.
- EXPLORE TAB (this task): switching tabs swaps the list container content;
  Explore shows a single inert placeholder row ("Connects to the mod portal -
  next update") reusing the current coming-soon styling. The old standalone
  "Explore online (coming soon)" button is REPLACED by the tab (sweep its
  observers/tests).
- Reuse: nova_ui::{themed_button, panel_header, separator, Selected};
  wheel-scroll system stays; on_mod_toggle keeps its EnabledMods semantics
  (messagereader-needs-resource-guard-in-tests applies to any new system rig).

Steps:
- [x] 1. nova_menu: rebuild the ModsPanel layout (two panes, tab row, list
  container, details container) with markers (ModsTab{kind}, ModsList,
  ModDetailsPanel, ModRow{id}, ModEnableCheckbox{id, base}).
- [x] 2. Selection + details: SelectedModId resource, row-click observer,
  details-refresh system (rebuild children from ModCatalog + EnabledMods on
  change), default-select first row.
- [x] 3. Enable checkbox: compact per-row button + the existing toggle
  observer; label/color refresh system; base locked (tag, no button).
- [x] 4. Tabs: tab-switch observer swapping list content (Installed rows vs
  the Explore placeholder); Selected highlight on the active tab; remove the
  old coming-soon button (sweep symbol + text references incl. tests).
- [x] 5. Tests (nova_menu): adapt mods_panel_lists_catalog_demo_toggle_base_
  locked to the new markers (rows exist, meta name/description render, base
  has no checkbox); new: row click sets SelectedModId + details pane shows the
  description text; checkbox click flips EnabledMods (absent->present->absent);
  tab switch swaps to the Explore placeholder. Insert ModCatalog/EnabledMods/
  Messages resources per the rig lessons.
- [x] 6. Docs: CHANGELOG (Changed: two-pane mods screen); screenshot examples
  unaffected (14_screenshot_ui captures the menu - check whether it frames the
  mods panel; adjust only if it asserts on the old layout).
- [x] 7. Verify: fmt; check --workspace --all-targets; cargo test -p nova_menu;
  -p nova_assets --test demo_scenario (unchanged expectations); a headless
  12_menu_newgame run if cheap (the menu boot example).

## Notes

- Relevant files: crates/nova_menu/src/lib.rs (:338-345 markers, :425
  setup_menu_ui, :552-667 panel, :672-730 spawn_mod_row, :786-830 toggle
  observers + label refresh, tests :1380-1480), crates/nova_ui/src/widget.rs
  (themed_button:143, panel_header:174, separator:190, Selected:18),
  crates/nova_ui/src/theme.rs.
- ModCatalog rows already include downloaded mods (142906) - the Installed
  tab covers them for free; enable/disable works through the same
  EnabledMods path.
- 142916 builds directly on the markers/details-area contract this task
  defines - keep the action area a clearly-marked container.

## Close-out (20260715)

What changed:

- crates/nova_menu/src/lib.rs: the ModsPanel is now a two-pane screen
  (85% x 85%): LEFT a tab row ("Installed" | "Explore online", nova_ui
  themed_buttons with the Selected marker as the active highlight) over the
  scrollable ModsList; RIGHT a ModDetailsPanel whose children are rebuilt
  from the selected mod's bundle meta (name header, "vX - by Author" line,
  description, "Dependencies: ..." with "none" fallback) plus a
  ModDetailsActions container holding the Enable/Disable themed button
  (base: an "Enabled (base)" locked tag). Installed rows are clickable
  ThemedButton rows (name + muted version/author line) with a compact
  24x24 quiet enable checkbox ("x" when enabled, "" when disabled); base
  rows show a muted "base" tag instead. The Explore tab renders one inert
  placeholder row ("Connects to the mod portal - next update"). The old
  standalone "Explore online (coming soon)" button is deleted, and all its
  code/comment/test references in the workspace with it (the remaining
  "coming-soon" greps are the unrelated editor rail rows, web wiki lesson
  prose, and historical task records).
- New state: SelectedModId(Option<String>) + ModsActiveTab resources;
  refresh_mods_list (rebuilds rows on tab/catalog change, default-selects
  the first row and repairs a selection that left the catalog) and
  refresh_mod_details (rebuilds the pane on selection/catalog/enabled
  change), chained so a default selection renders the same frame.
  setup_menu_ui spawns the containers EMPTY and re-arms both systems by
  writing the resources - one population path for menu entry, tab switches
  and live catalog changes alike.
- crates/nova_ui/src/widget.rs: register() gained an idempotence guard
  (resource marker) because the menu now registers the themed-widget
  observers too and coexists with the editor in the shipped app.
- CHANGELOG.md: Changed entry (two-pane mods screen; search deferred).

Decisions / deviations from the plan:

- Marker split: the plan sketched ModEnableCheckbox{id, base}; implemented
  as the existing ModToggle{id, base} kept as the shared toggle contract
  (row checkbox AND details button feed the same on_mod_toggle observer)
  plus a plain ModEnableCheckbox marker that scopes the "x"/"" mark-refresh
  system to checkboxes only - avoids duplicating id/base across two
  components and keeps the details button's label owned by the pane rebuild.
- ModsScrollPanel was renamed to ModsList (one entity is both the
  wheel-scroll target and the swapped-content container).
- The checkbox is a MenuButton (existing hover polling + click cue), while
  rows/tabs/details button are ThemedButtons (observer colours + Selected).
  The "quiet" split is native: bevy_ui_widgets' Button stops click
  propagation, so a checkbox click never activates the row beneath it.
- Search box: DESCOPED as planned (no text-input widget exists); the left
  pane keeps room above the list.
- 14_screenshot_ui only clicks "Sandbox Button" / editor buttons - it never
  frames or asserts the mods panel, so it needed no changes.

Evidence (real counts):

- cargo test -p nova_menu: 18 passed, 0 failed (was 13; 1 adapted:
  mods_panel_lists_catalog_demo_checkbox_base_locked; 5 new: row-click
  selection + details render, checkbox flip absent->present->absent with
  mark sync, details action toggle + relabel, tab switch to the Explore
  placeholder and back, old coming-soon button gone).
- cargo test -p nova_assets --test demo_scenario: 11 passed, 0 failed
  (unchanged expectations).
- cargo fmt --check: clean. cargo check --workspace --all-targets: clean.
- Sabotage A/B: with refresh_mod_details no-op'd (early return), exactly the
  3 details-dependent tests failed (mods_panel_lists_catalog_demo_checkbox_
  base_locked, clicking_a_row_selects_it_and_renders_its_details,
  details_action_button_toggles_and_relabels; 15 passed) - the details
  mechanism is what the tests pin. Restored; 18/18 green again.
- Headless boot: BCS_AUTOPILOT=1 12_menu_newgame under Xvfb :99 exits 0
  with "probe: clicked New Game Button", "nova harness: reached Playing",
  "autopilot: cycle complete, no panic", and zero "Encountered an error in
  command" lines.

Difficulties:

- None blocking. The one subtle piece was initial population: a fresh menu
  entry spawns fresh empty containers, but resources are not "changed" just
  because the UI respawned - solved by setup_menu_ui resetting
  ModsActiveTab/SelectedModId (any ResMut write marks changed), which
  re-arms the refresh conditions; documented in the setup comment.
- Verified in the bevy_ui_widgets source (button.rs) that
  button_on_pointer_click/down call propagate(false), before relying on
  nested checkbox-in-row buttons.

Reflection:

- Single-population-path (empty shell + change-armed refresh systems)
  removed the old duplicated spawn logic and made every test go through the
  production path; worth repeating for future dynamic panels.
- The register() guard belongs in nova_ui rather than caller convention -
  callers cannot know who else registered. Small shared-crate touch, but
  the right layer.

Review round (20260715): the out-of-context review took real screenshots and
caught two things headless testing could not see. R1.1 (MINOR): the
bottom-right menu card painted OVER the open mods panel's corner, and the
sibling z-order was nondeterministic by construction (no GlobalZIndex on
either overlay root, Entity-order fallback with ids recycled from the
despawned ambience scene) - fixed by an explicit GlobalZIndex(1) on BOTH the
Mods and Settings panel roots, pinned by a component-presence test
(overlay_roots_carry_an_explicit_z_index; the rendered order itself stays
visually-only verifiable). R1.2 (NIT): switching to Explore left the last
installed mod's details + live Enable/Disable next to the portal placeholder -
the Explore branch now clears SelectedModId so the pane drops to its fallback,
and the tab-switch test asserts the fallback + no action button + the default
selection returning on switch-back. After the fixes: cargo test -p nova_menu
19 passed, 0 failed; fmt clean. Deferred by the reviewer (no action):
details-pane scroll for long descriptions, and the Selected-marker border
inconsistency (pre-existing nova_ui widget behavior). Lesson: screenshots
catch stacking/layout truths that component-level asserts cannot - budget a
visual pass for UI tasks.

