# Section destruction visuals: explode/break apart gltf sections on destroy

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.4.0,health,polish


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
