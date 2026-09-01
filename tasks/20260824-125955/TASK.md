# An audio direction pass: combat and UI soundscape

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.13.0,audio

Promoted 2026-08-31 from ideation into v0.13.0. The audio side has never
had a dedicated pass; this is the first.

Give the game a coherent soundscape: combat weight, UI feedback, engine
and RCS character, and a music direction. (Docking and station ambience
wait for `20260824-125943`, a future promise.)

## Owner's concrete cue

"Improve audio, e.g. in the main menu based on distance": the menu
backdrop is a live scene with ships moving past the camera - the ships
should be HEARD, and their loudness should follow their distance to the
listener. Spatial attenuation in the menu is the proof that the same
model works in flight.

## Direction (2026-09-01)

Two lanes, one writer each. The ENGINE lane (sprout `sound-engine`) owns the
plumbing; this lane owns the FILES and the Python that renders them. Neither
touches the other's paths, so the wiring of new cues waits for the engine to
land.

`INVENTORY.md` is the first shape item, done: what has a sound today, what is
silent, the four ways the current set is wrong, and a 40-file production list.
`audition.html` is the style probe - five cues rendered against the brief, with
scopes and measurements, for the owner to accept or reject the language before
the other thirty-five follow it.

Decisions taken, in the order they were needed:

1. THE VOICES. Two, kept deliberately disjoint, plus a sub-voice.
   - INTERFACE is settled and unchanged: the eleven NOVA OS files are the
     standard, and everything in `assets/sounds/` joins them.
   - WORLD is new, and its brief is the fiction: sound does not travel in
     vacuum, so what the pilot hears is either conducted through their own hull
     or synthesized by the ship's computer as feedback. A gun heard through a
     deck plate, not through air. Three layers - transient, body, ring - and a
     rule set that is checkable, not vibes: mono, no musical intervals, bulk
     energy under 2 kHz, attack under 5 ms, nothing dry-ringing past 400 ms.
   - AVIONICS is the controller's lock/radar/safety cues: world CONTENT, so a
     mod can reship them, but cockpit instruments rather than machinery. The
     interface recipe darkened, with a touch of the world voice's ring.
2. THE BUSES. The owner asked for two tracks (UI, SFX) with music reserved, and
   left the realism toggle open. Two mixer sliders is right, and the toggle
   wants a line the sliders do not draw: inside the World bus every cue is
   tagged `Hull` (your own ship - never attenuated, never panned, it is the
   room you are sitting in) or `Exterior` (everything else - attenuated and
   panned). `SoundEmulation::Vacuum` silences Exterior and leaves Interface and
   Hull. The tag also retires the existing `if player { skip attenuation }`
   special case in `compute_thruster_hum_volume`: your own engines are Hull by
   definition, not an if-statement.
3. PANNING keeps our tuned geometric rolloff (NEAR 20 / FAR 320) as the
   amplitude law rather than adopting rodio's 1/d. The engine lane is trying
   the fixed-radius emitter - place the emitter on a sphere around the listener
   in the true bearing, so rodio contributes pan only - and is instructed to
   record why if it does not hold up.
4. PYTHON. `flake.nix` gains a `python3.withPackages [numpy scipy]`. The NOVA
   OS renderer stays pure stdlib and is untouched; a layered
   noise-body-plus-resonator hit wants filter design and resonator banks, and
   hand-rolling those is the difference between a day and a week. Both new
   generators seed PER CUE from a hash of the cue's name, which fixes the one
   thing `gen-nova-os-sfx.py` got wrong: it draws from one shared stream in list
   order, so inserting a cue churns every later file.
5. FORMAT stays mono 44100 Hz 16-bit PCM WAV for effects. Music, when it lands,
   is the one thing that should be OGG.

Not decided here: music itself. The bus and its slider ship now so the saved
settings format does not break later; the direction is a separate call.

## Probe status (2026-09-01)

Five cues rendered by `scripts/gen-world-sfx.py` and auditioned in
`audition.html`: the PDC round (and the burst it actually becomes), the railgun
discharge, a kinetic impact, a section failing, and the main drive bed. Four of
them replace legacy files in place and are audible in game with no code change;
the fifth retired a real bug - `railgun_fire_sound` was literally
`torpedo_launch.wav` - so the lance is now authored onto its own file. `content
lint` is clean. AWAITING the owner's verdict on the language before the
remaining thirty-five are produced.

## Shape

- Inventory what exists first: which events have sounds today, which are
  silent, what the mixing story is. Keep the inventory with this task.
- Spatial attenuation and pan from listener distance and bearing, in the
  menu backdrop and in flight.
- Combat: weapons with weight, impacts that read through the hull,
  torpedo launch and PDC character distinct at a glance (the same
  at-a-glance rule the visuals follow).
- UI: confirm/back/focus feedback in menus and the editor.
- A music direction decision, even if the tracks land later - record it.
- Licence rule as for art: every asset carries its exact licence and
  attribution; share-alike is flagged loudly, not quietly borrowed.

## Done when

- The menu backdrop is audible and distance-attenuated.
- Flight, combat and UI have a coherent first-pass soundscape.
- An audio settings surface exists (master/music/effects at minimum).
- Every shipped sound's licence and attribution are recorded.
