# Turret joint tree: multi-muzzle firing (N muzzles, per-muzzle timers, shared magazine)

- STATUS: CLOSED
- PRIORITY: 9
- TAGS: v0.7.0, spike, refactor, weapons, turret

## Goal

Generalize the fire path from one barrel to N muzzles per mount.
`shoot_spawn_projectile` currently reads the single `TurretSectionMuzzleEntity`
and its one `TurretSectionBarrelFireState`; iterate the root's
`TurretSectionMuzzles` instead, one fire timer per muzzle, keeping the sub-tick
lead spawn per muzzle. Keep `SectionAmmo` + reload on the section root as a
shared magazine (any muzzle consumes a round); fire rate is per-muzzle. Update
the AI fire-discipline alignment gate in `input/ai.rs` to check per-muzzle (or
the representative shared tip). Audio (`Add<TurretBulletProjectileMarker>`) is
already per-projectile and needs no change -- verify.

## Notes

Spike: tasks/20260717-214834/SPIKE.md (multi-muzzle firing option A). Depends on
20260717-215804 (muzzle set) and 20260717-215835 (aim). Direction-level; run
/plan for steps.
