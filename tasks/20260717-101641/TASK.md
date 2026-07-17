# Per-target impact and destroy sounds: sections, asteroids, torpedo detonation

- STATUS: OPEN
- PRIORITY: 28
- TAGS: spike,v0.7.0,audio,modding,feature


## Goal

Impact and destruction sounds become per-TARGET content: `impact_sound` +
`destroy_sound` on `BaseSectionConfig` (every section kind) and on
`AsteroidConfig` (already an AssetRef-carrying content config), plus
`detonation_sound` on `TorpedoSectionConfig` (snapshotted onto the projectile).
The `On<HealthApplyDamage>` / `On<Add, IntegrityDestroyMarker>` observers read
the target/destroyed entity's snapshot; throttling, area cells and distance
attenuation unchanged. Per-target = per-material variety (a rock, a light hull
and a reinforced hull can each sound different) with no bullet-x-material
matrix - a projectile-side modifier is a deferred extension, out of scope here.
gen_content authors base defaults; delete the impact/explosion `WorldSfx` keys.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md. Depends on 20260717-101615 (bank split).
- The fiddly one of the family: most cue sites and target kinds; sweep every
  spawner that must snapshot the sounds (sections, asteroids, torpedo blast).
- Stepless direction-level task: run /plan before /work.
