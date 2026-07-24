# Bug: Tab drawer must render on top of the flight HUD (z-order)

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.9.0,bug,ui,hud

## Goal

Playtest feedback (owner, 2026-07-24): when the Tab ship-computer drawer opens
it must render ON TOP of everything else. Right now the flight HUD - notably the
compact top-right objectives panel text - still draws over the drawer panel, so
the drawer looks like it is behind the HUD instead of a modal surface above it.

This is a shell/z-order concern from 20260724-102304 (the drawer shell), NOT the
diegetic-objectives task (20260721-211520).

Scope (direction-level; /plan breaks into steps at pickup):

- Give the drawer backdrop + panel a global stacking context above all flight
  HUD widgets (Bevy 0.19: `GlobalZIndex` on the drawer root/backdrop; a plain
  `ZIndex` only reorders within one stacking context, which is why the top-right
  objectives panel currently wins). Pick a z tier above the HUD chrome.
- Verify the backdrop dims the whole HUD and the panel sits above the compact
  objectives panel, comms panel, markers, readouts - the drawer is a modal.
- The tab handle's z can stay with the HUD (it is chrome); only the OPEN
  surface must rise above.

## Notes

- From the drawer shell (20260724-102304, LANDED c13143d4). Files:
  crates/nova_gameplay/src/hud/drawer.rs (setup_drawer spawns backdrop + panel),
  crates/nova_gameplay/src/hud/mod.rs (HUD widget spawn order). The drawer parts
  intentionally carry NO HudTier (modal axis); z-order is the remaining gap.
- Owner also confirmed (2026-07-24) they like the drawer transparency + slide
  animation - keep those.
