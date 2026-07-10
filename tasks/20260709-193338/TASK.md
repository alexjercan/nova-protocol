# Gravity wells: bounded one-way gravity with sphere of influence (physics substrate)

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.5.0, physics, gravity, spike

Spike: docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md

## Goal

Designated bodies (asteroids above a radius threshold, per-scenario override)
carry a `GravityWell`; ships and torpedoes opt in via `GravityAffected` and
feel `a = mu / r^2` toward the center - clamped at the surface, smoothstep-
faded to zero at the SOI edge, zero outside, one dominant well with hysteresis
when SOIs overlap. Wells never pull wells (a well source never carries
`GravityAffected`), so the world cannot clump. Strength is authored
(surface_gravity, radius-derived defaults, capped well below main-drive
acceleration), not mass-derived. Prerequisite for the ORBIT verb
(20260709-193339).

## Steps

- [x] Create `crates/nova_gameplay/src/gravity.rs` with the pure math core
      (no ECS): `well_accel(mu, r, body_radius, soi_radius, fade_fraction,
      surface_margin)` (inverse square toward center, clamped to the surface
      value below `body_radius + margin`, smoothstep-faded to zero over the
      outer `fade_fraction` of the SOI, exactly zero at and beyond the
      boundary), `circular_orbit_speed(mu, r) = sqrt(mu / r)`, and a
      `dominant_well` selection helper that keeps the incumbent unless a
      challenger beats its pull by the hysteresis factor. Register the module
      in `lib.rs` and the prelude, following `flight.rs`.
- [x] Add components: `GravityWell { mu, body_radius, soi_radius }` with a
      constructor from `surface_gravity` (`mu = surface_gravity * radius^2`,
      surface gravity clamped to `GravitySettings::max_surface_gravity`),
      `GravityAffected` marker, and `DominantWell(Entity)` state component for
      hysteresis tracking (removed when the entity leaves all SOIs). All
      Reflect-derived.
- [x] Add a reflected `GravitySettings` resource (pattern: `FlightSettings`,
      flight.rs:145): `default_surface_gravity`, `min_well_radius` (rocks
      below it get no well by default), `soi_factor` (SOI radius = factor *
      body radius), `fade_fraction` (~0.15), `surface_margin`,
      `switch_hysteresis` (~1.1), `max_surface_gravity` (the "never
      out-muscles a live ship" cap, tuned well under main-drive accel at the
      surface). Register the whole tree (settings + all new components) per
      juice retro R1.1.
- [x] Add `NovaGravityPlugin` + `NovaGravitySystems` set with one FixedUpdate
      force system: for each `GravityAffected` entity (filtered
      `Without<GravityWell>` - wells never pull wells), find containing
      wells, pick the dominant one via the hysteresis helper (update/remove
      `DominantWell`), and apply the pull with avian's
      `Forces::apply_linear_acceleration` (mass-independent, applied over the
      physics step - simpler than the originally planned mass * a * dt
      impulse via ComputedMass, and exactly gravity's semantics). Schedule
      the set before `SpaceshipSectionSystems` alongside `NovaFlightSystems`;
      wire the plugin into `NovaGameplayPlugin` (plugin.rs).
- [x] Auto-attach wells: observer on `Add<AsteroidMarker>` inserts a
      `GravityWell` with radius-derived defaults when `AsteroidRadius >=
      min_well_radius`. Extend `AsteroidConfig` (asteroid.rs) with
      `surface_gravity: Option<f32>` as the per-scenario override (Some =
      always a well at that strength even below the threshold, subject to the
      cap; None = threshold + defaults), carried onto the entity as
      `AsteroidSurfaceGravity`.
- [x] Opt in the affected set: observers insert `GravityAffected` on
      `Add<SpaceshipRootMarker>` (covers player and AI roots) and
      `Add<TorpedoProjectileMarker>`. Turret rounds and debris deliberately
      skip v1 (spike decision 5). Never insert it on well sources.
- [x] Give the asteroid_field scenario one large designated asteroid
      (nova_assets/src/scenario.rs, `asteroid_field`): 20u "Gravity Rock" at
      (250, 0, 0), authored surface_gravity 3.0, clear of the spawn/combat
      scatter cube. The 1-3u field rocks stay well-free via the threshold.
- [x] Unit tests for the pure helpers (inline `#[cfg(test)]`, pattern
      flight.rs:1144): force profile (surface clamp, unfaded core matches
      mu/r^2, fade reaches exactly zero at the SOI edge, zero outside),
      v_circ, hysteresis (incumbent kept inside the margin, challenger wins
      beyond it), cap applied at construction.
- [x] Physics-level integration tests (harness:
      integrity/test_support.rs:25 + flight.rs:1855 pattern): (1) a
      `GravityAffected` body seeded at radius r with tangential v_circ keeps
      a bounded radius over a full ~64s lap; (2) a body outside the SOI feels
      zero force; (3) a body without `GravityAffected` inside the SOI feels
      zero force; (4) with two overlapping wells the dominant one owns the
      body and ownership does not flicker at the boundary; (5) a well source
      itself never accelerates, even when misconfigured as affected.
- [x] Run fmt + check + the new tests; document the implementation in
      `docs/2026-07-10-gravity-wells.md`.
- [x] Review round 1 fixes: well sources on rails (`RigidBody::Static`
      inserted with the well), `On<Remove, GravityWell>` cleanup of
      `DominantWell`, placeholder-Position candidate gate, tests through the
      real `NovaGravityPlugin`, cap documented as a tuning contract.
- [x] Debug visualization (user request mid-flow): `GravityDebugPlugin` in
      nova_debug draws each well's SOI sphere + unfaded-core sphere and a
      line from every affected body to its dominant well, gated behind the
      F11 debug toggle.

## Notes

- Relevant files: crates/nova_gameplay/src/gravity.rs (new),
  crates/nova_gameplay/src/{lib,plugin}.rs (wiring),
  crates/nova_scenario/src/objects/asteroid.rs (config + designation
  observer), crates/nova_assets/src/scenario.rs (Gravity Rock),
  crates/nova_scenario/src/loader.rs + crates/nova_editor/src/lib.rs
  (AsteroidConfig field), docs/2026-07-10-gravity-wells.md.
- Well sources are put on rails at designation time (review R1.1): the
  observer inserts `RigidBody::Static` with the well, so nothing can shove
  an SOI around. Well-less field rocks keep the base bundle's Dynamic body.
  The no-clump guarantee is doubly held: one-way wells (sources never carry
  `GravityAffected`, plus a `Without<GravityWell>` filter on the affected
  query) and static sources.
- Scale sanity (spike decision 1): 20u rock at surface gravity 3 u/s^2 gives
  mu = 1200, v_circ ~ 4.9 u/s at r = 50u; SOI factor 4 puts the SOI at 80u.
  Field rocks (1-3u) fall below min_well_radius 5u.
- The gravity pull is a central linear acceleration (no torque); ships are
  point masses to the well.
- HUD (GRAV line, orbit-available cue) is NOT this task - it ships with the
  ORBIT verb (20260709-193339).
- Math helpers are pure and game-agnostic for a future bevy_common_systems
  promotion (spike decision 8).

## Resolution

Shipped as planned with one substitution: the force application uses avian
0.7's `Forces::apply_linear_acceleration` (mass-independent, continuous over
the physics step) instead of the planned mass * a * dt impulse - fewer query
dependencies and semantically exact. Verification: 12 new tests in
gravity.rs + 2 in asteroid.rs, all passing; cargo fmt + cargo check
--workspace clean; full suite and clippy deliberately left to CI per project
policy. Difficulties: none material. Reflection: checking the physics API
and the spike's Static-bodies assumption against the real code before
implementing avoided both a needless ComputedMass dependency and a false
no-clump premise; details in docs/2026-07-10-gravity-wells.md.
