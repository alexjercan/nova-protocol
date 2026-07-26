# NOVA OS map app: 3D minimap launched from the terminal - v0.9.0 STRETCH

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,stretch,spike,feature,ui,hud

## Goal

Build the `map` app for the one-screen NOVA OS drawer. The 3D minimap is still a
v0.9.0 stretch item, but it no longer lives in a permanent center drawer panel
between left logs and right objectives. Post-feedback direction is one inset
cockpit monitor: terminal commands either print inline output or launch an app
that swallows the same monitor until exited. This task owns the `map` app that
opens from the terminal command.

v0.9.0 STRETCH, still LAST in Strand C and cut first if the terminal OS core runs
long. The original minimap design came from
`tasks/20260721-211512/SPIKE.md` option C; the current direction is superseded by
`tasks/20260725-104330/SPIKE.md` and the visual PoC at
`examples/ui/nova_os_terminal_poc.html`.

Scope THIS SPRINT (direction-level; /plan breaks into steps at pickup):

- Add a `map` command in NOVA OS that launches a map app, replacing terminal
  scrollback inside the same monitor until the app is closed.
- Show a downsized 3D or schematic map view of the local game space. A small
  dedicated camera/render-to-texture path is acceptable if it proves clean in
  Bevy 0.19; a schematic proxy view is acceptable if the real render path runs
  long.
- Include placeholder markers for map contents: player ship, allies, enemies,
  asteroids and objective/area-of-interest markers. Simple proxy meshes/blips at
  scaled world positions are fine.
- Give the map app its own input ownership while active. WASD or similar camera
  controls belong to the app, not to the terminal prompt.
- Provide a way back to the terminal that matches the NOVA OS app runtime (for
  example app chrome close plus the chosen keyboard chord), without making Tab
  close the drawer from inside the terminal.

LATER (out of scope this sprint, captured for the reader): zoom levels, panning
to plan flights, richer marker filters, route planning, map-boundary gameplay and
ship commands that act from the map. The render mode stays a swappable back layer
so a 2D top-down plot is a valid interim if the 3D view runs long.

## Notes

- Original spike: `tasks/20260721-211512/SPIKE.md` (RECOMMENDED) captured the
  minimap options and recommended the schematic/proxy approach over rendering
  the real scene as the safer first path.
- Superseding feedback spike: `tasks/20260725-104330/SPIKE.md` changes the shell
  model from multiple drawer panels to one NOVA OS monitor. This task should
  plan against that newer spike.
- Depends on the NOVA OS app runtime task `20260726-115334`; do not implement
  the map as a separate permanent drawer panel.
- Visual reference: `examples/ui/nova_os_terminal_poc.html` shows the intended
  app takeover behavior with mocked map data. Treat it as a design target, not
  production code.
