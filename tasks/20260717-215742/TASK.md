# Turret joint tree: recursive TurretJoint RON data model + migrate shipped def

- STATUS: CLOSED
- PRIORITY: 6
- TAGS: v0.7.0, spike, refactor, weapons, turret

## Goal

Replace the flat `TurretSectionConfig` (fixed `yaw/pitch/barrel/muzzle`
offsets, meshes, `min_pitch`/`max_pitch`) with a recursive `TurretJoint` tree:
each node has `offset: Vec3`, optional rotation `axis` (None = fixed), `min`/
`max`/`speed`, optional `render_mesh`, optional `muzzle`, and `children`. The
config carries firing/ammo defaults plus a `root: TurretJoint`. Migrate the one
shipped turret def (`assets/base/sections/base.content.ron`,
`better_turret_section`) to the tree form and delete the legacy flat fields
(one code path, no serde compat shim). Golden test: the migrated def produces
the same 6-entity chain and aim behavior as today.

This is the first task in the chain; the ECS joint-walk, aim solver,
multi-muzzle firing, and editor/lint tasks build on this data model.

## Notes

Spike: tasks/20260717-214834/SPIKE.md (data model options A/B/C, migration
A/B). This task is direction-level; run /plan to break it into steps.
