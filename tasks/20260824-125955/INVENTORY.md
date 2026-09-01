# Sound inventory and production list

The task's first shape item: what has a sound today, what is silent, and what
has to be produced. Written 2026-09-01, before any audio work landed.

Two lanes run off this document. The ENGINE lane (sprout `sound-engine`) owns
the plumbing - buses, routing, panning, the loop API. This lane owns the FILES
and the Python that renders them. Neither touches the other's paths.

## 0. Status (2026-09-01)

ALL 44 FILES ARE RENDERED. Sections 2 to 4 are the record of what was found on
2026-08-31 and are left as they were; section 6 carries the current state of
each file. What remains is wiring, not sound design: 18 of the 44 have nothing
that plays them yet, split between this lane's authoring and the engine lane's
hooks.

Two things moved after the list below was written:

- `warn_lock` and `warn_hull` are AVIONICS, not interface. They are the ship's
  threat panel, a mod would want them, and the rule that decides is where the
  variation lives - so they render into `assets/base/sounds/`.
- Nine cues render onto LEGACY filenames and KEEP them. `impact.wav`,
  `explosion.wav`, `turret_fire.wav` and the rest are public modding surface,
  documented in `web/src/create/sections.md` and `objects.md` and referenced by
  content we do not own. Renaming them to match the cue names would break other
  people's mods to make our filenames prettier. The cue name is the design's
  name, the path is the content's name, and they are allowed to differ.

## 1. Where sounds live

Two roots, and the split is ownership, not convenience.

- `assets/sounds/` - INTERFACE chrome. Engine assets like `assets/icons/`:
  loaded straight from the asset root, in no bundle's `resources`, and
  unreachable by `self://` / `dep://`. Keyed by the `UiSfx` enum
  (`crates/nova_gameplay/src/audio/mod.rs`) and mapped by `UI_SFX_FILES`.
- `assets/base/sounds/` - WORLD content. Part of the base mod, declared in
  `assets/base/base.bundle.ron`, referenced by content as
  `self://sounds/<name>.wav` and by other mods as `dep://base/sounds/<name>.wav`.
  Never keyed by an enum: every world cue resolves an `AssetRef<AudioSource>`
  authored on the section or object that makes the sound, and is
  AUTHORED-OR-SILENT.

Format, uniform across both roots and unchanged: mono, 44100 Hz, 16-bit PCM
WAV. Music, when it lands, is the one thing that should be OGG instead - the
size argument only bites above a few seconds.

## 2. What exists today

### Interface - `assets/sounds/`, 15 `UiSfx` keys over 11 files

| Key | File | Fired from | Verdict |
| --- | --- | --- | --- |
| `MenuSelect` | `menu_select.wav` | `nova_menu/widgets.rs` `On<Activate>` | RE-VOICE |
| `UiToggle` | `ui_toggle.wav` | `nova_menu/pause.rs` | RE-VOICE |
| `ObjectiveNew` | `objective_new.wav` | `nova_hud/objective_feedback.rs` | RE-VOICE |
| `ObjectiveComplete` | `objective_complete.wav` | `nova_hud/objective_feedback.rs` | RE-VOICE |
| `CommsLine` | `ui_toggle.wav` | `nova_hud/comms_panel.rs` | ALIAS - needs its own file |
| `NovaOsKey` | `nova_key.wav` | `nova_os_ui/terminal/sound.rs` | KEEP |
| `NovaOsBack` | `nova_back.wav` | same | KEEP |
| `NovaOsEnter` | `nova_enter.wav` | same | KEEP |
| `NovaOsOk` | `nova_ok.wav` | same | KEEP |
| `NovaOsError` | `nova_error.wav` | same | KEEP |
| `NovaOsTick` | `nova_tick.wav` | same | KEEP |
| `NovaOsCoil` | `nova_coil.wav` | same | KEEP |
| `NovaOsPowerUp` | `nova_powerup.wav` | same | KEEP |
| `NovaOsPowerDown` | `nova_powerdown.wav` | same | KEEP |
| `NovaOsBed` | `nova_bed.wav` (loop) | same | KEEP |

The eleven `nova_*.wav` are the only sounds in the game that were DESIGNED
rather than placeheld: `scripts/gen-nova-os-sfx.py` renders them from the
WebAudio recipes in `web/design/nova_os_terminal_poc.html`. They are the
interface standard and they stay. The other four are
`scripts/gen-placeholder-sounds.py` output and do not match them.

### World - `assets/base/sounds/`, 13 files

| File | Authored on | Cue | Verdict |
| --- | --- | --- | --- |
| `turret_fire.wav` | turret `fire_sound` | `on_turret_fire_play_sfx` | REDO - the flagship |
| `dry_fire.wav` | turret `dry_fire_sound` | `play_dry_fire_cue` | REDO |
| `torpedo_launch.wav` | bay `launch_sound` | `on_torpedo_launch_play_sfx` | REDO |
| `explosion.wav` | every section `destroy_sound`, bay `detonation_sound` | `on_destroyed_play_explosion` | REDO + SPLIT |
| `impact.wav` | every section `impact_sound` | `on_damage_play_impact` | REDO + SPLIT |
| `thruster_loop.wav` | thruster `loop_sound` | `ensure_thruster_loops` | REDO + SPLIT |
| `rcs_loop.wav` | controller `rcs_loop_sound` | `ensure_rcs_loops` | REDO |
| `lock_on.wav` | controller `lock_on_sound` | `play_lock_cues` | REDO |
| `lock_off.wav` | controller `lock_off_sound` | `play_lock_cues` | REDO |
| `radar_deny.wav` | controller `radar_deny_sound` | `play_lock_cues` | REDO |
| `radar_retarget.wav` | controller `radar_retarget_sound` | `play_lock_cues` | REDO |
| `safety_on.wav` | controller `safety_on_sound` | `play_safety_engaged_cue` | REDO |
| `salvage_pickup.wav` | crate `pickup_sound` | `on_crate_pickup_play_sfx` | REDO |

## 3. What is wrong today

Four defects, all of them "convenient for the PoC".

1. **Sounds are shared across events that are not the same event.**
   - `railgun_fire_sound` is literally `torpedo_launch.wav`
     (`base_content/assets.rs:179`). A spinal lance and a torpedo tube sound
     identical.
   - `CommsLine` is literally `ui_toggle.wav`. A story beat sounds like ESC.
   - The gatling PDC and the twin PDC share `turret_fire.wav`. Two different
     guns, one voice.
   - The basic, vector and capital thrusters share `thruster_loop.wav`. A
     capital drive sounds like an attitude jet.
   - The torpedo's `detonation_sound` is `section_destroy_sound`. A warhead
     sounds like a girder snapping.
   - Every section, of every material and size, shares one `impact_sound` and
     one `destroy_sound`. Asteroids author the same two.
2. **Damage type is inaudible.** `DamageType::{Kinetic, Pierce, Explosive}` are
   three different verbs the visuals already distinguish by colour. All three
   sound the same.
3. **Twelve of the twenty-four files are undesigned.** They are
   `gen-placeholder-sounds.py` output: three synths (noise burst, pitch sweep,
   steady tone) with no shared design language. They do not sit in the same
   world as the NOVA OS family.
4. **Entire subsystems are silent** - see the next section.

## 4. What is silent

Grouped by whether the code already has somewhere to hang the sound.

### Has a live fire site or a live state - only the asset and one call are missing

| Moment | Where | Note |
| --- | --- | --- |
| Railgun CHARGE | `RailgunCharge::Charging { elapsed }`, `charge_seconds` | The biggest gap. The charge has a light riding the bore and no sound at all. A loop whose pitch tracks `progress` is the whole cue. |
| Railgun reload finishing | `SectionReload` on the lance | Twelve seconds of silence ending in nothing. |
| Turret stow doors | `SectionAnimationCue::StowDoors`, driven by `turret_section/stow.rs` | The lids open and shut in silence. |
| Turret muzzle door | `SectionAnimationCue::MuzzleDoor` | Same. |
| Ammo running dry | `SectionAmmo::is_empty` | Distinct from the dry-fire click: the click is a mistake, this is a state change. |

### Needs both a hook and an asset

| Moment | Note |
| --- | --- |
| A whole SHIP dying | Only per-section `explosion` fires. A ship coming apart sounds like one girder. |
| Incoming lock / incoming torpedo | The HUD shows it (`lock_crosshairs`, `torpedo_target`); nothing warns by ear. |
| Hull integrity critical | `HudSituations` senses low ammo but no health band. |
| Ship-to-ship or ship-to-rock collision | Physics contacts make no sound. |

### Whole crates with no audio at all

- `nova_editor` - place, remove, rotate, invalid placement, save, load. Zero
  cues. The task's "Done when" names the editor explicitly.
- `nova_menu` beyond the two clicks it has - no focus, no back, no slider detent.
- Docking and station ambience are DEFERRED to `20260824-125943`; out of scope
  here, listed so the gap is not rediscovered.

## 5. The style

Two voices, kept deliberately disjoint. A sound is legible because you know
which world it came from before you have identified it.

### The INTERFACE voice - settled, unchanged

The NOVA OS family is the standard and everything in `assets/sounds/` joins it.
Its recipe is already written down in `scripts/gen-nova-os-sfx.py`: one or two
oscillators (square / saw / sine) with an optional exponential pitch slide,
through an RBJ biquad, under an exponential attack-decay envelope, 20-60 ms for
a tick and up to 700 ms for a sweep, peak-normalized to -3 dBFS, rendered
offline from a fixed seed. It is a CRT terminal: thin, electric, tonal, dry.

### The WORLD voice - new

Ordinary game sounds. Combat in a vacuum would be silent and a silent fight is
a boring fight, so Nova's guns sound the way a film's guns sound - present,
bright and physical - and the game does not apologise for it.

A "vacuum sounds" mode, where every cue is instead conducted through the
player's own hull or synthesized by the ship's computer as feedback, stays on
the table as a FUTURE setting. It is deliberately not built now, and keeping it
open costs nothing: every world sound is mod content addressed by an
`AssetRef`, so that mode is a second set of files behind the same names.
Nothing in this pass has to change to allow it.

Every world sound is built from the same three layers - which is how a
percussive sound works, not a physics argument:

1. **Transient**, 0-8 ms. The crack. Broadband and BRIGHT: this is where a gun
   gets its edge and most of its identity.
2. **Body**, 10-200 ms. Filtered noise carrying the mass, 80-800 Hz. This is
   where a PDC gets its chest punch.
3. **Ring**, up to ~400 ms. Three to six detuned resonant modes - the hardware
   answering.

Rules that hold across the whole world set:

- **Designed for the rate it is heard at**, which is not always the rate the
  hardware runs at. The PDC authors 50 rounds a second per muzzle, but its cue
  throttles to twenty (`TURRET_FIRE_MIN_INTERVAL`), so its round is shaped to
  stand alone at 50 ms spacing. A held loop at the true rate was built and
  rejected - at a 10 ms period the rounds fuse and the gun saws.
- Mono. The engine pans it; a pre-panned file cannot be placed.
- Full spectrum. Punch lives under 500 Hz, identity lives 2-8 kHz. A cue with
  only the first is dull; with only the second, thin.
- No musical intervals, no arpeggios, no bare square waves. Tonal content is
  the interface voice's job and the separation is the point. This is the "no
  bit vibe" rule, stated so it is checkable.
- Attack under 5 ms on anything that is an event.
- Peak-normalize to -3 dBFS and let the per-cue volume constants in
  `ship_audio/mod.rs` do the mixing. The file is not where balance lives.
- Loops render seamless. Steady beds are synthesized in the FREQUENCY domain
  (a shaped magnitude spectrum with random phases, inverse-transformed), which
  is periodic by construction; fire loops place an exact whole number of
  rounds and wrap each tail onto the front. A time-domain crossfade always
  leaves a soft spot the ear finds by the third repeat.

### The AVIONICS sub-voice

`lock_on`, `lock_off`, `radar_deny`, `radar_retarget`, `safety_on` are world
CONTENT (authored on the controller section, so a mod can reship them) but they
are cockpit instruments, not machinery. They take the interface voice darkened:
the same oscillator-through-bandpass recipe, pitched lower, with a touch of the
world voice's ring on the tail. That is what makes the ship's console sound
like it belongs to the ship rather than to the OS.

## 6. Production list

`[site]` = a fire site exists today, only the asset and its authoring are
missing. `[hook]` = needs a code hook too, which is the engine lane's or a
follow-up's. `[content]` = no code at all, just a new file and an `AssetRef`.

### Interface - `assets/sounds/` (interface voice)

| File | Cue | Status |
| --- | --- | --- |
| `menu_select.wav` | menu button pressed | re-voice `[site]` |
| `menu_back.wav` | backing out of a panel | new `[hook]` |
| `menu_focus.wav` | focus moves to a button | new `[hook]` |
| `ui_toggle.wav` | pause overlay open/close | re-voice `[site]` |
| `ui_tick.wav` | slider detent, toggle step | new `[hook]` |
| `objective_new.wav` | objective posted | re-voice `[site]` |
| `objective_complete.wav` | objective completed | re-voice `[site]` |
| `objective_fail.wav` | objective failed | new `[hook]` |
| `comms_line.wav` | comms line shown | new `[site]` - retires the `ui_toggle` alias |
| `editor_place.wav` | section placed | new `[hook]` |
| `editor_remove.wav` | section removed | new `[hook]` |
| `editor_rotate.wav` | rotation step | new `[hook]` |
| `editor_deny.wav` | invalid placement | new `[hook]` |

The eleven `nova_*.wav` are untouched.

### World - `assets/base/sounds/` (world voice)

| File | Authored on | Status |
| --- | --- | --- |
| `turret_fire.wav` (cue `pdc_gatling_fire`) | gatling turret `fire_sound` | redo `[site]` - the flagship, tuned for the cue's 20/s throttle |
| `pdc_twin_fire.wav` | twin turret `fire_sound` | new `[content]` - retires the shared voice |
| `dry_fire.wav` (cue `pdc_dry_fire`) | turret `dry_fire_sound` | redo `[site]` |
| `pdc_stow_open.wav` | turret housing | new `[hook]` |
| `pdc_stow_close.wav` | turret housing | new `[hook]` |
| `torpedo_launch.wav` | bay `launch_sound` | redo `[site]` |
| `torpedo_detonate.wav` | bay `detonation_sound` | new `[content]` - retires the section-destroy reuse |
| `bay_door.wav` | bay muzzle door | new `[hook]` |
| `railgun_charge.wav` (loop) | lance | new `[hook]` - the biggest single gap |
| `railgun_fire.wav` | lance `fire_sound` | new `[content]` - retires the torpedo-launch alias |
| `railgun_reload.wav` | lance | new `[hook]` |
| `impact.wav` (cue `impact_kinetic`) | section `impact_sound` | redo `[site]` |
| `impact_pierce.wav` | section, pierce hits | new `[hook]` - damage type is known at hit time |
| `impact_explosive.wav` | section, blast hits | new `[hook]` |
| `impact_rock.wav` | asteroid `impact_sound` | new `[content]` |
| `explosion.wav` (cue `destroy_section`) | section `destroy_sound` | redo `[site]` |
| `destroy_rock.wav` | asteroid `destroy_sound` | new `[content]` |
| `destroy_ship.wav` | ship root | new `[hook]` |
| `thruster_loop.wav` (cue `thruster_basic_loop`) | basic thruster `loop_sound` | redo `[site]` |
| `thruster_vector_loop.wav` | vector thruster `loop_sound` | new `[content]` |
| `thruster_capital_loop.wav` | capital thruster `loop_sound` | new `[content]` |
| `rcs_loop.wav` | controller `rcs_loop_sound` | redo `[site]` |
| `salvage_pickup.wav` | crate `pickup_sound` | redo `[site]` |

### Avionics - `assets/base/sounds/` (avionics voice)

| File | Authored on | Status |
| --- | --- | --- |
| `lock_on.wav` | controller `lock_on_sound` | redo `[site]` |
| `lock_off.wav` | controller `lock_off_sound` | redo `[site]` |
| `radar_deny.wav` | controller `radar_deny_sound` | redo `[site]` |
| `radar_retarget.wav` | controller `radar_retarget_sound` | redo `[site]` |
| `safety_on.wav` | controller `safety_on_sound` | redo `[site]` |
| `ammo_dry.wav` | turret / lance, magazine empty | new `[hook]` |
| `warn_lock.wav` | a hostile has locked you | new `[hook]` - `ThreatContacts` has the data |
| `warn_hull.wav` | hull integrity critical | new `[hook]` - no threshold alert exists |

Totals: 13 interface files, 23 world files, 8 avionics files - 44 rendered,
plus the 11 NOVA OS files kept untouched as the interface standard.

Of the 44, 26 have something that plays them today. The remaining 18 split into
FOUR that need only an `AssetRef` in the content builders (`pdc_twin_fire`,
`torpedo_detonate`, `thruster_vector_loop`, `thruster_capital_loop` - this
lane's work), TWO that wait on the material decision (`impact_rock`,
`destroy_rock`), and TWELVE that need a code hook, which is the engine lane's
or a follow-up's.

## 7. How they get made

`scripts/gen-nova-os-sfx.py` proves the approach: render offline from a fixed
seed, commit the WAVs, and a rerun must be byte-identical or it shows up as
asset churn. Both new generators keep that rule.

It also has a flaw worth not repeating: its cues draw from ONE shared RNG in
the order `build_cues` lists them, so inserting a cue rewrites every later
cue's bytes. The new generators seed PER SOUND from a hash of the sound's name,
so adding a sound touches exactly one file.

The DSP is where the two differ. The NOVA OS script hand-rolls its
oscillators and biquads in pure stdlib, which is fine for a 30 ms square-wave
tick and painful for a layered noise-body-plus-resonator hit. The world voice
wants filter design, resonator banks and spectral shaping, so it wants numpy
and scipy - see the flake note in `TASK.md`.

As built: `scripts/nova_sfx.py` holds the shared toolkit (sources, envelopes,
filters, resonators, the WAV writer), `scripts/gen-world-sfx.py` renders the 23
machinery cues, and `scripts/gen-ui-sfx.py` renders the 13 interface and 8
avionics cues. `scripts/gen-nova-os-sfx.py` is untouched and stays stdlib-only.
`scripts/gen-placeholder-sounds.py` is DELETED - its whole job was to keep the
game audible until real assets existed, and running it now would overwrite
them.

`scripts/gen-sfx-audition.py` renders `audition.html`, the bench the owner
validates on: every cue grouped by family, with an A/B take against whatever it
has to be distinguishable from, a scope, and the measurements.

## 8. Licensing

Every file listed here is generated by a script in this repo from first
principles: no samples, no libraries, no third-party audio. The licence and
attribution requirement in the task's "Done when" is therefore satisfied by
the generator plus this document, and the rule to hold going forward is that a
sound arriving from ANYWHERE else carries its exact licence and attribution,
with share-alike flagged loudly.
