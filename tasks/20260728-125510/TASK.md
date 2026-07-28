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

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Design

Orbit center follows the selected section with a frame-rate-independent
exponential ease; `T` re-frames the whole ship. All are in the same
`ShipOrbit`-local scene frame, so no projection change is needed (blips
reproject from `cam_gt` each frame - `reused-render-pattern-verify-coordinate-frame`).

- Extend `ShipOrbit` with `center_target: Vec3`, `center_home: Vec3`, and
  `centered_on: Option<Entity>`. `manage_ship_scene` inits all three centers to
  the whole-ship centroid and `centered_on` to the default selection, so the app
  still OPENS framed on the whole ship (the default selection is treated as
  already centered at home, not chased on frame 1).
- `ship_input` reconciles: when `runtime.selected != orbit.centered_on`, set
  `center_target` to that section's `local.translation` and record `centered_on`.
  This single funnel covers `[`/`]`, blip clicks, and the default selection -
  each selection caller only sets `runtime.selected`.
- `T` (reset) sets `center_target = center_home` AND `centered_on = selected`,
  so the whole-ship reframe STICKS (the reconcile won't chase the still-selected
  section back) until the player picks a section again. Load-bearing fork the
  task flagged as "likely yes" - recorded in DECISION.md.
- `drive_ship_camera` gains `Res<Time>` and eases `orbit.center` toward
  `center_target` via `center.lerp(target, 1 - exp(-k*dt))`, then builds the eye
  and look-at from the eased center. Radius and `T`'s existing theta/phi reset
  are unchanged; `T` does NOT reset the zoom (out of scope).

## Steps

- [ ] Add `center_target`, `center_home`, `centered_on` to `ShipOrbit`; init them
      in `manage_ship_scene` (centers = centroid, `centered_on` = default selection).
- [ ] Add a `SHIP_CENTER_EASE` constant and ease `orbit.center` toward
      `center_target` in `drive_ship_camera` (add `Res<Time>`, `&mut ShipOrbit`).
- [ ] Reconcile `center_target`/`centered_on` from `runtime.selected` in
      `ship_input`; make `T` retarget to `center_home` and consume the selection.
- [ ] Live-tree tests (off-origin fixture): selecting a non-default section
      retargets `center_target` and, after N `drive_ship_camera` frames,
      `orbit.center` approaches it (fails if the ease is a no-op); `T` returns
      `center_target` to `center_home` and the reconcile does not snap it back.
- [ ] Verify blips still line up (screenshot example AppExit::Success) and run
      the check suite.

## Definition of Done

1. Selecting a section (via `[`/`]` or a blip click) eases the orbit center onto
   that section's local position; the app still opens framed on the whole ship.
   (test: `ship_input` + `drive_ship_camera` live-tree test in nova_os_ship.rs)
2. `T` re-frames the whole ship and the reframe sticks until re-selection.
   (test: T-reset live-tree test)
3. Blip projection unregressed. (cmd: `BCS_AUTOPILOT=1 cargo run --example
   screenshot_nova_os --features debug` exits `AppExit::Success`)
4. Check suite green. (cmd: `cargo check -p nova_gameplay`)
5. Playtest: orbiting recenters on the selected section; T reframes the ship.
   (manual: owner confirms in a run)
