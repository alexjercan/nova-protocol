# Thrust balancing: compensate off-center engine torque under burn

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.4.0,handling,physics

From review R1.2 of the flight-feel retune (20260709-095043). Since the torque
budget cut (max_torque 100 -> 40) the flight computer can only hold roughly
`max_torque / 64` units of lateral lever arm per unit thruster magnitude
(~0.6 units): an asymmetric editor build, or a damage-shifted COM, pulls or
pinwheels under burn. Documented as diegetic in
docs/2026-07-09-flight-feel-retune.md and pinned by the
`off_center_burn_pulls_but_a_centered_drive_is_held` test - but a real flight
computer would balance thrust, not fight it with RCS.

## Steps

- [ ] Decide the model with the user: differential throttle (down-throttle the
      off-axis engine so net torque ~0, losing some thrust), PD feed-forward
      (subtract predicted thruster torque from the command), or keep the pull
      as gameplay (close as by-design).
- [ ] Implement in the flight layer (manual_burn_system and the autopilot's
      spool loop both set thruster inputs; balancing belongs where the inputs
      are chosen, using each engine's lever arm about the live COM).
- [ ] Extend the off-axis physics test: balanced burn tracks the command
      within the centered-drive tolerance.

## Notes

- Lever math: torque_i = (engine_pos - COM) x (world_dir * magnitude * input).
- Related: 20260709-155922 (disabled controller torque), the multi-thruster
  spike's deferred torque-aware allocation (docs/spikes/20260709-121746).
