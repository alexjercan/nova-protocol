# Diegetic flight instruments: in-world autopilot/maneuver UI

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.5.0, hud, autopilot, spike

Spike: docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md
(design language decided there; original ask from
docs/spikes/20260709-103324-diegetic-autopilot.md)

## Goal

Maneuver instruments v1, in the hybrid language the 2026-07-10 spike
decided (3D world-space for spatial geometry, projected chips for
numbers - the split the velocity sphere + indicator substrate already
use): (i) enrich the destination indicator with ETA, closing speed and
standoff distance from the arrival rule; (ii) a flip-point marker - a
Point-anchored indicator where `v_allowed(d)` says the flip happens,
labeled with seconds-to-flip; (iii) the ORBIT ring as the first
world-space holo element (3D line loop at `OrbitPlan { radius, normal }`,
velocity-sphere visual family) with the r/v_circ chip anchored to it.
The ring deliberately pilots the holo language on the simplest geometry
before the ribbon/shell expansion (task 20260710-174629). Direction-level:
/plan owns the steps.

## Notes

- User priority (2026-07-10): this is the most important part of the HUD.
- Followed by: 20260710-174646 (keybind hints - the cluster docks with
  these instruments), 20260710-174629 (holo expansion - after the ring).
- All maneuver data is already computed per tick by autopilot_system
  (flight.rs): arrival rule, Autopilot phase, OrbitPlan, DominantWell.
  Nothing new to simulate, only to surface.
- Prerequisites all shipped: autopilot verbs (STOP/GOTO/ORBIT), the
  screen-indicator substrate, the flight-status line.
