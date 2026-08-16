# Turret aim smoothing is framerate-dependent

- STATUS: CLOSED
- PRIORITY: 67
- TAGS: v0.11.0,bug,combat,ship

## Defect

Turret aim smoothing is framerate-dependent: `AIM_CORRECTION_GAIN` damps the
aim error per FRAME, not per second. Measured residual lag on a tracking
turret: ~0.43 deg at 60 fps vs ~1.8 deg at 14 fps.

This used to be invisible. Since 23103d4e a turret holds fire until the barrel
is within the bearing gate (0.92 deg), so at low framerate the residual lag sits
ABOVE the gate and a PDC can refuse to fire at exactly the moment the machine
struggles - framerate now changes combat outcomes.

## Fix

Make the smoothing dt-based so convergence is framerate-invariant: exponential
decay of the aim error with a per-second rate (`1 - exp(-rate * dt)` form), rate
chosen to match today's 60 fps feel so tuning does not shift.

## Definition of done

- aim convergence trajectory is the same across simulated frame rates (test
  steps the app at different fixed dts and asserts matching residual error at
  matching sim time)
- the bearing-gate behaviour holds at low fps: a turret tracking a crossing
  target at 14 fps equivalent dt still reaches the gate and fires
- 60 fps feel unchanged (rate constant derived from the old per-frame gain at
  60 fps, stated in a comment)

## Closure

Landed as 8852fa5e (2026-08-16), lane turret-aim-dt. AIM_CORRECTION_GAIN
(0.35/frame) is now AIM_CORRECTION_RATE = 25.847/s, gain = 1 - exp(-rate*dt);
-ln(1-0.35)*60 reproduces the shipped 60 fps feel exactly. Residual at matched
sim time: 1.057 deg -> 0.105 deg at 1/14 (0.244 at 1/60 both before and after).
The live 14 fps gate sees ~0.6 deg, under the 0.92 deg fire gate - a PDC on a
struggling machine fires again. Two tests pin it (invariance in aim.rs, the
full 14 fps gate-and-fire chain in firing.rs); both fail on the old flat gain.
Full workspace lib run: 22 binaries, 1790 passed; the one failure
(nova_editor skin::tests::the_build_view_wears_its_ship_style) is pre-existing
at the branch point and tracked separately.
