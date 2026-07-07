# Integrity: physics-level tests for collision damage + graph construction

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.4.0,test,integrity

Follow-up from the destruction-pipeline tests (task 20260525-133008). Those cover the
avian-free core of the integrity pipeline (disable -> destroy -> chain, aggregation,
meshless despawn) by driving `ConnectedTo` / `HealthApplyDamage` directly. The
physics-driven *inputs* to the pipeline are still untested because they need an avian
world:

- `on_impact_collision_deal_damage` - impulse/energy damage on `CollisionStart`.
- `on_blast_collision_deal_damage` - radial blast damage on collision with a blast
  sensor (also carries the FIXME in `20260706-162912` about inconsistent firing).
- `build_integrity_relations` - derives each node's `ConnectedTo` from `ColliderOf`
  and section grid positions, and marks the body `IntegrityRoot`.

## Steps

- [x] Add a headless avian test harness (`integrity/test_support.rs`: `integrity_physics_app`
      + `settle`) modelled on avian's own test setup: `MinimalPlugins` + `TransformPlugin` +
      `AssetPlugin` + `MeshPlugin` + `PhysicsPlugins` + `HealthPlugin` + `IntegrityPlugin`,
      zero gravity, a fixed manual timestep.
- [x] `build_integrity_relations` (glue.rs): a ship of 3 sections in a line yields the right
      adjacency (middle has 2 neighbors, ends have 1) and its body is `IntegrityRoot`; a lone
      asteroid body gets an empty `ConnectedTo` and is `IntegrityRoot`. Sim-driven end to end.
- [x] Impact damage (plugin.rs): real avian mass, injected `CollisionStart`, assert the
      impulse/energy damage matches the observer's own formula/constants; a sub-threshold graze
      deals nothing.
- [x] Blast damage (plugin.rs): a real sensor overlap fires a deterministic `CollisionStart`,
      and the linear falloff deals the expected damage at distance; a body outside the sensor
      takes none.
- [x] Full check suite green: `cargo test --workspace` (48 nova_gameplay, incl. 6 new;
      examples_smoke under Xvfb), `cargo clippy --workspace --all-targets`.

## Resolution

Added a shared headless-avian harness (`integrity/test_support.rs`) and six physics-level
tests co-located with the code under test (4 in `plugin.rs`, 2 in `glue.rs`), covering the
avian-dependent inputs the destruction-pipeline tests (133008) deliberately left out.

Two design decisions surfaced during the work:

- `MeshPlugin` is required in the harness. nova enables avian's `collider-from-mesh` feature
  (asteroids use `Collider::trimesh_from_mesh`), whose backend reads `AssetEvent<Mesh>`; without
  registering the `Mesh` asset the app panics with "Message not initialized". This mirrors
  avian's own `create_app`.
- Impact damage is tested by *injecting* `CollisionStart` against a real avian world (real
  `ComputedMass`), not by simulating the collision: the solver zeroes the contact velocity
  before the observer can read it, so a sim-driven impact reads ~0 relative velocity and deals
  no damage - the value the observer needs is gone by the time it runs. Blast damage, by
  contrast, comes from a *sensor* overlap with no solver response, so that one is driven by the
  real simulation (place body inside the sensor, step, assert falloff). Masses must be read
  after a few `update()`s: `ComputedMass` is `NaN` on the first step.

## Notes

Source: `crates/nova_gameplay/src/integrity/plugin.rs` (collision observers),
`crates/nova_gameplay/src/integrity/glue.rs` (`build_integrity_relations`). Pairs with
the FIXME in `20260706-162912` (OnDestroyed / blast-collision inconsistency); the blast test
exercises and documents the working `(object, blast)` ordering that the FIXME contrasts with
the broken reverse order.
