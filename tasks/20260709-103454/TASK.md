# Diegetic flight instruments: in-world autopilot/maneuver UI

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.5.0, hud, autopilot, spike

Spike: docs/spikes/20260709-103324-diegetic-autopilot.md

## Goal

The user wants the autopilot to feel in-world, not like debug text: an
instrument/panel treatment showing the engaged maneuver, the flip point, the
deceleration curve/ETA, and the destination - designed UI/UX work, not just
readouts. Needs the shared HUD screen-projection substrate
(docs/spikes/20260708-165647-weapons-hud.md) and the autopilot mechanics
(20260709-103434) to exist first; direction-level until then. `/plan` breaks
it into steps when picked up, likely starting with its own design spike on
the diegetic language (holo panel vs cockpit vs projected volumes).

## Notes

- Parked at v0.5.0/p0 per the roadmap spike's rule (this sprint is combat
  feel; the arc gets re-prioritized when v0.5.0 is planned).
- Depends on: 20260709-103434 (autopilot), HUD phase 1 substrate.
