# Authorable thruster exhaust cone: offset/rotation/shape config + attach to custom-mesh thrusters

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.7.0, gameplay, thruster, shader

## Goal

Make the thruster exhaust cone authorable and attach it to custom-mesh thrusters.

Problem: `insert_thruster_section_render` only spawns the `ThrusterExhaustConfig`
+ cone in the `None` (procedural body) branch, so a thruster with a custom
`render_mesh` (like the cut Kenney engines) gets NO exhaust. The cone placement
is hardcoded (`rot_x(90deg)`, translate `(0,0,0.3)`, default shape).

## Design

- New authorable `ThrusterExhaust { offset: Vec3, rotation: Quat, shape:
  ThrusterExhaustConfig }` (serde + Reflect, Default = today's hardcoded
  placement/shape). `offset`/`rotation` place the cone; `shape` is the existing
  shader/size struct (made serde).
- Add `exhaust: Option<ThrusterExhaust>` to `ThrusterSectionConfig` (serde
  default None). `thruster_section()` snapshots it as `ThrusterSectionExhaust`.
- `insert_thruster_section_render` spawns the exhaust child ALWAYS (both mesh
  branches) from the config (or default), applying `offset`/`rotation` to the
  child `Transform` and inserting `shape` (which `insert_thruster_shader` already
  turns into the cone mesh + material).

## Steps

- [x] Make `ThrusterExhaustConfig` serde-serializable (`#[serde(default)]`).
- [x] Add `ThrusterExhaust` struct + Default matching current placement.
- [x] Add `exhaust: Option<ThrusterExhaust>` to `ThrusterSectionConfig` + snapshot component.
- [x] Spawn the exhaust child unconditionally in `insert_thruster_section_render`.
- [x] Author the craft_cargob thrusters' exhaust (offset/rotation out the back) and see the cone.
- [x] cargo check nova_gameplay.

## Notes

- Follow-up: tatr 20260717-235517 (square/rect exhaust shape; Kenney nozzles are
  square, the shader only does a round cone).
