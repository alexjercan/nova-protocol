# Surface-relative GOTO standoff

Task: tasks/20260710-202408 - Playtest finding: `arrival_standoff` (50u)
measured from the target's CENTER, so a GOTO at a big object stopped too
close to (or, before the gravity-aware arrival landed, inside) its
surface.

## What changed

- New `BodyRadius(f32)` component in crates/nova_gameplay/src/flight.rs
  (prelude-exported, reflected): the geometric radius of a scenario
  object - the surface the arrival standoff measures from. Originally
  authored from `config.radius`; superseded same day by the
  collider-derived version (the noise-displaced mesh reaches past the
  nominal radius - see docs/retros/20260710-collider-derived-body-radius.md).
- The GOTO arm of `autopilot_system` resolves the target's radius as
  `max(BodyRadius, GravityWell::body_radius)` from whichever components
  the target carries - max is conservative if they ever disagree; unsized
  targets and GotoPos stay at zero, which is exactly the old behavior.
- `arrival_desired` plans against `standoff = arrival_standoff +
  target_radius` everywhere: the inside-standoff gate, the gravity
  budget's rest-point evaluation, the flip point, the ETA and the desired
  velocity. The pure helpers did not change - the radius folds into the
  standoff they already take.
- `ManeuverTelemetry.distance` is now SURFACE-relative (center distance
  minus the resolved radius), so the readout chip never says "50" while
  hovering over a mountain. The only current consumer is the chip text;
  `goal`/`flip_point` remain world positions and are unaffected.

## Alternatives considered

- LockSignature as the radius source: it is authored from the radius on
  asteroids today, but it is a scanner magnitude, not geometry - ships
  will get signatures that are not sizes (task 20260710-195953). A
  dedicated geometric component keeps the two meanings from fusing.
- GravityWell::body_radius alone: covers the case that hurts today (the
  Gravity Rock) but leaves big well-less rocks parking inside their own
  silhouette; BodyRadius costs one line at the asteroid spawn.

## Known limits

- The park point inside a strong well still settles below the standoff by
  the terminal-creep margin (release fires at near-rest, the pull keeps
  dragging): the well integration test parks ~11u inside the 90u point.
  That is the pre-existing arrival-release behavior, not the geometry;
  task 20260710-195954 (GOTO parks into ORBIT) owns the real answer.
- Ships as GOTO targets stay center-relative (no BodyRadius authored);
  their extent is small at a 50u standoff. Revisit with the sensor-model
  task if capital-scale ships appear.
