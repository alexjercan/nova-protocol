# Mods screen rework: two-pane Installed|Explore layout with search, quiet enable toggles, details side panel

- STATUS: OPEN
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
- [ ] 1. nova_menu: rebuild the ModsPanel layout (two panes, tab row, list
  container, details container) with markers (ModsTab{kind}, ModsList,
  ModDetailsPanel, ModRow{id}, ModEnableCheckbox{id, base}).
- [ ] 2. Selection + details: SelectedModId resource, row-click observer,
  details-refresh system (rebuild children from ModCatalog + EnabledMods on
  change), default-select first row.
- [ ] 3. Enable checkbox: compact per-row button + the existing toggle
  observer; label/color refresh system; base locked (tag, no button).
- [ ] 4. Tabs: tab-switch observer swapping list content (Installed rows vs
  the Explore placeholder); Selected highlight on the active tab; remove the
  old coming-soon button (sweep symbol + text references incl. tests).
- [ ] 5. Tests (nova_menu): adapt mods_panel_lists_catalog_demo_toggle_base_
  locked to the new markers (rows exist, meta name/description render, base
  has no checkbox); new: row click sets SelectedModId + details pane shows the
  description text; checkbox click flips EnabledMods (absent->present->absent);
  tab switch swaps to the Explore placeholder. Insert ModCatalog/EnabledMods/
  Messages resources per the rig lessons.
- [ ] 6. Docs: CHANGELOG (Changed: two-pane mods screen); screenshot examples
  unaffected (14_screenshot_ui captures the menu - check whether it frames the
  mods panel; adjust only if it asserts on the old layout).
- [ ] 7. Verify: fmt; check --workspace --all-targets; cargo test -p nova_menu;
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

