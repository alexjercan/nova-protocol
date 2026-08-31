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
