# Review: render_mesh_transform on all section kinds

- TASK: 20260718-121205
- BRANCH: section-render-mesh-transform

## Round 1

- VERDICT: APPROVE

Independently verified the load-bearing claims:

- **Type move is sound.** `RenderMeshTransform` moved from turret_section.rs to
  base_section.rs and is re-exported through the sections prelude. Turret code is
  unchanged and its 5 render tests still pass, so the move did not regress the
  original feature. The RON type name is unaffected (serde uses the struct name,
  which is stable across modules).
- **Every kind actually APPLIES the transform, not just queries it.** Grep-checked
  the `Some` branch of all four observers: hull, thruster, controller each
  compute `render_mesh_transform.map(to_transform).unwrap_or_default()` and pass
  `transform` into the `children![(...)]` mesh child; torpedo does the same from
  `config.render_mesh_transform`. No "compute-but-drop" bug.
- **It lands on the child, not the section root.** In all four, the transform is
  inside the `children![...]` block; the section entity's own Transform (which
  carries the collider/physics) is never written. So art moves, collider does
  not - the whole point.
- **Shared vs per-kind is correct.** Hull/thruster/controller use the identical
  shared `SectionRenderMeshTransform` component snapshotted in their bundle fn;
  torpedo (whose body is a separate entity and whose observer already reads the
  full config) reads the field directly. Both paths are exercised: hull by
  integration test (representative for the three identical component-based
  kinds), torpedo by its own test (the distinct config-read path).
- **Ripple contained.** Adding a field to 4 config structs broke their `Default`
  impls, the nova_assets generator (5 sites), and 2 test literals; all fixed and
  `cargo check --workspace --all-targets --features debug` is clean.
- **Parity/back-compat.** `serde(default, skip_serializing_if)` means omitted
  fields do not serialize; `content_ron_parity` and `content_lint_gate` pass, so
  shipped content is byte-identical. Test strength: reverting a Some-branch edit
  breaks that kind's render test; removing a field breaks the serde test.

Findings:

- [ ] R1.1 (NIT) thruster_section.rs / controller_section.rs - these two kinds
  have no dedicated integration test; they are covered by the hull test because
  the mechanism (shared `SectionRenderMeshTransform` component + identical
  observer application) is byte-identical across the three. Acceptable and
  documented in docs/design; a future refactor that diverges one of them should
  add its own test.
  - Response: Intentional; the three share one mechanism, hull is the
    representative. Documented.

- [ ] R1.2 (NIT) default procedural primitives (drawn when `render_mesh` is
  `None`) ignore `render_mesh_transform`, matching the turret precedent (the
  field describes a render MESH; an unmeshed section has none). Recorded as a
  deliberate decision.
  - Response: Intentional and documented.
