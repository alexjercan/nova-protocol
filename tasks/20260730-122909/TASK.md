# Floating chip background covers only a corner of its label

- PRIORITY: 48
- TAGS: v0.9.0, bug, ui, hud, feedback
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

Owner playtest 2026-07-30 (feedback wave on the v0.9.0 UI rework):

> there is an attempt to add a background rectangle for floating labels (e.g
> the text for objective "BEACON 1" in Shakedown run, there is a label square
> in top left of the text but doesn't cover the entire text, I assume it's a
> bug and it should be a full background for the text)

The world-anchored chips (objective marker, beacon chip) are supposed to be
members of the phosphor chip family: a bordered, filled pill hugging its text.
On screen the fill/border is a small box in the label's top-left corner and the
text spills out of it unbacked.

## Understanding (2026-07-30) - the suspected mechanism

Both offenders put `Text` on the SAME entity as `chip_node()` AND give that
entity children:

- `crates/nova_gameplay/src/hud/objective_markers.rs`
  `objective_marker_chip_hud`: `screen_indicator_node(.., chip_node())` +
  `chip_paint(ChipTone::Amber)` + `Text::new("")` +
  `children![objective_marker_diamond(), objective_marker_arrow()]`.
- `crates/nova_gameplay/src/hud/beacon_chips.rs` `beacon_chip_hud`: the same
  shape with `ChipTone::Phosphor` + `children![beacon_chip_arrow()]`.

Taffy only calls a node's measure function when the node is a LEAF. Give a text
node children (even `PositionType::Absolute` ones) and it becomes a container,
the text measure is dropped, and the box collapses to its in-flow content
(nothing) plus `chip_node`'s padding - an ~18x8 px slab at the origin - while
the text still RENDERS at full length over it. That matches the report exactly,
including "top left".

This is a hypothesis until the rig proves it. Reproduce FIRST.

The sizing path is not the suspect: `ScreenIndicatorSize::Content` deliberately
leaves width/height to UI layout (`screen_indicator.rs`, "Content leaves the box
to UI layout"), so the collapse is upstream of the indicator.

### Planning notes (2026-07-30)

Source-level confirmations gathered before planning (the rig still has to
reproduce it empirically):

- `chip_node()` is `Display::Flex` with `padding: axes(9, 4)` and a 1 px border
  (`crates/nova_ui/src/hud.rs`), so a childless-but-textless chip computes to
  20x10 px - the reported corner slab.
- taffy runs a node's measure function only on the leaf path
  (`bevy_ui-0.19.0/src/layout/ui_surface.rs:131` registers the measure as leaf
  context; `compute_layout_with_measure` only reaches it for childless nodes).
- The indicator CENTERS the box on the anchor (`node.left = center.x - size.x/2`,
  `screen_indicator.rs:553`), so the collapsed slab sits at the anchor while the
  text renders rightwards from its content origin - "square in the top left".
- The chevron is found by a DESCENDANT walk (`update_arrows`), which writes only
  `UiTransform::rotation` - a `translation` authored on the arrow survives.
- No live-tree taffy rig exists in the repo yet (no test touches `UiPlugin`), so
  the rig in step 1 is new. bevy's `default_font` feature is on by default and
  the chips pass no font handle, so headless text measurement is
  production-faithful.
- No screenshot example currently spawns an `ObjectiveMarkerTarget`;
  `screenshot_combat` spawns a nav beacon (so it already frames a beacon chip).

Shape decision (owner-confirmed, see DECISION.md): both chips become pure
containers with a `Text` CHILD, and the objective chip's diamond moves INSIDE
the pill as an in-flow flex item.

## Steps

- [x] Reproduce first: a live-tree layout rig (new `#[cfg(test)]` support module
      under `crates/nova_gameplay/src/hud/`) that builds an App with the real
      taffy layout + text measurement, spawns the objective marker chip bundle
      with a real label, runs a frame and asserts the chip's `ComputedNode`
      covers its text child. Watch it fail on the collapsed slab and RECORD the
      measured numbers in NOTES.md - that is the fail-first evidence.
- [x] Confirm the mechanism against the actual tree rather than the theory: dump
      the chip's computed size with the children present vs removed, so the fix
      is aimed at the measured cause.
- [x] Make the assertion shared and DERIVED, not a per-site constant: the helper
      asserts `chip.size == text_child.size + chip.padding + chip.border` read
      off the live `ComputedNode`s (no re-multiplied em fractions, ledger
      `test-must-not-reuse-the-formula-under-test`), and both chips call it.
- [x] Fix `objective_marker_chip_hud`: the indicator entity keeps `chip_node()`
      + `chip_paint` + the anchor and drops every `Text*` component; the label
      becomes a `Text` child carrying `TextFont`/`TextLayout`/`TextColor`/
      `TextShadow` and a new text marker; the diamond becomes an IN-FLOW flex
      child left of it (dropping its absolute offsets for `chip_node`'s
      `column_gap`/`align_items`); the chevron stays an absolute child of the
      chip and its `left` is re-derived to center over the now-full-width pill
      (`Percent(50)` plus a `UiTransform::translation` of `-ARROW_PX/2`, which
      `update_arrows` preserves).
- [x] Fix `beacon_chip_hud` the same way (no diamond; chevron re-derived).
- [x] Follow the text to the child: `update_objective_marker_labels` and
      `update_beacon_chip_labels` query `Text` on the chip entity today and
      reach the layer via one `ChildOf` hop - both need the extra hop. Keep the
      existing `*ChipLabelMarker` on the chip entity (it is the node carrying
      `ScreenIndicatorAnchor`, which the beacon suppress/restore observers and
      their tests query) and update the in-module tests.
- [x] Sweep for the same shape elsewhere before closing: any node carrying both
      `Text` and `children!` in the HUD/UI crates (cmd in DoD 3, reviewed by
      hand - the multiline grep also matches sibling-then-children shapes that
      are benign). Fix every real hit or record why it is benign.
- [x] Give the objective chip a capture: extend `screenshot_combat` so one frame
      carries both a marked objective entity (gold chip) and the plain nav
      beacon (cyan chip) - the beacon-chip dedupe means one entity cannot show
      both.
- [x] Screenshot the result and LOOK at it - a layout fix is unverified until
      someone sees it rendered (ledger `render-output-eyeball`). Run the example
      under Xvfb, open the capture, confirm both pills are full-width.

## Definition of Done

1. The objective-marker chip's background/border covers its whole label plus
   the chip padding, asserted against live `ComputedNode` values (test: the
   layout rig from step 1, which failed first).
2. The beacon chip does the same through the SAME shared helper (test: same
   rig, second case).
3. No HUD/UI node carries `Text` alongside `children!` without a deliberate,
   commented reason (cmd:
   `rg -n --multiline 'Text::new[^;]*\n(?s).{0,600}?children!' crates/nova_gameplay/src/hud crates/nova_ui/src`
   reviewed by hand; excludes `tasks/`).
4. A screenshot example shows the objective chip AND the beacon chip fully
   backed (cmd: `cargo test --test examples_smoke screenshots`, plus the
   capture eyeballed - the fix is not verified until someone sees the pills).
5. The chevron still parks centered above each pill and the diamond sits inside
   the objective pill's fill (test: the rig asserts the diamond is an in-flow
   child inside the chip's content box; the capture from DoD 4 eyeballed).
6. Owner sees a full background behind "BEACON 1" in the Shakedown run
   (manual).

## Outcome (2026-07-30)

Reproduced, fixed and eyeballed; measured numbers in `NOTES.md`.

- The chip collapsed to exactly `chip_node()`'s frame (20x10 px) because the
  node carrying `Text` also carried `children!`, which takes it off taffy's
  leaf path and drops the text measure. Confirmed against the engine, not
  inferred: the rig lays the same bundle out as a leaf and as a container.
- Both chips are now pure containers with a leaf `Text` child; the objective
  chip's diamond became an in-flow flex item inside the pill (owner decision,
  `DECISION.md`) and each chevron centres over the real pill width via
  `left: Percent(50)` + a `UiTransform` translation `update_arrows` preserves.
- Post-fix objective chip measures 94x25 with its 58x15 label fully inside the
  content box; the beacon chip passes the same shared assertion.
- Review renamed the chip-node markers: the Steps above and `DECISION.md` say
  `*ChipLabelMarker`, which after this change would have named the CONTAINER
  while `*ChipTextMarker` named the label. They ship as
  `ObjectiveMarkerChipNodeMarker` / `BeaconChipNodeMarker` (review R1.4); the
  planning text above is left verbatim as history.
- New live-tree layout rig: `crates/nova_gameplay/src/hud/chip_layout_rig.rs`.
  It is the first test in this repo to run the real `UiPlugin` layout, so it is
  reusable for any future HUD layout question.
- `screenshot_combat` grew a `hud-nav-chips.png` beat framing both chips at
  once; capture eyeballed under Xvfb on the real GPU.

## Notes

Sits under epic 20260728-175719 (UI rework); a defect in the chip family that
epic shipped.
