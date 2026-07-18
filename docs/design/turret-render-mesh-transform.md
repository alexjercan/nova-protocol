# Per-joint render-mesh transform for turret sections (task 20260718-113307)

## What changed

Turret joints can now offset/reorient their render mesh independently of the
joint's kinematic frame. A new optional field on `TurretJoint`:

```rust
pub struct RenderMeshTransform {
    pub position: Vec3,   // omitted when zero
    pub rotation: Quat,   // omitted when identity
}

// on TurretJoint, right after render_mesh:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub render_mesh_transform: Option<RenderMeshTransform>,
```

`RenderMeshTransform::to_transform()` builds a bevy `Transform`
(`from_translation(position).with_rotation(rotation)`, scale left at 1).

RON authoring (both sub-fields default, so either alone is legal):

```ron
(
    offset: (0.0, 0.2, 0.0),
    render_mesh: Some("self://gltf/turret-barrel.glb#Scene0"),
    render_mesh_transform: Some((
        position: (0.0, 0.1, -0.05),
        rotation: (0.0, 0.70710677, 0.0, 0.70710677),
    )),
    // ...
)
```

Scope is the turret section only for now (`TurretJoint`); other section kinds
(hull/thruster/torpedo/controller) are untouched.

## Where it applies: the mesh CHILD, not the joint

`spawn_turret_joint` builds the joint entity with
`Transform::from_translation(joint.offset)` - that is the KINEMATIC frame that
drives the aim solver and parents child joints. The visible mesh is a separate
CHILD entity spawned by the `insert_turret_joint_render` observer (a
`WorldAssetRoot`). The render transform is applied to that child, so it moves
the art only and never perturbs aim, hinge axes, or the joint tree. When the
field is `None` the child gets an identity transform - byte-for-byte the old
behavior.

The transform is snapshotted onto the joint entity as a
`TurretJointRenderMeshTransform(Option<RenderMeshTransform>)` component
alongside `TurretJointRenderMesh`, because the observer keys on the joint entity
and does not re-walk the authored joint tree. It is applied only in the
authored-mesh (`WorldAssetRoot`) branch; the default fallback primitive (drawn
for an unmeshed structural joint) is deliberately left at its existing
`from_xyz(0.0, 0.05, 0.0)` base-plate pose, since `render_mesh_transform`
describes a render MESH and an unmeshed joint has none.

## Backward compatibility / parity

Both `RenderMeshTransform` sub-fields and the joint field itself carry serde
`default` + `skip_serializing_if`, so any joint that omits the field serializes
identically to before. The base turret section (generated in
`nova_assets/src/sections.rs`) sets none of them, so `base.content.ron` is
unchanged and `content_ron_parity` stays green.

## The struct-field ripple (again)

`TurretJoint` has no `Default` and is built via ~19 explicit literals across
four crates (turret_section.rs, lint.rs, balance.rs, sections.rs). Adding the
field broke every one. Complicating it: `render_mesh` is ALSO a field name on
hull/thruster/torpedo/controller configs, so a blind "insert after render_mesh"
sweep would have touched the wrong structs. The insertion was scoped to files
whose only `render_mesh` literals are `TurretJoint` (turret_section.rs,
lint.rs, balance.rs) plus the turret joint-tree line range in sections.rs, then
`cargo check --workspace --all-targets --features debug` confirmed no
`TurretJoint` literal was missed and no other struct was wrongly edited. (Lesson
`check-all-targets-for-struct-field`.)

## Tests

- `render_mesh_transform_positions_the_meshed_render_child`: end-to-end through
  the real `insert_turret_section` + `insert_turret_joint_render` observers - a
  meshed joint with an authored transform yields a `WorldAssetRoot` child whose
  local `Transform` matches (translation exact, rotation via `abs_diff_eq`,
  scale untouched), and a meshed joint without it gets `Transform::IDENTITY`.
- `render_mesh_transform_type_defaults_and_round_trips`: default is identity;
  `to_transform` maps position/rotation correctly.
- `render_mesh_transform_serde_round_trips_and_omits_defaults`: full round-trip,
  zero-position omission, and a joint omitting the field not serializing it.

## Difficulty note

The render test first panicked twice in the asset server: the `WorldAssetRoot`
child does `asset_server.load::<WorldAsset>(path)`, which needs
`app.init_asset::<WorldAsset>()` (not just `Mesh`/`StandardMaterial`) in the
minimal test app, and a plain schemeless path (a `dep://` source is not
registered in the test). Then `Quat::angle_between` was the wrong comparison for
"same orientation"; `Quat::abs_diff_eq` is the correct quaternion equality.
