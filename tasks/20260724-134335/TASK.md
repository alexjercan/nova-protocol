# Drawer open: hide the flight HUD + blur the background; keep the top status bar + lower-left keys visible; panels slide from both sides

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.9.0,feature,ui,hud

## Goal

Playtest rework (owner, 2026-07-24): change how the drawer OPENS. Instead of a
panel stacked over the still-visible flight HUD, the drawer should HIDE the flight
HUD and blur the background, so the old UI does not fight the drawer content for
readability.

Owner feedback driving this:
- HIDE the flight UI elements when in Drawer mode, and make the background blurry +
  gray enough that you do not notice the old UI is gone (a present-but-dimmed HUD
  makes the drawer harder to read).
- Keep the gray transparent backdrop (owner likes it) and ADD a blur.
- The STATUS BAR (top strip, hud/readout.rs) should STAY visible - leave space at
  the top for it; treat it like a window-manager status bar, do not put drawer UI
  on it.
- The lower-left keybind hint buttons (hud/keybind_hints.rs, bottom:8 left:8) must
  NOT be overlapped/hidden by the drawer's left panel - keep the keys visible.
- The drawer is two side panels: the RIGHT panel (objectives, task 102304/D) slides
  in from the right; the LEFT panel (comms/flight-log, task 102309) slides in from
  the left.

Scope (direction-level; /plan breaks into steps at pickup):

- On entering PauseStates::Drawer, HIDE the flight HUD widgets (they already have a
  HudVisibility axis - reuse it, or a Drawer-scoped hide) and restore on close;
  EXCEPT the top status-bar strip (readout), which stays visible.
- Backdrop: keep the gray transparent dim, add a blur of the scene behind (bevy
  0.19 UI blur / a blur material or post-process on the backdrop region).
- Layout: reserve the top strip (no drawer UI in it); ensure the left panel does
  not cover the lower-left keybind hints (either the hints stay above the panel, or
  the panel is inset to clear them).
- Left panel slides from left, right panel from right (the shell's slide currently
  only does the right panel).

## Notes

- From the 2026-07-24 playtest. Builds on the shell (20260724-102304, LANDED) and
  the z-order fix (20260724-121541, LANDED) - hiding the HUD reduces the z-order's
  load-bearing role but the panels/backdrop still layer above the blurred bg.
- Files: hud/drawer.rs (backdrop, panels, slide), hud/mod.rs (HudVisibility +
  widget spawn), hud/readout.rs (status strip to keep), hud/keybind_hints.rs
  (lower-left cluster to keep clear).
