# Drawer 3D minimap / nav section (schematic orrery) - DEFERRED from v0.9.0

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,spike,feature,ui,hud

## Goal

The drawer's 3D MINIMAP / nav section. DEFERRED from v0.9.0 by the spike to keep
the release from ballooning (Spike: tasks/20260721-211512/SPIKE.md, option C). A
net-new subsystem - the drawer's largest single unknown. Only pull into a
release once the drawer shell exists and the owner wants it.

Scope (direction-level; /plan breaks into steps at pickup):

- A plottable-contacts data model (player, gravity wells, objective targets,
  radar contacts - all enumerable from existing components).
- Recommended render: schematic ORRERY (option C2) - a lightweight proxy scene
  of blips rendered by a small dedicated camera to a texture, rotatable; reads as
  3D without a second pass over real geometry. Render mode is a swappable back
  layer, so a 2D top-down plot is a valid interim.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). Slots into the drawer's
  section framework (shell task 20260724-102304). Backlog until the owner pulls
  it into a release.
