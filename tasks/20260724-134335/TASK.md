# Drawer open: hide the flight HUD + blur the background; keep the top status bar + lower-left keys visible; panels slide from both sides

- STATUS: CLOSED
- PRIORITY: 62
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

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
  load-bearing role but the panels/backdrop still layer above the dimmed bg.
- Files: hud/drawer.rs (backdrop, panels, slide), hud/mod.rs (HudVisibility +
  widget spawn), hud/readout.rs (status strip to keep), hud/keybind_hints.rs
  (lower-left cluster to keep clear).

## Decisions (2026-07-24 gate)

See DECISION.md in this folder for the full record. Load-bearing calls:

- BLUR: dropped. The written scope asked to ADD a scene blur; at the /flow gate
  the owner chose HEAVY GRAY ONLY - deepen the existing gray dim so the scene
  reads as an inert field, NO camera post-process. bevy 0.19 has no UI
  backdrop-filter, so a real blur would mean a fullscreen post-process render
  node (WebGL2/wasm risk) or depth-dependent DoF; owner judged neither worth it
  this sprint. The "background blurry" acceptance item becomes "background gray
  enough that you do not notice the old UI is gone".
- LEFT PANEL: this task builds the left-panel SHELL + a titled placeholder
  section (slides from the left). Comms/flight-log CONTENT stays in task
  20260724-102309, which fills the existing shell.

## Steps

1. HUD hide on Drawer (hud/mod.rs). Add a `HudDrawerExempt` marker (Reflect).
   Fold `PauseStates::Drawer` into `apply_hud_visibility`: hide every tiered
   root + screen indicator while the drawer is open UNLESS it carries
   `HudDrawerExempt`; the one-shot restore branch also fires on
   `pause.is_changed()` so a close restores in one frame. Tag the `readout`
   root and the `keybind_hint_cluster` with `HudDrawerExempt`.
2. Keep status strip + keys readable above the dim (hud/mod.rs, readout.rs,
   keybind_hints.rs). Add a `DRAWER_EXEMPT_Z` (> backdrop z) `GlobalZIndex` on
   the readout root and the keybind cluster so the deepened backdrop cannot dim
   them.
3. Deepen the gray backdrop (hud/drawer.rs). Raise `DRAWER_BACKDROP_ALPHA` so
   the frozen scene reads as an inert gray field (no blur). Keep the same
   transparent gray hue the owner likes.
4. Dual-side panels + reserved top strip (hud/drawer.rs). Generalize
   `drive_drawer_slide` to drive BOTH a right panel (`right` offset, existing)
   and a new left panel (`left` offset) from `DrawerOpenness`. Inset both
   panels' `top` below the status strip (reserve it). Spawn the new LEFT panel
   in `setup_drawer` with a placeholder "COMMS / LOG" section, its `bottom`
   inset (and/or the keys' higher z) so it never covers the lower-left keybind
   cluster; `remove_drawer` despawns both panels.
5. Verify: `cargo check -p nova_gameplay`, `cargo fmt --check`, the new tests,
   and a `nova_probe` playable run.

## Definition of Done

1. Opening the drawer hides the flight HUD except the status strip + keybind
   hints; closing restores them
   (test: `drawer_hides_flight_hud_except_exempt`, restore-on-close test).
2. `readout` + keybind cluster carry `HudDrawerExempt` and a `GlobalZIndex`
   above the backdrop (test: exempt-tag + z-order assertions).
3. `drive_drawer_slide` drives both left and right panel offsets from openness;
   left panel bottom clears the keybind cluster region (test: dual-slide).
4. Backdrop alpha deepened; scene reads as a gray inert field, no post-process
   (manual: owner opens the drawer in-game).
5. Build green: `nix develop --command cargo check -p nova_gameplay` and
   `cargo fmt --check` clean (cmd).
6. Playable probe OK: `cargo run -p nova_probe -- run playable` (probe/manual).
