# The velocity HUD sphere casts a world shadow over the ship's

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.11.0,hud,render

# The velocity HUD sphere casts a world shadow over the ship's

Owner report (2026-08-14, live play): the shadow on screen is the HUD
sphere's, not the ship's.

## Where

`crates/nova_hud/src/velocity.rs`, `insert_velocity_hud_sphere_system` -
the "VelocityHUD Sphere" child.

## Diagnosis

Not a shader bug. The sphere is a real world-space `Mesh3d` with a
`StandardMaterial`-based `ExtendedMaterial`, and Bevy casts shadows from
every such mesh by default. The spawn carries no shadow opt-out:

```rust
Name::new("VelocityHUD Sphere"),
VelocityHudSphereMarker,
Transform::from_translation(...).with_scale(Vec3::splat(radius)),
Mesh3d(meshes.add(mesh)),
MeshMaterial3d(direction_materials.add(ExtendedMaterial { ... })),
```

The sphere is centred a radius ahead of the ship and scaled to that
radius, so it sits between the light and the hull and its round shadow
covers the ship's own.

`NotShadowCaster` appears NOWHERE in the workspace, so this is a whole
missing convention rather than one bad spawn. Every other world-space HUD
mesh has the same defect and should be swept in the same pass:

- `velocity.rs` - the direction sphere and its cone marker
- `holo_instruments.rs` - segment and gate meshes
- `maneuver_instruments.rs` - the torus and segment meshes
- `target_inset.rs` - the inset mesh

## Fix

Add `NotShadowCaster` to each. Add `NotShadowReceiver` too where the
instrument should read as emissive rather than lit - check each against a
frame before deciding, since some may be deliberately shaded.

Consider whether the HUD layer wants a shared bundle/marker so a future
instrument cannot forget it; a lint-style test asserting every entity in
the HUD's render layer carries `NotShadowCaster` would hold it.

## Done when

- The ship casts its own shadow with the velocity HUD live.
- Verified from RENDERED FRAMES, not exit status: `Xvfb` + a lit scene
  with a directional light and shadows on (`screenshot_flight` is already
  a shadowed set) - diff a before/after capture.
