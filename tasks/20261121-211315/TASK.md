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
  run reaches the cap and no path shows. Cause: GOTO aimed to rest in the
  world frame. Fixed: a GOTO now flies in its target's frame (relative
  velocity, tidal pull for a falling target), so it arrives at the
  target's velocity. Proof: `a_goto_at_a_target_orbiting_a_planet_*` in
  `flight/tests/goto.rs` and `flight/tests/prediction.rs`.
- Static leg end: the flown ship passes the beacon by 83 u in x and turns
  back to stop 67.5 u off its centre (`loop_predicted_path`, full leg). The
  prediction matches the flown track within 0.8 u, so the hook is the
  autopilot's flight, not the line.

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

## Follow-up (2026-11-22): docked pair shows no line
- Owner: "when I am docked to a 'derelict' or I take helm I don't see the
  flight path we should also fix that, as a follow up of something that
  wasn't done in the task".
- Cause: `predict_flight` skips any root with `DockedShip`
  (`crates/nova_ship/src/flight/prediction.rs:610`, `!docked`), since #107
  (fa2c58d2d). The prediction models one rigid body; a docked pair flies as
  two jointed roots with the helm wrench spread by
  `apply_docked_helm_wrench`. This task's proof used single ships only.
- Done when: a GOTO flown by a docked pair, either root at the helm, shows
  the predicted flight of the pair to arrival on the HUD and Map; the owner
  has checked it.

### Follow-up result (2026-11-22)
- Map of the docked flight: `measure_docked_assemblies` (FixedUpdate, before
  physics) writes `DockedShip::drives` and `DockedAssembly`; only the driver's
  thrusters burn, its PD output is spread as one rigid wrench
  (`apply_docked_helm_wrench`), RCS gives every root the same delta-v, and
  the autopilot flies `flight_body(.., Some(assembly))`. Each root feels its
  own well pull at its own origin. The predictor runs in FixedPostUpdate, so
  it re-measures the assembly from live root state, not the one-tick-stale
  component.
- Reproduced before the fix: the held-warship GOTO published no
  `FlightPrediction` in 4473 ticks.
- Change: `prediction.rs:771-786` replaces the `!docked` gate. A docked
  root predicts only while it drives; `docked_assembly` (`prediction.rs:523`)
  measures the pair through `assembly_roots`/`assembly_root`
  (`docking_section/assembly.rs:64,88`) and seeds the flight body with the
  assembly mass, centre and inertia. `well_pull` (`prediction.rs:482`) sums
  each root's pull at its own origin into a force and a torque about the
  assembly centre. `FlightPrediction::flown_point` (`prediction.rs:119`)
  carries the followed point in the root frame; the HUD ribbon anchors on it
  (`holo_instruments.rs:214`). Deleted: the `!docked` gate, the run's
  `ship_feels_wells`/`dominant_well`, the ribbon's own
  `ComputedCenterOfMass` anchor.
- Helm cases: held from a living ship or a derelict, and a dead station hub,
  draw the pair's path. A neutral helm (the partner drives) or a living
  station's helm draws nothing: the player's GOTO waits and flies nothing.
- Proof (`nova_scenario` `a_docked_pair_goto_is_predicted_to_its_arrival`):
  held warship 69.9 s, 503 predictions, max time-aligned deviation 1.69 u,
  max off the drawn line 0.64 u, max end gap 0.64 u; derelict station
  40.7 s, 440 predictions, 0.036 / 0.024 / 0.020 u. Undocked reference on
  the same 2000 u leg: 2.14 u deviation, 0.79 u end gap (a one-tick flip and
  brake shift). Joint drift during the leg stayed under 1.1e-4 u, so joint
  compliance is not a measurable error source.
- Frames (`loop_docked_helm` stills tail: the walk takes the HELM back,
  the waiting GOTO flies the pair, then HUD and Map stills), in
  `~/Pictures/nova-goto-path/docked/`: the HUD line runs from the pair to the
  arrival short of the HELM beacon; the ribbon ends on the predicted arrival
  (9.7e-8 u) and flies 36.7 m off the tug's own centre of mass.
- Audit fix: the Map dropped the route of a GOTO whose target is not a map
  contact (a scenario beacon locked in flight); the first frame
  (`map-before-noncontact-fix.png`) shows no line and `GOTO LOST`.
  `project_map_route` (`map/scene.rs`) now draws the predicted route for
  any GOTO and places the tag only on a plotted target.
- Open, not this defect class: the panel reads `GOTO LOST` for a beacon that
  was never on the Map; the Map zoom reach follows the contact spread, so a
  route to an unplotted target runs off the viewport edge.

## Owner check (2026-11-22)
- Docked GOTO line played and approved: "works".
- Owner: fix both open points from the docked build audit: the Map panel
  reads `GOTO LOST` for a GOTO target the Map does not plot (a scenario
  beacon), and the Map zoom limit comes from its contacts, so a route to a
  target it does not plot runs off the edge. Close after these land.

## Build (2026-11-22, Map follow-up)
- `GOTO LOST` cause: `update_map_panel` looked the GOTO target up in the
  Map contact list and read LOST on a miss, so a live target the Map does
  not plot (a scenario beacon locked in flight) read LOST. v0.18.0 had the
  same rule (`GOTO target lost`).
- Target gone: the GOTO target entity has no `GlobalTransform` any more
  (despawned: destroyed, streamed out). This is the rule the autopilot
  disengages on: `arrival_target` (`nova_ship/src/flight/autopilot.rs:762`)
  needs the target's `GlobalTransform`, and a miss ends the leg with
  `DisengageType::TargetGone` (`autopilot.rs:2081`). Not plotted is not gone.
- Change: `update_map_panel` (`map/scene.rs:1219`) reads the target through
  its own `(GlobalTransform, Option<Name>)` query. A plotted target reads its
  code, an unplotted one its `Name` (as the target inset), an unnamed one
  its range alone; LOST only when the query misses (`scene.rs:1294-1331`).
  `GotoLegType::Contact` is now `Target`.
- Zoom cause: `map_input` clamped the wheel at `map_radius_max(map_spread)`,
  the furthest contact only, so a route to an unplotted target ran past it.
- Change: `map_route_vertices` (`scene.rs:959`) is the one route the Map
  draws (ship, points ahead, decimated); `project_map_route` and `map_input`
  (`scene.rs:354`) both read it. The reach is
  `map_radius_max(max(map_spread, route_spread))` (`map/mod.rs:143`). The
  opening framing and Reframe still follow the contacts.
- Proof (`map/tests.rs`): `a_goto_on_a_body_off_the_map_reads_its_name_and_range_until_it_is_gone`
  (before: `GOTO LOST` for `GOTO Helm Beacon  2.10 km`) and
  `the_map_zoom_reaches_the_end_of_the_goto_route` (before: wheel stopped at
  520.2 u, route end needs 21678.4 u). After: 46/46 `map::tests` pass.
- Frames (`loop_docked_helm` tail, `~/Pictures/nova-map-goto-beacon/`):
  `docked-goto-map.png` reads `GOTO Helm Beacon  3.05 km` with the route
  running off the right edge at the opening framing;
  `docked-goto-map-reach.png` after the wheel shows the whole route ending
  inside the viewport. Log: camera 17681 m out; the contacts alone allowed
  5202 m; the route end is 2388 m out.
