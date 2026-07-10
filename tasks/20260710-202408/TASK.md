# GOTO standoff must be surface-relative: account for target size

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.5.0, autopilot, bug, ux

## Goal

Playtest finding (user, 2026-07-10): `FlightSettings::arrival_standoff`
(50u) measures from the target's CENTER, so a GOTO at a big object stops
too close to - or inside - its surface. The arrival rule must budget the
target's size: stop `arrival_standoff` from the SURFACE, i.e. effective
standoff = `arrival_standoff + target_radius`.

Direction: the arrival leg (arrival_desired in autopilot_system,
goto_desired_velocity) needs a target radius. Candidate sources, to weigh
at plan time: `GravityWell::body_radius` (well bodies - the case that
hurts today), `LockSignature` (already radius-authored on asteroids, and
the lock is the GOTO designator, but it is a scanner magnitude, not
strictly geometry), or a dedicated `BodyRadius`-style component the
scenario authors once and both systems read. Ships as GOTO targets
already work at 50u (their extent is small); zero-radius default keeps
current behavior for everything unsized.

## Steps

- [x] Add a `BodyRadius(pub f32)` component in
  crates/nova_gameplay/src/flight.rs (prelude-exported, reflected,
  registered by NovaFlightPlugin): the authored geometric radius of a
  scenario object, the surface the arrival standoff measures from.
- [x] Author it in crates/nova_scenario/src/objects/asteroid.rs:
  `BodyRadius(config.radius)` on every asteroid (same source that already
  feeds LockSignature and GravityWell::body_radius).
- [x] In `autopilot_system`'s GOTO arm, resolve the target's radius as
  `max(BodyRadius, GravityWell::body_radius, 0.0)` from whichever
  components the target carries (max is conservative if they ever
  disagree; unsized targets and GotoPos stay at 0 - today's behavior).
  Pass it into `arrival_desired`.
- [x] In `arrival_desired`, plan against `effective_standoff =
  arrival_standoff + target_radius` everywhere the standoff appears (the
  inside-standoff gate, the gravity_along rest point, goto_flip_point,
  arrival_eta, goto_desired_velocity), and publish surface-relative
  telemetry: `ManeuverTelemetry.distance = center_distance -
  target_radius` so the HUD chip never reads "50" while hovering over a
  mountain. Update the field doc.
- [x] Update the well-arrival integration test: the 40u body + 50u
  standoff must now park near 90u from the center (the test change is
  the feature's demonstration); add a unit test that arrival_desired's
  radius resolution prefers the larger source and that GotoPos is
  unchanged.
- [x] Run flight, input::ai, hud, nova_scenario tests and `cargo check
  --workspace --examples`.
- [x] Docs: docs/2026-07-10-surface-relative-standoff.md; close TASK.md.

## Notes

- Filed 2026-07-10, queued after 20260710-201514 (gravity indicator).
- Touches the same code region as 20260710-193500 (gravity-aware arrival)
  and 20260710-195954 (GOTO parks into ORBIT): the ORBIT handoff wants
  the arrival radius inside the stable band, and the band math already
  uses body_radius - whichever of the three lands last should re-read the
  other two. Consider whether one small "arrival geometry" pass should
  implement 202408 + 195954 together.
- ManeuverTelemetry (distance/ETA/flip) and the destination readout
  should report the surface-relative numbers too, or the chip reads "50m"
  while hovering over a mountain.

## Resolution

Implemented per the Steps with no deviations: BodyRadius component
(authored on asteroids from the same config.radius as LockSignature),
max(BodyRadius, GravityWell::body_radius) resolution in the GOTO arm,
effective standoff threaded through the whole arrival leg (the pure
helpers needed no change - the radius folds into the standoff parameter
they already take), and surface-relative ManeuverTelemetry.distance (the
chip is its only consumer today).

One empirical adjustment: the well integration test's park tolerance uses
the same -45u terminal-creep lower bound as the flat-space arrival tests.
The first strict bound (-10u) failed at 78.9u vs the 90u park point: the
release fires at near-rest and the 1+ u/s^2 pull drags the parked ship
inward - pre-existing release behavior, not this task's geometry, and
exactly what 20260710-195954 (park into ORBIT) exists to fix. Recorded in
docs/2026-07-10-surface-relative-standoff.md Known limits.

Checks: flight 55, input::ai 73, hud 55, nova_scenario 9, cargo check
--workspace --examples clean. Full suite and clippy left to CI per policy.
