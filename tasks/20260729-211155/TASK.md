# nova_ui slider track: live re-skin + hardware fill follows the value

- STATUS: OPEN
- PRIORITY: 51
- TAGS: v0.9.0,feedback,bug,ui

## Story

Owner playtest (2026-07-29) of the settings screen: "the audio slider doesn't
change to its style when switching phosphor/hardware, and the hardware variant
doesn't move the slider - it stays fixed, but the value changes correctly".

Two defects in `nova_ui`'s `slider_track` widget, both known-shaped:

1. No live re-skin. `slider_track(fraction, skin)` bakes the skin into the
   node (height/radius/padding/gap) AND into its children (phosphor: N
   `SliderBlock` bars; hardware: one solid fill child). Nothing reconciles it
   on a `UiSkin` flip - unlike `panel()`/`button()`/`list_row` - and the
   settings body is not rebuilt on a skin change, so the track only adopts the
   new skin on the next settings-open. This is the exact residual gap task
   20260729-121847's retro recorded.
2. The hardware fill never moves. `sync_slider_meters` only recolours
   `SliderBlock` children (the phosphor meter). The hardware variant's fill
   child is an unmarked node with a baked `width: percent(fraction * 100)`, so
   dragging updates `SliderValue` (and the `NN%` label) while the bar stays
   put.

## Steps

- [ ] Reproduce first: two nova_ui live-tree tests that FAIL on master -
      (a) a hardware-skin slider's fill child width does not follow a changed
      `SliderValue`; (b) a slider's track does not repaint/rebuild when
      `UiSkin` flips (assert the skin-distinguishing property: the phosphor
      block children vs the hardware single fill, plus the track height).
- [ ] Mark the hardware fill (`SliderFill`) and drive its width from the same
      value-sync system that lights the phosphor meter, so ONE system owns
      "value changed -> track shows it" for both variants.
- [ ] Add a `SliderTrackSkin` marker + reconciler mirroring the existing
      `PanelSkin`/`ListRow` reconcilers: on a `UiSkin` change, repaint the
      track node (height, radius, padding, gap, bg, border) and REBUILD its
      children for the new variant, then immediately re-apply the current
      `SliderValue` so the rebuilt track shows the right fill/lit blocks (see
      `rebuilt-view-writes-go-to-state-not-the-entity` in LESSONS).
- [ ] Sweep the callers, not just the widget: the settings volume slider
      (`nova_menu`) and the widget zoo both wear `slider_track` - confirm each
      end-to-end, and check whether the segmented CONTAINER border (the other
      half of 20260729-121847's recorded gap) rides the same fix or needs its
      own reconciler; do the container too if it is the same mechanism.
- [ ] Verify by RUNNING the widget zoo / the settings screen (Xvfb) and
      flipping the skin, dragging the slider in BOTH skins.

## Definition of Done

1. test: `cargo test -p nova_ui` - both new tests pass (each failed first).
2. cmd: `nix develop --command cargo check --all-targets` green; `cargo test -p
   nova_menu` still green.
3. render eyeball: the widget zoo / settings RUN in-engine - flipping the skin
   restyles the track live, and dragging moves the bar in BOTH skins.
4. manual: owner confirms in-engine.

## Notes

- Follow-up to 20260729-121847, which fixed panels/buttons/rows and explicitly
  parked the slider track + segmented container as the remaining gap.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
