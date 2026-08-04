# Drawer ship-status / damage section - DEFERRED from v0.9.0 (rides critical-damage model)

- PRIORITY: 0
- TAGS: backlog, spike, feature, ui, hud
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Goal

The drawer's SHIP STATUS / damage section. DEFERRED from v0.9.0 by the spike
(Spike: tasks/20260721-211512/SPIKE.md, option B2) because it overlaps the
STRETCH critical-damage model (20260722-092320) and would risk double-work if
that lands differently. Sequence AFTER the critical-damage model settles.

Scope (direction-level; /plan breaks into steps at pickup):

- A ship status / section-damage readout inside the drawer, reading section
  health (SpaceshipSectionSystems). Reflect the critical-damage model
  (weapons+thrusters destroyed => combat-dead) once 20260722-092320 defines it.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). Slots into the drawer's
  section framework (shell task 20260724-102304). Backlog; depends on
  20260722-092320 (critical-damage model).


## Dropped

- REASON: likely superseded. Requested ship-status/damage surface now exists through NOVA OS ship blips, integrity bars, detailed section view, and repair commands.
