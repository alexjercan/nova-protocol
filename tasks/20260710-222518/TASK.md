# Allow zooming out further while orbiting to visualize the area

- STATUS: IN_PROGRESS
- PRIORITY: 45
- TAGS: v0.5.0,camera,ux


## Goal

Playtest request (user, 2026-07-10): while orbiting (AP ORBIT engaged)
the camera should be able to zoom out further than the normal flight
limit, so the orbited body, the ring and the surrounding area read as a
whole. Parked orbit is a "survey" posture - the current zoom range keeps
the body filling the screen.

## Steps

- [ ] Add a survey dolly to `update_camera_rig`
  (crates/nova_gameplay/src/camera_controller.rs): when the player ship's
  Autopilot action is `Orbit { plan: Some(plan) }`, scale the mode rig's
  offset so the camera distance reaches
  `(plan.radius * SURVEY_RING_FACTOR).clamp(base_len, SURVEY_MAX_DISTANCE)`
  - the ring radius is the "area" to visualize, so the dolly adapts to the
  orbit scale instead of a fixed zoom step. Focus offset stays the mode's
  own. Apply in Normal and FreeLook; Turret (combat) keeps its rig - a
  fight while orbiting should not be fought from survey range.
- [ ] Ride the existing `CAMERA_SMOOTHING` chase for the transition: the
  dolly changes only the per-frame offset target, so engage and breakout
  ease exactly like the mode switches do (no new smoothing code). The
  burn push composes on top unchanged.
- [ ] Constants with doc comments: SURVEY_RING_FACTOR (start 1.4 - the
  ring fills the frame with margin) and SURVEY_MAX_DISTANCE (start 250 -
  a cap so a giant well cannot dolly the camera into the skybox); both
  playtest knobs.
- [ ] Tests in the existing harness (burn_push test pattern): engaged
  Orbit-with-plan scales the offset to the expected length and returns to
  the base rig on disengage; Turret mode is unaffected while orbiting; a
  plan-less Orbit (first tick) keeps the base rig.
- [ ] Run the camera_controller module tests, input::ai NOT needed (no
  autopilot signature change) but hud + flight smoke: `cargo test -p
  nova_gameplay --lib camera_controller:: flight::` and `cargo check
  --workspace --examples`.
- [ ] Docs: docs/2026-07-10-orbit-survey-zoom.md; close TASK.md with the
  resolution.

## Notes

- Scope: extend the zoom-out range while the ship's autopilot action is
  Orbit (or possibly whenever inside an SOI - decide in /plan by feel);
  restore the normal range on breakout, smoothly (see the related snap
  task 20260710-222517 for the smoothing reference).
- Relevant code: camera_controller.rs (zoom limits), flight.rs
  (AutopilotAction::Orbit as the state to key off).
- Filed together with 20260710-222517; both touch the camera controller -
  consider one cycle for the pair.
