# Fix attitude G-limit overspeed authority direction

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.13.0

## Bug

The attitude envelope intends `sqrt(LOAD_LIMIT / arm)` to be the maximum sustained angular rate. Once a hull exceeds that rate, `AttitudeEnvelope::available` restores the full scalar acceleration ceiling so the controller can shed spin. The scalar does not preserve the required direction. If the desired attitude remains ahead of the hull, the PD controller can spend the restored authority to accelerate farther above the structural rate instead of braking.

This makes the documented G-force limit ineffective under continuous held-turn input. A shorter surviving hull gets more angular acceleration authority, but an already settled intact hull does not move from its old structural rate to the higher damaged rate as intended.

## Source evidence

Confirmed on `master` at `cfad467c4b1ae998b4735215ec76074e551149f5`:

- `crates/nova_ship/src/physics/attitude.rs`: `available(spin)` returns `structural_ceiling` whenever `spin * spin > structural_ceiling`, with the comment that this authority exists to shed rate.
- `crates/nova_ship/src/sections/controller_section.rs`: the returned unsigned scalar becomes `PDController::max_angular_acceleration`.
- `crates/nova_ship/src/physics/pd_controller.rs`: the PD computes the signed acceleration from attitude error and damping, then clamps it symmetrically to that scalar. Nothing requires overspeed acceleration to oppose angular velocity.

For the shipped controller tuning, a target held 90 degrees ahead settles at pi rad/s because `kp = 144`, `kd = 72`, and `kp * (pi / 2) / kd = pi`. This is a PD equilibrium, not Avian's `MaxAngularSpeed`; Avian defaults that component to infinity.

## Reproduction proof

Port the minimal non-capture part of Content Machine's `damaged-turn-rate` experiment into a Nova system example or focused integration fixture:

1. Spawn two identical dynamic ships as straight nine-part lines: seven reinforced hull sections, one controller, and one rear thruster.
2. Keep one 90-degree relative right-turn demand on both controllers from the first fixed tick. Do not change input at the damage beat.
3. Let both intact ships spin for 5-6 seconds.
4. Destroy the two front hull sections of one ship through `HealthApplyDamage` while both continue spinning.
5. Preserve live angular velocity, structural arm, controller authority, and section-count measurements. Do not include Content Machine recording or camera code.

Current behavior:

- Intact arm: about 4.604 world units; nominal sustained rate: about 1.306 rad/s.
- Shortened arm: about 3.565 world units; nominal sustained rate: about 1.484 rad/s.
- Both ships nevertheless settle at about 3.142 rad/s under the unchanged held input.

The proof must fail against the old directional behavior and pass with the fix. Keep it deterministic and runnable through the normal debug autopilot path.

## Required behavior

- At or below the structural corner, retain the vector combination of tangential and centripetal tip acceleration.
- Above the structural sustained rate, permit authority that reduces angular-speed magnitude, but reject authority that increases it.
- Apply the rule correctly for arbitrary spin and requested-torque axes, not only positive yaw.
- Preserve enough braking authority that an overspun hull cannot become trapped above the limit.
- Recompute the limit from live sections so losing the nose can raise the allowed sustained rate during flight.
- Keep torque-limited hull behavior and controller stacking intact.

## Acceptance

- A regression test proves that overspeed controller torque cannot increase angular-speed magnitude and can reduce it.
- Tests cover positive and negative spin, aligned and cross-axis requests, the structural corner, and an overspun hull.
- The non-capture damaged-hull proof shows the intact ship constrained near its old structural rate and the shortened survivor able to accelerate toward its higher structural rate under unchanged input.
- Existing attitude, controller-stack, autopilot, and manual-flight tests pass.
- The system example completes through the debug autopilot with measured assertions and no capture dependency.
- Player-facing documentation and the v0.13.0 changelog describe the corrected G-force turn limit without exposing internal implementation details.

## Log

- 2026-09-03 12:05 Reproduced, fixed and proved. `examples/systems/system_turn_limit.rs`
  is the non-capture port of Content Machine's `damaged-turn-rate`: two
  nine-cell hulls hold one 90-degree demand, one loses its two nose cells
  mid-turn, and the range reads arm, sustained rate, live spin and controller
  authority off both. Before the fix it measured the reported numbers exactly -
  46.0 m arm, 1.306 rad/s allowed, both hulls turning at 3.142 rad/s (pi, the
  PD equilibrium `kp * (pi/2) / kd`) on the full 1.705 rad/s2 the overspeed
  branch hands back.

  The fix pairs the returned ceiling with a DIRECTION. `PDController` gains
  `sustained_angular_speed`, written by `update_controller_stack_tuning` from
  `AttitudeEnvelope::sustained_turn_rate` - whole, not shared out, because
  every computer on a hull is fighting the same metal. Past that speed
  `brake_only_past` drops the part of the clamped acceleration that lies along
  the spin and keeps the rest, so the authority can brake and steer but not
  tighten. It works off the spin's own direction, so it holds for any spin and
  request axis; if taking that component out pushes a principal axis past the
  clamp the whole vector is scaled, never re-clipped, because a clip would put
  some of the spin direction back.

  After: intact 1.306 rad/s over 46.0 m, shortened 1.485 rad/s over 35.7 m
  (14% faster on an unchanged input), each within 0.1% of its own limit.

  Proof: five new PD tests (both spin signs, aligned, cross-axis, oblique, the
  corner, and braking at the full limit), a held-command integration test with
  an unlimited twin as its control, and a stack test that every computer on a
  hull carries the whole rate. Green: attitude 13, flight 79 (stacking,
  manual, autopilot), controller_section 16, ai 122, player input 35, and the
  ranges system_turn_limit, system_attitude_hold, system_player_path.
  Not run, per the standing instruction: the workspace suite and Clippy.

  Found on the way, NOT this task's: `system_outcomes` panics in
  `nova_gameplay::audio::voice::start_sfx_voices`, inserting an `AudioPlayer`
  on an entity despawned the same frame. Confirmed on the baseline with this
  task's changes stashed.
