# RETRO - Menu UI polish: full skin reactivity, fix flicker, fixed panel widths

- TASK: 20260729-121847 (follow-up to 20260729-105359, from owner eyeball)
- DATE: 2026-07-29
- OUTCOME: shipped; review APPROVE; owner in-engine confirmation pending.

## What shipped

Three owner-reported fixes after the widget adoption: (1) panels now live-reskin
on a UiSkin flip (`PanelSkin` + `reconcile_panel_skins`; mods/scenarios lists
rebuild on skin change so their rows/checkboxes/badges re-spawn); (2) the
"text gets bigger for a split second" flash is gone (`apply_ui_font` moved to
`PostUpdate` before UI text measure); (3) two-pane panel widths are pinned
(`min_width:0`) so a long/short selected-mod description can't resize them.

## What went well

- Every one of the three issues traced to a single, specific mechanism, so each
  fix was small and targeted: a missing reconciler (panels), a deferred-by-one-
  frame system (font), and flexbox `min-width:auto` (widths). Naming the exact
  mechanism before touching code kept the diff tiny.
- `reconcile_panel_skins` + `panel_paint` reused the exact button-reconciler
  shape, so the panel reskin was correct-by-construction and got a passing
  live-tree test on the first try.

## What went wrong / difficulties

- The font flash was subtle: `apply_ui_font` ran in `Update` on `Added<UiText>`,
  so any text spawned THIS frame rendered one frame in the LARGER default face
  before Iosevka applied next frame - visible as a flicker on every respawn (the
  mods detail pane / Enable button on a checkbox toggle). The fix is a schedule
  placement, not logic: run the font router in `PostUpdate` before
  `UiSystems::Content` so the font is set before the text is measured.
- Not everything reskins live yet: the settings volume-slider TRACK (a phosphor
  block-meter vs a hardware solid fill - DIFFERENT children) and the
  `segmented_container` border+radius do NOT live-reskin, because
  `build_settings_body` re-runs only on settings-open, not on `skin.is_changed()`.
  So a live flip WITH settings open gives a MIXED state: the segment buttons
  repaint (button reconciler) inside a stale-radius/stale-border container, and
  the slider stays a block-meter. Deferred (see follow-up) - the panels, buttons,
  rows, badges, and checkboxes all reskin live, which was the owner's complaint.

## Lessons (for the ledger)

- `deferred-font-system-flashes-default-face` (domain): a system that assigns
  `TextFont.font` on `Added<Text>` in `Update` renders newly-spawned text ONE
  frame in the default face (larger metrics) before it applies - a visible flash
  on every respawn. Run font-assignment in `PostUpdate` BEFORE
  `UiSystems::Content` (text measure/layout) so the font is set the same frame
  the text is laid out. 20260729-121847.
- `flex-item-needs-min-width-0-to-not-grow-with-content` (domain): a `flex_grow`
  or fixed-`width%` pane keeps its default `min-width: auto` = min-content, so a
  long unbreakable child stretches the pane (and its container). `min_width:
  px(0)` lets it hold its flex share and wrap the content. The standard fix for
  "selecting a long item resizes the panel". 20260729-121847.
- `static-factory-widget-needs-a-skin-reconciler-to-live-reskin` (x1): a widget
  spawned by a factory with a baked skin (panel, slider_track, segmented
  container) does NOT restyle when the global `UiSkin` flips unless it has a
  reconciler (like ThemedButton/ListRow) OR its screen is rebuilt on skin change.
  "Reskin everything live" means a reconciler per skin-structural widget, or a
  rebuild-on-skin for its screen. 20260729-121847.

## Follow-ups

- Settings body live-reskin: rebuild `build_settings_body` on `skin.is_changed()`
  (or add a `slider_track` + `segmented_container` reconciler) so the volume
  slider + segmented container reskin live, not just on next open.
