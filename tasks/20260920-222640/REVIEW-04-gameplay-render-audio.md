# Review 04: Gameplay render and audio support

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_gameplay/src/transform/sphere_orbit.rs` - Unused orbit rigs are installed in every app

Workspace-wide symbol searches found no spawner or consumer for `SphereOrbit`, `RandomSphereOrbit`, their input/output components, or their plugins. `NovaGameplayPlugin` still installs both plugins and their empty-query systems unconditionally. Prospective OS UI callers explicitly implement their own orbit behavior instead.

No measurable cost or wrong behavior was established. This is obsolete internal surface and plugin machinery, contrary to the repository deletion policy.

### MINOR - `crates/nova_gameplay/src/mesh/builder.rs:162,274,290,342` - Dead vertex-mesh paths retain stale caller documentation

Workspace-wide searches found no production callers for `apply_noise`, `subdivide`, generic `new`, or `try_from_mesh`. Documentation in `nova_scenario/src/objects/asteroid.rs:684-686` says planet noise is handed to `apply_noise`, while production asteroid meshing uses `SignedField`. The `try_from_mesh` documentation also cites the explode plugin, which does not call it.

The live octahedron/cone builder paths remain used. The finding is limited to obsolete methods and stale ownership claims.

## Adjudication

No correctness or contract defects were found in audio routing, juice throttling, camera shake, transient-light budgets, impact effects, freeze ownership, deterministic hashes, signed-field meshing, markers, objectives, or plugin registration.

## Coverage

Fully read all 35 remaining `nova_gameplay` Rust files, including audio, transform, mesh, effects, support resources, plugin composition, and inline tests. Combined with reviews 02-03, every Rust file in `nova_gameplay` has a full static pass.

Cross-crate symbol searches and targeted reads checked transform and mesh consumers, asteroid generation, explode behavior, audio consumers, asset paths, objective consumers, and clock-freeze ownership.

Not checked: no Cargo command, content lint, game, probe, GPU measurement, wasm build, workspace test, or Clippy run. Downstream ship, scenario, HUD, and OS UI behavior was only traced enough to establish the named contracts and remains assigned to later batches.
