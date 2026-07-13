# Orbit survey zoom

Task: tasks/20260710-222518 - User request: while orbiting, the camera
should pull out so the orbited body, the ring and the surrounding area
read as a whole instead of the hull filling the screen.

## What changed

One seam in `update_camera_rig` (crates/nova_gameplay/src/
camera_controller.rs): while the player ship's autopilot flies a PLANNED
orbit (`Orbit { plan: Some(_) }`), the mode rig's offset is stretched
along its own direction so the camera distance reaches
`plan.radius * SURVEY_RING_FACTOR` (1.4), capped at
`SURVEY_MAX_DISTANCE` (250) and never closer than the mode's own rig.
The ring radius IS the area to visualize, so the dolly adapts to the
orbit scale - a 60u rock ring and a 200u giant ring both frame
correctly - instead of a fixed zoom step.

Design decisions:

- No new smoothing code: the dolly only changes the per-frame offset
  TARGET, and the chase camera's existing `CAMERA_SMOOTHING` eases the
  transition - engage and breakout blend exactly like a mode switch (the
  reference the user pointed at). The burn push composes on top
  unchanged.
- Turret (combat) mode keeps its own rig even while orbiting: a fight is
  not fought from survey range. Normal and FreeLook both get the dolly
  (FreeLook is the natural "look around the area" posture).
- No dolly on the plan-less first orbit tick, other verbs, or manual
  flight - `survey_scale` is a pure helper returning 1.0 for all of
  those, unit-tested.
- Both constants are playtest knobs with doc comments.

## Verification

`survey_scale` unit test (reach, cap, no-dolly-in, all 1.0 cases) plus an
app-level test through the real rig system: engage planned orbit ->
offset stretches to ring * factor along the rig direction; switch to
Turret -> combat rig; breakout -> base rig. camera_controller 6, flight
57, gameplay lib 341, `cargo check --workspace --examples` clean.

## Notes

- The related camera task 20260710-222517 (smooth the autopilot-to-manual
  handback snap) is about the MOUSE RIG re-seed, not the chase offset;
  this change neither fixes nor worsens it.
