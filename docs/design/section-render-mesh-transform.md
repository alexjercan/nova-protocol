# render_mesh_transform for all section kinds (task 20260718-121205)

## What changed

`render_mesh_transform` (the optional position+rotation offset for a section's
render mesh, added for turrets in task 20260718-113307) now works on every
section kind: hull, thruster, controller, and torpedo. As before, it moves the
authored art only and never the section's physics/collider frame.

`RenderMeshTransform` moved from `turret_section.rs` to `base_section.rs` - it is
now a cross-kind concept, so it lives with the other shared section types
(`SectionCollider`, `BaseSectionConfig`). It is re-exported through the sections
prelude, so turret code (and the RON type name) is unchanged. A shared component
`SectionRenderMeshTransform(Option<RenderMeshTransform>)` was added there too.

Per-kind wiring:

| Kind | field on | carrier | applied in |
|------|----------|---------|-----------|
| Hull | `HullSectionConfig` | `SectionRenderMeshTransform` component | `insert_hull_section_render` |
| Thruster | `ThrusterSectionConfig` | `SectionRenderMeshTransform` component | `insert_thruster_section_render` |
| Controller | `ControllerSectionConfig` | `SectionRenderMeshTransform` component | `insert_controller_section_render` |
| Torpedo | `TorpedoSectionConfig` | read straight off `TorpedoSectionConfigHelper` | `insert_torpedo_section_render` |
| Turret | `TurretJoint` (per joint) | `TurretJointRenderMeshTransform` | `insert_turret_joint_render` |

Hull/thruster/controller snapshot the field into the shared
`SectionRenderMeshTransform` component in their `*_section()` bundle fn, and
their observer queries it. The torpedo body is spawned as a separate entity and
its observer already reads the whole `TorpedoSectionConfigHelper`, so it reads
`config.render_mesh_transform` directly - no snapshot component needed.

In every case the transform is applied to the meshed (`WorldAssetRoot`) render
CHILD via `render_mesh_transform.map(RenderMeshTransform::to_transform)
.unwrap_or_default()`, so `None` is identity and the section root (which carries
the collider) is never touched. The default procedural primitives (drawn when
`render_mesh` is `None`) keep their existing poses, matching the turret.

## RON authoring

Any section kind now accepts the same block its turret sibling does, e.g. a hull
cube whose cut GLB needs a nudge:

```ron
kind: Hull((
    render_mesh: Some("self://gltf/cube_i0_j0_k0.glb#Scene0"),
    render_mesh_transform: Some((
        position: (0.0, 0.5, 0.0),
        rotation: (0.0, 0.0, 0.70710677, 0.70710677),
    )),
)),
```

Both sub-fields default (`position` zero, `rotation` identity) and are skipped
when at their default, so authoring just one is legal and omitting the block
serializes to nothing - `content_ron_parity` stays byte-identical.

## Backward compatibility / ripple

The four config structs each gained a field. Every explicit literal was updated
with `render_mesh_transform: None` (the `Default` impls, the nova_assets content
generator, and two test literals), verified by `cargo check --workspace
--all-targets --features debug`. The field carries `serde(default,
skip_serializing_if = "Option::is_none")`, so `base.content.ron` and the parity
set are unchanged.

## Tests

- `hull_section`: the authored transform lands on the meshed hull render child,
  identity when unset (covers the shared-component path used by
  hull/thruster/controller, which are structurally identical); plus a serde test
  that the field round-trips and is omitted when unset.
- `torpedo_section::render`: the same end-to-end assertion on the torpedo body's
  render child - the DISTINCT path that reads the transform off the config
  rather than a snapshot component.
- Existing turret render tests still pass after the type move.
- 101 section tests, `content_ron_parity`, and `content_lint_gate` all green.

## Note

Thruster and controller share the exact hull mechanism (same
`SectionRenderMeshTransform` component, same `map/unwrap_or_default` on the mesh
child), so they are covered by the hull integration test rather than duplicated
per kind; the torpedo path differs and has its own test.
