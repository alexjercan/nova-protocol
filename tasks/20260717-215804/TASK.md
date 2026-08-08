# Turret joint tree: generic TurretJoint ECS component + single chain-walk system + tree render

- STATUS: CLOSED
- PRIORITY: 7
- TAGS: v0.7.0, spike, refactor, weapons, turret

## Goal

Retire the per-joint marker zoo (`TurretRotatorBaseMarker`,
`TurretSectionRotatorYaw{Base}Marker`, `...Pitch{Base}Marker`,
`...BarrelMarker`) and the two sync systems (`sync_turret_rotator_yaw_system`,
`sync_turret_rotator_pitch_system`). Replace with:

- one generic `TurretJoint { axis, min, max, speed }` component (backed by the
  existing `SmoothLookRotation`) on each articulated node,
- a `TurretMuzzle` component on each fire-point leaf and
  `TurretSectionMuzzles(Vec<Entity>)` on the root (superseding the single
  `TurretSectionMuzzleEntity`),
- the `insert_turret_section` Add-observer walking the `TurretJoint` tree to
  spawn the entity chain + per-node render meshes,
- one generic sync system (or fold controller output straight onto transforms).

Aim solving stays in the next task; this task delivers the generic chain
representation and rendering with behavior parity for the legacy tree.

## Notes

Spike: tasks/20260717-214834/SPIKE.md (ECS representation option A). Depends on
the data-model task 20260717-215742. Direction-level; run /plan for steps.
