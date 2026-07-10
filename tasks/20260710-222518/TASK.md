# Allow zooming out further while orbiting to visualize the area

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,camera,ux


## Goal

Playtest request (user, 2026-07-10): while orbiting (AP ORBIT engaged)
the camera should be able to zoom out further than the normal flight
limit, so the orbited body, the ring and the surrounding area read as a
whole. Parked orbit is a "survey" posture - the current zoom range keeps
the body filling the screen.

## Notes

- Scope: extend the zoom-out range while the ship's autopilot action is
  Orbit (or possibly whenever inside an SOI - decide in /plan by feel);
  restore the normal range on breakout, smoothly (see the related snap
  task 20260710-222517 for the smoothing reference).
- Relevant code: camera_controller.rs (zoom limits), flight.rs
  (AutopilotAction::Orbit as the state to key off).
- Filed together with 20260710-222517; both touch the camera controller -
  consider one cycle for the pair.
