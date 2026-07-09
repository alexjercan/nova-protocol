# Camera twitch when flying: ship transform stair-steps at fixed ticks under the smoothed camera

- STATUS: CLOSED
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

- [x] Verify the mechanism: confirm avian 0.7's interpolation setup (is the
      interpolation plugin in PhysicsPlugins::default(); when does it write
      Transform relative to Update) and where the chase camera samples the
      ship (update_chase_camera_input in Update -> ChaseCameraInput -> bcs
      chase systems), including system ordering within the frame - the user's
      "lagging a frame behind" memory may be a real second bug (anchor
      written after the bcs chase system consumed it).
- [x] Fix: add `TransformInterpolation` to the spaceship root at spawn
      (nova_scenario spaceship object), and fix any ordering hole found in
      the camera chain. Consider the torpedo projectile root too (fast mover
      watched by the same smoothed camera).
- [x] Test: `scenario_objects_interpolate_their_transforms` (bundle
      presence), a TransformInterpolation assertion in the 11_com_range smoke
      (live ship), and - from review R1.2 -
      `scenario_bodies_move_between_fixed_ticks`, the behavioral test: 4 ms
      render frames against the 15.6 ms tick, translation must advance every
      frame. Proven against the bug: without the component it fails with the
      exact stair-step the user reported.
- [x] Smokes: 06/11 headless still green.
- [x] Document the mechanism and fix (docs/, referencing the flight-feel
      retune doc), including why the twitch appeared only with camera
      smoothing.

## Resolution

Theory confirmed, with one correction to the reported hypothesis: there is no
frame lag - the interpolation/anchor/camera chain is correctly ordered within
one frame (RunFixedMainLoop easing -> Update anchor -> PostUpdate camera).
The twitch was the missing interpolation OPT-IN: avian's interpolation plugin
is on by default but per-body; turret bullets had `TransformInterpolation`,
the ship root did not, and the new camera smoothing was the first consumer to
sample the ship at render rate. Fixed at the shared spawn seam
(`base_scenario_object` - ships, asteroids, gates) plus the torpedo root.
Pinned by a bundle unit test and a wiring assertion in the 11_com_range
smoke; mechanism documented in docs/2026-07-09-camera-twitch-interpolation.md.

Honest scope: nova_scenario + nova_gameplay tests and both smokes run
locally, green; fmt + cargo check green; full suite/clippy deferred per
AGENTS.md.

Reflection: the user's "I had this issue long ago, it was transform syncing"
was the right neighborhood and the wrong culprit - checking the actual
schedule ordering before believing the frame-lag theory kept the fix to two
targeted component insertions instead of a system reorder.

## Notes

- Related: turret bullet spawn (turret_section.rs ~830) already inserts
  TransformInterpolation - the in-repo precedent.
- The camera anchor also derives from ComputedCenterOfMass (world COM); COM
  is fixed in body space between ticks, so interpolating the root Transform
  smooths the anchor fully.
- If ordering is wrong (anchor written after bcs chase consumed it), the fix
  is a system-set constraint between NovaCameraSystems and the bcs chase set.
