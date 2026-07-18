# Extend render_mesh_transform to all section kinds (hull, thruster, controller, torpedo)

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.7.0, feature, content, sections

## Goal

Task 20260718-113307 added `render_mesh_transform: Option<RenderMeshTransform>`
to the TURRET joint. Extend the same authorable position+rotation offset for the
render mesh to every other section kind: hull, thruster, controller, torpedo.
Field omitted = identity = unchanged behavior; parity stays green.

## Architecture (from exploration)

Each non-turret kind has a config struct with `render_mesh:
Option<AssetRef<WorldAsset>>` and an observer that spawns the mesh as a CHILD of
the section entity (never the section root, so a transform on the child moves
art only):
- Hull: `HullSectionConfig` / `hull_section()` / `insert_hull_section_render`
  (hull_section.rs) - reads snapshot `HullSectionRenderMesh`.
- Thruster: `ThrusterSectionConfig` / `thruster_section()` /
  `insert_thruster_section_render` (thruster_section.rs) - snapshot
  `ThrusterSectionRenderMesh`.
- Controller: `ControllerSectionConfig` / `controller_section()` /
  `insert_controller_section_render` (controller_section.rs) - snapshot
  `ControllerSectionRenderMesh`.
- Torpedo: `TorpedoSectionConfig` / `insert_torpedo_section_render`
  (torpedo_section/render.rs) - reads the whole `TorpedoSectionConfigHelper`
  directly (no snapshot), body is a child entity `TorpedoSectionBodyMarker`.

`RenderMeshTransform` currently lives in turret_section.rs but is now a
cross-kind concept.

## Design

- MOVE `RenderMeshTransform` + serde helpers (`is_zero_translation`,
  `is_identity_rotation`) from turret_section.rs to base_section.rs (its correct
  home); export from base_section prelude. turret keeps working via the crate
  prelude; drop it from turret's prelude export.
- Add a shared component `SectionRenderMeshTransform(Option<RenderMeshTransform>)`
  in base_section.rs.
- Hull/Thruster/Controller: add `render_mesh_transform` field (after
  render_mesh, `serde(default, skip_serializing_if = "Option::is_none")`); the
  bundle fn inserts `SectionRenderMeshTransform(config.render_mesh_transform)`;
  the observer queries it and applies `to_transform()` to the meshed (Some)
  render child, identity when None.
- Torpedo: add the field; `insert_torpedo_section_render` reads
  `config.render_mesh_transform` and applies it to the body's meshed child.
- Apply only to the authored-mesh (`WorldAssetRoot`) branch, matching turret;
  default procedural primitives keep their existing poses.
- Ripple: the 4 config structs gain a field -> fix every literal
  (`cargo check --workspace --all-targets --features debug`).

## Steps

- [x] Move RenderMeshTransform (+ helpers) to base_section.rs; add SectionRenderMeshTransform; fix turret to use it
- [x] Hull: field + snapshot + observer applies transform
- [x] Thruster: field + snapshot + observer applies transform
- [x] Controller: field + snapshot + observer applies transform
- [x] Torpedo: field + observer (reads config) applies transform
- [x] Fix all config-struct literals; cargo check --all-targets clean
- [x] Tests: per-kind render child carries the transform (+ identity when unset); serde
- [x] check + fmt; docs/ note; RON parity + lint gate green

## Close-out

Delivered. `render_mesh_transform` now works on hull, thruster, controller and
torpedo, matching the turret. `RenderMeshTransform` moved to base_section.rs
(shared home) + new shared `SectionRenderMeshTransform` component;
hull/thruster/controller snapshot it and apply in their render observer, torpedo
reads it off the config. Always applied to the meshed render child, never the
collider frame; None = identity. Full design + the shared-vs-per-kind split are
in docs/design/section-render-mesh-transform.md. Reviewed R1 APPROVE (two
documented NITs). 101 section tests + hull/torpedo render-transform tests + serde
+ content_ron_parity + content_lint_gate all green; workspace --all-targets clean.
