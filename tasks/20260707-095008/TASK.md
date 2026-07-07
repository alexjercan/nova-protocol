# PDC turret test range example (playable + gates + autopilot)

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.4.0,example,turret

The PDC (point-defense) turret section feels clunky to tune because there is no
focused way to exercise it. Build a dedicated example scene that is a small
playable test range for the turret, in the spirit of the gated example scenes in
`~/personal/bevy-common-systems` (e.g. 09_reactor, 13_glide).

Goal: make the turret cheap to iterate on and to regression-test.

## Steps

- [ ] Add `examples/0X_turret_range.rs` that spawns a single ship with one turret
      section (no other clutter) and a WASD/orbit camera to observe it.
- [ ] Spawn a set of **target gates / dummies** at known offsets (static and slowly
      drifting via `transform/random_sphere_orbit`) so aiming can be judged: the
      turret should track and hit them. Score/log hits so the range gives feedback.
- [ ] Expose the tuning knobs (yaw/pitch speed, pitch limits, fire_rate,
      muzzle_speed) via on-screen sliders like `examples/02_thruster_shader.rs` and
      `examples/04_asteroids.rs` do, so tuning is live.
- [ ] Wire the BCS autopilot + screenshot harness (see task for that infra) so the
      example doubles as a headless smoke test: drive to Playing, fire at the gates,
      assert no panic. Depends on the harness-wiring task.
- [ ] Use the range to diagnose the "clunky" aiming (fixed angular-rate slew with no
      smoothing/deadzone/lead in `update_turret_target_yaw/pitch_system`). File a
      follow-up bug if a concrete aiming fix is identified; do the fix here if small.
- [ ] Run the example once by hand and via `BCS_AUTOPILOT=1` before closing (per the
      "an example is not done until it has been run once" rule).

## Notes

Turret aiming lives in `crates/nova_gameplay/src/sections/turret_section.rs`
(`update_turret_target_yaw_system` / `update_turret_target_pitch_system`, the yaw/pitch
rotators with `speed`/`min`/`max`). Reference the gated examples and `docs/dev-harness.md`
in the bevy-common-systems repo for the harness shape.
