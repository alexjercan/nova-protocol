# Collider-derived BodyRadius + geometric orbit parking

Playtest (user, 2026-07-10, after the surface-relative standoff landed):
GOTO still stops too close, and the parked orbit can be bad enough to
fall into the well object. Both had the same root cause: everything
measured from the NOMINAL radius, but the asteroid's actual surface is a
noise-displaced mesh whose vertices are pushed OUTWARD from the unit
sphere (bevy_common_systems `apply_noise`: `pos + normalize(pos) *
height`, height >= 0 from `PlanetHeight`) - the real rock edge sits past
the designation radius, sometimes far past.

## What changed

- `BodyRadius` is no longer authored from `config.radius`. The asteroid
  collider observer (crates/nova_scenario/src/objects/asteroid.rs,
  `insert_asteroid_collider`) derives it from the collider volume itself:
  `nominal radius * max vertex distance` of the generated unit mesh
  (`mesh_max_vertex_radius`, floored at 1.0 so a degenerate mesh never
  shrinks the radius). Nothing about the edge is hardcoded anymore; a
  lumpier noise profile automatically widens the standoff.
- The GravityWell keeps deriving mu/SOI from the NOMINAL radius: physics
  stays deterministic across seeds, tests stay exact, and the designation
  radius remains the tuning knob. Geometry (where surfaces are) and
  physics (how hard the well pulls) are deliberately separate readings;
  the arrival and the orbit band take `max(BodyRadius,
  GravityWell::body_radius)`.
- The ORBIT band's clearance floor now clears the GEOMETRIC surface: the
  plan block and the parking handoff clamp the ring against a well whose
  `body_radius` is raised to the derived BodyRadius (the `band_well`
  closure in `autopilot_system`), so a planned ring can no longer sit
  inside the actual rock.
- The parking handoff plans its ring at the handoff itself, from the
  LEG'S INTENT - the park point, standoff above the geometric surface
  (`max(park radius, current radius)` clamped into the band) - instead of
  handing `plan: None` to the plan block, which would clamp whatever
  radius terminal creep dragged the ship to (the observed
  falls-into-the-rock orbit). A bandless well releases instead of
  parking.

## Verification

- nova_scenario: `body_radius_derives_from_the_generated_collider`
  (derived >= nominal, sanity-bounded) and the pure
  `mesh_max_vertex_radius` unit test; 11 tests green.
- flight: `handoff_ring_clears_the_geometric_radius` (well nominal 40 +
  BodyRadius 70 -> ring at 120, not a nominal-band clamp of the crept
  position); the full well arc test unchanged and green; flight 57,
  gameplay lib 339, `cargo check --workspace --examples` clean.

## Known limits

- Fragments and small field rocks get no BodyRadius (unsized,
  center-relative at 50u - their extent is tiny).
- The derived radius is the outermost vertex, i.e. a bounding sphere: a
  single tall spike widens the standoff everywhere. Conservative by
  design; revisit only if rocks get authored spikes.
- Ships still contribute no radius (q_wells' design statement; sensor
  task 20260710-195953 owns ship-scale geometry if capital ships appear).
