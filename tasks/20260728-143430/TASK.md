# Bug: LMB drag orbits and steals blip selection in map + ship apps - keep RMB-only orbit

- STATUS: CLOSED
- PRIORITY: 31
- TAGS: v0.9.0,feedback,bug,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Playtest feedback (2026-07-28): in both the NOVA OS `map` and `ship` apps,
dragging with the LEFT mouse button orbits the camera. But LMB is also the
click that SELECTS a blip (the `bevy_ui` `Button` widget fires `Activate` on a
Primary click). So a press-with-any-motion is read as an orbit drag: the camera
moves, the blip slides out from under the cursor, and the click never lands -
"it thinks you want to drag so it doesn't select". Keep only RMB for orbit-drag
so LMB is free to select.

## What it should do

- Orbit-drag responds to the RIGHT mouse button ONLY, in both apps.
- LMB press/click no longer orbits, so clicking a blip selects it reliably.
- Keyboard orbit (Q/E/R/F), wheel zoom, WASD pan, and `[`/`]` cycling are
  unchanged.

## Mechanism

Both input systems gate orbit-drag on `pressed(Right) || pressed(Left)`:

- `crates/nova_gameplay/src/hud/nova_os_map.rs:834` (`map` input system)
- `crates/nova_gameplay/src/hud/nova_os_ship.rs:1369` (`ship_input`)

Drop the `|| pressed(Left)` half in both, leaving RMB. Update the adjacent
"Mouse drag (LMB or RMB)" comments to say RMB only.

## Steps

- [x] Write failing live-tree tests first: run each input system with LMB held +
      a `MouseMotion` delta and assert the orbit angles do NOT change; with RMB
      held + the same delta, assert they DO change (pins both apps, both buttons).
      `ship_orbit_drag_is_rmb_only` + `map_orbit_drag_is_rmb_only`; both RED
      before the fix (LMB moved angles `(0.8,0.62)` -> `(0.656,0.716)`).
- [x] Remove `MouseButton::Left` from the orbit-drag condition in
      `nova_os_map.rs` and `nova_os_ship.rs`; fix the neighbouring comments.
- [x] Confirm the tests pass; run the check suite and the screenshot example.

## Definition of Done

1. Orbit-drag fires on RMB only, not LMB, in both apps. (test: LMB-no-orbit /
   RMB-orbits live-tree tests for `map` and `ship` input systems)
2. Check suite green. (cmd: `cargo check -p nova_gameplay`)
3. NOVA OS still boots. (cmd: `BCS_AUTOPILOT=1 cargo run --example
   screenshot_nova_os --features debug` exits `AppExit::Success`)
4. Playtest: LMB click on a blip selects it in both apps; RMB still orbits.
   (manual: owner confirms in a run)

## Notes

- Sibling of the ship recenter task (`20260728-125510`) and the blip-overlay
  minimize task (`20260728-125514`); same playtest round.

## Verification

- `cargo test -p nova_gameplay --lib orbit_drag_is_rmb_only`: 2 passed (RED
  before the one-line-per-file fix, GREEN after).
- `BCS_AUTOPILOT=1 cargo run --example screenshot_nova_os --features debug`:
  exit 0, harness reached `Playing` and shut down cleanly (AppExit::Success).
- DoD #4 (playtest: LMB selects, RMB orbits) stays a manual owner check.
