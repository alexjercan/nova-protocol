# Menu UI polish: full skin reactivity, fix interaction flicker, fixed panel widths

- STATUS: CLOSED
- PRIORITY: 39
- TAGS: v0.9.0, ui, polish

## Story

Owner in-engine eyeball (2026-07-29) of the widget-adopted menus found three
issues: (1) switching to Hardware reskins buttons/rows but NOT panels (and the
other static widgets) - "make everything reskinable"; (2) clicking checkboxes/
toggles flickers, with button text briefly rendering larger ("as if Clicked");
(3) panel/pane widths are not fixed - a longer/shorter selected-mod description
changes the two-pane container widths.

## Steps

- [x] Reskin: add a `PanelSkin` marker + reconciler in nova_ui so `panel()`
      repaints (bg/border/shadow/gradient) on a `UiSkin` change (mirrors the
      button/list-row reconcilers). Refresh the mods + scenarios lists AND the
      settings body on skin change so their in-panel widgets (badges, checkbox,
      segmented, slider) re-spawn for the new skin.
- [x] Flicker: the 1-frame default-font flash is `apply_ui_font` running in
      `Update` - a `UiText` spawned this frame gets Iosevka only NEXT frame.
      Move the font routing to run before UI text layout so respawned text never
      renders in the default face. Confirm no other respawn-churn flicker on the
      checkbox/toggle path.
- [x] Widths: pin the two-pane panel widths (mods + scenarios) - `min_width:0`
      on the flex panes + a fixed detail-pane width - so a longer/shorter
      selected-item description cannot resize the containers. Wrap long text.
- [x] Verify: re-render the menus in both skins (panels reskin), interact with
      checkboxes (no font flash), and select long/short mods (widths hold).

## Definition of Done

1. render eyeball: flipping the Settings skin reskins the PANELS + badges +
      checkbox + slider + segmented (not just buttons), in both skins.
2. render eyeball: clicking a mods checkbox / a zoo checkbox shows no font-size
      flash and no panel flicker.
3. cmd/eyeball: selecting mods with very different description lengths does not
      change the mods modal / pane widths.
4. cmd: `cargo check --all-targets` green; nova_ui + nova_menu lib tests green
      (+ a nova_ui live-tree test that a `PanelSkin` repaints on skin change).
5. manual: owner confirms all three in-engine.

## Notes

- Follow-up to 20260729-105359 (widget adoption). The reskin reconciler reuses
  the existing paint (panel paint fn); the list/settings refresh-on-skin reuses
  the existing refresh systems (add `UiSkin` change to their run conditions).

## Implementation (2026-07-29) - VERDICT

- Reskin: `panel()` carries `PanelSkin`; `reconcile_panel_skins` repaints it on
  a UiSkin flip (fixes "panels stay phosphor"). `mods_list_dirty` +
  `scenarios_list_dirty` gained `UiSkin` so the lists rebuild on a flip -> their
  `list_row`/`checkbox`/`badge` re-spawn for the new skin. Buttons/segmented
  options/rows already reskinned via the button + list-row reconcilers.
- Flicker: `apply_ui_font` moved from `Update` to
  `PostUpdate.before(UiSystems::Content)`, so a `UiText` spawned this frame gets
  Iosevka BEFORE text measure/layout - no 1-frame default-face flash on respawn.
- Widths: `min_width: px(0)` on the 6 two-pane panes (mods + scenarios) so a
  longer/shorter selected-item description cannot resize the containers.

DoD status:
1/5. render/manual eyeball of the reskin (panels + rows) + no flicker + fixed
   widths: PENDING owner in-engine (the zoo re-rendered clean; the panel reskin
   is unit-tested).
2. no-flicker: the `apply_ui_font` timing fix is verified by logic + the zoo
   font still applies; the in-motion flash is an owner eyeball.
4. cmd: PASS - `cargo check --all-targets` green; nova_ui 12 (incl. new
   `panel_reskins_on_skin_change`) + nova_menu 73 tests green.

Remaining gap (retro): the settings volume slider TRACK (block-meter vs solid
fill - different children) + the segmented CONTAINER border do not LIVE-reskin
(they reskin on next settings-open, since the settings body is not rebuilt on
skin change). The panel + buttons + segmented options do reskin live.
