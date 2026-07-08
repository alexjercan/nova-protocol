# Camera snaps to origin for one frame when switching camera modes

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.4.0, bug, camera

Reported in scene mode (playing the game): switching between camera modes (combat / normal /
freelook) makes the camera snap to the origin for one frame before it settles into the right
mode.

## Root cause

`sync_spaceship_control_mode` (nova_gameplay `camera_controller.rs`) re-inserted a whole new
`ChaseCamera` component on every mode change to change the camera's offsets. Re-inserting fires
bcs's `On<Insert, ChaseCamera>` observer (`initialize_chase_camera`), which *unconditionally*
inserts `ChaseCameraInput::default()` - resetting the camera anchor (`anchor_pos`) to the origin.
With the chase camera's default smoothing of `0.0` (snap, no lerp), `chase_camera_update_state_system`
then places the camera at `origin + offset` that frame. The next frame,
`update_chase_camera_input` writes the ship position back into the anchor and the camera jumps to
the correct spot - hence the visible one-frame origin snap.

## Fix

Mutate the existing `ChaseCamera` in place instead of re-inserting it. `sync_spaceship_control_mode`
now takes `Single<&mut ChaseCamera, ...>` and sets `offset` / `focus_offset` directly. A `&mut`
mutation does not fire `On<Insert, ChaseCamera>`, so `ChaseCameraInput` (the anchor) and
`ChaseCameraState` (the smoothing state) are left untouched, and there is no origin frame. Re-
inserting a whole component just to change two fields was wasteful anyway.

## Steps

- [x] Diagnose: mode switch re-inserts `ChaseCamera` -> bcs `initialize_chase_camera` resets
      `ChaseCameraInput` to the origin -> one-frame snap.
- [x] Mutate `ChaseCamera.offset` / `focus_offset` in place in `sync_spaceship_control_mode`
      instead of re-inserting the component.
- [x] Regression test (`switching_camera_mode_keeps_the_anchor_off_origin`): with a ship far from
      the origin, a mode switch keeps the anchor and retunes the offset. Verified it fails on the
      old code (anchor -> `(0,0,0)`) and passes on the fix.
- [x] Green: `cargo clippy --workspace --all-targets`, `cargo test --workspace` (59 nova_gameplay
      incl. the new test; examples_smoke under Xvfb).

## Notes

The underlying trigger is a bcs behaviour (`initialize_chase_camera` resets `ChaseCameraInput`
on every insert, not just the first). Fixing it nova-side by mutating in place is both correct and
leaner than re-inserting; a future bcs change could also guard that observer (only add
`ChaseCameraInput` when absent, as it already does for `ChaseCameraState`), but that is a cross-
repo change and not needed here.
