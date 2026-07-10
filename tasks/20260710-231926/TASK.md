# Diegetic flight status v1: rehome the bottom-left status text and delete it

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0,hud,ux,spike


## Goal

Replace the bottom-left flight status line (hud/flight_status.rs,
flight::flight_status_line) with diegetic presentation and delete it in
the same change. Spiked 2026-07-10; the user's questionnaire answers
fixed the direction:

- **Speed**: a numeric chip anchored to the player ship with an offset
  parking it just outside the velocity sphere (screen_indicator
  substrate). Always visible.
- **Mode + phase**: a ship-anchored chip showing verb and phase
  (`AP GOTO - BURN`) only while the autopilot is engaged; manual mode
  shows no chip (quiet HUD = manual). A family-wide shader tint
  reinforces it - that part is split out as task 20260710-234115.
- **Orbit radius**: a radius spoke holo while ORBIT is engaged - a thin
  world-space line (ribbon/ring visual language, unlit NAV_CYAN) from the
  well center to the ship, with the current radius as a chip riding it.
  The planned ring and its `r | v_circ` chip stay as-is.
- **Dropped without replacement**: the `GRAV <name>` coasting cue (the
  yellow gravity sphere carries it) and the standalone GOTO distance (the
  destination chip already shows distance, ETA, closing speed).

Deletion criterion: flight_status_line, its tests, and the bottom-left
text node go away here - the goal is REPLACEMENT, not addition. Check
whether the keybind hint cluster's docking position needs a nudge once
the line under it disappears.

## Notes

- Spike: docs/spikes/20260710-234019-diegetic-flight-status.md
- Substrate: hud/screen_indicator.rs (chips), hud/holo_instruments.rs
  (spoke language), hud/velocity.rs (sphere the speed chip parks beside),
  hud/maneuver_instruments.rs (existing ring + chips).
- Open questions parked for /plan: fixed-px vs projected-radius chip
  offset; spoke endpoint (well-to-ship first); hint cluster reflow.
