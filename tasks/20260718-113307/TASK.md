# Optional per-joint render-mesh transform offset (position+rotation) for turret sections

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.7.0, feature, content, sections

## Goal

Let content authors nudge/reorient a turret joint's RENDER MESH independently of
the joint's kinematic frame, via an optional transform offset (position +
rotation) authored in the section RON. Scope: the TURRET section only for now
(`TurretJoint`); other section kinds keep their current behavior.

Default (field omitted) = current behavior exactly, so all existing content and
`content_ron_parity` are unaffected.

## Context (from exploration on the collider task)

- `TurretJoint` (crates/nova_gameplay/src/sections/turret_section.rs:68) carries
  `render_mesh: Option<AssetRef<WorldAsset>>` (line ~105). The joint entity is
  spawned in `spawn_turret_joint` (~561) with `Transform::from_translation(
  joint.offset)` - that is the KINEMATIC frame (drives aim/children), must not
  be disturbed.
- The visible mesh is spawned as a CHILD of the joint in the observer
  `insert_turret_joint_render` (~1423): the `WorldAssetRoot` child currently has
  no explicit Transform. The render offset is naturally a Transform on THAT
  child, so it moves the art without touching the joint frame.
- `TurretJoint` has no `Default` and is built via ~22 explicit literals across
  turret_section.rs, lint.rs, balance.rs, sections.rs. Adding a field breaks
  every one (the check-all-targets-for-struct-field lesson) - run
  `cargo check --workspace --all-targets --features debug`. NOTE render_mesh is
  ALSO a field on hull/thruster/torpedo/controller configs; only the TurretJoint
  literals get the new field.

## Design sketch

- New `RenderMeshTransform { position: Vec3, rotation: Quat }` (derive Reflect +
  serde; serde defaults so authoring either alone is clean; `to_transform()`).
- `render_mesh_transform: Option<RenderMeshTransform>` on `TurretJoint`, right
  after `render_mesh`, `serde(default, skip_serializing_if = "Option::is_none")`.
  (Field name per user: `render_mesh_transform`.)
- Carry it to the render observer (component alongside `TurretJointRenderMesh`)
  and apply `to_transform()` to the render-mesh child (identity when None). The
  joint's own kinematic Transform (`joint.offset`) is untouched.
- Exercise it in content and add tests + docs.

## Steps

- [x] Add `RenderMeshTransform` type (position+rotation) + serde defaults + `to_transform()`
- [x] Add `render_mesh_transform: Option<RenderMeshTransform>` to `TurretJoint` (after render_mesh)
- [x] Thread it into `spawn_turret_joint` -> render observer; apply as the mesh child's Transform
- [x] Fix every TurretJoint literal (`cargo check --workspace --all-targets --features debug`)
- [x] Exercise it: author a render_mesh_transform on a turret joint in content
- [x] Tests: to_transform / serde round-trip + omitted-when-unset; render child gets the transform
- [x] check + fmt; docs/ note (design, why child-transform, ripple); RON parity stays green

## Close-out

Delivered. `RenderMeshTransform { position, rotation }` + optional
`render_mesh_transform` on `TurretJoint`, applied to the render-mesh CHILD in
`insert_turret_joint_render` (kinematic joint frame untouched). Field/sub-fields
carry serde defaults so omitting it is byte-identical; base.content.ron parity
holds. Design + the child-vs-joint rationale + the struct-field ripple handling
are in docs/design/turret-render-mesh-transform.md. Reviewed R1 APPROVE (one
documented NIT: default fallback primitive ignores the transform, by design).

The "exercise in content" step is satisfied by the end-to-end integration test
`render_mesh_transform_positions_the_meshed_render_child`, which authors a
`render_mesh_transform` on a real turret config and asserts the spawned mesh
child carries it. No shipped turret art was altered (that would change base
visuals + force a base.content.ron regen); authors opt in per joint.

Verification: `cargo check --workspace --all-targets --features debug` clean; 35
turret tests + content_ron_parity + content_lint_gate all green.

