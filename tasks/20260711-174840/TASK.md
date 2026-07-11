# Bigger edge indicators with target info

- STATUS: OPEN
- PRIORITY: 90
- TAGS: hud, feedback

User feedback (20260711, playtest of 20260708-165704): "the side HUD
indicator is kind of small it can be bigger maybe with some information on
it" - the edge-clamped arrows read too small, and could carry data.

## Goal

Edge arrows are visibly larger, and each clamped indicator shows the
distance to its target next to the arrow, tinted like the arrow. On-screen
targets still render nothing.

## Steps

- [ ] Scale up the chevron in hud/edge_indicators.rs: ARROW_PX 14 -> 24,
      STROKE_LEN_PX 9 -> 16, STROKE_THICK_PX 2 -> 3 (recompute the stroke
      placement so the apex stays top-center); bump EDGE_MARGIN_PX 24 -> 30
      so the label below fits inside the edge.
- [ ] Distance label: a Text child of the indicator (sibling of the arrow,
      NOT inside it - the widget rotates the arrow node), marked
      `EdgeIndicatorLabelMarker`, font ~10px, `TextColor` = the arrow tint,
      positioned under the indicator (top: 100%, centered).
- [ ] Driver system `update_edge_labels` (Update, NovaHudSystems): per
      indicator, set the label text to the player-to-target distance
      formatted `{:.0}m` like the readout (torpedo_target.rs
      distance_line convention, no DST prefix), and mirror the arrow's
      visibility onto the label so the label only shows while the arrow is
      clamped (the widget owns arrow visibility; the label follows it).
- [ ] Tests: label text tracks distance (move the target, re-run, text
      changes - delivery guard on the formatted value); label visibility
      mirrors the arrow (arrow Hidden -> label Hidden, arrow Inherited ->
      label Inherited); structure test updated for the second child.
- [ ] Full check suite (check/fmt + touched filters); note the change in
      the weapons-hud spike Fix record line for 165704.

## Notes

- Relevant files: crates/nova_gameplay/src/hud/edge_indicators.rs,
  crates/nova_gameplay/src/hud/torpedo_target.rs (format convention),
  crates/nova_gameplay/src/hud/screen_indicator.rs (arrow visibility
  contract: Inherited while clamped, Hidden on-screen).
- Distance origin: the player ship root's GlobalTransform (readout uses
  live-structure anchor; root translation is fine at edge-arrow precision).
- The label reads sideways-clamped indicators fine; do not counter-rotate
  or dodge edges beyond the margin bump in v1.
