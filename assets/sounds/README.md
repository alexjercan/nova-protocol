# Sound effects

Nova Protocol plays one sound per core gameplay moment. The files committed here
are **tiny generated placeholders** (short noise bursts, pitch sweeps and a
steady hum) produced by `scripts/gen-placeholder-sounds.py` so the game is
audible and wired end to end out of the box. They are not the final sound
design.

The audio layer itself is the reusable `SfxPlugin` / `SoundBank` from
`bevy-common-systems`; Nova only owns the mapping from gameplay events to these
files (see `crates/nova_gameplay/src/audio.rs`).

## Dropping in real audio

Replace each file below with a real sound **at the same path and filename**. No
code changes are needed: the loader (`crates/nova_assets/src/lib.rs`) loads
these fixed paths and the audio module plays whatever handle it is given.

- Formats: WAV works today (the `bevy` dependency enables the `wav` decoder in
  `crates/nova_gameplay/Cargo.toml`). OGG Vorbis also works, since vorbis is on
  by default; to use `.ogg` files, change the extensions in the `#[asset(path =
  "sounds/...")]` fields of `GameAssets`.
- Suggested: 44.1 kHz, normalized but not clipping. Keep the one-shots short;
  `thruster_loop.wav` is the only looping asset and should be seamless (its
  start and end must meet without a click).
- To regenerate the placeholders (e.g. after deleting them):
  `python3 scripts/gen-placeholder-sounds.py` from the repo root.

## Required files

| File | Event | Character / length |
| --- | --- | --- |
| `turret_fire.wav` | A PDC/turret round is fired (`shoot_spawn_projectile`) | dry gunshot pop, ~0.07 s, played quietly (fires ~100/s) |
| `torpedo_launch.wav` | A torpedo leaves its bay (`shoot_spawn_projectile`) | airy rising whoosh, ~0.3 s |
| `explosion.wav` | A section/asteroid is destroyed or a torpedo detonates (`IntegrityDestroyMarker`) | noisy burst, fast decay, ~0.45 s |
| `impact.wav` | Damage is applied to a target (`HealthApplyDamage`) | short low thud, ~0.1 s, played quietly (fires per hit) |
| `thruster_loop.wav` | The engine hum, played continuously; volume tracks throttle | steady low drone, loops seamlessly, ~1 s |

## Web (wasm) builds

`index.html` already ships this directory into the web build via
`<link data-trunk rel="copy-dir" href="assets"/>`, so no per-file directive is
needed. Browser audio needs a user gesture before it will play; the existing
`build/web/sound.js` shim resumes the audio context on the first interaction.
