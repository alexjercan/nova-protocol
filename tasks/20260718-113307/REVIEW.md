# Review: Per-joint render-mesh transform for turret sections

- TASK: 20260718-113307
- BRANCH: turret-render-offset

## Round 1

- VERDICT: APPROVE

Independently re-verified the load-bearing claims:

- **Transform lands on the mesh child, not the joint.** `spawn_turret_joint`
  still builds the joint entity with `Transform::from_translation(joint.offset)`
  (the kinematic frame); my change only adds a sibling
  `TurretJointRenderMeshTransform` component. The authored transform is inserted
  inside `children![( transform, ... WorldAssetRoot )]` in
  `insert_turret_joint_render`. So the joint frame (aim, hinge axes, child
  joints) is provably untouched. The integration test confirms the child's local
  `Transform` matches the authored value and is identity when unset.
- **Only TurretJoint got the field, and no other struct was wrongly edited.**
  `render_mesh` is a field on hull/thruster/torpedo/controller configs too. The
  perl insertion was scoped to files whose only `render_mesh` literals are
  `TurretJoint` plus the turret line-range in sections.rs. The decisive proof is
  that `cargo check --workspace --all-targets --features debug` compiles clean:
  had a `render_mesh_transform: None` landed in a `HullSectionConfig` (etc.)
  literal it would be an "unknown field" error, and had any `TurretJoint` literal
  been missed it would be a "missing field" error. Neither occurred.
- **Backward compat / parity.** `render_mesh_transform` and both sub-fields carry
  serde `default` + `skip_serializing_if`; the serde test asserts an unset field
  is not serialized, and `content_ron_parity` + `content_lint_gate` pass, so
  shipped content is byte-identical.
- **No render regression.** The pre-existing
  `every_turret_joint_render_child_is_parented_to_its_joint` and the full 35-test
  turret module still pass; adding an identity `Transform` to the meshed child is
  a no-op relative to the previous no-Transform child.
- **Test strength.** Deleting the observer wiring would break the integration
  test (child would be identity, not the authored transform); removing the field
  breaks the serde test. Both would fail with the fix gone.

Findings:

- [ ] R1.1 (NIT) turret_section.rs `insert_turret_joint_render` - the authored
  transform is applied only in the `Some(mesh)` branch; the default fallback
  primitive (unmeshed structural joint) keeps its fixed `from_xyz(0,0.05,0)`
  pose and ignores `render_mesh_transform`. Intentional (the field describes a
  render MESH; an unmeshed joint has none) and documented in docs/design, but
  flagged so it is a recorded decision rather than an oversight.
  - Response: Intentional and documented; leaving as-is.
