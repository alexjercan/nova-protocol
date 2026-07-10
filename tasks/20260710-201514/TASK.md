# Replace SOI shell with a velocity-sphere-style gravity indicator (yellow)

- STATUS: CLOSED
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

## Steps

- [x] Generalize the velocity widget (hud/velocity.rs) into the
      directional-HUD family: `VelocityHudSource` component
      (Velocity | Gravity, default Velocity) and `VelocityHudPalette`
      (indicator + sphere colors, defaults = the current white/blue), both
      carried on the bundle from `VelocityHudConfig`; the child-spawn
      observers read the palette instead of hardcoding colors. Module doc
      says the file hosts the family (renaming the module is deliberate
      churn-avoidance; note for a future cleanup).
- [x] Gravity feeder: `update_velocity_hud_input` matches the source -
      Velocity keeps LinearVelocity; Gravity points at the target ship's
      DominantWell (direction toward the well center) and hides the whole
      widget (root Visibility) while no well owns the ship. Magnitude in
      `direction_shader_update_system` per source: velocity/100 as today;
      gravity = well_accel at the ship's position normalized by
      GravitySettings::max_surface_gravity (pure helper, unit-tested).
- [x] Spawn the gravity variant in setup_hud_velocity (hud/mod.rs):
      yellow palette, radius slightly outside the velocity sphere (5.6 vs
      5.0) so the two shells nest instead of z-fighting; the existing
      remove observer already covers it (same marker + target).
- [x] Remove the SOI shell: sync_soi_shell + SoiShellRing + its test out
      of hud/holo_instruments.rs, the q_shell cleanup out of hud/mod.rs,
      the prelude export, and the module doc line (user feedback: reads
      the same as the orbit ring).
- [x] Tests: gravity feeder truth table (in a well -> direction toward
      the well + Visible; no well -> Hidden; velocity source untouched);
      pure magnitude helper; holo module tests still green minus the
      shell.
- [x] fmt + check --workspace --examples + affected modules (hud, flight,
      gravity); document in docs/2026-07-10-gravity-indicator.md
      (including the removal rationale).

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

## Resolution

Shipped per plan: VelocityHudSource + VelocityHudPalette generalize the
widget (defaults preserve the velocity readout exactly), the yellow
gravity variant points down the dominant well's pull with magnitude
normalized by the strength cap and hides in flat space, and the SOI shell
is fully removed. 3 new tests; hud/holo/velocity modules green; the
05_directional example updated; fmt + check --workspace --examples clean.
Details: docs/2026-07-10-gravity-indicator.md.
