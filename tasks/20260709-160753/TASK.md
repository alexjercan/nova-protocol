# Camera twitch when flying: ship transform stair-steps at fixed ticks under the smoothed camera

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0,bug,camera,handling

Playtest report on the flight-feel retune (20260709-095043): the camera
weight feels right, but the view "twitches" when moving. User's hypothesis
(from a similar old issue): a transform syncing problem - physics updates,
then the camera updates, lagging a frame behind.

Working theory: avian writes the ship's Transform only on 64 Hz FixedUpdate
ticks, so at render rate the ship moves in stair-steps. With the old rigid
camera (smoothing 0.0) the camera was a pure function of the ship transform -
both stepped together, no relative motion, no visible twitch. With smoothing
0.15 the camera eases at render rate against a stair-stepping anchor: the
step shows up as ship-vs-camera relative jitter, worst at speed. Turret
bullets already carry avian's `TransformInterpolation` for exactly this
reason; the ship root never did because nothing sampled it smoothly before.

## Steps

- [ ] Verify the mechanism: confirm avian 0.7's interpolation setup (is the
      interpolation plugin in PhysicsPlugins::default(); when does it write
      Transform relative to Update) and where the chase camera samples the
      ship (update_chase_camera_input in Update -> ChaseCameraInput -> bcs
      chase systems), including system ordering within the frame - the user's
      "lagging a frame behind" memory may be a real second bug (anchor
      written after the bcs chase system consumed it).
- [ ] Fix: add `TransformInterpolation` to the spaceship root at spawn
      (nova_scenario spaceship object), and fix any ordering hole found in
      the camera chain. Consider the torpedo projectile root too (fast mover
      watched by the same smoothed camera).
- [ ] Test: assert the spawned ship root carries TransformInterpolation; if
      feasible, an app-level test that the root's Transform advances on
      render frames between fixed ticks (the observable the fix creates).
- [ ] Smokes: 06/11 headless still green.
- [ ] Document the mechanism and fix (docs/, referencing the flight-feel
      retune doc), including why the twitch appeared only with camera
      smoothing.

## Notes

- Related: turret bullet spawn (turret_section.rs ~830) already inserts
  TransformInterpolation - the in-repo precedent.
- The camera anchor also derives from ComputedCenterOfMass (world COM); COM
  is fixed in body space between ticks, so interpolating the root Transform
  smooths the anchor fully.
- If ordering is wrong (anchor written after bcs chase consumed it), the fix
  is a system-set constraint between NovaCameraSystems and the bcs chase set.
