# Review: Allow zooming out further while orbiting

- TASK: 20260710-222518
- BRANCH: feature/orbit-survey-zoom

## Round 1

- VERDICT: APPROVE (findings fixed in-round; see responses)

Verified sound by the reviewer: the smoothing claim is empirically true
(bcs chase_camera_update_state_system lerps toward anchor+offset, so the
dolly moves the lerp target through the exact mechanism mode switches
use); plan.radius is strictly positive through orbit_target_radius; only
the player ship's Autopilot is consulted and Single-param failures skip
the system as before; FreeLook scaling dollies straight back along the
mouse-rig frame and the Turret round-trip is clean; the wheel-is-taken
interpretation note matches input/player.rs; camera tests 6 pass with no
pre-existing test weakened; the burn push composes harmlessly at survey
range.

- [x] R1.1 (MINOR) camera_controller.rs (survey_scale) - latent
  f32::clamp panic when base rig length > SURVEY_MAX_DISTANCE; both
  bounds are advertised playtest knobs, so the panic edge is one knob
  turn away.
  - Response: fixed - min-then-max reorder, identical in the sane range,
    degrades to no-dolly instead of panicking; comment states why.
- [x] R1.2 (MINOR) TASK.md said IN_PROGRESS while the steps claimed
  closure.
  - Response: fixed - STATUS: CLOSED.
- [x] R1.3 (NIT) at the 250 cap the Normal rig sits ~60u above the ship
  and the focus stays on the ship, centering the ship rather than the
  orbited body; framing the body might read better for a survey posture.
  - Response: acknowledged as a design call, deliberately unchanged -
    the ship-centered stretch is coherent and documented; reframe toward
    the body is playtest territory.
- [x] R1.4 (NIT) the app-level test registers update_camera_rig without
  the chained mode-switch system; the reviewer confirmed no race exists
  (the mode switch never touches ChaseCamera fields), so fidelity is
  optional.
  - Response: left as is per the reviewer's own analysis.
