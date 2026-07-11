# Camera jumps at high speed give the controls a twitchy feel

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.5.0,camera,bug

## Goal

Playtest (user, 2026-07-11, second session - AFTER the HUD/indicator fixes
landed and the text chips read clean): "sometimes the camera jumps at high
speeds and it gives that twitchy feeling to the spaceship controls".
Discrete JUMPS, not continuous drift - distinct from the zoom-out cap
request (20260711-121711).

## Steps

- [x] Build verification: the report is from a session WITH the ordering
      pin (the user confirmed the text chips fixed in the same message,
      and the chip fix landed in the same commit as the pin, 5ba0e3c).
      Jumps persist post-pin; proceeded to the hunt.
- [x] Headless hunt (diagnostic trace, deleted after recording; rig: real
      avian + interpolation + bcs ChaseCamera at production smoothing
      0.15, ship cruising 300 u/s, ship position measured in CAMERA space
      per frame = the on-screen motion):
      - Steady cruise: on-screen motion is PERFECTLY smooth (delta
        0.0000-0.0010 u/frame). The double-tick beat, the eased clock and
        the pinned ordering are all clean. BUT the camera-to-ship
        distance is 40.5 u vs the designed 20.6 u: the chase lerp has a
        steady-state lag of v * tau (tau = -1/(7 ln smoothing) = 75 ms;
        22.4 u at 300 u/s) - this is 20260711-121711's "zooms out too
        much / pivot too far behind", now with its real mechanism: it is
        not a designed speed zoom at all.
      - A single 100 ms frame hitch: 6.84 u on-screen JUMP in one frame,
        then ~15 frames of gap-breathing recovery. THE reported jump.
        Mechanism: the smoothing eases the camera in WORLD space, so any
        frame-pacing variation converts to on-screen ship displacement of
        about (1 - closed_fraction) * v * dt_spike - proportional to
        SPEED, hence "sometimes ... at high speeds" (hitches happen; only
        at speed do they show).
      - Burn-push offset toggle: a smooth 0.6 u surge - intended feel,
        not the jump.
- [x] Fix routing (not a code fix in this task, deliberately): the jump
      cannot be removed while the camera smooths its world-space
      translation - velocity-lead cancels the steady lag but leaves the
      hitch transient (~8 u, computed). The cure is easing the camera in
      SHIP-RELATIVE space (rigid translation channel, eased feel
      channels), which is precisely the smoothing-architecture fork the
      user queued as the feel spike (20260711-125227) - it now inherits
      these numbers as its input. The velocity-lead itself lands in
      20260711-121711 (kills the lag = the zoom-out, and cuts the
      post-jump recovery wobble from ~15 frames of 22 u gap-breathing to
      a ~5 frame decay).
- [x] Coordination recorded on both follow-ups; no production code change
      on this branch (trace deleted per convention, numbers preserved
      here).
## Notes

- Related: docs/spikes/20260711-103527-twitching-family-two-clocks.md
  (falsified: physical hull wobble - the hull is provably steady, see
  tasks/20260711-121701), tasks/20260711-121711 (zoom cap + decel zoom
  slew), tasks/20260711-125227 (feel-smoothing spike, queued last).
- The "twitchy CONTROLS feel" wording suggests the jump lands between
  input and what the player sees - consistent with camera-side, not
  physics (the input rig is camera-INDEPENDENT for rotation, verified in
  20260711-121701).

## Resolution

Diagnosis-only close (the 20260711-121701 precedent): the jump is
confirmed, measured, and mechanically explained - a world-space smoothing
transient under frame hitches, speed-proportional. Fix split per the
user's own queue: lag elimination (velocity-lead) goes to the zoom cap
task next in the queue; the smoothing-architecture decision (ship-relative
easing) goes to the feel spike the user queued last, now armed with
numbers instead of vibes.

Evidence rig: unfinished_integrity_physics_app + bcs ChaseCameraPlugin +
the production ordering pin; ship RigidBody + TransformInterpolation at
300 u/s; anchor input driven from the eased ship Transform per frame;
camera offset (0,5,-20), focus (0,0,20), smoothing 0.15; events at frame
120 (100 ms hitch) and 180 (burn push toggle); metric = per-frame delta of
the ship position in camera space.

Self-reflection: the task plan listed four candidate mechanisms and the
real one (frame pacing x world-space smoothing) was none of them -
the trace-first discipline (fourth cycle running) again beat the
hypothesis list. The lesson from the falsification cycles holds: measure
the exact user scenario before theorizing.
