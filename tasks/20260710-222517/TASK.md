# Smooth the camera when autopilot hands back to manual (no snap)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,camera,ux,bug


## Goal

Playtest finding (user, 2026-07-10): switching from automatic mode
(autopilot engaged) back to manual snaps the camera way too suddenly.
The camera already knows how to blend - switching between free-look and
normal mode, or into combat mode, eases smoothly. Give the
autopilot-to-manual handback the same smoothing.

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
