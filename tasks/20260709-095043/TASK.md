# Flight feel polish: rotation slew, handling stats, camera weight, retune

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.4.0,handling,juice

Spike: docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md (design
calls 4 and 5); re-scoped for the diegetic-autopilot rework
(docs/spikes/20260709-103324-diegetic-autopilot.md) which replaced the
velocity-servo layer.

## Goal

Make the hull's weight legible on top of the autopilot-era flight layer: the
manually commanded rotation slews instead of teleporting, the PD constants
become tuned handling stats instead of buried defaults, and the chase camera
conveys acceleration. Ends with a playtest retune of every feel constant from
the flight cycle - spool rates, autopilot margin/standoff/alignment gate, PD
constants, slew rate, camera smoothing/push (reused tuning constants are
decisions, not defaults - juice retro).

## Steps

- [ ] Slew-limit the commanded rotation: in
      `crates/nova_gameplay/src/input/player.rs`
      (`update_controller_target_rotation_torque`), track the camera quat
      through a max-deg/s rate limiter (or `smooth_nudge`) before writing
      `ControllerSectionRotationInput`, with the rate in the handling block.
      The camera stays instant; only the hull's commanded target lags. Pure
      helper for the limiter, unit-tested (angle never exceeds rate * dt).
- [ ] Surface handling stats: PD frequency/damping/max_torque
      (controller_section.rs:30) plus the new slew rate as a tuned, documented
      handling block (extend `FlightSettings` or the controller section
      config - decide in-work; register the reflected tree either way), with
      capital-ship defaults picked deliberately.
- [ ] Camera weight: set chase `smoothing` > 0 for the gameplay camera modes in
      `crates/nova_gameplay/src/camera_controller.rs`
      (`sync_spaceship_control_mode` currently leaves bcs default 0.0) and add
      a small offset push-back proportional to spooled main-drive input;
      respect the mode-switch anchor fix (mutate in place, no re-insert).
- [ ] App-level tests: commanded-rotation lag (a 180-degree camera flip takes
      more than one frame to reach the input component), camera offset returns
      to baseline when thrust stops.
- [ ] Playtest retune with the user: shake/flash overlap check under burn
      (juice), spool rates, autopilot margin/standoff/alignment, the
      attitude_deadband (0.4) + align_hysteresis (0.03) twitch thresholds,
      slew rate, PD constants, camera smoothing/push; record the final
      values and reasoning in the flight design note.
- [ ] Verify: fmt, clippy --all-targets, cargo test --workspace, wasm check.

## Notes

- Depends on: 20260709-103434 (diegetic autopilot rework; provides
  FlightSettings + spooled thruster input this task reads). The slew limiter
  must only shape the MANUAL rotation command path - the autopilot writes
  `ControllerSectionRotationInput` directly and plans its own turns.
- Relevant: bcs chase.rs:86 (smoothing field exists, unused), bcs shake.rs
  (juice already owns trauma - do not double-feed from thrust here).
- Camera offsets are currently hard-set per mode in
  `sync_spaceship_control_mode`; the burn push must compose with mode
  switches, not fight them (additive on top of the mode's base offset).
