# Mods main-menu panel: list installed mods with enable/disable toggles, Explore online coming-soon, base locked-enabled

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: menu, modding

Spike: tasks/20260714-174000/SPIKE.md
Depends on: 20260714-174120 (catalog + EnabledMods).

Goal: a "Mods" main-menu section - THE GOAL of the flow (see the `demo` mod in the list
and enable it). Add a "Mods" button to the main menu that toggles a modal `ModsPanel`
(mirror the existing `SettingsPanel`: hidden panel, `Visibility` toggle). Inside: a
scrollable list (reuse the `EditorScrollPanel` + `Overflow::scroll_y()` + wheel-scroll
pattern) of INSTALLED-catalog entries, each a row with the mod name/description + an
enable/disable toggle button whose label+colour reflect `EnabledMods`; the `base` row is
shown enabled and LOCKED (no toggle). Plus a disabled "Explore online (coming soon)"
button and a Back button. A toggle observer flips `EnabledMods` and (via 174120's
re-merge) applies live. Follow nova_menu's `button()`/`observe(handler)` idiom and the
editor palette's data-driven list iteration.

## Plan (20260714)

Menu-facing metadata: nova_menu depends on nova_assets (not nova_modding), so expose the
catalog metadata there. `register_bundles`/`seed_enabled_mods` already read the
`InstalledCatalog` asset; add a menu-friendly `ModCatalog(Vec<ModEntry>)` resource built
at `OnEnter(Processing)`, so the menu reads `Res<ModCatalog>` + `Res<EnabledMods>` without
touching the asset machinery. Re-export `ModCatalog` + `ModEntry` from `nova_assets::prelude`.

Follow nova_menu's proven idioms (Settings modal panel + `Visibility` toggle;
`button()`/`observe()`; `update_button_colors`). base row = locked (shown enabled, no
toggle). Toggling a mod flips its id in `EnabledMods`, which 174120's `resource_changed`
re-merge applies live.

Steps:
- [x] 1. nova_assets: re-export `nova_modding::ModEntry`; add `#[derive(Resource, Default)]
  ModCatalog(pub Vec<ModEntry>)` + a `build_mod_catalog` system that fills it from the
  loaded `InstalledCatalog` at `OnEnter(Processing)` (chain it before `seed_enabled_mods`).
  Export `ModCatalog`/`ModEntry` in the prelude. Unit test (or reuse demo_scenario): the
  built `ModCatalog` lists base + demo with metadata.
- [x] 2. nova_menu: a `ModsPanel` marker + a hidden modal overlay in `setup_menu_ui`,
  mirroring `SettingsPanel`. Add a "Mods" button (`observe(on_mods)`) to the main panel,
  between Settings and Exit. `on_mods` toggles `ModsPanel` `Visibility`; `on_mods_back`
  hides it.
- [x] 3. nova_menu: populate the panel from `Option<Res<ModCatalog>>` in `setup_menu_ui`
  (Option so the menu's own unit app, which has no ModCatalog, still builds the shell). For
  each `ModEntry`: a row with the mod `name` + `description`, and either a `ModToggle { id,
  base }` toggle button (`observe(on_mod_toggle)`) or, for `base`, a locked "Enabled" label
  (no toggle). Wrap the rows in a `ModsScrollPanel` node (`Overflow::scroll_y()` +
  `ScrollPosition`) so a long list scrolls; add a small `scroll_mods_panel` wheel system
  (the editor's pattern). Add a disabled "Explore online (coming soon)" button (greyed,
  no observer, not a `MenuButton`) and a Back button.
- [x] 4. nova_menu: `on_mod_toggle(On<Activate>, Query<&ModToggle>, ResMut<EnabledMods>)` -
  read the clicked entity's `ModToggle`; if `base`, do nothing (locked); else flip the id
  in `EnabledMods` (insert if absent, remove if present). A `update_mod_toggle_labels`
  Update system (in MainMenu) sets each toggle button's label ("Enabled"/"Disabled") +
  colour from `EnabledMods`, and renders the base row as a fixed "Enabled (base)".
- [x] 5. Tests (nova_menu): (a) `on_mod_toggle` on a `ModToggle{demo}` button flips
  `demo` in `EnabledMods` (absent->present->absent), driven via `trigger(Activate)` like
  the existing button tests; (b) `on_mod_toggle` on a base toggle is a no-op (base stays
  enabled). Insert a `ModCatalog` + `EnabledMods` in the test app.
- [x] 6. Verify: `cargo test --workspace --no-run`; nova_menu + nova_assets tests;
  `12_menu_newgame` headless - the Mods button + panel exist and the game runs clean; a
  manual/log check that the panel lists base (locked) + demo (toggleable). THE GOAL:
  demo is in the list and enabling it is wired to the live re-merge (proven by 174120's
  `toggling_enabled_mods_remerges_live` + this task's toggle test).

Note: enabling persists only in-session until 174131 (persistence) lands.