# Integration test for SFX event->sound wiring

- STATUS: OPEN
- PRIORITY: 20
- TAGS: v0.6.0,audio,test,testing

## Goal

Finding F2 from the PR #53 review (`tasks/20260708-162011/REVIEW.md`):
every audio test today is a pure-function unit test (throttle, engine_volume,
distance_attenuation, area_cell). Nothing exercises the event->sound wiring end
to end, so a future refactor of the observers or the plugin could silently break
"a gameplay event plays a sound". Add a headless integration test that asserts
the wiring, without needing an audio device.

## Steps

- [ ] Build a minimal `App` in a test: `MinimalPlugins` (+ `AssetPlugin` /
      `init_asset::<AudioSource>` as needed) and `NovaAudioPlugin`, with a
      `SoundBank<NovaSfx>` resource inserted (use dummy/`AssetServer`-loaded
      handles - the handles need not resolve to real files for the observer to
      fire and spawn an audio entity).
- [ ] For each one-shot seam, trigger the event and assert a `PlaySfx` fired (or
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
- [ ] Cover the graceful-degradation path too: with no `SoundBank` resource, the
      same events must NOT panic and must produce no audio entity.
- [ ] Optionally assert the throttle at the system level (two co-located
      destroys in one run collapse to one Explosion; two distinct turrets each
      sound) to complement the existing pure `allow` unit test.
- [ ] Verify: cargo test --workspace, fmt, clippy --all-targets. Shared
      CARGO_TARGET_DIR.

## Notes

- Source: PR #53 review F2. Depends on: 20260708-162011 (CLOSED).
- The point is to guard the *wiring* (event -> observer -> PlaySfx/audio entity),
  which the current pure-function tests do not touch. Keep it device-free so it
  runs in CI/headless like the rest of the suite.
- Check how bcs tests `SfxPlugin` (`~/personal/bevy-common-systems/src/audio/`)
  for the minimal-app + `PlaySfx`/`AudioPlayer` assertion pattern to reuse.
