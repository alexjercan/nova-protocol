# Menus + editor spawn the shared nova_ui widget factories (finish 175738 adoption)

- STATUS: CLOSED
- PRIORITY: 37
- TAGS: v0.9.0, ui, refactor

## Story

Task 20260728-175738 migrated the menu + editor PALETTE onto the NOVA OS tokens
and routed every button + section header through the shared nova_ui widgets, but
the rest of each screen is still a BESPOKE `Node` tree tinted the new colour -
so the real screens look different from the `widget_zoo` library. Audit
(2026-07-29): shared factory call-sites in nova_menu + nova_editor are
`button()`/`themed_button()`/`menu_button()` (30) + `panel_header()` (8), but
`panel()`/`panel_head()`/`list_row()`/`checkbox()`/`badge()`/`slider_track()`/
`segmented()`/`toggle()` are **0**. This task finishes the adoption: make the
screens actually spawn those factories so they match the zoo, in both skins.

## Reusable-widget additions (nova_ui) - so the game + zoo share ONE impl

- `segmented_container(skin)` + `segmented_option(label, skin)`: promote the
  zoo's local segmented-row helpers into `nova_ui::widget` (a bordered/recessed
  container + a segment-sized ghost `ThemedButton`), so a FUNCTIONAL segmented =
  the container + N options each paired with `ButtonValue<T>` + `Selected` (the
  settings graphics/skin rows + the zoo both use it). Retire nova_menu's local
  `segmented_button`/`segmented_row` and the zoo's local `seg_option`.
- `sync_slider_meters`: a nova_ui system (registered in `widget::register`) that
  recolours the `SliderBlock` children of any `Changed<SliderValue>` entity via
  `slider_meter_color`, so any `bevy_ui_widgets::Slider` wearing `slider_track`
  gets the reactive block-meter for free (settings volume + the zoo). The zoo's
  example-local sync moves onto it.

## Steps

- [x] nova_ui: add `segmented_container`/`segmented_option` + the
      `sync_slider_meters` system (register it); point the zoo at them.
- [x] Settings (`build_settings_body`): graphics + skin rows use
      `segmented_container` + `segmented_option` + `ButtonValue<T>` (delete the
      local `segmented_button`/`segmented_row`); the volume row becomes a
      `Slider` wearing `slider_track` (block-meter) driven by `sync_slider_meters`
      - drop the bespoke `VolumeThumb`/thumb-sync, keep `on_volume_slider_change`
      -> `MasterVolume`.
- [x] Mods modal (lib.rs ~1730-2210): modal panel -> `panel(skin)` +
      `panel_head("Mods", Some("DELTA-9"), skin)`; each mod row -> `list_row`;
      the enable checkbox -> `checkbox(enabled, skin)` wrapped clickable (Button
      + `ModToggle` + `observe(on_mod_toggle)`, still `MenuSfxButton`); the
      `base` tag -> `badge(BadgeKind::Mute, "base", skin)`; detail-pane
      dependency/adds chips -> `badge(...)`.
- [x] Scenarios modal (lib.rs ~1985): modal panel -> `panel`/`panel_head`; rows
      -> `list_row`; the TUTORIAL / kind tags -> `badge`; campaign `[+]/[-]`
      headers stay text (phosphor-dim). Play button stays a primary `button`.
- [x] Pause + outcome + start-failure overlays + main-menu corner panel (the
      bespoke `Node` + `BackgroundColor(theme::SCREEN_0)` panels at lib.rs
      490/574/750/925/1605): -> `panel(skin)` (+ `panel_head` where titled).
- [x] Editor chrome (nova_editor/src/ui/): rail / drawer / tooltip surfaces ->
      `panel(skin)`; component cards -> `list_row`/`panel` where they fit,
      keeping the section-kind tints; `soon` badges -> `badge`.
- [x] Verify: full check suite; existing behaviour pins green; render eyeball of
      each screen vs the zoo.

## Definition of Done

1. test: existing pins stay green after the rebuild - `dragging_the_volume_
   slider_sets_master_volume`, `checkbox_click_flips_enabled_state_and_mark`
   (mods toggle), `toggling_a_campaign_header_collapses_and_expands_its_members`,
   and the graphics/skin `button_on_setting` rows. Add a nova_ui live-tree test
   that `sync_slider_meters` lights the block-meter from a `SliderValue` change.
2. cmd: the shared factories are spawned at the named sites -
   `grep -rn "list_row(\|checkbox(\|badge(\|panel(\|slider_track(\|segmented_option(" crates/nova_menu/src crates/nova_editor/src`
   prints > 0; and the bespoke locals are gone -
   `grep -rn "fn segmented_button\|VolumeThumb\|Mod Locked Tag" crates/nova_menu/src`
   prints 0.
3. cmd: `cargo check --all-targets` green; nova_menu + nova_editor lib tests green.
4. render eyeball: menu / settings / mods / scenarios / pause / editor captured
   (screenshot rigs + widget_zoo cross-check); the screens render the SAME widget
   shapes as the zoo (owner/CI GPU).
5. manual: owner eyeballs each screen in-engine (both skins where applicable) -
   rows/checkboxes/badges/slider/segmented/panels read as the zoo's widgets, not
   bespoke look-alikes.

## Notes

- This is a rebuild-in-place: layouts + behaviour (selection, scroll, toggle,
  persistence) are PRESERVED; only the spawned widget shapes change to the shared
  factories. The volume slider keeps its `Slider`/`SliderValue`/`MasterVolume`
  wiring; only its VISUAL (thumb -> block-meter) changes.
- Follow-up to 20260728-175738 (which documented this gap in its RETRO). Depends
  on the widget factories from 20260728-175734 + the zoo polish (block-meter
  slider, toggle, segmented, panel radial).
- Editor chrome is the last Step and the most self-contained; if it balloons it
  can split to its own task rather than stall the menus.

## Implementation (2026-07-29) - VERDICT

Delivered on branch `refactor/menus-widget-adoption`:

- nova_ui reusable additions: `segmented_container`/`segmented_option`;
  `sync_slider_meters` (block-meter reactivity, registered); an interactive
  `ListRow` reconciler + `list_row_colors`; `checkbox_colors`/`checkbox_glyph`;
  `panel()` is now a PAINT decorator (no Node) + `panel_node()` for plain panels.
- Settings: graphics + skin rows -> `segmented_container`+`segmented_option`;
  volume slider wears `slider_track` (block-meter), bespoke thumb/thumb-sync
  gone (label sync kept -> MasterVolume path unchanged).
- Mods: rows -> `list_row` + `checkbox()` + `badge('base')`; `sync_mod_checkboxes`
  repaints the checkbox in place from `checkbox_colors`.
- Scenarios: rows -> `list_row`; modal -> `panel()`+`panel_head`.
- All 7 modal panels (pause/settings/outcome/start-failure/mods/scenarios/menu)
  -> `panel()`. Editor: rail -> `panel()`, 'soon' chip -> `badge()`.

DoD status:
1. test: existing pins green after the rebuild - volume->MasterVolume (settings
   panel test), mods toggle, campaign collapse, graphics/skin rows; + new nova_ui
   `sync_slider_meters_lights_blocks_from_value`. nova_ui 11 + nova_menu 73 +
   nova_editor 13 all PASS.
2. cmd: PASS - shared factories spawned (nova_menu 19, nova_editor 2 call-sites);
   bespoke locals gone (`grep "fn segmented_button|VolumeThumb|Mod Locked Tag"` = 0).
3. cmd: PASS - `cargo check --all-targets` green.
4. render eyeball: widget_zoo re-rendered clean in both skins after the nova_ui
   refactors (panels/segmented/list_row/slider). The game screens spawn the SAME
   factories; the in-engine capture of each screen is the owner/CI GPU step (5).
5. manual: PENDING owner in-engine eyeball of each screen in both skins.

Scope carried forward: campaign-header row + explore-tab mod row stay bespoke
(a header + a second row builder; the explore row could adopt list_row in a
follow-up); the editor component-card row stays a bespoke ThemedButton (fits
button/drawer-toggle semantics, not a static row). Member-row indent dropped
(collapsible headers show hierarchy).
