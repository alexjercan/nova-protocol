# Sound effects

Nova Protocol plays a sound for each core gameplay and UI moment. The files
committed here are **tiny generated placeholders** (short noise bursts, pitch
sweeps and a steady hum) produced by `scripts/gen-placeholder-sounds.py` so the
game is audible and wired end to end out of the box. They are not the final
sound design.

The audio layer itself is the reusable `SfxPlugin` / `SoundBank` from
`bevy-common-systems`; Nova only owns the mapping from gameplay events to these
files (see `crates/nova_gameplay/src/audio.rs`).

These files live UNDER `assets/base/` because the base game is just a mod
(task 20260717-002228) and these are its WORLD/GAMEPLAY cues - the sounds of
things that exist in the game world, which mods can reship or reference. They
are declared in the base bundle's `resources` list
(`assets/base/base.bundle.ron`), so a mod can reference any of them with
`dep://base/sounds/<name>.wav` - the same scheme the base uses with `self://`.
The UI/interface cues (menu clicks, objective chimes) are engine chrome like
`assets/icons/` and live at the asset ROOT in `assets/sounds/` instead - see
the ownership split in spike `tasks/20260717-101524/SPIKE.md`.

## Section-authored sounds

A section can declare a sound as an authorable `AssetRef<AudioSource>` content
field, exactly like it declares a render mesh, and ship + reference it through
the `self://` / `dep://base` / `dep://<id>` pipeline. The weapon-section family owns
its sounds this way: the turret's `fire_sound` + `dry_fire_sound` and the
torpedo bay's `launch_sound` (base sections author `self://sounds/...` for
each, so the shipped game sounds unchanged - but a mod section can ship and
name its own). These cues are AUTHORED-OR-SILENT: a section that declares no
sound plays none (their old `WorldSfx` bank keys are deleted). The remaining
world cues are migrating onto their owning configs family by family (spike
20260717-101524); until then they play from the transitional `WorldSfx` bank.

## Dropping in real audio

Replace each file below with a real sound **at the same path and filename**. No
code changes are needed: the loader (`crates/nova_assets/src/lib.rs`) loads
these fixed paths and the audio module plays whatever handle it is given.

- Formats: WAV works out of the box (the `bevy` dependency enables the `wav`
  decoder in `crates/nova_gameplay/Cargo.toml`). These sounds are loaded by
  `register_sounds` in `crates/nova_assets/src/lib.rs` into the transitional
  `WorldSfx` bank via `SoundBank::load_paths(&assets, ...)` with full
  `base/sounds/<name>.wav` paths; each cue family is migrating onto its owning
  section/object config as an authorable `AssetRef<AudioSource>` (spike
  20260717-101524), after which the file here is just the base mod's authored
  default. OGG Vorbis also decodes (vorbis is on by default); to use `.ogg`,
  change the extension in the paths `register_sounds` builds (and in base
  content refs).
- Suggested: 44.1 kHz, normalized but not clipping. Keep the one-shots short;
  `thruster_loop.wav` is the only looping asset and should be seamless (its
  start and end must meet without a click).
- To regenerate the placeholders (e.g. after deleting them):
  `python3 scripts/gen-placeholder-sounds.py` from the repo root.

## Required files

The full set is the single source of truth `WORLD_SFX_FILES` in
`crates/nova_gameplay/src/audio.rs` (one row per `WorldSfx` variant); the
`every_world_sfx_key_has_a_file` test guards that each key has a file here.
Combat/world cues are **positional** (distance-attenuated from the listener
camera); the feedback ticks are **non-positional**.

### Combat / world (positional)

| File | Event | Character / length |
| --- | --- | --- |
| `turret_fire.wav` | A PDC/turret round is fired (`shoot_spawn_projectile`) | dry gunshot pop, ~0.07 s, played quietly (fires ~100/s) |
| `torpedo_launch.wav` | A torpedo leaves its bay (`shoot_spawn_projectile`) | airy rising whoosh, ~0.3 s |
| `explosion.wav` | A section/asteroid is destroyed or a torpedo detonates (`IntegrityDestroyMarker`) | noisy burst, fast decay, ~0.45 s |
| `impact.wav` | Damage is applied to a target (`HealthApplyDamage`) | short low thud, ~0.1 s, played quietly (fires per hit) |
| `thruster_loop.wav` | The engine hum, played continuously; volume tracks throttle | steady low drone, loops seamlessly, ~1 s |

### Ship feedback ticks (non-positional)

| File | Event | Character / length |
| --- | --- | --- |
| `lock_on.wav` | A radar gesture acquires its first target (once per gesture) | quick rising chirp, ~0.09 s |
| `lock_off.wav` | A tap-clear releases a lock | falling mirror of `lock_on`, ~0.09 s |
| `safety_on.wav` | The weapons safety re-engages (hot -> cold) | dull low click, ~0.06 s |
| `radar_deny.wav` | A radar hold is denied (computer grants no Lock) | low flat buzz, ~0.16 s |
| `salvage_pickup.wav` | A salvage crate is picked up | light rising "ding", quieter than the objective chime, ~0.10 s |
| `dry_fire.wav` | A turret pulls its trigger on an empty magazine | dull descending click, ~0.06 s |
| `radar_retarget.wav` | A held radar gesture re-designates to a new target | very short quiet tick (subtler than `lock_on`), ~0.045 s |

(The UI cues - `menu_select`, `ui_toggle`, `objective_new`,
`objective_complete` - are engine chrome and live in root `assets/sounds/`.)

## Web (wasm) builds

`index.html` already ships this directory into the web build via
`<link data-trunk rel="copy-dir" href="assets"/>`, so no per-file directive is
needed. Browser audio needs a user gesture before it will play; the existing
`build/web/sound.js` shim resumes the audio context on the first interaction.
