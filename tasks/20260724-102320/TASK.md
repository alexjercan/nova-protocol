# Drawer 3D minimap / nav section (schematic orrery) - v0.9.0 STRETCH

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,stretch,spike,feature,ui,hud

## Goal

The drawer's 3D MINIMAP / nav section. v0.9.0 STRETCH (owner call, 2026-07-24):
the owner wants this in v0.9.0 but at the END, after the core drawer sections
land - "curious how it will look". Cut FIRST if Strand C runs long. A net-new
subsystem and the drawer's largest single unknown (Spike:
tasks/20260721-211512/SPIKE.md, option C), so it stays last in the drawer order.
Do not start until the shell (20260724-102304) exists.

Scope (direction-level; /plan breaks into steps at pickup):

- A plottable-contacts data model (player, gravity wells, objective targets,
  radar contacts - all enumerable from existing components).
- Recommended render: schematic ORRERY (option C2) - a lightweight proxy scene
  of blips rendered by a small dedicated camera to a texture, rotatable; reads as
  3D without a second pass over real geometry. Render mode is a swappable back
  layer, so a 2D top-down plot is a valid interim.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). Slots into the drawer's
  section framework (shell task 20260724-102304). v0.9.0 stretch - last in
  Strand C, cut first if the core drawer runs long.
