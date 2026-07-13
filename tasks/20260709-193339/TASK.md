# ORBIT autopilot verb: circularize and station-keep inside a gravity well

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.5.0, handling, autopilot, gravity, spike

Spike: docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md
Depends on: 20260709-193338 (gravity-well substrate, CLOSED)

## Goal

Third diegetic autopilot verb next to STOP/GOTO. Inside a well, one input
engages ORBIT; the maneuver machine flies a real insertion through the
existing actuator seams: Plan (target radius clamped into the stable SOI
band; plane from r x v with a fallback when velocity is near-zero/radial) ->
Align -> Burn to tangential v_circ -> Hold (micro-burn station-keeping
against drift). Breakout on any flight input; capability/destruction
coupling inherited. HUD v1: flight-status states (GRAV well / AP ORBIT
phases) + an orbit-available cue on the screen-indicator substrate.

## Steps

- [x] Prerequisite fix discovered in playtest: static gravity-well sources
      fell out of the lock candidate set (targeting.rs filtered to
      RigidBody::Dynamic), so the Gravity Rock could not be locked for GOTO.
      Lockable = Dynamic OR carries GravityWell; regression test pins both
      sides. (b01a76b)
- [x] Pure orbit math in flight.rs (or gravity.rs where it fits the promotion
      posture): `orbit_plane_normal(r_vec, velocity, ship_up)` (normal from
      r x v, falling back to a plane through the ship's up axis when the
      velocity is near-zero or near-radial, always unit and perpendicular to
      r_vec), `orbit_target_radius(r, well, settings)` (clamp current radius
      into the stable band: above the surface clearance, below the unfaded
      fade-start with a safety factor), and `orbit_desired_velocity(...)` -
      tangential `circular_orbit_speed(mu, R)` on the plane plus a bounded
      correction toward the target ring (reuses arrival_speed_limit for the
      radial/plane error), so on-orbit it degenerates to pure tangential
      v_circ. Unit tests for all three (fallbacks, clamp bounds, on-orbit
      and off-orbit properties).
- [x] Extend the maneuver machine (flight.rs): `AutopilotAction::Orbit
      { well: Entity, plan: Option<OrbitPlan> }` where `OrbitPlan { radius,
      normal }` is computed by autopilot_system on the first engaged tick
      (the Plan phase) and stays sticky; add `AutopilotPhase::Hold`. Desired
      velocity from `orbit_desired_velocity`; phase = Hold when the velocity
      error is inside a hold tolerance (with hysteresis so Hold/Burn does
      not flicker), else Align/Burn as today. ORBIT never self-completes
      (station-keeps until breakout/off); disengage when the well entity is
      gone (mirrors GOTO's vanished target) and inherit the existing
      capability coupling (no live controller / no live engines =
      disengage) - NOTE: the original task text said "dead engines = aligns
      but cannot burn", but STOP/GOTO uniformly disengage on zero engines
      and ORBIT inherits that; recorded as a deviation.
- [x] New FlightSettings knobs (reflected, registered): orbit hold
      enter/exit tolerances (u/s), orbit band safety factor, orbit surface
      clearance factor. Sensible defaults documented in the same style as
      the existing fields.
- [x] Input (input/player.rs): `AutopilotOrbitInput` bound to KeyCode::KeyO
      + GamepadButton::South, toggle semantics like STOP: engaged ORBIT ->
      off; otherwise engage `Orbit { well }` from the ship's current
      `DominantWell` (no well = no-op). Breakout on flight input and the
      Z off-switch come free from the existing observers. NOTE: the
      STOP/GOTO input observers have no unit tests in the repo (no
      observer-trigger harness exists) and ORBIT matches that convention;
      the engage precondition (well must exist) is re-validated every tick
      in autopilot_system and covered by the physics tests.
- [x] HUD flight-status line (flight.rs flight_status_line +
      hud/flight_status.rs): grow the signature with well/orbit context -
      manual inside a well renders `MAN <speed> u/s | GRAV <name>`, engaged
      ORBIT renders `AP ORBIT - <ALIGN|BURN|HOLD> | r <radius> | <speed>
      u/s`. The HUD system reads DominantWell + the well's Name + Position
      for radius. Extend the flight_status_line unit tests.
- [x] Orbit-available cue on the screen-indicator substrate
      (hud/flight_status.rs pattern, AutopilotDestination indicator as
      template): while the player ship has a DominantWell and no ORBIT
      engaged, anchor a small `[O] ORBIT` indicator to the well entity;
      hidden otherwise. This doubles as the first keybind hint (the full
      diegetic keybind-hint system is task 20260709-103454's scope).
- [x] Physics-level integration tests (flight_app harness + NovaGravityPlugin
      so gravity actually pulls): (1) engage from near-rest at r ~ 50 inside
      a well -> the ship reaches the planned ring and holds it for a full
      lap (radius band + speed near v_circ + phase reaches Hold); (2) the
      well despawning mid-orbit disengages the autopilot; (3) dead engines
      (SectionInactiveMarker on all thrusters) disengage, dead controller
      disengages - same as STOP/GOTO.
- [x] fmt + check (workspace AND --examples per the gravity retro) + the
      new tests; update docs/retros/20260710-gravity-wells.md's companion doc or
      a new docs/retros/20260710-orbit-verb.md with decisions, deviations,
      difficulties, self-reflection.

## Notes

- Seam map (from code reading): AutopilotAction/Phase + Autopilot::engage
  flight.rs:90-140; autopilot_system flow and disengage points
  flight.rs:677-1001 (dead computer 737-748, dead engines 770-774, done
  911-917, phase transition 975-979); input bindings player.rs:268-387,
  breakout pattern 403-423; flight_status_line flight.rs:638-661 rendered
  by hud/flight_status.rs:131-156; screen-indicator substrate
  hud/screen_indicator.rs:130-160 with the AutopilotDestination driver
  (hud/flight_status.rs:161-179) as the template; well name via Name on the
  well entity (scenario BaseScenarioObjectConfig names it "Gravity Rock").
- The orbit plan is computed once and stored in the action (sticky radius +
  plane); STOP/GOTO replan every tick, but a re-planned orbit would chase
  its own drift. Replanning happens only by re-engaging.
- Out-of-plane and radial error both fold into the desired-velocity
  correction; the existing allocator, alignment gates, spool, and
  attitude_deadband handle everything downstream - no new actuator code.
- AI usage of ORBIT is out of scope (spike open question, smarter-AI task).
- Turret rounds/debris still skip gravity; nothing here changes the
  affected set.

## Resolution

Shipped as planned. The verb is pure goal computation on top of the
existing maneuver machine (no new actuator code): sticky OrbitPlan in the
action, orbit_desired_velocity feeding the shared error/allocation path,
Hold as a hysteresis label over the existing micro-burn behavior. HUD grew
the GRAV/ORBIT states, the destination marker doubles for the well, and an
[O] ORBIT cue projects onto the dominant well (first hand-placed keybind
hint; the system belongs to 20260709-103454). Deviation recorded: dead
engines disengage (uniform capability rule) instead of "aligns but cannot
burn". Prerequisite playtest fix folded in: static well sources are
lockable again (targeting.rs). Verification: 12 new tests + all affected
modules green (51 flight / 14 gravity / 25 targeting / 7 HUD), fmt +
check --workspace --examples clean; full suite and clippy on CI per
project policy. Details: docs/retros/20260710-orbit-verb.md.
