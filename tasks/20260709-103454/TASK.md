# Diegetic flight instruments: in-world autopilot/maneuver UI

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.5.0, hud, autopilot, spike

Spike: tasks/20260710-174523/SPIKE.md
(design language decided there; original ask from
tasks/20260709-103324/SPIKE.md)

## Goal

Maneuver instruments v1, in the hybrid language the 2026-07-10 spike
decided (3D world-space for spatial geometry, projected chips for
numbers - the split the velocity sphere + indicator substrate already
use): (i) enrich the destination indicator with ETA, closing speed and
standoff distance from the arrival rule; (ii) a flip-point marker - a
Point-anchored indicator where `v_allowed(d)` says the flip happens,
labeled with seconds-to-flip; (iii) the ORBIT ring as the first
world-space holo element (3D line loop at `OrbitPlan { radius, normal }`,
velocity-sphere visual family) with the r/v_circ chip anchored to it.
The ring deliberately pilots the holo language on the simplest geometry
before the ribbon/shell expansion (task 20260710-174629).

## Steps

- [x] Publish maneuver telemetry from the physics side: a reflected
      `ManeuverTelemetry` component on the ship, written by
      autopilot_system each engaged GOTO/GotoPos tick - distance to goal,
      closing speed, planned brake acceleration, flip point (world Vec3 +
      seconds-to-flip, None once past it) and a rough ETA - and removed
      when the Autopilot is (On<Remove, Autopilot> observer). Pure helper
      `goto_flip_point(to_target, speed, accel, margin, standoff) ->
      Option<(f32, f32)>` (distance-from-target where braking starts +
      seconds until the ship reaches it) with unit tests; the HUD stays a
      dumb reader.
- [x] New `hud/maneuver_instruments.rs` module + plugin (registered in
      NovaHudPlugin, systems in NovaHudSystems): owns everything below.
- [x] Destination readout: give the existing AutopilotDestinationUI
      indicator a small text child ("ETA 12s | 45m/s | 320m" style) driven
      from ManeuverTelemetry; empty/hidden without telemetry.
- [x] Flip-point marker: a Point-anchored screen indicator at the
      telemetry's flip point, labeled "FLIP <n>s"; hidden when None (no
      maneuver, already braking, STOP, ORBIT).
- [x] ORBIT holo ring: a world-space 3D entity (thin torus mesh at
      OrbitPlan radius, emissive alpha-blended StandardMaterial in the
      velocity-sphere family) spawned while an Orbit autopilot with a plan
      is engaged, positioned at the well, oriented Y->plan.normal;
      despawned on disengage/well death. Systems take Assets<Mesh>/
      Assets<StandardMaterial> so the lifecycle is testable headless.
- [x] Ring chip: a small indicator anchored to the ring point nearest the
      ship showing "r <plan radius> | <v_circ> u/s" while ORBIT is engaged
      (Point anchor updated per frame).
- [x] Tests: unit tests for goto_flip_point (before/past the flip,
      degenerate accel); physics-level test that an engaged GOTO publishes
      telemetry with the flip point between ship and target and removes it
      on disengage; lifecycle tests for the ring (engage -> entity with
      the plan's radius and orientation; disengage -> gone) and for the
      destination/flip/ring-chip drivers (flight_status.rs test style).
- [x] fmt + check --workspace --examples + the new tests and the affected
      modules (flight, hud, input::ai - autopilot_system signature rule
      from the orbit retro); document in tasks/20260709-103454/NOTES.md.

## Notes (planning)

- Telemetry on the physics side keeps the arrival-rule internals (brake
  authority, lead) out of the HUD and makes the flip math unit-testable;
  the alternative (HUD recomputes from engine queries) duplicates
  autopilot_system internals and drifts.
- The ring is spawned per engage (mesh built at plan radius) rather than
  scaled, so the torus minor radius stays constant on screen.
- MaterialPlugin/render caveats: the ring systems must not require the
  render app; headless tests init the asset resources directly.

## Notes

- User priority (2026-07-10): this is the most important part of the HUD.
- Followed by: 20260710-174646 (keybind hints - the cluster docks with
  these instruments), 20260710-174629 (holo expansion - after the ring).
- All maneuver data is already computed per tick by autopilot_system
  (flight.rs): arrival rule, Autopilot phase, OrbitPlan, DominantWell.
  Nothing new to simulate, only to surface.
- Prerequisites all shipped: autopilot verbs (STOP/GOTO/ORBIT), the
  screen-indicator substrate, the flight-status line.

## Resolution

Shipped per the plan: ManeuverTelemetry seam on the physics side (pure
flip/ETA helpers, published per GOTO tick, cleared on verb switch and
disengage), hud/maneuver_instruments.rs with the destination readout, flip
marker, ORBIT holo ring (first world-space instrument; headless-testable
lifecycle) and ring chip. 7 new tests; flight/hud/AI-patrol modules green;
fmt + check --workspace --examples clean, full suite on CI. Details:
tasks/20260709-103454/NOTES.md. Telemetry deliberately covers GOTO
legs only in v1 (STOP has no spatial goal, ORBIT has the ring).
