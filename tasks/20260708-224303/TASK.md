# Integration test for SFX event->sound wiring

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.7.0,audio,test,testing,wontdo

## Outcome (CLOSED wontdo 2026-07-17)

Superseded by this version's audio refactor (spike 20260717-101524, the
UiSfx/WorldSfx split then the shrink-to-deletion of WorldSfx). The refactor's
own review-gated tests already deliver exactly what F2 asked for - device-free,
App-level integration tests of the event -> observer -> `PlaySfx` wiring for
every seam this task enumerated - so there is nothing left to build.

Ground truth: `crates/nova_gameplay/src/audio.rs` tests module (`#[cfg(test)]`,
lines 905-1912). The task's premise is also obsolete: there is no single
`SoundBank<NovaSfx>`; world sounds are now per-target/per-section authored
`AssetRef<AudioSource>` fields resolved authored-or-silent by the cue observers,
and only the `UiSfx` engine-chrome bank remains.

Seam-by-seam coverage that already exists (all build a `MinimalPlugins +
AssetPlugin` App, `init_asset::<AudioSource>`, add the real observer, and assert
on an observed `PlaySfx` - no audio device):

- `IntegrityDestroyMarker` -> Explosion (`on_destroyed_play_explosion`):
  `impact_and_destroy_play_the_targets_authored_sounds_or_stay_silent`
  (audio.rs:1241) and `the_sound_lookup_walks_up_to_the_asteroid_parent` (:1320).
- `HealthApplyDamage` -> Impact (`on_damage_play_impact`):
  `a_propagated_hit_on_a_straddling_hierarchy_plays_one_impact` (:1043, also the
  system-level throttle-collapse check F2 called optional) and the impact half of
  `impact_and_destroy...` (:1241).
- `TurretBulletProjectileMarker` (+ `Transform` + `TurretSectionPartOf`) ->
  TurretFire (`on_turret_fire_play_sfx`):
  `a_turret_with_a_declared_fire_sound_plays_that_handle` (:1135) and
  `a_turret_without_a_declared_fire_sound_fires_silently` (:1162).
- `TorpedoProjectileMarker` (+ `Transform` + `TorpedoSectionSpawnerEntity`) ->
  TorpedoLaunch (`on_torpedo_launch_play_sfx`):
  `a_torpedo_bay_with_a_declared_launch_sound_plays_it_and_silent_without`
  (:1188).

Graceful degradation (F2's third step) is covered by the "or stay silent" /
"fires silently" halves above: with no authored sound (the refactor's equivalent
of "no `SoundBank` resource") the same events run without panic and produce no
`PlaySfx`/audio entity. The throttle-at-system optional step is covered by the
one-impact propagation test plus `throttle_is_independent_per_key` (:923).

Residual sliver considered and declined: the existing tests register each
observer directly rather than through `NovaAudioPlugin::build` (audio.rs:303-324),
so a dropped `add_observer` line in the plugin would not be caught. A
plugin-level smoke test would close that, but it would pull in `SfxPlugin`
(bcs), `PauseStates`, settings and the `UiSfx` bank load for marginal gain over
the six seam tests above - not worth the heavier, more fragile rig. If plugin
wiring regressions ever bite, file a focused task then.


## Goal

Finding F2 from the PR #53 review (`tasks/20260708-162011/REVIEW.md`):
every audio test today is a pure-function unit test (throttle, engine_volume,
distance_attenuation, area_cell). Nothing exercises the event->sound wiring end
to end, so a future refactor of the observers or the plugin could silently break
"a gameplay event plays a sound". Add a headless integration test that asserts
the wiring, without needing an audio device.

## Steps

- [x] Build a minimal `App` in a test: `MinimalPlugins` (+ `AssetPlugin` /
      `init_asset::<AudioSource>` as needed) and `NovaAudioPlugin`, with a
      `SoundBank<NovaSfx>` resource inserted (use dummy/`AssetServer`-loaded
      handles - the handles need not resolve to real files for the observer to
      fire and spawn an audio entity).
- [x] For each one-shot seam, trigger the event and assert a `PlaySfx` fired (or
      an `AudioPlayer` entity was spawned by `SfxPlugin`):
      - add `IntegrityDestroyMarker` to an entity with a `GlobalTransform` ->
        Explosion;
      - trigger `HealthApplyDamage` on an entity with a `GlobalTransform` ->
        Impact;
      - add `TurretBulletProjectileMarker` (+ `Transform` + `TurretSectionPartOf`)
        -> TurretFire; add `TorpedoProjectileMarker` (+ `Transform`) ->
        TorpedoLaunch.
      `SfxPlugin` spawns an `AudioPlayer` on `PlaySfx` even with no audio device,
      so counting spawned audio entities (or observing `PlaySfx`) is a
      device-free assertion.
- [x] Cover the graceful-degradation path too: with no `SoundBank` resource, the
      same events must NOT panic and must produce no audio entity.
- [x] Optionally assert the throttle at the system level (two co-located
      destroys in one run collapse to one Explosion; two distinct turrets each
      sound) to complement the existing pure `allow` unit test.
- [x] Verify: cargo test --workspace, fmt, clippy --all-targets. Shared
      CARGO_TARGET_DIR.

## Notes

- Source: PR #53 review F2. Depends on: 20260708-162011 (CLOSED).
- The point is to guard the *wiring* (event -> observer -> PlaySfx/audio entity),
  which the current pure-function tests do not touch. Keep it device-free so it
  runs in CI/headless like the rest of the suite.
- Check how bcs tests `SfxPlugin` (`~/personal/bevy-common-systems/src/audio/`)
  for the minimal-app + `PlaySfx`/`AudioPlayer` assertion pattern to reuse.

## v0.7.0 (20260716)

Carried over: the one v0.6.0 leftover, retagged v0.7.0 unchanged (p20).
Plan: docs/plans/20260716-v0.7.0-plan.md, strand 2.
