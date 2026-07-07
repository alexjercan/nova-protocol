# Turret aim lags moving targets: add lead (and/or smoothing) to the slew

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.4.0,turret,bug

Surfaced by the turret test range (task 20260707-095008, `examples/08_turret_range.rs`).
The turret aims by slewing its yaw/pitch rotators toward the target's *current*
position at a fixed angular rate (`update_turret_target_yaw_system` /
`update_turret_target_pitch_system` in `crates/nova_gameplay/src/sections/turret_section.rs`),
with no lead and no smoothing. Against a moving target the barrel therefore
tail-chases: in the range the aim error catches the sweeping gate down to ~7 deg,
then breathes back up to ~20 deg and oscillates as the gate reverses direction -
the "clunky" feel. A PDC that never settles on a crosser also wastes most of its
fire.

Expected: the turret leads a moving target (aims where it will be, given bullet
`muzzle_speed`), so the aim error against a constant-velocity crosser stays small.

## Steps

- [ ] Give the turret aim a lead solution: from the target position + velocity and
      the bullet `muzzle_speed`, compute the intercept point (same constant-bearing
      idea as the torpedo PN work) and slew toward that instead of the raw position.
      Target velocity can come from the target entity's `LinearVelocity`.
- [ ] Consider a small deadzone / smoothing so a near-aligned barrel does not jitter.
- [ ] Verify in `08_turret_range`: the aim-error readout against the sweeping gate
      should stay low (single digits) instead of oscillating to ~20 deg. Add a
      unit test for the pure lead/intercept function.

## Notes

Source: `crates/nova_gameplay/src/sections/turret_section.rs`
(`update_turret_target_yaw_system`, `update_turret_target_pitch_system`, `muzzle_speed`).
The torpedo PN guidance (task 20260525-133021) is the reference for a lead solution.
