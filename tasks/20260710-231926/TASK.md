# Diegetic flight status: replace the bottom-left status text (spike first)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0,hud,ux,spike


## Goal

Playtest (user, 2026-07-10): the bottom-left flight status line packs too
much information and its meaning is opaque to anyone who does not know
the project. Replace it with diegetic, in-world presentation and ideally
delete the status line entirely. Ideas from the user to weigh in a
/spike:

- Orbit radius as a thin line from the ship (or well) with the number on
  it, in-world.
- Speed shown near the velocity direction shader (close to the sphere
  widget, but still readable as UI).
- Flight mode (MAN/AP STOP/GOTO/ORBIT) as text on or near the ship
  itself, possibly via a shader treatment in the same family as the
  velocity/gravity spheres.

## Notes

- Spike first (/spike): brainstorm the diegetic language, inventory what
  the status line currently encodes (mode, verb, phase, GRAV state,
  speed, distance readouts) and where each piece should live instead;
  the existing instruments (velocity/gravity spheres, maneuver chips,
  orbit ring, keybind cues) are the vocabulary to extend.
- Related substrate: hud/flight_status.rs (the line to retire),
  hud/velocity.rs (the widget family), hud/maneuver_instruments.rs
  (chips), screen_indicator (anchoring).
- The goal is REPLACEMENT, not addition: the status line goes away when
  its last consumer is rehomed.
