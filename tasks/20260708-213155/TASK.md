# Distance attenuation + quieter SFX (audio feel pass)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0,audio,polish

## Goal

User feedback on the just-shipped audio (20260708-162011): the cues are
non-positional (bcs `SfxPlugin` plays each at a fixed volume regardless of where
it happens), so a distant explosion is as loud as one next to you. Add
distance-based volume attenuation so far-away positional cues fade out, and lower
the base volumes overall. Keep it cinematic-but-legible; full stereo panning is a
separate, larger step (noted below), not this task.

## Steps

- [x] Lower the base volume constants in `crates/nova_gameplay/src/audio.rs`
      (turret/impact/explosion/torpedo/engine) a notch so nothing is loud at
      point-blank.
- [x] Add a pure `distance_attenuation(distance) -> f32` (linear rolloff: full
      within `SFX_NEAR_DISTANCE`, zero beyond `SFX_FAR_DISTANCE`, linear between)
      plus the two tunable distance constants, in `audio.rs`.
- [x] Add a `play_positional` helper that multiplies a cue's base volume by the
      attenuation from the listener to the source and skips playing when the
      result is below an audibility threshold (no inaudible audio entities).
- [x] Wire the listener as the gameplay camera (`Query<&GlobalTransform,
      With<Camera3d>>`, first match; fall back to full volume if absent).
- [x] Attenuate the four positional one-shots by source position:
      - explosion (`IntegrityDestroyMarker`): source = entity `GlobalTransform`
        (a section/asteroid that has existed for frames -> valid world transform).
      - impact (`HealthApplyDamage`): source = `damage.entity` `GlobalTransform`.
      - turret fire / torpedo launch (`On<Add, *ProjectileMarker>`): source =
        the projectile's `Transform.translation` - both spawn as ROOT entities
        with a world-space transform, so their `GlobalTransform` is still identity
        on the spawn frame; use `Transform`.
      Leave the thruster loop un-attenuated (it is the player's own ship).
- [x] Unit-test `distance_attenuation` (near -> 1, far -> 0, midpoint ~0.5,
      clamped below/above).
- [x] Verify: `cargo fmt --check`, `cargo clippy --all-targets`,
      `cargo test --workspace`, and a headless `BCS_AUTOPILOT=1 10_gameplay`
      autopilot run (reaches Playing, no panic, no sound asset errors). Use the
      shared `CARGO_TARGET_DIR` (see docs/development.md) for the worktree build.
- [x] Update `docs/retros/20260708-audio-sfx-system.md`: note the attenuation model,
      the tunable NEAR/FAR constants, the listener = camera choice, and that true
      stereo panning (bevy spatial audio: `SpatialListener` + `spatial: true`)
      remains a future step.

## Notes

- Depends on: 20260708-162011 (CLOSED) - this refines that module.
- Seams/APIs (verified): observer entity via `add.entity`; damage target via
  `damage.entity`; both projectiles are root entities with world-space
  `Transform` (turret_section.rs / torpedo_section/mod.rs `shoot_spawn_projectile`);
  listener camera is the bcs `ChaseCamera` (a `Camera3d`).
- Why distance-volume, not bevy spatial audio: bcs `SfxPlugin` plays
  non-spatial `AudioPlayer`s, so real spatialization would mean spawning our own
  spatial audio entities + a `SpatialListener` on the camera and tuning
  `spatial_scale` - bigger and harder to verify headlessly. Distance-volume
  directly satisfies "far away should be quieter", is pure/testable, and keeps
  the bcs reuse. Panning is the documented next step if wanted.

## Outcome

Added distance-based volume attenuation to the four positional one-shots and
lowered the base volumes, in response to user feedback that the cues sounded the
same from anywhere. `distance_attenuation` (pure, unit-tested) is a linear
rolloff over `SFX_NEAR_DISTANCE`..`SFX_FAR_DISTANCE`; `play_positional` applies
it relative to the gameplay camera and skips inaudible cues. Source positions:
`GlobalTransform` for the already-existing destroyed/damaged entities, local
`Transform` for the two freshly-spawned root projectiles (whose GlobalTransform
is still identity on the spawn frame). The thruster hum stays un-attenuated.
Verified: fmt, clippy --all-targets (clean), cargo test --workspace (5 audio unit
tests incl. the new rolloff test), headless 10_gameplay autopilot reached Playing
with no panic/asset errors. Full model documented in
docs/retros/20260708-audio-sfx-system.md.

Reviewed by self (focused refinement of an already-reviewed module); no findings.
Deliberately left as a future step: true stereo panning via bevy spatial audio
(SpatialListener + spatial playback), which the doc records. NEAR/FAR distances
and the base volumes are tune-by-ear knobs and may want adjustment once heard on
real hardware / real content.
