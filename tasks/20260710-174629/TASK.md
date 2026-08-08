# World-space holo instruments: trajectory ribbon, SOI shell, flip gate

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.5.0, hud, autopilot, spike

Spike: tasks/20260710-174523/SPIKE.md
Depends on: 20260709-103454 (the ORBIT holo ring must prove the world-space
language first)

## Goal

Expand the world-space holo language the ORBIT ring pilots (velocity-sphere
visual family, UI stays chips): a trajectory ribbon for engaged GOTO/STOP
(arrival-rule curve made visible), an SOI shell on well approach in normal
play (deferred here from the gravity spike), and the flip point as world
geometry (a gate on the path).

## Steps

- [x] Extend ManeuverTelemetry to STOP legs (flight.rs): STOP does have a
      spatial goal - the predicted rest point,
      `position + v_hat * (v*lead + v^2/(2 a margin))`, the same terms as
      the flip math. Publish goal/distance/closing/eta for engaged STOP
      (flip_point stays None - the "flip" is the initial retrograde
      alignment, already inside the lead); pure helper + unit test;
      physics test that STOP publishes and completion clears. The
      destination readout chip then covers STOP for free.
- [x] New `hud/holo_instruments.rs` module + plugin (NovaHudPlugin,
      NovaHudSystems): owns the three world-space elements below, in the
      orbit-ring visual family (thin unlit NAV_CYAN meshes, plain
      Assets<_> access so lifecycles run headless).
- [x] Trajectory ribbon: the engaged leg's path as thin cylinder
      segments - ship -> flip -> goal when a flip is predicted, ship ->
      goal otherwise (GOTO and STOP both, via telemetry). One shared unit
      cylinder mesh, per-segment Transform (midpoint, Y-axis rotated onto
      the segment, length scaled); segments live and die with the
      telemetry.
- [x] Flip gate: a small torus at telemetry.flip_point, oriented
      perpendicular to the path, sized to fly through (a few units);
      despawns when the flip estimate is None (braking) or the leg ends.
- [x] SOI shell: three orthogonal great-circle rings (world-axis planes)
      at soi_radius around the relevant well, shown in normal play while
      the ship is inside the SOI or on approach (within an
      approach-factor of it); despawn otherwise. Well selection: the
      dominant well when inside, else the nearest well within the
      approach factor.
- [x] Despawn all holo elements with the player HUD (hud/mod.rs remove
      observer), same as the orbit ring.
- [x] Tests: pure rest-point math; STOP telemetry physics test; headless
      lifecycle tests for ribbon (segment count with/without flip,
      death with the leg), gate (appears with flip, dies braking), and
      shell (inside SOI, on approach, gone in flat space).
- [x] fmt + check --workspace --examples + affected modules (flight incl.
      input::ai per the signature rule, hud); document in
      tasks/20260710-174629/NOTES.md.

## Notes (planning)

- The ribbon renders the leg's straight-line plan, which is exactly what
  the computer flies today; when task 20260710-193500 makes the arrival
  solve gravity-aware, a curved prediction can replace the segments -
  deliberately not attempted here (the instrument must not out-promise
  the autopilot, AGENTS.md rule).
- Shell rings reuse the torus + rotation trick from the orbit ring; three
  axis-aligned great circles read as a wire globe without any wireframe
  render tech.
- Approach factor is a module const for now (display concern, not a
  physics tunable); promote to settings if playtest wants tuning.

## Resolution

Shipped per plan: STOP telemetry (rest-point goal via stop_rest_distance),
hud/holo_instruments.rs with the ribbon (shared unit cylinder, 1-2
segments), the flip gate, and the SOI shell (wire globe of three tori,
dominant-well-inside or nearest-on-approach), all despawned with the
player HUD. 5 new tests + 1 updated; flight/hud/input modules green; fmt +
check --workspace --examples clean. Details:
tasks/20260710-174629/NOTES.md.
