# Live tuning sliders for the turret range

- STATUS: CLOSED
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

- [x] Add sliders (bevy `ui_widgets::Slider`, as in 02/04) bound to `yaw_speed`,
      `pitch_speed`, `min_pitch`, `max_pitch`, `fire_rate`, `muzzle_speed`, updating the live
      turret config. A `Knob` enum maps each slider to its config field in display units
      (degrees for angles), with a live value readout per slider.
- [x] Keep the autopilot/screenshot harness working (sliders inert under autopilot - no pointer
      to drag them). Headless run reaches Playing, tracks + fires, cycle complete no panic.

## Resolution

Added a tuning panel to `examples/08_turret_range.rs`: six `ui_widgets::Slider`s (mirroring
02/04) with per-knob value readouts, each writing the live turret's `TurretSectionConfigHelper`.

The turret's runtime turned out to snapshot most knobs onto child entities at spawn
(`insert_turret_section` copies `yaw_speed`/`pitch_speed`/`min_pitch`/`max_pitch` into child
`SmoothLookRotation`s and `fire_rate` into the fire timer; only `muzzle_speed` is read live). So
mutating the config alone would not have retuned a live turret. Rather than reach into those
private child components from the example, this makes `TurretSectionConfigHelper` the single
tunable source of truth: it is now `pub`, and a new `apply_turret_config_to_children` system
(gated on `Changed`) pushes the snapshotted knobs onto the live children whenever the config is
edited. This is reusable (the editor can retune the same way) and keeps the example thin.

## Notes

Source: `examples/08_turret_range.rs`, `examples/02_thruster_shader.rs` /
`examples/04_asteroids.rs` (slider pattern),
`crates/nova_gameplay/src/sections/turret_section.rs` (`TurretSectionConfig`,
`TurretSectionConfigHelper`, `apply_turret_config_to_children`).

Two co-located tests cover the propagation (editing the config retunes the child rotators + fire
timer; the `TurretSectionPartOf` guard scopes edits to the edited turret). The interactive
slider drag itself is not unit-tested (it needs pointer input); the ValueChange wiring mirrors
the working 02/04 examples and the headless run confirms no panic.
