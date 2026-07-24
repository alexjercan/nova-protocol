# Drawer CENTER: 3D minimap - downsized map view with WASD (placeholder this sprint) - v0.9.0 STRETCH

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,stretch,spike,feature,ui,hud

## Goal

The drawer's CENTER 3D minimap. RESCOPED by the 2026-07-24 playtest (owner): it
sits in the MIDDLE of the drawer, between the left (log/events) and right
(objectives) panels. THIS SPRINT it is a PLACEHOLDER - "a 3D downsized map view
with a WASD view or something like that". The full vision (zoom levels, flight
planning, moving through it) is much LATER.

v0.9.0 STRETCH, still LAST in Strand C and cut first if the strand runs long - it
is the drawer's largest single unknown (Spike: tasks/20260721-211512/SPIKE.md,
option C). Do not start until the shell (20260724-102304) and the drawer-open
rework (20260724-134335, which owns the center layout slot) exist.

Scope THIS SPRINT (direction-level; /plan breaks into steps at pickup):

- A downsized 3D VIEW of the game map in the drawer's center slot: a small
  dedicated camera rendering to a texture (bevy 0.19 render-to-texture), shown in
  the drawer.
- Placeholder MARKERS for map contents: asteroids, ships, enemy markers - simple
  proxy meshes/blips at scaled world positions (enumerable from existing
  components / a plottable-contacts model). Placeholder art is fine.
- A WASD (or similar) camera to look around the downsized view.

LATER (out of scope this sprint, captured for the reader): zoom levels, panning to
plan flights, richer markers, interaction. The render mode stays a swappable back
layer so a 2D top-down plot is a valid interim if the 3D view runs long.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). RESCOPED to the center
  placeholder (downsized 3D view + WASD) by the 2026-07-24 playtest; full
  planning/zoom deferred. Slots into the drawer's center layout slot owned by
  20260724-134335. v0.9.0 stretch - last in Strand C, cut first if it runs long.
