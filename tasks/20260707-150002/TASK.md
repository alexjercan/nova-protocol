# Live tuning sliders for the turret range

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.4.0,example,turret

Deferred from the turret range task (20260707-095008, step 3). Add on-screen
sliders to `examples/08_turret_range.rs` for the turret's tuning knobs - yaw/pitch
speed, pitch limits, fire_rate, muzzle_speed - so they can be adjusted live while
watching the aim-error readout, in the style of `examples/02_thruster_shader.rs`
and `examples/04_asteroids.rs`.

Not urgent: the range's aim gizmos and aim-error telemetry already make tuning
legible, and the knobs can be changed by editing the section config and re-running.
This is the convenience pass.

## Steps

- [ ] Add sliders (bevy `ui_widgets::Slider`, as in 02/04) bound to the turret
      section's `yaw_speed`, `pitch_speed`, `min_pitch`, `max_pitch`, `fire_rate`,
      `muzzle_speed`, updating the live turret config.
- [ ] Keep the autopilot/screenshot harness working (sliders inert under autopilot).

## Notes

Source: `examples/08_turret_range.rs`, `examples/02_thruster_shader.rs` /
`examples/04_asteroids.rs` (slider pattern),
`crates/nova_gameplay/src/sections/turret_section.rs` (`TurretSectionConfig`).
