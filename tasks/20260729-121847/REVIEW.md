# REVIEW - Menu UI polish: skin reactivity, flicker, fixed widths (121847)

- ROUND: 1 (out-of-context reviewer, fresh context)
- VERDICT: APPROVE (no MAJOR/MINOR)
- DATE: 2026-07-29

All three fixes verified against bevy_ui 0.19 source:
- `apply_ui_font` -> `PostUpdate.before(UiSystems::Content)` sets the font before
  `measure_text_system`/`text_system` run (both in `Content`), the SAME frame -
  `Added<UiText>` still fires for Update-spawned entities (trackers clear only at
  end of the full app update). Kills the 1-frame default-face flash.
- Reskin-refresh is loop-free: `skin.is_changed()` is one frame; the refreshes
  write `SelectedModId`/`SelectedScenarioId` only under a stale-selection guard,
  so `selected.is_changed()` does not self-retrigger in steady state.
- `reconcile_panel_skins` mirrors the button reconciler (Added override as a
  SYSTEM, try_insert, disjoint `With<PanelSkin>` query); `panel()` still Node-less
  and every call-site supplies its own Node.
- `min_width: px(0)` is on 6 flex CONTAINER panes only (40% left panes + flex-grow
  lists/detail panes) - the standard flexbox fix; no leaf node collapses.

## Findings + resolution

- NIT (mixed-state nuance): on a LIVE skin flip with the settings panel open, the
  segmented OPTION buttons repaint (via reconcile_button_skins) but their
  `segmented_container` border+radius AND the `slider_track` (block-meter vs
  solid) do NOT - a partial/mixed reskin, since `build_settings_body` re-runs
  only on settings-open, not on `skin.is_changed()`. RECORDED in the RETRO as
  the remaining gap (a settings-body rebuild-on-skin, or a slider/segmented
  reconciler, is the follow-up). Not a code change in this fix.
