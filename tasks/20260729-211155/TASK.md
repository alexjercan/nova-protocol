# nova_ui slider track: live re-skin + hardware fill follows the value

- STATUS: CLOSED
- PRIORITY: 51
- TAGS: v0.9.0,feedback,bug,ui
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

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

- [x] Reproduce first: two nova_ui live-tree tests that FAIL on master -
      (a) a hardware-skin slider's fill child width does not follow a changed
      `SliderValue`; (b) a slider's track does not repaint/rebuild when
      `UiSkin` flips (assert the skin-distinguishing property: the phosphor
      block children vs the hardware single fill, plus the track height).
- [x] Mark the hardware fill (`SliderFill`) and drive its width from the same
      value-sync system that lights the phosphor meter, so ONE system owns
      "value changed -> track shows it" for both variants.
- [x] Add a `SliderTrackSkin` marker + reconciler mirroring the existing
      `PanelSkin`/`ListRow` reconcilers: on a `UiSkin` change, repaint the
      track node (height, radius, padding, gap, bg, border) and REBUILD its
      children for the new variant, then immediately re-apply the current
      `SliderValue` so the rebuilt track shows the right fill/lit blocks (see
      `rebuilt-view-writes-go-to-state-not-the-entity` in LESSONS).
- [x] Sweep the callers, not just the widget: the settings volume slider
      (`nova_menu`) and the widget zoo both wear `slider_track` - confirm each
      end-to-end, and check whether the segmented CONTAINER border (the other
      half of 20260729-121847's recorded gap) rides the same fix or needs its
      own reconciler; do the container too if it is the same mechanism.
- [x] Verify by RUNNING the widget zoo / the settings screen (Xvfb) and
      flipping the skin, dragging the slider in BOTH skins.

## Definition of Done

1. test: `cargo test -p nova_ui` - both new tests pass (each failed first).
2. cmd: `nix develop --command cargo check --all-targets` green; `cargo test -p
   nova_menu` still green.
3. render eyeball: the widget zoo / settings RUN in-engine - flipping the skin
   restyles the track live, and dragging moves the bar in BOTH skins.
4. manual: owner confirms in-engine.

## Close-out (2026-07-30)

Shipped in `nova_ui::widget`:

- `SliderFill` marks the hardware track's solid fill, and `sync_slider_meters`
  became `sync_slider_tracks` - ONE system now lights the phosphor block-meter
  AND moves the hardware fill. It derives its fraction from `SliderRange`
  (`slider_fraction`), so it WATCHES the range as well as the value: widening a
  range at a steady value moves the fill, because what the value means changed.
  A bare `SliderValue` with no range keeps the old normalized reading.
- `SliderTrackSkin(f32)` + `reconcile_slider_track_skins` repaint the track's
  own node and RESPAWN its children for the new skin (the two skins are
  structurally different widgets, so a colour swap cannot carry it). The
  reconciler spawns the new children AT the right fraction: it prefers the live
  `SliderValue`, and falls back to the fraction the marker remembers for a
  DISPLAY-ONLY track, which has no slider at all and used to silently empty
  itself on a flip. `sync_slider_tracks` is ordered `.after` it and also sees
  the rebuild via `Changed<Children>`, so the value path is belt-and-braces.
  The remembered fraction lives on the surviving track, never on the children
  the rebuild throws away (`rebuilt-view-writes-go-to-state-not-the-entity`).
- The rebuild is ONE silenced entity command
  (`rebuild_slider_track_children` + `queue_silenced`). A caller may despawn a
  track's subtree on the same `UiSkin` change (the widget zoo rebuilds its whole
  body) with no ordering edge to force a flush between them, so neither half may
  panic on a dead entity. A split `despawn_related().try_insert(..)` does NOT
  achieve this - `try_insert` is silenced, `despawn_related` is not.
- The sibling half of the recorded gap rode the same pass: `SegmentedSkin` +
  `reconcile_segmented_skins` repaint the segmented CONTAINER (its options were
  already `ThemedButton`s, which is what made the row read as half-reskinned).
- `slider_track` / `segmented_container` now build from shared per-skin sources
  (`slider_track_node`, `slider_track_colors`, `spawn_slider_track_children`,
  `segmented_container_paint`), so a rebuilt widget cannot drift from a spawned
  one.

Evidence:

- 6 new nova_ui live-tree tests + 1 nova_menu caller test on the SHIPPED volume
  slider (`pin-each-caller-not-just-shared-core`). Every one was falsified
  against the pre-fix code and goes red, so none is a no-op pass - including the
  despawn-race pin, which required turning the schedule's
  `auto_insert_apply_deferred` OFF to make the ordering accident deterministic.
- `cargo test -p nova_ui --lib` 21 passed; `cargo test -p nova_menu --lib` 74
  passed; `cargo check --workspace --all-targets` clean, no new warnings.
- Render eyeball (Xvfb, real GPU): `NOVA_ZOO_CAPTURE=1` widget_zoo in both
  skins - hardware draws the solid fill AT the 0.66 value, phosphor lights 16
  of 24 blocks. The zoo's top-bar segmented control is spawned once in `setup`
  and never rebuilt, so its visibly different face across the two captures is
  in-engine proof of `reconcile_segmented_skins` reskinning LIVE.

No CHANGELOG entry: the UI-skin feature these widgets belong to is itself still
in `[Unreleased]`, so this fixes something no release ever shipped - same call
the sibling fix c235a429 made.

Caveat found while verifying (and corrected in review round 3, after a first
diagnosis that was WRONG): a BARE `cargo check --all-targets` at the repo root
green-lit a nova_menu test module that would not build. The cause is not that
`--all-targets` ignores `#[cfg(test)]` - it does not. The root Cargo.toml is a
PACKAGE with deliberately no `default-members` (documented at Cargo.toml:274),
so a bare invocation scopes `--all-targets` to the ROOT package's targets and
never builds member crates' test targets. `cargo check --workspace
--all-targets` (what CI runs) and `-p <crate> --all-targets` both catch it.

## Notes

- Follow-up to 20260729-121847, which fixed panels/buttons/rows and explicitly
  parked the slider track + segmented container as the remaining gap.
