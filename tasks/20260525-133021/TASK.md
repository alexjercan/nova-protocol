# Implement PN guidance for torpedo

- STATUS: CLOSED
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

## Follow-up: closed-loop tests + guidance harness (doubt raised in review)

Concern raised that PN "doesn't work properly". Added evidence it does:

- Closed-loop kinematic tests (turn-rate-limited torpedo flying the law):
  `pn_intercepts_a_crossing_target`, `pn_intercepts_a_target_crossing_either_way`
  (sign symmetry), and `pn_demands_less_turning_than_pure_pursuit` (PN's peak turn
  demand is < 75% of pursuit's - the defining advantage). All pass (6 PN tests total).
- New example `examples/07_torpedo_guidance.rs`: a real torpedo vs a single fast
  crossing target (15 u/s), reporting the closest approach and torpedo speed. Headless
  autopilot run: the torpedo builds to ~60 u/s and closes to **1.2 units** of the
  crosser (well inside the blast trigger), 3 detonations, no panic. So PN leads and
  intercepts a moving target both in the law and in the full physics/PD stack.

## Follow-up: fire-without-lock now flies straight (was steering at the origin)

A torpedo fired with no lock used to steer toward the world origin, because it
spawned with a sentinel `TorpedoTargetPosition(ZERO)`. Fixed: the projectile no
longer spawns with a target position; `update_target_position` inserts it on first
lock (and updates in place after). `torpedo_pn_guidance` flies straight ahead
(holds heading) when there is no `TorpedoTargetPosition`, and still freezes on the
last-known position after a target is *lost* (100004 behavior preserved). Covered by
`untargeted_torpedo_flies_straight_not_toward_origin`; both example harnesses still
intercept with no regression.

## Follow-up 2: the reported "flies off randomly" - two real root causes, fixed

User report: the torpedo always thrusts and flies off in a random direction, never
turning toward the target even when it is stationary. Both root causes found and
fixed; the earlier tests missed them because they initialized the torpedo already
flying at the target at speed.

1. **Velocity-anchored PN cannot recover from the real launch.** The torpedo
   leaves the bay slowly and *sideways* (spawner up, ~1 u/s, nose perpendicular to
   the velocity). The old command `normalize(V + N*(Omega x V))` is anchored on
   the velocity: from that state Omega is tiny (target far), so the command is
   essentially "keep flying the way you drift", thrust builds speed that way, and
   the loop chases its own velocity - the climb-away the user saw. Rewrote
   `pn_steer_direction` anchored on the line of sight: a constant-bearing lead
   course (match the target's across-LOS velocity, close along the LOS; exactly
   "point at the target" for a stationary one) plus the classic PN LOS-rate term
   as damping. Law-level regression: `pn_points_at_a_stationary_target_from_a_sideways_launch`.

2. **Unbounded speed made the turning circle bigger than the fuze.** The torpedo
   thrusts the whole flight; by arrival on a long shot it was doing 60+ u/s and
   could only orbit the target at ~20u standoff - outside the 15u blast trigger
   (the observed plateau matched speed/turn-rate). Added a cruise cap: config
   `max_speed` (35) gating thrust on the *along-nose* speed (a total-speed cut
   left it ballistic and unable to steer; measured 21u miss), plus config
   `linear_damping` (0.8) on the body, because turning against a moving target
   "pumps" total speed past an along-nose gate alone (measured 60 u/s). With both,
   in-game speed holds ~30-33.

Verification (headless, Xvfb):
- 06 range, stationary gates: fire -> detonate in ~0.9s, three in a row.
- 07 guidance, 15 u/s crosser: speeds 30-33, closest approach descends through the
  fuze, 2 detonations in the window. Previously: 0 detonations, 19-21u standoff.
- Closed-loop tests now start from the real launch state (slow, sideways, nose
  forward) with thrust-along-nose dynamics, the cruise cap, and drag:
  `pn_turns_a_sideways_launch_onto_a_stationary_target`,
  `pn_intercepts_a_crossing_target`, `pn_intercepts_a_target_crossing_either_way`,
  `thrust_tapers_to_zero_at_cruise_speed`. 18 torpedo tests green.

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
