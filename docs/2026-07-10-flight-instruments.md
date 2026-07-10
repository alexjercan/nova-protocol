# Maneuver instruments v1: telemetry seam, chips, and the ORBIT holo ring

- TASK: 20260709-103454
- SPIKE: docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md
- MODULE: crates/nova_gameplay/src/hud/maneuver_instruments.rs (+ the
  ManeuverTelemetry seam in flight.rs)

## What was built

The first instruments pass in the spike's hybrid language:

- **ManeuverTelemetry** (flight.rs): a reflected component the autopilot
  publishes on the ship every engaged GOTO/GotoPos tick - goal, distance,
  closing speed, planned brake accel, flip point + seconds-to-flip, rough
  ETA - and clears on verb switch or disengage (`On<Remove, Autopilot>`
  observer). Two pure helpers (`goto_flip_point`, `arrival_eta`) own the
  math; the HUD computes nothing. This is the load-bearing design choice:
  the arrival rule's internals (brake authority, rotation lead) stay in
  autopilot_system, and the instruments cannot drift from what the
  computer actually flies.
- **Destination readout**: "ETA 18s | 12.0 u/s | 300m" chip below the GOTO
  destination, Point-anchored to the telemetry goal.
- **Flip marker**: "FLIP 15s" chip projected on the flight path where the
  flip-and-burn starts; disappears once braking (the estimate is None).
- **ORBIT holo ring**: the first world-space holo instrument - a thin
  unlit emissive torus at the engaged plan's ring (well position, plan
  normal, plan radius), spawned when the player's plan appears, rebuilt on
  replan, despawned with the maneuver or the well. Asset access is plain
  `Assets<_>` so the whole lifecycle runs headless in tests.
- **Ring chip**: "r 50 | 4.9 u/s" anchored to the ring point nearest the
  ship (shared `orbit_ring_point` helper with the autopilot math).

## Decisions

- Telemetry only for GOTO legs in v1: STOP has no spatial goal (the
  status line already covers it) and ORBIT has the ring + chip; padding
  the component with variant-shaped data for all three verbs was rejected.
- The readout chips are their own indicator layer rather than children of
  the existing destination marker: no cross-module reach into
  flight_status internals, independent testability, same visual result.
- ETA is explicitly a rough estimate (coast + brake ramp; 2d/v while
  braking) and documented as such on the field - the instrument reads
  "about", not "promise".

## Verification

- 4 new flight tests: two pure (flip point incl. braking/degenerate arms,
  ETA in both regimes) and one physics-level (engaged GotoPos publishes
  telemetry with the flip point on the ship-goal segment; breakout clears
  it). 3 instruments tests: chips follow/clear with telemetry, flip hides
  while braking, ring + chip live and die with the plan (radius,
  orientation, despawn pinned).
- Affected modules green: flight (48), hud (47), AI patrol (the
  autopilot_system-signature rule from the orbit retro). fmt + check
  --workspace --examples clean. Full suite and clippy on CI.

## Difficulties

None material; the substrate absorbed all three chips without changes,
which was the point of building it.

## Self-reflection

- Publishing telemetry from the physics side was decided at plan time
  (the alternative - HUD recomputing from engine queries - was named and
  rejected); no mid-implementation redesign. The pattern (compute where
  the truth lives, render dumb) should be the default for the keybind
  hints task too: resolve availability where the verbs live, render chips.
