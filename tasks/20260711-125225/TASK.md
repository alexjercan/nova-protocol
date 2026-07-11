# Camera jumps at high speed give the controls a twitchy feel

- STATUS: OPEN
- PRIORITY: 78
- TAGS: v0.5.0,camera,bug

## Goal

Playtest (user, 2026-07-11, second session - AFTER the HUD/indicator fixes
landed and the text chips read clean): "sometimes the camera jumps at high
speeds and it gives that twitchy feeling to the spaceship controls".
Discrete JUMPS, not continuous drift - distinct from the zoom-out cap
request (20260711-121711).

## Steps

- [ ] Verify which build the report is from: the chase-camera ordering pin
      (ChaseCameraSystems::Sync before TransformSystems::Propagate, landed
      5ba0e3c) fixes a per-build ordering coin flip whose symptom is
      exactly "the whole frame renders one camera step late". If the
      session predates it, have the user re-test on master first.
- [ ] If jumps persist: hunt discrete camera discontinuities headlessly -
      frame-step a cruising + maneuvering ship at 250-300 u/s and record
      the camera Transform per frame; assert/inspect max per-frame camera
      travel vs the smoothed expectation. Candidate mechanisms, in order:
      the survey/mode rig switching offsets discretely
      (update_camera_rig), the speed-zoom mapping stepping with speed
      brackets, lerp_and_snap's snap branch triggering at range
      (bcs meth/lerp.rs snaps when within EPSILON - at large offsets the
      threshold may misbehave with f32 at speed), and double-tick frames
      (64 vs 60 Hz beat) making the anchor advance 2 ticks in one frame
      while the camera eases per-frame.
- [ ] Fix at the mechanism; regression in the style of
      indicator_projects_with_the_frames_final_camera_pose (per-frame
      camera travel bounded/smooth at speed, with delivery guards).
- [ ] Coordinate with 20260711-121711 (zoom cap): if both land in one
      camera cycle, keep them separate commits - a cap is tuning, a jump
      is a bug.

## Notes

- Related: docs/spikes/20260711-103527-twitching-family-two-clocks.md
  (falsified: physical hull wobble - the hull is provably steady, see
  tasks/20260711-121701), tasks/20260711-121711 (zoom cap + decel zoom
  slew), tasks/20260711-125227 (feel-smoothing spike, queued last).
- The "twitchy CONTROLS feel" wording suggests the jump lands between
  input and what the player sees - consistent with camera-side, not
  physics (the input rig is camera-INDEPENDENT for rotation, verified in
  20260711-121701).
