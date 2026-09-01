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

7. WHERE A SOUND LIVES. The directory split is not UI against SFX - that
   question has no stable answer, and "is a lock tone chrome or an effect?"
   went round twice before the right axis turned up. It is WHERE THE VARIATION
   LIVES:
   - The ENGINE plays it uniformly for everyone, and it still has an event to
     fire on with zero mods loaded -> chrome, `assets/`.
   - CONTENT authors it per thing, because two of that thing could reasonably
     differ -> `assets/base/`, behind an `AssetRef`.

   That puts the whole cockpit in the base mod, which is the owner's own
   argument: locking is a CAPABILITY of a controller section, so a cheap
   civilian controller and a military one should be allowed to sound
   different. It costs nothing to allow - `controller_lock_on_sound` is
   already an `AssetRef` on the controller config, so this was never a
   question about machinery, only about which directory the defaults sit in.
   `warn_lock` and `warn_hull` moved there too, by the same argument. What is
   left in `assets/` is menus, the editor, objectives, comms, and the eleven
   NOVA OS files.

8. FILENAMES DO NOT FOLLOW CUE NAMES. Nine cues render onto legacy paths -
   `impact.wav`, `explosion.wav`, `turret_fire.wav` - and keep them. Those are
   public modding surface, documented in `web/src/create/` and referenced by
   content we do not own. The cue name is the design's name, the path is the
   content's name, and a rename would break other people's mods to make ours
   prettier.

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
section failing (round two), the PDC and the drive (round three).

ROUND FOUR (2026-09-01). "Sounds good let's use these" closed the probe, and
the other 39 files were produced in the accepted language. The five accepted
cues are byte-identical except `thruster_loop.wav`, which changed by DC removal
only: `loop_noise` was putting the shaping function's value at bin 0, so every
frequency-domain bed carried a constant offset worth 0.8-1.7% of its headroom.
Inaudible, but wrong by construction in a primitive two voices now build on.

What the batch itself taught, both caught by measuring rather than by ear:

- Four of the new destruction cues came out at 1-2% of their energy in the
  2-8 kHz identity band, against the accepted section-failure's 9%. Each had
  a bright layer that was simply swamped by its own low end. Retuned onto the
  accepted profile - and `impact_explosive` only moved once the BLAST came
  down, not when the top went up, which is the general lesson: when a layer
  cannot be heard, the fix is usually the layer on top of it.
- Three drives on one recipe at 34 / 52 / 78 Hz reads as three sizes of the
  same machine. Pitch is the whole separation; nothing else about them differs.
  That generalised into a rule for the set: same event, different hardware,
  separated by pitch and not by decoration.

`audition.html` is regenerated for all 44, grouped by family, and most strips
carry an A/B take against the cue they have to be distinguishable from -
auditioning the twin PDC alone says nothing, next to the gatling says
everything.

## The mix pass (2026-09-01)

Three owner notes, and one of them turned out to be a rule rather than a
number.

THE BACKDROP CAMERA. The engine lane's measurement: the duel framed its fight
from 313 u with `SFX_FAR_DISTANCE` silent past 320, so peak gain over a 35 s
run was 0.095 and the backdrop was mute. The owner's call was to move the
camera rather than retune the rolloff. It reaches two of the four backdrops:

- The DUEL comes in to 200 (fight gain at frame centre 0.004 -> 0.12) and the
  shot improves - the first cut was mostly empty black above the action.
- The WEAVE comes in to 353, the closest pose that still frames its 140 u
  patrol ring at 4:3.
- The GAUNTLET cannot. Its KP-7 beacon sits at x -150 and a 4:3 frame sees
  ~0.55 x the camera distance, so 260 already has the beacon exactly on the
  edge; coming in far enough to matter crops it. Verified by capture, not by
  arithmetic - the move was made, the beacon was cut, the move was reverted.
- The WAYSTATION cannot either, and does not need to: the planetoid at the
  origin IS the shot and the traffic works its flanks at +-140..180, so the
  near arc is already inside the rolloff.

Both held poses say why at their own `SetCamera`. What the camera cannot reach
is left open - a per-scenario listener range is the answer neither of us named,
and it is the owner's call whether the gauntlet is worth one.

A LINEAR VOLUME IS NOT A LOUDNESS. Two cues in the table were set by comparing
numbers across cues that share no spectrum, which is not a comparison:

- `RCS_MAX_VOLUME` 0.22 sat under `ENGINE_MAX_VOLUME` 0.30 and read as "a touch
  quieter than the main drive". Measured A-weighted it was 11.4 dB LOUDER: the
  drive is a 66 Hz spine where the ear is least sensitive, the RCS a 1.6 kHz
  hiss where it is most. 0.05 makes the old comment true for the first time.
- `TORPEDO_LAUNCH_VOLUME` 0.45 measured as the LOUDEST cue in the game, over
  the railgun the table calls loudest and the explosion it is meant to sit
  under. Same trap, same octave-and-a-half gap. Now 0.30.

The railgun keeps 0.55 and does NOT get the crown its comment claims: its
report is almost all sub-100 Hz, so it measures ~7 dB under the explosion.
Raising it is a call for the seat, and the comment now says so rather than
asserting a rank the numbers do not support.

A SALVO IS ONE REPORT. Every tube on a hull shares a trigger and a 1 s reload,
so a multi-bay ship launched N thumps into one frame and they summed into a
clipped one. `ThrottleKey::TorpedoLaunch` keys on the firing SHIP - deliberately
the opposite policy to `TurretFire`, which keys on the gun because a gun stream
should read as many guns while a salvo should read as one event. A world cell
would not have done it: a capital hull's tubes sit further apart than
`SFX_AREA_CELL`.

## The wiring pass (2026-09-01)

The renders were done and 22 of them were reachable from nothing. Four commits
closed that down to five, and the recurring lesson was that a cue's HOOK is
never the thing its filename suggests - it is whatever the mechanic actually
has an edge on.

MENUS AND THE EDITOR (`d1d885cd`, `007c6ebd`). Eight interface cues and four
editor ones. Two of them needed a component read rather than an event, because
Bevy has no event for either: `Hovered` is written in place, so focus is
`Changed<Hovered>` plus a rising-edge check with disabled buttons excluded; the
ghost's rotation is a `Local` compare of `PlacementPose`, which had to grow
`PartialEq` for it. A back button is a MARKER on the button
(`back_button(text)`), not a string test on its label - four of them across the
menus and the pause stack. The settings slider ticks on `value != was` and
nothing else: `SliderStep` quantisation IS the detent, so there was no second
mechanism to build. Defeat is gated on `outcome.is_changed()` or a queued
scenario switch re-plays it on the restack.

THE WORLD (`01a517b1`). Six cues, and each one is a note about where the edge
lives:

- `warn_lock` reads `CombatLock` on hostile ships and latches. Not
  `ThreatContacts`, which is the look-ray candidate list and would fire on
  being GLANCED at. Hostility is the `relation()` test the rest of combat uses,
  so a scripted defection changes the answer for free.
- `ammo_dry` is a cockpit gauge on the flight computer, sounding once per ship
  per frame behind the mounts' own per-gun clicks. Eight dead triggers on a
  broadside are eight clicks - that is what eight guns sound like - and ONE
  pip, because a magazine state is one fact.
- `bay_door` hangs on the muzzle iris's animation TARGET changing.
  `cue_progress` cannot say it: a door 40% open reads identically whether it is
  opening or closing. `SectionAnimations::cue_target` is the counterpart that
  makes a mechanism's direction readable, and it comes out once per SALVO for
  free, since a held trigger holds the target.
- `railgun_reload` answers a magazine returning to CAPACITY, reported by
  `tick_section_reload` as `SectionReloadComplete`. Full is the only reload
  boundary that means the same thing on every weapon: a PDC trickling rounds
  back has no moment it finished, a one-shell lance has exactly one.
- `railgun_charge` is a loop kept deliberately OFF the shared `reconcile_loops`
  reconciler. A hum must ease; a capacitor must not. It rises in gain from a
  floor and in playback RATE with the charge, so the gun sounds like it is
  approaching the shot - which needed `drive_sfx_voices` to push `speed` to a
  live sink, not only at spawn.
- `destroy_ship` rides the `StructuralCollapseMarker` edge, not the root's
  despawn: collapse is the frame a ship stops being a ship, and the peel that
  follows runs for several frames under the cue. Deliberately unthrottled, or
  the section explosions it overlaps would swallow it through their shared cell
  key and a hull dying would sound like one more piece coming off.

THE ROCKS (`d33bd9a9`). Every shipped asteroid authored `impact.wav` and
`explosion.wav` - the files written for a hull being hit and a section failing -
so shooting a rock sounded exactly like shooting a ship. `impact_rock` and
`destroy_rock` needed no material table to reach: the TARGET half of "what hit
what" is already a per-object field.

Levels throughout were solved, not guessed: each new cue was measured
A-weighted and placed at a deliberate offset from an anchor in its own part of
the spectrum, per the mix pass's rule above.

WHAT IS LEFT. Five files at the end of the pass. `warn_hull` is the next
section and is now built; `impact_pierce` and `impact_explosive` wait on the
material table. (`impact_rock` and `destroy_rock` were the other two and are
authored above.)

## warn_hull: the hull alarm (2026-09-01)

The three decisions the queue entry left open, answered by the owner as "one
tier on the controller", and built:

1. ONE TIER. Several would be a gauge, and a gauge wants a readout to sit next
   to - there is no hull readout in `nova_hud` at all, so this alarm IS the
   integrity instrument and it says one thing: you are in trouble. It sounds on
   the FALLING edge only, latched.
2. ON THE CONTROLLER, both halves. `warn_hull_sound` and `warn_hull_fraction`
   are `ControllerSectionConfig` fields, by decision 7: knowing you are dying
   is a sensor capability, and the threshold is what makes "a cheap civilian
   computer warns late, or never" expressible. `0.0` is a computer that never
   warns; the default is `DEFAULT_WARN_HULL_FRACTION` = 0.30.
3. IT ARRIVES ALONE, and that is now a deliberate answer rather than an
   omission. Audio-only integrity feedback is the game for the moment; if it
   reads badly in the seat, the fix is a readout, not a second alarm.

The fraction is the aggregate `Health` on the ship ROOT -
`aggregate_ship_health` recomputes it every frame as the sum over standing
sections against the pinned built maximum. That is the same quantity structural
collapse is priced in, so 0.30 and the collapse default 0.05 are directly
comparable: the alarm sits six times clear of the wreckage floor, which is what
leaves a pilot room to break off.

Three things the implementation had to get right, none of them obvious:

- SILENT ONCE THE HULL HAS COLLAPSED. The peel drives the fraction straight to
  zero, so without the gate a one-shot kill from full would play a damage
  warning underneath the ship coming apart. `Without<StructuralCollapseMarker>`.
- NOT AT BIRTH. A root mid-spawn has no sections counted yet and reads as zero
  of zero, which is every ship screaming the frame it appears. Guarded on
  `health.max > 0`.
- THE LATCH IS KEYED BY THE SHIP, not a bare flag, because a new scenario is a
  new hull and a leftover latch would swallow its first warning. The rearm band
  (`WARN_HULL_REARM_MARGIN`, 0.05 over the threshold) is dead code in a fight -
  nothing repairs a hull today - and is there so that the day something does,
  an alarm sitting exactly on its line cannot chatter.

Level 0.32, which measures -35.3 dBA: about 4 dB over the threat alarm by the
same step that one takes over the lock chirp, and stopping just under a section
failing. That ceiling is the rule the cue is built to - the instrument
reporting the hull coming apart must not be louder than the hull coming apart.

With this, `impact_pierce` and `impact_explosive` are the only two rendered
files nothing plays, and they wait on the table below.

## The impact table (2026-09-01)

The owner's idea, landed. `impact_sound` sat on the thing being HIT, so the
game modelled half of "what on what" and had no notion of the other half - a
slug and a penetrator on the same plate made the same noise. It is now a table:
content authors `(damage type, material) -> sound`, and a target only says what
it is MADE of.

The queued design said TWO tables, and that was wrong. It read a body-on-body
ram as symmetric - an unordered material pair needing its own mechanism. But
`on_impact_collision_deal_damage` (`nova_gameplay/src/integrity/core.rs`)
already routes every ram through `apply_damage(.., DamageType::Kinetic, at)`,
so a ram IS `(Kinetic, material)` under the asymmetric table and was already
audible. A second mechanism answering a question the first answers is the
compatibility machinery the conventions forbid. ONE table.

What it is made of:

- `SurfaceImpact { entity, kind, at }` in `nova_gameplay::damage`, emitted by
  `apply_damage` exactly when the caller knew WHERE the hit was - the same
  condition that earns a crater, so a scripted `destroy` and a test rig stay
  silent. `apply_blast_damage` emits it too, at the blast centre, which is what
  makes `Explosive` reachable at all.
- It does NOT propagate, and that deleted a guard rather than adding one. The
  old cue rode `HealthApplyDamage` up `ChildOf` to the ship root and had to
  filter the hops back out with `damage.entity != original_event_target()`. It
  also carries the CONTACT POINT, so the cue plays where the round bit instead
  of at the struck body's origin.
- `SurfaceMaterial(String)` - open strings, not an enum, for the reason style
  ids are: a mod adds ice or ceramic by naming it. `None` on a section is
  `"hull"` and on an asteroid is `"rock"`, because the field exists so a mod
  can say otherwise, not so the base catalog can restate what everything
  already is.
- `ImpactSoundConfig` + `Content::Impact` + `GameImpacts`, one content item per
  ROW so a mod re-voices one pair by re-declaring that row's id alone.
- The base table is four rows in `assets/base/impacts/base.content.ron`: three
  defaults, one per damage type, plus `(Kinetic, "rock")`. The lookup falls
  back exactly once - to the damage type's default - and is otherwise silent.
  A material with no `Pierce` row does not borrow its own `Kinetic` row.

`impact_sound` is DELETED, not kept as an override, on both `BaseSectionConfig`
and `AsteroidConfig` - a `**(breaking)**` content-format change. That closed
the last two unreachable renders: `impact_pierce.wav` and
`impact_explosive.wav`. Every file in `assets/base/sounds/` is now authored.

`ImpactDestroySounds` lost its impact half and is `DestroySound`, a newtype - a
struct with only a destroy field could not keep that name.

Two rocks the wiring pass missed, found here and fixed: the editor's placed
asteroids and `wfc_arena`/`system_turret_gunnery`'s rocks were still authoring
`explosion.wav` for their destruction, the hull's voice on stone. They author
`destroy_rock.wav` now.

Agreed and unchanged: `destroy_sound` stays per section and object (a
destruction has no second party to key on), `fire_sound` / `dry_fire_sound`
stay per turret, `detonation_sound` stays per warhead.

### Open: impact_rock is 4.6 dBA hot

Measured after the table landed, loudest-50 ms A-weighted at unity:

    impact.wav            -34.3 dBA
    impact_pierce.wav     -34.7 dBA
    impact_explosive.wav  -34.0 dBA
    impact_rock.wav       -29.7 dBA

The three defaults are within 0.7 dB of each other. The rock is 4.6 dB over
them, and all four play at the one `IMPACT_VOLUME`, so a hit on stone is
audibly louder than the same round on plate. That is not the table's doing - it
arrived with `d33bd9a9`, when asteroids stopped borrowing the hull's voice, and
nobody has flown it yet.

Not fixed here, deliberately. The lever is in the RENDER, not a constant: the
rock's saturated body sets its own peak, so peak normalization leaves its
sustained level high where the kinetic hit's bright strike pulls everything
else down. Backing the body's saturation drive from 1.8 to about 0.5 lands it
on -34, and that is a change to how the cue SOUNDS - it is the "broad dull
body" the recipe is built around. Measured, not auditioned; the owner should
hear it before it is reshaped.

## Open: the release note wants the before and after

The owner's idea. `audition.html` is a REVIEWER's page - 4.4 MB, every cue
base64'd inline, A/B takes against the neighbour each cue has to be
distinguishable from. A release note wants a different thing: a handful of
old-against-new pairs a reader can click.

Nothing needs preserving for it. The pre-pass set is in git at `68a2cb38`, and
the pass changed 17 files, left the 10 NOVA OS files byte-identical, and added
27 new ones. So "delete the old sounds" is already done - every replaced file
was overwritten at its own path, and nothing on disk is an orphan. The files
that nothing plays yet are the `[hook]` cues waiting for observers, not
leftovers - two of them now, both blocked on the material table.

If it is built, the sounds should ship as FILES under `web/src/assets/`, the
way the video loops already do, not inlined - 44 WAVs are 2.2 MB on disk and
base64 adds a third to that for nothing.

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
