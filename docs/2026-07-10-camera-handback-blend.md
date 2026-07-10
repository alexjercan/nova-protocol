# Camera handback blend (autopilot to manual)

Task: tasks/20260710-222517 - Playtest: disengaging the autopilot snapped
the camera, while mode switches (free look, combat) blend smoothly.

## Why it snapped

Mode switches are smooth because they re-seed the incoming rig's
PointRotation from the CURRENT rig output - the quat never jumps. The
autopilot disengage is the one re-seed that cannot do that: the rig must
land on the hull attitude instantly or the PD swings the ship to
wherever the mouse free-looked during the maneuver (the no-lurch
contract). The camera's anchor rotation follows the rig quat, so it
inherited that one-frame jump.

## The fix

Split the two consumers of the discontinuity:

- The SHIP keeps the instant re-seed - contract untouched.
- The CAMERA gets a `CameraHandbackBlend { from, elapsed }` on the
  controller entity, seeded (by `on_autopilot_disengaged`) with the rig
  output it was actually following at the moment of disengage - and only
  when the normal rig is the active one (in FreeLook/Turret the normal
  rig is dormant while another rig drives; the eventual switch back to
  Normal after a dormant re-seed is a separate, pre-existing transition
  this task does not change). `update_chase_camera_input`
  eases `anchor_rot` from the held direction onto the live rig with a
  smoothstep slerp over `HANDBACK_BLEND_SECONDS` (0.45, playtest knob)
  and removes the component when done. `anchor_rot` feeds only the chase
  camera, so the blend cannot affect aim, targeting or the flight rig.
- Mouse motion during the blend moves the live target, so the ease
  converges to wherever the player is looking, never to a stale quat.

Accepted edge (documented in code): a re-disengage inside the 0.45s
window restarts the blend from the rig's pre-reseed output rather than
the mid-blend display value - a small pop in a rare double-handback,
the price of a stateless observer.

## Verification

Pure-helper test (endpoints, monotonic ease; epsilons account for
acos amplification in Quat::angle_between, ~7e-4 rad on "equal" quats),
an app-level test through the real observer + input system (anchor stays
on the held view at the disengage frame while the rig already reads the
hull; forced expiry lands on the live rig and removes the blend), and an
inactive-rig test (no bridge when FreeLook/Turret owns the camera).
camera_controller 9, gameplay lib 344, examples check clean.
