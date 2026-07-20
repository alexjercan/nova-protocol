# Gravity wells: bounded one-way gravity with a sphere of influence

- TASK: 20260709-193338
- SPIKE: tasks/20260709-193147/SPIKE.md
- MODULE: crates/nova_gameplay/src/gravity.rs

## What was built

The physics substrate for orbital play (spike option B, the first half of
recommendation D): designated bodies carry a `GravityWell` component and pull
opted-in entities (`GravityAffected`: ship roots and torpedo projectiles)
with the real inverse square `a = mu / r^2` toward their center. The pull is
clamped to its surface value below `body_radius + surface_margin`, smoothstep-
faded to exactly zero over the outer 15% of the sphere of influence, and zero
beyond it. When SOIs overlap, exactly one well owns an entity: the strongest
pull at its position, with a 10% switch hysteresis tracked in a
`DominantWell` component so boundary flicker cannot flip ownership tick to
tick.

Strength is authored, never mass-derived: `GravityWell::from_surface_gravity`
derives `mu = surface_gravity * radius^2` and clamps surface gravity to
`GravitySettings::max_surface_gravity`, the guardrail that keeps every well
escapable under main drive. All tunables live in the reflected
`GravitySettings` resource (whole tree registered, per the juice retro
lesson).

Scenario side, asteroids designate themselves: an observer on
`Add<AsteroidMarker>` gives big rocks (radius >= `min_well_radius`, default
5u) a default-strength well and leaves the 1-3u field rocks in flat space;
`AsteroidConfig::surface_gravity: Option<f32>` is the per-body override
(always a well when Some, capped). The asteroid_field scenario gained a 20u
"Gravity Rock" at (250, 0, 0) - outside the combat scatter cube - at the
spike's sanity strength (3 u/s^2, SOI 160u after the reach retune, v_circ ~ 4.9
u/s at r = 50u) so
the well is playtestable today.

## Decisions and deviations

- **`Forces::apply_linear_acceleration` instead of a mass-scaled impulse.**
  The plan originally said "apply mass * a * dt via ComputedMass". avian 0.7's
  `Forces` helper turned out to expose `apply_linear_acceleration`, which is
  mass-independent and applied continuously over the physics step - exactly
  gravity's semantics, one less query dependency, and no manual dt handling.
  The step was updated to match reality.
- **Well sources go on rails.** Asteroids are `RigidBody::Dynamic` by
  default (`base_scenario_object`), and round 1 of the review caught that a
  dynamic well source can be shoved by rams/blasts, dragging its SOI and
  every orbit in it. The designation observer therefore inserts
  `RigidBody::Static` alongside the well (spike option B, "bodies on
  rails"); small well-less rocks stay dynamic. The one-way invariant (well
  sources never receive `GravityAffected`, plus `Without<GravityWell>` on
  the affected query as belt-and-braces) still holds independently and is
  pinned by a test.
- **Well death is handled eagerly.** The playtest rock is destructible, so
  an `On<Remove, GravityWell>` observer strips `DominantWell` handles the
  moment a well dies - consumers (HUD, ORBIT verb) never see a dangling
  entity beyond the current flush. Pinned by a despawn-mid-orbit test.
- **Placeholder positions are gated out.** avian 0.7 spawns rigid bodies at
  `Position::PLACEHOLDER` (Vector::MAX) until the first physics sync, which
  makes same-flush spawns momentarily coincident; the candidate loop
  requires a normalizable, finite offset before a well can own anything, so
  no spurious `DominantWell` flashes at scenario start.
- **Debug visualization (user request).** `GravityDebugPlugin` in nova_debug
  (F11-gated like the section gizmos) draws each well's SOI boundary and
  unfaded-core wire spheres plus an amber line from every affected body to
  its dominant well - enough to see wells, the trustworthy orbit band, and
  hysteresis handoffs live. Diegetic (non-debug) readability stays with the
  ORBIT task's HUD line and the parked diegetic-instruments task.
- **Dominance compares the faded pull, not raw mu/r^2.** Near the SOI edge
  the faded value is what the entity actually feels; comparing it makes the
  handoff physically honest and naturally releases ownership at the boundary.
- **The well-attach observer lives in nova_scenario** (asteroid.rs), not in
  the gravity module: nova_gameplay cannot see `AsteroidMarker` (dependency
  direction), and "which bodies are designated" is scenario policy anyway.
  `AsteroidPlugin` init_resources `GravitySettings` so scenario-only apps
  work; `init_resource` is idempotent, so the gameplay plugin owning the same
  resource is fine.
- **`DominantWell` is public API on purpose**: the ORBIT verb (20260709-193339)
  and the HUD GRAV line will read it to know whose orbit the ship is in.

## Verification

- 14 unit/integration tests in gravity.rs: pure force profile (core formula,
  surface clamp, exact zero at/beyond the SOI edge, monotonic fade), v_circ
  sanity, hysteresis helper, observer opt-in, and physics-level tests
  running the real NovaGravityPlugin on the avian harness - a full ~64s
  orbit lap staying within [0.8r, 1.25r], flat space staying exactly flat
  outside the SOI and for non-opted-in bodies, wells never pulling wells,
  dominant-well handoff with hysteresis under teleported probes, a ship
  root pulled purely through the plugin's wiring, and a well despawned
  mid-orbit releasing dominance without a panic.
- 2 scenario tests in asteroid.rs: threshold + default well derivation with
  static-source asserts, and authored override + cap.
- cargo fmt + cargo check --workspace clean. Full suite and clippy left to
  CI per project policy.

## Difficulties

None material. The orbit-stability test needed a full-lap horizon (4200
updates) to be meaningful; semi-implicit Euler at this strength holds the
radius band comfortably, so the Hold phase of the future ORBIT verb has an
easy job at v1 scale.

## Self-reflection

- Reading the avian Forces API before implementing (instead of trusting the
  plan's mass * a * dt sketch) removed a whole query dependency; checking the
  substrate API against the plan's assumption early paid off. Do the same
  for the maneuver-machine seams when building ORBIT.
- The spike's "sources stay static" claim did not survive contact with the
  code (dynamic scenario objects). Catching it during planning - not mid-
  implementation - kept the invariant honest ("one-way by construction")
  instead of accidentally relying on a false premise. Verify spike claims
  against the code at plan time, always.

## Collider-derived BodyRadius + geometric orbit parking (2026-07-10 hotfix)

Playtest (user, 2026-07-10, after the surface-relative standoff landed):
GOTO still stops too close, and the parked orbit can be bad enough to
fall into the well object. Both had the same root cause: everything
measured from the NOMINAL radius, but the asteroid's actual surface is a
noise-displaced mesh whose vertices are pushed OUTWARD from the unit
sphere (bevy_common_systems `apply_noise`: `pos + normalize(pos) *
height`, height >= 0 from `PlanetHeight`) - the real rock edge sits past
the designation radius, sometimes far past.

### What changed

- `BodyRadius` is no longer authored from `config.radius`. The asteroid
  collider observer (crates/nova_scenario/src/objects/asteroid.rs,
  `insert_asteroid_collider`) derives it from the collider volume itself:
  `nominal radius * max vertex distance` of the generated unit mesh
  (`mesh_max_vertex_radius`, floored at 1.0 so a degenerate mesh never
  shrinks the radius). Nothing about the edge is hardcoded anymore; a
  lumpier noise profile automatically widens the standoff.
- The GravityWell derives from the GEOMETRIC radius too (superseding the
  first cut of this change, which kept the well nominal for seed
  determinism): with the real surface several times the nominal sphere, a
  nominal-sized SOI cannot contain an orbit band above the geometric
  clearance floor - the O-key ORBIT immediately hit "no stable band" in
  playtest. The well observer now triggers on `On<Add, BodyRadius>` (which
  sequences it after the collider derivation) and sizes mu/SOI on the
  derived radius; only the WELL-QUALIFICATION rule (min_well_radius) stays
  keyed on the nominal designation radius. Physics now varies with the
  mesh seed - accepted: surface gravity is still the authored/capped
  knob, and it now means gravity at the REAL surface. The arrival and the
  band still take `max(BodyRadius, GravityWell::body_radius)` as a safety
  net for bare wells.
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

### Verification

- nova_scenario: `body_radius_derives_from_the_generated_collider`
  (derived >= nominal, sanity-bounded), the pure `mesh_max_vertex_radius`
  unit test, and `the_well_derives_from_the_geometric_radius` (full
  observer chain: well.body_radius == derived BodyRadius, SOI and mu
  scale with it); 12 tests green.
- flight: `handoff_ring_clears_the_geometric_radius` (well nominal 40 +
  BodyRadius 70 -> ring at 120, not a nominal-band clamp of the crept
  position); the full well arc test unchanged and green; flight 57,
  gameplay lib 339, `cargo check --workspace --examples` clean.

### Known limits

- Fragments and small field rocks get no BodyRadius (unsized,
  center-relative at 50u - their extent is tiny).
- The derived radius is the outermost vertex, i.e. a bounding sphere: a
  single tall spike widens the standoff everywhere. Conservative by
  design; revisit only if rocks get authored spikes.
- Ships still contribute no radius (q_wells' design statement; sensor
  task 20260710-195953 owns ship-scale geometry if capital ships appear).

### Postmortem: the "no stable band" regression

The first cut of this change raised the band's clearance floor to the
geometric radius while the ceiling stayed nominal-derived, and shipped
without recomputing both sides with realistic magnitudes (floor 1.5*101 =
151 > ceiling 122 for a 20u rock) - O-key ORBIT refused everywhere within
minutes of playtest. Diagnosable from one pasted log line only because every
ORBIT disengage path logs a distinct reason string. Lessons in
LESSONS.md: recompute-both-sides-of-a-band, distinct-refusal-reasons.
