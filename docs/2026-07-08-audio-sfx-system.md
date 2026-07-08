# Audio / SFX system

Date: 2026-07-08
Task: 20260708-162011 (v0.4.0, audio)

## What shipped

The game's first audio. Five placeholder sound effects play on the core combat
moments, wired off events that already existed:

| Cue | Seam | Notes |
| --- | --- | --- |
| Explosion | `On<Add, IntegrityDestroyMarker>` | section/asteroid destruction and torpedo detonation all funnel through this marker |
| Impact | `On<HealthApplyDamage>` | throttled (a blast damages many colliders in one frame) |
| Turret fire | `On<Add, TurretBulletProjectileMarker>` | throttled hard: the PDC fires ~100/s |
| Torpedo launch | `On<Add, TorpedoProjectileMarker>` | |
| Thruster loop | continuous | one looping entity, volume tracks summed thruster input |

The placeholders are tiny generated WAVs under `assets/sounds/`, produced by the
stdlib-only `scripts/gen-placeholder-sounds.py` (noise bursts, pitch sweeps, and
one steady loopable hum), committed so the game is audible out of the box. Real
audio drops in by overwriting the files at the same paths;
`assets/sounds/README.md` is the canonical list.

## Design decisions

- **Reuse over reinvention.** The playback machinery is entirely
  `bevy_common_systems`: `SfxPlugin` (self-despawning one-shots via `PlaySfx`),
  `SoundBank<K>` (a keyed handle registry), and `SfxMasterVolume`. All already
  shipped at Nova's pinned bcs rev. Nova adds only the game-specific mapping in
  `crates/nova_gameplay/src/audio.rs` (`NovaSfx`, `NovaAudioPlugin`, the
  observers, the thruster loop). This keeps the generic half promotable and the
  Nova-specific half local - the same tier boundary the crate policy already
  draws.

- **Observers on spawn markers, not edits to the weapon systems.** Turret and
  torpedo projectiles each get a distinct marker component at spawn, so
  `On<Add, Marker>` gives a decoupled fire/launch seam without threading a
  `SoundBank` param through `shoot_spawn_projectile`. This mirrors how the
  destruction cue rides the existing `IntegrityDestroyMarker`.

- **`SoundBank::load`, not the `GameAssets` collection.** The bank is populated
  in a `register_sounds` system on `OnEnter(GameAssetsStates::Processing)` via
  `SoundBank::load(&assets, NOVA_SFX_FILES)`. The bcs registry has no public
  "build from existing handles" constructor, so routing the WAVs through the
  gated `bevy_asset_loader` collection was not possible without a cross-repo
  change; loading them here still happens well before the first gameplay sound.
  `NOVA_SFX_FILES` is the single source of truth for the key->file map, shared
  between `nova_gameplay` (the keys) and `nova_assets` (the load).

- **Throttling.** Turret fire and impact can each fire many times per frame
  (100/s PDC; a blast hitting every collider of a ship). Un-throttled that is a
  wall of overlapping audio entities. A tiny per-cue min-interval collapses them
  to a legible rate; the timestamps default to `NEG_INFINITY` so the first event
  always fires.

- **Distance attenuation (feel pass, task 20260708-213155).** The four
  positional one-shots are scaled by how far the event is from the listener (the
  gameplay `Camera3d`), so a distant explosion is quieter than a point-blank one.
  `distance_attenuation` is a linear rolloff: full within `SFX_NEAR_DISTANCE`
  (20 units), silent beyond `SFX_FAR_DISTANCE` (320), linear between - both are
  tunable-by-ear constants. `play_positional` applies it and skips spawning an
  audio entity below an audibility threshold. Source positions come from the
  destroyed/damaged entity's `GlobalTransform` (valid, it has existed for
  frames) and, for the two projectile cues, from the projectile's local
  `Transform` - both projectiles spawn as ROOT entities whose `GlobalTransform`
  is still identity on the spawn frame, so the local transform is the correct
  world position. Base volumes were lowered at the same time. The thruster hum is
  the player's own ship and is deliberately not attenuated. This is volume-only,
  not true spatialization: **stereo panning** would need bevy spatial audio
  (`SpatialListener` on the camera + `spatial: true` on the audio entities, which
  means spawning our own spatial players instead of bcs `play_sfx`) and is left
  as a future step.

- **`wav` decoder as a normal feature.** Bevy's default audio decoder is vorbis;
  WAV needs the `wav` feature. It is set on `bevy` in
  `crates/nova_gameplay/Cargo.toml` (a normal, not dev-only, dependency feature,
  because the shipped binary decodes the placeholders). Cargo unifies it across
  the workspace. Vorbis stays default-on, so `.ogg` swap-ins need only a
  path-extension change.

## Thruster loop tradeoff

The engine hum sums `ThrusterSectionInput` across every active thruster in the
world - a single "the ship is burning" hum. Per-ship attribution (only the
player's thrusters) would need to relate each thruster to the player root and is
deferred until there is more than one audible ship. The volume is exponentially
smoothed so throttle changes fade rather than click, and the `AudioSink` (which
appears a frame or two after the entity spawns) is polled defensively.

## Difficulties

- `AudioSink::set_volume` takes `&mut self` in Bevy 0.19, so the sink query had
  to be `Query<&mut AudioSink>` / `single_mut`. The compiler caught it.
- The throttle unit test caught a real off-by-semantics bug: seeding the
  last-played timestamp at `0.0` swallowed the first event at `t == 0`. Fixed by
  defaulting to `NEG_INFINITY`.

## Verification

`cargo build`, `cargo clippy --all-targets` (clean), `cargo fmt --check`,
`cargo test --workspace` (audio unit tests plus the existing
`harnessed_examples_reach_playing_without_panic` integration test, all green). A
headless `BCS_AUTOPILOT=1` run of `10_gameplay` reached Playing with the scenario
loaded, `SfxPlugin` built, no sound asset-load errors and no panic. Audible
confirmation needs a graphical session with an audio device (same caveat as the
bcs audio module).

## Follow-ups (not in scope)

- A `SfxMasterVolume` slider in the UI (the knob exists; nothing drives it yet).
- Per-ship thruster attribution once enemies are audible.
- Spatial/positional audio (currently all cues are non-positional).
- Torpedo bay particle SFX coupling (task 133024) and lock-on cue audio
  (task 165703) build on this layer.
