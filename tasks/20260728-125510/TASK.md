# NOVA OS ship app: recenter the orbit camera on the selected section

- STATUS: OPEN
- PRIORITY: 29
- TAGS: v0.9.0,feedback,feature,ui,hud,gameplay

## Story

As a player using the `ship` app, when I select a section (click a blip or press
`[`/`]`), I want the orbit camera to recenter on that section, so I orbit around
the thing I am inspecting instead of always around the whole-ship centroid.

Playtest verdict (2026-07-28) on the landed legibility change (`20260728-115435`):
"the cubes look really good, I like a lot the idea of having a smaller cube inside
a frame that WORKS" - keep the fill+outline wireframe. Follow-up request: "add
some recentering mechanic because you should orbit around the section that you
selected."

## What it should do

- On selection change, move the orbit CENTER (`ShipOrbit.center`) from the
  whole-ship centroid to the selected section's local translation, so Q/E/R/F +
  drag orbit around the selected block.
- Prefer a smooth ease to the new center over an instant snap (tune later); keep
  the current radius/zoom unless it needs adjusting to keep the section framed.
- `T` (reset) should still restore the default framing; decide whether reset also
  returns the center to the whole-ship centroid (likely yes).
- Do not regress blip projection: blips project from section local positions in
  the scene frame - moving only the camera center is safe, but confirm the
  projection still lines up (`reused-render-pattern-verify-coordinate-frame`).

## Notes

- Orbit state is `ShipOrbit { theta, phi, radius, center }` in
  `crates/nova_gameplay/src/hud/nova_os_ship.rs`; `drive_ship_camera` builds the
  eye from `center + orbit_eye(...)`. Selection lives in `ShipRuntime.selected`.
- The initial `center` is the section centroid, set in `manage_ship_scene`.
- Follows `20260726-115339` / `20260728-115435`. Sibling of the blip-overlay
  minimize task (`20260728-125514`) and the side-inspector-panel task
  (`20260728-115430`).
