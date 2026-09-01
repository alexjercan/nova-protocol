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
   - WORLD is new, and its brief is ordinary game sound. Combat in a vacuum
     would be silent and a silent fight is a boring fight, so Nova's guns sound
     the way a film's guns sound - present, bright and physical. Three layers -
     transient, body, ring - and a rule set that is checkable, not vibes: mono,
     full spectrum (punch under 500 Hz, identity 2-8 kHz), no musical
     intervals, attack under 5 ms, designed for the rate it is heard at.
   - AVIONICS is the controller's lock/radar/safety cues: world CONTENT, so a
     mod can reship them, but cockpit instruments rather than machinery. The
     interface recipe darkened, with a touch of the world voice's ring.
2. THE BUSES. Two mixer sliders as asked (Interface, World) with Music
   reserved so the saved settings format does not break later. Inside the World
   bus every cue is tagged `Hull` (your own ship - never attenuated, never
   panned, it is the room you are sitting in) or `Exterior` (everything else -
   attenuated and panned). The tag retires the existing
   `if player { skip attenuation }` special case in
   `compute_thruster_hum_volume`: your own engines are Hull by definition, not
   an if-statement. NO vacuum toggle is built - see 6.
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

6. VACUUM SOUNDS ARE NOT BUILT. The first probe took "sound does not travel in
   vacuum" as the design brief and rendered everything as hull-conducted. The
   owner's call: normal sounds now, and the realism version becomes either a
   setting or a mod later - undecided. So the ROUTING that would gate it is
   built (it pays for itself by deleting the player-attenuation hack) and the
   gate is not. A mod is the cheaper home for it either way: every world sound
   is content behind an `AssetRef`, so a vacuum mod is a second set of files
   under the same names and needs no engine support at all.

Not decided here: music itself. The bus and its slider ship now so the saved
settings format does not break later; the direction is a separate call. Nor
whether the vacuum mode is a setting or a mod.

## Probe rounds

ROUND ONE (2026-09-01). Five cues on the hull-conducted brief. Verdict: the
brief itself was wrong for now - see decision 6 - and two cues needed work. The
railgun was accepted as-is and has not been touched since; per-cue seeding
means later retuning cannot reach its bytes.

ROUND TWO (2026-09-01). The brief is ordinary game sound, and the notes were
"the PDC is missing a bit of high pitch", "would also be cool to have the
100 RPS because that's the standard PDC", and "main drive feels a bit noisy".
The PDC got its top end - a primer, a muzzle report and the rotary action
across 2-9 kHz - moving its centroid from 987 Hz to 6.0 kHz with the punch
under 500 Hz held at 66%. A held fire loop at the gun's true rate was built to
answer the 100 RPS note. Impact and section-failing were brightened to match,
since the rule they were built against ("bulk energy under 2 kHz") is the one
the new brief drops.

ROUND THREE (2026-09-01). The fire loop was auditioned and REJECTED - "it's
like a buzzing sound". At a 10 ms period the rounds fuse and the gun saws
instead of firing, which answers a question worth writing down: the rate a cue
is designed for is the rate the CUE plays at, not the rate the hardware runs
at. The gun authors 50 rounds a second per muzzle (100 on a twin) and
`TURRET_FIRE_MIN_INTERVAL` already collapses that to twenty, which is where it
stays. The loop asset and its generator are deleted, and the engine lane is NOT
asked for a held-loop cue.

The drive was still floaty, and the owner's read was that the old placeholder -
a bare two-oscillator hum - was closer to what they wanted despite being
cruder. So it is built the other way up: a tonal spine carries it (52 Hz under
load with 26 underneath, felt more than heard, each partial breathing on its
own slow LFO so the stack never freezes into a chord) and the turbulence is
texture over the top rather than the substance. 90% of its energy is now under
120 Hz.

Accepted and settled: the railgun (round one), the kinetic round and the
section failing (round two). Per-cue name-seeding is what makes "settled" mean
something here - a later round cannot reach an accepted cue's bytes.

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
