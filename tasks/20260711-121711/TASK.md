# Cap the chase camera zoom-out and pivot distance at high speed

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.5.0,camera,tuning

## Goal

Playtest (user, 2026-07-11): at really high speeds (~300 u/s) the chase
camera zooms out - as the speed-based rig intends - but TOO much: when you
turn around, the pivot sits too far behind the spaceship. Requested
change: cap it - "the radius of the camera and the pivot distance should
not allow the camera to move the pivot too far away".

## Steps

- [ ] Locate the speed-to-zoom mapping in the camera rig
      (crates/nova_gameplay/src/camera_controller.rs, update_camera_rig /
      the ChaseCamera offset computation) and write down the current
      formula and its value at 300 u/s versus the comfortable range.
- [ ] Add explicit caps: a maximum camera radius (offset length) and a
      maximum pivot/focus distance behind the ship, as named tuning
      constants next to CAMERA_SMOOTHING and friends; the mapping
      saturates at the caps instead of growing with speed.
- [ ] Pick cap values from the playtest observation (comfortable at
      cruise, saturating before the ~300 u/s regime becomes unwieldy) and
      document the choice for the user to retune.
- [ ] Test pinning saturation: rig speed above the cap threshold, assert
      the produced offset/focus distances equal the caps (with a delivery
      guard that the mapping still GROWS below the threshold, so the cap
      cannot pass by the mapping being dead).

## Notes

- Filed from user feedback mid-flow (2026-07-11). Feel-tuning: expect a
  user retune round after landing; make the caps easy to tweak.
- The camera anchor/ordering wiring was just reworked in 20260710-231928;
  this task is about the RIG VALUES (offset magnitudes), not scheduling.
