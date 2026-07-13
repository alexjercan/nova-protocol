# Smooth the camera when autopilot hands back to manual (no snap)

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,camera,ux,bug


## Goal

Playtest finding (user, 2026-07-10): switching from automatic mode
(autopilot engaged) back to manual snaps the camera way too suddenly.
The camera already knows how to blend - switching between free-look and
normal mode, or into combat mode, eases smoothly. Give the
autopilot-to-manual handback the same smoothing.

## Steps

- [x] Add a `CameraHandbackBlend { from: Quat, elapsed: f32 }` component
  in crates/nova_gameplay/src/camera_controller.rs with a
  `HANDBACK_BLEND_SECONDS` constant (~0.45, playtest knob) and a pure
  eased-slerp helper (`handback_anchor_rot(from, to, elapsed)` with a
  smoothstep ease; unit-tested).
- [x] In `on_autopilot_disengaged`, capture the active rig's CURRENT
  `PointRotationOutput` (what the camera was looking along) BEFORE the
  re-seed, and insert the blend component on the camera controller
  entity. The re-seed itself stays instant - the PD's no-lurch contract
  is untouched; only the camera's view of the discontinuity is smoothed.
- [x] In `update_chase_camera_input`, when a blend is present: tick
  `elapsed` with `Time`, set `anchor_rot =
  handback_anchor_rot(blend.from, live rig output, elapsed)`, remove the
  component when done. Mouse input during the blend keeps moving the
  live output; the blend converges to wherever the player is looking.
- [x] Tests: pure helper (t=0 -> from, t>=duration -> to, monotonic ease);
  app-level through the real observer + input system: disengage keeps
  anchor_rot continuous (pre-disengage rig direction, NOT the hull quat)
  on the first frame, and force-expiring the blend lands anchor_rot on
  the live rig output with the component removed.
- [x] Run camera_controller + flight tests and `cargo check --workspace
  --examples`.
- [x] Docs: docs/retros/20260710-camera-handback-blend.md; close TASK.md.

## Notes

- Reference behavior: the existing mode transitions with smoothing (free
  look <-> normal, combat mode) in camera_controller.rs - reuse the same
  blend mechanism/curve rather than inventing a new one.
- Likely seam: disengaging re-seeds the mouse rig from the ship's current
  facing "so nothing lurches" (flight.rs module doc, camera_controller.rs)
  - the re-seed itself is instantaneous; the smoothing should wrap it.
- Covers every disengage path: Z, any flight input, capability loss, GOTO
  completion at a well-less target (well targets now park into ORBIT and
  keep the autopilot).

## Resolution

Implemented per the Steps: the ship-side instant re-seed is untouched
(no-lurch contract), the camera bridges the discontinuity with an eased
slerp on anchor_rot (its only consumer is the chase camera - verified),
gated on the normal rig being the active one. One test-writing lesson:
Quat::angle_between reads ~7e-4 rad for effectively-equal quats (acos
amplification), so the assertions use 2e-3 epsilons with a comment.

Checks: camera_controller 9, gameplay lib 344, cargo check --workspace
--examples clean. Full suite and clippy left to CI per policy.
