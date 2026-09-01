# World and avionics sound effects

The base mod's WORLD cues - the sounds of things that exist in the game world -
and its AVIONICS cues, the ship's own cockpit instruments.

These live under `assets/base/` because the base game is just a mod (task
20260717-002228) and a mod can reship or reference any of them. They are
declared in the base bundle's `resources` list (`assets/base/base.bundle.ron`),
so a mod reaches them with `dep://base/sounds/<name>.wav` - the same scheme the
base uses with `self://`. Engine chrome (menus, the editor, objectives, comms)
lives at the asset ROOT in `assets/sounds/`; see the README there for the rule
that decides which directory a sound belongs in.

## Authored-or-silent

A section or object declares a sound as an authorable `AssetRef<AudioSource>`
content field, exactly like it declares a render mesh. Content that declares no
sound plays none - there is no fallback bank, and the transitional `WorldSfx`
bank was deleted when the migration completed (spike 20260717-101524). So a
file in this directory that nothing authors is silent until something does.

Combat and world cues are POSITIONAL (attenuated by distance from the listener
camera). The avionics cues are the ship talking to its own pilot.

## The voices

Two renderers, kept deliberately disjoint, over the shared DSP toolkit in
`scripts/nova_sfx.py`:

    nix develop --command python3 scripts/gen-world-sfx.py
    nix develop --command python3 scripts/gen-ui-sfx.py

`gen-world-sfx.py` builds machinery from three layers - a broadband transient,
a filtered-noise body, and a bank of resonator modes - plus saturation for
weight. `gen-ui-sfx.py` builds the avionics from the interface recipe darkened,
with a little of the world voice's metal ring underneath, because a lock tone
is an instrument reporting rather than a thing happening in space.

Every cue seeds its own generator from a hash of its NAME, so a rerun is
byte-identical and adding a cue rewrites no other cue's bytes. Files are mono,
44100 Hz, 16-bit PCM, peak normalized to -3 dBFS: balance is NOT set here, the
per-cue constants in `nova_ship/src/ship_audio/` do that.

Loops are synthesized in the frequency domain so the last sample joins the
first exactly. A time-domain crossfade always leaves a soft spot the ear finds
after the third repeat. Check a new loop by concatenating three copies: the
step at the joins must sit inside the loop's own internal step range.

Two design rules worth keeping when adding one:

- A cue is designed for the RATE IT IS HEARD AT, which is not always the rate
  the hardware runs at. The PDC authors 50 rounds a second per muzzle and its
  cue throttles to twenty; twenty is what the round is shaped against.
- The same event on different hardware is separated by PITCH, not decoration.
  The drives run 34 / 52 / 78 Hz from capital to basic to vector.

OGG Vorbis also decodes if you prefer it; change the extension in the base
content refs and regenerate with `cargo run -- content gen`.

## Files

`Authored on` is where base content references the file today. Files marked
`-` are rendered and waiting for the cue that plays them.

### Guns

| File | Authored on |
| --- | --- |
| `turret_fire.wav` | turret `fire_sound` |
| `pdc_twin_fire.wav` | - (the twin mount shares the gatling's file today) |
| `dry_fire.wav` | turret `dry_fire_sound` |
| `pdc_stow_open.wav` | - (the `StowLift` / `StowDoors` animation) |
| `pdc_stow_close.wav` | - |
| `bay_door.wav` | - (the `MuzzleDoor` animation) |
| `railgun_fire.wav` | railgun `fire_sound` |
| `railgun_charge.wav` | - (a LOOP, meant to be played at a rising rate as the charge fills) |
| `railgun_reload.wav` | - |

### Ordnance and impacts

| File | Authored on |
| --- | --- |
| `torpedo_launch.wav` | torpedo bay `launch_sound` |
| `torpedo_detonate.wav` | - (warheads play `explosion.wav` today) |
| `impact.wav` | every section's `impact_sound` |
| `impact_pierce.wav` | - (`DamageType::Pierce`) |
| `impact_explosive.wav` | - (`DamageType::Explosive`) |
| `impact_rock.wav` | - (asteroids are silent on hit today) |

### Destruction

| File | Authored on |
| --- | --- |
| `explosion.wav` | every section's `destroy_sound` and the torpedo's `detonation_sound` |
| `destroy_rock.wav` | - |
| `destroy_ship.wav` | - (a ship dying is silent today) |

### Drives

| File | Authored on |
| --- | --- |
| `thruster_loop.wav` | the basic thruster's `loop_sound` |
| `thruster_vector_loop.wav` | - (the 3x3x2 drive shares the basic file today) |
| `thruster_capital_loop.wav` | - (the 5x5x3 drive shares it too) |
| `rcs_loop.wav` | the controller's `rcs_loop_sound` |

### Avionics

| File | Authored on |
| --- | --- |
| `lock_on.wav` | controller `lock_on_sound` |
| `lock_off.wav` | controller `lock_off_sound` |
| `radar_deny.wav` | controller `radar_deny_sound` |
| `radar_retarget.wav` | controller `radar_retarget_sound` |
| `safety_on.wav` | controller `safety_on_sound` |
| `ammo_dry.wav` | - (the ammo readout) |
| `warn_lock.wav` | - (a hostile has locked you; `ThreatContacts` has the data) |
| `warn_hull.wav` | - (no hull threshold alert exists yet) |

### Handling

| File | Authored on |
| --- | --- |
| `salvage_pickup.wav` | the salvage crate's `pickup_sound` |

## Web (wasm) builds

`index.html` ships this directory into the web build via
`<link data-trunk rel="copy-dir" href="assets"/>`, so no per-file directive is
needed. Browser audio needs a user gesture before it will play; the
`build/web/sound.js` shim resumes the audio context on the first interaction.
