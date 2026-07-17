# Weapon-section one-shot sounds: dry_fire on the turret, launch sound on the torpedo bay

- STATUS: IN_PROGRESS
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

## Plan (2026-07-17)

Verified mechanisms: dry-fire cue polls turrets directly (turret Entity in
hand, audio.rs `play_dry_fire_cue`); the torpedo projectile carries
`TorpedoSectionSpawnerEntity` -> the SPAWNER entity, which already snapshots
`TorpedoSectionSpawnerEffect(Option<AssetRef<EffectAsset>>)` (torpedo_section/
mod.rs:212, insert at :475/:644) - the exact sibling template for a launch
sound (LESSONS: mirror-sibling-resolve-site - snapshot unresolved at build,
resolve in the audio observer).

### Steps

- [x] `dry_fire_sound: Option<AssetRef<AudioSource>>` on `TurretSectionConfig`
      (attrs mirror `fire_sound`); snapshot as `TurretSectionDryFireSound` next
      to `TurretSectionFireSound` in `insert_turret_section`; `play_dry_fire_cue`
      resolves the firing turret's authored ref (Res<AssetServer>) -
      authored-or-silent, latch logic untouched.
- [x] `launch_sound: Option<AssetRef<AudioSource>>` on `TorpedoSectionConfig`;
      snapshot as `TorpedoSectionLaunchSound` on the SPAWNER (both insert
      sites, mod.rs:475 + :644 region - sweep all spawner builds);
      `on_torpedo_launch_play_sfx` reaches it via the projectile's
      `TorpedoSectionSpawnerEntity` and resolves - authored-or-silent.
- [x] Flip `fire_sound` to authored-or-silent: drop the WorldSfx::TurretFire
      fallback in `on_turret_fire_play_sfx` (base content always authors it via
      gen_content, so shipped audio is unchanged).
- [x] Shrink WorldSfx: delete TurretFire, TorpedoLaunch, DryFire keys + their
      WORLD_SFX_FILES rows (12 -> 9) + the every-key guard rows. The wavs STAY
      shipped + in base resources (content references them now).
- [x] gen_content: `SectionMeshRefs` += `turret_dry_fire_sound` +
      `torpedo_launch_sound` (self://sounds/...); wire both turrets' `dry_fire_
      sound` and the torpedo bay's `launch_sound` in build_sections; regenerate
      base content; parity + lint gates stay green.
- [x] Tests: (a) turret with authored dry_fire_sound clicks with THAT handle,
      without one is silent (delivery guard = the authored case); latch
      behavior tests keep passing with authored rigs; (b) torpedo launch plays
      the authored handle, silent when unset; (c) fire cue: authored plays,
      unset is SILENT now (replaces the bank-fallback test).
- [x] Docs: wiki guide-author-section (turret `dry_fire_sound`, torpedo
      `launch_sound` fields), base sounds README rows, CHANGELOG modding line
      (turrets/torpedoes fully own their weapon sounds), spike fix record.
      Check every surface the step names against the diff (LESSONS x4).
- [x] Verify: fmt; nova_gameplay lib; content gates; workspace all-targets
      check reading output (LESSONS: piped-cargo-masks-exit-code).
