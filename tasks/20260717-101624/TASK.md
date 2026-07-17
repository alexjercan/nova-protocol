# Weapon-section one-shot sounds: dry_fire on the turret, launch sound on the torpedo bay

- STATUS: OPEN
- PRIORITY: 31
- TAGS: spike,v0.7.0,audio,modding,feature


## Goal

Extend the landed turret `fire_sound` pattern (task 20260717-002228) to the
other weapon one-shots: `dry_fire_sound` on `TurretSectionConfig` (cue site
`play_dry_fire_cue` already holds the turret entity) and `launch_sound` on
`TorpedoSectionConfig` (the projectile carries `TorpedoSectionPartOf` /
`TorpedoSectionSpawnerEntity` back-refs). gen_content authors base defaults;
cues become authored-or-silent and their `WorldSfx` keys are deleted.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md. Depends on 20260717-101615 (bank split).
- Stepless direction-level task: run /plan before /work.
