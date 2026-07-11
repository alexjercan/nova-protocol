# HUD visibility levels: tilde cycles ALL/MINIMAL/NONE

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,hud,ui,spike

Goal: let the player hide UI chrome for cinematic shots. Add a
`HudVisibility { All, Minimal, None }` resource; pressing tilde (grave, `~`)
cycles All -> Minimal -> None -> All. Each HUD module declares its tier
(e.g. edge indicators and keybind hints are All-only; velocity/flight
instruments survive Minimal; None hides everything including the status
bar).

There is no HUD master toggle today: HUD widgets spawn per-module when
PlayerSpaceshipMarker appears, so the plan needs a central visibility
gate (shared root node or a per-module Visibility system).

Notes:
- Spike: docs/spikes/20260711-180500-main-menu.md (Recommendation section
  covers the gesture choice: plain press-to-cycle, no hold gesture)
- Parent task: 20260711-174915
- Independent of the menu tasks; can land any time.

