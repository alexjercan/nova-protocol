# Sound effects

Nova Protocol plays a sound for each core gameplay and UI moment. The files
committed here are **tiny generated placeholders** (short noise bursts, pitch
sweeps and a steady hum) produced by `scripts/gen-placeholder-sounds.py` so the
game is audible and wired end to end out of the box. They are not the final
sound design.

The audio layer itself is the reusable `SfxPlugin` / `SoundBank` from
`bevy-common-systems`; Nova only owns the mapping from gameplay events to these
files (see `crates/nova_gameplay/src/audio.rs`).

## Dropping in real audio

Replace each file below with a real sound **at the same path and filename**. No
code changes are needed: the loader (`crates/nova_assets/src/lib.rs`) loads
these fixed paths and the audio module plays whatever handle it is given.

- Formats: WAV works out of the box (the `bevy` dependency enables the `wav`
  decoder in `crates/nova_gameplay/Cargo.toml`). Sounds are loaded by
  `register_sounds` in `crates/nova_assets/src/lib.rs` via
  `SoundBank::load(&assets, NOVA_SFX_FILES)`, which applies the fixed
  `sounds/<name>.wav` convention. OGG Vorbis also decodes (vorbis is on by
  default), but because that convention hard-codes the `.wav` extension, using
  `.ogg` means switching `register_sounds` to `SoundBank::load_paths(&assets,
  [(NovaSfx::TurretFire, "sounds/turret_fire.ogg"), ...])` with full paths.
- Suggested: 44.1 kHz, normalized but not clipping. Keep the one-shots short;
  `thruster_loop.wav` is the only looping asset and should be seamless (its
  start and end must meet without a click).
- To regenerate the placeholders (e.g. after deleting them):
  `python3 scripts/gen-placeholder-sounds.py` from the repo root.

## Required files

The full set is the single source of truth `NOVA_SFX_FILES` in
`crates/nova_gameplay/src/audio.rs` (one row per `NovaSfx` variant); the
`every_nova_sfx_key_has_a_file` test guards that each key has a file here.
Combat/world cues are **positional** (distance-attenuated from the listener
camera); UI/feedback cues are **non-positional**.

### Combat / world (positional)

| File | Event | Character / length |
| --- | --- | --- |
| `turret_fire.wav` | A PDC/turret round is fired (`shoot_spawn_projectile`) | dry gunshot pop, ~0.07 s, played quietly (fires ~100/s) |
| `torpedo_launch.wav` | A torpedo leaves its bay (`shoot_spawn_projectile`) | airy rising whoosh, ~0.3 s |
| `explosion.wav` | A section/asteroid is destroyed or a torpedo detonates (`IntegrityDestroyMarker`) | noisy burst, fast decay, ~0.45 s |
| `impact.wav` | Damage is applied to a target (`HealthApplyDamage`) | short low thud, ~0.1 s, played quietly (fires per hit) |
| `thruster_loop.wav` | The engine hum, played continuously; volume tracks throttle | steady low drone, loops seamlessly, ~1 s |

### UI / feedback (non-positional)

| File | Event | Character / length |
| --- | --- | --- |
| `objective_new.wav` | A new objective is posted to the panel | short neutral blip, ~0.12 s |
| `objective_complete.wav` | An objective is completed | rising fifth (success), ~0.22 s |
| `lock_on.wav` | A radar gesture acquires its first target (once per gesture) | quick rising chirp, ~0.09 s |
| `lock_off.wav` | A tap-clear releases a lock | falling mirror of `lock_on`, ~0.09 s |
| `safety_on.wav` | The weapons safety re-engages (hot -> cold) | dull low click, ~0.06 s |
| `radar_deny.wav` | A radar hold is denied (computer grants no Lock) | low flat buzz, ~0.16 s |
| `salvage_pickup.wav` | A salvage crate is picked up | light rising "ding", quieter than the objective chime, ~0.10 s |
| `menu_select.wav` | A menu button is pressed (New Game / Sandbox / Settings / Exit, pause, mods) | crisp rising click, ~0.06 s |
| `ui_toggle.wav` | The pause overlay toggles open/close (ESC) | soft two-state blip, ~0.05 s |
| `dry_fire.wav` | A turret pulls its trigger on an empty magazine | dull descending click, ~0.06 s |
| `radar_retarget.wav` | A held radar gesture re-designates to a new target | very short quiet tick (subtler than `lock_on`), ~0.045 s |

## Web (wasm) builds

`index.html` already ships this directory into the web build via
`<link data-trunk rel="copy-dir" href="assets"/>`, so no per-file directive is
needed. Browser audio needs a user gesture before it will play; the existing
`build/web/sound.js` shim resumes the audio context on the first interaction.
