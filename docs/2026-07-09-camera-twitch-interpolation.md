# Camera twitch when flying: fixed-tick stair-steps under the smoothed camera

Task: `tasks/20260709-160753`. Playtest report on the flight-feel retune
(`docs/2026-07-09-flight-feel-retune.md`): the camera weight felt right, but
the view twitched when moving - suspected transform-sync / frame-lag issue.

## Mechanism (why it appeared only now)

avian advances a rigid body's `Transform` only on 64 Hz FixedUpdate ticks. At
render rate the ship therefore moves in stair-steps: several render frames
show the same pose, then it jumps a full physics tick (~15.6 ms of motion at
once).

The old camera never exposed this: with `smoothing = 0.0` the chase camera was
a rigid function of the ship's transform, so camera and ship stepped
TOGETHER - zero relative motion on screen. The flight-feel retune gave the
camera weight (`smoothing = 0.15`), which eases the camera at render rate
against that stair-stepping anchor: the steps became ship-vs-camera relative
motion, i.e. the twitch. The suspected "lagging a frame behind" was checked
and ruled out - the chain is ordered correctly within one frame:

1. `RunFixedMainLoop`: physics ticks; then (in
   `RunFixedMainLoopSystems::AfterFixedMainLoop`) bevy_transform_interpolation
   eases opted-in transforms between the previous and current physics states.
2. `Update`: nova writes the camera anchor from the ship transform.
3. `PostUpdate`: the bcs chase camera consumes the anchor and moves.

The missing piece was step 1's opt-in: avian's `PhysicsInterpolationPlugin`
ships inside `PhysicsPlugins::default()`, but bodies must carry
`TransformInterpolation`. Turret bullets always had it; the ship root never
did, because until the camera eased, nothing sampled it smoothly.

## Fix

- `TransformInterpolation` added to `base_scenario_object`
  (nova_scenario/actions.rs) - every dynamic scenario body (ships, asteroids,
  the drifting range gates) now renders interpolated between fixed ticks.
- Also added to the torpedo projectile root (fast mover watched by the same
  smoothed camera; brings it in line with turret bullets).

Physics integration is untouched: avian works on `Position`/`Rotation`
(raw), the flight layer and autopilot read those in FixedUpdate, and the
easing snaps back to the true pose before every physics tick. What DOES
change is every Update-schedule reader of `Transform`/`GlobalTransform`:
aim rays, gizmos, the camera anchor - and also projectile spawn points and
the torpedo arming/fuze/guidance loop - now operate on the eased pose, up to
one tick (~15.6 ms of motion) behind raw physics. That is intentional:
launches and hits line up with what the player sees.

## Verification

- `scenario_objects_interpolate_their_transforms` (actions.rs) pins the
  component on the shared spawn bundle.
- `11_com_range`'s headless script asserts the player root carries
  `TransformInterpolation` (wiring regression net).
- Both headless smokes green; the twitch itself is a feel observation - the
  playtest checklist re-run confirms it.

## Notes

- If a future body must NOT interpolate (e.g. server-authoritative snapping),
  remove the component per entity rather than dropping it from the shared
  bundle.
