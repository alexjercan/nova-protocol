# Implement PN guidance for torpedo

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.4.0, torpedo

Proportional navigation intercept. Legacy #142.

Replace the ad-hoc pursuit in `torpedo_sync_system` + `torpedo_thrust_system`
(`crates/nova_gameplay/src/sections/torpedo_section.rs`) - which points the nose
straight at the target's current position plus a hand-tuned drift term - with
proper proportional navigation, so the torpedo leads a moving target instead of
tail-chasing it.

The torpedo is not free-acceleration: it steers a PD attitude controller
(`ControllerSectionRotationInput`, a desired orientation) and thrusts forward
(`ThrusterSectionInput`). So PN produces a desired heading (lead direction) that
we feed to the controller; thrust follows the nose.

## Approach

Vector "true PN": with LOS `R = target - torpedo`, relative velocity
`Vrel = target_vel - torpedo_vel`, LOS rate `Ω = (R × Vrel) / (R·R)`, the
commanded turn is `a = N · (Ω × V_torpedo)` (perpendicular to velocity, ∝ LOS
rate). The desired heading is `(V_torpedo + a).normalize()`, falling back to
straight pursuit when the torpedo is nearly stationary (spawn) or geometry is
degenerate. Target velocity comes from the target entity's `LinearVelocity`
(0 when the target is lost, so PN degrades to pursuit of the frozen position -
consistent with 20260707-100004).

## Steps

- [x] Added pure `pn_steer_direction(rel_pos, rel_vel, missile_vel, n) -> Vec3` (vector
      true PN, `a = N·(Ω × V)`) with unit tests: `pn_leads_a_crossing_target` (a
      +X-crossing target yields a +X lead), `pn_pursues_a_stationary_target_straight`
      (no LOS rate -> points at target), `pn_handles_degenerate_inputs` (coincident /
      stationary -> finite unit vector, no NaN).
- [x] Added `nav_constant` to `TorpedoSectionConfig` (default 3.0) and threaded it onto
      the projectile via a `TorpedoGuidance` component; updated the in-game section in
      `nova_assets/src/sections.rs`.
- [x] Added `torpedo_pn_guidance`, which writes the steering direction into a
      `TorpedoSteering` component using the target entity's `LinearVelocity` (zero when
      the target is lost, so PN degrades to pursuit of the frozen position). Rewrote
      `torpedo_sync_system` (orientation) and `torpedo_thrust_system` (thrust along the
      nose) to consume `TorpedoSteering`; removed the old drift-correction pursuit.
- [x] Scheduled `torpedo_pn_guidance` after `update_target_position` and before sync/thrust.
- [x] Verified: 12 torpedo tests pass (incl. the 3 PN tests); clippy clean; no-debug
      build green (config field wired at both construction sites); range autopilot smoke
      green (4 fired, 4 armed, 3 detonated, no panic). Leading against the moving gate is
      visually confirmable in `06_torpedo_range`.

## Resolution

Replaced the ad-hoc pursuit (nose pointed straight at the target + a hand-tuned drift
term) with vector proportional navigation. A pure `pn_steer_direction` computes the
LOS rate `Ω = (R × Vrel)/(R·R)` and the PN turn command `a = N·(Ω × V_missile)`, then
returns `(V_missile + a)` normalized as the desired heading - which leads a crossing
target. `torpedo_pn_guidance` writes this to `TorpedoSteering`; the sync system orients
the PD controller to it and the thrust system pushes along the nose. `nav_constant` (N)
is config-driven (default 3.0). Target velocity comes from the target's `LinearVelocity`
so PN degrades cleanly to pursuit when the target is lost. Covered by 3 new unit tests.

## Notes

Source: `crates/nova_gameplay/src/sections/torpedo_section.rs`. Pairs with the arming
(20260707-100003) and target-loss (20260707-100004) fixes. Blast/param unhardcoding is
tracked separately by 20260706-162913.
