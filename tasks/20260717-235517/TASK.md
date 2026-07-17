# Thruster exhaust shape: support square/rect exhaust (Kenney nozzles are square, not round cones)

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.7.0, gameplay, thruster, shader, backlog

## Goal

Let a thruster's exhaust render as a square (axis-aligned) nozzle flame instead
of only a round cone, so the cut Kenney engines (square nozzles) look right.

## Design

The exhaust shader (`assets/shaders/thruster_exhaust.wgsl`) elongates the mesh
along +Y using `length(position.xz)` for the radial falloff and `position.y` for
the flame length - it works for ANY mesh whose base is at y=0 and tip at y=1, so
only the MESH shape needs to change, not the shader.

- Add `ThrusterExhaustShape { Cone, Square }` (Reflect + serde, Default `Cone`).
  `Square` reuses `exhaust_radius` / `exhaust_inner_radius` as the HALF-SIDE, so
  no new fields; the shader falloff (`max_r = radius`) still works (corners fade).
- Add `shape: ThrusterExhaustShape` to `ThrusterExhaustConfig`.
- Build a unit square pyramid inline (base square [-1,1]^2 at y=0, tip at y=1, 4
  axis-aligned sides subdivided by height + a base cap), mirroring
  `TriangleMeshBuilder::new_cone`, then scale by `(radius, height, radius)` like
  the cone. `bevy_common_systems` is a pinned dep so the builder lives here.
- `insert_thruster_shader` picks cone vs square per `config.shape` for both the
  outer and inner meshes.
- Author craft_cargob's engines with `shape: Square`.

## Steps

- [x] `ThrusterExhaustShape` enum (Cone | Square), Reflect + serde, Default Cone.
- [x] Add `shape` field to `ThrusterExhaustConfig` (serde default).
- [x] Inline `square_exhaust_builder(height_subdivisions)` -> TriangleMeshBuilder.
- [x] `insert_thruster_shader`: select cone/square mesh per shape (outer + inner).
- [x] craft_cargob thrusters author `shape: Square`.
- [x] cargo check nova_gameplay + content lint.

## Notes

- Rectangle with independent width/depth is a further extension if ever needed;
  square (half-side = radius) covers the Kenney nozzles.
