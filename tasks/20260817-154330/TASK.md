# Generalize destruction visuals for sections, ordnance, and ships

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.11.0,destruction,ship,torpedo,render,physics,spike

## Goal

Replace the current representation-dependent destruction finale with one path
that can break apart any rendered spaceship section, shot-down ordnance, and a
whole spaceship when appropriate. Destruction eligibility must not depend on a
`Mesh3d` living directly on the gameplay entity.

This is a destruction-VFX and render-geometry task. It complements, but does
not absorb, the persistent carving epic in `tasks/20260813-224826`: health and
the integrity graph still decide WHEN a semantic section dies. This task decides
how its authored or procedural visual geometry comes apart afterward.

## Current limitation

- Most sections carry health, collider, and `ExplodableEntity` on a gameplay
  root, but render through descendant entities under `WorldAssetRoot`.
- The explosion observer requires `Mesh3d` on the destroyed entity itself even
  though the cutter can recursively collect descendant meshes.
- Meshless section roots therefore spawn eight generic gray cubes for two
  seconds and despawn recursively.
- The shipped procedural torpedo body can be sliced only when the directly
  meshed controller child is the section destroyed first. Another child dying,
  or an authored glTF torpedo, takes the cube/despawn path instead.
- Proximity detonation directly consumes the torpedo and spawns its blast. Keep
  that behavior: the blast is the intended finale, not a shard burst.
- `ExplodableEntity` propagates to parent roots. Relaxing the `Mesh3d` filter
  without defining root behavior can accidentally slice an entire hierarchy or
  leave a meshless root alive.

## Investigation

Design a render-geometry destruction seam rather than adding another marker
special case. Compare at least:

- recursively slicing loaded descendant mesh primitives in place;
- snapshotting or detaching render descendants before gameplay despawn;
- an explicit render-geometry provider that hides whether art came from
  procedural `Mesh3d`, glTF `WorldAssetRoot`, articulated joints, or derived
  skin;
- offline fracture data for authored assets, with runtime slicing as fallback.

Measure and resolve:

- scene-loading races and a deterministic fallback when render meshes are not
  available;
- local-to-world transforms, `render_mesh_transform`, multiple materials,
  multiple primitives, turret joints, cladding descendants, and skinned meshes;
- fragment budget semantics across a whole section rather than accidentally per
  primitive;
- collider cost, native and WASM slicing cost, fragment lifetime, asset cleanup,
  and capital-collapse body count;
- exact-once `OnDestroyed`/`OnDefeated` behavior and immediate removal of dead
  gameplay collision/capabilities;
- direct destruction of a spaceship root, including whether surviving section
  visuals are fragmented or handed to the structural teardown first.

Keep the existing generic cube burst as the production fallback until a
rendered and measured replacement is accepted. Never emit both fallback cubes
and successful mesh fragments for one destroyed section.

## Required coverage

Build an isolated player-path range before production integration. It must
cover:

- every section kind: hull, thruster, controller, turret, and torpedo bay;
- procedural one-mesh art and authored glTF descendant meshes;
- a multi-part articulated turret and a clad section;
- unloaded, unsupported, and failed-to-slice meshes using the cube fallback;
- a shot-down torpedo through either child producing the same destruction
  result;
- proximity detonation producing the blast and no shard burst;
- direct whole-spaceship destruction with one exact scenario outcome;
- a capital-scale collapse with bounded peak bodies and complete cleanup;
- native and WASM behavior.

## Done when

- destruction is keyed to semantic destructibility, not direct `Mesh3d`
  placement;
- any shipped or modded section representation follows one documented finale
  contract;
- shot-down torpedo destruction is independent of which child dies first;
- intentional torpedo detonation remains direct consumption into its blast;
- the cube fallback remains reliable for unavailable or unsupported geometry;
- measured fragment budgets prevent a compound ship from multiplying one
  section death into unbounded physics bodies;
- relevant gameplay, modding, and developer documentation matches the shipped
  contract.
