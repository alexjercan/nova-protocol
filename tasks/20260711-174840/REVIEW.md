# Review: Bigger edge indicators with target info

- TASK: 20260711-174840
- BRANCH: feature/edge-indicator-info

## Round 1

- VERDICT: APPROVE

Verified independently: hud:: 75/75 green, cargo check --workspace green,
fmt clean. Re-derived the label placement constraint from the widget code:
the widget rotates the ARROW node via UiTransform (update_arrows), so the
label as a sibling under the un-rotated indicator node is correct - inside
the arrow it would spin with the chevron. The visibility mirror matches
the widget's documented arrow contract (Inherited while clamped, Hidden
on-screen), and the new test drives both directions plus a live distance
change with a positive text assertion.

Findings (non-blocking):

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/edge_indicators.rs -
  update_edge_labels runs in Update, but the widget writes the arrow's
  Visibility in PostUpdate (ScreenIndicatorSystems), so the label mirrors
  it one frame late: when an arrow first clamps, its label appears a frame
  after (and can flash one frame of stale text when re-shown). Move the
  system to PostUpdate, after ScreenIndicatorSystems and before
  UiSystems::Layout - the same slot reasoning the widget itself documents.
  Distance staleness is irrelevant at label precision.
  - Response: fixed - update_edge_labels moved to PostUpdate,
    .after(ScreenIndicatorSystems).before(UiSystems::Layout), with the
    slot reasoning in a comment.

## Round 2

- VERDICT: APPROVE

R1.1 verified fixed in the plugin registration (same-frame mirror, before
layout). hud::edge_indicators 6/6 green, workspace check + fmt clean. No
new findings.
