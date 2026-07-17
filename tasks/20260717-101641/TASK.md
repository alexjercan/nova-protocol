# Per-target impact and destroy sounds: sections, asteroids, torpedo detonation

- STATUS: CLOSED
- PRIORITY: 28
- TAGS: spike, v0.7.0, audio, modding, feature

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

## Plan (2026-07-17, grounded)

Verified surfaces:
- Sections: `base_section(config)` (base_section.rs:63) is the bundle EVERY
  live section gets (Health via destructible_body) - the snapshot point. The
  damage/destroy observers' target IS the section entity. BaseSectionConfig
  DERIVES Default, and there are 19 struct literals repo-wide (7 in
  nova_assets/sections.rs catalog to wire with real sounds; the rest get
  `..default()` where not already present).
- Asteroids: bundle puts config components on the PARENT; Collider+Health live
  on a CHILD node (asteroid.rs:138 comment), and IntegrityDestroyMarker lands
  on the node - observers must WALK UP from the target to find the sounds
  component (bounded ChildOf walk, like hum_source_root). 10+
  `Asteroid(AsteroidConfig {` builder sites in nova_assets get uniform
  `impact_sound`/`destroy_sound` lines inserted after the opener.
- Torpedo detonation: projectile gains IntegrityDestroyMarker on detonation;
  snapshot `TorpedoSectionConfig::detonation_sound` onto the projectile as the
  same component (destroy slot only).

### Steps

- [x] `ImpactDestroySounds { impact, destroy }` (pub, nova_gameplay
      base_section.rs; Options of AssetRef<AudioSource>) + BaseSectionConfig
      gains `impact_sound`/`destroy_sound` (serde default, skip-if-none);
      `base_section()` snapshots. AsteroidConfig gains the same two fields;
      the asteroid bundle snapshots onto the parent (observer walk finds it
      from the node). TorpedoSectionConfig gains `detonation_sound`; projectile
      spawn snapshots destroy-only.
- [x] Observers: `on_damage_play_impact` + `on_destroyed_play_explosion` drop
      the bank; resolve via a bounded ancestor walk from the target entity to
      the nearest `ImpactDestroySounds`; authored-or-silent. Throttle/cell/
      attenuation untouched. WorldSfx 4 -> 2 (ThrusterLoop, SalvagePickup).
- [x] gen_content: SectionMeshRefs += impact/destroy/detonation refs
      (self://sounds/impact.wav, explosion.wav); wire the 6 catalog sections'
      base configs + all asteroid builder sites + the torpedo bay; regenerate.
- [x] Fix the other BaseSectionConfig literals (`..default()`); compile-driven.
- [x] Tests: authored section plays impact/destroy handles; unauthored silent
      (delivery-guarded); the asteroid NODE shape (component on parent, marker
      on child) resolves via the walk; torpedo detonation plays. Keep the
      propagated-hit single-impact test green (rig entities need the component
      now).
- [x] Docs: wiki section guide (base fields) + asteroid/scenario page if any;
      sounds README tables (move impact/explosion to authored, bank -> 2);
      CHANGELOG bullet extension; spike fix record. Prose-grep "bank" across
      assets/ web/ docs/ after the flip (101633 retro).
- [x] Verify: fmt; workspace all-targets check; nova_gameplay lib +
      nova_scenario (serde) + gates; read outputs not pipe exits.
