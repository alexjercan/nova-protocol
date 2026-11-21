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

## Map (2026-11-21)
Paths and lines are at HEAD a82db744d, before the change.
- Owner: `predict_flight_path` (`crates/nova_ship/src/flight/prediction.rs`)
  writes `FlightPrediction` on the player ship. Readers that draw a route:
  `sync_trajectory_ribbon` (`crates/nova_hud/src/holo_instruments.rs`) and
  `project_map_route` (`crates/nova_interface/src/map/scene.rs`). The flip
  gate (`holo_instruments.rs`) reads points but draws no route.
- Straight line 1, 30 s horizon: `PREDICTION_HORIZON = 30.0`
  (`prediction.rs:38`) ends a run with `Horizon` (`prediction.rs:113`). A
  leg longer than 30 s has no path past that point.
- Straight line 2, Map guide: `project_map_route` (`scene.rs:626-643`,
  `scene.rs:772-781`) draws a stroke at alpha 0.3 from the path end, or from
  the player blip with no path, to the live target. `clip_predicted_path`
  (`scene.rs:911`) returns empty for a moving target so the guide runs from
  the blip.
- Straight line 3, HUD guide: `sync_trajectory_ribbon` appends the live
  target (`holo_instruments.rs:298-313`) and draws that last segment with
  `GUIDE_ALPHA = 0.3` (`holo_instruments.rs:44`, `:336-339`). With no
  prediction the ribbon is the one guide segment from the ship.
- Straight line 4, path removal: the target departure checks
  (`prediction.rs:58-63`, `:684-707`) remove `FlightPrediction`
  (`prediction.rs:695`) and reseed when the target changes course or spin.
  Lines 2 and 3 then draw alone. A target that keeps maneuvering keeps the
  path hidden.
- Callers of `FlightPredictionEndType::Horizon`: the two examples
  `loop_predicted_path` and `screenshot_goto_moving_target` assert the guide.
  `nova_probe` and `nova_bench` do not read the prediction.
- Cap sizing: open-world cells are 32 km with active radius 2, so a loaded
  target is at most about 166 km away. A start hull accelerates at 46.3 to
  77.6 m/s2 (`tasks/20261116-132143/CENSUS-AFTER.md`): 2*sqrt(166 km / 46.3)
  = 120 s. A cap of 1800 s fails only below 0.21 m/s2.
- Found after the change: a GOTO at a target on a circular orbit of a well
  never arrives live. The ship trails 55 to 80 u back for 937 s
  (`~/Pictures/nova-goto-path/orbiting-target-goto-experiment.log`), so the
  run reaches the cap and no path shows. `screenshot_goto_moving_target`
  fails on it. Marked `FIXME` at `PREDICTION_CAP_SECONDS`.

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
