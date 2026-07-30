# DECISION: the fixed chip shape (text child + in-pill diamond)

- STATUS: ACCEPTED
- ACCEPTED BY: owner at the 2026-07-30 plan gate

## The fork

The chip's fill/border collapses because the entity carrying `Text` also carries
`children!`, which takes it off taffy's leaf path and drops the text measure.
Fixing that means the chip entity can no longer BE the text node, which forces
two load-bearing choices:

1. Where does the label live? Options: (a) move the label into a `Text` CHILD and
   leave the chip entity as the bordered container; (b) keep `Text` on the chip
   entity and move the diamond/chevron OUT to be siblings of the chip under the
   indicator layer.
2. Where does the objective chip's diamond glyph sit? Today it is
   `PositionType::Absolute` at `left: -(8 + 6) px` - OUTSIDE the box. That read
   as "attached" only because the box was a collapsed 20x10 slab at the same
   origin. With a real full-width pill the two wants are mutually exclusive: an
   absolute -14 px diamond is visibly detached from the bordered fill, and an
   in-flow diamond inside the pill makes the pill ~16 px wider.

## The decision

1. **(a) the label moves into a `Text` child.** The chip entity is the thing the
   screen-indicator widget owns (`ScreenIndicatorAnchor`, `Node.left/top`,
   `ScreenIndicatorSize::Content`) and the thing `chip_paint` paints, so it must
   stay one node. Option (b) would hoist the chevron out from under the chip,
   putting the widget's arrow walk and the chevron's offsets on a node whose
   origin is the layer, not the chip - a bigger change to the widget contract
   for no gain.
2. **The diamond moves INSIDE the pill** as an in-flow flex item left of the
   text, using `chip_node`'s existing `column_gap: 8` and `align_items: Center`
   instead of hand-authored absolute offsets. Owner picked this at the plan gate:
   it is the chip family's glyph+text idiom and the mark ends up backed by the
   same fill and border as the label. The pill gets ~16 px wider; that is
   accepted.

The chevron stays an absolute child of the chip (the widget finds any
descendant), but its `left` is re-derived: `-ARROW_PX/2` used to sit near the
collapsed slab's origin, and over a full-width pill it would hang off the left
edge. It becomes `Percent(50)` plus a `UiTransform::translation` of
`-ARROW_PX/2` - `update_arrows` writes only `.rotation`, so the translation
survives.

## Consequences

- New text markers for the label nodes; the existing `*ChipLabelMarker` stays on
  the chip entity because it identifies the anchor-carrying node that the beacon
  suppress/restore observers query.
- `update_objective_marker_labels` / `update_beacon_chip_labels` gain one
  `ChildOf` hop to reach the layer that holds the target entity.
- Both chips get ~16 px (objective) / same-width (beacon) wider footprints, which
  the `Content` sizing path absorbs; the indicator keeps centering the box.
