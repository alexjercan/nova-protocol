# Cap the chase camera zoom-out and pivot distance at high speed

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.5.0,camera,tuning

## Goal

Playtest (user, 2026-07-11): at really high speeds (~300 u/s) the chase
camera zooms out - as the speed-based rig intends - but TOO much: when you
turn around, the pivot sits too far behind the spaceship. Requested
change: cap it - "the radius of the camera and the pivot distance should
not allow the camera to move the pivot too far away".

## Steps

- [x] Located the mechanism - and it is NOT a designed speed zoom: there
      is no speed-to-zoom mapping in the rig at all (the only dollies are
      the orbit survey scale, already capped at SURVEY_MAX_DISTANCE, and
      the 3 u burn push). The growth is the chase lerp's steady-state lag:
      tracking a constant-velocity anchor settles v * dt * r / (1 - r)
      behind the rig position (r = (smoothing^7)^dt), ~20 u at 300 u/s -
      measured 40.5 u camera distance vs the designed 20.6 u in the
      20260711-125225 trace.
- [x] Fix: velocity lead in update_camera_rig - the rig offset is led by
      exactly the lerp's discrete-time lag constant
      (chase_lag_lead_seconds), expressed in the anchor rotation frame.
      Only the CAMERA position leads; focus_offset is untouched, so the
      look-at point and the ship's framing are identical at every speed.
      The input->mode->rig systems are now fully chained (the lead uses
      this frame's anchor rotation).
- [x] Cap semantics delivered structurally: the steady camera distance IS
      the rig distance at any cruise speed (no growth to cap); the survey
      dolly keeps its existing SURVEY_MAX_DISTANCE cap; the burn push
      stays a fixed 3 u. No new knobs needed - the "pivot too far behind"
      distance no longer exists.
- [x] Decel zoom slew (the 20260711-121701 note): with the lead, braking
      shrinks the lead continuously with v and the same chase smoothing
      eases the camera home - no discrete contraction to slew. Also cuts
      the post-hitch recovery tail measured in 20260711-125225 (steady
      error is now 0, so hitches decay in a few frames instead of
      breathing a 20 u gap).
- [x] Regression camera_framing_is_speed_invariant (camera_controller.rs):
      the real update_camera_rig + bcs chase + physics ship; the ship's
      converged position in CAMERA space at 300 u/s must equal the 5 u/s
      baseline within 0.5 u, with a delivery guard that the cruise
      happened. A/B: with the lead zeroed it fails at 20 u of drift.
      First landed formula used the continuous tau and overshot by 2.4 u
      at 60 fps - the test caught it; the discrete form landed.
- [x] fmt + full lib suite 358/358.
## Notes

- Filed from user feedback mid-flow (2026-07-11). Feel-tuning: expect a
  user retune round after landing; make the caps easy to tweak.
- The camera anchor/ordering wiring was just reworked in 20260710-231928;
  this task is about the RIG VALUES (offset magnitudes), not scheduling.
- From the 20260711-121701 investigation: the "wobble while decelerating"
  has NO physical mechanism (the hull is dead steady in a 300 u/s
  hold-reverse trace, max spin 0.0023 rad/s), so the remaining suspect is
  this rig: during deceleration the speed-based zoom CONTRACTS every
  frame while the smoothed camera chases a decelerating anchor. While
  capping the radius, also check the zoom mapping's rate of change during
  decel (a slew/smoothing on the zoom target may be needed so braking
  does not read as hull instability).

## Resolution

One rig change (velocity lead, discrete-time exact), one helper with the
derivation, systems chained, one speed-invariance regression. The
user-facing outcome: the camera holds its framing at any speed, the pivot
stays where the rig puts it, decel eases home smoothly, and post-hitch
recovery is a few frames.

Self-reflection: the first formula (continuous tau) was a textbook
discretization slip; the regression's tight 0.5 u bound caught a 2.4 u
error that a looser "did it improve" assertion would have shipped. Tight
bounds on physical invariants keep paying.
