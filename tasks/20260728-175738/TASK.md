# Menus + editor adopt the reworked widget language

- STATUS: OPEN
- PRIORITY: 38
- TAGS: v0.9.0,ui,menu,editor

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Story

With the shared skin-aware widgets landed (20260728-175734), bring every
consuming screen to the accepted look. Good news from the code map
(2026-07-28): every LAYOUT already matches demo 1 - the main menu is already
a bottom-right 280px corner panel with the exact button set, settings is
already stacked AUDIO/GRAPHICS/CONTROLS, mods and scenarios are already
two-pane 85% modals and campaigns already collapse via [+]/[-] headers. So
this task is a restyle-in-place plus three real behavior changes: the new
INTERFACE settings section (UI skin choice + persistence), the scenarios
list scroll fix, and regenerated web screenshots.

## Steps (re-planned 2026-07-28 from the implemented demo 1 + code map)

- [ ] Main menu (`setup_menu_ui`, nova_menu lib.rs:1563-1635): corner panel
      goes phosphor (panel treatment from 175734), New Game gets the
      `primary` variant, Exit the `danger` variant; title block styled per
      demo (glowing title over the live backdrop); add the demo's footer
      line (version left / `NOVA OS` right, muted 10px) reading the real
      version.
- [ ] Settings (`build_settings_body`, lib.rs:2712-2850): restyle the three
      existing sections with the new panel-head/slider/segmented/row
      widgets, then ADD the `Interface` section: a `UI skin` segmented
      control (Phosphor | Hardware) wired via `ButtonValue<UiSkin>` +
      `Selected` exactly like the GraphicsQuality row. Persist it: new
      `ui_skin` field on `PersistedSettings` (settings_store.rs, serde
      default Phosphor) through the existing from_resources/load round-trip.
- [ ] Mods modal (lib.rs:1701-1884): tabs, rows, enable checkboxes, badges
      and the detail pane adopt the shared widgets (rows/checks/badges from
      175734); footer Back button styled.
- [ ] Scenarios modal (lib.rs:1886-2021): same widget adoption for campaign
      headers ([+]/[-] markers stay text, phosphor-dim), rows, detail pane +
      Play button (primary, with the `Enter` key chip per demo). FIX the
      broken list scroll: `scroll_mods_panel` (lib.rs:4044-4060) only drives
      `ModsList` - generalize it to a shared scrollable-list marker so
      `ScenariosList` (lib.rs:1976) scrolls too, and clamp the STORED offset
      against content height per lesson
      bevy-ui-scroll-input-clamps-stored-offset (not just the top).
- [ ] Pause + outcome + start-failure overlays (lib.rs:427-590, 668-803,
      868+): panel + button variants (Resume primary with `Esc` key chip,
      Exit danger); VICTORY/DEFEAT banner colors move to the new semantic
      accents.
- [ ] Editor chrome (nova_editor/src/ui/): rail (mod.rs:144-216), component
      cards (card.rs:89-126), drawer (mod.rs:219-257), tooltip
      (tooltip.rs:32-99) inherit the new widgets/theme automatically -
      verify each in the phosphor palette and re-tune the hardcoded
      section-kind tints (card.rs) so they read against the dark screen
      surface; "soon" badges use the bracketed badge widget.
- [ ] Screenshots: extend `ui_capture_script` (examples/ui/screenshot_ui.rs)
      with mods + scenarios + settings beats (rig-first per
      render-output-eyeball), regenerate the committed web captures via
      `scripts/gen-web-screenshots.py`, and eyeball every capture.
- [ ] Docs sweep (keep-docs-in-sync): regenerate/replace stale screenshots
      referenced by web tutorial + index, CHANGELOG [Unreleased] line for
      the restyle + skin setting, wiki pages that show old-theme menu
      captures or describe settings sections.

## Definition of Done (re-planned 2026-07-28)

1. test: `scenarios_list_scrolls_on_wheel_and_clamps` - MouseWheel messages
   move `ScrollPosition` on the scenarios list and the stored offset clamps
   at both ends (fails before the wiring fix).
2. test: `ui_skin_setting_persists_across_save_load` - settings_store
   round-trip keeps a Hardware choice; and a live-tree test that pressing
   the Hardware segmented button updates the `UiSkin` resource + `Selected`
   marker (button_on_setting path).
3. test: existing behavior pins stay green where touched - campaign
   collapse/expand and mods enable-toggle drive the right tree after the
   restyle.
4. render eyeball: updated captures for menu / settings / mods / scenarios /
   pause / editor reviewed; `scripts/gen-web-screenshots.py` output refreshed.
5. manual: owner eyeballs every restyled screen in-engine in both skins - no
   screen still shows the old flat navy/cyan theme.

## Notes

- Layouts are NOT changing; demo 1 mirrored the shipped screens on purpose
  (spike D2). Only the widget language, the Interface section and the scroll
  fix change behavior.
- Settings body is shared by main menu and pause (one builder), so the
  Interface section appears in both for free.
- The `~` HUD-detail row in the CONTROLS reference will be renamed by
  20260728-175747 (On/Cinematic); do not pre-rename it here.
- Web screenshots are committed under `web/src/assets/` and validated by
  `scripts/gen-web-screenshots.py` (staging via `NOVA_SHOT_DIR`); the
  capture examples run with
  `NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example
  screenshot_ui --features debug` (BCS: never the full test suite).
- Depends on: 20260728-175734 (shared widgets + UiSkin resource).

## 2026-07-29 update (post-175734 delivery, owner /flow)

175734 landed the skin-aware widget layer but, per its DECISION.md (owner
call), did NOT delete the legacy navy/cyan `theme.rs` consts - that migration
is now this task's job for the editor + menu surfaces. Concretely, ADD to the
scope:

- Migrate every `theme::{BG,PANEL,PANEL_RAISED,BORDER,BORDER_BRIGHT,CYAN,
  CYAN_BRIGHT,SELECTED_FILL}` reference in `crates/nova_menu/` and
  `crates/nova_editor/` onto the NOVA OS tokens / the new skin-aware widgets
  (~114 refs: editor 24, menu 90). `AMBER/TEXT/TEXT_MUTED` may map to
  `AMBER_NOVA/SCREEN_TEXT/PHOSPHOR_MUTED` or stay if still apt.
- Deletion ordering: the `LEGACY web palette (retiring)` block in `theme.rs` is
  DELETED by whichever of THIS task and 20260728-175742 lands SECOND (the HUD
  task owns the other 23 refs; deleting the block while the other still
  references it breaks that build). If this task lands first, leave the block +
  a one-line "175742 still references these" note; if second, delete it and
  prove `grep -rn "theme::\(BG\|PANEL\|...\)" crates/` = 0.

- UiSkin persistence + Settings row (already in the steps): `UiSkin` is
  Resource-only, so `button_on_setting::<UiSkin>` works exactly like the shipped
  `button_on_setting::<GraphicsQuality>` row - clone that pattern. `UiSkin` is
  `Copy`, so the new `ui_skin` field on `PersistedSettings` (serde default =
  Phosphor) fits its existing `#[derive(Copy)]` + from_resources round-trip.

- LIVE-RESKIN DECISION (owner, 2026-07-29, KISS): only `ThemedButton`
  live-restyles on a skin flip today; the non-button factories (`panel/
  panel_head/segmented/slider_track/checkbox/list_row/badge`) read the skin at
  spawn. Prefer a live in-place reskin of the OPEN screen IF it is cheap (a
  small marker + a reconciler that rebuilds/recolours the non-button widgets on
  `UiSkin` change); if that turns out non-trivial, fall back to rebuild-on-flip
  (the skin change fully applies when the screen is next opened) and say so in
  the RETRO. Do not gold-plate.
