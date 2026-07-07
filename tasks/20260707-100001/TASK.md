# Torpedo bay test range example (playable + gates + autopilot)

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.4.0,example,torpedo

The torpedo bay is the clunkiest section: torpedoes sometimes spawn too close and
die instantly, the controls feel weird, and the homing is weak. Build a dedicated
playable test range for it (mirrors the PDC turret range task) so torpedo behavior
is easy to observe, tune, and regression-test.

Goal: a scene where you can fire torpedoes at targets at varied ranges and clearly
see whether they arm, home, and detonate correctly.

## Steps

- [ ] Add `examples/0X_torpedo_range.rs` with a single ship carrying one torpedo bay
      section and an observation camera.
- [ ] Place **target gates / dummies** at a spread of distances: very near (to catch
      the self-detonate-on-spawn bug), mid, and far, plus a slowly moving target (to
      judge homing / lead). Log/score arm -> home -> hit for each shot.
- [ ] Visualize the guidance: draw the torpedo's target position and the line-of-sight
      so the homing quality is legible while tuning.
- [ ] Wire the BCS autopilot + screenshot harness so it doubles as a headless smoke
      test (fire at each gate, assert torpedoes arm and no panic). Depends on the
      harness-wiring task.
- [ ] Use the range to validate the two torpedo bug fixes (arming delay; survive
      target loss) and any guidance improvement (PN guidance, task 20260525-133021).
- [ ] Run once by hand and via `BCS_AUTOPILOT=1` before closing.

## Notes

Torpedo logic: `crates/nova_gameplay/src/sections/torpedo_section.rs`
- spawn: `shoot_spawn_projectile` (spawns at `spawner` transform, ~0.01 ahead).
- detonation: `torpedo_detonate_system` (fires when within `BLAST_RADIUS*0.5` of the
  target position, no arming gate).
- guidance: `torpedo_sync_system` + `torpedo_thrust_system` (ad-hoc pursuit feeding an
  absolute quaternion into the PD controller).
This range is the harness for tasks 20260707-100003 (arming), 20260707-100004 (target
loss), 20260525-133021 (PN guidance), and 20260706-162913 (torpedo module extraction).
