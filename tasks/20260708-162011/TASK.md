# Audio/SFX system (thrust, weapons, explosions, impacts)

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.4.0,audio

Spike: tasks/20260708-161726/SPIKE.md (roadmap)

## Goal

Give the game its first audio: play placeholder one-shot SFX on the core combat
feedback moments (turret fire, torpedo launch, explosions/destruction, impact
hits) plus a continuous thruster engine loop, by reusing the
`bevy_common_systems` audio layer. Ship committed, generated placeholder WAVs so
it runs out of the box; real audio drops in by overwriting files at the same
paths. Done = game boots with sound firing off real gameplay events, no
asset-load errors, wasm still serves the sounds, checks green.

## Steps (as executed - deviations from the original plan noted inline)

- [x] Enable the wav decoder: added `features = ["wav"]` to the `bevy`
      dependency. DEVIATION: placed in `crates/nova_gameplay/Cargo.toml`, NOT the
      root `Cargo.toml` - root `bevy` is a dev-dependency only, so the shipped
      binary's bevy comes transitively through the crates; the decoder must be on
      a normal dependency. Cargo unifies it across the workspace. Vorbis stays
      default-on for `.ogg` swap-ins.
- [x] Added `scripts/gen-placeholder-sounds.py` (Python stdlib
      `wave`/`struct`/`math`/`random`, deterministic - RNG seeded per filename).
      Emits `turret_fire`/`torpedo_launch`/`explosion`/`impact`/`thruster_loop`
      WAVs to `assets/sounds/` (noise bursts, pitch sweeps, and a whole-cycle
      loopable hum). Ran it; committed the WAVs. Verified byte-identical on rerun.
- [x] Added `assets/sounds/README.md`: file -> event -> character table, swap-in
      instructions, and the regenerate command.
- [x] New module `crates/nova_gameplay/src/audio.rs`: `NovaSfx` key enum, a
      `NovaAudioPlugin` adding bcs `SfxPlugin` + the observers/systems, plus
      `NOVA_SFX_FILES` (the key->filename map). Exported `NovaSfx`,
      `NovaAudioPlugin`, `NOVA_SFX_FILES` from the prelude; `mod audio;` in
      `lib.rs`; `NovaAudioPlugin` added in `plugin.rs`.
- [x] Load the sound bank via `register_sounds` on
      `OnEnter(GameAssetsStates::Processing)` in `crates/nova_assets/src/lib.rs`.
      DEVIATION: used `SoundBank::load(&assets, NOVA_SFX_FILES)` rather than
      `GameAssets` collection fields - the bcs `SoundBank` has no public
      build-from-handles constructor, so it cannot be built from the gated
      collection without a cross-repo change; loading here still runs well before
      the first gameplay sound.
- [x] Wired one-shot SFX off existing seams via observers (in `audio.rs`).
      DEVIATION: turret/torpedo cues use `On<Add, TurretBulletProjectileMarker>` /
      `On<Add, TorpedoProjectileMarker>` observers rather than editing
      `shoot_spawn_projectile` - both projectiles already carry a distinct spawn
      marker, so the observer seam is decoupled and needs no change to the weapon
      systems. Explosion = `On<Add, IntegrityDestroyMarker>`, impact =
      `On<HealthApplyDamage>` (throttled). Turret fire and impact are throttled by
      a small per-cue min-interval (timestamps default to NEG_INFINITY so the
      first event fires).
- [x] Thruster engine loop: one looping `ThrusterLoopSfx` entity spawned once the
      bank exists (`ensure_thruster_loop`), volume driven by summed
      `ThrusterSectionInput` and exponentially smoothed (`update_thruster_loop_volume`).
      NOTE: aggregates all active thrusters (single "ship is burning" hum);
      per-ship attribution deferred (documented).
- [x] Verified wasm path (no code change): `index.html` already
      `copy-dir`s `assets/`, and `build/web/sound.js` handles the audio-unlock
      gesture. Noted in the doc.
- [x] Verified: `cargo build`, `cargo clippy --all-targets` (clean),
      `cargo fmt --check`, `cargo test --workspace` (all green, incl. the audio
      unit tests + `harnessed_examples_reach_playing_without_panic`). Headless
      `BCS_AUTOPILOT=1 10_gameplay --features debug`: reached Playing, scenario
      loaded, `SfxPlugin` built, no sound asset-load error, no panic.
- [x] Documented in `tasks/20260708-162011/NOTES.md` (seams, the
      SfxPlugin-generic / nova-owns-the-map boundary, thruster-loop tradeoff,
      difficulties, swap-in path).

## Notes

- Reuse, do not reinvent: bcs at nova's pinned rev `34b3f0a` already ships
  `SfxPlugin`, `PlaySfx`, `SfxCommandsExt` (`play_sfx`/`play_sfx_volume`),
  `SfxMasterVolume`, and the `SoundBank<K>` registry + `sounds_loaded` gate, all
  in `bevy_common_systems::prelude`. No dependency bump needed.
- Seams (verified in code): destruction = `On<Add, IntegrityDestroyMarker>`
  observers in `crates/nova_gameplay/src/integrity/explode.rs`; damage/impact =
  bcs `HealthApplyDamage` entity-event (`~/personal/bevy-common-systems/
  src/health/mod.rs`), applied by bcs `on_impact_collision_deal_damage` /
  `on_blast_collision_deal_damage`; turret + torpedo fire = the respective
  `shoot_spawn_projectile` systems; thrust = `ThrusterSectionInput` on
  `sections/thruster_section.rs`.
- App assembly: bcs helper plugins are added in
  `crates/nova_gameplay/src/plugin.rs` (StatusBarPlugin, HealthPlugin, ...);
  `SfxPlugin`/`NovaAudioPlugin` belong there. Assets load via `bevy_asset_loader`
  `GameAssets` in `crates/nova_assets/src/lib.rs`, Processing state runs
  `register_sections`/`register_scenario`.
- Boundary (promotion): the generic "play a sound" mechanism stays in bcs
  (`SfxPlugin`, `SoundBank`); only nova's event->sound mapping (`NovaSfx`,
  `NovaAudioPlugin`, the observers) is game-specific and lives in nova.
- Assumptions to confirm at build time: (a) wav feature is a normal dep feature,
  not dev-only, since the shipped game plays wavs; (b) `SoundBank<NovaSfx>` built
  from the gated `GameAssets` handles (alternative: `SoundBank::load` at startup,
  ungated - rejected to avoid a first-play race); (c) turret fire needs a
  volume/throttle tune because the PDC fires ~100/s.
- Verify with `cargo test --workspace` (not bare `cargo test`) - root+members
  workspace, unit tests live in members (recent-retro lesson). For headless
  example runs, build cold first then time only the run; check
  `${PIPESTATUS[0]}` when reading a piped command's status.

## Outcome

Shipped the game's first audio: five placeholder SFX (explosion, impact, turret
fire, torpedo launch, thruster loop) wired off existing gameplay seams, playing
through the reusable bcs `SfxPlugin`/`SoundBank`. Placeholder WAVs are generated
by `scripts/gen-placeholder-sounds.py` and committed so it runs out of the box;
real audio drops in by overwriting files. Full write-up in
`tasks/20260708-162011/NOTES.md`.

### What changed and why

- `NovaAudioPlugin` (`crates/nova_gameplay/src/audio.rs`) owns only Nova's
  event->sound mapping; the generic playback stays in bevy_common_systems. This
  respects the crate tier boundary and keeps the promotable half promotable.
- Chose decoupled `On<Add, Marker>` observers over editing the weapon systems,
  and `SoundBank::load` over the gated `GameAssets` collection (the bank has no
  from-handles constructor). Both trade a small purity cost (ungated load) for a
  much smaller, decoupled diff - reasonable for placeholder audio.
- `wav` decoder enabled as a normal (not dev) bevy feature in nova_gameplay,
  since the shipped binary decodes the placeholders.

### Difficulties

- `AudioSink::set_volume` is `&mut self` in Bevy 0.19 - needed `&mut AudioSink` /
  `single_mut`; compiler caught it on first build.
- A unit test caught a real bug: seeding the throttle timestamp at 0.0 swallowed
  the first event at t=0. Fixed with a `NEG_INFINITY` default.

### Self-reflection

- The shared `CARGO_TARGET_DIR` trick (pointing the sprout worktree at the main
  repo's warm target) turned a would-be multi-minute cold bevy build into ~27s.
  Worth doing by default for any nova worktree build.
- Writing the two pure helpers (`throttle`, `engine_volume`) as free functions
  paid off: they carry the only real logic and are unit-testable without an
  audio device, which is exactly the part a headless run cannot verify.
- Could have gone better: I planned the SoundBank to ride the gated GameAssets
  collection before checking that the bcs registry exposes no from-handles
  constructor. Reading the constructor surface first would have saved a
  plan-vs-reality correction. Lesson: when a plan step depends on a library
  type's API shape, confirm that shape while planning, not while implementing.
