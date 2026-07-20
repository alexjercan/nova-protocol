# Restyle nova_menu to nova_ui theme (main menu, settings, mods, pause)

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: ui,v0.6.0

## Close-out (20260714)

Restyled every `nova_menu` screen onto `nova_ui::theme`:
- Deleted the 5 private palette consts (NORMAL/HOVERED/PRESSED_BUTTON,
  BACKGROUND_COLOR, TEXT_COLOR) + the "not worth a shared crate yet" comment;
  `use nova_ui::theme`.
- `button()`: 1px border + 2px radius (dropped the `BorderRadius::MAX` pill),
  `theme::PANEL`/`theme::BORDER`/`theme::TEXT`. `update_button_colors` now sets
  BOTH fill and border with the crisp instrument states (PANEL/PANEL_RAISED/
  SELECTED_FILL + BORDER/BORDER_BRIGHT/CYAN). Kept the menu's own polling colour
  system (`MenuButton`) rather than `nova_ui`'s observers, to stay self-contained
  and avoid double-registering the global observers alongside the editor.
- Panels (main menu, settings, mods, pause) got 1px `theme::BORDER` frames + 2px
  radius + `theme::PANEL` fills; separators -> `theme::BORDER`; subtitles/
  descriptions -> `theme::TEXT_MUTED`; mod rows -> `theme::PANEL_RAISED` + a border;
  the "Explore online" placeholder -> `theme::BG` + border + muted text.
- SEMANTIC colours kept meaningful, mapped to shared accents: the enabled/base
  "on" greens -> `theme::CYAN`/`CYAN_BRIGHT` (active), disabled -> `theme::TEXT`;
  the pause scrim `srgba(0,0,0,0.6)` left as-is (it is a dim, not chrome).
- No `Name`, layout-structure, or interaction changes -> the `12_menu_newgame`
  autopilot still drives the menu.

### Verification
- `cargo check --workspace --all-targets --features debug`: clean.
- `cargo test -p nova_menu`: 11 pass. `cargo fmt`.
- `12_menu_newgame` autopilot (headless): menu -> New Game loads the shakedown_run
  scenario (crates/beacons/player ship spawn), cycle complete, no panic - proves
  the restyled buttons still fire by `Name`.

Depends on: 20260714-214111 (nova_ui). Sibling: 20260714-214118 (HUD).

Umbrella: task 20260714-212139. Depends on: 20260714-214111 (nova_ui).

## Goal

Restyle every `nova_menu` screen onto the shared `nova_ui` theme so the menus
match the web app (and the editor): deep navy panels, 1px borders, 2px corners,
crisp cyan/amber hover - replacing the ad-hoc grays + green "pressed" button
colour that make the current menus look random. Palette/metrics only; keep the
default font.

Done = main menu, Settings panel, Mods panel (+ "explore" placeholder + mod rows),
and the pause overlay all use `nova_ui` colours/metrics and the shared button;
`nova_menu` no longer defines its own palette consts; menu tests + the
`12_menu_newgame` autopilot stay green.

## Steps

- [x] Add `nova_ui = { path = "../nova_ui" }` to `crates/nova_menu/Cargo.toml`.
- [x] Delete the private palette consts in `crates/nova_menu/src/lib.rs:62-66`
  (NORMAL_BUTTON, HOVERED_BUTTON, PRESSED_BUTTON, BACKGROUND_COLOR, TEXT_COLOR)
  and the "not worth a shared UI crate yet" comment at line ~60.
- [x] Replace the local `button()` factory (lines ~850-875) and
  `update_button_colors` (lines ~830-848): either use `nova_ui::themed_button` +
  `nova_ui::register` (the observer-based colouring), or keep `MenuButton` but
  point its colours at `nova_ui::theme`. Prefer adopting the shared
  `themed_button` + `ThemedButton` so hover/press is identical to the editor;
  keep `MenuButton` only if a menu-specific query still needs it.
- [x] Restyle the panels to the shared look (1px `nova_ui::theme::BORDER`, 2px
  `RADIUS`, `PANEL`/`BG` backgrounds), dropping `BorderRadius::MAX` pills:
  - main menu root + title + separator (`setup_menu_ui`, ~line 390-430)
  - Settings panel (~lines 455-511)
  - pause overlay (`setup_pause_ui`, ~lines 211-277) - keep the dim scrim
    `srgba(0,0,0,0.6)` but theme the panel
- [x] Restyle the Mods panel (`setup_mods_ui`/`spawn_mod_row`, ~lines 519-683):
  panel + subtitle -> `TEXT_MUTED`; mod row background -> `PANEL`/`PANEL_RAISED`
  with a 1px border; the "Enabled (base)"/enabled toggle greens -> keep a
  semantic "on" accent (define/use `nova_ui::theme` for it, e.g. reuse CYAN or a
  shared success colour) rather than the ad-hoc `srgb(0.5,0.75,0.5)`; the
  "Explore online (coming soon)" placeholder -> the greyed coming-soon treatment
  (muted text; consider an amber "soon" badge to match the editor's rail).
- [x] Keep all `Name` components and the toggle/observer wiring intact (the
  `12_menu_newgame` autopilot clicks menu buttons by `Name`, e.g. "Sandbox
  Button", "New Game Button"): restyle is colour/metrics/child-structure only.
- [x] `cargo check --workspace --all-targets --features debug` clean; `cargo fmt`;
  `cargo test -p nova_menu`; run the `12_menu_newgame` autopilot headless and
  confirm it still drives the menu -> new game path.

## Notes

- Relevant files: `crates/nova_menu/src/lib.rs` (all menu UI). Palette consts
  lines 62-66; inline colours at 230 (scrim), 430 (separator), 568/605/612/638/
  656/669/772 (mods panel greys + greens). Button factory ~850-875;
  `update_button_colors` ~830-848.
- The mods "enabled" green and the base-tag green are semantic (on/locked). Do not
  just flatten them to cyan - pick a shared accent in `nova_ui` and use it
  consistently (this keeps the "state" legible while unifying the source).
- Verify against the autopilot: `12_menu_newgame` (and `09_editor`) find UI by
  `Name` and insert `Pressed` / trigger `Activate`; preserve those names + the
  `Button`/`Hovered`/value components so selection still fires.
- Depends on: 20260714-214111. Sibling: 20260714-214118 (HUD).
