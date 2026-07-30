# Floating chip background covers only a corner of its label

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.9.0,bug,ui,hud,feedback

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

## Steps

- [ ] Reproduce first: a live-tree layout rig that spawns the objective marker
      chip with a real label and asserts the chip's `ComputedNode` width is at
      least the rendered text width (i.e. the fill covers the label). Watch it
      fail with the collapsed slab, and RECORD the measured numbers in NOTES.md
      - they are the fail-first evidence.
- [ ] Confirm the mechanism against the actual tree rather than the theory:
      dump the chip's computed size with and without the children present, so
      the fix is aimed at the real cause.
- [ ] Fix the shape: the chip entity keeps `chip_node()` + `chip_paint` and
      becomes a pure container; the label moves into its own `Text` CHILD. Keep
      the diamond/arrow as absolute siblings of that text child so their
      hand-tuned offsets keep meaning the same thing (re-derive them if the new
      parent origin moves).
- [ ] Whoever writes the label text must follow it to the child: the
      `update_*_labels` systems query `Text` on the chip entity today.
- [ ] Sweep for the same shape elsewhere before closing: any node carrying both
      `Text` and `children!` in the HUD/UI crates
      (cmd in DoD 3). Fix every hit or record why it is benign.
- [ ] Add the invariant as a shared rig assertion rather than a per-site
      constant, so a future chip cannot silently regress.
- [ ] Screenshot the result and LOOK at it - a layout fix is unverified until
      someone sees it rendered (ledger: eyeball the rendered output).

## Definition of Done

1. The objective-marker chip's background/border covers its whole label plus
   the chip padding (test: the layout rig from step 1, which failed first).
2. The beacon chip does the same (test: same rig, second case).
3. No HUD/UI node carries `Text` alongside `children!` without a deliberate,
   commented reason (cmd:
   `rg -n --multiline 'Text::new[^;]*\n(?s).{0,600}?children!' crates/nova_gameplay/src/hud crates/nova_ui/src`
   reviewed by hand; excludes `tasks/`).
4. A screenshot example shows the objective chip fully backed (cmd:
   `cargo test --test examples_smoke screenshots`, plus the capture eyeballed).
5. Owner sees a full background behind "BEACON 1" in the Shakedown run
   (manual).

## Notes

Sits under epic 20260728-175719 (UI rework); a defect in the chip family that
epic shipped.

## Flow State

- FLOW STEP: PLANNED
