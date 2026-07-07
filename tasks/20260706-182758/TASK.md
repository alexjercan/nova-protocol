# Section destruction visuals: explode/break apart gltf sections on destroy

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.3.1, health, polish

Follow-up from task 20260706-174738 (the section-destruction fix). Destroyed ship
sections now despawn correctly, but they vanish silently instead of exploding - there is
no visual for a section being destroyed.

Why it is not already handled: the explosion system (integrity/explode.rs +
bevy_common_systems ExplodeMesh) slices an entity's own `Mesh3d` into flying fragments.
That works for asteroids (procedural `Mesh3d` on the collider entity) but not for
sections: sections render through a gltf `WorldAssetRoot`, so their meshes live in child
entities of the loaded scene, with gltf materials - they cannot be fed to the current
slicer, and `on_explode_entity` skips them (it requires `With<Mesh3d>`), so
`despawn_destroyed_without_mesh` just removes them with no effect.

Options to implement:
- Slice the section's gltf submeshes: on destroy, find the `Mesh3d` descendants of the
  section, detach them, tag them so `handle_entity_explosion` will process them (it
  currently requires `ExplodableEntity` and `MeshMaterial3d<StandardMaterial>` - gltf
  materials may not match), and explode each. Most faithful to the asteroid look but the
  most work and the most coupling to untangle.
- Spawn a generic debris/particle burst at the destroyed section's transform (e.g. a
  short-lived `ParticleEffect` + `TempEntity`, like the torpedo blast effect) and keep the
  silent despawn. Simple, decoupled, good enough visually.

Suggested: start with the particle-burst version; revisit gltf slicing only if a
chunkier look is wanted.

## Steps

- [x] Add a `spawn_section_debris` observer on `Add<IntegrityDestroyMarker>` in
      integrity/explode.rs, filtered to meshless `SectionMarker` entities.
- [x] Scatter a burst of small physics debris cubes at the section's world position,
      launched outward with random spin, auto-despawned via `TempEntity`.
- [x] Register it in `ExplodablePlugin` and build/clippy/fmt clean.
- [x] Verify in-game that destroyed sections visibly break apart.

## Implementation notes

Went with the debris-burst option (not gltf slicing) - it is fully decoupled from the
gltf/material coupling that made slicing expensive, and matches the existing zero-gravity
space feel. `spawn_section_debris` (integrity/explode.rs) runs alongside
`despawn_destroyed_without_mesh` on the same `Add<IntegrityDestroyMarker>` event: it reads
the section's `GlobalTransform` (still present - both observers defer via `Commands`, so
the despawn has not been applied yet) and spawns 8 `Cuboid` debris entities.

Chose physics debris over a hanabi `ParticleEffect` for two reasons: bevy_hanabi is
disabled on wasm (the torpedo blast has a wasm FIXME), and physics debris reuses the same
`RigidBody::Dynamic` + `Collider` + `LinearVelocity` path as the asteroid fragments, so it
behaves consistently under the scene's zero gravity. Each cube gets a small `Collider`
(supplies mass, avoids avian's NaN "no mass" warning), an outward `LinearVelocity`, a
random `AngularVelocity` for tumble, `MeshFragmentMarker` (so it is scenario-scoped and
cleaned on scene switch), and `TempEntity(2.0)` (self-despawn after 2s). The mesh/material
are created once per burst and cloned across the 8 cubes.

Only `SectionMarker` entities are handled; the meshless ship root is excluded via the
query filter (its sections have already burst by the time the root dies).
