# HUD text anchored to moving objects twitches (e.g. velocity on the ship)

- STATUS: OPEN
- PRIORITY: 82
- TAGS: v0.5.0, hud, bug

## Goal

Playtest bug (user, 2026-07-10): HUD text anchored to moving objects
twitches - most visibly the velocity/speed text written at the ship, and
generally "things that fly". Diagnose and fix the jitter.

## Notes

- Likely root-cause family: fixed-tick physics positions vs per-frame
  render sampling (avian Position updates in FixedUpdate; UI anchors
  sampling un-interpolated transforms alias against camera motion).
  Investigate together with 20260710-231930 (bullets twitch) and
  20260710-231931 (ship twitchy at speed) - probably one interpolation
  story, and with 20260710-231929 (turret crosshair) if the anchor math
  shares code.
- Relevant: screen_indicator anchoring (Entity vs Point), the
  flight-status/instrument text spawn sites, ChaseCamera smoothing
  (camera moves per-frame while anchors move per-tick).
