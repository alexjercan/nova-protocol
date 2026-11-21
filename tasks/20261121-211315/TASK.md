# GOTO line: always draw the computed flight path, never a straight line

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.19.0, navigation, hud

## User facts
- Owner 2026-11-21, angry, about the fifth request: "when I do a GOTO and
  we show the blue line that indicates the flight path WE MUST ALWAYS ...
  SHOW the ... real path, not a straight line, we don't use a ... fallback
  we must always compute it correctly and show the path correctly!!"
- Earlier asks: tasks 20260731-232634 and 20260927-224016 (closed) replaced
  the straight ribbon with a predicted path, but a straight guide stayed.

## Agent findings
- Map route (`crates/nova_interface/src/map/scene.rs:630-643`,
  `project_map_route`): draws the predicted path, then "a dim straight
  guide from that closest approach to the live target", and with no
  prediction or every point flown "the guide runs from the player blip
  instead". `clip_predicted_path` (`scene.rs:900-911`) returns empty to make
  the caller fall back to the straight guide.
- The prediction stops at `PREDICTION_HORIZON = 30.0` s
  (`crates/nova_ship/src/flight/prediction.rs:38`), so any GOTO longer than
  30 s shows a straight line for the rest.
- The world HUD ribbon (`crates/nova_hud/src/holo_instruments.rs`) also
  reads the prediction; check it for the same fallback.

## Delivery
1. The prediction runs to arrival, not to a fixed horizon, within a
   per-frame budget across frames (the run already spreads over updates).
2. Delete every straight-line guide and fallback in the Map route and the
   HUD ribbon. No prediction yet means no line yet, never a straight one.
3. A prediction that cannot reach the target is a defect that fails loudly
   (log error and test), not a hidden straight line.

## Verification
- Tests: a GOTO longer than 30 s draws a path that ends at the arrival
  point; no code path draws a straight guide; a fresh GOTO draws the real
  path once the first prediction lands.
- Frames of the Map and the HUD ribbon on a long curved GOTO.

## Done when
- Every GOTO line in the Map and the HUD is the computed flight path end to
  end, with no straight-line fallback anywhere, and the owner has checked it.
