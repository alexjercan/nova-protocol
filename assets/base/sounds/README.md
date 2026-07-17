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
the `self://` / `dep://base` / `dep://<id>` pipeline. The weapon and controller
families own their sounds this way: the turret's `fire_sound` +
`dry_fire_sound`, the torpedo bay's `launch_sound` + `detonation_sound`,
every section's / asteroid's `impact_sound` + `destroy_sound`, the thruster's
`loop_sound`, and the
controller's
`lock_on_sound`/`lock_off_sound`/`radar_deny_sound`/`radar_retarget_sound`/
`safety_on_sound` - plus the salvage crate's `pickup_sound` (base content
authors `self://sounds/...` for each, so the shipped game sounds unchanged -
but a mod can ship and name its own). Every cue is AUTHORED-OR-SILENT: content
that declares no sound plays none. The migration is COMPLETE (spike
20260717-101524): the transitional `WorldSfx` bank is deleted and no world
sound plays from any bank.

## Dropping in real audio

Replace each file below with a real sound **at the same path and filename**. No
code changes are needed: the loader (`crates/nova_assets/src/lib.rs`) loads
these fixed paths and the audio module plays whatever handle it is given.

- Formats: WAV works out of the box (the `bevy` dependency enables the `wav`
  decoder in `crates/nova_gameplay/Cargo.toml`). Nothing here loads through a
  bank: each file is the base mod's authored default, referenced from base
  content (`self://sounds/...`) and resolved by its cue at play time. OGG
  Vorbis also decodes (vorbis is on by default); to use `.ogg`, change the
  extension in the base content refs (regenerate via gen_content; and in base
  content refs).
- Suggested: 44.1 kHz, normalized but not clipping. Keep the one-shots short;
  `thruster_loop.wav` is the only looping asset and should be seamless (its
  start and end must meet without a click).
- To regenerate the placeholders (e.g. after deleting them):
  `python3 scripts/gen-placeholder-sounds.py` from the repo root.

## Required files

Every file here is a SECTION/OBJECT-AUTHORED DEFAULT: referenced from base
content (`self://sounds/...` on the owning config) and in no bank anywhere -
replacing a file re-voices the base content, and a mod can author its own
instead (spike 20260717-101524's end state; the transitional WorldSfx bank is
gone). Combat/world cues are **positional** (distance-attenuated from the
listener camera); the feedback ticks are **non-positional**.

### Authored defaults

| File | Authored on | Character / length |
| --- | --- | --- |
| `turret_fire.wav` | turret `fire_sound` (positional) | dry gunshot pop, ~0.07 s, played quietly (fires ~100/s) |
| `dry_fire.wav` | turret `dry_fire_sound` | dull descending click, ~0.06 s |
| `torpedo_launch.wav` | torpedo bay `launch_sound` (positional) | airy rising whoosh, ~0.3 s |
| `lock_on.wav` | controller `lock_on_sound` | quick rising chirp, ~0.09 s |
| `lock_off.wav` | controller `lock_off_sound` | falling mirror of `lock_on`, ~0.09 s |
| `radar_deny.wav` | controller `radar_deny_sound` | low flat buzz, ~0.16 s |
| `radar_retarget.wav` | controller `radar_retarget_sound` | very short quiet tick (subtler than `lock_on`), ~0.045 s |
| `safety_on.wav` | controller `safety_on_sound` | dull low click, ~0.06 s |
| `salvage_pickup.wav` | the salvage crate's `pickup_sound` | light rising "ding", quieter than the objective chime, ~0.10 s |
| `impact.wav` | every section's / asteroid's `impact_sound` (positional) | short low thud, ~0.1 s, played quietly (fires per hit) |
| `explosion.wav` | every section's / asteroid's `destroy_sound` + the torpedo's `detonation_sound` (positional) | noisy burst, fast decay, ~0.45 s |
| `thruster_loop.wav` | the thruster's `loop_sound` (one loop per distinct sound; volume tracks the loudest burning ship) | steady low drone, loops seamlessly, ~1 s |


(The UI cues - `menu_select`, `ui_toggle`, `objective_new`,
`objective_complete` - are engine chrome and live in root `assets/sounds/`.)

## Web (wasm) builds

`index.html` already ships this directory into the web build via
`<link data-trunk rel="copy-dir" href="assets"/>`, so no per-file directive is
needed. Browser audio needs a user gesture before it will play; the existing
`build/web/sound.js` shim resumes the audio context on the first interaction.
