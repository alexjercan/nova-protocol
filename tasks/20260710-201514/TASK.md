# Replace SOI shell with a velocity-sphere-style gravity indicator (yellow)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0, hud, gravity, ux

## Goal

Playtest feedback (user, 2026-07-10): the SOI shell (three great-circle
rings from the holo-expansion task) reads the same as the orbit ring and
should go. Replace it with a gravity indicator in the velocity sphere's
own language: the same shader/instrument family around the spaceship
(hud/velocity.rs, DirectionalSphereOrbit + the direction-sphere
materials), a second orbiting indicator showing the local gravity
direction (toward the dominant well) and ideally its strength, tinted a
different color - yellow - so velocity (current color) and gravity never
read as the same quantity.

Direction: remove sync_soi_shell + SoiShellRing from
hud/holo_instruments.rs (tests and hud/mod.rs cleanup with it); add a
gravity indicator alongside the velocity HUD - likely a second
velocity_hud-style widget whose input is the dominant well's pull
direction (and magnitude via well_accel at the ship's position) instead
of LinearVelocity, sharing the DirectionalSphereOrbit machinery and
shader with a yellow tint. Present only while a DominantWell exists.

## Notes

- /plan owns the steps when picked up. Read hud/velocity.rs first: the
  widget already separates input (direction) from rendering; the gravity
  variant may be a config knob (color + input source) rather than a copy.
- Magnitude display: the velocity sphere encodes direction; whether the
  gravity variant should also scale/pulse with well_accel is a design
  call to make in /work by eye.
- The orbit ring, ribbon, and flip gate stay; only the SOI shell goes.
  The [O] ORBIT cue and GRAV status line remain the "you are in a well"
  text channels.
